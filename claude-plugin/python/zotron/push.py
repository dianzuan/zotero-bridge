"""Source-agnostic push layer on top of zotron.rpc.

Exposes high-level helpers used by scholar-source plugins (cnki-plugin,
future arxiv-plugin, etc.) to push item metadata + PDF attachments into
Zotero via the bridge XPI.

This module does NOT know anything CNKI-specific. All CNKI mapping lives
in cnki-plugin's exporters/zotero.py.
"""
from __future__ import annotations

import os
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Literal

from zotron.errors import CollectionAmbiguous, CollectionNotFound, InvalidPDF


def _is_wsl() -> bool:
    """Detect running inside Microsoft's Windows Subsystem for Linux."""
    if os.environ.get("WSL_DISTRO_NAME"):
        return True
    try:
        with open("/proc/sys/kernel/osrelease") as f:
            release = f.read().lower()
    except OSError:
        return False
    return "microsoft" in release or "wsl" in release


def _zotero_path(local_path: Path) -> str:
    """Translate a local filesystem path to a string Zotero can open.

    On WSL, Zotero typically runs on Windows and cannot read POSIX paths
    like /tmp/... . We convert to the Windows UNC form
    (\\\\wsl.localhost\\<distro>\\tmp\\...) using `wslpath -w`.

    On native Linux/macOS the path is returned unchanged.
    """
    path_str = str(local_path)
    if _is_wsl():
        try:
            result = subprocess.run(
                ["wslpath", "-w", path_str],
                capture_output=True, text=True, timeout=5, check=True,
            )
            return result.stdout.strip()
        except (subprocess.SubprocessError, FileNotFoundError):
            return path_str
    return path_str


def check_pdf_magic(path: Path) -> bool:
    """Return True iff `path` exists and starts with the `%PDF-` magic prefix.

    Used to block HTML error pages (login redirects, throttling blocks) that
    get served with a .pdf filename and Content-Type: application/pdf but
    whose body is actually HTML.
    """
    try:
        with open(path, "rb") as f:
            head = f.read(5)
    except (OSError, FileNotFoundError):
        return False
    return head == b"%PDF-"


def _item_has_pdf_attachment(rpc: Any, item_id: int) -> bool:
    existing = rpc.call("attachments.list", {"parentId": item_id}) or []
    # Match on MIME type OR .pdf filename suffix. Zotero stores some
    # attachments with blank/octet-stream/x-pdf contentType depending
    # on the source (manual import, older plugin builds, browser save
    # that wrote the wrong header). A MIME-only check false-negatives
    # on those and would re-attach a duplicate PDF.
    return any(
        a.get("contentType") == "application/pdf"
        or (a.get("path") or "").lower().endswith(".pdf")
        for a in existing
    )


def _attach_pdf(rpc: Any, item_id: int, pdf_path: Path) -> None:
    rpc.call("attachments.add", {
        "parentId": item_id,
        "path": _zotero_path(pdf_path),
        "title": "Full Text PDF",
    })


def _normalize(s: str) -> str:
    """Lowercase + collapse whitespace for fuzzy name matching."""
    return " ".join(s.lower().split())


def resolve_collection(
    rpc: Any,
    name_or_id: str | int | None,
    library: str | int | None = None,
) -> int:
    """Resolve a user-supplied collection reference to a concrete Zotero ID.

    Priority:
    1. `int` → returned as-is.
    2. `str` that parses as int → converted.
    3. `str` (non-numeric) → `collections.list`, then:
       a. exact `name` match → ID
       b. case-insensitive / whitespace-normalized fuzzy match
          - exactly one hit → ID
          - multiple hits → CollectionAmbiguous(candidates=[...])
       c. zero hits → CollectionNotFound
    4. `None` → `system.currentCollection`. If GUI has a selection, use it.
       Otherwise return 0, the sentinel for "library root (no collection)".

    `library` is reserved for future multi-library support; currently ignored.
    """
    if isinstance(name_or_id, int):
        return name_or_id
    if isinstance(name_or_id, str) and name_or_id.strip().lstrip("-").isdigit():
        return int(name_or_id.strip())
    if isinstance(name_or_id, str):
        collections = rpc.call("collections.list") or []
        target = name_or_id.strip()
        # exact name
        exact = [c for c in collections if c.get("name") == target]
        if len(exact) == 1:
            return int(exact[0]["id"])
        # fuzzy: contains match on normalized strings
        needle = _normalize(target)
        fuzzy = [c for c in collections if needle in _normalize(c.get("name", ""))]
        if len(fuzzy) == 1:
            return int(fuzzy[0]["id"])
        if len(fuzzy) > 1:
            raise CollectionAmbiguous(
                f"Multiple collections match {target!r}",
                candidates=[{"id": c["id"], "name": c["name"]} for c in fuzzy],
            )
        raise CollectionNotFound(f"No collection named {target!r}")
    # name_or_id is None — try GUI-selected first, fall back to library root.
    # Legacy XPI builds (<0.2) don't register system.currentCollection; we
    # keep the METHOD_NOT_FOUND fallback to library root for compatibility.
    try:
        selected = rpc.call("system.currentCollection")
    except RuntimeError as e:
        if "-32601" in str(e) or "Method not found" in str(e):
            return 0
        raise
    if selected and selected.get("id") is not None:
        return int(selected["id"])
    return 0


