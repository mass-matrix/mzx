"""Tests for waters_lockmass_config() — the _extern.inf lockmass discovery.

No mocking needed; it reads the directory and returns params.
"""

from pathlib import Path

import pytest

from mzx import WatersConvertException, waters_lockmass_config


def _waters_params(infile: str, **overrides):
    p = {
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
    p.update(overrides)
    return p


def test_forces_mzml_and_index(tmp_path: Path) -> None:
    d = tmp_path / "waters_dir"
    d.mkdir()
    (d / "foo_extern.inf").write_text("no reference line\n", encoding="latin-1")

    out = waters_lockmass_config(_waters_params(str(d)))

    assert out["vendor"] == "waters"
    assert out["type"] == "mzml"
    assert out["index"] is True


def test_no_reference_line_leaves_lockmass_off(tmp_path: Path) -> None:
    d = tmp_path / "waters_dir"
    d.mkdir()
    (d / "foo_extern.inf").write_text("no reference line\n", encoding="latin-1")

    out = waters_lockmass_config(_waters_params(str(d)))

    assert out["lockmass"] is False
    assert out["lockmass_function_exclude"] is None


def test_reference_line_sets_exclude(tmp_path: Path) -> None:
    d = tmp_path / "w"
    d.mkdir()
    (d / "x_extern.inf").write_text(
        "REFERENCE Function 2 something\n", encoding="latin-1"
    )

    out = waters_lockmass_config(_waters_params(str(d), lockmass_disabled=False))

    assert out["lockmass"] is True
    assert out["lockmass_function_exclude"] == 2


def test_missing_extern_inf_raises(tmp_path: Path) -> None:
    d = tmp_path / "empty"
    d.mkdir()

    with pytest.raises(WatersConvertException, match="_extern.inf"):
        waters_lockmass_config(_waters_params(str(d)))
