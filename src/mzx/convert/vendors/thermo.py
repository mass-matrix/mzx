"""Thermo Fisher .raw converter via optional opentfraw package."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Iterator, Optional

from ..base import Chromatogram, Spectrum, VendorConverter


class ThermoConverter(VendorConverter):
    """Native Thermo Fisher ``.raw`` file converter.

    Reads Thermo acquisitions via the optional ``opentfraw`` package.

    Install: ``pip install mzx[thermo]`` or ``pip install opentfraw``.
    """

    vendor = "Thermo"

    def __init__(self) -> None:
        self._path: Optional[str] = None
        self._raw: Any = None

    def open(self, path: str) -> None:
        path_obj = Path(path)
        if not path_obj.is_file():
            raise FileNotFoundError(f"Thermo .raw file not found: {path}")
        try:
            import opentfraw  # type: ignore
        except ImportError as exc:
            raise ImportError(
                "Thermo native conversion requires the optional dependency "
                "'opentfraw'. Install with: pip install mzx[native] "
                "or pip install opentfraw"
            ) from exc

        self._path = str(path_obj.resolve())
        # Support both RawFile(path) constructor styles used across versions.
        if hasattr(opentfraw, "RawFile"):
            self._raw = opentfraw.RawFile(self._path)
        elif hasattr(opentfraw, "open"):
            self._raw = opentfraw.open(self._path)
        else:
            raise ImportError("opentfraw does not expose RawFile or open()")

    def iter_spectra(self) -> Iterator[Spectrum]:
        if self._raw is None:
            raise RuntimeError("ThermoConverter.open() must be called first")

        # Prefer a high-level spectrum iterator when available.
        if hasattr(self._raw, "iter_spectra"):
            for i, spec in enumerate(self._raw.iter_spectra()):
                yield self._from_mapping(i, spec)
            return

        n_scans = self._scan_count()
        for i in range(n_scans):
            scan_number = i + 1
            mz, intensity = self._peaks(scan_number)
            info = self._scan_info(scan_number)
            yield Spectrum(
                index=i,
                scan_id=str(info.get("id", f"scan={scan_number}")),
                ms_level=int(info.get("ms_level", info.get("msLevel", 1))),
                retention_time_sec=float(
                    info.get(
                        "retention_time_sec",
                        info.get("rt", info.get("retention_time", 0.0)),
                    )
                ),
                mz=list(mz),
                intensity=list(intensity),
                polarity=info.get("polarity"),
                precursor_mz=info.get("precursor_mz"),
                precursor_charge=info.get("precursor_charge"),
                collision_energy=info.get("collision_energy"),
            )

    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        if self._raw is None:
            raise RuntimeError("ThermoConverter.open() must be called first")
        if hasattr(self._raw, "iter_chromatograms"):
            for chrom in self._raw.iter_chromatograms():
                yield Chromatogram(
                    id=str(getattr(chrom, "id", chrom.get("id", "chrom"))),
                    times=list(getattr(chrom, "times", chrom.get("times", []))),
                    intensities=list(
                        getattr(chrom, "intensities", chrom.get("intensities", []))
                    ),
                )

    def metadata(self) -> dict[str, Any]:
        if self._path is None:
            raise RuntimeError("ThermoConverter.open() must be called first")
        meta: dict[str, Any] = {
            "source_file": self._path,
            "vendor": self.vendor,
            "software": "mzx-native/opentfraw",
        }
        if self._raw is not None and hasattr(self._raw, "instrument_model"):
            meta["instrument_model"] = self._raw.instrument_model
        elif self._raw is not None and hasattr(self._raw, "metadata"):
            raw_meta = self._raw.metadata()
            if isinstance(raw_meta, dict):
                meta.update(raw_meta)
        return meta

    def close(self) -> None:
        if self._raw is not None and hasattr(self._raw, "close"):
            self._raw.close()
        self._raw = None
        self._path = None

    def _scan_count(self) -> int:
        assert self._raw is not None
        for attr in ("n_scans", "scan_count", "num_spectra"):
            if hasattr(self._raw, attr):
                value = getattr(self._raw, attr)
                return int(value() if callable(value) else value)
        if hasattr(self._raw, "__len__"):
            return len(self._raw)
        raise AttributeError("Cannot determine Thermo scan count from opentfraw object")

    def _peaks(self, scan_number: int) -> tuple[list[float], list[float]]:
        assert self._raw is not None
        if hasattr(self._raw, "peaks"):
            mz, intensity = self._raw.peaks(scan_number)
            return list(mz), list(intensity)
        raise AttributeError("opentfraw object has no peaks() method")

    def _scan_info(self, scan_number: int) -> dict[str, Any]:
        assert self._raw is not None
        if hasattr(self._raw, "scan"):
            info = self._raw.scan(scan_number)
            return dict(info) if not isinstance(info, dict) else info
        return {"id": f"scan={scan_number}", "ms_level": 1, "retention_time_sec": 0.0}

    @staticmethod
    def _from_mapping(index: int, spec: Any) -> Spectrum:
        if isinstance(spec, Spectrum):
            return Spectrum(
                index=index,
                scan_id=spec.scan_id,
                ms_level=spec.ms_level,
                retention_time_sec=spec.retention_time_sec,
                mz=list(spec.mz),
                intensity=list(spec.intensity),
                polarity=spec.polarity,
                precursor_mz=spec.precursor_mz,
                precursor_charge=spec.precursor_charge,
                collision_energy=spec.collision_energy,
            )
        get = spec.get if isinstance(spec, dict) else lambda k, d=None: getattr(spec, k, d)
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
