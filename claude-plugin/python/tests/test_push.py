"""Tests for zotron.push.push_item."""
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from zotron.errors import InvalidPDF
from zotron.push import PushResult, push_item


def _make_rpc(item_key: str = "ABCD1234", *, duplicate: str | None = None,
              collections: list | None = None, library_editable: bool = True,
              dup_attachments: list | None = None):
    rpc = MagicMock()

    def call(method: str, params: dict | None = None):
        params = params or {}
        if method == "collections.list":
            return collections or []
        if method == "system.currentCollection":
            return {"key": None}
        if method == "search.byIdentifier":
            if duplicate is not None:
                return [{"key": duplicate}]
            return []
        if method == "search.quick":
            if duplicate is not None:
                return [{"key": duplicate, "title": params.get("q")}]
            return []
        if method == "items.create":
            return {"ok": True, "key": item_key, "version": 1}
        if method == "items.update":
            return {"ok": True, "key": item_key}
        if method == "attachments.add":
            return {"ok": True, "key": f"{item_key}-att"}
        if method == "attachments.list":
            return dup_attachments or []
        if method == "collections.addItems":
            return {"ok": True, "key": "COL1", "count": 1}
        if method == "system.libraries":
            return [{"id": 1, "type": "user", "name": "My Library",
                     "editable": library_editable}]
        return None

    rpc.call.side_effect = call
    return rpc


def _good_pdf(tmp_path: Path) -> Path:
    p = tmp_path / "paper.pdf"
    p.write_bytes(b"%PDF-1.7\n% fake content")
    return p


def test_create_metadata_only():
    rpc = _make_rpc(item_key="ITEM100")
    item = {"itemType": "journalArticle", "title": "My Title", "DOI": "10.x/y"}
    result = push_item(rpc, item, collection=0)
    assert isinstance(result, PushResult)
    assert result.status == "created"
    assert result.zotero_item_id == "ITEM100"
    assert result.pdf_attached is False


def test_create_with_pdf(tmp_path: Path):
    rpc = _make_rpc(item_key="ITEM200")
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Has PDF", "DOI": "10.x/z"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0)
    assert result.status == "created"
    assert result.pdf_attached is True
    assert result.pdf_size_bytes > 0


def test_skip_on_duplicate():
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, collection=0, on_duplicate="skip")
    assert result.status == "skipped_duplicate"
    assert result.zotero_item_id == "DUP999"


def test_skip_duplicate_still_links_to_target_collection():
    """When dup is found AND on_duplicate=skip AND a target collection is
    specified, the existing item must be linked into the target collection.
    Otherwise the CLI can't put the same paper in multiple collections —
    a standard Zotero workflow.
    """
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, collection=77, on_duplicate="skip")
    assert result.status == "skipped_duplicate"
    assert result.zotero_item_id == "DUP999"
    add_calls = [c for c in rpc.call.call_args_list
                 if c.args[0] == "collections.addItems"]
    assert len(add_calls) == 1
    assert add_calls[0].args[1] == {"key": 77, "keys": ["DUP999"]}


def test_skip_duplicate_no_collection_no_addItems():
    """collection=0 (library root) → no collections.addItems call."""
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    push_item(rpc, item, collection=0, on_duplicate="skip")
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "collections.addItems" not in methods


def test_skip_duplicate_attaches_pdf_if_dup_has_none(tmp_path: Path):
    """dup already in Zotero without a PDF attachment + new push has PDF →
    attach. Common workflow: first push was --no-pdf, second push fills in."""
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999", dup_attachments=[])
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="skip")
    assert result.status == "skipped_duplicate"
    assert result.pdf_attached is True
    add_calls = [c for c in rpc.call.call_args_list
                 if c.args[0] == "attachments.add"]
    assert len(add_calls) == 1
    assert add_calls[0].args[1]["parentKey"] == "DUP999"


def test_skip_duplicate_does_not_reattach_if_pdf_exists(tmp_path: Path):
    """dup already has a PDF attachment → don't attach a second copy."""
    existing_pdf = [{"key": "ATT1001", "contentType": "application/pdf",
                     "title": "Full Text PDF", "path": "/x/paper.pdf"}]
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999", dup_attachments=existing_pdf)
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="skip")
    assert result.status == "skipped_duplicate"
    assert result.pdf_attached is False
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "attachments.add" not in methods


