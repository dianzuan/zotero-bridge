// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2026 diamondrill
// zotron/src/handlers/rag.ts
import { registerHandlers } from "../server";
import { rpcError, INVALID_PARAMS } from "../utils/errors";

const CHUNKS_SUFFIX = ".zotron-chunks.jsonl";
const EMBEDDING_SUFFIX = ".zotron-embed.npz";

type SearchHitsParams = {
  query: string;
  collection?: string | number;
  collectionId?: number;
  itemIds?: number[];
  itemKeys?: string[];
  limit?: number;
  top_spans_per_item?: number;
  include_fulltext_spans?: boolean;
};

type RetrievalMode = "lexical" | "lexical_fallback";

type EmbeddingArtifactMetadata = {
  title: string;
  path?: string;
};

type RetrievalMetadata = {
  mode: RetrievalMode;
  semantic_available: boolean;
  semantic_used: boolean;
  embedding_artifacts: number;
  reason?: string;
};

type SearchHitsResult = {
  hits: RetrievalHit[];
  total: number;
  retrieval: RetrievalMetadata;
};

type ItemArtifacts = {
  chunks: Record<string, any>[];
  embeddingArtifacts: EmbeddingArtifactMetadata[];
};

type RetrievalHit = {
  item_key: string;
  title: string;
  text: string;
  authors?: string[];
  year?: number;
  venue?: string;
  doi?: string;
  zotero_uri: string;
  section_heading?: string;
  chunk_id: string;
  block_ids?: string[];
  query: string;
  score: number;
  retrieval_mode?: RetrievalMode;
  embedding_artifact_title?: string;
  embedding_artifact_path?: string;
};

function parseJsonl(text: string, source: string): Record<string, any>[] {
  return text
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .map((line, index) => {
      try {
        return JSON.parse(line);
      } catch (err: any) {
        throw rpcError(INVALID_PARAMS, `Invalid JSONL in ${source} at line ${index + 1}: ${err.message}`);
      }
    });
}

function queryTerms(query: string): string[] {
  const terms = query
    .toLowerCase()
    .split(/[\s,;，；、]+/)
    .map((term) => term.trim())
    .filter(Boolean);
  return terms.length > 0 ? Array.from(new Set(terms)) : [query.toLowerCase()];
}

function lexicalScore(text: string, query: string): number {
  const haystack = text.toLowerCase();
  const needle = query.trim().toLowerCase();
  if (!haystack || !needle) return 0;

  let score = haystack.includes(needle) ? 1 : 0;
  for (const term of queryTerms(query)) {
    if (term && haystack.includes(term)) score += 1;
  }
  return score;
}

function creatorName(creator: any): string {
  if (creator.name) return creator.name;
  if (
    creator.firstName
    && creator.lastName
    && /[\u3400-\u9fff]/.test(`${creator.firstName}${creator.lastName}`)
  ) {
    return `${creator.lastName}${creator.firstName}`;
  }
  return [creator.firstName, creator.lastName].filter(Boolean).join(" ").trim();
}

function itemAuthors(item: any): string[] {
  return (item.getCreators?.() || [])
    .map(creatorName)
    .filter((name: string) => name.length > 0);
}

function itemYear(item: any): number | undefined {
  const date = String(item.getField?.("date") || "");
  const match = date.match(/\b(18|19|20|21)\d{2}\b/);
  return match ? Number(match[0]) : undefined;
}

function itemVenue(item: any): string {
  return String(
    item.getField?.("publicationTitle")
    || item.getField?.("journalAbbreviation")
    || item.getField?.("conferenceName")
    || item.getField?.("publisher")
    || "",
  );
}

async function resolveCollectionItems(params: SearchHitsParams): Promise<any[]> {
  if (params.itemIds?.length) {
    const items = await Zotero.Items.getAsync(params.itemIds);
    return (Array.isArray(items) ? items : [items]).filter(Boolean);
  }

  if (params.itemKeys?.length) {
    const libraryID = Zotero.Libraries.userLibraryID;
    const items = [];
    for (const key of params.itemKeys) {
      const item = await Zotero.Items.getByLibraryAndKeyAsync(libraryID, key);
      if (item) items.push(item);
    }
    return items;
  }

  const collectionRef = params.collectionId ?? params.collection;
  if (collectionRef === undefined || collectionRef === null || collectionRef === "") {
    throw rpcError(INVALID_PARAMS, "rag.searchHits requires collection, collectionId, itemIds, or itemKeys");
  }

  let collection: any = null;
  if (typeof collectionRef === "number") {
    collection = await Zotero.Collections.getAsync(collectionRef);
  } else {
    const collections = Zotero.Collections.getByLibrary(Zotero.Libraries.userLibraryID, true);
    collection = collections.find((col: any) => col.name === collectionRef);
  }
  if (!collection) {
    throw rpcError(INVALID_PARAMS, `Collection not found: ${collectionRef}`);
  }
  return (collection.getChildItems(false) || []).filter((item: any) => !item.isNote?.() && !item.isAttachment?.());
}

