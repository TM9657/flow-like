from __future__ import annotations

from typing import Any, Literal

from ._http import HTTPClient
from ._types import EmbeddingResult


class EmbeddingsMixin(HTTPClient):
    def embed(
        self,
        bit_id: str,
        input: str | list[str],
        embed_type: Literal["query", "document"] = "query",
        **kwargs: Any,
    ) -> EmbeddingResult:
        texts = [input] if isinstance(input, str) else input
        payload: dict[str, Any] = {
            "model": bit_id,
            "input": texts,
            "embed_type": embed_type,
            **kwargs,
        }
        resp = self._request("POST", "/embeddings/embed", json=payload)
        data = resp.json()
        return EmbeddingResult(
            embeddings=data.get("embeddings", []),
            usage=data.get("usage", {}),
            raw=data,
        )

    async def aembed(
        self,
        bit_id: str,
        input: str | list[str],
        embed_type: Literal["query", "document"] = "query",
        **kwargs: Any,
    ) -> EmbeddingResult:
        texts = [input] if isinstance(input, str) else input
        payload: dict[str, Any] = {
            "model": bit_id,
            "input": texts,
            "embed_type": embed_type,
            **kwargs,
        }
        resp = await self._arequest("POST", "/embeddings/embed", json=payload)
        data = resp.json()
        return EmbeddingResult(
            embeddings=data.get("embeddings", []),
            usage=data.get("usage", {}),
            raw=data,
        )


__all__ = ["EmbeddingsMixin"]
