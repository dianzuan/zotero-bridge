from zotron.rag.search import VectorStore


def make_store() -> VectorStore:
    return VectorStore(collection="TestCol", collection_id=1, model="test-model")


def test_save_and_load(tmp_path):
    store = make_store()
    store.add_chunk("item1", "Title A", "Author A", "intro", 0, "text one", [1.0, 0.0])
    store.add_chunk("item2", "Title B", "Author B", "method", 0, "text two", [0.0, 1.0])

    path = tmp_path / "store.json"
    store.save(path)

    loaded = VectorStore.load(path)
    assert loaded.collection == "TestCol"
    assert loaded.collection_id == 1
    assert loaded.model == "test-model"
    assert len(loaded.chunks) == 2
    assert loaded.chunks[0]["item_id"] == "item1"
    assert loaded.chunks[1]["text"] == "text two"
    assert loaded.chunks[0]["vector"] == [1.0, 0.0]


def test_search_cosine_similarity():
    store = make_store()
    # chunk 0: close to query [1, 0, 0]
    store.add_chunk("i1", "T1", "A1", "s1", 0, "close", [1.0, 0.0, 0.0])
    # chunk 1: orthogonal
    store.add_chunk("i2", "T2", "A2", "s2", 0, "ortho", [0.0, 1.0, 0.0])
    # chunk 2: opposite
    store.add_chunk("i3", "T3", "A3", "s3", 0, "far", [-1.0, 0.0, 0.0])

    results = store.search([1.0, 0.0, 0.0], top_k=3)
    assert len(results) == 3
    assert results[0]["item_id"] == "i1"
    assert results[0]["score"] > results[1]["score"]
    assert results[1]["score"] > results[2]["score"]


def test_search_top_k():
    store = make_store()
    for i in range(20):
        store.add_chunk(f"item{i}", f"T{i}", f"A{i}", "s", i, f"text {i}", [float(i), 1.0])

    results = store.search([1.0, 0.0], top_k=5)
    assert len(results) == 5


def test_search_empty_store():
    store = make_store()
    results = store.search([1.0, 0.0, 0.0])
    assert results == []


def test_clear_item():
    store = make_store()
    store.add_chunk("item1", "T1", "A1", "s1", 0, "chunk 1a", [1.0, 0.0])
    store.add_chunk("item1", "T1", "A1", "s1", 1, "chunk 1b", [0.5, 0.5])
    store.add_chunk("item2", "T2", "A2", "s2", 0, "chunk 2a", [0.0, 1.0])

    store.clear_item("item1")

    assert len(store.chunks) == 1
    assert store.chunks[0]["item_id"] == "item2"


def test_search_includes_attachment_id_and_chunk_index():
    """Search results expose attachment_id and chunk_index for citation provenance."""
    from zotron.rag.search import VectorStore
    store = VectorStore(collection="test", collection_id=1, model="m")
    store.add_chunk(
        item_id="ITEM_A",
        title="Title A",
        authors="Author A",
        section="Intro",
        chunk_index=3,
        text="alpha beta",
        vector=[1.0, 0.0],
        attachment_id=99,
    )
    results = store.search([1.0, 0.0], top_k=1)
    assert len(results) == 1
    r = results[0]
    assert r["attachment_id"] == 99
    assert r["chunk_index"] == 3
    assert r["item_id"] == "ITEM_A"


def test_search_results_include_academic_zh_search_hit_contract():
    store = make_store()
    store.add_chunk(
        item_id="ITEM_A",
        title="产业贸易中心性、贸易外向度与金融风险",
        authors="王姝黛; 杨子荣",
        section="三、研究设计",
        chunk_index=42,
        text="本文利用世界投入产出表和金融风险指标...",
        vector=[1.0, 0.0],
        attachment_id=99,
        attachment_key="ATT1",
        chunk_id="ATT1:c42",
        block_ids=["ATT1:p12:b08", "ATT1:p12:b09"],
        year=2022,
        venue="中国工业经济",
        doi="10/example",
    )

    hit = store.search([1.0, 0.0], top_k=1, query="贸易中心性 金融风险")[0]

    assert hit["item_key"] == "ITEM_A"
    assert hit["title"] == "产业贸易中心性、贸易外向度与金融风险"
    assert hit["text"] == "本文利用世界投入产出表和金融风险指标..."
    assert hit["chunk_id"] == "ATT1:c42"
    assert hit["block_ids"] == ["ATT1:p12:b08", "ATT1:p12:b09"]
    assert hit["section_heading"] == "三、研究设计"
    assert hit["query"] == "贸易中心性 金融风险"
    assert hit["year"] == 2022
    # Existing search/cite compatibility remains intact.
    assert hit["item_id"] == "ITEM_A"
    assert hit["section"] == "三、研究设计"
    assert hit["chunk_index"] == 42


