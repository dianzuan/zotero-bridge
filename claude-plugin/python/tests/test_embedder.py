"""Tests for RAG embedding backends."""
import pytest
import httpx

from zotron.rag.embedder import (
    BUILTIN_EMBEDDING_SPECS,
    DoubaoMultimodalEmbedder,
    GeminiEmbedder,
    OllamaEmbedder,
    CloudEmbedder,
    create_embedder,
)


def _ollama_client(responses: list[dict]) -> httpx.Client:
    idx = 0

    def handler(request: httpx.Request) -> httpx.Response:
        nonlocal idx
        data = responses[idx] if idx < len(responses) else {}
        idx += 1
        return httpx.Response(200, json=data)

    return httpx.Client(transport=httpx.MockTransport(handler))


def _cloud_client(response: dict) -> httpx.Client:
    def handler(request: httpx.Request) -> httpx.Response:
        return httpx.Response(200, json=response)

    return httpx.Client(transport=httpx.MockTransport(handler))


# --- factory ---

def test_create_ollama_embedder():
    emb = create_embedder("ollama", "nomic-embed-text")
    assert isinstance(emb, OllamaEmbedder)
    assert emb.model == "nomic-embed-text"


def test_create_cloud_embedder():
    emb = create_embedder("zhipu", "embedding-3", api_key="key123")
    assert isinstance(emb, CloudEmbedder)
    assert emb.model == "embedding-3"


def test_create_jina_embedder_from_builtin_registry():
    emb = create_embedder("jina", "jina-embeddings-v3", api_key="key123")
    assert isinstance(emb, CloudEmbedder)
    assert emb.model == "jina-embeddings-v3"
    assert BUILTIN_EMBEDDING_SPECS["jina"].query_task == "retrieval.query"
    assert BUILTIN_EMBEDDING_SPECS["jina"].document_task == "retrieval.passage"


def test_create_new_embedding_providers_from_registry():
    for provider, model in [
        ("siliconflow", "BAAI/bge-m3"),
        ("voyage", "voyage-4"),
        ("cohere", "embed-v4.0"),
    ]:
        emb = create_embedder(provider, model, api_key="key123")
        assert isinstance(emb, CloudEmbedder)
        assert emb.model == model

    gemini = create_embedder("gemini", "gemini-embedding-001", api_key="key123")
    assert isinstance(gemini, GeminiEmbedder)


def test_create_doubao_embedder_uses_multimodal_adapter():
    emb = create_embedder("doubao", "doubao-embedding-vision-251215", api_key="key123")
    assert isinstance(emb, DoubaoMultimodalEmbedder)
    assert emb.model == "doubao-embedding-vision-251215"


def test_create_unknown_embedder():
    with pytest.raises(ValueError, match="Unknown provider"):
        create_embedder("nonexistent", "some-model")


# --- OllamaEmbedder ---

def test_ollama_embed():
    client = _ollama_client([{"embedding": [0.1, 0.2, 0.3]}])
    emb = OllamaEmbedder(model="nomic-embed-text", api_url="http://localhost:11434", client=client)
    result = emb.embed("hello world")
    assert result == [0.1, 0.2, 0.3]


def test_ollama_embed_batch():
    client = _ollama_client([
        {"embedding": [0.1, 0.2, 0.3]},
        {"embedding": [0.4, 0.5, 0.6]},
    ])
    emb = OllamaEmbedder(model="nomic-embed-text", api_url="http://localhost:11434", client=client)
    results = emb.embed_batch(["foo", "bar"])
    assert results == [[0.1, 0.2, 0.3], [0.4, 0.5, 0.6]]


# --- CloudEmbedder ---

def test_cloud_embed():
    client = _cloud_client({"data": [{"embedding": [0.5, 0.6, 0.7]}]})
    emb = CloudEmbedder(
        provider="zhipu",
        model="embedding-3",
        api_key="key123",
        client=client,
    )
    result = emb.embed("test text")
    assert result == [0.5, 0.6, 0.7]


