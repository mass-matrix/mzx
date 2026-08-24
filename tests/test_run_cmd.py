"""Tests for subprocess wrapper run_cmd."""

import subprocess
from unittest import mock

import pytest

from mzx import ConversionTerminatedAbnormally, run_cmd


def test_run_cmd_returns_exit_code() -> None:
    with mock.patch("mzx.subprocess.run") as run:
        run.return_value = mock.Mock(returncode=0)
        assert run_cmd(["echo", "hello"]) == 0

    run.assert_called_once()
    assert run.call_args[0][0] == ["echo", "hello"]


def test_run_cmd_surfaces_nonzero_exit_code() -> None:
    """The previous implementation read stdout and never checked returncode."""
    with mock.patch("mzx.subprocess.run") as run:
        run.return_value = mock.Mock(returncode=2)
        assert run_cmd(["false"]) == 2


def test_run_cmd_forwards_timeout() -> None:
    with mock.patch("mzx.subprocess.run") as run:
        run.return_value = mock.Mock(returncode=0)
        run_cmd(["sleep", "1"], timeout=30)

    assert run.call_args.kwargs["timeout"] == 30


def test_run_cmd_raises_on_timeout() -> None:
    with mock.patch(
        "mzx.subprocess.run", side_effect=subprocess.TimeoutExpired("sleep", 5)
    ):
        with pytest.raises(ConversionTerminatedAbnormally):
            run_cmd(["sleep", "10"], timeout=5)
