import argparse

from . import convert_raw_file, types, vendor


def main():
    parser = argparse.ArgumentParser(
        description="Converts a file to mzML format using msconvert."
    )
    parser.add_argument("file", type=str, help="The file to convert.")
    parser.add_argument("--type", type=str, default="mzml", help="The output format.")
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
    parser.add_argument("--output", type=str, default=None, help="The output file.")
    args = parser.parse_args()
    vendor_name = vendor.vendor_name_from_file(args.file)
    params: types.TConfig = {
        "infile": args.file,
        "index": args.index,
        "sortbyscan": args.sortbyscan,
        "peak_picking": args.peak_picking,
        "remove_zeros": args.remove_zeros,
        "vendor": vendor_name,
        "outfile": None,
        "type": args.type,
        "overwrite": args.overwrite,
        "debug": args.debug,
        "verbose": args.verbose,
    }

    _status = convert_raw_file(params)


if __name__ == "__main__":
    main()
