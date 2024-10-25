import argparse

from . import convert_raw_file, get_vendor


def main():
    parser = argparse.ArgumentParser(
        description="Converts a file to mzML format using msconvert."
    )
    parser.add_argument("file", type=str, help="The file to convert.")
    parser.add_argument("--type", type=str, default="mzml", help="The output format.")
    args = parser.parse_args()
    infile = args.file

    vendor = get_vendor(infile)
    _outfile = convert_raw_file(infile, vendor)


if __name__ == "__main__":
    main()
