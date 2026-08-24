"""Tests for docker command construction (Docker not run).

Filter/flag construction is covered by test_build_msconvert_args.py; this module
only asserts the container plumbing.
"""

from pathlib import Path
from unittest import mock

from mzx import docker_image, run_msconvert_docker


def _base_params(infile: str, **overrides):
    p = {
        "infile": infile,
        "index": True,
        "sortbyscan": False,
        "peak_picking": "msms",
        "remove_zeros": True,
        "vendor": "thermo",
        "outfile": None,
        "type": "mzml",
        "overwrite": False,
        "debug": False,
        "verbose": False,
        "lockmass_disabled": False,
        "lockmass": False,
        "neg_lockmass": None,
        "pos_lockmass": None,
        "lockmass_tolerance": None,
        "lockmass_function_exclude": None,
    }
    p.update(overrides)
    return p


@mock.patch("mzx.run_cmd", return_value=0)
def test_builds_docker_command(mock_run, tmp_path: Path) -> None:
    params = _base_params(str(tmp_path / "run.raw"))

    code = run_msconvert_docker(params, "/in/run.raw", "/out/dir/run.mzML")

    assert code == 0
    cmd = mock_run.call_args[0][0]
    assert cmd[:3] == ["docker", "run", "--rm"]
    assert docker_image in cmd
    assert cmd[cmd.index(docker_image) + 1 : cmd.index(docker_image) + 3] == [
        "wine",
        "msconvert",
    ]


@mock.patch("mzx.run_cmd", return_value=0)
def test_mounts_input_and_output_directories(mock_run, tmp_path: Path) -> None:
    params = _base_params(str(tmp_path / "run.raw"))

    run_msconvert_docker(params, "/in/run.raw", "/out/dir/run.mzML")

    cmd = mock_run.call_args[0][0]
    assert "/in:/data" in cmd
    assert "/out/dir:/out" in cmd


@mock.patch("mzx.run_cmd", return_value=0)
def test_paths_are_translated_into_the_container(mock_run, tmp_path: Path) -> None:
    """msconvert sees container paths, not host paths."""
    params = _base_params(str(tmp_path / "run.raw"))

    run_msconvert_docker(params, "/in/run.raw", "/out/dir/run.mzML")

    cmd = mock_run.call_args[0][0]
    assert "/data/run.raw" in cmd
    assert "/out/run.mzML" in cmd
    assert "/in/run.raw" not in cmd[cmd.index(docker_image) :]


@mock.patch("mzx.run_cmd", return_value=0)
def test_custom_image(mock_run, tmp_path: Path) -> None:
    params = _base_params(str(tmp_path / "run.raw"))

    run_msconvert_docker(params, "/in/run.raw", "/out/run.mzML", image="pwiz:pinned")

    assert "pwiz:pinned" in mock_run.call_args[0][0]


@mock.patch("mzx.run_cmd", return_value=0)
def test_timeout_is_forwarded(mock_run, tmp_path: Path) -> None:
    params = _base_params(str(tmp_path / "run.raw"))

    run_msconvert_docker(params, "/in/run.raw", "/out/run.mzML", timeout=120)

    assert mock_run.call_args.kwargs["timeout"] == 120


@mock.patch("mzx.run_cmd", return_value=4)
def test_returns_exit_code(mock_run, tmp_path: Path) -> None:
    params = _base_params(str(tmp_path / "run.raw"))

    assert run_msconvert_docker(params, "/in/run.raw", "/out/run.mzML") == 4
