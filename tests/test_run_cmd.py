"""Tests for subprocess wrapper run_cmd."""

import sys
import time

import pytest

from mzx import CommandTimeoutError, RawFileConversionError, run_cmd


def _py(code: str) -> list[str]:
    return [sys.executable, "-c", code]


def test_run_cmd_collects_stdout() -> None:
    assert run_cmd(_py("print('hello')")) == "hello\n"


def test_run_cmd_accumulates_multiple_lines() -> None:
    assert run_cmd(_py("print('first'); print('second')")) == "first\nsecond\n"


def test_run_cmd_accepts_string_command() -> None:
    out = run_cmd(f"'{sys.executable}' -c \"print('from string')\"")
    assert out == "from string\n"


def test_run_cmd_list_args_are_not_reparsed() -> None:
    # A quote inside an argument must reach the process unchanged
    out = run_cmd(_py("import sys; print(sys.argv[1])") + ["it's a path"])
    assert out == "it's a path\n"


def test_run_cmd_merges_stderr() -> None:
    out = run_cmd(_py("import sys; print('err', file=sys.stderr)"))
    assert out == "err\n"


def test_run_cmd_raises_on_nonzero_exit_with_output_tail() -> None:
    with pytest.raises(RawFileConversionError, match="exited with code 3") as excinfo:
        run_cmd(_py("import sys; print('boom', file=sys.stderr); sys.exit(3)"))
    assert "boom" in str(excinfo.value)


def test_run_cmd_timeout_kills_process() -> None:
    started = time.monotonic()
    with pytest.raises(CommandTimeoutError, match="timed out after 0.5s"):
        run_cmd(_py("import time; time.sleep(30)"), timeout=0.5)
    assert time.monotonic() - started < 15


def test_command_timeout_is_a_conversion_error() -> None:
    assert issubclass(CommandTimeoutError, RawFileConversionError)
