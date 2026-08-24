"""Tests for the transport-agnostic argument builder."""

import pytest

from mzx import build_msconvert_args, output_filename


def _base_params(infile: str = "/data/run.raw", **overrides):
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


@pytest.mark.parametrize(
    "type_, expected",
    [("mzml", "run.mzML"), ("mgf", "run.mgf"), ("mzxml", "run.mzXML")],
)
def test_output_filename_follows_type(type_, expected):
    assert output_filename(_base_params(type=type_)) == expected


def test_output_filename_prefers_outfile_basename():
    params = _base_params(outfile="/somewhere/else/custom.mzML")
    assert output_filename(params) == "custom.mzML"


def test_output_filename_handles_vendor_directory():
    params = _base_params(infile="/data/sample.raw/")
    assert output_filename(params) == "sample.mzML"


def test_args_include_format_and_outfile():
    args = build_msconvert_args(_base_params(), "/out/run.mzML")
    assert args[0] == "--mzML"
    assert args[args.index("--outfile") + 1] == "/out/run.mzML"


def test_args_omit_input_file():
    """Runners position the input themselves."""
    args = build_msconvert_args(_base_params(), "/out/run.mzML")
    assert "run.raw" not in args


@pytest.mark.parametrize(
    "peak_picking, expected",
    [
        ("all", "peakPicking true 1-"),
        ("ms1", "peakPicking true 1"),
        ("msms", "peakPicking true 2-"),
    ],
)
def test_peak_picking_filters(peak_picking, expected):
    args = build_msconvert_args(
        _base_params(peak_picking=peak_picking), "/out/run.mzML"
    )
    assert expected in args


def test_peak_picking_off_adds_no_filter():
    args = build_msconvert_args(_base_params(peak_picking="off"), "/out/run.mzML")
    assert not any("peakPicking" in a for a in args)


def test_noindex_and_sort():
    args = build_msconvert_args(
        _base_params(index=False, sortbyscan=True), "/out/run.mzML"
    )
    assert "--noindex" in args
    assert "sortByScanTime" in args


def test_remove_zeros_false_skips_filter():
    args = build_msconvert_args(_base_params(remove_zeros=False), "/out/run.mzML")
    assert not any("zeroSamples" in a for a in args)


def test_lockmass_defaults_when_values_none():
    args = build_msconvert_args(_base_params(lockmass=True), "/out/run.mzML")
    assert "lockmassRefiner mz=556.2771 mzNegIons=554.2615 tol=0.1" in args


def test_lockmass_scan_event_exclusion():
    args = build_msconvert_args(
        _base_params(lockmass=True, lockmass_function_exclude=3), "/out/run.mzML"
    )
    assert "scanEvent 1-2 4-" in args


def test_filter_values_are_single_arguments():
    """Each --filter takes one argv entry; splitting them breaks msconvert."""
    args = build_msconvert_args(_base_params(), "/out/run.mzML")
    for i, arg in enumerate(args):
        if arg == "--filter":
            assert " " in args[i + 1] or args[i + 1].isalnum()
