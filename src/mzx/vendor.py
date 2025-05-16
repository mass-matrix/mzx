import os
from loguru import logger

from . import types


def vendor_name_from_file(filename: str) -> types.TVendor:
    """
    Determine the vendor of the file.
    Currently only supports Thermo and Agilent.
    Returns the vendor name.
    If the vendor is not supported, returns None.
    """
    logger.info(f"Getting vendor for file: {filename}")

    if os.path.isdir(filename):
        logger.info("Input is a directory.")
        if ".d" in filename:
            return "bruker"
        elif ".raw" in filename:
            return "waters"
        else:
            files = os.listdir(filename)
            for f in files:
                if "_FUNC" in f:
                    return "waters"
    else:
        logger.info("Input is a file.")
        if filename.endswith(".raw"):
            return "Thermo"
        elif filename.endswith(".d"):
            return "Agilent"
    return "unspecified"
