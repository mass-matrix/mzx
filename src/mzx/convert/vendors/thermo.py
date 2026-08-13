"""Thermo Fisher .raw converter via optional opentfraw package."""

from __future__ import annotations

from pathlib import Path
from typing import Any, Iterator, Optional

from ..base import Chromatogram, Polarity, Spectrum, VendorConverter


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
        if hasattr(opentfraw, "RawFile"):
            self._raw = opentfraw.RawFile(self._path)
        elif hasattr(opentfraw, "open"):
            self._raw = opentfraw.open(self._path)
        else:
            raise ImportError("opentfraw does not expose RawFile or open()")

    def iter_spectra(self) -> Iterator[Spectrum]:
        if self._raw is None:
            raise RuntimeError("ThermoConverter.open() must be called first")

        n_scans = self._scan_count()
        for i in range(n_scans):
            scan_number = i + 1
            info = self._scan_info(scan_number)

            mz_arr = info.get("mz", [])
            int_arr = info.get("intensity", [])
            mz = list(mz_arr) if hasattr(mz_arr, "__iter__") else []
            intensity = list(int_arr) if hasattr(int_arr, "__iter__") else []

            # Non-empty vendor arrays are centroided peaks; fall back to raw
            # profile only when no vendor centroids are stored for the scan.
            is_centroid = len(mz) > 0
            if len(mz) == 0 and hasattr(self._raw, "profile"):
                profile_mz, profile_int = self._raw.profile(scan_number)
                mz = list(profile_mz)
                intensity = list(profile_int)
                is_centroid = False

            polarity = self._parse_polarity(info.get("polarity"))

            precursor_mz = info.get("precursor_mz")
            if precursor_mz == 0.0:
                precursor_mz = None

            precursor_charge = info.get("charge")
            if precursor_charge == 0:
                precursor_charge = None

            collision_energy = info.get("collision_energy")

            tic = info.get("total_ion_current")
            base_mz = info.get("base_peak_mz")
            base_int = info.get("base_peak_intensity")

            filter_str = info.get("filter_string")
            ion_inject = info.get("ion_injection_time_ms")
            low_mz = info.get("low_mz")
            high_mz = info.get("high_mz")

            yield Spectrum(
                index=i,
                scan_id=f"scan={scan_number}",
                ms_level=int(info.get("ms_level", 1)),
                retention_time_sec=float(info.get("retention_time", 0.0)) * 60.0,
                mz=mz,
                intensity=intensity,
                polarity=polarity,
                precursor_mz=precursor_mz if precursor_mz else None,
                precursor_charge=precursor_charge,
                collision_energy=collision_energy,
                total_ion_current=float(tic) if tic is not None else None,
                base_peak_mz=float(base_mz) if base_mz is not None else None,
                base_peak_intensity=float(base_int) if base_int is not None else None,
                filter_string=filter_str,
                ion_injection_time_ms=float(ion_inject)
                if ion_inject is not None
                else None,
                scan_window_lower=float(low_mz) if low_mz is not None else None,
                scan_window_upper=float(high_mz) if high_mz is not None else None,
                is_centroid=is_centroid,
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
            return

        times: list[float] = []
        intensities: list[float] = []
        n_scans = self._scan_count()
        for i in range(n_scans):
            scan_number = i + 1
            info = self._scan_info(scan_number)
            rt = info.get("retention_time", 0.0)
            tic = info.get("total_ion_current", 0.0)
            times.append(float(rt) * 60.0)
            intensities.append(float(tic) if tic is not None else 0.0)

        if times:
            yield Chromatogram(id="TIC", times=times, intensities=intensities)

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
        for attr in ("num_scans", "n_scans", "scan_count", "num_spectra"):
            if hasattr(self._raw, attr):
                value = getattr(self._raw, attr)
                return int(value() if callable(value) else value)
        if hasattr(self._raw, "__len__"):
            return len(self._raw)
        raise AttributeError("Cannot determine Thermo scan count from opentfraw object")

    def _scan_info(self, scan_number: int) -> dict[str, Any]:
        assert self._raw is not None
        if hasattr(self._raw, "scan"):
            info = self._raw.scan(scan_number)
            return dict(info) if not isinstance(info, dict) else info
        return {"id": f"scan={scan_number}", "ms_level": 1, "retention_time": 0.0}

    @staticmethod
    def _parse_polarity(value: Any) -> Optional[Polarity]:
        if value is None:
            return None
        if isinstance(value, str):
            if value in ("+", "positive", "Positive"):
                return "positive"
            if value in ("-", "negative", "Negative"):
                return "negative"
        return None
