"""
Optional pure-Rust conversion backend (the ``mzx-rs`` package, ``rust/`` in this repo).

The Rust backend only converts inputs whose output is proven byte-identical to
the pinned msconvert (``rust/CONTRACT.md``, ``rust/support_matrix.toml``). For
everything else it raises ``Unsupported`` and the caller falls back to msconvert.
"""

from . import types

try:
    import mzx_rs as _mzx_rs
except ImportError:  # the Rust backend is optional
    _mzx_rs = None  # type: ignore[assignment]

BACKENDS = ("auto", "rust", "docker")


class Unsupported(Exception):
    """The Rust backend cannot prove its output for this input or these options."""


class RustConversionError(Exception):
    """The Rust backend failed on an input it supports."""


def available() -> bool:
    """Whether the ``mzx_rs`` extension module is installed."""
    return _mzx_rs is not None


def option_set(params: types.TConfig) -> str | None:
    """
    The contract option set that ``params`` asks msconvert for, or None when
    the options are outside the proven sets.

    ``default`` is ``--mzML --noindex --filter "peakPicking true 2-" --filter
    "zeroSamples removeExtra"`` (mzx's CLI defaults); ``nofilter`` is
    ``--mzML --noindex``; ``pickall`` is ``--mzML --noindex --filter
    "peakPicking true 1-"`` (the GUI's peak picking) and ``pickall_rz`` adds
    ``--filter "zeroSamples removeExtra"``.
    """
    if (
        params["type"].lower() != "mzml"
        or params["index"]
        or params["sortbyscan"]
        or params["lockmass"]
    ):
        return None
    match (params["peak_picking"], params["remove_zeros"]):
        case ("msms", True):
            return "default"
        case ("off", False):
            return "nofilter"
        case ("all", False):
            return "pickall"
        case ("all", True):
            return "pickall_rz"
        case _:
            return None


def convert(infile: str, outfile: str, params: types.TConfig) -> str:
    """
    Convert a Thermo RAW file to mzML at ``outfile`` with the Rust backend.

    Raises ``Unsupported`` when the backend is not installed or the input or
    options are outside proven coverage, and ``RustConversionError`` on any
    other failure. ``outfile`` is written atomically, so neither leaves a
    partial file behind.
    """
    if _mzx_rs is None:
        raise Unsupported("mzx-rs is not installed")
    opts = option_set(params)
    if opts is None:
        raise Unsupported("conversion options are outside the proven option sets")
    try:
        _mzx_rs.convert(infile, outfile, opts, f"mzx-rs {infile} --options {opts}")
    except _mzx_rs.Unsupported as e:
        raise Unsupported(str(e)) from e
    except _mzx_rs.ConversionError as e:
        raise RustConversionError(str(e)) from e
    return outfile
