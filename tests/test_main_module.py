"""Smoke test for python -m mzx."""

import subprocess
import sys

from mzx import __version__


def test_mzx_module_help_exits_zero() -> None:
    r = subprocess.run(
        [sys.executable, "-m", "mzx", "--help"],
        check=False,
        capture_output=True,
        text=True,
    )
    assert r.returncode == 0
    assert "usage:" in r.stdout.lower()
    assert "file" in r.stdout.lower()


def test_mzx_module_version_exits_zero() -> None:
    r = subprocess.run(
        [sys.executable, "-m", "mzx", "--version"],
        check=False,
        capture_output=True,
        text=True,
    )
    assert r.returncode == 0
    assert __version__ in r.stdout
