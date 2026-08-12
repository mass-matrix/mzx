"""Simple peak-picking filter for native conversion."""

from __future__ import annotations

from typing import Iterable, Iterator, Literal

from ..base import Spectrum

PeakPickingMode = Literal["off", "all", "msms", "ms1"]


def _local_maxima(
    mz: list[float], intensity: list[float]
) -> tuple[list[float], list[float]]:
    if len(intensity) < 3:
        return mz, intensity
    out_mz: list[float] = []
    out_i: list[float] = []
    for i in range(1, len(intensity) - 1):
        if intensity[i] >= intensity[i - 1] and intensity[i] >= intensity[i + 1]:
            if intensity[i] > 0:
                out_mz.append(mz[i])
                out_i.append(intensity[i])
    return out_mz, out_i


def apply_peak_picking(
    spectra: Iterable[Spectrum],
    mode: PeakPickingMode = "off",
) -> Iterator[Spectrum]:
    """
    Yield spectra, optionally reducing profile-like arrays to local maxima.

    Modes mirror the Docker msconvert CLI ``--peak_picking`` option:

    * ``off`` — pass through unchanged
    * ``all`` — pick on every spectrum
    * ``ms1`` — pick only MS1 spectra
    * ``msms`` — pick only MSn (level >= 2)

    This is a lightweight native implementation and may not match ProteoWizard
    peak picking exactly.

    Args:
        spectra: Input spectra from a vendor converter.
        mode: Peak picking mode.

    Yields:
        Filtered :class:`~mzx.convert.base.Spectrum` instances.
    """
    for spectrum in spectra:
        should_pick = (
            mode == "all"
            or (mode == "ms1" and spectrum.ms_level == 1)
            or (mode == "msms" and spectrum.ms_level >= 2)
        )
        if not should_pick or mode == "off":
            yield spectrum
            continue
        mz, intensity = _local_maxima(spectrum.mz, spectrum.intensity)
        yield Spectrum(
            index=spectrum.index,
            scan_id=spectrum.scan_id,
            ms_level=spectrum.ms_level,
            retention_time_sec=spectrum.retention_time_sec,
            mz=mz,
            intensity=intensity,
            polarity=spectrum.polarity,
            precursor_mz=spectrum.precursor_mz,
            precursor_charge=spectrum.precursor_charge,
            collision_energy=spectrum.collision_energy,
        )
