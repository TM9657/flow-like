"""
Control Flow Nodes — Logic and branching operations

Demonstrates if/else branching, comparison, boolean gates,
conditional pass-through, and sequencing.
"""

from sdk import (
    Context,
    ExecutionResult,
    NodeDefinition,
    PinDefinition,
    PinType,
)


def get_definitions() -> list[NodeDefinition]:
    nodes: list[NodeDefinition] = []

    # If Branch
    if_branch = NodeDefinition("if_branch_py", "If Branch", "Branches based on boolean condition", "Control/Branch")
    if_branch.add_pin(PinDefinition.input_exec("exec"))
    if_branch.add_pin(PinDefinition.input_pin("condition", PinType.BOOL, default=False))
    if_branch.add_pin(PinDefinition.output_exec("true"))
    if_branch.add_pin(PinDefinition.output_exec("false"))
    nodes.append(if_branch)

    # Compare
    compare = NodeDefinition("compare_py", "Compare", "Compares two values", "Control/Logic")
    compare.add_pin(PinDefinition.input_exec("exec"))
    compare.add_pin(PinDefinition.input_pin("a", PinType.F64, default=0.0))
    compare.add_pin(PinDefinition.input_pin("b", PinType.F64, default=0.0))
    compare.add_pin(PinDefinition.output_exec("exec_out"))
    compare.add_pin(PinDefinition.output_pin("equal", PinType.BOOL))
    compare.add_pin(PinDefinition.output_pin("less_than", PinType.BOOL))
    compare.add_pin(PinDefinition.output_pin("greater_than", PinType.BOOL))
    nodes.append(compare)

    # AND Gate
    and_gate = NodeDefinition("and_gate_py", "AND Gate", "Logical AND of two booleans", "Control/Logic")
    and_gate.add_pin(PinDefinition.input_exec("exec"))
    and_gate.add_pin(PinDefinition.input_pin("a", PinType.BOOL, default=False))
    and_gate.add_pin(PinDefinition.input_pin("b", PinType.BOOL, default=False))
    and_gate.add_pin(PinDefinition.output_exec("exec_out"))
    and_gate.add_pin(PinDefinition.output_pin("result", PinType.BOOL))
    nodes.append(and_gate)

    # OR Gate
    or_gate = NodeDefinition("or_gate_py", "OR Gate", "Logical OR of two booleans", "Control/Logic")
    or_gate.add_pin(PinDefinition.input_exec("exec"))
    or_gate.add_pin(PinDefinition.input_pin("a", PinType.BOOL, default=False))
    or_gate.add_pin(PinDefinition.input_pin("b", PinType.BOOL, default=False))
    or_gate.add_pin(PinDefinition.output_exec("exec_out"))
    or_gate.add_pin(PinDefinition.output_pin("result", PinType.BOOL))
    nodes.append(or_gate)

    # NOT Gate
    not_gate = NodeDefinition("not_gate_py", "NOT Gate", "Logical NOT", "Control/Logic")
    not_gate.add_pin(PinDefinition.input_exec("exec"))
    not_gate.add_pin(PinDefinition.input_pin("value", PinType.BOOL, default=False))
    not_gate.add_pin(PinDefinition.output_exec("exec_out"))
    not_gate.add_pin(PinDefinition.output_pin("result", PinType.BOOL))
    nodes.append(not_gate)

    # Gate (conditional pass-through)
    gate = NodeDefinition("gate_py", "Gate", "Passes execution only if condition is true", "Control/Branch")
    gate.add_pin(PinDefinition.input_exec("exec"))
    gate.add_pin(PinDefinition.input_pin("condition", PinType.BOOL, default=True))
    gate.add_pin(PinDefinition.output_exec("exec_out"))
    nodes.append(gate)

    # Sequence
    sequence = NodeDefinition("sequence_py", "Sequence", "Activates multiple outputs in order", "Control/Flow")
    sequence.add_pin(PinDefinition.input_exec("exec"))
    sequence.add_pin(PinDefinition.output_exec("out_1"))
    sequence.add_pin(PinDefinition.output_exec("out_2"))
    sequence.add_pin(PinDefinition.output_exec("out_3"))
    nodes.append(sequence)

    return nodes


def run_if_branch(ctx: Context) -> ExecutionResult:
    condition = ctx.get_bool("condition", False)
    if condition:
        ctx.activate_exec("true")
    else:
        ctx.activate_exec("false")
    return ctx.finish()


def run_compare(ctx: Context) -> ExecutionResult:
    a = ctx.get_f64("a", 0.0)
    b = ctx.get_f64("b", 0.0)
    ctx.set_output("equal", a == b)
    ctx.set_output("less_than", a < b)
    ctx.set_output("greater_than", a > b)
    return ctx.success()


def run_and_gate(ctx: Context) -> ExecutionResult:
    a = ctx.get_bool("a", False)
    b = ctx.get_bool("b", False)
    ctx.set_output("result", a and b)
    return ctx.success()


def run_or_gate(ctx: Context) -> ExecutionResult:
    a = ctx.get_bool("a", False)
    b = ctx.get_bool("b", False)
    ctx.set_output("result", a or b)
    return ctx.success()


def run_not_gate(ctx: Context) -> ExecutionResult:
    value = ctx.get_bool("value", False)
    ctx.set_output("result", not value)
    return ctx.success()


def run_gate(ctx: Context) -> ExecutionResult:
    condition = ctx.get_bool("condition", True)
    if condition:
        return ctx.success()
    return ctx.finish()


def run_sequence(ctx: Context) -> ExecutionResult:
    ctx.activate_exec("out_1")
    ctx.activate_exec("out_2")
    ctx.activate_exec("out_3")
    return ctx.finish()


DISPATCH = {
    "if_branch_py": run_if_branch,
    "compare_py": run_compare,
    "and_gate_py": run_and_gate,
    "or_gate_py": run_or_gate,
    "not_gate_py": run_not_gate,
    "gate_py": run_gate,
    "sequence_py": run_sequence,
}


def run(node_name: str, ctx: Context) -> ExecutionResult:
    handler = DISPATCH.get(node_name)
    if handler is None:
        return ctx.fail(f"Unknown node: {node_name}")
    return handler(ctx)
