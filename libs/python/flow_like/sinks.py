from __future__ import annotations

from typing import Any

from ._http import HTTPClient


class SinksMixin(HTTPClient):
    def trigger_http_sink(
        self,
        app_id: str,
        path: str,
        method: str = "POST",
        body: Any = None,
        **kwargs: Any,
    ) -> dict[str, Any]:
        resp = self._request(
            method.upper(),
            f"/sink/trigger/http/{app_id}/{path}",
            json=body,
            **kwargs,
        )
        return resp.json()

    async def atrigger_http_sink(
        self,
        app_id: str,
        path: str,
        method: str = "POST",
        body: Any = None,
        **kwargs: Any,
    ) -> dict[str, Any]:
        resp = await self._arequest(
            method.upper(),
            f"/sink/trigger/http/{app_id}/{path}",
            json=body,
            **kwargs,
        )
        return resp.json()


__all__ = ["SinksMixin"]
