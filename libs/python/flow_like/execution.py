from __future__ import annotations

from typing import Any

from ._http import HTTPClient
from ._types import PollResult, RunStatus


class ExecutionMixin(HTTPClient):
    def get_run_status(self, run_id: str) -> RunStatus:
        resp = self._request("GET", f"/execution/run/{run_id}")
        data = resp.json()
        return RunStatus(
            run_id=data.get("run_id", run_id),
            status=data.get("status", "unknown"),
            result=data.get("result"),
            error=data.get("error"),
            raw=data,
        )

    async def aget_run_status(self, run_id: str) -> RunStatus:
        resp = await self._arequest("GET", f"/execution/run/{run_id}")
        data = resp.json()
        return RunStatus(
            run_id=data.get("run_id", run_id),
            status=data.get("status", "unknown"),
            result=data.get("result"),
            error=data.get("error"),
            raw=data,
        )

    def poll_execution(
        self,
        poll_token: str,
        after_sequence: int = 0,
        timeout: int = 30,
        **kwargs: Any,
    ) -> PollResult:
        resp = self._request(
            "GET",
            "/execution/poll",
            params={
                "poll_token": poll_token,
                "after_sequence": after_sequence,
                "timeout": timeout,
            },
            timeout=float(timeout + 5),
            **kwargs,
        )
        data = resp.json()
        return PollResult(
            events=data.get("events", []),
            done=data.get("done", False),
            raw=data,
        )

    async def apoll_execution(
        self,
        poll_token: str,
        after_sequence: int = 0,
        timeout: int = 30,
        **kwargs: Any,
    ) -> PollResult:
        resp = await self._arequest(
            "GET",
            "/execution/poll",
            params={
                "poll_token": poll_token,
                "after_sequence": after_sequence,
                "timeout": timeout,
            },
            timeout=float(timeout + 5),
            **kwargs,
        )
        data = resp.json()
        return PollResult(
            events=data.get("events", []),
            done=data.get("done", False),
            raw=data,
        )


__all__ = ["ExecutionMixin"]
