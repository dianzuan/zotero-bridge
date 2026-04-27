"""Normalize OCR provider outputs into Zotron blocks and chunks."""
from __future__ import annotations

import re
from typing import Any

_ALLOWED_TYPES = {"heading", "paragraph", "table", "figure", "equation", "caption", "footnote", "header", "footer", "reference", "unknown"}
_HEADING_RE = re.compile(r"^(?:#{1,6}\s+)?(.+)$")


def _block_type(value: str | None) -> str:
    value = (value or "paragraph").lower()
    return value if value in _ALLOWED_TYPES else "unknown"


def _clean_heading(text: str) -> str:
    match = _HEADING_RE.match(text.strip())
    return (match.group(1) if match else text).strip()


def _structured_blocks(raw: Any) -> list[dict[str, Any]]:
    if isinstance(raw, dict):
        for key in ("blocks", "elements", "content", "items"):
            val = raw.get(key)
            if isinstance(val, list) and val and all(isinstance(x, dict) for x in val):
                return val
    if isinstance(raw, list) and all(isinstance(x, dict) for x in raw):
        return raw
    return []


def _markdown_pages(raw: Any) -> list[tuple[int | None, str, str]]:
    pages: list[tuple[int | None, str, str]] = []
    if isinstance(raw, dict) and isinstance(raw.get("pages"), list):
        for idx, page in enumerate(raw["pages"]):
            if not isinstance(page, dict):
                continue
            text = page.get("markdown") or page.get("text") or ""
            if text:
                pages.append((page.get("page") or page.get("page_number") or idx + 1, str(text), f"pages[{idx}].markdown"))
    elif isinstance(raw, dict):
        text = raw.get("markdown") or raw.get("text") or ""
        if text:
            pages.append((None, str(text), "markdown" if raw.get("markdown") else "text"))
    elif isinstance(raw, str):
        pages.append((None, raw, "text"))
    return pages


def _blocks_from_markdown(text: str, *, page: int | None, source_ref: str) -> list[dict[str, Any]]:
    blocks: list[dict[str, Any]] = []
    section = ""
    for part in [p.strip() for p in re.split(r"\n\s*\n", text) if p.strip()]:
        lines = [line.strip() for line in part.splitlines() if line.strip()]
        if not lines:
            continue
        first = lines[0]
        is_heading = first.startswith("#") or bool(re.match(r"^([一二三四五六七八九十]+、|\d+[.、]|第[一二三四五六七八九十\d]+[章节部分])", first))
        btype = "heading" if is_heading else "paragraph"
        block_text = "\n".join(lines)
        if btype == "heading":
            section = _clean_heading(block_text.lstrip("# "))
        blocks.append({"type": btype, "page": page, "section_heading": section, "text": block_text, "source_ref": source_ref})
    return blocks


def normalize_provider_raw(*, provider: str, raw: Any, item_key: str, attachment_key: str) -> list[dict[str, Any]]:
    """Return Zotron block dictionaries from structured provider output or fallback text."""
    source_blocks = _structured_blocks(raw)
    normalized: list[dict[str, Any]] = []
    section = ""
    if source_blocks:
        for idx, block in enumerate(source_blocks):
            text = str(block.get("text") or block.get("content") or block.get("markdown") or "").strip()
            if not text:
                continue
            btype = _block_type(block.get("type") or block.get("category"))
            if btype == "heading":
                section = _clean_heading(text)
            page = block.get("page") or block.get("page_number")
            order = block.get("reading_order", idx)
            normalized.append({
                "block_id": block.get("block_id") or f"{attachment_key}:p{page or 0}:b{int(order):02d}",
                "attachment_key": attachment_key,
                "item_key": item_key,
                "type": btype,
                "page": page,
                "bbox": block.get("bbox") or block.get("bounding_box"),
                "reading_order": order,
                "section_heading": block.get("section_heading") or section,
                "text": text,
                "caption": block.get("caption", ""),
                "image_ref": block.get("image_ref", ""),
                "source_provider": provider,
                "source_ref": block.get("source_ref") or f"blocks[{idx}]",
                "confidence": block.get("confidence"),
            })
        return normalized

    idx = 0
    for page, text, source_ref in _markdown_pages(raw):
        for block in _blocks_from_markdown(text, page=page, source_ref=source_ref):
            page_part = page or 0
            block.update({
                "block_id": f"{attachment_key}:p{page_part}:b{idx:02d}",
                "attachment_key": attachment_key,
                "item_key": item_key,
                "source_provider": provider,
                "reading_order": idx,
            })
            normalized.append(block)
            idx += 1
    return normalized


def build_chunks_from_blocks(blocks: list[dict[str, Any]], *, max_chars: int = 2000) -> list[dict[str, Any]]:
    """Build section-aware RAG chunks from normalized blocks without crossing sections."""
    chunks: list[dict[str, Any]] = []
    current: list[dict[str, Any]] = []
    current_section: str | None = None
    current_len = 0
    item_key = blocks[0].get("item_key", "") if blocks else ""
    attachment_key = blocks[0].get("attachment_key", "") if blocks else ""

    def flush() -> None:
        nonlocal current, current_len
        if not current:
            return
        pages = [b.get("page") for b in current if b.get("page") is not None]
        chunks.append({
            "chunk_id": f"{attachment_key}:c{len(chunks) + 1:06d}",
            "item_key": current[0].get("item_key", item_key),
            "attachment_key": current[0].get("attachment_key", attachment_key),
            "block_ids": [b["block_id"] for b in current],
            "section_heading": current[0].get("section_heading", ""),
            "page_start": min(pages) if pages else None,
            "page_end": max(pages) if pages else None,
            "text": "\n\n".join(b.get("text", "") for b in current),
            "char_start": 0,
            "char_end": sum(len(b.get("text", "")) for b in current),
            "level": "chunk",
        })
        current = []
        current_len = 0

    for block in blocks:
        if block.get("type") == "heading":
            flush()
            current_section = block.get("section_heading") or block.get("text", "")
            block = {**block, "section_heading": current_section}
            current.append(block)
            current_len += len(block.get("text", ""))
            continue
        text = block.get("text", "")
        section = block.get("section_heading") or current_section or ""
        block = {**block, "section_heading": section}
        if current and (section != current_section or current_len + len(text) > max_chars):
            flush()
        current_section = section
        current.append(block)
        current_len += len(text)
    flush()
    return chunks
