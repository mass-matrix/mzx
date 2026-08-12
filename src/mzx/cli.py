"""Command-line interface for mzx mass spectrometry file conversion.

The CLI supports two conversion paths:

* **Default** — ProteoWizard ``msconvert`` via Docker (``mzx file.raw``).
* **Experimental native** — vendor parsers without Docker (``mzx --native file.raw``).

See :func:`mzx.convert_file` and :mod:`mzx.convert` for programmatic use.
"""

import argparse
import os
import sys

from loguru import logger

from . import (
    convert_file,
    export_chromatograms,
    extract_tic_from_mzml,
    get_chromatogram_info,
    types,
    vendor,
)
from .convert import detect_vendor as detect_native_vendor


def main():
    """Parse arguments and run file conversion.

    Uses :func:`mzx.convert_file` with ``native=args.native``. The default path
    requires Docker. The ``--native`` flag enables experimental conversion without
    Docker (mzML output only; optional deps via ``pip install mzx[native]``).
    """
    parser = argparse.ArgumentParser(
        description=(
            "Convert mass spectrometry vendor files to open formats. "
            "Default path uses ProteoWizard/msconvert via Docker. "
            "Use --native for experimental conversion without Docker."
        )
    )
    parser.add_argument("file", type=str, help="The file to convert.")
    parser.add_argument("--type", type=str, default="mzml", help="The output format.")
    parser.add_argument(
        "--native",
        action="store_true",
        default=False,
        help=(
            "(experimental) Convert using native vendor parsers without "
            "Docker/ProteoWizard. Supported vendors: Thermo, Waters, Agilent, "
            "Bruker. Requires optional deps (pip install mzx[native]). mzML only."
        ),
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        default=False,
        help="Overwrite the output file if it exists.",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        default=False,
        help="Enable debug mode.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        default=False,
        help="Enable verbose output.",
    )
    parser.add_argument(
        "--index",
        action="store_true",
        default=False,
        help="Index the output file.",
    )
    parser.add_argument(
        "--sortbyscan",
        action="store_true",
        default=False,
        help="Sort the output file by scan.",
    )
    parser.add_argument(
        "--peak_picking",
        action="store",
        choices=["off", "all", "msms", "ms1"],
        default="msms",
        help="Enable peak picking.",
    )
    parser.add_argument(
        "--remove_zeros",
        action="store_true",
        default=True,
        help="Remove zeros from the output file.",
    )
    parser.add_argument(
        "--vendor",
        type=str,
        default=None,
        help="Specify the vendor of the raw file.",
    )
    parser.add_argument(
        "--lockmass_disabled",
        action="store_true",
        default=False,
        help="Enable lockmass correction.",
    )
    parser.add_argument(
        "--lockmass_mz_pos",
        type=float,
        default=556.2771,
        help="Lockmass m/z for positive mode.",
    )
    parser.add_argument(
        "--lockmass_mz_neg",
        type=float,
        default=554.2615,
        help="Lockmass m/z for negative mode.",
    )
    parser.add_argument(
        "--lockmass_tolerance",
        type=float,
        default=0.1,
        help="Lockmass tolerance in ppm.",
    )
    parser.add_argument(
        "--chromatograms",
        action="store_true",
        default=False,
        help="Export Waters chromatograms (UV, pressure, etc.) to CSV.",
    )
    parser.add_argument("--output", type=str, default=None, help="The output file.")
    args = parser.parse_args()

    if args.native:
        logger.warning(
            "Experimental native conversion enabled; output may differ from "
            "ProteoWizard/msconvert."
        )
        if args.type.lower() != "mzml":
            logger.error(
                f"Native conversion supports mzML only (got type={args.type!r})."
            )
            sys.exit(1)

    if args.vendor:
        vendor_name: types.TVendor = args.vendor  # type: ignore[assignment]
    elif args.native:
        vendor_name = detect_native_vendor(args.file)
    else:
        vendor_name = vendor.vendor_name_from_file(args.file)

    params: types.TConfig = {
        "infile": args.file,
        "index": args.index,
        "sortbyscan": args.sortbyscan,
        "peak_picking": args.peak_picking,
        "remove_zeros": args.remove_zeros,
        "vendor": vendor_name,
        "outfile": args.output,
        "type": args.type,
        "overwrite": args.overwrite,
        "debug": args.debug,
        "verbose": args.verbose,
        "lockmass_disabled": args.lockmass_disabled,
        "lockmass": False,
        "neg_lockmass": args.lockmass_mz_neg,
        "pos_lockmass": args.lockmass_mz_pos,
        "lockmass_tolerance": args.lockmass_tolerance,
        "lockmass_function_exclude": None,
    }

    mzml_path = None
    try:
        mzml_path = convert_file(params, native=args.native)
    except Exception as e:
        logger.error("Raw file conversion failed!")
        logger.error(str(e))
        if args.native:
            sys.exit(1)

    if args.chromatograms:
        if vendor_name == "waters":
            chrom_info = get_chromatogram_info(args.file)
            if not chrom_info:
                logger.warning("No chromatogram metadata found in Waters file.")
            else:
                exported = export_chromatograms(args.file, chrom_info)
                logger.info(f"Exported {len(exported)} chromatogram(s).")

        if mzml_path and os.path.exists(mzml_path):
            extract_tic_from_mzml(mzml_path)


if __name__ == "__main__":
    main()
