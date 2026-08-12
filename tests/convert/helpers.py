"""Test helpers for native conversion contract tests."""

from __future__ import annotations

from typing import Any, Iterator

from mzx.convert.base import Chromatogram, Spectrum, VendorConverter


def sample_spectra() -> list[Spectrum]:
    return [
        Spectrum(
            index=0,
            scan_id="scan=1",
            ms_level=1,
            retention_time_sec=60.0,
            mz=[100.0, 200.0, 300.0],
            intensity=[10.0, 50.0, 20.0],
            polarity="positive",
        ),
        Spectrum(
            index=1,
            scan_id="scan=2",
            ms_level=2,
            retention_time_sec=61.5,
            mz=[80.0, 150.0],
            intensity=[5.0, 25.0],
            polarity="positive",
            precursor_mz=200.0,
            precursor_charge=2,
            collision_energy=25.0,
        ),
    ]


def sample_chromatograms() -> list[Chromatogram]:
    return [
        Chromatogram(
            id="TIC",
            times=[60.0, 61.5],
            intensities=[80.0, 30.0],
        )
    ]


class FakeConverter(VendorConverter):
    """In-memory converter used to drive the VendorConverter contract."""

    vendor = "fake"

    def __init__(
        self,
        spectra: list[Spectrum] | None = None,
        chromatograms: list[Chromatogram] | None = None,
        meta: dict[str, Any] | None = None,
    ) -> None:
        self._spectra = spectra if spectra is not None else sample_spectra()
        self._chromatograms = (
            chromatograms if chromatograms is not None else sample_chromatograms()
        )
        self._meta = meta or {
            "source_file": "fake.raw",
            "vendor": "fake",
            "instrument_model": "FakeInstrument",
            "software": "mzx-test",
        }
        self._opened = False
        self._path: str | None = None

    def open(self, path: str) -> None:
        self._path = path
        self._opened = True
        self._meta["source_file"] = path

    def iter_spectra(self) -> Iterator[Spectrum]:
        if not self._opened:
            raise RuntimeError("not opened")
        yield from self._spectra

    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        if not self._opened:
            raise RuntimeError("not opened")
        yield from self._chromatograms

    def metadata(self) -> dict[str, Any]:
        if not self._opened:
            raise RuntimeError("not opened")
        return dict(self._meta)

    def close(self) -> None:
        self._opened = False
