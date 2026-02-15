from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Any

from ._http import HTTPClient
from ._types import AsyncInvokeResult, SSEEvent


class EventsMixin(HTTPClient):
    def trigger_event(
        self,
        app_id: str,
        event_id: str,
        payload: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> Iterator[SSEEvent]:
        return self._stream_sse(
            "POST",
            f"/apps/{app_id}/events/{event_id}/invoke",
            json=payload,
            **kwargs,
        )

    async def atrigger_event(
        self,
        app_id: str,
        event_id: str,
        payload: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> AsyncIterator[SSEEvent]:
        return self._astream_sse(
            "POST",
            f"/apps/{app_id}/events/{event_id}/invoke",
            json=payload,
            **kwargs,
        )

    def trigger_event_async(
        self,
        app_id: str,
        event_id: str,
        payload: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> AsyncInvokeResult:
        resp = self._request(
            "POST",
            f"/apps/{app_id}/events/{event_id}/invoke-async",
            json=payload,
            **kwargs,
        )
        data = resp.json()
        return AsyncInvokeResult(
            run_id=data.get("run_id", ""),
            poll_token=data.get("poll_token", ""),
            raw=data,
        )

    async def atrigger_event_async(
        self,
        app_id: str,
        event_id: str,
        payload: dict[str, Any] | None = None,
        **kwargs: Any,
    ) -> AsyncInvokeResult:
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/events/{event_id}/invoke-async",
            json=payload,
            **kwargs,
        )
        data = resp.json()
        return AsyncInvokeResult(
            run_id=data.get("run_id", ""),
            poll_token=data.get("poll_token", ""),
            raw=data,
        )


__all__ = ["EventsMixin"]