async function readItemArtifacts(item: any): Promise<ItemArtifacts> {
  const chunks: Record<string, any>[] = [];
  const embeddingArtifacts: EmbeddingArtifactMetadata[] = [];
  const attachmentIds = item.getAttachments?.() || [];

  for (const attachmentId of attachmentIds) {
    const attachment = await Zotero.Items.getAsync(attachmentId);
    if (!attachment?.isAttachment?.()) continue;

    const title = String(attachment.getField?.("title") || "");
    if (!title.endsWith(CHUNKS_SUFFIX) && !title.endsWith(EMBEDDING_SUFFIX)) continue;

    const path = await attachment.getFilePathAsync?.();
    if (title.endsWith(EMBEDDING_SUFFIX)) {
      const metadata: EmbeddingArtifactMetadata = { title };
      if (path) metadata.path = String(path);
      embeddingArtifacts.push(metadata);
      continue;
    }

    if (!path) continue;
    const content = String((await Zotero.File.getContentsAsync(path)) || "");
    chunks.push(...parseJsonl(content, title));
  }

  return { chunks, embeddingArtifacts };
}

function hitFromChunk(
  item: any,
  chunk: Record<string, any>,
  query: string,
  score: number,
  embeddingArtifact?: EmbeddingArtifactMetadata,
): RetrievalHit {
  const itemKey = String(chunk.item_key || item.key || item.id);
  const title = String(chunk.title || item.getField?.("title") || "");
  const chunkId = String(chunk.chunk_id || `${itemKey}:c${chunk.chunk_index ?? 0}`);
  const hit: RetrievalHit = {
    item_key: itemKey,
    title,
    text: String(chunk.text || ""),
    authors: Array.isArray(chunk.authors) ? chunk.authors : itemAuthors(item),
    zotero_uri: String(chunk.zotero_uri || `zotero://select/library/items/${itemKey}`),
    section_heading: String(chunk.section_heading || chunk.section || ""),
    chunk_id: chunkId,
    query,
    score,
    retrieval_mode: embeddingArtifact ? "lexical_fallback" : "lexical",
  };
  const year = Number(chunk.year) || itemYear(item);
  if (year) hit.year = year;
  const venue = String(chunk.venue || itemVenue(item));
  if (venue) hit.venue = venue;
  const doi = String(chunk.doi || item.getField?.("DOI") || "");
  if (doi) hit.doi = doi;
  if (Array.isArray(chunk.block_ids)) hit.block_ids = chunk.block_ids;
  if (embeddingArtifact) {
    hit.embedding_artifact_title = embeddingArtifact.title;
    if (embeddingArtifact.path) hit.embedding_artifact_path = embeddingArtifact.path;
  }
  return hit;
}

async function searchChunkArtifacts(params: SearchHitsParams): Promise<SearchHitsResult> {
  const query = params.query?.trim();
  if (!query) throw rpcError(INVALID_PARAMS, "rag.searchHits requires query");

  const limit = Math.max(1, params.limit ?? 50);
  const topSpansPerItem = Math.max(1, params.top_spans_per_item ?? 3);
  const items = await resolveCollectionItems(params);
  const scored: RetrievalHit[] = [];
  let embeddingArtifactCount = 0;

  for (const item of items) {
    const artifacts = await readItemArtifacts(item);
    const embeddingArtifact = artifacts.embeddingArtifacts[0];
    embeddingArtifactCount += artifacts.embeddingArtifacts.length;

    for (const chunk of artifacts.chunks) {
      const text = String(chunk.text || "");
      const score = lexicalScore(text, query);
      if (score <= 0) continue;
      scored.push(hitFromChunk(item, chunk, query, score, embeddingArtifact));
    }
  }

  scored.sort((a, b) => b.score - a.score || a.chunk_id.localeCompare(b.chunk_id));
  const perItem = new Map<string, number>();
  const hits: RetrievalHit[] = [];
  for (const hit of scored) {
    const seen = perItem.get(hit.item_key) || 0;
    if (seen >= topSpansPerItem) continue;
    perItem.set(hit.item_key, seen + 1);
    hits.push(hit);
    if (hits.length >= limit) break;
  }

  const hasEmbeddingArtifacts = embeddingArtifactCount > 0;
  const retrieval: RetrievalMetadata = {
    mode: hasEmbeddingArtifacts ? "lexical_fallback" : "lexical",
    semantic_available: hasEmbeddingArtifacts,
    semantic_used: false,
    embedding_artifacts: embeddingArtifactCount,
  };
  if (hasEmbeddingArtifacts) {
    retrieval.reason = "Embedding NPZ parsing and query embedding are not available in Zotero JS without new dependencies; lexical fallback was used.";
  }

  return { hits, total: hits.length, retrieval };
}

export const ragHandlers = {
  async searchHits(params: SearchHitsParams) {
    return searchChunkArtifacts(params);
  },
};

registerHandlers("rag", ragHandlers);
