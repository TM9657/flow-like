from __future__ import annotations

from .apps import AppsMixin
from .bits import BitsMixin
from .boards import BoardsMixin
from .chat import ChatMixin
from .database import DatabaseMixin
from .embeddings import EmbeddingsMixin
from .events import EventsMixin
from .execution import ExecutionMixin
from .files import FilesMixin
from .sinks import SinksMixin
from .workflows import WorkflowsMixin


class FlowLikeClient(
    WorkflowsMixin,
    EventsMixin,
    FilesMixin,
    DatabaseMixin,
    ExecutionMixin,
    SinksMixin,
    ChatMixin,
    EmbeddingsMixin,
    AppsMixin,
    BitsMixin,
    BoardsMixin,
):
    """Unified client for the Flow-Like API.

    Composes all feature mixins into a single client. Authentication and
    base URL can be provided explicitly or read from environment variables
    (FLOW_LIKE_PAT / FLOW_LIKE_API_KEY / FLOW_LIKE_BASE_URL).
    """

    def __init__(
        self,
        base_url: str | None = None,
        pat: str | None = None,
        api_key: str | None = None,
        token: str | None = None,
        timeout: float = 30.0,
    ):
        if token is not None:
            from ._auth import PAT_PREFIX, API_KEY_PREFIX

            if token.startswith(PAT_PREFIX):
                pat = token
            elif token.startswith(API_KEY_PREFIX):
                api_key = token
            else:
                from ._errors import AuthenticationError

                raise AuthenticationError(
                    f"Token must start with '{PAT_PREFIX}' or '{API_KEY_PREFIX}'."
                )

        super().__init__(base_url=base_url, pat=pat, api_key=api_key, timeout=timeout)

    def as_langchain_chat(
        self,
        bit_id: str,
        *,
        temperature: float | None = None,
        max_tokens: int | None = None,
        top_p: float | None = None,
    ):
        from .langchain import FlowLikeChatModel

        return FlowLikeChatModel(
            base_url=self._base_url,
            token=self._token,
            bit_id=bit_id,
            temperature=temperature,
            max_tokens=max_tokens,
            top_p=top_p,
        )

    def as_langchain_embeddings(self, bit_id: str):
        from .langchain import FlowLikeEmbeddings

        return FlowLikeEmbeddings(
            base_url=self._base_url,
            token=self._token,
            bit_id=bit_id,
        )


__all__ = ["FlowLikeClient"]
