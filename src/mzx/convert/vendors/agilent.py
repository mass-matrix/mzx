"""Agilent MassHunter .d/ converter via optional openaraw package."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Iterator, Optional

from ..base import Chromatogram, Spectrum, VendorConverter


class AgilentConverter(VendorConverter):
    """Native Agilent MassHunter ``.d/`` directory converter.

    Reads Agilent acquisitions via the optional ``openaraw`` package.

    Install: ``pip install mzx[agilent]`` or ``pip install openaraw``.
    """

    vendor = "Agilent"

    def __init__(self) -> None:
        self._path: Optional[str] = None
        self._reader: Any = None

    def open(self, path: str) -> None:
        path_obj = Path(path)
        if not path_obj.exists():
            raise FileNotFoundError(f"Agilent .d path not found: {path}")
        try:
            import openaraw  # type: ignore
        except ImportError as exc:
            raise ImportError(
                "Agilent native conversion requires the optional dependency "
                "'openaraw'. Install with: pip install mzx[native] "
                "or pip install openaraw"
            ) from exc

        self._path = str(path_obj.resolve())
        if hasattr(openaraw, "RawReader"):
            self._reader = openaraw.RawReader(self._path)
        elif hasattr(openaraw, "open"):
            self._reader = openaraw.open(self._path)
        else:
            raise ImportError("openaraw does not expose RawReader or open()")

    def iter_spectra(self) -> Iterator[Spectrum]:
        if self._reader is None:
            raise RuntimeError("AgilentConverter.open() must be called first")
        if hasattr(self._reader, "iter_spectra"):
            for i, spec in enumerate(self._reader.iter_spectra()):
                yield self._from_mapping(i, spec)
            return
        if hasattr(self._reader, "read_spectrum"):
            i = 0
            while True:
                try:
                    spec = self._reader.read_spectrum(i)
                except Exception:
                    break
                if spec is None:
                    break
                yield self._from_mapping(i, spec)
                i += 1
            return
        raise AttributeError("openaraw reader does not expose a supported spectrum API")

    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        if self._reader is None:
            raise RuntimeError("AgilentConverter.open() must be called first")
        if hasattr(self._reader, "iter_chromatograms"):
            for chrom in self._reader.iter_chromatograms():
                yield Chromatogram(
                    id=str(getattr(chrom, "id", "chrom")),
                    times=list(getattr(chrom, "times", [])),
                    intensities=list(getattr(chrom, "intensities", [])),
                )

    def metadata(self) -> dict[str, Any]:
        if self._path is None:
            raise RuntimeError("AgilentConverter.open() must be called first")
        return {
            "source_file": self._path,
            "vendor": self.vendor,
            "software": "mzx-native/openaraw",
            "instrument_model": "Agilent",
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
                get(
                    "retention_time_sec",
                    get("rt", get("retention_time", 0.0)),
                )
            ),
            mz=list(get("mz", [])),
            intensity=list(get("intensity", get("intensities", []))),
            polarity=get("polarity"),
            precursor_mz=get("precursor_mz"),
            precursor_charge=get("precursor_charge"),
            collision_energy=get("collision_energy"),
        )
