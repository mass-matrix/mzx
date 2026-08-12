"""Shared fixtures and helpers for native conversion contract tests."""

from __future__ import annotations

import struct
from pathlib import Path

import pytest

from tests.convert.helpers import FakeConverter, sample_chromatograms, sample_spectra

FIXTURES = Path(__file__).resolve().parent / "fixtures"
REFERENCE = FIXTURES / "reference"
SYNTHETIC = FIXTURES / "synthetic"


def _build_chroinf(records):
    header = b"\x00" * 0x84
    body = b""
    for name, unit in records:
        entry = f"{name}\x00$CC$,1.000000,3,0,0,{unit}"
        raw = entry.encode("latin-1").ljust(0x55, b"\x00")
        body += raw
    return header + body


def _build_chrodat(samples):
    header = b"\x00" * 0x80
    data = b"".join(struct.pack("<ff", t, v) for t, v in samples)
    return header + data


@pytest.fixture
def fake_converter() -> FakeConverter:
    return FakeConverter()


@pytest.fixture
def reference_mzml_path() -> Path:
    path = REFERENCE / "synthetic_reference.mzml.xml"
    assert path.exists(), f"Missing reference fixture: {path}"
    return path


@pytest.fixture
def waters_chrom_raw_dir(tmp_path: Path) -> Path:
    """Minimal Waters-like .raw directory with chromatogram channels only."""
    raw_dir = tmp_path / "sample.raw"
    raw_dir.mkdir()
    (raw_dir / "_HEADER.TXT").write_text("Sample Description\n", encoding="latin-1")
    (raw_dir / "_extern.inf").write_text("no lockmass\n", encoding="latin-1")
    (raw_dir / "_chroms.inf").write_bytes(_build_chroinf([("TUV 260", " AU")]))
    (raw_dir / "_chro001.dat").write_bytes(_build_chrodat([(0.5, 100.0), (1.0, 200.0)]))
    return raw_dir


@pytest.fixture
def thermo_fixture_path() -> Path | None:
    matches = sorted((FIXTURES / "thermo").glob("*.raw"))
    return matches[0] if matches else None


@pytest.fixture
def waters_fixture_path() -> Path | None:
    root = FIXTURES / "waters"
    if not root.exists():
        return None
    for child in sorted(root.iterdir()):
        if child.is_dir() and child.name.lower().endswith(".raw"):
            return child
    return None


__all__ = [
    "FakeConverter",
    "sample_chromatograms",
    "sample_spectra",
]
