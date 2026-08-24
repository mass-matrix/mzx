"""Tests for the convert_raw_file / waters_convert docker aliases."""

from pathlib import Path
from unittest import mock

import pytest

from mzx import RawFileConversionError, convert_raw_file, waters_convert

MZML_BODY = b'<?xml version="1.0"?><mzML><run/></mzML>'


def _minimal_params(infile: str, vendor: str):
    return {
        "infile": infile,
        "index": True,
        "sortbyscan": False,
        "peak_picking": "msms",
        "remove_zeros": True,
        "vendor": vendor,
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


def _writes(params, infile, outfile, **kwargs):
    Path(outfile).write_bytes(MZML_BODY)
    return 0


@pytest.mark.parametrize("vendor", ["Thermo", "thermo", "Agilent", "bruker", "Bruker"])
@mock.patch("mzx.run_msconvert_docker", side_effect=_writes)
def test_convert_raw_file_routes_through_docker(
    mock_run, tmp_path: Path, vendor: str
) -> None:
    f = tmp_path / "s.raw"
    f.write_text("x")

    out = convert_raw_file(_minimal_params(str(f), vendor))

    assert out == str(tmp_path / "s.mzML")
    mock_run.assert_called_once()


@mock.patch("mzx.run_msconvert_docker", side_effect=_writes)
def test_convert_raw_file_unspecified_still_converts(mock_run, tmp_path: Path) -> None:
    f = tmp_path / "s.txt"
    f.write_text("x")

    convert_raw_file(_minimal_params(str(f), "unspecified"))

    mock_run.assert_called_once()


@mock.patch("mzx.run_msconvert_docker", side_effect=_writes)
def test_waters_convert_reads_lockmass_and_uses_docker(
    mock_run, tmp_path: Path
) -> None:
    d = tmp_path / "w.raw"
    d.mkdir()
    (d / "x_extern.inf").write_text(
        "REFERENCE Function 2 something\n", encoding="latin-1"
    )

    waters_convert(_minimal_params(str(d), "waters"))

    passed = mock_run.call_args[0][0]
    assert passed["lockmass"] is True
    assert passed["lockmass_function_exclude"] == 2


def test_convert_raw_file_waters_propagates_as_raw_file_error(tmp_path: Path) -> None:
    with pytest.raises(RawFileConversionError, match="_extern.inf"):
        convert_raw_file(_minimal_params(str(tmp_path), "waters"))


def test_convert_raw_file_unsupported_vendor() -> None:
    with pytest.raises(RawFileConversionError, match="Unsupported vendor"):
        convert_raw_file(_minimal_params("/tmp/fake.raw", "unknown_vendor"))
