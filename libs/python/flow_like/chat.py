from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Any

from ._http import HTTPClient
from ._types import ChatCompletionResult, ChatMessage, SSEEvent


def _build_messages(messages: list[ChatMessage | dict[str, str]]) -> list[dict[str, str]]:
    return [
        {"role": m.role, "content": m.content} if isinstance(m, ChatMessage) else m
        for m in messages
    ]


class ChatMixin(HTTPClient):
    def chat_completions(
        self,
        messages: list[ChatMessage | dict[str, str]],
        bit_id: str,
        stream: bool = False,
        **kwargs: Any,
    ) -> ChatCompletionResult | Iterator[SSEEvent]:
        payload: dict[str, Any] = {
            "messages": _build_messages(messages),
            "model": bit_id,
            "stream": stream,
            **kwargs,
        }
        if stream:
            return self._stream_sse("POST", "/chat/completions", json=payload)
        resp = self._request("POST", "/chat/completions", json=payload)
        data = resp.json()
        return ChatCompletionResult(
            choices=data.get("choices", []),
            usage=data.get("usage", {}),
            raw=data,
        )

    async def achat_completions(
        self,
        messages: list[ChatMessage | dict[str, str]],
        bit_id: str,
        stream: bool = False,
        **kwargs: Any,
    ) -> ChatCompletionResult | AsyncIterator[SSEEvent]:
        payload: dict[str, Any] = {
            "messages": _build_messages(messages),
            "model": bit_id,
            "stream": stream,
            **kwargs,
        }
        if stream:
            return self._astream_sse("POST", "/chat/completions", json=payload)
        resp = await self._arequest("POST", "/chat/completions", json=payload)
        data = resp.json()
        return ChatCompletionResult(
            choices=data.get("choices", []),
            usage=data.get("usage", {}),
            raw=data,
        )

    def get_usage(self) -> dict[str, Any]:
        resp = self._request("GET", "/chat/usage")
        return resp.json()

    async def aget_usage(self) -> dict[str, Any]:
        resp = await self._arequest("GET", "/chat/usage")
        return resp.json()


__all__ = ["ChatMixin"]
