from __future__ import annotations

from typing import Any, Optional

import httpx
from langchain_core.callbacks import CallbackManagerForLLMRun
from langchain_core.embeddings import Embeddings
from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage, BaseMessage
from langchain_core.outputs import ChatGeneration, ChatResult

_ROLE_MAP = {
    "human": "user",
    "ai": "assistant",
    "system": "system",
    "tool": "tool",
}


def _build_headers(token: str) -> dict[str, str]:
    headers = {"Content-Type": "application/json"}
    if token.startswith("pat_"):
        headers["Authorization"] = token
    else:
        headers["X-API-Key"] = token
    return headers


def _to_api_message(msg: BaseMessage) -> dict[str, str]:
    return {
        "role": _ROLE_MAP.get(msg.type, "user"),
        "content": str(msg.content),
    }


class FlowLikeChatModel(BaseChatModel):
    base_url: str
    token: str
    bit_id: str
    temperature: Optional[float] = None
    max_tokens: Optional[int] = None
    top_p: Optional[float] = None

    @property
    def _llm_type(self) -> str:
        return "flow-like"

    def _generate(
        self,
        messages: list[BaseMessage],
        stop: list[str] | None = None,
        run_manager: CallbackManagerForLLMRun | None = None,
        **kwargs: Any,
    ) -> ChatResult:
        body: dict[str, Any] = {
            "messages": [_to_api_message(m) for m in messages],
            "model": self.bit_id,
        }
        if self.temperature is not None:
            body["temperature"] = self.temperature
        if self.max_tokens is not None:
            body["max_tokens"] = self.max_tokens
        if self.top_p is not None:
            body["top_p"] = self.top_p
        if stop:
            body["stop"] = stop

        resp = httpx.post(
            f"{self.base_url}/api/v1/chat/completions",
            headers=_build_headers(self.token),
            json=body,
            timeout=120,
        )
        resp.raise_for_status()
        data = resp.json()

        text = data["choices"][0]["message"]["content"]
        return ChatResult(generations=[ChatGeneration(message=AIMessage(content=text))])


class FlowLikeEmbeddings(Embeddings):
    def __init__(self, *, base_url: str, token: str, bit_id: str):
        super().__init__()
        self.base_url = base_url
        self.token = token
        self.bit_id = bit_id

    def _embed(self, texts: list[str], embed_type: str) -> list[list[float]]:
        resp = httpx.post(
            f"{self.base_url}/api/v1/embeddings/embed",
            headers=_build_headers(self.token),
            json={"model": self.bit_id, "input": texts, "embed_type": embed_type},
            timeout=120,
        )
        resp.raise_for_status()
        return resp.json()["embeddings"]

    def embed_documents(self, texts: list[str]) -> list[list[float]]:
        return self._embed(texts, "document")

    def embed_query(self, text: str) -> list[float]:
        return self._embed([text], "query")[0]

    async def _aembed(self, texts: list[str], embed_type: str) -> list[list[float]]:
        async with httpx.AsyncClient() as client:
            resp = await client.post(
                f"{self.base_url}/api/v1/embeddings/embed",
                headers=_build_headers(self.token),
                json={"model": self.bit_id, "input": texts, "embed_type": embed_type},
                timeout=120,
            )
            resp.raise_for_status()
            return resp.json()["embeddings"]

    async def aembed_documents(self, texts: list[str]) -> list[list[float]]:
        return await self._aembed(texts, "document")

    async def aembed_query(self, text: str) -> list[float]:
        return (await self._aembed([text], "query"))[0]
