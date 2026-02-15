from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:
    from lancedb.db import LanceDBConnection

from ._http import HTTPClient
from ._types import (
    CountResult,
    LanceConnectionInfo,
    PresignDbAccessResponse,
    QueryResult,
    TableSchema,
)


def _parse_presign_response(data: dict[str, Any]) -> PresignDbAccessResponse:
    return PresignDbAccessResponse(
        shared_credentials=data.get("shared_credentials", {}),
        db_path=data.get("db_path", ""),
        table_name=data.get("table_name", ""),
        access_mode=data.get("access_mode", "read"),
        expiration=data.get("expiration"),
        raw=data,
    )


def _resolve_connection_info(resp: PresignDbAccessResponse) -> LanceConnectionInfo:
    creds = resp.shared_credentials

    if "Aws" in creds:
        raw = creds["Aws"]
        cfg_raw = raw.get("content_config") or {}
        uri = f"s3://{raw['content_bucket']}/{resp.db_path}"
        opts: dict[str, str] = {}
        if raw.get("access_key_id"):
            opts["aws_access_key_id"] = raw["access_key_id"]
        if raw.get("secret_access_key"):
            opts["aws_secret_access_key"] = raw["secret_access_key"]
        if raw.get("session_token"):
            opts["aws_session_token"] = raw["session_token"]
        if raw.get("region"):
            opts["aws_region"] = raw["region"]
        if cfg_raw.get("endpoint"):
            opts["aws_endpoint"] = cfg_raw["endpoint"]
        return LanceConnectionInfo(uri=uri, storage_options=opts)

    if "Azure" in creds:
        raw = creds["Azure"]
        uri = f"az://{raw['content_container']}/{resp.db_path}"
        opts = {"azure_storage_account_name": raw["account_name"]}
        if raw.get("content_sas_token"):
            opts["azure_storage_sas_token"] = raw["content_sas_token"]
        if raw.get("account_key"):
            opts["azure_storage_account_key"] = raw["account_key"]
        return LanceConnectionInfo(uri=uri, storage_options=opts)

    if "Gcp" in creds:
        raw = creds["Gcp"]
        uri = f"gs://{raw['content_bucket']}/{resp.db_path}"
        opts = {}
        if raw.get("access_token"):
            opts["google_cloud_token"] = raw["access_token"]
        elif raw.get("service_account_key"):
            opts["google_service_account_key"] = raw["service_account_key"]
        return LanceConnectionInfo(uri=uri, storage_options=opts)

    raise ValueError(f"Unknown shared credentials provider: {list(creds.keys())}")


