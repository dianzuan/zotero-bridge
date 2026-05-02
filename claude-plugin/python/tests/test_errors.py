"""Tests for zotron.errors."""
from zotron.errors import (
    ZotronError,
    ZoteroUnavailable,
    CollectionNotFound,
    CollectionAmbiguous,
    InvalidPDF,
)


def test_all_subclass_base():
    for cls in (
        ZoteroUnavailable, CollectionNotFound, CollectionAmbiguous, InvalidPDF,
    ):
        assert issubclass(cls, ZotronError)


def test_collection_ambiguous_carries_candidates():
    err = CollectionAmbiguous("ambiguous", candidates=[{"key": "COL1", "name": "A"}, {"key": "COL2", "name": "B"}])
    assert err.candidates == [{"key": "COL1", "name": "A"}, {"key": "COL2", "name": "B"}]
