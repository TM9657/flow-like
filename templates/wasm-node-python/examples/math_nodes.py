"""
Math Nodes — Basic arithmetic and mathematical operations

Demonstrates creating multiple nodes for add, subtract, multiply,
divide, power, and clamp operations.
"""

from sdk import (
    Context,
    ExecutionResult,
    NodeDefinition,
    PinDefinition,
    PinType,
)


def _base_math_node(name: str, friendly_name: str, description: str) -> NodeDefinition:
    nd = NodeDefinition(name, friendly_name, description, "Math/Arithmetic")
    nd.add_pin(PinDefinition.input_exec("exec"))
    nd.add_pin(PinDefinition.input_pin("a", PinType.F64, default=0.0))
    nd.add_pin(PinDefinition.input_pin("b", PinType.F64, default=0.0))
    nd.add_pin(PinDefinition.output_exec("exec_out"))
    nd.add_pin(PinDefinition.output_pin("result", PinType.F64))
    return nd


def get_definitions() -> list[NodeDefinition]:
    nodes: list[NodeDefinition] = []

    nodes.append(_base_math_node("math_add_py", "Add", "Adds two numbers together"))
    nodes.append(_base_math_node("math_subtract_py", "Subtract", "Subtracts B from A"))
    nodes.append(_base_math_node("math_multiply_py", "Multiply", "Multiplies two numbers"))

    # Divide has an extra output
    divide = NodeDefinition("math_divide_py", "Divide", "Divides A by B", "Math/Arithmetic")
    divide.add_pin(PinDefinition.input_exec("exec"))
    divide.add_pin(PinDefinition.input_pin("a", PinType.F64, default=0.0))
    divide.add_pin(PinDefinition.input_pin("b", PinType.F64, default=1.0))
    divide.add_pin(PinDefinition.output_exec("exec_out"))
    divide.add_pin(PinDefinition.output_pin("result", PinType.F64))
    divide.add_pin(PinDefinition.output_pin("is_valid", PinType.BOOL))
    nodes.append(divide)

    # Power
    power = NodeDefinition("math_power_py", "Power", "Raises A to the power of B", "Math/Arithmetic")
    power.add_pin(PinDefinition.input_exec("exec"))
    power.add_pin(PinDefinition.input_pin("base", PinType.F64, default=0.0))
    power.add_pin(PinDefinition.input_pin("exponent", PinType.F64, default=1.0))
    power.add_pin(PinDefinition.output_exec("exec_out"))
    power.add_pin(PinDefinition.output_pin("result", PinType.F64))
    nodes.append(power)

    # Clamp
    clamp = NodeDefinition("math_clamp_py", "Clamp", "Clamps a value between min and max", "Math/Utility")
    clamp.add_pin(PinDefinition.input_exec("exec"))
    clamp.add_pin(PinDefinition.input_pin("value", PinType.F64, default=0.0))
    clamp.add_pin(PinDefinition.input_pin("min", PinType.F64, default=0.0))
    clamp.add_pin(PinDefinition.input_pin("max", PinType.F64, default=1.0))
    clamp.add_pin(PinDefinition.output_exec("exec_out"))
    clamp.add_pin(PinDefinition.output_pin("result", PinType.F64))
    nodes.append(clamp)

    return nodes


def run_add(ctx: Context) -> ExecutionResult:
    a = ctx.get_f64("a", 0.0)
    b = ctx.get_f64("b", 0.0)
    ctx.set_output("result", a + b)
    return ctx.success()


def run_subtract(ctx: Context) -> ExecutionResult:
    a = ctx.get_f64("a", 0.0)
    b = ctx.get_f64("b", 0.0)
    ctx.set_output("result", a - b)
    return ctx.success()


def run_multiply(ctx: Context) -> ExecutionResult:
    a = ctx.get_f64("a", 0.0)
    b = ctx.get_f64("b", 0.0)
    ctx.set_output("result", a * b)
    return ctx.success()


def run_divide(ctx: Context) -> ExecutionResult:
    a = ctx.get_f64("a", 0.0)
    b = ctx.get_f64("b", 1.0)

    if b == 0.0:
        ctx.set_output("result", 0.0)
        ctx.set_output("is_valid", False)
        ctx.warn("Division by zero")
    else:
        ctx.set_output("result", a / b)
        ctx.set_output("is_valid", True)

    return ctx.success()


def run_power(ctx: Context) -> ExecutionResult:
    base = ctx.get_f64("base", 0.0)
    exponent = ctx.get_f64("exponent", 1.0)
    ctx.set_output("result", base**exponent)
    return ctx.success()


def run_clamp(ctx: Context) -> ExecutionResult:
    value = ctx.get_f64("value", 0.0)
    min_val = ctx.get_f64("min", 0.0)
    max_val = ctx.get_f64("max", 1.0)
    ctx.set_output("result", max(min_val, min(max_val, value)))
    return ctx.success()


DISPATCH = {
    "math_add_py": run_add,
    "math_subtract_py": run_subtract,
    "math_multiply_py": run_multiply,
    "math_divide_py": run_divide,
    "math_power_py": run_power,
    "math_clamp_py": run_clamp,
}


def run(node_name: str, ctx: Context) -> ExecutionResult:
    handler = DISPATCH.get(node_name)
    if handler is None:
        return ctx.fail(f"Unknown node: {node_name}")
    return handler(ctx)
