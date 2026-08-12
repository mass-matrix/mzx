"""Contract tests for VendorConverter implementations."""

from __future__ import annotations

from pathlib import Path
from unittest import mock

import pytest

from mzx.convert import (
    Spectrum,
    convert_native,
    detect_vendor,
    get_converter,
    list_converters,
    write_mzml,
)
from mzx.convert.base import VendorConverter
from mzx.convert.vendors.thermo import ThermoConverter
from mzx.convert.vendors.waters import WatersConverter

from tests.convert.helpers import FakeConverter, sample_spectra


def _assert_converter_contract(converter: VendorConverter, path: str) -> None:
    converter.open(path)
    try:
        spectra = list(converter.iter_spectra())
        assert spectra, "converter must yield at least one spectrum"
        for i, spectrum in enumerate(spectra):
            assert isinstance(spectrum, Spectrum)
            assert spectrum.index == i or spectrum.index >= 0
            assert spectrum.scan_id
            assert spectrum.ms_level >= 1
            assert spectrum.retention_time_sec >= 0
            assert len(spectrum.mz) == len(spectrum.intensity)

        chroms = list(converter.iter_chromatograms())
        for chrom in chroms:
            assert chrom.id
            assert len(chrom.times) == len(chrom.intensities)

        meta = converter.metadata()
        assert "source_file" in meta
        assert "vendor" in meta
    finally:
        converter.close()


def test_fake_converter_satisfies_contract(
    fake_converter: FakeConverter, tmp_path: Path
):
    _assert_converter_contract(fake_converter, str(tmp_path / "fake.raw"))


def test_list_converters_includes_core_vendors() -> None:
    names = set(list_converters())
    assert {"Thermo", "waters", "Agilent", "bruker"} <= names


def test_get_converter_unknown_raises() -> None:
    with pytest.raises(KeyError):
        get_converter("not-a-vendor")


@pytest.mark.parametrize(
    "path,expected",
    [
        ("run.raw", "Thermo"),
        ("sample.d", "Agilent"),
    ],
)
def test_detect_vendor_files(path: str, expected: str, tmp_path: Path) -> None:
    f = tmp_path / path
    f.write_text("x")
    assert detect_vendor(str(f)) == expected


def test_detect_vendor_waters_dir(tmp_path: Path) -> None:
    d = tmp_path / "x.raw"
    d.mkdir()
    assert detect_vendor(str(d)) == "waters"


def test_detect_vendor_bruker_tdf(tmp_path: Path) -> None:
    d = tmp_path / "run.d"
    d.mkdir()
    (d / "analysis.tdf").write_text("sqlite")
    assert detect_vendor(str(d)) == "bruker"


def test_detect_vendor_agilent_acqdata(tmp_path: Path) -> None:
    d = tmp_path / "run.d"
    d.mkdir()
    (d / "AcqData").mkdir()
    assert detect_vendor(str(d)) == "Agilent"


def test_thermo_converter_requires_opentfraw(tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")
    conv = ThermoConverter()
    import builtins

    real_import = builtins.__import__

    def _import(name, *args, **kwargs):
        if name == "opentfraw":
            raise ImportError("missing opentfraw")
        return real_import(name, *args, **kwargs)

    with mock.patch("builtins.__import__", side_effect=_import):
        with pytest.raises(ImportError, match="opentfraw"):
            conv.open(str(f))


def test_thermo_converter_with_mock_opentfraw(tmp_path: Path) -> None:
    f = tmp_path / "a.raw"
    f.write_text("x")

    class FakeRaw:
        n_scans = 1

        def peaks(self, scan_number):
            return [100.0, 200.0], [1.0, 2.0]

        def scan(self, scan_number):
            return {
                "id": f"scan={scan_number}",
                "ms_level": 1,
                "retention_time_sec": 12.0,
                "polarity": "positive",
            }

        def close(self):
            pass

    fake_mod = mock.Mock()
    fake_mod.RawFile = mock.Mock(return_value=FakeRaw())
    with mock.patch.dict("sys.modules", {"opentfraw": fake_mod}):
        conv = ThermoConverter()
        _assert_converter_contract(conv, str(f))


def test_waters_chromatogram_fallback_without_openwraw(
    waters_chrom_raw_dir: Path,
) -> None:
    conv = WatersConverter()
    with mock.patch.dict("sys.modules", {"openwraw": None}):
        import builtins

        real_import = builtins.__import__

        def _import(name, *args, **kwargs):
            if name == "openwraw":
                raise ImportError("missing")
            return real_import(name, *args, **kwargs)

        with mock.patch("builtins.__import__", side_effect=_import):
            conv.open(str(waters_chrom_raw_dir))
    chroms = list(conv.iter_chromatograms())
    assert len(chroms) == 1
    assert chroms[0].id == "TUV 260"
    assert abs(chroms[0].times[0] - 30.0) < 1e-4
    with pytest.raises(ImportError, match="openwraw"):
        list(conv.iter_spectra())
    conv.close()


def test_convert_native_with_fake_registry(tmp_path: Path, monkeypatch) -> None:
    fake_path = tmp_path / "input.raw"
    fake_path.write_text("x")

    def _get_converter(vendor: str):
        return FakeConverter()

    monkeypatch.setattr("mzx.convert.get_converter", _get_converter)
    monkeypatch.setattr("mzx.convert.detect_vendor", lambda p: "Thermo")

    # Patch convert_native's imported get_converter
    import mzx.convert as convert_mod

    monkeypatch.setattr(convert_mod, "get_converter", _get_converter)

    params = {
        "infile": str(fake_path),
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
    out = convert_native(params)  # type: ignore[arg-type]
    assert Path(out).exists()
    text = Path(out).read_text()
    assert "mzML" in text
    assert "scan=1" in text


def test_write_mzml_roundtrip(tmp_path: Path) -> None:
    from mzx.convert.mzml_writer import parse_spectra_from_mzml

    spectra = sample_spectra()
    out = tmp_path / "round.mzML"
    write_mzml(
        out,
        spectra,
        metadata={"source_file": "x.raw", "vendor": "fake"},
    )
    parsed = parse_spectra_from_mzml(out)
    assert len(parsed) == 2
    assert parsed[0].ms_level == 1
    assert abs(parsed[0].mz[1] - 200.0) < 1e-9
    assert abs(parsed[1].retention_time_sec - 61.5) < 1e-6
    assert parsed[1].precursor_mz == 200.0
