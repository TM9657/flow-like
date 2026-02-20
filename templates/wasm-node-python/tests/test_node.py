"""Tests for the main template node."""

from conftest import make_context

from node import get_definition, run


class TestNodeDefinition:
    def test_name(self):
        nd = get_definition()
        assert nd.name == "my_custom_node_py"
        assert nd.category == "Custom/WASM"

    def test_has_required_pins(self):
        nd = get_definition()
        pin_names = {p.name for p in nd.pins}
        assert "exec" in pin_names
        assert "exec_out" in pin_names
        assert "input_text" in pin_names
        assert "multiplier" in pin_names
        assert "output_text" in pin_names
        assert "char_count" in pin_names

    def test_pin_types(self):
        nd = get_definition()
        by_name = {p.name: p for p in nd.pins}
        assert by_name["exec"].data_type == "Exec"
        assert by_name["input_text"].data_type == "String"
        assert by_name["multiplier"].data_type == "I64"
        assert by_name["output_text"].data_type == "String"
        assert by_name["char_count"].data_type == "I64"

    def test_serialization(self):
        nd = get_definition()
        d = nd.to_dict()
        assert d["name"] == "my_custom_node_py"
        assert len(d["pins"]) == 6

    def test_defaults(self):
        nd = get_definition()
        by_name = {p.name: p for p in nd.pins}
        assert by_name["input_text"].default_value == ""
        assert by_name["multiplier"].default_value == 1


class TestNodeRun:
    def test_basic_repeat(self):
        ctx = make_context({"input_text": "ab", "multiplier": 3})
        result = run(ctx)
        assert result.error is None
        assert result.outputs["output_text"] == "ababab"
        assert result.outputs["char_count"] == 6
        assert "exec_out" in result.activate_exec

    def test_empty_text(self):
        ctx = make_context({"input_text": "", "multiplier": 5})
        result = run(ctx)
        assert result.outputs["output_text"] == ""
        assert result.outputs["char_count"] == 0

    def test_zero_multiplier(self):
        ctx = make_context({"input_text": "hello", "multiplier": 0})
        result = run(ctx)
        assert result.outputs["output_text"] == ""
        assert result.outputs["char_count"] == 0

    def test_negative_multiplier(self):
        ctx = make_context({"input_text": "hello", "multiplier": -3})
        result = run(ctx)
        assert result.outputs["output_text"] == ""
        assert result.outputs["char_count"] == 0

    def test_default_inputs(self):
        ctx = make_context({})
        result = run(ctx)
        assert result.error is None
        assert result.outputs["output_text"] == ""

    def test_single_char(self):
        ctx = make_context({"input_text": "x", "multiplier": 5})
        result = run(ctx)
        assert result.outputs["output_text"] == "xxxxx"
        assert result.outputs["char_count"] == 5

    def test_streaming(self):
        from sdk import MockHostBridge
        host = MockHostBridge()
        ctx = make_context({"input_text": "hi", "multiplier": 2}, host=host, stream=True)
        run(ctx)
        assert any("4 characters" in s[1] for s in host.streams)
