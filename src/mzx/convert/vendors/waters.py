"""Waters MassLynx .raw/ converter via optional openwraw + existing helpers."""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any, Iterator, Optional

from ..base import Chromatogram, Spectrum, VendorConverter


class WatersConverter(VendorConverter):
    """
    Native Waters MassLynx ``.raw/`` directory converter.

    Spectrum decoding uses the optional ``openwraw`` package. Chromatogram export
    can fall back to built-in pure-Python ``_CHRO*`` parsers when openwraw is
    unavailable.

    Install: ``pip install mzx[waters]`` or ``pip install openwraw``.
    """

    vendor = "waters"

    def __init__(self) -> None:
        self._path: Optional[str] = None
        self._reader: Any = None
        self._use_openwraw = False

    def open(self, path: str) -> None:
        path_obj = Path(path)
        if not path_obj.is_dir():
            raise NotADirectoryError(f"Waters .raw path must be a directory: {path}")
        self._path = str(path_obj.resolve())

        try:
            import openwraw  # type: ignore
        except ImportError:
            self._reader = None
            self._use_openwraw = False
            # Chromatograms can still be read via pure-Python helpers.
            return

        self._use_openwraw = True
        if hasattr(openwraw, "RawReader"):
            self._reader = openwraw.RawReader(self._path)
        elif hasattr(openwraw, "open"):
            self._reader = openwraw.open(self._path)
        else:
            self._reader = None
            self._use_openwraw = False

    def iter_spectra(self) -> Iterator[Spectrum]:
        if self._path is None:
            raise RuntimeError("WatersConverter.open() must be called first")

        if not self._use_openwraw or self._reader is None:
            raise ImportError(
                "Waters spectrum conversion requires the optional dependency "
                "'openwraw'. Install with: pip install mzx[native] "
                "or pip install openwraw. Chromatogram export remains available "
                "via the built-in parsers."
            )

        if hasattr(self._reader, "iter_spectra"):
            for i, spec in enumerate(self._reader.iter_spectra()):
                yield self._from_mapping(i, spec)
            return

        # Fallback API: function/scan nested access
        if hasattr(self._reader, "functions") and hasattr(
            self._reader, "read_spectrum"
        ):
            index = 0
            for function_id in self._reader.functions():
                n = self._reader.n_scans(function_id)
                for scan_idx in range(n):
                    spec = self._reader.read_spectrum(function_id, scan_idx)
                    yield self._from_mapping(
                        index,
                        {
                            "id": (
                                f"function={function_id} process=0 scan={scan_idx + 1}"
                            ),
                            "ms_level": getattr(spec, "ms_level", 1),
                            "retention_time_sec": getattr(
                                spec, "retention_time_sec", getattr(spec, "rt", 0.0)
                            ),
                            "mz": list(getattr(spec, "mz", [])),
                            "intensity": list(
                                getattr(
                                    spec, "intensity", getattr(spec, "intensities", [])
                                )
                            ),
                            "polarity": getattr(spec, "polarity", None),
                            "precursor_mz": getattr(spec, "precursor_mz", None),
                            "precursor_charge": getattr(spec, "precursor_charge", None),
                            "collision_energy": getattr(spec, "collision_energy", None),
                        },
                    )
                    index += 1
            return

        raise AttributeError("openwraw reader does not expose a supported spectrum API")

    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        if self._path is None:
            raise RuntimeError("WatersConverter.open() must be called first")

        if self._reader is not None and hasattr(self._reader, "iter_chromatograms"):
            for chrom in self._reader.iter_chromatograms():
                yield Chromatogram(
                    id=str(getattr(chrom, "id", chrom.get("id", "chrom"))),
                    times=list(getattr(chrom, "times", chrom.get("times", []))),
                    intensities=list(
                        getattr(chrom, "intensities", chrom.get("intensities", []))
                    ),
                )
            return

        # Pure-Python fallback using existing mzx chromatogram helpers.
        from mzx import get_chromatogram_info, parse_chrodat
        import re

        chrom_info = get_chromatogram_info(self._path)
        pattern = re.compile(r"_chro(\d+)", re.IGNORECASE)
        for name in sorted(os.listdir(self._path)):
            base, ext = os.path.splitext(name)
            if ext.lower() != ".dat":
                continue
            match = pattern.match(base)
            if not match:
                continue
            number = int(match.group(1))
            parsed = parse_chrodat(os.path.join(self._path, name))
            if parsed is None:
                continue
            times, intensities = parsed
            times = [t * 60.0 for t in times]
            if number <= len(chrom_info):
                channel = chrom_info[number - 1][0]
            else:
                channel = f"channel_{number}"
            yield Chromatogram(id=channel, times=times, intensities=intensities)

    def metadata(self) -> dict[str, Any]:
        if self._path is None:
            raise RuntimeError("WatersConverter.open() must be called first")
        meta: dict[str, Any] = {
            "source_file": self._path,
            "vendor": self.vendor,
            "software": "mzx-native/openwraw"
            if self._use_openwraw
            else "mzx-native/waters-fallback",
            "instrument_model": "Waters",
        }
        header = Path(self._path) / "_HEADER.TXT"
        if header.exists():
            meta["header_txt"] = header.read_text(encoding="latin-1", errors="ignore")[
                :500
            ]
        return meta

    def close(self) -> None:
        if self._reader is not None and hasattr(self._reader, "close"):
            self._reader.close()
        self._reader = None
        self._path = None
        self._use_openwraw = False

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
