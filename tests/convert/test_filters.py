"""Tests for native conversion filters."""

from mzx.convert.base import Spectrum
from mzx.convert.filters import apply_lockmass, apply_peak_picking


def test_peak_picking_msms_only() -> None:
    spectra = [
        Spectrum(0, "scan=1", 1, 0.0, [1.0, 2.0, 3.0], [1.0, 5.0, 1.0]),
        Spectrum(1, "scan=2", 2, 1.0, [1.0, 2.0, 3.0], [1.0, 5.0, 1.0]),
    ]
    out = list(apply_peak_picking(spectra, mode="msms"))
    assert out[0].mz == [1.0, 2.0, 3.0]
    assert out[1].mz == [2.0]
    assert out[1].intensity == [5.0]


def test_lockmass_shifts_near_peak() -> None:
    spectra = [
        Spectrum(
            0,
            "function=1 process=0 scan=1",
            1,
            0.0,
            [556.2, 600.0],
            [100.0, 10.0],
            polarity="positive",
        )
    ]
    out = list(apply_lockmass(spectra, pos_mz=556.2771, tolerance=0.1))
    assert abs(out[0].mz[0] - 556.2771) < 1e-9
