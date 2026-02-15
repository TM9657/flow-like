from __future__ import annotations

from typing import IO, Any

from ._http import HTTPClient
from ._types import FileInfo, PresignResult


class FilesMixin(HTTPClient):
    def list_files(self, app_id: str, **kwargs: Any) -> list[FileInfo]:
        resp = self._request("GET", f"/apps/{app_id}/data/list", **kwargs)
        data = resp.json()
        items = data if isinstance(data, list) else data.get("files", [])
        return [
            FileInfo(
                key=f.get("key", ""),
                size=f.get("size"),
                last_modified=f.get("last_modified"),
                raw=f,
            )
            for f in items
        ]

    async def alist_files(self, app_id: str, **kwargs: Any) -> list[FileInfo]:
        resp = await self._arequest("GET", f"/apps/{app_id}/data/list", **kwargs)
        data = resp.json()
        items = data if isinstance(data, list) else data.get("files", [])
        return [
            FileInfo(
                key=f.get("key", ""),
                size=f.get("size"),
                last_modified=f.get("last_modified"),
                raw=f,
            )
            for f in items
        ]

    def upload_file(self, app_id: str, file: IO[bytes], **kwargs: Any) -> dict[str, Any]:
        filename = getattr(file, "name", "upload")
        resp = self._request(
            "POST",
            f"/apps/{app_id}/data/upload",
            files={"file": (filename, file)},
            **kwargs,
        )
        return resp.json()

    async def aupload_file(self, app_id: str, file: IO[bytes], **kwargs: Any) -> dict[str, Any]:
        filename = getattr(file, "name", "upload")
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/data/upload",
            files={"file": (filename, file)},
            **kwargs,
        )
        return resp.json()

    def download_file(self, app_id: str, key: str, **kwargs: Any) -> bytes:
        resp = self._request(
            "POST",
            f"/apps/{app_id}/data/download",
            json={"key": key},
            **kwargs,
        )
        return resp.content

    async def adownload_file(self, app_id: str, key: str, **kwargs: Any) -> bytes:
        resp = await self._arequest(
            "POST",
            f"/apps/{app_id}/data/download",
            json={"key": key},
            **kwargs,
        )
        return resp.content

    def delete_file(self, app_id: str, key: str) -> None:
        self._request("DELETE", f"/apps/{app_id}/data/delete", json={"key": key})

    async def adelete_file(self, app_id: str, key: str) -> None:
        await self._arequest("DELETE", f"/apps/{app_id}/data/delete", json={"key": key})

    def presign_data(self, app_id: str, **kwargs: Any) -> PresignResult:
        resp = self._request("POST", f"/apps/{app_id}/data/presign", **kwargs)
        data = resp.json()
        return PresignResult(
            url=data.get("url", ""),
            headers=data.get("headers", {}),
            raw=data,
        )

    async def apresign_data(self, app_id: str, **kwargs: Any) -> PresignResult:
        resp = await self._arequest("POST", f"/apps/{app_id}/data/presign", **kwargs)
        data = resp.json()
        return PresignResult(
            url=data.get("url", ""),
            headers=data.get("headers", {}),
            raw=data,
        )


__all__ = ["FilesMixin"]
