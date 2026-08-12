"""Bruker timsTOF .d/ converter via optional opentimstdf package."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Iterator, Optional

from ..base import Chromatogram, Spectrum, VendorConverter


class BrukerConverter(VendorConverter):
    """Native Bruker timsTOF ``.d/`` TDF bundle converter.

    Reads Bruker acquisitions via the optional ``opentimstdf`` package.

    Install: ``pip install mzx[bruker]`` or ``pip install opentimstdf``.
    """

    vendor = "bruker"

    def __init__(self) -> None:
        self._path: Optional[str] = None
        self._reader: Any = None

    def open(self, path: str) -> None:
        path_obj = Path(path)
        if not path_obj.is_dir():
            raise NotADirectoryError(f"Bruker .d path must be a directory: {path}")
        try:
            import opentimstdf  # type: ignore
        except ImportError as exc:
            raise ImportError(
                "Bruker native conversion requires the optional dependency "
                "'opentimstdf'. Install with: pip install mzx[native] "
                "or pip install opentimstdf"
            ) from exc

        self._path = str(path_obj.resolve())
        if hasattr(opentimstdf, "Reader"):
            self._reader = opentimstdf.Reader(self._path)
        elif hasattr(opentimstdf, "open"):
            self._reader = opentimstdf.open(self._path)
        else:
            raise ImportError("opentimstdf does not expose Reader or open()")

    def iter_spectra(self) -> Iterator[Spectrum]:
        if self._reader is None:
            raise RuntimeError("BrukerConverter.open() must be called first")
        if hasattr(self._reader, "iter_spectra"):
            for i, spec in enumerate(self._reader.iter_spectra()):
                yield self._from_mapping(i, spec)
            return
        raise AttributeError(
            "opentimstdf reader does not expose iter_spectra(); "
            "frame-level APIs need an adapter before spectrum export"
        )

    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        if self._reader is None:
            raise RuntimeError("BrukerConverter.open() must be called first")
        if hasattr(self._reader, "iter_chromatograms"):
            for chrom in self._reader.iter_chromatograms():
                yield Chromatogram(
                    id=str(getattr(chrom, "id", "chrom")),
                    times=list(getattr(chrom, "times", [])),
                    intensities=list(getattr(chrom, "intensities", [])),
                )

    def metadata(self) -> dict[str, Any]:
        if self._path is None:
            raise RuntimeError("BrukerConverter.open() must be called first")
        return {
            "source_file": self._path,
            "vendor": self.vendor,
            "software": "mzx-native/opentimstdf",
            "instrument_model": "Bruker",
        }

    def close(self) -> None:
        if self._reader is not None and hasattr(self._reader, "close"):
            self._reader.close()
        self._reader = None
        self._path = None

    @staticmethod
    def _from_mapping(index: int, spec: Any) -> Spectrum:
        get = (
            spec.get
            if isinstance(spec, dict)
            else lambda k, d=None: getattr(spec, k, d)
        )
        return Spectrum(
            index=index,
            scan_id=str(get("id", get("scan_id", f"scan={index + 1}"))),
            ms_level=int(get("ms_level", get("msLevel", 1))),
            retention_time_sec=float(
                get("retention_time_sec", get("rt", get("retention_time", 0.0)))
            ),
            mz=list(get("mz", [])),
            intensity=list(get("intensity", get("intensities", []))),
            polarity=get("polarity"),
            precursor_mz=get("precursor_mz"),
            precursor_charge=get("precursor_charge"),
            collision_energy=get("collision_energy"),
        )
