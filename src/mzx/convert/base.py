"""Vendor-neutral data model and converter contract for native mzML export.

Implementations live under :mod:`mzx.convert.vendors`. Each converter reads a
vendor acquisition and yields :class:`Spectrum` and :class:`Chromatogram`
instances consumed by :func:`~mzx.convert.mzml_writer.write_mzml`.
"""

from __future__ import annotations

from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Iterator, Literal, Optional


Polarity = Literal["positive", "negative"]


@dataclass
class Spectrum:
    """One mass spectrum from a vendor acquisition.

    Attributes:
        index: Zero-based position in the output mzML spectrum list.
        scan_id: Native identifier written to the mzML ``spectrum`` element.
        ms_level: MS level (1 for survey scans, 2+ for MS/MS).
        retention_time_sec: Scan start time in seconds.
        mz: m/z values (centroid or profile points).
        intensity: Intensity values parallel to ``mz``.
        polarity: Ion polarity when known.
        precursor_mz: Isolation m/z for MSn spectra.
        precursor_charge: Precursor charge state when known.
        collision_energy: Collision energy in eV when known.
        total_ion_current: TIC; computed from intensities if omitted.
        base_peak_mz: Base peak m/z; computed if omitted.
        base_peak_intensity: Base peak intensity; computed if omitted.
    """

    index: int
    scan_id: str
    ms_level: int
    retention_time_sec: float
    mz: list[float]
    intensity: list[float]
    polarity: Optional[Polarity] = None
    precursor_mz: Optional[float] = None
    precursor_charge: Optional[int] = None
    collision_energy: Optional[float] = None
    total_ion_current: Optional[float] = None
    base_peak_mz: Optional[float] = None
    base_peak_intensity: Optional[float] = None

    def __post_init__(self) -> None:
        if len(self.mz) != len(self.intensity):
            raise ValueError("mz and intensity arrays must have the same length")
        if self.total_ion_current is None and self.intensity:
            self.total_ion_current = float(sum(self.intensity))
        if self.intensity and self.base_peak_intensity is None:
            max_i = max(self.intensity)
            self.base_peak_intensity = float(max_i)
            self.base_peak_mz = float(self.mz[self.intensity.index(max_i)])


@dataclass
class Chromatogram:
    """One chromatogram channel (TIC, UV, pressure, etc.).

    Attributes:
        id: Channel identifier written to mzML.
        times: Time axis in seconds.
        intensities: Intensity values parallel to ``times``.
        unit: Optional display unit from vendor metadata.
    """

    id: str
    times: list[float]
    intensities: list[float]
    unit: Optional[str] = None

    def __post_init__(self) -> None:
        if len(self.times) != len(self.intensities):
            raise ValueError("times and intensities must have the same length")


@dataclass
class RunMetadata:
    """Minimal run-level metadata written into mzML."""

    source_file: str
    vendor: str
    instrument_model: Optional[str] = None
    software: Optional[str] = None
    extras: dict[str, Any] = field(default_factory=dict)

    def as_dict(self) -> dict[str, Any]:
        data: dict[str, Any] = {
            "source_file": self.source_file,
            "vendor": self.vendor,
        }
        if self.instrument_model is not None:
            data["instrument_model"] = self.instrument_model
        if self.software is not None:
            data["software"] = self.software
        data.update(self.extras)
        return data


class VendorConverter(ABC):
    """Contract every native vendor converter must implement.

    Typical usage::

        converter = ThermoConverter()
        converter.open("/path/to/run.raw")
        spectra = list(converter.iter_spectra())
        meta = converter.metadata()
        converter.close()

    Converters may also be used as context managers.
    """

    vendor: str = "unspecified"

    @abstractmethod
    def open(self, path: str) -> None:
        """Open a vendor acquisition for reading.

        Args:
            path: Path to a vendor file or directory.

        Raises:
            FileNotFoundError: When the path does not exist.
            ImportError: When the optional vendor parser is not installed.
        """

    @abstractmethod
    def iter_spectra(self) -> Iterator[Spectrum]:
        """Yield spectra in acquisition order.

        Raises:
            RuntimeError: When :meth:`open` has not been called.
        """

    @abstractmethod
    def iter_chromatograms(self) -> Iterator[Chromatogram]:
        """Yield chromatogram channels when available.

        May yield nothing if the vendor file has no chromatogram data.
        """

    @abstractmethod
    def metadata(self) -> dict[str, Any]:
        """Return run-level metadata used by the mzML writer.

        Should include at least ``source_file`` and ``vendor`` keys.
        """

    @abstractmethod
    def close(self) -> None:
        """Release any open handles."""

    def __enter__(self) -> VendorConverter:
        return self

    def __exit__(self, exc_type, exc, tb) -> None:
        self.close()
