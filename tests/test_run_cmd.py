"""Tests for subprocess wrapper run_cmd."""

from unittest import mock

from mzx import run_cmd


def test_run_cmd_collects_stdout() -> None:
    class FakeStdout:
        def readline(self):
            return ""

    class FakeProc:
        stdout = FakeStdout()

    with mock.patch("mzx.subprocess.Popen", return_value=FakeProc()) as popen:
        out = run_cmd("echo hello")

    popen.assert_called_once()
    assert out == ""


def test_run_cmd_accumulates_multiple_lines() -> None:
    remaining = ["first\n", "second\n", ""]

    class FakeStdout:
        def readline(self):
            return remaining.pop(0)

    class FakeProc:
        stdout = FakeStdout()

    with mock.patch("mzx.subprocess.Popen", return_value=FakeProc()):
        out = run_cmd("dummy")

    assert out == "first\nsecond\n"