def test_artifact_backed_store_loads_chunks_and_embedding_npz(tmp_path):
    """TDD: Zotero-attached chunk + embedding artifacts become searchable spans."""
    import numpy as np

    from zotron.artifacts import metadata_for_chunks, write_chunks_jsonl, write_embedding_npz
    from zotron.rag.search import ArtifactBackedVectorStore

    chunks = [
        {
            "chunk_id": "ATT1:c000001",
            "block_ids": ["ATT1:p1:b01"],
            "section_heading": "研究设计",
            "text": "金融风险与贸易中心性相关的证据。",
            "page_start": 1,
            "page_end": 1,
        },
        {
            "chunk_id": "ATT1:c000002",
            "block_ids": ["ATT1:p2:b01"],
            "section_heading": "稳健性",
            "text": "安慰剂检验结果保持一致。",
            "page_start": 2,
            "page_end": 2,
        },
    ]
    chunks_path = write_chunks_jsonl(tmp_path, "ITEM1", chunks)
    embeddings_path = write_embedding_npz(
        tmp_path,
        "ITEM1",
        np.array([[1.0, 0.0], [0.0, 1.0]], dtype=np.float32),
        metadata_for_chunks(chunks),
        model="test-embedding",
    )

    store = ArtifactBackedVectorStore.from_item_artifacts(
        collection="Artifacts",
        collection_id=7,
        item_key="ITEM1",
        chunks_path=chunks_path,
        embeddings_path=embeddings_path,
        item_metadata={
            "item_id": "101",
            "title": "产业贸易中心性、贸易外向度与金融风险",
            "authors": ["王姝黛", "杨子荣"],
            "year": 2022,
            "venue": "中国工业经济",
            "zotero_uri": "zotero://select/library/items/ITEM1",
        },
    )

    hits = store.search_hits([1.0, 0.0], query="贸易中心性 金融风险", top_k=1)

    assert store.model == "test-embedding"
    assert hits == [
        {
            "item_key": "ITEM1",
            "title": "产业贸易中心性、贸易外向度与金融风险",
            "text": "金融风险与贸易中心性相关的证据。",
            "authors": ["王姝黛", "杨子荣"],
            "year": 2022,
            "venue": "中国工业经济",
            "zotero_uri": "zotero://select/library/items/ITEM1",
            "section_heading": "研究设计",
            "chunk_id": "ATT1:c000001",
            "block_ids": ["ATT1:p1:b01"],
            "query": "贸易中心性 金融风险",
            "score": 1.0,
        }
    ]


def test_artifact_backed_store_rejects_stale_embedding_metadata(tmp_path):
    import numpy as np
    import pytest

    from zotron.artifacts import metadata_for_chunks, write_chunks_jsonl, write_embedding_npz
    from zotron.rag.search import ArtifactBackedVectorStore

    chunks = [{"chunk_id": "c1", "text": "current text", "section_heading": "Intro"}]
    stale_metadata = metadata_for_chunks([{"chunk_id": "c1", "text": "old text"}])
    chunks_path = write_chunks_jsonl(tmp_path, "ITEM1", chunks)
    embeddings_path = write_embedding_npz(
        tmp_path,
        "ITEM1",
        np.array([[1.0, 0.0]], dtype=np.float32),
        stale_metadata,
        model="test-embedding",
    )

    with pytest.raises(ValueError, match="stale embedding metadata"):
        ArtifactBackedVectorStore.from_item_artifacts(
            collection="Artifacts",
            collection_id=7,
            item_key="ITEM1",
            chunks_path=chunks_path,
            embeddings_path=embeddings_path,
        )
