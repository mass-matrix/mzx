"""Tests for msconvert() Docker command construction (Docker not run)."""

from pathlib import Path
from unittest import mock

import pytest

from mzx import (
    CommandTimeoutError,
    RawFileConversionError,
    docker_image,
    kill_container,
    msconvert,
)


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


def _fake_run(cmd: list[str], timeout: float | None = None) -> str:
    """Stand-in for run_cmd that writes the output file msconvert expects."""
    data_dir = cmd[cmd.index("-v") + 1].rsplit(":/data", 1)[0]
    outfile = cmd[cmd.index("--outfile") + 1].removeprefix("/data/")
    Path(data_dir, outfile).write_text("")
    return ""


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_builds_docker_command_and_returns_output_path(
    mock_run, tmp_path: Path
) -> None:
    f = tmp_path / "run.raw"
    f.write_text("x")
    params = _base_params(str(f))
    out = msconvert(params)
    assert out.endswith("run.mzML")
    mock_run.assert_called_once()
    cmd = " ".join(mock_run.call_args[0][0])
    assert "docker run --rm" in cmd
    assert docker_image in cmd
    assert "wine msconvert" in cmd
    assert "run.raw" in cmd
    assert "--mzML" in cmd


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_output_format_mgf(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), type="mgf"))
    assert out.endswith("a.mgf")
    assert "--mgf" in " ".join(mock_run.call_args[0][0])


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_output_format_mzxml(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), type="mzxml"))
    assert out.endswith("a.mzXML")
    assert "--mzXML" in " ".join(mock_run.call_args[0][0])


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_custom_outfile_basename(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "in.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), outfile=str(tmp_path / "custom.mzML")))
    assert out.endswith("custom.mzML")
    mock_run.assert_called_once()


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
@pytest.mark.parametrize(
    "peak_picking, needle",
    [
        ("all", "peakPicking true 1-"),
        ("ms1", "peakPicking true 1"),
        ("msms", "peakPicking true 2-"),
    ],
)
def test_msconvert_peak_picking_filters(
    mock_run, tmp_path: Path, peak_picking: str, needle: str
) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), peak_picking=peak_picking))
    assert needle in " ".join(mock_run.call_args[0][0])


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_peak_picking_off_adds_no_peak_filter(
    mock_run, tmp_path: Path
) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), peak_picking="off"))
    cmd = " ".join(mock_run.call_args[0][0])
    assert "peakPicking" not in cmd


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_noindex_and_sort(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), index=False, sortbyscan=True))
    cmd = " ".join(mock_run.call_args[0][0])
    assert "--noindex" in cmd
    assert "sortByScanTime" in cmd


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_remove_zeros_false_skips_filter(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), remove_zeros=False))
    cmd = " ".join(mock_run.call_args[0][0])
    assert "zeroSamples" not in cmd


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_lockmass_defaults_when_values_none(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(
        _base_params(
            str(f),
            lockmass=True,
            neg_lockmass=None,
            pos_lockmass=None,
            lockmass_tolerance=None,
        )
    )
    cmd = " ".join(mock_run.call_args[0][0])
    assert "556.2771" in cmd
    assert "554.2615" in cmd
    assert "0.1" in cmd


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_lockmass_and_scan_event_exclude(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(
        _base_params(
            str(f),
            lockmass=True,
            pos_lockmass=500.0,
            neg_lockmass=500.5,
            lockmass_tolerance=0.2,
            lockmass_function_exclude=3,
        )
    )
    cmd = " ".join(mock_run.call_args[0][0])
    assert "lockmassRefiner" in cmd
    assert "mz=500.0" in cmd
    assert "scanEvent" in cmd
    assert "1-2 4-" in cmd  # exclusion_string(3)


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_passes_paths_as_single_arguments(mock_run, tmp_path: Path) -> None:
    d = tmp_path / "it's here"
    d.mkdir()
    f = d / "run one.raw"
    f.write_text("x")
    msconvert(_base_params(str(f)))
    cmd = mock_run.call_args[0][0]
    assert f"{d}:/data" in cmd
    assert "/data/run one.raw" in cmd
    assert cmd[cmd.index("--filter") + 1] == "peakPicking true 2-"


@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_passes_timeout(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), timeout=12.5))
    assert mock_run.call_args.kwargs["timeout"] == 12.5


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_raises_when_output_missing(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    with pytest.raises(RawFileConversionError, match="did not produce"):
        msconvert(_base_params(str(f)))


@mock.patch("mzx.kill_container")
@mock.patch("mzx.run_cmd", side_effect=CommandTimeoutError("timed out"))
def test_msconvert_kills_named_container_on_failure(
    mock_run, mock_kill, tmp_path: Path
) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    with pytest.raises(CommandTimeoutError):
        msconvert(_base_params(str(f)))
    cmd = mock_run.call_args[0][0]
    container_name = cmd[cmd.index("--name") + 1]
    assert container_name.startswith("mzx-")
    mock_kill.assert_called_once_with(container_name)


@mock.patch("mzx.kill_container")
@mock.patch("mzx.run_cmd", side_effect=_fake_run)
def test_msconvert_does_not_kill_container_on_success(
    mock_run, mock_kill, tmp_path: Path
) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f)))
    mock_kill.assert_not_called()


@mock.patch("mzx.subprocess.run", side_effect=OSError("docker not found"))
def test_kill_container_swallows_errors(mock_subprocess_run) -> None:
    kill_container("mzx-abc")
    assert mock_subprocess_run.call_args[0][0] == ["docker", "kill", "mzx-abc"]
