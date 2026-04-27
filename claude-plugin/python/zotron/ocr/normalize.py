"""Normalize OCR provider raw payloads into Zotron blocks and chunks."""

from __future__ import annotations

import re
from collections.abc import Mapping, Sequence
from typing import Any

_ALLOWED_TYPES = {
    "heading", "paragraph", "table", "figure", "equation", "caption",
    "footnote", "header", "footer", "reference", "unknown",
}
_HEADING_RE = re.compile(r"^(#{1,6}\s+.+|[一二三四五六七八九十]+、.+|\d+[.、]\s*.+)$")


def _block_type(value: Any) -> str:
    value = str(value or "paragraph").lower()
    return value if value in _ALLOWED_TYPES else "unknown"


def _block_id(attachment_key: str, page: int | None, order: int) -> str:
    page_part = f"p{page}" if page is not None else "p0"
    return f"{attachment_key}:{page_part}:b{order:02d}"


def _markdown_blocks(markdown: str, *, page: int | None, start_order: int) -> list[dict[str, Any]]:
    parts = [part.strip() for part in re.split(r"\n\s*\n", markdown) if part.strip()]
    rows: list[dict[str, Any]] = []
    section = ""
    for offset, part in enumerate(parts, start_order):
        text = re.sub(r"^#{1,6}\s+", "", part).strip()
        typ = "heading" if _HEADING_RE.match(part) else "paragraph"
        if typ == "heading":
            section = text
        rows.append({
            "type": typ,
            "page": page,
            "reading_order": offset,
            "section_heading": section,
            "text": text,
            "source_ref": f"markdown:{page or 0}:{offset}",
        })
    return rows


def _raw_blocks(raw: Mapping[str, Any]) -> list[dict[str, Any]]:
    if isinstance(raw.get("blocks"), list):
        return [dict(b) for b in raw["blocks"]]
    if isinstance(raw.get("pages"), list):
        rows: list[dict[str, Any]] = []
        for page in raw["pages"]:
            if not isinstance(page, Mapping):
                continue
            markdown = page.get("markdown") or page.get("text") or ""
            rows.extend(_markdown_blocks(str(markdown), page=page.get("page"), start_order=len(rows) + 1))
        return rows
    markdown = raw.get("markdown") or raw.get("md_results") or raw.get("text") or ""
    return _markdown_blocks(str(markdown), page=None, start_order=1)


def normalize_provider_raw(
    *,
    provider: str,
    raw: Mapping[str, Any],
    item_key: str,
    attachment_key: str,
) -> list[dict[str, Any]]:
    """Convert provider-specific raw data into schema-versioned Zotron blocks."""
    normalized: list[dict[str, Any]] = []
    for index, block in enumerate(_raw_blocks(raw), start=1):
        text = str(block.get("text") or block.get("markdown") or block.get("content") or "").strip()
        if not text:
            continue
        page = block.get("page")
        order = int(block.get("reading_order") or index)
        row: dict[str, Any] = {
            "schema_version": "1",
            "block_id": block.get("block_id") or _block_id(attachment_key, page, order),
            "attachment_key": attachment_key,
            "item_key": item_key,
            "type": _block_type(block.get("type")),
            "page": page,
            "bbox": block.get("bbox"),
            "reading_order": order,
            "section_heading": block.get("section_heading") or "",
            "text": text,
            "caption": block.get("caption") or "",
            "image_ref": block.get("image_ref") or "",
            "source_provider": provider,
            "source_ref": block.get("source_ref") or f"{provider}:blocks:{index}",
            "confidence": block.get("confidence"),
        }
        normalized.append({k: v for k, v in row.items() if v is not None})
    return normalized


def build_chunks_from_blocks(blocks: Sequence[Mapping[str, Any]], *, max_chars: int = 3500) -> list[dict[str, Any]]:
    """Build section-aware RAG chunks from normalized blocks without crossing sections."""
    chunks: list[dict[str, Any]] = []
    current: list[Mapping[str, Any]] = []
    current_section: str | None = None
    current_len = 0

    def flush() -> None:
        nonlocal current, current_section, current_len
        if not current:
            return
        first = current[0]
        attachment_key = str(first.get("attachment_key") or "att")
        pages = [b.get("page") for b in current if b.get("page") is not None]
        text = "\n\n".join(str(b.get("text", "")).strip() for b in current if str(b.get("text", "")).strip())
        chunks.append({
            "schema_version": "1",
            "chunk_id": f"{attachment_key}:c{len(chunks) + 1:06d}",
            "item_key": first.get("item_key") or "",
            "attachment_key": attachment_key,
            "block_ids": [str(b.get("block_id")) for b in current if b.get("block_id")],
            "section_heading": current_section or "",
            "page_start": min(pages) if pages else None,
            "page_end": max(pages) if pages else None,
            "text": text,
            "char_start": 0,
            "char_end": len(text),
            "level": "chunk",
        })
        current = []
        current_section = None
        current_len = 0

    for block in blocks:
        section = str(block.get("section_heading") or "")
        text = str(block.get("text") or "")
        should_flush = bool(
            current
            and ((section != current_section) or (current_len + len(text) > max_chars))
        )
        if should_flush:
            flush()
        current.append(block)
        current_section = section
        current_len += len(text)
    flush()
    return [{k: v for k, v in chunk.items() if v is not None} for chunk in chunks]