def find_duplicate(rpc: Any, item_json: dict) -> int | None:
    """Look up an existing Zotero item ID that matches `item_json`.

    Priority (each step short-circuits on first hit):
      1. DOI   → search.byIdentifier({doi})
      2. ISSN  → search.byIdentifier({issn})       (XPI ≥0.2)
      3. Title → search.quick, exact match only (skip if title <10 chars)

    Returns the first matching item ID, or None if not found.

    We deliberately do NOT use items.findDuplicates — it's a library-wide
    scan (Zotero.Duplicates object) and is too heavy for per-item lookup.
    """
    def _items(resp: Any) -> list:
        """search.* returns {items, total}; some mocks return bare lists."""
        if isinstance(resp, dict):
            return resp.get("items") or []
        if isinstance(resp, list):
            return resp
        return []

    doi = item_json.get("DOI")
    if doi:
        hits = _items(rpc.call("search.byIdentifier", {"doi": doi}))
        if hits:
            return int(hits[0]["id"])
    issn = item_json.get("ISSN")
    if issn:
        hits = _items(rpc.call("search.byIdentifier", {"issn": issn}))
        if hits:
            return int(hits[0]["id"])
    title = item_json.get("title", "")
    if len(title) >= 10:
        hits = _items(rpc.call("search.quick", {"query": title, "limit": 5}))
        for h in hits:
            if h.get("title") == title:
                return int(h["id"])
    return None


_FLAT_NON_FIELD_KEYS = frozenset({"itemType", "creators", "tags", "collections", "attachments", "relations", "notes", "id", "key", "version"})


def _to_xpi_payload(item_json: dict) -> dict:
    """Transform a flat Zotero-Connector-style item dict into the shape
    `items.create` / `items.update` expect on the XPI side:

        {
          itemType: str,
          fields: {title, DOI, abstractNote, ...},
          creators: [{firstName, lastName, creatorType}],  # no "type" key
          tags: [str],                                      # plain strings
          collections: [int],
        }
    """
    fields: dict[str, Any] = {
        k: v for k, v in item_json.items()
        if k not in _FLAT_NON_FIELD_KEYS and v is not None and v != ""
    }
    payload: dict[str, Any] = {
        "itemType": item_json.get("itemType", "journalArticle"),
        "fields": fields,
    }
    creators = item_json.get("creators")
    if creators:
        payload["creators"] = [
            {
                "firstName": c.get("firstName", ""),
                "lastName": c.get("lastName", ""),
                "creatorType": c.get("creatorType", "author"),
            }
            for c in creators
        ]
    tags = item_json.get("tags")
    if tags:
        payload["tags"] = [
            t["tag"] if isinstance(t, dict) else t
            for t in tags
        ]
    return payload


@dataclass
class PushResult:
    """Outcome of a single push_item() call."""
    status: Literal["created", "updated", "skipped_duplicate", "failed"]
    zotero_item_id: int | None = None
    pdf_attached: bool = False
    pdf_size_bytes: int = 0
    error: dict | None = None

    @property
    def pdf_size_kb(self) -> int:
        return self.pdf_size_bytes // 1024


