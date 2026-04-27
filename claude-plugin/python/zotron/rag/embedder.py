"""Embedding backends for RAG pipeline."""
from __future__ import annotations

from abc import ABC, abstractmethod
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from typing import Literal

import httpx

EmbeddingRole = Literal["query", "document"]
RequestStyle = Literal["openai-compatible", "jina"]


@dataclass(frozen=True)
class EmbeddingProviderSpec:
    """Declarative embedding provider behavior.

    ``embed`` is query-time by convention and ``embed_batch`` is document-time
    indexing. Most OpenAI-compatible providers use identical payloads for both
    roles; role-aware providers such as Jina require explicit task markers so
    query/document vectors remain compatible.
    """

    provider: str
    default_url: str
    request_style: RequestStyle = "openai-compatible"
    query_task: str | None = None
    document_task: str | None = None


BUILTIN_EMBEDDING_SPECS: dict[str, EmbeddingProviderSpec] = {
    "openai": EmbeddingProviderSpec(
        provider="openai",
        default_url="https://api.openai.com/v1/embeddings",
    ),
    "zhipu": EmbeddingProviderSpec(
        provider="zhipu",
        default_url="https://open.bigmodel.cn/api/paas/v4/embeddings",
    ),
    "dashscope": EmbeddingProviderSpec(
        provider="dashscope",
        default_url="https://dashscope.aliyuncs.com/compatible-mode/v1/embeddings",
    ),
    "jina": EmbeddingProviderSpec(
        provider="jina",
        default_url="https://api.jina.ai/v1/embeddings",
        request_style="jina",
        query_task="retrieval.query",
        document_task="retrieval.passage",
    ),
    "doubao": EmbeddingProviderSpec(
        provider="doubao",
        default_url="https://ark.cn-beijing.volces.com/api/v3/embeddings/multimodal",
    ),
}

_CLOUD_URLS = {
    provider: spec.default_url
    for provider, spec in BUILTIN_EMBEDDING_SPECS.items()
}


class Embedder(ABC):
    @abstractmethod
    def embed(self, text: str) -> list[float]:
        ...

    @abstractmethod
    def embed_batch(self, texts: list[str]) -> list[list[float]]:
        ...


class OllamaEmbedder(Embedder):
    def __init__(self, model: str, api_url: str, client: httpx.Client | None = None):
        self.model = model
        self.api_url = api_url.rstrip("/")
        self._client = client or httpx.Client()

    def embed(self, text: str) -> list[float]:
        resp = self._client.post(
            f"{self.api_url}/api/embeddings",
            json={"model": self.model, "prompt": text},
        )
        resp.raise_for_status()
        return resp.json()["embedding"]

    def embed_batch(self, texts: list[str]) -> list[list[float]]:
        return [self.embed(t) for t in texts]


class CloudEmbedder(Embedder):
    def __init__(
        self,
        provider: str,
        model: str,
        api_key: str,
        api_url: str | None = None,
        client: httpx.Client | None = None,
    ):
        spec = BUILTIN_EMBEDDING_SPECS.get(provider)
        self.model = model
        self._spec = spec or EmbeddingProviderSpec(
            provider=provider,
            default_url=api_url or "",
        )
        self._url = api_url or self._spec.default_url
        self._headers = {"Authorization": f"Bearer {api_key}"}
        self._client = client or httpx.Client()

    def _payload(self, input_value: str | list[str], role: EmbeddingRole) -> dict:
        payload: dict = {"model": self.model, "input": input_value}
        if self._spec.request_style == "jina":
            task = (
                self._spec.query_task if role == "query"
                else self._spec.document_task
            )
            if task:
                payload["task"] = task
        return payload

    def embed(self, text: str) -> list[float]:
        resp = self._client.post(
            self._url,
            json=self._payload(text, "query"),
            headers=self._headers,
        )
        resp.raise_for_status()
        return resp.json()["data"][0]["embedding"]

    def embed_batch(self, texts: list[str]) -> list[list[float]]:
        resp = self._client.post(
            self._url,
            json=self._payload(texts, "document"),
            headers=self._headers,
        )
        resp.raise_for_status()
        return [item["embedding"] for item in resp.json()["data"]]


class DoubaoMultimodalEmbedder(Embedder):
    """豆包多模态 embedding with instructions and concurrent batch.

    Uses the doubao-embedding-vision multimodal API.
    - Query (search): uses retrieval-oriented instruction
    - Corpus (index): uses compression instruction
    - embed_batch uses ThreadPoolExecutor for concurrent requests
    """

    # Instruction templates per doubao docs
    _QUERY_INSTRUCTION = (
        "Target_modality: text.\n"
        "Instruction:为这个句子生成表示以用于检索相关文章\n"
        "Query:"
    )
    _CORPUS_INSTRUCTION = (
        "Instruction:Compress the text into one word.\n"
        "Query:"
    )

    def __init__(
        self,
        model: str,
        api_key: str,
        api_url: str | None = None,
        concurrency: int = 8,
    ):
        self.model = model
        self._url = api_url or _CLOUD_URLS["doubao"]
        self._headers = {"Authorization": f"Bearer {api_key}"}
        self._concurrency = concurrency

    def _call_api(self, text: str, instruction: str | None = None) -> list[float]:
        import time as _time

        payload: dict = {
            "model": self.model,
            "input": [{"type": "text", "text": text}],
        }
        if instruction:
            payload["instructions"] = instruction
        for attempt in range(3):
            try:
                with httpx.Client() as client:
                    resp = client.post(
                        self._url, json=payload, headers=self._headers,
                        timeout=60.0,
                    )
                    resp.raise_for_status()
                data = resp.json()["data"]
                if isinstance(data, list):
                    return data[0]["embedding"]
                return data["embedding"]
            except (httpx.ConnectError, httpx.ReadError, httpx.RemoteProtocolError):
                if attempt < 2:
                    _time.sleep(1.0 * (attempt + 1))
                else:
                    raise
        raise RuntimeError("embedding API retry loop exited unexpectedly")

    def embed(self, text: str) -> list[float]:
        """Embed a query text (search-time) with query instruction."""
        return self._call_api(text, self._QUERY_INSTRUCTION)

    def embed_batch(self, texts: list[str]) -> list[list[float]]:
        """Embed corpus texts concurrently with corpus instruction."""
        with ThreadPoolExecutor(max_workers=self._concurrency) as pool:
            futures = [
                pool.submit(self._call_api, t, self._CORPUS_INSTRUCTION)
                for t in texts
            ]
            return [f.result() for f in futures]


def create_embedder(
    provider: str,
    model: str,
    api_key: str | None = None,
    api_url: str | None = None,
) -> Embedder:
    if provider == "ollama":
        url = api_url or "http://localhost:11434"
        return OllamaEmbedder(model=model, api_url=url)
    if provider == "doubao":
        return DoubaoMultimodalEmbedder(
            model=model, api_key=api_key or "", api_url=api_url,
        )
    if provider in _CLOUD_URLS or api_url:
        if provider not in _CLOUD_URLS and api_url is None:
            raise ValueError(f"Unknown provider: {provider!r}")
        return CloudEmbedder(provider=provider, model=model, api_key=api_key or "", api_url=api_url)
    raise ValueError(f"Unknown provider: {provider!r}")