def test_role_aware_jina_query_and_document_payloads():
    requests: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(json_request(request))
        return httpx.Response(200, json={"data": [{"embedding": [0.1, 0.2]}]})

    client = httpx.Client(transport=httpx.MockTransport(handler))
    emb = CloudEmbedder(
        provider="jina",
        model="jina-embeddings-v3",
        api_key="key123",
        client=client,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert emb.embed_batch(["document text"]) == [[0.1, 0.2]]

    assert requests[0] == {
        "model": "jina-embeddings-v3",
        "input": "query text",
        "task": "retrieval.query",
    }
    assert requests[1] == {
        "model": "jina-embeddings-v3",
        "input": ["document text"],
        "task": "retrieval.passage",
    }


def test_openai_compatible_payload_remains_legacy_without_task_field():
    requests: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(json_request(request))
        return httpx.Response(200, json={"data": [{"embedding": [0.1, 0.2]}]})

    client = httpx.Client(transport=httpx.MockTransport(handler))
    emb = CloudEmbedder(
        provider="openai",
        model="text-embedding-3-small",
        api_key="key123",
        client=client,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert requests == [{"model": "text-embedding-3-small", "input": "query text"}]


def test_voyage_query_and_document_payloads():
    requests: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(json_request(request))
        return httpx.Response(200, json={"data": [{"embedding": [0.1, 0.2]}]})

    client = httpx.Client(transport=httpx.MockTransport(handler))
    emb = CloudEmbedder(
        provider="voyage",
        model="voyage-4",
        api_key="key123",
        client=client,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert emb.embed_batch(["document text"]) == [[0.1, 0.2]]

    assert requests[0] == {
        "model": "voyage-4",
        "input": "query text",
        "input_type": "query",
    }
    assert requests[1] == {
        "model": "voyage-4",
        "input": ["document text"],
        "input_type": "document",
    }


def test_cohere_query_and_document_payloads_and_response_shape():
    requests: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(json_request(request))
        return httpx.Response(200, json={"embeddings": {"float": [[0.1, 0.2]]}})

    client = httpx.Client(transport=httpx.MockTransport(handler))
    emb = CloudEmbedder(
        provider="cohere",
        model="embed-v4.0",
        api_key="key123",
        client=client,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert emb.embed_batch(["document text"]) == [[0.1, 0.2]]

    assert requests[0] == {
        "model": "embed-v4.0",
        "texts": ["query text"],
        "input_type": "search_query",
        "embedding_types": ["float"],
    }
    assert requests[1] == {
        "model": "embed-v4.0",
        "texts": ["document text"],
        "input_type": "search_document",
        "embedding_types": ["float"],
    }


def test_gemini_query_and_document_payloads_and_response_shape():
    requests: list[dict] = []

    def handler(request: httpx.Request) -> httpx.Response:
        requests.append(json_request(request))
        return httpx.Response(200, json={"embedding": {"values": [0.1, 0.2]}})

    client = httpx.Client(transport=httpx.MockTransport(handler))
    emb = GeminiEmbedder(
        model="gemini-embedding-001",
        api_key="key123",
        client=client,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert emb.embed_batch(["document text"]) == [[0.1, 0.2]]

    assert requests[0] == {
        "taskType": "RETRIEVAL_QUERY",
        "content": {"parts": [{"text": "query text"}]},
    }
    assert requests[1] == {
        "taskType": "RETRIEVAL_DOCUMENT",
        "content": {"parts": [{"text": "document text"}]},
    }


def test_doubao_query_and_document_payloads_and_response_shapes(monkeypatch):
    requests: list[dict] = []

    class FakeClient:
        def __enter__(self):
            return self

        def __exit__(self, exc_type, exc, tb):
            return False

        def post(self, url, *, json, headers, timeout):
            requests.append(
                {
                    "url": url,
                    "json": json,
                    "headers": headers,
                    "timeout": timeout,
                }
            )
            request = httpx.Request("POST", url)
            if len(requests) == 1:
                return httpx.Response(200, json={"data": [{"embedding": [0.1, 0.2]}]}, request=request)
            return httpx.Response(200, json={"data": {"embedding": [0.3, 0.4]}}, request=request)

    monkeypatch.setattr(httpx, "Client", FakeClient)
    emb = DoubaoMultimodalEmbedder(
        model="doubao-embedding-vision-251215",
        api_key="key123",
        api_url="https://example.test/embeddings",
        concurrency=1,
    )

    assert emb.embed("query text") == [0.1, 0.2]
    assert emb.embed_batch(["document text"]) == [[0.3, 0.4]]

    assert requests[0]["url"] == "https://example.test/embeddings"
    assert requests[0]["headers"] == {"Authorization": "Bearer key123"}
    assert requests[0]["timeout"] == 60.0
    assert requests[0]["json"]["model"] == "doubao-embedding-vision-251215"
    assert requests[0]["json"]["input"] == [{"type": "text", "text": "query text"}]
    assert "检索相关文章" in requests[0]["json"]["instructions"]

    assert requests[1]["json"]["model"] == "doubao-embedding-vision-251215"
    assert requests[1]["json"]["input"] == [{"type": "text", "text": "document text"}]
    assert "Compress the text" in requests[1]["json"]["instructions"]


def json_request(request: httpx.Request) -> dict:
    import json

    return json.loads(request.content.decode("utf-8"))
