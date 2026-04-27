"""Normalize provider OCR payloads into Zotron blocks and RAG chunks."""
from __future__ import annotations

import re
from typing import Any

_BLOCK_TYPES = {
    "heading", "paragraph", "table", "figure", "equation", "caption",
    "footnote", "header", "footer", "reference", "unknown",
}


def _coerce_type(value: Any) -> str:
    text = str(value or "paragraph").lower()
    aliases = {"text": "paragraph", "title": "heading", "image": "figure"}
    return aliases.get(text, text if text in _BLOCK_TYPES else "unknown")


def _iter_structured_blocks(payload: Any):
    if not isinstance(payload, dict):
        return
    pages = payload.get("pages")
    if isinstance(pages, list):
        for p_idx, page in enumerate(pages):
            page_no = page.get("page") or page.get("page_number") or p_idx + 1
            blocks = page.get("blocks") or page.get("elements") or page.get("layout") or []
            for b_idx, block in enumerate(blocks):
                if isinstance(block, dict):
                    yield p_idx, b_idx, int(page_no), block, f"pages[{p_idx}].blocks[{b_idx}]"
    blocks = payload.get("blocks") or payload.get("elements") or payload.get("layout")
    if isinstance(blocks, list):
        for b_idx, block in enumerate(blocks):
            if isinstance(block, dict):
                page_no = block.get("page") or block.get("page_number") or 1
                yield 0, b_idx, int(page_no), block, f"blocks[{b_idx}]"


def _markdown_blocks(markdown: str):
    section = ""
    order = 0
    paragraphs = [p.strip() for p in re.split(r"\n\s*\n", markdown) if p.strip()]
    for idx, para in enumerate(paragraphs):
        heading = re.match(r"^#{1,6}\s+(.+)$", para)
        if heading:
            section = heading.group(1).strip()
            yield idx, order, "heading", section, section
        else:
            yield idx, order, "paragraph", para, section
        order += 1


def blocks_from_provider_payload(
    payload: Any,
    *,
    item_key: str,
    attachment_key: str,
    provider: str,
) -> list[dict[str, Any]]:
    """Return normalized Zotron block dictionaries from provider raw data.

    Structured provider fields are preferred. Markdown is only a fallback when
    the provider exposes no block/page structure.
    """
    blocks: list[dict[str, Any]] = []
    structured = list(_iter_structured_blocks(payload) or [])
    section = ""
    for _p_idx, b_idx, page_no, block, source_ref in structured:
        text = str(block.get("text") or block.get("content") or block.get("markdown") or "").strip()
        if not text:
            continue
        block_type = _coerce_type(block.get("type") or block.get("category"))
        if block_type == "heading":
            section = text
        blocks.append({
            "block_id": f"{attachment_key}:p{page_no}:b{b_idx}",
            "attachment_key": attachment_key,
            "item_key": item_key,
            "type": block_type,
            "page": page_no,
            "bbox": block.get("bbox") or block.get("box"),
            "reading_order": int(block.get("reading_order", b_idx)),
            "section_heading": block.get("section_heading") or section,
            "text": text,
            "caption": block.get("caption", ""),
            "image_ref": block.get("image_ref") or block.get("image", ""),
            "source_provider": provider,
            "source_ref": source_ref,
            "confidence": block.get("confidence"),
        })
    if blocks:
        return blocks

    markdown = ""
    if isinstance(payload, dict):
        markdown = str(payload.get("markdown") or payload.get("md_results") or payload.get("result") or "")
    elif isinstance(payload, str):
        markdown = payload
    for para_idx, order, block_type, text, section in _markdown_blocks(markdown):
        blocks.append({
            "block_id": f"{attachment_key}:p0:b{order}",
            "attachment_key": attachment_key,
            "item_key": item_key,
            "type": block_type,
            "page": None,
            "bbox": None,
            "reading_order": order,
            "section_heading": "" if block_type == "heading" else section,
            "text": text,
            "caption": "",
            "image_ref": "",
            "source_provider": provider,
            "source_ref": f"markdown:{para_idx}",
            "confidence": None,
        })
    return blocks


def chunks_from_blocks(blocks: list[dict[str, Any]], *, max_chars: int = 1000) -> list[dict[str, Any]]:
    """Build section-aware RAG chunks from normalized blocks."""
    chunks: list[dict[str, Any]] = []
    current: list[dict[str, Any]] = []
    current_section = None
    current_len = 0
    attachment_key = str(blocks[0].get("attachment_key", "att")) if blocks else "att"

    def flush() -> None:
        nonlocal current, current_len
        if not current:
            return
        text = "\n\n".join(str(b.get("text", "")) for b in current).strip()
        pages = [b.get("page") for b in current if b.get("page") is not None]
        chunk_index = len(chunks)
        chunks.append({
            "chunk_id": f"{attachment_key}:c{chunk_index}",
            "item_key": current[0].get("item_key"),
            "attachment_key": current[0].get("attachment_key"),
            "block_ids": [b.get("block_id") for b in current],
            "section_heading": current[0].get("section_heading", ""),
            "page_start": min(pages) if pages else None,
            "page_end": max(pages) if pages else None,
            "text": text,
            "char_start": 0,
            "char_end": len(text),
            "level": "chunk",
        })
        current = []
        current_len = 0

    for block in blocks:
        if block.get("type") == "heading":
            flush()
            current_section = block.get("text")
            continue
        section = block.get("section_heading") or current_section or ""
        block = {**block, "section_heading": section}
        text_len = len(str(block.get("text", "")))
        if current and (section != current_section or current_len + text_len > max_chars):
            flush()
        current_section = section
        current.append(block)
        current_len += text_len
    flush()
    return chunks
