"""Tests for convert() and run_msconvert()."""

from pathlib import Path
from unittest import mock

import pytest

from mzx import (
    ConversionProducedNoOutput,
    ConversionTerminatedAbnormally,
    RawFileConversionError,
    convert,
    run_msconvert,
)

MZML_BODY = b'<?xml version="1.0"?><mzML><run/></mzML>'


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


def _writes(content: bytes = MZML_BODY, returncode: int = 0):
    """side_effect for run_msconvert that writes `content` to the output path."""

    def side_effect(params, infile, outfile, **kwargs):
        if returncode == 0:
            Path(outfile).write_bytes(content)
        return returncode

    return side_effect


@mock.patch("mzx.run_msconvert", side_effect=_writes())
def test_convert_returns_output_path(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")
    out_dir = tmp_path / "out"

    result = convert(_base_params(str(raw)), output_dir=str(out_dir))

    assert result == str(out_dir / "run.mzML")
    assert Path(result).exists()


@mock.patch("mzx.run_msconvert", side_effect=_writes())
def test_convert_defaults_output_dir_to_input_parent(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    result = convert(_base_params(str(raw)))

    assert result == str(tmp_path / "run.mzML")


@mock.patch("mzx.run_msconvert", side_effect=_writes())
def test_convert_passes_executable_and_timeout(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    convert(
        _base_params(str(raw)),
        executable=["wine64_anyuser", "msconvert"],
        timeout=90,
    )

    assert mock_run.call_args.kwargs["executable"] == ["wine64_anyuser", "msconvert"]
    assert mock_run.call_args.kwargs["timeout"] == 90


@mock.patch("mzx.run_msconvert", side_effect=_writes(returncode=1))
def test_convert_raises_on_nonzero_exit(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    with pytest.raises(ConversionTerminatedAbnormally):
        convert(_base_params(str(raw)))


@mock.patch("mzx.run_msconvert", return_value=0)
def test_convert_raises_when_nothing_written(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    with pytest.raises(ConversionProducedNoOutput):
        convert(_base_params(str(raw)))


@mock.patch(
    "mzx.run_msconvert",
    side_effect=_writes(content=b'<?xml version="1.0"?><mzML><run>'),
)
def test_convert_raises_on_truncated_output(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    with pytest.raises(ConversionProducedNoOutput):
        convert(_base_params(str(raw)))


@mock.patch(
    "mzx.run_msconvert",
    side_effect=_writes(
        content=b"<indexedmzML><mzML/>"
        + b"<offset>1</offset>" * 500
        + b"</indexedmzML>"
    ),
)
def test_convert_accepts_indexed_mzml_closing_tag(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    assert Path(convert(_base_params(str(raw)))).exists()


@mock.patch("mzx.run_msconvert", side_effect=_writes(content=b"BEGIN IONS"))
def test_convert_skips_tag_check_for_mgf(mock_run, tmp_path: Path) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")

    result = convert(_base_params(str(raw), type="mgf"))

    assert result.endswith("run.mgf")


@mock.patch("mzx.run_msconvert", side_effect=_writes())
def test_convert_waters_reads_lockmass_config(mock_run, tmp_path: Path) -> None:
    d = tmp_path / "sample.raw"
    d.mkdir()
    (d / "x_extern.inf").write_text(
        "REFERENCE Function 2 something\n", encoding="latin-1"
    )

    convert(
        _base_params(str(d), vendor="waters", lockmass_disabled=False),
        output_dir=str(tmp_path / "out"),
    )

    passed = mock_run.call_args[0][0]
    assert passed["lockmass"] is True
    assert passed["lockmass_function_exclude"] == 2


@mock.patch("mzx.run_msconvert", side_effect=_writes())
def test_convert_waters_without_extern_inf_raises(mock_run, tmp_path: Path) -> None:
    d = tmp_path / "sample.raw"
    d.mkdir()

    with pytest.raises(RawFileConversionError):
        convert(_base_params(str(d), vendor="waters"))


@mock.patch("mzx.subprocess.run")
def test_run_msconvert_builds_command_with_executable(mock_run, tmp_path: Path) -> None:
    mock_run.return_value = mock.Mock(returncode=0, stdout=b"", stderr=b"")
    params = _base_params(str(tmp_path / "run.raw"))

    code = run_msconvert(
        params,
        "/in/run.raw",
        "/out/run.mzML",
        executable=["wine64_anyuser", "msconvert"],
    )

    assert code == 0
    cmd = mock_run.call_args[0][0]
    assert cmd[:3] == ["wine64_anyuser", "msconvert", "/in/run.raw"]
    assert "--mzML" in cmd
    assert "/out/run.mzML" in cmd


@mock.patch("mzx.subprocess.run")
def test_run_msconvert_defaults_to_msconvert_on_path(mock_run, tmp_path: Path) -> None:
    mock_run.return_value = mock.Mock(returncode=3, stdout=b"", stderr=b"boom")
    params = _base_params(str(tmp_path / "run.raw"))

    code = run_msconvert(params, "/in/run.raw", "/out/run.mzML")

    assert code == 3
    assert mock_run.call_args[0][0][0] == "msconvert"


def test_run_msconvert_timeout_raises(tmp_path: Path) -> None:
    import subprocess

    params = _base_params(str(tmp_path / "run.raw"))

    with mock.patch(
        "mzx.subprocess.run", side_effect=subprocess.TimeoutExpired("msconvert", 5)
    ):
        with pytest.raises(ConversionTerminatedAbnormally):
            run_msconvert(params, "/in/run.raw", "/out/run.mzML", timeout=5)
