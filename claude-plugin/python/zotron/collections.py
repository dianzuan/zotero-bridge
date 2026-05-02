"""Collection-tree helpers.

`resolve_collection` (in push.py) handles the write-path — it uses the
flat `collections.list` and raises on ambiguity. For read paths that
traverse the nested hierarchy (OCR batch, RAG index), use
`find_by_name` below: it calls `collections.tree` and returns the first
match by name, or None.
"""

from __future__ import annotations

from typing import Any


def find_by_name(rpc: Any, name: str) -> int | str | None:
    """Return the key of the first collection named *name*, or None.

    Traverses the full `collections.tree` recursively. Returns the first
    match found in depth-first order; does not disambiguate. Callers
    that need ambiguity detection should use `resolve_collection`.
    """
    tree = rpc.call("collections.tree") or []
    return _search_tree(tree, name)


def _search_tree(nodes: list[dict], name: str) -> int | str | None:
    for node in nodes:
        if node.get("name") == name:
            return node.get("key")
        found = _search_tree(node.get("children") or [], name)
        if found is not None:
            return found
    return None
