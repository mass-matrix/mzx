"""Tests for the native mzML writer."""

from __future__ import annotations

from pathlib import Path

from lxml import etree

from mzx.convert.base import Spectrum
from mzx.convert.mzml_writer import MZML_NS, parse_spectra_from_mzml, write_mzml


def test_write_mzml_creates_valid_xml_structure(tmp_path: Path) -> None:
    spectra = [
        Spectrum(0, "scan=1", 1, 1.0, [100.0], [1.0], polarity="positive"),
    ]
    out = tmp_path / "a.mzML"
    write_mzml(out, spectra, metadata={"source_file": "a.raw", "vendor": "Thermo"})
    root = etree.parse(str(out)).getroot()
    assert root.tag == f"{{{MZML_NS}}}mzML"
    assert root.find(f"{{{MZML_NS}}}cvList") is not None
    run = root.find(f"{{{MZML_NS}}}run")
    assert run is not None
    spectrum_list = run.find(f"{{{MZML_NS}}}spectrumList")
    assert spectrum_list is not None
    assert spectrum_list.get("count") == "1"


def test_write_indexed_mzml_wrapper(tmp_path: Path) -> None:
    spectra = [Spectrum(0, "scan=1", 1, 1.0, [100.0], [1.0])]
    out = tmp_path / "idx.mzML"
    write_mzml(out, spectra, metadata={"source_file": "a.raw"}, indexed=True)
    root = etree.parse(str(out)).getroot()
    assert root.tag == f"{{{MZML_NS}}}indexedmzML"
    parsed = parse_spectra_from_mzml(out)
    assert len(parsed) == 1


def test_empty_spectrum_arrays(tmp_path: Path) -> None:
    spectra = [Spectrum(0, "scan=1", 1, 0.0, [], [])]
    out = tmp_path / "empty.mzML"
    write_mzml(out, spectra, metadata={"source_file": "a.raw"})
    parsed = parse_spectra_from_mzml(out)
    assert parsed[0].mz == []
    assert parsed[0].intensity == []
