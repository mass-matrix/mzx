"""Tests for convert_file native routing."""

from pathlib import Path
from unittest import mock

from mzx import convert_file


def test_convert_file_native_flag_routes_to_convert_native(tmp_path: Path) -> None:
    params = {
        "infile": str(tmp_path / "a.raw"),
        "index": False,
        "sortbyscan": False,
        "peak_picking": "off",
        "remove_zeros": False,
        "vendor": "Thermo",
        "outfile": str(tmp_path / "out.mzML"),
        "type": "mzml",
        "overwrite": True,
        "debug": False,
        "verbose": False,
        "lockmass_disabled": True,
        "lockmass": False,
        "neg_lockmass": None,
        "pos_lockmass": None,
        "lockmass_tolerance": None,
        "lockmass_function_exclude": None,
    }
    with mock.patch("mzx.convert.convert_native", return_value="/tmp/out.mzML") as m:
        out = convert_file(params, native=True)  # type: ignore[arg-type]
    assert out == "/tmp/out.mzML"
    m.assert_called_once()


def test_convert_file_default_uses_docker_path(tmp_path: Path) -> None:
    params = {
        "infile": str(tmp_path / "a.raw"),
        "index": False,
        "sortbyscan": False,
        "peak_picking": "off",
        "remove_zeros": False,
        "vendor": "Thermo",
        "outfile": None,
        "type": "mzml",
        "overwrite": False,
        "debug": False,
        "verbose": False,
        "lockmass_disabled": True,
        "lockmass": False,
        "neg_lockmass": None,
        "pos_lockmass": None,
        "lockmass_tolerance": None,
        "lockmass_function_exclude": None,
    }
    with mock.patch("mzx.convert_raw_file", return_value="/docker/out.mzML") as m:
        out = convert_file(params, native=False)  # type: ignore[arg-type]
    assert out == "/docker/out.mzML"
    m.assert_called_once()
