"""
HTTP Request Example — Demonstrates how to declare permissions and make HTTP requests

Permissions tell the runtime which host capabilities your node requires.
When users place the node, they see the requested permissions and must
consent before execution.

This example declares the "http" permission and makes a GET request
to a public API.
"""

from sdk import (
    Context,
    ExecutionResult,
    NodeDefinition,
    PinDefinition,
    PinType,
)

# ============================================================================
# Node definition — note `nd.add_permission("http")`
# ============================================================================


def get_definitions() -> list[NodeDefinition]:
    nd = NodeDefinition(
        "http_request_example_py",
        "HTTP Request Example",
        "Fetches data from a public API using HTTP",
        "Examples/HTTP",
    )

    nd.add_permission("http")

    nd.add_pin(PinDefinition.input_exec("exec"))
    nd.add_pin(
        PinDefinition.input_pin(
            "url",
            PinType.STRING,
            default="https://httpbin.org/get",
        )
    )
    nd.add_pin(PinDefinition.output_exec("exec_out"))
    nd.add_pin(PinDefinition.output_pin("status", PinType.I64))
    nd.add_pin(PinDefinition.output_pin("body", PinType.STRING))

    return [nd]


# ============================================================================
# Run handler — makes an HTTP GET request
# ============================================================================


def _run_http_request(ctx: Context) -> ExecutionResult:
    url = ctx.get_string("url", "https://httpbin.org/get")

    ctx.info(f"Making GET request to: {url}")

    response = ctx.http_get(url)
    if response is None:
        return ctx.fail("HTTP request failed or permission denied")

    ctx.set_output("status", response.get("status", 0))
    ctx.set_output("body", response.get("body", ""))
    ctx.info(f"Response status: {response.get('status')}")

    return ctx.success()


DISPATCH = {"http_request_example_py": _run_http_request}


def run(node_name: str, ctx: Context) -> ExecutionResult:
    handler = DISPATCH.get(node_name)
    if handler is None:
        return ctx.fail(f"Unknown node: {node_name}")
    return handler(ctx)
