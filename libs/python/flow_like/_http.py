from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Any

import httpx

from ._auth import resolve_auth, resolve_base_url
from ._errors import APIError, NotFoundError, RateLimitError, ServerError
from ._types import SSEEvent

DEFAULT_TIMEOUT = 30.0
SSE_TIMEOUT = 300.0


class HTTPClient:
    def __init__(
        self,
        base_url: str | None = None,
        pat: str | None = None,
        api_key: str | None = None,
        timeout: float = DEFAULT_TIMEOUT,
    ):
        self._base_url = resolve_base_url(base_url)
        self._api_base = f"{self._base_url}/api/v1"
        self._auth_headers = resolve_auth(pat=pat, api_key=api_key)
        self._token = self._auth_headers.get("Authorization") or self._auth_headers.get("X-API-Key", "")
        self._timeout = timeout
        self._client: httpx.Client | None = None
        self._async_client: httpx.AsyncClient | None = None

    def _get_client(self) -> httpx.Client:
        if self._client is None:
            self._client = httpx.Client(
                base_url=self._api_base,
                headers=self._auth_headers,
                timeout=self._timeout,
            )
        return self._client

    def _get_async_client(self) -> httpx.AsyncClient:
        if self._async_client is None:
            self._async_client = httpx.AsyncClient(
                base_url=self._api_base,
                headers=self._auth_headers,
                timeout=self._timeout,
            )
        return self._async_client

    def close(self) -> None:
        if self._client is not None:
            self._client.close()
            self._client = None
        if self._async_client is not None:
            import asyncio

            try:
                loop = asyncio.get_running_loop()
                loop.create_task(self._async_client.aclose())
            except RuntimeError:
                asyncio.run(self._async_client.aclose())
            self._async_client = None

    async def aclose(self) -> None:
        if self._client is not None:
            self._client.close()
            self._client = None
        if self._async_client is not None:
            await self._async_client.aclose()
            self._async_client = None

    def __enter__(self) -> HTTPClient:
        return self

    def __exit__(self, *args: Any) -> None:
        self.close()

    async def __aenter__(self) -> HTTPClient:
        return self

    async def __aexit__(self, *args: Any) -> None:
        await self.aclose()

    @staticmethod
    def _raise_for_status(response: httpx.Response) -> None:
        if response.is_success:
            return
        body = response.text
        msg = f"{response.reason_phrase}: {body}"
        if response.status_code == 404:
            raise NotFoundError(response.status_code, msg, body)
        if response.status_code == 429:
            raise RateLimitError(response.status_code, msg, body)
        if response.status_code >= 500:
            raise ServerError(response.status_code, msg, body)
        raise APIError(response.status_code, msg, body)

    def _request(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        data: Any = None,
        files: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
        timeout: float | None = None,
    ) -> httpx.Response:
        client = self._get_client()
        response = client.request(
            method,
            path,
            json=json,
            data=data,
            files=files,
            params=params,
            headers=headers,
            timeout=timeout or self._timeout,
        )
        self._raise_for_status(response)
        return response

    async def _arequest(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        data: Any = None,
        files: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
        timeout: float | None = None,
    ) -> httpx.Response:
        client = self._get_async_client()
        response = await client.request(
            method,
            path,
            json=json,
            data=data,
            files=files,
            params=params,
            headers=headers,
            timeout=timeout or self._timeout,
        )
        self._raise_for_status(response)
        return response

    def _stream_sse(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> Iterator[SSEEvent]:
        client = self._get_client()
        with client.stream(
            method,
            path,
            json=json,
            params=params,
            headers={**(headers or {}), "Accept": "text/event-stream"},
            timeout=SSE_TIMEOUT,
        ) as response:
            self._raise_for_status(response)
            yield from _parse_sse_stream(response.iter_lines())

    async def _astream_sse(
        self,
        method: str,
        path: str,
        *,
        json: Any = None,
        params: dict[str, Any] | None = None,
        headers: dict[str, str] | None = None,
    ) -> AsyncIterator[SSEEvent]:
        client = self._get_async_client()
        async with client.stream(
            method,
            path,
            json=json,
            params=params,
            headers={**(headers or {}), "Accept": "text/event-stream"},
            timeout=SSE_TIMEOUT,
        ) as response:
            self._raise_for_status(response)
            async for event in _parse_sse_stream_async(response.aiter_lines()):
                yield event


def _parse_sse_stream(lines: Iterator[str]) -> Iterator[SSEEvent]:
    event = SSEEvent()
    for line in lines:
        if line == "":
            if event.data:
                yield event
            event = SSEEvent()
            continue
        if line.startswith("event:"):
            event.event = line[len("event:"):].strip()
        elif line.startswith("data:"):
            event.data += line[len("data:"):].strip()
        elif line.startswith("id:"):
            event.id = line[len("id:"):].strip()
        elif line.startswith("retry:"):
            try:
                event.retry = int(line[len("retry:"):].strip())
            except ValueError:
                pass
    if event.data:
        yield event


async def _parse_sse_stream_async(lines: AsyncIterator[str]) -> AsyncIterator[SSEEvent]:
    event = SSEEvent()
    async for line in lines:
        if line == "":
            if event.data:
                yield event
            event = SSEEvent()
            continue
        if line.startswith("event:"):
            event.event = line[len("event:"):].strip()
        elif line.startswith("data:"):
            event.data += line[len("data:"):].strip()
        elif line.startswith("id:"):
            event.id = line[len("id:"):].strip()
        elif line.startswith("retry:"):
            try:
                event.retry = int(line[len("retry:"):].strip())
            except ValueError:
                pass
    if event.data:
        yield event


__all__ = ["HTTPClient"]
