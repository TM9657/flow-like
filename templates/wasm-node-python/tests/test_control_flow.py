"""Tests for control flow example nodes."""

from conftest import make_context

from control_flow import (
    get_definitions,
    run,
    run_and_gate,
    run_compare,
    run_gate,
    run_if_branch,
    run_not_gate,
    run_or_gate,
    run_sequence,
)


class TestControlFlowDefinitions:
    def test_node_count(self):
        defs = get_definitions()
        assert len(defs) == 7

    def test_node_names(self):
        names = {d.name for d in get_definitions()}
        expected = {
            "if_branch_py", "compare_py", "and_gate_py", "or_gate_py",
            "not_gate_py", "gate_py", "sequence_py",
        }
        assert names == expected


class TestIfBranch:
    def test_true_branch(self):
        result = run_if_branch(make_context({"condition": True}))
        assert "true" in result.activate_exec
        assert "false" not in result.activate_exec

    def test_false_branch(self):
        result = run_if_branch(make_context({"condition": False}))
        assert "false" in result.activate_exec
        assert "true" not in result.activate_exec

    def test_default_is_false(self):
        result = run_if_branch(make_context({}))
        assert "false" in result.activate_exec


class TestCompare:
    def test_equal(self):
        result = run_compare(make_context({"a": 5.0, "b": 5.0}))
        assert result.outputs["equal"] is True
        assert result.outputs["less_than"] is False
        assert result.outputs["greater_than"] is False

    def test_less_than(self):
        result = run_compare(make_context({"a": 3.0, "b": 7.0}))
        assert result.outputs["equal"] is False
        assert result.outputs["less_than"] is True
        assert result.outputs["greater_than"] is False

    def test_greater_than(self):
        result = run_compare(make_context({"a": 10.0, "b": 2.0}))
        assert result.outputs["equal"] is False
        assert result.outputs["less_than"] is False
        assert result.outputs["greater_than"] is True


class TestAndGate:
    def test_true_true(self):
        result = run_and_gate(make_context({"a": True, "b": True}))
        assert result.outputs["result"] is True

    def test_true_false(self):
        result = run_and_gate(make_context({"a": True, "b": False}))
        assert result.outputs["result"] is False

    def test_false_false(self):
        result = run_and_gate(make_context({"a": False, "b": False}))
        assert result.outputs["result"] is False


class TestOrGate:
    def test_true_true(self):
        result = run_or_gate(make_context({"a": True, "b": True}))
        assert result.outputs["result"] is True

    def test_true_false(self):
        result = run_or_gate(make_context({"a": True, "b": False}))
        assert result.outputs["result"] is True

    def test_false_false(self):
        result = run_or_gate(make_context({"a": False, "b": False}))
        assert result.outputs["result"] is False


class TestNotGate:
    def test_true(self):
        result = run_not_gate(make_context({"value": True}))
        assert result.outputs["result"] is False

    def test_false(self):
        result = run_not_gate(make_context({"value": False}))
        assert result.outputs["result"] is True


class TestGate:
    def test_open(self):
        result = run_gate(make_context({"condition": True}))
        assert "exec_out" in result.activate_exec

    def test_closed(self):
        result = run_gate(make_context({"condition": False}))
        assert "exec_out" not in result.activate_exec

    def test_default_open(self):
        result = run_gate(make_context({}))
        assert "exec_out" in result.activate_exec


class TestSequence:
    def test_all_outputs_activated(self):
        result = run_sequence(make_context({}))
        assert "out_1" in result.activate_exec
        assert "out_2" in result.activate_exec
        assert "out_3" in result.activate_exec

    def test_order_preserved(self):
        result = run_sequence(make_context({}))
        assert result.activate_exec == ["out_1", "out_2", "out_3"]


class TestDispatch:
    def test_if_branch(self):
        result = run("if_branch_py", make_context({"condition": True}))
        assert "true" in result.activate_exec

    def test_unknown_node(self):
        result = run("nonexistent", make_context({}))
        assert result.error is not None
        assert "Unknown node" in result.error
