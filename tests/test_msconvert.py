"""Tests for msconvert() Docker command construction (Docker not run)."""

from pathlib import Path
from unittest import mock

import pytest

from mzx import docker_image, msconvert


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


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_builds_docker_command_and_returns_output_path(
    mock_run, tmp_path: Path
) -> None:
    f = tmp_path / "run.raw"
    f.write_text("x")
    params = _base_params(str(f))
    out = msconvert(params)
    assert out.endswith("run.mzML")
    mock_run.assert_called_once()
    cmd = mock_run.call_args[0][0]
    assert "docker run --rm" in cmd
    assert docker_image in cmd
    assert "wine msconvert" in cmd
    assert "run.raw" in cmd
    assert "--mzML" in cmd


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_output_format_mgf(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), type="mgf"))
    assert out.endswith("a.mgf")
    assert "--mgf" in mock_run.call_args[0][0]


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_output_format_mzxml(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), type="mzxml"))
    assert out.endswith("a.mzXML")
    assert "--mzXML" in mock_run.call_args[0][0]


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_custom_outfile_basename(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "in.raw"
    f.write_text("x")
    out = msconvert(_base_params(str(f), outfile=str(tmp_path / "custom.mzML")))
    assert out.endswith("custom.mzML")
    mock_run.assert_called_once()


@mock.patch("mzx.run_cmd", return_value="")
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
    assert needle in mock_run.call_args[0][0]


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_peak_picking_off_adds_no_peak_filter(
    mock_run, tmp_path: Path
) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), peak_picking="off"))
    cmd = mock_run.call_args[0][0]
    assert "peakPicking" not in cmd


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_noindex_and_sort(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), index=False, sortbyscan=True))
    cmd = mock_run.call_args[0][0]
    assert "--noindex" in cmd
    assert "sortByScanTime" in cmd


@mock.patch("mzx.run_cmd", return_value="")
def test_msconvert_remove_zeros_false_skips_filter(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "x.raw"
    f.write_text("x")
    msconvert(_base_params(str(f), remove_zeros=False))
    cmd = mock_run.call_args[0][0]
    assert "zeroSamples" not in cmd


@mock.patch("mzx.run_cmd", return_value="")
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
    cmd = mock_run.call_args[0][0]
    assert "556.2771" in cmd
    assert "554.2615" in cmd
    assert "0.1" in cmd


@mock.patch("mzx.run_cmd", return_value="")
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
    cmd = mock_run.call_args[0][0]
    assert "lockmassRefiner" in cmd
    assert "mz=500.0" in cmd
    assert "scanEvent" in cmd
    assert "1-2 4-" in cmd  # exclusion_string(3)
