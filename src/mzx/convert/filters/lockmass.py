"""Waters-style lockmass refinement filter (native path)."""

from __future__ import annotations

from typing import Iterable, Iterator, Optional

from ..base import Spectrum


def apply_lockmass(
    spectra: Iterable[Spectrum],
    *,
    pos_mz: float = 556.2771,
    neg_mz: float = 554.2615,
    tolerance: float = 0.1,
    exclude_function: Optional[int] = None,
) -> Iterator[Spectrum]:
    """
    Shift each spectrum so the nearest lockmass peak sits on the reference m/z.

    Lightweight stand-in for ProteoWizard's ``lockmassRefiner`` filter. Spectra
    whose ``scan_id`` contains ``function={exclude_function}`` are skipped
    (used for Waters lockmass reference functions).

    Args:
        spectra: Input spectra from a vendor converter.
        pos_mz: Reference m/z for positive mode (default Waters lockmass).
        neg_mz: Reference m/z for negative mode.
        tolerance: Maximum absolute m/z deviation to apply correction.
        exclude_function: Optional Waters function number to skip.

    Yields:
        Mass-corrected :class:`~mzx.convert.base.Spectrum` instances.
    """
    for spectrum in spectra:
        if (
            exclude_function is not None
            and f"function={exclude_function}" in spectrum.scan_id
        ):
            yield spectrum
            continue
        if not spectrum.mz:
            yield spectrum
            continue

        target = pos_mz
        if spectrum.polarity == "negative":
            target = neg_mz

        nearest = min(spectrum.mz, key=lambda x: abs(x - target))
        if abs(nearest - target) > tolerance:
            yield spectrum
            continue

        delta = target - nearest
        yield Spectrum(
            index=spectrum.index,
            scan_id=spectrum.scan_id,
            ms_level=spectrum.ms_level,
            retention_time_sec=spectrum.retention_time_sec,
            mz=[m + delta for m in spectrum.mz],
            intensity=list(spectrum.intensity),
            polarity=spectrum.polarity,
            precursor_mz=spectrum.precursor_mz,
            precursor_charge=spectrum.precursor_charge,
            collision_energy=spectrum.collision_energy,
        )
