"""Tests for string example nodes."""

from conftest import make_context

from string_nodes import (
    get_definitions,
    run,
    run_concat,
    run_contains,
    run_length,
    run_lowercase,
    run_replace,
    run_reverse,
    run_trim,
    run_uppercase,
)


class TestStringDefinitions:
    def test_node_count(self):
        defs = get_definitions()
        assert len(defs) == 8

    def test_node_names(self):
        names = {d.name for d in get_definitions()}
        expected = {
            "string_uppercase_py", "string_lowercase_py", "string_trim_py",
            "string_reverse_py", "string_length_py", "string_contains_py",
            "string_replace_py", "string_concat_py",
        }
        assert names == expected


class TestUppercase:
    def test_basic(self):
        result = run_uppercase(make_context({"text": "hello"}))
        assert result.outputs["result"] == "HELLO"

    def test_already_upper(self):
        result = run_uppercase(make_context({"text": "HELLO"}))
        assert result.outputs["result"] == "HELLO"

    def test_empty(self):
        result = run_uppercase(make_context({"text": ""}))
        assert result.outputs["result"] == ""

    def test_mixed(self):
        result = run_uppercase(make_context({"text": "Hello World 123!"}))
        assert result.outputs["result"] == "HELLO WORLD 123!"


class TestLowercase:
    def test_basic(self):
        result = run_lowercase(make_context({"text": "HELLO"}))
        assert result.outputs["result"] == "hello"

    def test_mixed(self):
        result = run_lowercase(make_context({"text": "HeLLo WoRLd"}))
        assert result.outputs["result"] == "hello world"


class TestTrim:
    def test_basic(self):
        result = run_trim(make_context({"text": "  hello  "}))
        assert result.outputs["result"] == "hello"

    def test_tabs_and_newlines(self):
        result = run_trim(make_context({"text": "\t\nhello\n\t"}))
        assert result.outputs["result"] == "hello"

    def test_no_whitespace(self):
        result = run_trim(make_context({"text": "hello"}))
        assert result.outputs["result"] == "hello"


class TestReverse:
    def test_basic(self):
        result = run_reverse(make_context({"text": "hello"}))
        assert result.outputs["result"] == "olleh"

    def test_palindrome(self):
        result = run_reverse(make_context({"text": "racecar"}))
        assert result.outputs["result"] == "racecar"

    def test_empty(self):
        result = run_reverse(make_context({"text": ""}))
        assert result.outputs["result"] == ""

    def test_single_char(self):
        result = run_reverse(make_context({"text": "x"}))
        assert result.outputs["result"] == "x"


class TestLength:
    def test_basic(self):
        result = run_length(make_context({"text": "hello"}))
        assert result.outputs["length"] == 5
        assert result.outputs["is_empty"] is False

    def test_empty(self):
        result = run_length(make_context({"text": ""}))
        assert result.outputs["length"] == 0
        assert result.outputs["is_empty"] is True

    def test_with_spaces(self):
        result = run_length(make_context({"text": "  hi  "}))
        assert result.outputs["length"] == 6


class TestContains:
    def test_found(self):
        result = run_contains(make_context({"text": "hello world", "search": "world"}))
        assert result.outputs["result"] is True

    def test_not_found(self):
        result = run_contains(make_context({"text": "hello world", "search": "xyz"}))
        assert result.outputs["result"] is False

    def test_empty_search(self):
        result = run_contains(make_context({"text": "hello", "search": ""}))
        assert result.outputs["result"] is True

    def test_case_sensitive(self):
        result = run_contains(make_context({"text": "Hello", "search": "hello"}))
        assert result.outputs["result"] is False


class TestReplace:
    def test_basic(self):
        result = run_replace(make_context({"text": "hello world", "find": "world", "replace_with": "python"}))
        assert result.outputs["result"] == "hello python"
        assert result.outputs["count"] == 1

    def test_multiple_occurrences(self):
        result = run_replace(make_context({"text": "aabaa", "find": "a", "replace_with": "x"}))
        assert result.outputs["result"] == "xxbxx"
        assert result.outputs["count"] == 4

    def test_no_match(self):
        result = run_replace(make_context({"text": "hello", "find": "xyz", "replace_with": "abc"}))
        assert result.outputs["result"] == "hello"
        assert result.outputs["count"] == 0

    def test_empty_find(self):
        result = run_replace(make_context({"text": "hello", "find": "", "replace_with": "x"}))
        assert result.outputs["result"] == "hello"
        assert result.outputs["count"] == 0


class TestConcat:
    def test_basic(self):
        result = run_concat(make_context({"a": "hello", "b": "world"}))
        assert result.outputs["result"] == "helloworld"

    def test_with_separator(self):
        result = run_concat(make_context({"a": "hello", "b": "world", "separator": " "}))
        assert result.outputs["result"] == "hello world"

    def test_empty_strings(self):
        result = run_concat(make_context({"a": "", "b": ""}))
        assert result.outputs["result"] == ""

    def test_separator_only(self):
        result = run_concat(make_context({"a": "", "b": "", "separator": "-"}))
        assert result.outputs["result"] == "-"


class TestDispatch:
    def test_all_nodes_dispatch(self):
        cases = [
            ("string_uppercase_py", {"text": "hi"}, "result", "HI"),
            ("string_lowercase_py", {"text": "HI"}, "result", "hi"),
            ("string_trim_py", {"text": " hi "}, "result", "hi"),
            ("string_reverse_py", {"text": "abc"}, "result", "cba"),
        ]
        for node_name, inputs, output_key, expected in cases:
            result = run(node_name, make_context(inputs))
            assert result.outputs[output_key] == expected, f"Failed for {node_name}"

    def test_unknown_node(self):
        result = run("nonexistent", make_context({}))
        assert result.error is not None
