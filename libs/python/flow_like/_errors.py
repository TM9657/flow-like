from __future__ import annotations


class FlowLikeError(Exception):
    pass


class AuthenticationError(FlowLikeError):
    pass


class ConfigurationError(FlowLikeError):
    pass


class APIError(FlowLikeError):
    def __init__(self, status_code: int, message: str, response_body: str | None = None):
        self.status_code = status_code
        self.response_body = response_body
        super().__init__(f"HTTP {status_code}: {message}")


class NotFoundError(APIError):
    pass


class RateLimitError(APIError):
    pass


class ServerError(APIError):
    pass


__all__ = [
    "FlowLikeError",
    "AuthenticationError",
    "ConfigurationError",
    "APIError",
    "NotFoundError",
    "RateLimitError",
    "ServerError",
]
