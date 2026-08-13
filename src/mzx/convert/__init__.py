"""Native vendor to mzML conversion without Docker or ProteoWizard.

This package implements an experimental alternative to the default Docker/msconvert
path. Vendor-specific readers decode acquisitions into :class:`~mzx.convert.base.Spectrum`
objects, optional filters are applied, and :func:`~mzx.convert.mzml_writer.write_mzml`
serializes the result.

Public entry points:

* :func:`convert_native` — high-level conversion from a :class:`~mzx.types.TConfig` dict
* :func:`detect_vendor` — path-based vendor detection for native converters
* :func:`list_converters` — registered vendor names

Install optional parsers with ``pip install mzx[native]`` or per-vendor extras
(``mzx[thermo]``, ``mzx[waters]``, etc.).
"""

from __future__ import annotations

import os
from pathlib import Path

from loguru import logger

from .. import types
from .base import Chromatogram, Spectrum, VendorConverter
from .filters import apply_lockmass, apply_peak_picking
from .mzml_writer import parse_spectra_from_mzml, write_mzml
from .vendors import (
    converter_for_path,
    detect_vendor,
    get_converter,
    list_converters,
)


class NativeConversionError(Exception):
    """Raised when native conversion fails.

    Typical causes: undetected vendor, missing optional parser package, unsupported
    output format, or an existing output file when ``overwrite`` is False.
    """


def convert_native(params: types.TConfig) -> str:
    """
    Convert a vendor acquisition to mzML using native converters.

    Reads spectra via a :class:`~mzx.convert.base.VendorConverter`, applies native
    filters (peak picking, lockmass, zero removal, sort-by-scan), and writes mzML.

    Args:
        params: Same configuration dict used by the Docker msconvert path
            (:class:`~mzx.types.TConfig`). Only ``type="mzml"`` is supported.

    Returns:
        Absolute path to the written mzML file.

    Raises:
        NativeConversionError: On vendor detection failure, missing optional deps,
            unsupported output type, or write conflicts.
        ImportError: When a vendor parser package is not installed.

    Example::

        from mzx.convert import convert_native

        out = convert_native(params)

    Requires optional dependencies: ``pip install mzx[native]`` (or per-vendor extras).
    Output may differ from ProteoWizard/msconvert.
    """
    infile = params["infile"]
    vendor = params.get("vendor") or detect_vendor(infile)
    if vendor == "unspecified":
        vendor = detect_vendor(infile)
    if vendor == "unspecified":
        raise NativeConversionError(f"Unsupported or undetected vendor for {infile}")

    logger.info(f"Native conversion: vendor={vendor} file={infile}")

    try:
        converter = get_converter(str(vendor))
    except KeyError as exc:
        raise NativeConversionError(str(exc)) from exc

    converter.open(infile)
    try:
        spectra = list(converter.iter_spectra())
        chromatograms = list(converter.iter_chromatograms())
        metadata = converter.metadata()
    finally:
        converter.close()

    peak_mode = params.get("peak_picking") or "off"
    spectra_iter = apply_peak_picking(spectra, mode=peak_mode)  # type: ignore[arg-type]

    if params.get("lockmass") and not params.get("lockmass_disabled"):
        spectra_iter = apply_lockmass(
            spectra_iter,
            pos_mz=float(params.get("pos_lockmass") or 556.2771),
            neg_mz=float(params.get("neg_lockmass") or 554.2615),
            tolerance=float(params.get("lockmass_tolerance") or 0.1),
            exclude_function=params.get("lockmass_function_exclude"),
        )

    filtered = list(spectra_iter)
    if params.get("remove_zeros"):
        filtered = [
            Spectrum(
                index=s.index,
                scan_id=s.scan_id,
                ms_level=s.ms_level,
                retention_time_sec=s.retention_time_sec,
                mz=[m for m, i in zip(s.mz, s.intensity) if i != 0],
                intensity=[i for i in s.intensity if i != 0],
                polarity=s.polarity,
                precursor_mz=s.precursor_mz,
                precursor_charge=s.precursor_charge,
                collision_energy=s.collision_energy,
                total_ion_current=s.total_ion_current,
                base_peak_mz=s.base_peak_mz,
                base_peak_intensity=s.base_peak_intensity,
                filter_string=s.filter_string,
                ion_injection_time_ms=s.ion_injection_time_ms,
                scan_window_lower=s.scan_window_lower,
                scan_window_upper=s.scan_window_upper,
                is_centroid=s.is_centroid,
            )
            for s in filtered
        ]

    if params.get("sortbyscan"):
        filtered.sort(key=lambda s: (s.retention_time_sec, s.index))
        for i, s in enumerate(filtered):
            s.index = i

    out_path = _resolve_outfile(params)
    if os.path.exists(out_path) and not params.get("overwrite"):
        raise NativeConversionError(f"Output exists and overwrite is False: {out_path}")

    return write_mzml(
        out_path,
        filtered,
        chromatograms=chromatograms,
        metadata=metadata,
        indexed=bool(params.get("index")),
        compress=bool(params.get("compress")),
    )


def _resolve_outfile(params: types.TConfig) -> str:
    outfile = params.get("outfile")
    if outfile:
        return str(Path(outfile).resolve())
    infile = Path(params["infile"])
    base = infile.stem if infile.suffix else infile.name
    # Waters/Agilent/Bruker dirs often end with .raw / .d
    if base.lower().endswith(".raw") or base.lower().endswith(".d"):
        base = Path(base).stem
    parent = infile.parent if infile.suffix or infile.is_file() else infile.parent
    if infile.is_dir():
        parent = infile.parent
        base = infile.name
        if base.lower().endswith(".raw") or base.lower().endswith(".d"):
            base = base.rsplit(".", 1)[0]
    out_type = (params.get("type") or "mzml").lower()
    if out_type == "mzxml":
        ext = ".mzXML"
    elif out_type == "mgf":
        ext = ".mgf"
    else:
        ext = ".mzML"
    if out_type != "mzml":
        raise NativeConversionError(
            f"Native conversion currently supports mzML only (got type={out_type})"
        )
    return str((parent / f"{base}{ext}").resolve())


__all__ = [
    "Chromatogram",
    "NativeConversionError",
    "Spectrum",
    "VendorConverter",
    "apply_lockmass",
    "apply_peak_picking",
    "convert_native",
    "converter_for_path",
    "detect_vendor",
    "get_converter",
    "list_converters",
    "parse_spectra_from_mzml",
    "write_mzml",
]
