from __future__ import annotations

from collections.abc import AsyncIterator, Iterator
from typing import Any

from ._http import HTTPClient
from ._types import AsyncInvokeResult, SSEEvent


class WorkflowsMixin(HTTPClient):
    def trigger_workflow(
        self,
        app_id: str,
        board_id: str,
        node_id: str,
        payload: Any = None,
        *,
        stream_state: bool = True,
        version: tuple[int, int, int] | None = None,
        runtime_variables: dict[str, Any] | None = None,
        profile_id: str | None = None,
        **kwargs: Any,
    ) -> Iterator[SSEEvent]:
        body: dict[str, Any] = {"node_id": node_id, "stream_state": stream_state}
        if payload is not None:
            body["payload"] = payload
        if version is not None:
            body["version"] = list(version)
        if runtime_variables is not None:
            body["runtime_variables"] = runtime_variables
        if profile_id is not None:
            body["profile_id"] = profile_id
        return self._stream_sse(
            "POST",
            f"/apps/{app_id}/board/{board_id}/invoke",
            json=body,
            **kwargs,
        )

    async def atrigger_workflow(
        self,
        app_id: str,
        board_id: str,
        node_id: str,
        payload: Any = None,
        *,
        stream_state: bool = True,
        version: tuple[int, int, int] | None = None,
        runtime_variables: dict[str, Any] | None = None,
        profile_id: str | None = None,
        **kwargs: Any,
    ) -> AsyncIterator[SSEEvent]:
        body: dict[str, Any] = {"node_id": node_id, "stream_state": stream_state}
        if payload is not None:
            body["payload"] = payload
        if version is not None:
            body["version"] = list(version)
        if runtime_variables is not None:
            body["runtime_variables"] = runtime_variables
        if profile_id is not None:
            body["profile_id"] = profile_id
        return self._astream_sse(
            "POST",
            f"/apps/{app_id}/board/{board_id}/invoke",
            json=body,
            **kwargs,
        )

    def trigger_workflow_async(
        self,
        app_id: str,
        board_id: str,
        node_id: str,
        payload: Any = None,
        *,
        version: tuple[int, int, int] | None = None,
        runtime_variables: dict[str, Any] | None = None,
        profile_id: str | None = None,
        **kwargs: Any,
    ) -> AsyncInvokeResult:
        body: dict[str, Any] = {"node_id": node_id}
        if payload is not None:
            body["payload"] = payload
        if version is not None:
            body["version"] = list(version)
        if runtime_variables is not None:
            body["runtime_variables"] = runtime_variables
        if profile_id is not None:
            body["profile_id"] = profile_id
        resp = self._request(
            "POST",
            f"/apps/{app_id}/board/{board_id}/invoke/async",
            json=body,
            **kwargs,
        )
        data = resp.json()
        return AsyncInvokeResult(
            run_id=data.get("run_id", ""),
            poll_token=data.get("poll_token", ""),
            raw=data,
        )

    async def atrigger_workflow_async(
        self,
        app_id: str,
        board_id: str,
        node_id: str,
        payload: Any = None,
        *,
        version: tuple[int, int, int] | None = None,
        runtime_variables: dict[str, Any] | None = None,
        profile_id: str | None = None,
        **kwargs: Any,
    ) -> AsyncInvokeResult:
        body: dict[str, Any] = {"node_id": node_id}
        if payload is not None:
            body["payload"] = payload
        if version is not None:
            body["version"] = list(version)
        if runtime_variables is not None:
            body["runtime_variables"] = runtime_variables
        if profile_id is not None:
            body["profile_id"] = profile_id
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/board/{board_id}/invoke/async",
            json=body,
            **kwargs,
        )
        data = resp.json()
        return AsyncInvokeResult(
            run_id=data.get("run_id", ""),
            poll_token=data.get("poll_token", ""),
            raw=data,
        )


__all__ = ["WorkflowsMixin"]
