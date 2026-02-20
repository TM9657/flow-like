"""Shared test fixtures for all node tests."""

import sys
from pathlib import Path

import pytest

# Add src and examples to path so tests can import them
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "src"))
sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "examples"))

from sdk import Context, ExecutionInput, MockHostBridge


@pytest.fixture
def host() -> MockHostBridge:
    return MockHostBridge()


def make_context(
    inputs: dict | None = None,
    *,
    host: MockHostBridge | None = None,
    node_name: str = "",
    stream: bool = False,
    log_level: int = 0,
) -> Context:
    """Helper to build a Context with the given inputs."""
    ei = ExecutionInput(
        inputs=inputs or {},
        node_id="test-node-id",
        run_id="test-run-id",
        app_id="test-app-id",
        board_id="test-board-id",
        user_id="test-user-id",
        stream_state=stream,
        log_level=log_level,
        node_name=node_name,
    )
    return Context(ei, host or MockHostBridge())
