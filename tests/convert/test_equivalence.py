"""Equivalence tests: native mzML vs reference (ProteoWizard-style) output."""

from __future__ import annotations

from pathlib import Path

import pytest

from mzx.convert.base import Spectrum
from mzx.convert.mzml_writer import parse_spectra_from_mzml, write_mzml

from tests.convert.helpers import FakeConverter, sample_spectra


def assert_spectra_equivalent(
    native: Spectrum,
    reference: Spectrum,
    *,
    mz_tol: float = 1e-6,
    int_tol: float = 1e-4,
    rt_tol: float = 0.01,
) -> None:
    assert native.ms_level == reference.ms_level
    assert abs(native.retention_time_sec - reference.retention_time_sec) < rt_tol
    assert len(native.mz) == len(reference.mz)
    assert len(native.intensity) == len(reference.intensity)
    for n_mz, r_mz in zip(native.mz, reference.mz):
        assert abs(n_mz - r_mz) < mz_tol
    for n_i, r_i in zip(native.intensity, reference.intensity):
        assert abs(n_i - r_i) < int_tol
    if native.precursor_mz is not None or reference.precursor_mz is not None:
        assert native.precursor_mz is not None and reference.precursor_mz is not None
        assert abs(native.precursor_mz - reference.precursor_mz) < mz_tol


def test_native_writer_matches_committed_reference(
    reference_mzml_path: Path, tmp_path: Path
) -> None:
    """Native writer output must match the committed synthetic reference spectra."""
    converter = FakeConverter()
    converter.open("fake.raw")
    spectra = list(converter.iter_spectra())
    chroms = list(converter.iter_chromatograms())
    meta = converter.metadata()
    converter.close()

    out = tmp_path / "native.mzML"
    write_mzml(out, spectra, chromatograms=chroms, metadata=meta)

    native = parse_spectra_from_mzml(out)
    reference = parse_spectra_from_mzml(reference_mzml_path)
    assert len(native) == len(reference) == 2
    for n, r in zip(native, reference):
        assert_spectra_equivalent(n, r)


def test_roundtrip_preserves_peak_arrays(tmp_path: Path) -> None:
    spectra = sample_spectra()
    out = tmp_path / "rt.mzML"
    write_mzml(out, spectra, metadata={"source_file": "x", "vendor": "fake"})
    parsed = parse_spectra_from_mzml(out)
    for n, r in zip(parsed, spectra):
        assert_spectra_equivalent(n, r)


@pytest.mark.skipif(
    not any((Path(__file__).parent / "fixtures" / "thermo").glob("*.raw")),
    reason="No Thermo fixture .raw present under tests/convert/fixtures/thermo",
)
def test_thermo_native_vs_proteowizard_reference(thermo_fixture_path, tmp_path: Path):
    """
    When a real Thermo .raw and matching ProteoWizard reference mzML exist,
    compare native conversion against the reference.
    """
    pytest.importorskip("opentfraw")
    from mzx.convert.vendors.thermo import ThermoConverter

    assert thermo_fixture_path is not None
    ref_candidates = list(
        (Path(__file__).parent / "fixtures" / "reference").glob(
            f"{thermo_fixture_path.stem}*.mzml.xml"
        )
    )
    if not ref_candidates:
        pytest.skip("No ProteoWizard reference mzML for Thermo fixture")

    conv = ThermoConverter()
    conv.open(str(thermo_fixture_path))
    try:
        spectra = list(conv.iter_spectra())
        meta = conv.metadata()
    finally:
        conv.close()

    out = tmp_path / "thermo_native.mzML"
    write_mzml(out, spectra, metadata=meta)
    native = parse_spectra_from_mzml(out)
    reference = parse_spectra_from_mzml(ref_candidates[0])

    assert len(native) == len(reference)
    # Compare a sample of scans for numerical equivalence
    for n, r in list(zip(native, reference))[:20]:
        assert_spectra_equivalent(n, r, mz_tol=1e-4, int_tol=1e-2, rt_tol=0.05)


@pytest.mark.skipif(
    not any(
        p.is_dir() and p.name.lower().endswith(".raw")
        for p in (Path(__file__).parent / "fixtures" / "waters").glob("*")
    ),
    reason="No Waters fixture .raw directory present",
)
def test_waters_native_vs_proteowizard_reference(waters_fixture_path, tmp_path: Path):
    pytest.importorskip("openwraw")
    from mzx.convert.vendors.waters import WatersConverter

    assert waters_fixture_path is not None
    ref_candidates = list(
        (Path(__file__).parent / "fixtures" / "reference").glob(
            f"{waters_fixture_path.stem}*.mzml.xml"
        )
    )
    if not ref_candidates:
        pytest.skip("No ProteoWizard reference mzML for Waters fixture")

    conv = WatersConverter()
    conv.open(str(waters_fixture_path))
    try:
        spectra = list(conv.iter_spectra())
        meta = conv.metadata()
    finally:
        conv.close()

    out = tmp_path / "waters_native.mzML"
    write_mzml(out, spectra, metadata=meta)
    native = parse_spectra_from_mzml(out)
    reference = parse_spectra_from_mzml(ref_candidates[0])
    assert len(native) == len(reference)
    for n, r in list(zip(native, reference))[:20]:
        assert_spectra_equivalent(n, r, mz_tol=1e-4, int_tol=1e-2, rt_tol=0.05)
