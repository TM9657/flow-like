from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass
class AsyncInvokeResult:
    run_id: str
    poll_token: str
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class RunStatus:
    run_id: str
    status: str
    result: Any = None
    error: str | None = None
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class PollResult:
    events: list[dict[str, Any]] = field(default_factory=list)
    done: bool = False
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class SSEEvent:
    event: str | None = None
    data: str = ""
    id: str | None = None
    retry: int | None = None


@dataclass
class FileInfo:
    key: str
    size: int | None = None
    last_modified: str | None = None
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class PresignResult:
    url: str
    headers: dict[str, str] = field(default_factory=dict)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class BucketConfig:
    endpoint: str | None = None
    express: bool = False


@dataclass
class AwsSharedCredentials:
    meta_bucket: str = ""
    content_bucket: str = ""
    logs_bucket: str = ""
    region: str = ""
    access_key_id: str | None = None
    secret_access_key: str | None = None
    session_token: str | None = None
    meta_config: BucketConfig | None = None
    content_config: BucketConfig | None = None
    logs_config: BucketConfig | None = None
    expiration: str | None = None
    content_path_prefix: str | None = None
    user_content_path_prefix: str | None = None


@dataclass
class AzureSharedCredentials:
    meta_container: str = ""
    content_container: str = ""
    logs_container: str = ""
    account_name: str = ""
    meta_sas_token: str | None = None
    content_sas_token: str | None = None
    user_content_sas_token: str | None = None
    logs_sas_token: str | None = None
    account_key: str | None = None
    expiration: str | None = None
    content_path_prefix: str | None = None
    user_content_path_prefix: str | None = None


@dataclass
class GcpSharedCredentials:
    service_account_key: str = ""
    meta_bucket: str = ""
    content_bucket: str = ""
    logs_bucket: str = ""
    allowed_prefixes: list[str] = field(default_factory=list)
    write_access: bool = False
    access_token: str | None = None
    expiration: str | None = None
    content_path_prefix: str | None = None
    user_content_path_prefix: str | None = None


@dataclass
class PresignDbAccessResponse:
    shared_credentials: dict[str, Any] = field(default_factory=dict)
    db_path: str = ""
    table_name: str = ""
    access_mode: str = "read"
    expiration: str | None = None
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class LanceConnectionInfo:
    uri: str = ""
    storage_options: dict[str, str] = field(default_factory=dict)


@dataclass
class TableSchema:
    name: str
    columns: list[dict[str, Any]] = field(default_factory=list)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class QueryResult:
    rows: list[dict[str, Any]] = field(default_factory=list)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class CountResult:
    count: int
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class App:
    id: str
    name: str | None = None
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class HealthStatus:
    healthy: bool
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class ChatMessage:
    role: str
    content: str


@dataclass
class ChatCompletionResult:
    choices: list[dict[str, Any]] = field(default_factory=list)
    usage: dict[str, Any] = field(default_factory=dict)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class EmbeddingResult:
    embeddings: list[list[float]] = field(default_factory=list)
    usage: dict[str, Any] = field(default_factory=dict)
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class ModelInfo:
    bit_id: str
    name: str
    description: str = ""
    provider_name: str | None = None
    model_id: str | None = None
    context_length: int | None = None
    vector_length: int | None = None
    languages: list[str] = field(default_factory=list)
    tags: list[str] = field(default_factory=list)


@dataclass
class Board:
    id: str
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class UpsertBoardResponse:
    id: str
    raw: dict[str, Any] = field(default_factory=dict)


@dataclass
class PrerunBoardResponse:
    runtime_variables: list[dict[str, Any]] = field(default_factory=list)
    oauth_requirements: list[dict[str, Any]] = field(default_factory=list)
    requires_local_execution: bool = False
    execution_mode: str = ""
    can_execute_locally: bool = False
    raw: dict[str, Any] = field(default_factory=dict)


__all__ = [
    "AsyncInvokeResult",
    "RunStatus",
    "PollResult",
    "SSEEvent",
    "FileInfo",
    "PresignResult",
    "BucketConfig",
    "AwsSharedCredentials",
    "AzureSharedCredentials",
    "GcpSharedCredentials",
    "PresignDbAccessResponse",
    "LanceConnectionInfo",
    "TableSchema",
    "QueryResult",
    "CountResult",
    "App",
    "HealthStatus",
    "ChatMessage",
    "ChatCompletionResult",
    "EmbeddingResult",
    "ModelInfo",
    "Board",
    "UpsertBoardResponse",
    "PrerunBoardResponse",
]
