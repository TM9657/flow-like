"""Tests for math example nodes."""

from conftest import make_context
from sdk import MockHostBridge

from math_nodes import (
    get_definitions,
    run,
    run_add,
    run_clamp,
    run_divide,
    run_multiply,
    run_power,
    run_subtract,
)


class TestMathDefinitions:
    def test_node_count(self):
        defs = get_definitions()
        assert len(defs) == 6

    def test_node_names(self):
        names = {d.name for d in get_definitions()}
        expected = {
            "math_add_py", "math_subtract_py", "math_multiply_py",
            "math_divide_py", "math_power_py", "math_clamp_py",
        }
        assert names == expected

    def test_all_have_exec_pins(self):
        for nd in get_definitions():
            pin_names = {p.name for p in nd.pins}
            assert "exec" in pin_names, f"{nd.name} missing exec pin"
            assert "exec_out" in pin_names, f"{nd.name} missing exec_out pin"


class TestAdd:
    def test_basic(self):
        result = run_add(make_context({"a": 5.0, "b": 3.0}))
        assert result.outputs["result"] == 8.0

    def test_negative(self):
        result = run_add(make_context({"a": -5.0, "b": 3.0}))
        assert result.outputs["result"] == -2.0

    def test_defaults(self):
        result = run_add(make_context({}))
        assert result.outputs["result"] == 0.0

    def test_floats(self):
        result = run_add(make_context({"a": 0.1, "b": 0.2}))
        assert abs(result.outputs["result"] - 0.3) < 1e-10


class TestSubtract:
    def test_basic(self):
        result = run_subtract(make_context({"a": 10.0, "b": 4.0}))
        assert result.outputs["result"] == 6.0

    def test_negative_result(self):
        result = run_subtract(make_context({"a": 3.0, "b": 7.0}))
        assert result.outputs["result"] == -4.0


class TestMultiply:
    def test_basic(self):
        result = run_multiply(make_context({"a": 3.0, "b": 4.0}))
        assert result.outputs["result"] == 12.0

    def test_by_zero(self):
        result = run_multiply(make_context({"a": 99.0, "b": 0.0}))
        assert result.outputs["result"] == 0.0


class TestDivide:
    def test_basic(self):
        result = run_divide(make_context({"a": 10.0, "b": 2.0}))
        assert result.outputs["result"] == 5.0
        assert result.outputs["is_valid"] is True

    def test_by_zero(self):
        host = MockHostBridge()
        result = run_divide(make_context({"a": 10.0, "b": 0.0}, host=host))
        assert result.outputs["result"] == 0.0
        assert result.outputs["is_valid"] is False
        assert any("Division by zero" in msg for _, msg in host.logs)

    def test_fractional(self):
        result = run_divide(make_context({"a": 1.0, "b": 3.0}))
        assert abs(result.outputs["result"] - 1 / 3) < 1e-10


class TestPower:
    def test_square(self):
        result = run_power(make_context({"base": 3.0, "exponent": 2.0}))
        assert result.outputs["result"] == 9.0

    def test_cube(self):
        result = run_power(make_context({"base": 2.0, "exponent": 3.0}))
        assert result.outputs["result"] == 8.0

    def test_zero_exponent(self):
        result = run_power(make_context({"base": 5.0, "exponent": 0.0}))
        assert result.outputs["result"] == 1.0

    def test_fractional_exponent(self):
        result = run_power(make_context({"base": 4.0, "exponent": 0.5}))
        assert result.outputs["result"] == 2.0


class TestClamp:
    def test_within_range(self):
        result = run_clamp(make_context({"value": 0.5, "min": 0.0, "max": 1.0}))
        assert result.outputs["result"] == 0.5

    def test_below_min(self):
        result = run_clamp(make_context({"value": -5.0, "min": 0.0, "max": 10.0}))
        assert result.outputs["result"] == 0.0

    def test_above_max(self):
        result = run_clamp(make_context({"value": 15.0, "min": 0.0, "max": 10.0}))
        assert result.outputs["result"] == 10.0

    def test_equal_bounds(self):
        result = run_clamp(make_context({"value": 99.0, "min": 5.0, "max": 5.0}))
        assert result.outputs["result"] == 5.0


class TestDispatch:
    def test_known_node(self):
        result = run("math_add_py", make_context({"a": 1.0, "b": 2.0}))
        assert result.outputs["result"] == 3.0

    def test_unknown_node(self):
        result = run("nonexistent", make_context({}))
        assert result.error is not None
        assert "Unknown node" in result.error
