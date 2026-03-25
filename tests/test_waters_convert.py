"""Tests for waters_convert() happy path (msconvert mocked)."""

from pathlib import Path
from unittest import mock

from mzx import waters_convert


def _waters_params(infile: str):
    return {
        "infile": infile,
        "index": False,
        "sortbyscan": False,
        "peak_picking": "msms",
        "remove_zeros": True,
        "vendor": "waters",
        "outfile": None,
        "type": "mzml",
        "overwrite": False,
        "debug": False,
        "verbose": False,
        "lockmass": None,
        "lockmass_disabled": True,
        "lockmass_function_exclude": None,
        "lockmass_tolerance": None,
        "neg_lockmass": None,
        "pos_lockmass": None,
    }


@mock.patch("mzx.msconvert", return_value="/tmp/out/dir/file.mzML")
def test_waters_convert_with_extern_inf_calls_msconvert(
    mock_ms, tmp_path: Path
) -> None:
    d = tmp_path / "waters_dir"
    d.mkdir()
    (d / "foo_extern.inf").write_text("no reference line\n", encoding="latin-1")
    out = waters_convert(_waters_params(str(d)))
    assert out == "/tmp/out/dir/file.mzML"
    mock_ms.assert_called_once()
    passed = mock_ms.call_args[0][0]
    assert passed["vendor"] == "waters"
    assert passed["type"] == "mzml"
    assert passed["index"] is True


@mock.patch("mzx.msconvert", return_value="/out.mzML")
def test_waters_convert_lockmass_reference_line_sets_exclude(
    mock_ms, tmp_path: Path
) -> None:
    d = tmp_path / "w"
    d.mkdir()
    (d / "x_extern.inf").write_text(
        "REFERENCE Function 2 something\n", encoding="latin-1"
    )
    p = _waters_params(str(d))
    p["lockmass_disabled"] = False
    waters_convert(p)
    passed = mock_ms.call_args[0][0]
    assert passed["lockmass"] is True
    assert passed["lockmass_function_exclude"] == 2
