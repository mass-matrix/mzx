import os
from loguru import logger

from . import types


def vendor_name_from_file(filename: str) -> types.TVendor:
    """
    Determine the vendor of the file.
    Only the last path component is matched, ignoring case.
    Returns the vendor name.
    If the vendor is not supported, returns "unspecified".
    """
    logger.info(f"Getting vendor for file: {filename}")
    name = os.path.basename(os.path.normpath(filename)).lower()

    if os.path.isdir(filename):
        logger.info("Input is a directory.")
        if name.endswith(".d"):
            return "Agilent" if _is_agilent_directory(filename) else "bruker"
        elif name.endswith(".raw"):
            return "waters"
        else:
            files = os.listdir(filename)
            for f in files:
                if "_func" in f.lower():
                    return "waters"
    else:
        logger.info("Input is a file.")
        if name.endswith(".raw"):
            return "Thermo"
        elif name.endswith(".d"):
            return "Agilent"
    return "unspecified"


def _is_agilent_directory(path: str) -> bool:
    """Agilent .d directories contain an AcqData folder; Bruker .d directories do not."""
    try:
        return any(entry.lower() == "acqdata" for entry in os.listdir(path))
    except OSError:
        return False
