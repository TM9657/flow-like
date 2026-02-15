from __future__ import annotations

from typing import Any

from ._http import HTTPClient
from ._types import Board, PrerunBoardResponse, UpsertBoardResponse


class BoardsMixin(HTTPClient):
    def list_boards(self, app_id: str) -> list[Board]:
        resp = self._request("GET", f"/apps/{app_id}/board")
        data = resp.json()
        items = data if isinstance(data, list) else []
        return [Board(id=b.get("id", ""), raw=b) for b in items]

    async def alist_boards(self, app_id: str) -> list[Board]:
        resp = await self._arequest("GET", f"/apps/{app_id}/board")
        data = resp.json()
        items = data if isinstance(data, list) else []
        return [Board(id=b.get("id", ""), raw=b) for b in items]

    def get_board(
        self, app_id: str, board_id: str, version: str | None = None
    ) -> Board:
        params = {"version": version} if version else None
        resp = self._request("GET", f"/apps/{app_id}/board/{board_id}", params=params)
        data = resp.json()
        return Board(id=data.get("id", board_id), raw=data)

    async def aget_board(
        self, app_id: str, board_id: str, version: str | None = None
    ) -> Board:
        params = {"version": version} if version else None
        resp = await self._arequest(
            "GET", f"/apps/{app_id}/board/{board_id}", params=params
        )
        data = resp.json()
        return Board(id=data.get("id", board_id), raw=data)

    def upsert_board(
        self,
        app_id: str,
        board_id: str,
        *,
        name: str | None = None,
        description: str | None = None,
        stage: str | None = None,
        log_level: str | None = None,
        execution_mode: str | None = None,
        template: dict[str, Any] | None = None,
    ) -> UpsertBoardResponse:
        body: dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if description is not None:
            body["description"] = description
        if stage is not None:
            body["stage"] = stage
        if log_level is not None:
            body["log_level"] = log_level
        if execution_mode is not None:
            body["execution_mode"] = execution_mode
        if template is not None:
            body["template"] = template
        resp = self._request("PUT", f"/apps/{app_id}/board/{board_id}", json=body)
        data = resp.json()
        return UpsertBoardResponse(id=data.get("id", board_id), raw=data)

    async def aupsert_board(
        self,
        app_id: str,
        board_id: str,
        *,
        name: str | None = None,
        description: str | None = None,
        stage: str | None = None,
        log_level: str | None = None,
        execution_mode: str | None = None,
        template: dict[str, Any] | None = None,
    ) -> UpsertBoardResponse:
        body: dict[str, Any] = {}
        if name is not None:
            body["name"] = name
        if description is not None:
            body["description"] = description
        if stage is not None:
            body["stage"] = stage
        if log_level is not None:
            body["log_level"] = log_level
        if execution_mode is not None:
            body["execution_mode"] = execution_mode
        if template is not None:
            body["template"] = template
        resp = await self._arequest(
            "PUT", f"/apps/{app_id}/board/{board_id}", json=body
        )
        data = resp.json()
        return UpsertBoardResponse(id=data.get("id", board_id), raw=data)

    def delete_board(self, app_id: str, board_id: str) -> None:
        self._request("DELETE", f"/apps/{app_id}/board/{board_id}")

    async def adelete_board(self, app_id: str, board_id: str) -> None:
        await self._arequest("DELETE", f"/apps/{app_id}/board/{board_id}")

    def prerun_board(
        self, app_id: str, board_id: str, version: str | None = None
    ) -> PrerunBoardResponse:
        params = {"version": version} if version else None
        resp = self._request(
            "GET", f"/apps/{app_id}/board/{board_id}/prerun", params=params
        )
        data = resp.json()
        return PrerunBoardResponse(
            runtime_variables=data.get("runtime_variables", []),
            oauth_requirements=data.get("oauth_requirements", []),
            requires_local_execution=data.get("requires_local_execution", False),
            execution_mode=data.get("execution_mode", ""),
            can_execute_locally=data.get("can_execute_locally", False),
            raw=data,
        )

    async def aprerun_board(
        self, app_id: str, board_id: str, version: str | None = None
    ) -> PrerunBoardResponse:
        params = {"version": version} if version else None
        resp = await self._arequest(
            "GET", f"/apps/{app_id}/board/{board_id}/prerun", params=params
        )
        data = resp.json()
        return PrerunBoardResponse(
            runtime_variables=data.get("runtime_variables", []),
            oauth_requirements=data.get("oauth_requirements", []),
            requires_local_execution=data.get("requires_local_execution", False),
            execution_mode=data.get("execution_mode", ""),
            can_execute_locally=data.get("can_execute_locally", False),
            raw=data,
        )

    def get_board_versions(self, app_id: str, board_id: str) -> list[dict[str, Any]]:
        resp = self._request("GET", f"/apps/{app_id}/board/{board_id}/version")
        data = resp.json()
        return data if isinstance(data, list) else []

    async def aget_board_versions(
        self, app_id: str, board_id: str
    ) -> list[dict[str, Any]]:
        resp = await self._arequest(
            "GET", f"/apps/{app_id}/board/{board_id}/version"
        )
        data = resp.json()
        return data if isinstance(data, list) else []

    def execute_commands(
        self, app_id: str, board_id: str, commands: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        resp = self._request(
            "POST",
            f"/apps/{app_id}/board/{board_id}",
            json={"commands": commands},
        )
        data = resp.json()
        return data if isinstance(data, list) else []

    async def aexecute_commands(
        self, app_id: str, board_id: str, commands: list[dict[str, Any]]
    ) -> list[dict[str, Any]]:
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/board/{board_id}",
            json={"commands": commands},
        )
        data = resp.json()
        return data if isinstance(data, list) else []


__all__ = ["BoardsMixin"]
