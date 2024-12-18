import argparse
from loguru import logger
import os
import sys

from . import ParameterModel, convert_raw_file, get_vendor


def main():
    parser = argparse.ArgumentParser(
        description="Converts a file to mzML format using msconvert."
    )
    parser.add_argument("file", type=str, help="The file to convert.")
    parser.add_argument(
        "--output_type", type=str, default="mzml", help="The output format."
    )
    args = parser.parse_args()
    if os.path.exists(args.file):
        datafile = args.file
    else:
        logger.error(f"File {args.file} not found.")
        sys.exit()

    # vendor = get_vendor(infile)
    parameters = ParameterModel()
    parameters.output_type = args.output_type
    _outfile = convert_raw_file(datafile=datafile, parameters=parameters)


if __name__ == "__main__":
    main()
