"""Vendor converter registry and path-based auto-detection.

Registered converters: Thermo, Waters, Agilent, and Bruker. Use
:func:`get_converter` to instantiate by name or :func:`converter_for_path`
to detect and open from a file path.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Callable, Type

from ... import types
from ..base import VendorConverter
from .agilent import AgilentConverter
from .bruker import BrukerConverter
from .thermo import ThermoConverter
from .waters import WatersConverter

ConverterFactory = Callable[[], VendorConverter]

_REGISTRY: dict[str, Type[VendorConverter]] = {
    "thermo": ThermoConverter,
    "Thermo": ThermoConverter,
    "waters": WatersConverter,
    "agilent": AgilentConverter,
    "Agilent": AgilentConverter,
    "bruker": BrukerConverter,
}


def list_converters() -> list[str]:
    """Return canonical vendor names with registered native converters.

    Example::

        >>> from mzx.convert import list_converters
        >>> "Thermo" in list_converters()
        True
    """
    return sorted({cls.vendor for cls in _REGISTRY.values()})


def get_converter(vendor: str) -> VendorConverter:
    """Instantiate a converter for the given vendor name.

    Args:
        vendor: Vendor key such as ``"Thermo"``, ``"waters"``, ``"Agilent"``,
            or ``"bruker"`` (case-insensitive aliases are accepted).

    Returns:
        A new, unopened :class:`~mzx.convert.base.VendorConverter` instance.

    Raises:
        KeyError: When no converter is registered for ``vendor``.
    """
    key = vendor.strip()
    cls = _REGISTRY.get(key) or _REGISTRY.get(key.lower())
    if cls is None:
        raise KeyError(f"No native converter registered for vendor '{vendor}'")
    return cls()


def detect_vendor(path: str) -> types.TVendor:
    """
    Detect vendor from path layout for native conversion.

    Unlike :func:`mzx.vendor.vendor_name_from_file`, this distinguishes Agilent
    ``.d/`` directories (``AcqData/`` present) from Bruker timsTOF bundles
    (``analysis.tdf`` present).

    Args:
        path: Vendor file or directory path.

    Returns:
        Vendor name aligned with converter registry keys, or ``"unspecified"``.
    """
    p = Path(path)
    if p.is_dir():
        name = p.name.lower()
        if name.endswith(".d"):
            # Bruker timsTOF bundles contain analysis.tdf; Agilent has AcqData.
            if (p / "analysis.tdf").exists() or (p / "analysis.tdf_bin").exists():
                return "bruker"
            if (p / "AcqData").is_dir():
                return "Agilent"
            return "bruker"
        if name.endswith(".raw"):
            return "waters"
        for entry in os.listdir(p):
            if "_FUNC" in entry.upper():
                return "waters"
        return "unspecified"

    lower = p.name.lower()
    if lower.endswith(".raw"):
        return "Thermo"
    if lower.endswith(".d"):
        return "Agilent"
    if lower.endswith(".wiff"):
        return "unspecified"
    return "unspecified"


def converter_for_path(path: str) -> VendorConverter:
    """Detect vendor from ``path``, instantiate a converter, and open it.

    Args:
        path: Vendor file or directory path.

    Returns:
        An opened :class:`~mzx.convert.base.VendorConverter`.

    Raises:
        ValueError: When the vendor cannot be detected.
        KeyError: When no converter is registered for the detected vendor.
    """
    vendor = detect_vendor(path)
    if vendor == "unspecified":
        raise ValueError(f"Cannot detect vendor for path: {path}")
    converter = get_converter(vendor)
    converter.open(path)
    return converter
