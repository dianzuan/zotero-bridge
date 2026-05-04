"""OCR processor: orchestrates Zotero traversal, OCR, artifacts, and previews."""

from __future__ import annotations

import datetime
import tempfile
from pathlib import Path
from typing import Any, cast

import markdown as md  # type: ignore[import-untyped]

from zotron.collections import find_by_name as _find_collection_by_name
from zotron.ocr.artifacts import (
    CHUNKS_SUFFIX,
    ProviderRawArtifact,
    write_blocks_jsonl,
    write_chunks_jsonl,
    write_provider_raw_zip,
)
from zotron.ocr.engine import OCREngine
from zotron.ocr.normalize import blocks_from_provider_payload, chunks_from_blocks
from zotron.paths import linux_path, zotero_path
from zotron.rpc import ZoteroRPC


class OCRProcessor:
    """Orchestrates: find items → OCR → attach artifacts → optional Note preview."""

    def __init__(
        self,
        rpc: ZoteroRPC,
        engine: OCREngine,
        *,
        artifact_dir: str | Path | None = None,
        write_preview_note: bool = True,
    ) -> None:
        self.rpc = rpc
        self.engine = engine
        self.artifact_dir = Path(artifact_dir).expanduser() if artifact_dir else None
        self.write_preview_note = write_preview_note

    # ------------------------------------------------------------------
    # Collection helpers
    # ------------------------------------------------------------------

    def find_collection_id(self, name: str) -> str | None:
        """Search the collections tree recursively for a collection by name."""
        return _find_collection_by_name(self.rpc, name)

    # ------------------------------------------------------------------
    # Note helpers
    # ------------------------------------------------------------------

    def has_ocr_note(self, item_id: str) -> bool:
        """Return True if the item already has a Note tagged 'ocr'."""
        notes = cast(list[dict[str, Any]], self.rpc.call("notes.get", {"parentKey": item_id}) or [])
        for note in notes:
            tags = note.get("tags") or []
            if "ocr" in tags:
                return True
        return False

    def has_ocr_artifact(self, item_id: str) -> bool:
        """Return True if the item has OCR/RAG chunk artifacts attached."""
        attachments = cast(
            list[dict[str, Any]],
            self.rpc.call("attachments.list", {"parentKey": item_id}) or [],
        )
        return any(str(att.get("title") or "").endswith(CHUNKS_SUFFIX) for att in attachments)

    def has_ocr_result(self, item_id: str) -> bool:
        """Return True when either canonical artifacts or legacy OCR note exists."""
        return self.has_ocr_artifact(item_id) or self.has_ocr_note(item_id)

    # ------------------------------------------------------------------
    # Attachment helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _to_linux_path(win_path: str) -> str:
        """Convert a Windows path to a WSL Linux path if running under WSL."""
        return linux_path(win_path)

    def get_pdf_attachment(self, item_id: str) -> dict[str, Any] | None:
        """Return first PDF attachment with a resolved filesystem ``path``."""
        attachments = cast(
            list[dict[str, Any]],
            self.rpc.call("attachments.list", {"parentKey": item_id}) or [],
        )
        for att in attachments:
            if att.get("contentType") != "application/pdf":
                continue
            att_key = att.get("key")
            result = self.rpc.call("attachments.getPath", {"key": att_key})
            path_str = result.get("path") if isinstance(result, dict) else result
            if path_str:
                return {**att, "path": self._to_linux_path(str(path_str))}
        return None

    def get_pdf_path(self, item_id: str) -> Path | None:
        """Return the filesystem path to the first PDF attachment, or None."""
        attachment = self.get_pdf_attachment(item_id)
        return Path(attachment["path"]) if attachment else None

    def _item_key(self, item_id: str) -> str:
        try:
            item = self.rpc.call("items.get", {"key": item_id}) or {}
        except Exception:  # noqa: BLE001 - item key is a best-effort filename aid
            item = {}
        return str(item.get("key") or item.get("itemKey") or item_id)

    @staticmethod
    def _attachment_key(attachment: dict[str, Any], item_id: str) -> str:
        return str(
            attachment.get("key")
            or attachment.get("itemKey")
            or f"item-{item_id}-pdf"
        )

    def _attach_artifact(self, item_id: str, path: Path) -> None:
        self.rpc.call(
            "attachments.add",
            {"parentKey": item_id, "path": zotero_path(path), "title": path.name},
        )

    # ------------------------------------------------------------------
    # HTML formatting
    # ------------------------------------------------------------------

    def format_note_html(
        self,
        title: str,
        markdown: str,
        provider: str,
        page_count: int | None = None,
    ) -> str:
        """Convert markdown to HTML and wrap with OCR header."""
        date_str = datetime.date.today().isoformat()
        if page_count is not None:
            meta = f"OCR by {provider} | {date_str} | {page_count} pages"
        else:
            meta = f"OCR by {provider} | {date_str}"

        body_html = md.markdown(
            markdown, extensions=["tables", "fenced_code"]
        )

        return (
            f"<h1>OCR: {title}</h1>\n"
            f"<p><em>{meta}</em></p>\n"
            f"<hr/>\n"
            f"{body_html}"
        )

    # ------------------------------------------------------------------
    # OCR result helpers
    # ------------------------------------------------------------------

    @staticmethod
    def _result_field(result: Any, name: str, default: Any = None) -> Any:
        if isinstance(result, dict):
            return result.get(name, default)
        return getattr(result, name, default)

    def _coerce_ocr_result(self, result: Any) -> dict[str, Any]:
        if isinstance(result, str):
            provider = type(self.engine).__name__
            return {
                "provider": provider,
                "model": provider,
                "raw_payload": {"markdown": result},
                "markdown": result,
                "files": {},
            }

        markdown = self._result_field(result, "markdown") or self._result_field(result, "text") or ""
        raw_payload = self._result_field(result, "raw_payload")
        if raw_payload is None:
            raw_payload = {"markdown": markdown}
        provider = self._result_field(result, "provider") or type(self.engine).__name__
        return {
            "provider": str(provider),
            "model": str(self._result_field(result, "model") or provider),
            "raw_payload": raw_payload,
            "markdown": str(markdown),
            "files": dict(self._result_field(result, "files", {}) or {}),
        }

    def _write_artifacts(
        self,
        *,
        directory: Path,
        item_id: str,
        item_key: str,
        attachment_key: str,
        pdf_path: Path,
        ocr: dict[str, Any],
    ) -> list[Path]:
        raw_path = Path(
            write_provider_raw_zip(
                directory,
                ProviderRawArtifact(
                    item_key=item_key,
                    attachment_key=attachment_key,
                    provider=ocr["provider"],
                    payload=ocr["raw_payload"],
                    files=ocr["files"],
                    source_path=str(pdf_path),
                ),
            ),
        )
        blocks = blocks_from_provider_payload(
            ocr["raw_payload"],
            item_key=item_key,
            attachment_key=attachment_key,
            provider=ocr["provider"],
        )
        if not blocks and ocr["markdown"]:
            blocks = blocks_from_provider_payload(
                {"markdown": ocr["markdown"]},
                item_key=item_key,
                attachment_key=attachment_key,
                provider=ocr["provider"],
            )
        chunks = chunks_from_blocks(blocks)
        blocks_path = Path(write_blocks_jsonl(directory, item_key, blocks))
        chunks_path = Path(write_chunks_jsonl(directory, item_key, chunks))
        return [raw_path, blocks_path, chunks_path]

    # ------------------------------------------------------------------
    # Processing
    # ------------------------------------------------------------------

    def process_item(
        self, item_id: str, title: str, force: bool = False
    ) -> str:
        """OCR a single item, attach raw/block/chunk artifacts, and maybe a Note."""
        try:
            if not force and self.has_ocr_result(item_id):
                return "skipped"

            attachment = self.get_pdf_attachment(item_id)
            if attachment is None:
                return "error: no PDF attachment found"
            pdf_path = Path(attachment["path"])
            item_key = self._item_key(item_id)
            attachment_key = self._attachment_key(attachment, item_id)

            ocr = self._coerce_ocr_result(self.engine.ocr_pdf(pdf_path))

            if self.artifact_dir is None:
                with tempfile.TemporaryDirectory(prefix="zotron-ocr-") as tmp:
                    paths = self._write_artifacts(
                        directory=Path(tmp),
                        item_id=item_id,
                        item_key=item_key,
                        attachment_key=attachment_key,
                        pdf_path=pdf_path,
                        ocr=ocr,
                    )
                    for path in paths:
                        self._attach_artifact(item_id, path)
            else:
                self.artifact_dir.mkdir(parents=True, exist_ok=True)
                paths = self._write_artifacts(
                    directory=self.artifact_dir,
                    item_id=item_id,
                    item_key=item_key,
                    attachment_key=attachment_key,
                    pdf_path=pdf_path,
                    ocr=ocr,
                )
                for path in paths:
                    self._attach_artifact(item_id, path)

            if self.write_preview_note:
                html = self.format_note_html(title, ocr["markdown"], ocr["provider"])
                self.rpc.call(
                    "notes.create",
                    {
                        "parentKey": item_id,
                        "content": html,
                        "tags": ["ocr"],
                    },
                )
            return "ok"
        except Exception as exc:  # noqa: BLE001
            return f"error: {exc}"

    def process_collection(
        self, collection_name: str, force: bool = False
    ) -> dict:
        """OCR all items in the named collection."""
        result: dict = {"ok": 0, "skipped": 0, "errors": []}

        collection_id = self.find_collection_id(collection_name)
        if collection_id is None:
            result["errors"].append(
                f"Collection not found: {collection_name!r}"
            )
            return result

        raw = self.rpc.call("collections.getItems", {"key": collection_id, "limit": 500}) or {}
        items = raw.get("items", []) if isinstance(raw, dict) else raw

        for item in items:
            item_key = item.get("key")
            title = item.get("title", "(untitled)")
            status = self.process_item(item_key, title, force=force)
            if status == "ok":
                result["ok"] += 1
            elif status == "skipped":
                result["skipped"] += 1
            else:
                result["errors"].append(f"[{item_key}] {title}: {status}")

        return result
