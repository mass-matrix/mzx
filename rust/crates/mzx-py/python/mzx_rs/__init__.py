"""Proven pure-Rust conversion backend for mzx.

``convert`` writes mzML that is byte-identical to the pinned msconvert after
the provenance normalization in ``rust/CONTRACT.md``, or raises
``Unsupported`` so the caller can fall back to msconvert.
"""

from ._mzx_rs import (
    OPTION_SETS,
    ConversionError,
    Unsupported,
    __version__,
    convert,
)

__all__ = ["OPTION_SETS", "ConversionError", "Unsupported", "__version__", "convert"]
