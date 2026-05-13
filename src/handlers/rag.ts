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
  collectionKey?: string | number;
  keys?: (number | string)[];
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

type RetrievalEvidenceRef = {
  block_key: string;
  page_idx: number;
  bbox?: [number, number, number, number];
};

type ItemArtifacts = {
  chunks: Record<string, any>[];
  embeddingArtifacts: EmbeddingArtifactMetadata[];
};

type RetrievalHit = {
  item_key: string;
  attachment_key?: string;
  title: string;
  text: string;
  authors?: string[];
  year?: number;
  venue?: string;
  doi?: string;
  zotero_uri: string;
  section_heading?: string;
  section_path?: string[];
  chunk_key: string;
  block_keys?: string[];
  page_idx?: number;
  page_range?: [number, number];
  bbox?: [number, number, number, number];
  evidence_refs?: RetrievalEvidenceRef[];
  query: string;
  score: number;
  retrieval_mode?: RetrievalMode;
  embedding_artifact_title?: string;
  embedding_artifact_path?: string;
};

function parentDir(path: string): string {
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  if (index <= 0) return "";
  const parent = normalized.slice(0, index);
  return path.includes("\\") ? parent.replace(/\//g, "\\") : parent;
}

function joinPath(...parts: string[]): string {
  const separator = parts[0]?.includes("\\") ? "\\" : "/";
  return parts
    .filter(Boolean)
    .map((part, index) => index === 0 ? part.replace(/[\\/]+$/g, "") : part.replace(/^[\\/]+|[\\/]+$/g, ""))
    .join(separator);
}

async function readOptionalText(path: string): Promise<string | null> {
  try {
    return String((await Zotero.File.getContentsAsync(path)) || "");
  } catch {
    return null;
  }
}

function fileExists(path: string): boolean {
  try {
    const file = Zotero.File.pathToFile?.(path);
    return !!file?.exists?.();
  } catch {
    return false;
  }
}

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
  if (params.keys?.length) {
    const resolved = [];
    for (const k of params.keys) {
      if (typeof k === "number") {
        const item = await Zotero.Items.getAsync(k);
        if (item) resolved.push(item);
      } else {
        const libraryID = Zotero.Libraries.userLibraryID;
        const item = await Zotero.Items.getByLibraryAndKeyAsync(libraryID, k);
        if (item) resolved.push(item);
      }
    }
    return resolved;
  }

  const collectionRef = params.collectionKey ?? params.collection;
  if (collectionRef === undefined || collectionRef === null || collectionRef === "") {
    throw rpcError(INVALID_PARAMS, "rag.searchHits requires collection, collectionKey, or keys");
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
  const attachmentKeys = item.getAttachments?.() || [];

  for (const attachmentKey of attachmentKeys) {
    const attachment = await Zotero.Items.getAsync(attachmentKey);
    if (!attachment?.isAttachment?.()) continue;

    const title = String(attachment.getField?.("title") || "");
    const path = await attachment.getFilePathAsync?.();

    if (path && String(attachment.attachmentContentType || "").toLowerCase() === "application/pdf") {
      const sidecarRoot = joinPath(parentDir(String(path)), ".zotron");
      const chunksPath = joinPath(sidecarRoot, "chunks", "chunks.v1.jsonl");
      const chunksContent = await readOptionalText(chunksPath);
      if (chunksContent) chunks.push(...parseJsonl(chunksContent, chunksPath));

      const vectorsPath = joinPath(sidecarRoot, "embeddings", "vectors.jsonl");
      if (fileExists(vectorsPath)) {
        embeddingArtifacts.push({ title: "vectors.jsonl", path: vectorsPath });
      }
    }

    if (!title.endsWith(CHUNKS_SUFFIX) && !title.endsWith(EMBEDDING_SUFFIX)) continue;

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

function tuple4(value: any): [number, number, number, number] | undefined {
  return Array.isArray(value)
    && value.length === 4
    && value.every((one) => typeof one === "number" && Number.isFinite(one))
    ? [value[0], value[1], value[2], value[3]]
    : undefined;
}

function numericPair(value: any): [number, number] | undefined {
  return Array.isArray(value)
    && value.length === 2
    && value.every((one) => typeof one === "number" && Number.isFinite(one))
    ? [value[0], value[1]]
    : undefined;
}

function hitFromChunk(
  item: any,
  chunk: Record<string, any>,
  query: string,
  score: number,
  embeddingArtifact?: EmbeddingArtifactMetadata,
): RetrievalHit {
  const itemKey = String(chunk.item_key || item.key || item.id);
  const attachmentKey = chunk.attachment_key === undefined ? undefined : String(chunk.attachment_key);
  const title = String(chunk.title || item.getField?.("title") || "");
  const chunkKey = String(chunk.chunk_key || chunk.chunk_id || `${itemKey}:c${chunk.chunk_index ?? 0}`);
  const evidenceRefs = Array.isArray(chunk.evidence_refs)
    ? chunk.evidence_refs
      .map((ref: any) => {
        const blockKey = String(ref.block_key || ref.block_id || "");
        const pageIdx = Number(ref.page_idx ?? ref.page);
        if (!blockKey || !Number.isFinite(pageIdx)) return null;
        const normalized: RetrievalEvidenceRef = {
          block_key: blockKey,
          page_idx: pageIdx,
        };
        const bbox = tuple4(ref.bbox);
        if (bbox) normalized.bbox = bbox;
        return normalized;
      })
      .filter((ref): ref is RetrievalEvidenceRef => ref !== null)
    : [];
  const firstRef = evidenceRefs[0];
  const pageRange = numericPair(chunk.page_range);
  const bbox = tuple4(chunk.bbox) || firstRef?.bbox;
  const hit: RetrievalHit = {
    item_key: itemKey,
    title,
    text: String(chunk.text || ""),
    authors: Array.isArray(chunk.authors) ? chunk.authors : itemAuthors(item),
    zotero_uri: String(chunk.zotero_uri || `zotero://select/library/items/${itemKey}`),
    section_heading: String(chunk.section_heading || chunk.section || ""),
    chunk_key: chunkKey,
    query,
    score,
    retrieval_mode: embeddingArtifact ? "lexical_fallback" : "lexical",
  };
  if (attachmentKey) hit.attachment_key = attachmentKey;
  if (Array.isArray(chunk.section_path)) hit.section_path = chunk.section_path.map(String);
  if (Number.isFinite(Number(chunk.page_idx ?? chunk.page_start))) {
    hit.page_idx = Number(chunk.page_idx ?? chunk.page_start);
  } else if (firstRef) {
    hit.page_idx = firstRef.page_idx;
  }
  if (pageRange) hit.page_range = pageRange;
  if (bbox) hit.bbox = bbox;
  if (evidenceRefs.length) hit.evidence_refs = evidenceRefs;
  const year = Number(chunk.year) || itemYear(item);
  if (year) hit.year = year;
  const venue = String(chunk.venue || itemVenue(item));
  if (venue) hit.venue = venue;
  const doi = String(chunk.doi || item.getField?.("DOI") || "");
  if (doi) hit.doi = doi;
  if (Array.isArray(chunk.block_keys)) {
    hit.block_keys = chunk.block_keys;
  } else if (Array.isArray(chunk.block_ids)) {
    hit.block_keys = chunk.block_ids;
  }
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

  scored.sort((a, b) => b.score - a.score || a.chunk_key.localeCompare(b.chunk_key));
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