def test_skip_duplicate_detects_pdf_via_filename_when_mime_is_wrong(tmp_path: Path):
    """Real-world Zotero stores some PDFs with contentType=octet-stream or
    blank (manual import, older builds). Detect via .pdf filename too."""
    existing_pdf = [{"key": "ATT1001", "contentType": "application/octet-stream",
                     "title": "paper", "path": "/storage/paper.PDF"}]
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999", dup_attachments=existing_pdf)
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="skip")
    assert result.pdf_attached is False  # detected existing PDF, no re-attach
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "attachments.add" not in methods


def test_skip_duplicate_detects_pdf_via_x_pdf_mime(tmp_path: Path):
    existing_pdf = [{"key": "ATT1001", "contentType": "application/x-pdf",
                     "title": "paper", "path": None}]
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999", dup_attachments=existing_pdf)
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="skip")
    assert result.pdf_attached is False
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "attachments.add" not in methods


def test_skip_duplicate_attaches_when_only_non_pdf_attachments(tmp_path: Path):
    """dup has attachments but none are PDFs (e.g., snapshot HTML) → do attach."""
    non_pdf = [{"key": "ATT1002", "contentType": "text/html",
                "title": "Snapshot", "path": "/storage/snap.html"}]
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999", dup_attachments=non_pdf)
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="skip")
    assert result.pdf_attached is True


def test_skip_duplicate_no_pdf_no_attachment_check(tmp_path: Path):
    """If caller didn't pass a PDF, we don't even check dup's attachments."""
    rpc = _make_rpc(item_key="ITEM500", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    push_item(rpc, item, pdf_path=None, collection=0, on_duplicate="skip")
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "attachments.list" not in methods
    assert "attachments.add" not in methods


def test_create_when_on_duplicate_create():
    rpc = _make_rpc(item_key="ITEM501", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, collection=0, on_duplicate="create")
    assert result.status == "created"
    assert result.zotero_item_id == "ITEM501"


def test_update_on_duplicate():
    rpc = _make_rpc(item_key="ITEM502", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, collection=0, on_duplicate="update")
    assert result.status == "updated"
    assert result.zotero_item_id == "DUP999"


def test_update_duplicate_does_not_reattach_if_pdf_exists(tmp_path: Path):
    existing_pdf = [{"key": "ATT1001", "contentType": "application/pdf",
                     "title": "Full Text PDF", "path": "/x/paper.pdf"}]
    rpc = _make_rpc(item_key="ITEM502", duplicate="DUP999", dup_attachments=existing_pdf)
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="update")
    assert result.status == "updated"
    assert result.zotero_item_id == "DUP999"
    assert result.pdf_attached is False
    assert result.pdf_size_bytes == 0
    methods = [c.args[0] for c in rpc.call.call_args_list]
    assert "items.update" in methods
    assert "attachments.list" in methods
    assert "attachments.add" not in methods


def test_update_duplicate_attaches_pdf_if_missing(tmp_path: Path):
    rpc = _make_rpc(item_key="ITEM502", duplicate="DUP999", dup_attachments=[])
    pdf = _good_pdf(tmp_path)
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    result = push_item(rpc, item, pdf_path=pdf, collection=0,
                       on_duplicate="update")
    assert result.status == "updated"
    assert result.zotero_item_id == "DUP999"
    assert result.pdf_attached is True
    assert result.pdf_size_bytes > 0
    add_calls = [c for c in rpc.call.call_args_list
                 if c.args[0] == "attachments.add"]
    assert len(add_calls) == 1
    assert add_calls[0].args[1]["parentKey"] == "DUP999"


