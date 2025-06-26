__version__ = "0.3.1"

import os
import re
import shlex
import subprocess
from loguru import logger

from . import types

docker_image = "chambm/pwiz-skyline-i-agree-to-the-vendor-licenses"


class WatersConvertException(Exception):
    pass


class RawFileConversionError(Exception):
    pass


def run_cmd(cmd):
    """
    Run a command and return the output.
    """
    cmd = shlex.split(cmd, posix=True)
    # logger.info(f"Running command: {cmd}")
    p = subprocess.Popen(cmd, stdout=subprocess.PIPE, universal_newlines=True)

    output = ""
    while True:
        if p.stdout:
            line = p.stdout.readline()
            if not line:
                break
            (logger.info(line.strip(), flush=True),)
            output = output + line

    logger.info("Process Complete")
    return output


def format_function_number(s):
    match = re.search(r"Function (\d+)", s)
    if match:
        function_number = int(match.group(1))
        return f"_FUNC{function_number:03d}", function_number
    else:
        return None


def modify_waters_scan_header(line):
    """
    Modify the Waters scan header line.
    """
    pattern = re.compile(
        r'<spectrum index="(\d+)" id="function=(\d+) process=(\d+) scan=(\d+)"'
    )
    match = pattern.search(line)

    if match:
        index, function, process, scan = match.groups()
        # Calculate new scan value
        new_scan_value = int(index) + 1
        # Replace and add the scan and fscan values in the line
        modified_line = re.sub(r"scan=\d+", f"scan={new_scan_value} fscan={scan}", line)
        # logger.debug(line)
        # logger.debug(modified_line)
        return modified_line
    else:
        return line


def process_waters_scan_headers(file_path):
    """
    Process the Waters scan headers in the given file.
    """
    # TODO address UTF8 encoding issue
    with open(file_path, encoding="utf8", errors="ignore") as file:
        lines = file.readlines()

    modified_lines = []
    for line in lines:
        modified_lines.append(modify_waters_scan_header(line))

    with open(file_path, "w") as file:
        file.writelines(modified_lines)


def waters_convert(params: types.TConfig) -> str:
    """
    Convert Waters raw file to mzML format.
    """
    logger.info(f"Converting Waters file: {params['infile']}")

    # Find the lockmass reference in the _extern.inf file
    lockmass_present = False

    if not params["lockmass_disabled"]:
        logger.info("Using Lockmass reference is enabled if present.")
    # get the list of files in the directory
    files = os.listdir(params["infile"])
    # Test if _extern.inf file is present

    extern_file = [f for f in files if "_extern.inf" in f]
    if not extern_file:
        raise WatersConvertException(
            "Unable to convert Waters file, no _extern.inf file found!"
        )
    else:
        logger.info("Found _extern.inf file.")
        # Read the _extern.inf file
        ex_file_path: str = os.path.join(params["infile"], extern_file[0])
        with open(ex_file_path, "r", encoding="latin-1", errors="strict") as f:
            lines = f.readlines()
            # Identify the function file for the REFERENCE
            for line in lines:
                if "REFERENCE" in line:
                    function_string, function_number = format_function_number(line)
                    logger.info(f"Lockmass Reference found: {function_string}")
                    logger.info(
                        f"Lockmass ScanEvent Function number: {function_number}"
                    )
                    lockmass_present = True
                    break

    waters_params: types.TConfig = dict(
        type="mzml",
        vendor="waters",
        debug=False,
        infile=params["infile"],
        index=True,
        sortbyscan=params["sortbyscan"],
        peak_picking=params["peak_picking"],
        remove_zeros=params["remove_zeros"],
        outfile=None,
        overwrite=False,
        verbose=False,
        lockmass_disabled=params["lockmass_disabled"],
        lockmass=True if lockmass_present else False,
        neg_lockmass=params["neg_lockmass"],
        pos_lockmass=params["pos_lockmass"],
        lockmass_tolerance=params["lockmass_tolerance"],
        lockmass_function_exclude=function_number if lockmass_present else None,
    )

    outfile = msconvert(waters_params)

    return outfile


