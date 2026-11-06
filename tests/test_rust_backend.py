"""Tests for the optional Rust backend and backend selection in convert_raw_file."""

import os
from pathlib import Path
from unittest import mock

import pytest

from mzx import RawFileConversionError, convert_raw_file, rust_backend


def _params(infile: str, vendor: str = "Thermo", **overrides):
    params = {
        "infile": infile,
        "index": False,
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
    params.update(overrides)
    return params


class FakeUnsupported(Exception):
    pass


class FakeConversionError(Exception):
    pass


def _fake_mzx_rs(side_effect=None) -> mock.Mock:
    m = mock.Mock()
    m.Unsupported = FakeUnsupported
    m.ConversionError = FakeConversionError
    m.convert.side_effect = side_effect
    return m


@pytest.mark.parametrize(
    ("overrides", "expected"),
    [
        ({}, "default"),
        ({"type": "mzML"}, "default"),
        ({"peak_picking": "off", "remove_zeros": False}, "nofilter"),
        ({"index": True}, None),
        ({"sortbyscan": True}, None),
        ({"lockmass": True}, None),
        ({"type": "mgf"}, None),
        ({"peak_picking": "all"}, "pickall_rz"),
        ({"peak_picking": "all", "remove_zeros": False}, "pickall"),
        ({"peak_picking": "ms1"}, None),
        ({"remove_zeros": False}, None),
        ({"peak_picking": "off"}, None),
    ],
)
def test_option_set_maps_only_proven_options(overrides, expected) -> None:
    assert rust_backend.option_set(_params("/x/a.raw", **overrides)) == expected


@mock.patch.object(rust_backend, "_mzx_rs", None)
def test_convert_unavailable_raises_unsupported() -> None:
    assert not rust_backend.available()
    with pytest.raises(rust_backend.Unsupported, match="not installed"):
        rust_backend.convert("/x/a.raw", "/x/a.mzML", _params("/x/a.raw"))


def test_convert_passes_option_set_and_translates_errors() -> None:
    fake = _fake_mzx_rs()
    with mock.patch.object(rust_backend, "_mzx_rs", fake):
        out = rust_backend.convert("/x/a.raw", "/x/a.mzML", _params("/x/a.raw"))
        assert out == "/x/a.mzML"
        args = fake.convert.call_args[0]
        assert args[:3] == ("/x/a.raw", "/x/a.mzML", "default")

        fake.convert.side_effect = FakeUnsupported("ion trap centroiding")
        with pytest.raises(rust_backend.Unsupported, match="ion trap"):
            rust_backend.convert("/x/a.raw", "/x/a.mzML", _params("/x/a.raw"))

        fake.convert.side_effect = FakeConversionError("truncated")
        with pytest.raises(rust_backend.RustConversionError, match="truncated"):
            rust_backend.convert("/x/a.raw", "/x/a.mzML", _params("/x/a.raw"))


def test_convert_refuses_unproven_options_without_calling_rust() -> None:
    fake = _fake_mzx_rs()
    with mock.patch.object(rust_backend, "_mzx_rs", fake):
        with pytest.raises(rust_backend.Unsupported, match="option sets"):
            rust_backend.convert(
                "/x/a.raw", "/x/a.mzML", _params("/x/a.raw", index=True)
            )
    fake.convert.assert_not_called()


@mock.patch("mzx.msconvert")
@mock.patch("mzx.rust_backend.convert")
def test_auto_uses_rust_at_msconvert_output_path(
    mock_rust, mock_ms, tmp_path: Path
) -> None:
    raw = tmp_path / "run.raw"
    raw.write_text("x")
    out = convert_raw_file(_params(str(raw), outfile="named.mzML"))
    assert out == os.path.join(str(tmp_path), "named.mzML")
    mock_rust.assert_called_once()
    assert mock_rust.call_args[0][:2] == (str(raw), out)
    mock_ms.assert_not_called()


@pytest.mark.parametrize(
    "error", [rust_backend.Unsupported("no"), rust_backend.RustConversionError("bad")]
)
@mock.patch("mzx.msconvert", return_value="/out/docker.mzML")
@mock.patch("mzx.rust_backend.convert")
def test_auto_falls_back_to_msconvert(mock_rust, mock_ms, error, tmp_path) -> None:
    mock_rust.side_effect = error
    out = convert_raw_file(_params(str(tmp_path / "run.raw")))
    assert out == "/out/docker.mzML"
    mock_ms.assert_called_once()


@pytest.mark.parametrize(
    "error", [rust_backend.Unsupported("no"), rust_backend.RustConversionError("bad")]
)
@mock.patch("mzx.msconvert")
@mock.patch("mzx.rust_backend.convert")
def test_rust_backend_never_falls_back(mock_rust, mock_ms, error, tmp_path) -> None:
    mock_rust.side_effect = error
    with pytest.raises(RawFileConversionError, match="Rust backend"):
        convert_raw_file(_params(str(tmp_path / "run.raw"), backend="rust"))
    mock_ms.assert_not_called()


@mock.patch("mzx.msconvert", return_value="/out/docker.mzML")
@mock.patch("mzx.rust_backend.convert")
def test_docker_backend_skips_rust(mock_rust, mock_ms, tmp_path) -> None:
    out = convert_raw_file(_params(str(tmp_path / "run.raw"), backend="docker"))
    assert out == "/out/docker.mzML"
    mock_rust.assert_not_called()


@pytest.mark.parametrize("vendor", ["Agilent", "bruker"])
@mock.patch("mzx.msconvert", return_value="/out/docker.mzML")
@mock.patch("mzx.rust_backend.convert")
def test_auto_uses_msconvert_for_other_vendors(
    mock_rust, mock_ms, vendor, tmp_path
) -> None:
    convert_raw_file(_params(str(tmp_path / "run.d"), vendor=vendor))
    mock_rust.assert_not_called()
    mock_ms.assert_called_once()


@mock.patch("mzx.msconvert")
def test_rust_backend_rejects_other_vendors(mock_ms, tmp_path) -> None:
    with pytest.raises(RawFileConversionError, match="does not support"):
        convert_raw_file(_params(str(tmp_path / "run.d"), "bruker", backend="rust"))
    mock_ms.assert_not_called()


def test_unknown_backend_is_rejected(tmp_path) -> None:
    with pytest.raises(RawFileConversionError, match="Unknown backend"):
        convert_raw_file(_params(str(tmp_path / "run.raw"), backend="wasm"))
