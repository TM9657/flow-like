"""
String Nodes — Text manipulation utilities

Demonstrates string processing nodes including case conversion,
trimming, length analysis, search, replace, concat, and reverse.
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

    for name, friendly, desc in [
        ("string_uppercase_py", "To Uppercase", "Converts text to uppercase"),
        ("string_lowercase_py", "To Lowercase", "Converts text to lowercase"),
        ("string_trim_py", "Trim", "Removes leading and trailing whitespace"),
        ("string_reverse_py", "Reverse", "Reverses the characters in a string"),
    ]:
        nd = NodeDefinition(name, friendly, desc, "String/Transform")
        nd.add_pin(PinDefinition.input_exec("exec"))
        nd.add_pin(PinDefinition.input_pin("text", PinType.STRING, default=""))
        nd.add_pin(PinDefinition.output_exec("exec_out"))
        nd.add_pin(PinDefinition.output_pin("result", PinType.STRING))
        nodes.append(nd)

    # Length
    length = NodeDefinition("string_length_py", "String Length", "Returns the length of a string", "String/Analysis")
    length.add_pin(PinDefinition.input_exec("exec"))
    length.add_pin(PinDefinition.input_pin("text", PinType.STRING, default=""))
    length.add_pin(PinDefinition.output_exec("exec_out"))
    length.add_pin(PinDefinition.output_pin("length", PinType.I64))
    length.add_pin(PinDefinition.output_pin("is_empty", PinType.BOOL))
    nodes.append(length)

    # Contains
    contains = NodeDefinition("string_contains_py", "Contains", "Checks if text contains a substring", "String/Analysis")
    contains.add_pin(PinDefinition.input_exec("exec"))
    contains.add_pin(PinDefinition.input_pin("text", PinType.STRING, default=""))
    contains.add_pin(PinDefinition.input_pin("search", PinType.STRING, default=""))
    contains.add_pin(PinDefinition.output_exec("exec_out"))
    contains.add_pin(PinDefinition.output_pin("result", PinType.BOOL))
    nodes.append(contains)

    # Replace
    replace = NodeDefinition("string_replace_py", "Replace", "Replaces occurrences of a pattern", "String/Transform")
    replace.add_pin(PinDefinition.input_exec("exec"))
    replace.add_pin(PinDefinition.input_pin("text", PinType.STRING, default=""))
    replace.add_pin(PinDefinition.input_pin("find", PinType.STRING, default=""))
    replace.add_pin(PinDefinition.input_pin("replace_with", PinType.STRING, default=""))
    replace.add_pin(PinDefinition.output_exec("exec_out"))
    replace.add_pin(PinDefinition.output_pin("result", PinType.STRING))
    replace.add_pin(PinDefinition.output_pin("count", PinType.I64))
    nodes.append(replace)

    # Concat
    concat = NodeDefinition("string_concat_py", "Concatenate", "Joins two strings together", "String/Transform")
    concat.add_pin(PinDefinition.input_exec("exec"))
    concat.add_pin(PinDefinition.input_pin("a", PinType.STRING, default=""))
    concat.add_pin(PinDefinition.input_pin("b", PinType.STRING, default=""))
    concat.add_pin(PinDefinition.input_pin("separator", PinType.STRING, default=""))
    concat.add_pin(PinDefinition.output_exec("exec_out"))
    concat.add_pin(PinDefinition.output_pin("result", PinType.STRING))
    nodes.append(concat)

    return nodes


def run_uppercase(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    ctx.set_output("result", text.upper())
    return ctx.success()


def run_lowercase(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    ctx.set_output("result", text.lower())
    return ctx.success()


def run_trim(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    ctx.set_output("result", text.strip())
    return ctx.success()


def run_reverse(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    ctx.set_output("result", text[::-1])
    return ctx.success()


def run_length(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    ctx.set_output("length", len(text))
    ctx.set_output("is_empty", len(text) == 0)
    return ctx.success()


def run_contains(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    search = ctx.get_string("search", "")
    ctx.set_output("result", search in text)
    return ctx.success()


def run_replace(ctx: Context) -> ExecutionResult:
    text = ctx.get_string("text", "")
    find = ctx.get_string("find", "")
    replace_with = ctx.get_string("replace_with", "")
    count = text.count(find) if find else 0
    result = text.replace(find, replace_with) if find else text
    ctx.set_output("result", result)
    ctx.set_output("count", count)
    return ctx.success()


def run_concat(ctx: Context) -> ExecutionResult:
    a = ctx.get_string("a", "")
    b = ctx.get_string("b", "")
    separator = ctx.get_string("separator", "")
    result = f"{a}{separator}{b}" if separator else f"{a}{b}"
    ctx.set_output("result", result)
    return ctx.success()


DISPATCH = {
    "string_uppercase_py": run_uppercase,
    "string_lowercase_py": run_lowercase,
    "string_trim_py": run_trim,
    "string_reverse_py": run_reverse,
    "string_length_py": run_length,
    "string_contains_py": run_contains,
    "string_replace_py": run_replace,
    "string_concat_py": run_concat,
}


def run(node_name: str, ctx: Context) -> ExecutionResult:
    handler = DISPATCH.get(node_name)
    if handler is None:
        return ctx.fail(f"Unknown node: {node_name}")
    return handler(ctx)