def push_item(
    rpc: Any,
    item_json: dict,
    pdf_path: Path | None = None,
    collection: str | int | None = None,
    on_duplicate: Literal["skip", "update", "create"] = "skip",
    library: str | int | None = None,
) -> PushResult:
    """Push a Zotero-formatted item JSON to the library, optionally with a
    PDF attachment, optionally into a specific collection, with duplicate
    handling.

    Preconditions:
      * `item_json` has at minimum `itemType` and `title`.
      * If `pdf_path` is given, the file must start with %PDF- magic bytes
        (InvalidPDF raised otherwise — never uploads garbage).

    Flow:
      1. PDF magic-byte validation (if pdf_path given)
      2. collection = resolve_collection(...)  (0 = library root, skip addItems)
      3. dup_id = find_duplicate(...)
         - if dup_id and on_duplicate=='skip':    return skipped_duplicate
         - if dup_id and on_duplicate=='update':  items.update(dup_id, ...)
         - else (create):                          items.create(item_json)
      4. If pdf_path: attachments.add(parentID=item_id, path=...)
      5. If collection > 0: collections.addItems(collection, [item_id])

    Raises:
      InvalidPDF: if pdf_path exists but is not a PDF.
      CollectionAmbiguous / CollectionNotFound: from resolve_collection.
    """
    pdf_size = 0
    if pdf_path is not None:
        if not check_pdf_magic(pdf_path):
            raise InvalidPDF(f"{pdf_path} does not start with %PDF- magic bytes")
        pdf_size = pdf_path.stat().st_size

    collection_id = resolve_collection(rpc, collection, library=library)

    dup_id = find_duplicate(rpc, item_json)
    if dup_id is not None and on_duplicate == "skip":
        # A Zotero item can belong to multiple collections simultaneously
        # (collections behave like tags, not folders). When we find the item
        # already exists and skip re-creation, we still ensure it's linked
        # into the user-requested target collection — otherwise the same
        # paper can't appear in a second collection via this CLI.
        # collections.addItems is idempotent on the XPI side.
        if collection_id > 0:
            rpc.call("collections.addItems", {
                "id": collection_id,
                "itemIds": [dup_id],
            })
        # If we have a PDF and the dup has none, attach ours. Covers the
        # realistic workflow: first push was `--no-pdf`, now the user wants
        # to add the PDF without using `--on-duplicate=update` (which would
        # also rewrite metadata).
        skipped_pdf_attached = False
        if pdf_path is not None:
            if not _item_has_pdf_attachment(rpc, dup_id):
                _attach_pdf(rpc, dup_id, pdf_path)
                skipped_pdf_attached = True
        return PushResult(
            status="skipped_duplicate",
            zotero_item_id=dup_id,
            pdf_attached=skipped_pdf_attached,
            pdf_size_bytes=pdf_size if skipped_pdf_attached else 0,
        )

    xpi_payload = _to_xpi_payload(item_json)
    if collection_id > 0:
        xpi_payload["collections"] = [collection_id]

    if dup_id is not None and on_duplicate == "update":
        # XPI ≥ 0.3 items.update accepts fields + creators + tags (tags do a
        # full replace). Collections are applied separately below via
        # collections.addItems so existing collection memberships aren't
        # clobbered (additive, not replace).
        update_params: dict[str, Any] = {
            "id": dup_id,
            "fields": xpi_payload["fields"],
        }
        if "creators" in xpi_payload:
            update_params["creators"] = xpi_payload["creators"]
        if "tags" in xpi_payload:
            update_params["tags"] = xpi_payload["tags"]
        rpc.call("items.update", update_params)
        item_id = dup_id
        status: Literal["updated", "created"] = "updated"
    else:
        created = rpc.call("items.create", xpi_payload)
        if not created or "id" not in created:
            return PushResult(status="failed", error={
                "code": "CREATE_FAILED",
                "message": f"items.create returned unexpected shape: {created!r}",
            })
        item_id = int(created["id"])
        status = "created"

    pdf_attached = False
    if pdf_path is not None:
        if status == "updated":
            if not _item_has_pdf_attachment(rpc, item_id):
                _attach_pdf(rpc, item_id, pdf_path)
                pdf_attached = True
        else:
            _attach_pdf(rpc, item_id, pdf_path)
            pdf_attached = True

    # For `update` path, collections weren't applied by items.update; add them now.
    # For `create`, they were embedded in the create payload so re-calling is unnecessary.
    if status == "updated" and collection_id > 0:
        rpc.call("collections.addItems", {
            "id": collection_id,
            "itemIds": [item_id],
        })

    return PushResult(
        status=status,
        zotero_item_id=item_id,
        pdf_attached=pdf_attached,
        pdf_size_bytes=pdf_size,
    )