class DatabaseMixin(HTTPClient):
    def get_db_credentials(
        self,
        app_id: str,
        table_name: str = "_default",
        access_mode: str = "read",
    ) -> LanceConnectionInfo:
        resp = self._request(
            "POST",
            f"/apps/{app_id}/db/presign",
            json={"table_name": table_name, "access_mode": access_mode},
        )
        return _resolve_connection_info(_parse_presign_response(resp.json()))

    async def aget_db_credentials(
        self,
        app_id: str,
        table_name: str = "_default",
        access_mode: str = "read",
    ) -> LanceConnectionInfo:
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/db/presign",
            json={"table_name": table_name, "access_mode": access_mode},
        )
        return _resolve_connection_info(_parse_presign_response(resp.json()))

    def get_db_credentials_raw(
        self,
        app_id: str,
        table_name: str = "_default",
        access_mode: str = "read",
    ) -> PresignDbAccessResponse:
        resp = self._request(
            "POST",
            f"/apps/{app_id}/db/presign",
            json={"table_name": table_name, "access_mode": access_mode},
        )
        return _parse_presign_response(resp.json())

    async def aget_db_credentials_raw(
        self,
        app_id: str,
        table_name: str = "_default",
        access_mode: str = "read",
    ) -> PresignDbAccessResponse:
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/db/presign",
            json={"table_name": table_name, "access_mode": access_mode},
        )
        return _parse_presign_response(resp.json())

    def create_lance_connection(
        self, app_id: str, access_mode: str = "read"
    ) -> LanceDBConnection:
        try:
            import lancedb
        except ImportError as e:
            raise ImportError(
                "lancedb is required for create_lance_connection. "
                "Install it with: uv add flow-like[lance]"
            ) from e

        info = self.get_db_credentials(app_id, access_mode=access_mode)
        return lancedb.connect(info.uri, storage_options=info.storage_options)

    async def acreate_lance_connection(
        self, app_id: str, access_mode: str = "read"
    ) -> LanceDBConnection:
        try:
            import lancedb
        except ImportError as e:
            raise ImportError(
                "lancedb is required for acreate_lance_connection. "
                "Install it with: uv add flow-like[lance]"
            ) from e

        info = await self.aget_db_credentials(app_id, access_mode=access_mode)
        return lancedb.connect(info.uri, storage_options=info.storage_options)

    def list_tables(self, app_id: str) -> list[str]:
        resp = self._request("GET", f"/apps/{app_id}/db/tables")
        data = resp.json()
        return data if isinstance(data, list) else data.get("tables", [])

    async def alist_tables(self, app_id: str) -> list[str]:
        resp = await self._arequest("GET", f"/apps/{app_id}/db/tables")
        data = resp.json()
        return data if isinstance(data, list) else data.get("tables", [])

    def get_table_schema(self, app_id: str, table: str) -> TableSchema:
        resp = self._request("GET", f"/apps/{app_id}/db/{table}/schema")
        data: dict[str, Any] = resp.json()
        return TableSchema(
            name=data.get("name", table),
            columns=data.get("columns", []),
            raw=data,
        )

    async def aget_table_schema(self, app_id: str, table: str) -> TableSchema:
        resp = await self._arequest("GET", f"/apps/{app_id}/db/{table}/schema")
        data: dict[str, Any] = resp.json()
        return TableSchema(
            name=data.get("name", table),
            columns=data.get("columns", []),
            raw=data,
        )

    def query_table(self, app_id: str, table: str, query: dict[str, Any]) -> QueryResult:
        resp = self._request("POST", f"/apps/{app_id}/db/{table}/query", json=query)
        data: Any = resp.json()
        if isinstance(data, list):
            return QueryResult(rows=data, raw={"rows": data})
        rows: list[dict[str, Any]] = data.get("rows", []) if isinstance(data, dict) else []
        return QueryResult(rows=rows, raw=data if isinstance(data, dict) else {"value": data})

    async def aquery_table(self, app_id: str, table: str, query: dict[str, Any]) -> QueryResult:
        resp = await self._arequest("POST", f"/apps/{app_id}/db/{table}/query", json=query)
        data: Any = resp.json()
        if isinstance(data, list):
            return QueryResult(rows=data, raw={"rows": data})
        rows: list[dict[str, Any]] = data.get("rows", []) if isinstance(data, dict) else []
        return QueryResult(rows=rows, raw=data if isinstance(data, dict) else {"value": data})

    def add_to_table(self, app_id: str, table: str, data: list[dict[str, Any]]) -> dict[str, Any]:
        resp = self._request("POST", f"/apps/{app_id}/db/{table}/add", json=data)
        return resp.json()

    async def aadd_to_table(
        self, app_id: str, table: str, data: list[dict[str, Any]]
    ) -> dict[str, Any]:
        resp = await self._arequest("POST", f"/apps/{app_id}/db/{table}/add", json=data)
        return resp.json()

    def delete_from_table(self, app_id: str, table: str, filter: dict[str, Any]) -> None:
        self._request("DELETE", f"/apps/{app_id}/db/{table}/delete", json=filter)

    async def adelete_from_table(self, app_id: str, table: str, filter: dict[str, Any]) -> None:
        await self._arequest("DELETE", f"/apps/{app_id}/db/{table}/delete", json=filter)

    def count_items(self, app_id: str, table: str) -> CountResult:
        resp = self._request("GET", f"/apps/{app_id}/db/{table}/count")
        data = resp.json()
        return CountResult(count=data.get("count", 0), raw=data)

    async def acount_items(self, app_id: str, table: str) -> CountResult:
        resp = await self._arequest("GET", f"/apps/{app_id}/db/{table}/count")
        data = resp.json()
        return CountResult(count=data.get("count", 0), raw=data)


__all__ = ["DatabaseMixin"]
