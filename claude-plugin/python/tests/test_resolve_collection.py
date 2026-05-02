"""Tests for zotron.push.resolve_collection."""
from unittest.mock import MagicMock

import pytest

from zotron.errors import CollectionAmbiguous, CollectionNotFound
from zotron.push import resolve_collection


def _rpc_with(collections: list[dict], selected: dict | None = None) -> MagicMock:
    """Build a mock RPC whose collections.list returns `collections` and
    whose system.currentCollection returns `selected`."""
    rpc = MagicMock()

    def call(method: str, params: dict | None = None):
        params = params or {}
        if method == "collections.list":
            return collections
        if method == "system.currentCollection":
            return selected
        raise ValueError(f"unexpected method: {method}")

    rpc.call.side_effect = call
    return rpc


def test_int_returned_as_is():
    rpc = _rpc_with([])
    assert resolve_collection(rpc, 42) == 42


def test_str_numeric_converted():
    rpc = _rpc_with([])
    assert resolve_collection(rpc, "7") == 7


def test_exact_name_match():
    rpc = _rpc_with([
        {"key": "COL10", "name": "Research"},
        {"key": "COL11", "name": "研究生"},
    ])
    assert resolve_collection(rpc, "研究生") == "COL11"


def test_fuzzy_case_insensitive():
    rpc = _rpc_with([
        {"key": "COL10", "name": "Research Papers"},
    ])
    assert resolve_collection(rpc, "research papers") == "COL10"


def test_fuzzy_whitespace_normalized():
    rpc = _rpc_with([
        {"key": "COL10", "name": "My  Papers"},   # double space
    ])
    assert resolve_collection(rpc, "my papers") == "COL10"


def test_ambiguous_raises():
    rpc = _rpc_with([
        {"key": "COL10", "name": "Papers 2024"},
        {"key": "COL11", "name": "Papers 2025"},
    ])
    with pytest.raises(CollectionAmbiguous) as excinfo:
        resolve_collection(rpc, "papers")
    assert len(excinfo.value.candidates) == 2


def test_not_found_raises():
    rpc = _rpc_with([{"key": "COL10", "name": "Research"}])
    with pytest.raises(CollectionNotFound):
        resolve_collection(rpc, "nonexistent")


def test_none_with_gui_selection():
    rpc = _rpc_with([], selected={"key": "COL99", "name": "Current"})
    assert resolve_collection(rpc, None) == "COL99"


def test_none_fallback_to_library_root():
    """When nothing is selected in GUI, return 0 (sentinel meaning 'library root')."""
    rpc = _rpc_with([], selected=None)
    assert resolve_collection(rpc, None) == 0


def test_none_when_method_not_found_falls_back_to_library_root():
    """XPI build without system.currentCollection must not crash the resolver."""
    rpc = MagicMock()
    rpc.call.side_effect = RuntimeError("[-32601] Method not found: system.currentCollection")
    assert resolve_collection(rpc, None) == 0