def convert_raw_file(params: types.TConfig) -> str:
    """
    Convert the raw file to mzML format based on the vendor.
    """
    logger.info(f"Converting {params['vendor']} file: {params['infile']}")
    match params["vendor"].lower():
        case "thermo":
            return msconvert(params)
        case "agilent":
            return msconvert(params)
        case "waters":
            try:
                return waters_convert(params)
            except WatersConvertException as e:
                logger.error(str(e))
                raise RawFileConversionError(str(e))
        case "bruker":
            return msconvert(params)
        case "unspecified":
            logger.error("Vendor not supported, trying msconvert.")
            return msconvert(params)
        case _:
            raise RawFileConversionError("Unsupported vendor!")


def exclusion_string(x: int) -> str:
    """
    Return a string representing “all positive integers except x,”
    using “start-end” ranges.  By convention, “N-” means “N through ∞.”

    Examples:
      exclude 5 → "1-4 6-"
      exclude 1 → "2-"
      exclude 2 → "1 3-"
      exclude 3 → "1-2 4-"
    """
    if x < 1:
        raise ValueError("x must be a positive integer")

    parts = []

    # If x>1, we allow 1..(x-1).  Format as "1" if x-1==1, otherwise "1-(x-1)".
    if x > 1:
        if x - 1 == 1:
            parts.append("1")
        else:
            parts.append(f"1-{x-1}")

    # Always allow (x+1)..∞, shown as "(x+1)-"
    parts.append(f"{x+1}-")

    return " ".join(parts)


def msconvert(params):
    """
    Converts the given file to the mzML format using the msconvert tool.
    """
    raw_path: str = os.path.abspath(params["infile"])
    path = raw_path.strip("/") if raw_path.endswith("/") else raw_path
    directory = os.path.dirname(path)
    filename = os.path.basename(path)

    logger.info(f"Raw path = {raw_path}")
    logger.info(f"File path = {path}")
    logger.info(f"Converting {params['infile']} to {params['type']} format.")
    logger.info(f"Input directory: {directory}")
    logger.info(f"Input filename: {filename}")

    if params["outfile"] is not None:
        outfilename = os.path.basename(params["outfile"])
        base = os.path.splitext(outfilename)[0]
    else:
        base = os.path.splitext(filename)[0]

    filter_string = ""
    if params["type"] == "mzxml":
        filter_string += " --mzXML"
        outfile = base + ".mzXML"
    elif params["type"] == "mgf":
        filter_string += " --mgf"
        outfile = base + ".mgf"
    else:
        filter_string += " --mzML"
        outfile = base + ".mzML"

    logger.info(f"Output file: {outfile}")
    filter_string += f' --outfile "/data/{outfile}"'

    if params["index"] is False:
        filter_string += " --noindex"

    if params["peak_picking"] == "all":
        filter_string += " --filter 'peakPicking true 1-'"
    elif params["peak_picking"] == "ms1":
        filter_string += " --filter 'peakPicking true 1'"
    elif params["peak_picking"] == "msms":
        filter_string += " --filter 'peakPicking true 2-'"

    if params["sortbyscan"] is True:
        filter_string += " --filter 'sortByScanTime'"

    if params["remove_zeros"] is True:
        filter_string += " --filter 'zeroSamples removeExtra'"
    if params["lockmass"]:
        if params["neg_lockmass"] is not None:
            neg_lockmass = params["neg_lockmass"]
        else:
            neg_lockmass = 554.2615
        if params["pos_lockmass"] is not None:
            pos_lockmass = params["pos_lockmass"]
        else:
            pos_lockmass = 556.2771
        if params["lockmass_tolerance"] is not None:
            lockmass_tolerance = params["lockmass_tolerance"]
        else:
            lockmass_tolerance = 0.1
        filter_string += f" --filter 'lockmassRefiner mz={pos_lockmass} mzNegIons={neg_lockmass} tol={lockmass_tolerance}'"

        if params["lockmass_function_exclude"] is not None:
            filter_string += f" --filter 'scanEvent {exclusion_string(params['lockmass_function_exclude'])}'"

    cmd = "docker run --rm -v '{}':/data {} wine msconvert '/data/{}' {}".format(
        directory, docker_image, filename, filter_string
    )

    logger.info("Running msconvert")

    _output = run_cmd(cmd)

    logger.info("Conversion complete.")

    return os.path.join(directory, outfile)
