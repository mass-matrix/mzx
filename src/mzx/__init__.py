__version__ = "0.3.0"

import os
import re
import shlex
import subprocess
from loguru import logger

from . import types

docker_image = "chambm/pwiz-skyline-i-agree-to-the-vendor-licenses"


def run_cmd(cmd):
    """
    Run a command and return the output.
    """
    cmd = shlex.split(cmd, posix=True)
    logger.info(f"Running command: {cmd}")
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
        return f"_FUNC{function_number:03d}"
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


def waters_convert(file, two_pass=False):
    """
    Convert Waters raw file to mzML format.
    """
    logger.info(f"Converting Waters file: {file}")

    # get the list of files in the directory
    files = os.listdir(file)
    # Test if _extern.inf file is present

    extern_file = [f for f in files if "_extern.inf" in f]
    if not extern_file:
        logger.error("Could not find _extern.inf file.")
        return None
    else:
        logger.info("Found _extern.inf file.")
        # Read the _extern.inf file
        ex_file_path: str = os.path.join(file, extern_file[0])
        with open(ex_file_path, "r", encoding="latin-1", errors="strict") as f:
            lines = f.readlines()
            # Identify the function file for the REFERENCE

            for line in lines:
                if "REFERENCE" in line:
                    function_string = format_function_number(line)

                    logger.info(f"Reference found: {function_string}")
                    break

    # Identify the function files

    func_files = [f for f in files if "_FUNC" in f]
    old_function_file = None
    old_function_file = None
    for func_file in func_files:
        if function_string in func_file and (
            ".dat" in func_file or ".DAT" in func_file
        ):
            logger.info(f"Function file found: {func_file}")
            old_function_file = os.path.join(file, func_file)
            new_function_file = os.path.join(file, func_file + ".tmp")

    logger.info("Renaming lockmass function file")
    if old_function_file and new_function_file:
        os.rename(old_function_file, new_function_file)

    if two_pass:
        logger.info("Processing Waters file First Pass")
        two_pass_config: types.TConfig = dict(
            type="mzml",
            vendor="waters",
            debug=False,
            infile=file,
            index=False,
            sortbyscan=True,
            peak_picking="all",
            remove_zeros=True,
            outfile=None,
            overwrite=False,
            verbose=True,
        )
        outfile = msconvert(two_pass_config)
        outfile_temp = (
            os.path.splitext(outfile)[0] + "_tmp" + os.path.splitext(outfile)[1]
        )

        os.rename(outfile, outfile_temp)

        logger.info("Processing Waters scan headers")
        process_waters_scan_headers(outfile_temp)

        logger.info("Processing Waters mzML (adding index)")
        two_pass_config_2: types.TConfig = dict(
            type="mzml",
            vendor="waters",
            debug=False,
            infile=outfile_temp,
            index=True,
            sortbyscan=False,
            peak_picking="all",
            remove_zeros=True,
            outfile=None,
            overwrite=False,
            verbose=True,
        )
        outfile = msconvert(two_pass_config_2)

        # os.remove(outfile_temp)

    else:
        logger.info("Processing Waters file")
        single_pass_config: types.TConfig = dict(
            type="mzml",
            vendor="waters",
            debug=False,
            infile=file,
            index=True,
            sortbyscan=True,
            peak_picking="all",
            remove_zeros=True,
            outfile=None,
            overwrite=False,
            verbose=False,
        )
        outfile = msconvert(single_pass_config)

    if new_function_file and old_function_file:
        logger.info("Restoring lockmass function file")
        os.rename(new_function_file, old_function_file)

    return outfile


def convert_raw_file(params: types.TConfig) -> str:
    """
    Convert the raw file to mzML format based on the vendor.
    """
    logger.info(f"Converting {params['vendor']} file: {params['infile']}")
    match params["vendor"].lower():
        case "thermo":
            outfile = msconvert(params)
            return outfile

        case "agilent":
            outfile = msconvert(params)
            return outfile

        case "waters":
            outfile = waters_convert(params)
            return outfile

        case "bruker":
            outfile = msconvert(params)
            return outfile

        case "unspecified":
            logger.error("Vendor not supported, trying msconvert.")
            outfile = msconvert(params)
            return outfile
        case _:
            raise Exception("Invalid vendor")


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

    cmd = "docker run --rm -v '{}':/data {} wine msconvert '/data/{}' {}".format(
        directory, docker_image, filename, filter_string
    )

    logger.info("Running msconvert")

    _output = run_cmd(cmd)

    logger.info("Conversion complete.")

    return os.path.join(directory, outfile)
