"""Tests for convert_raw_file vendor routing."""

from pathlib import Path
from unittest import mock

import pytest

from mzx import RawFileConversionError, convert_raw_file


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


@pytest.mark.parametrize(
    "vendor",
    ["Thermo", "thermo", "Agilent", "bruker", "Bruker"],
)
@mock.patch("mzx.msconvert", return_value="/out/file.mzML")
def test_convert_raw_file_delegates_to_msconvert(
    mock_ms, tmp_path: Path, vendor: str
) -> None:
    f = tmp_path / "s.raw"
    f.write_text("x")
    out = convert_raw_file(_minimal_params(str(f), vendor))
    assert out == "/out/file.mzML"
    mock_ms.assert_called_once()


@mock.patch("mzx.msconvert", return_value="/out/u.mzML")
def test_convert_raw_file_unspecified_uses_msconvert(mock_ms, tmp_path: Path) -> None:
    f = tmp_path / "s.txt"
    f.write_text("x")
    convert_raw_file(_minimal_params(str(f), "unspecified"))
    mock_ms.assert_called_once()


@mock.patch("mzx.waters_convert", return_value="/out/w.mzML")
def test_convert_raw_file_waters(mock_wc, tmp_path: Path) -> None:
    f = tmp_path / "w.raw"
    f.write_text("x")
    out = convert_raw_file(_minimal_params(str(f), "waters"))
    assert out == "/out/w.mzML"
    mock_wc.assert_called_once()


def test_convert_raw_file_waters_propagates_as_raw_file_error(tmp_path: Path) -> None:
    with pytest.raises(RawFileConversionError, match="_extern.inf"):
        convert_raw_file(_minimal_params(str(tmp_path), "waters"))


def test_convert_raw_file_unsupported_vendor() -> None:
    with pytest.raises(RawFileConversionError, match="Unsupported vendor"):
        convert_raw_file(_minimal_params("/tmp/fake.raw", "unknown_vendor"))
