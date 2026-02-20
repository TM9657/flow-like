"""
Flow-Like WASM Node Template — Main Node

This file is the starting point for your custom node.
Edit get_definition() to define your node's pins and run() to implement the logic.
"""

from sdk import (
    Context,
    ExecutionResult,
    NodeDefinition,
    PinDefinition,
    PinType,
)


def get_definition() -> NodeDefinition:
    """Define the node's interface: name, category, inputs, and outputs."""
    nd = NodeDefinition(
        name="my_custom_node_py",
        friendly_name="My Custom Node",
        description="A template WASM node that demonstrates basic functionality",
        category="Custom/WASM",
    )

    nd.add_pin(PinDefinition.input_exec("exec"))
    nd.add_pin(PinDefinition.input_pin("input_text", PinType.STRING, default=""))
    nd.add_pin(PinDefinition.input_pin("multiplier", PinType.I64, default=1))

    nd.add_pin(PinDefinition.output_exec("exec_out"))
    nd.add_pin(PinDefinition.output_pin("output_text", PinType.STRING))
    nd.add_pin(PinDefinition.output_pin("char_count", PinType.I64))

    nd.add_permission("streaming")

    return nd


def run(ctx: Context) -> ExecutionResult:
    """Execute the node logic."""
    input_text = ctx.get_string("input_text", "")
    multiplier = ctx.get_i64("multiplier", 1)

    ctx.debug(f"Processing: '{input_text}' x {multiplier}")

    output_text = input_text * max(multiplier, 0)
    char_count = len(output_text)

    ctx.stream_text(f"Generated {char_count} characters")

    ctx.set_output("output_text", output_text)
    ctx.set_output("char_count", char_count)

    return ctx.success()