def test_update_passes_creators_and_tags():
    """Update path must include creators + tags in the items.update call;
    without them the XPI would only refresh `fields` and silently drop
    authors/keywords on re-push — silent data loss."""
    rpc = _make_rpc(item_key="ITEM502", duplicate="DUP999")
    item = {
        "itemType": "journalArticle",
        "title": "Dup",
        "DOI": "10.x/dup",
        "creators": [
            {"creatorType": "author", "lastName": "陈明昊", "firstName": ""},
        ],
        "tags": [{"tag": "乡村振兴", "type": 1}, {"tag": "数字经济", "type": 1}],
    }
    push_item(rpc, item, collection=0, on_duplicate="update")
    update_calls = [c for c in rpc.call.call_args_list
                    if c.args[0] == "items.update"]
    assert len(update_calls) == 1
    payload = update_calls[0].args[1]
    assert payload["key"] == "DUP999"
    assert "fields" in payload
    assert payload["creators"] == [
        {"creatorType": "author", "lastName": "陈明昊", "firstName": ""},
    ]
    assert payload["tags"] == ["乡村振兴", "数字经济"]


def test_update_without_creators_or_tags_omits_them():
    """Don't send empty creators/tags arrays — respect the minimal payload."""
    rpc = _make_rpc(item_key="ITEM502", duplicate="DUP999")
    item = {"itemType": "journalArticle", "title": "Dup", "DOI": "10.x/dup"}
    push_item(rpc, item, collection=0, on_duplicate="update")
    update_calls = [c for c in rpc.call.call_args_list
                    if c.args[0] == "items.update"]
    assert len(update_calls) == 1
    payload = update_calls[0].args[1]
    assert "creators" not in payload
    assert "tags" not in payload


def test_invalid_pdf_raises(tmp_path: Path):
    rpc = _make_rpc(item_key="ITEM600")
    bad = tmp_path / "bad.pdf"
    bad.write_bytes(b"<html>error page</html>")
    item = {"itemType": "journalArticle", "title": "X", "DOI": "10.x/y"}
    with pytest.raises(InvalidPDF):
        push_item(rpc, item, pdf_path=bad, collection=0)


def test_collection_embedded_in_create_payload():
    """Collections are included in the items.create payload on the create path,
    so no separate collections.addItems call is needed."""
    rpc = _make_rpc(item_key="ITEM700")
    item = {"itemType": "journalArticle", "title": "Coll", "DOI": "10.x/coll"}
    push_item(rpc, item, collection=55)
    create_calls = [call for call in rpc.call.call_args_list
                    if call.args[0] == "items.create"]
    assert len(create_calls) == 1
    payload = create_calls[0].args[1]
    assert payload["collections"] == [55]


def test_collection_added_via_addItems_on_update_path():
    """On the update path, items.update only writes fields, so we need
    a separate collections.addItems call."""
    rpc = _make_rpc(item_key="ITEM800", duplicate="ITEM800")
    item = {"itemType": "journalArticle", "title": "U", "DOI": "10.x/u"}
    push_item(rpc, item, collection=55, on_duplicate="update")
    methods_called = [call.args[0] for call in rpc.call.call_args_list]
    assert "collections.addItems" in methods_called


def test_collection_zero_means_no_collections_embedded():
    rpc = _make_rpc(item_key="ITEM900")
    item = {"itemType": "journalArticle", "title": "Root", "DOI": "10.x/root"}
    push_item(rpc, item, collection=0)
    create_calls = [call for call in rpc.call.call_args_list
                    if call.args[0] == "items.create"]
    assert len(create_calls) == 1
    assert "collections" not in create_calls[0].args[1]


def test_wsl_path_translation_on_wsl(monkeypatch):
    """On WSL, _zotero_path should invoke wslpath and return the UNC form."""
    from zotron.push import _zotero_path
    monkeypatch.setenv("WSL_DISTRO_NAME", "Ubuntu-24.04")

    def fake_run(cmd, **kw):
        class R:
            returncode = 0
            stdout = "\\\\wsl.localhost\\Ubuntu-24.04\\tmp\\x.pdf"
            stderr = ""
        assert cmd[:2] == ["wslpath", "-w"]
        return R()

    monkeypatch.setattr("zotron.paths.subprocess.run", fake_run)
    assert _zotero_path(Path("/tmp/x.pdf")) == "\\\\wsl.localhost\\Ubuntu-24.04\\tmp\\x.pdf"


def test_path_passthrough_on_non_wsl(monkeypatch):
    from zotron.push import _zotero_path
    monkeypatch.setattr("zotron.paths.is_wsl", lambda: False)
    assert _zotero_path(Path("/tmp/x.pdf")) == "/tmp/x.pdf"
