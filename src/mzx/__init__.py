__version__ = "0.2.2"

import os
import re
import shlex
import subprocess

from loguru import logger

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


def get_vendor(file):
    """
    Determine the vendor of the file.
    Currently only supports Thermo and Agilent.
    Returns the vendor name.
    If the vendor is not supported, returns None.
    """
    logger.info(f"Getting vendor for file: {file}")
    # determine if file is folder or directory

    vendor = None

    if os.path.isdir(file):
        logger.info("File is a directory.")
        if ".d" in file:
            vendor = "bruker"
        elif ".raw" in file:
            vendor = "waters"
        else:
            files = os.listdir(file)
            for f in files:
                if "_FUNC" in f:
                    vendor = "waters"
                else:
                    vendor = "unspecified"
    else:
        logger.info("File is a directory.")
        if file.endswith(".raw"):
            vendor = "Thermo"
        elif file.endswith(".d"):
            vendor = "Agilent"
        else:
            vendor = "unspecified"

    logger.info(f"Vendor is {vendor}.")
    return vendor


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


import numpy as np


def parse_chroinf(path):
    """
    Parses a Waters _CHROMS.INF file.

    Retrieves the name and unit for each analog data file. 

    Args:
        path (str): Path to the _CHROMS.INF file. 
    
    Returns:
        List of string lists that contain the name and unit (if it exists) \
            of the data in each analog file. 

    """

    analog_info = []
    with open(path, "r") as f:

        # header = re.sub(r"[\0-\x04]|\$CC\$|\([0-9]*\)", "", f.read(0x55)).strip()
        # print(header)

        f.seek(0x84)  # start offset
        while f.tell() < os.path.getsize(path):
            read_tmp = f.read(0x55)

            line = re.sub(r"[\0-\x04]|\$CC\$|\([0-9]*\)", "", read_tmp).strip()

            split = line.split(",")
            print(split)
            info = []
            info.append(split[0])  # name
            if len(split) == 6:
                info.append(split[5])  # unit
            analog_info.append(info)

    return analog_info


def parse_chrodat(path):
    """
    Parses a Waters _CHRO .DAT file.

    These files may contain data for CAD, ELSD, or UV channels. \
        They may also contain other miscellaneous data like system pressure.
    
    IMPORTANT: MassLynx classifies these files as "analog" data, but \
        a DataDirectory will not treat CAD, ELSD, or UV channels as analog. \
        Instead, those channels will be mapped to their detector.

    Learn more about this file format :ref:`here <chrodat>`.

    Args:
        path (str): Path to the _CHRO .DAT file. 
        name (str): Name of the analog data.
        units (str, optional): Units of the analog data.
    
    Returns:
        DataFile with analog data, if the file can be parsed. Otherwise, None.

    """
    data_start = 0x80

    num_times = (os.path.getsize(path) - data_start) // 8
    if num_times == 0:
        return None

    with open(path, "rb") as f:
        raw_bytes = f.read()
    times = np.ndarray(num_times, "<f", raw_bytes, data_start, 8)
    # print(times)
    vals = np.ndarray(num_times, "<f", raw_bytes, data_start + 4, 8)

    return times, vals


import pprint

pp = pprint.PrettyPrinter(indent=4)

import csv

import sys
from pathlib import Path
import re


def waters_convert(file, two_pass=False):
    """
    Convert Waters raw file to mzML format.
    """
    logger.info(f"Converting Waters file: {file}")

    # get the list of files in the directory
    files = os.listdir(file)
    path = Path(file)
    parent_path = path.parent.absolute()
    # Test if _extern.inf file is present

    for f in files:

        if "_chroms.inf" == f.lower():
            print(f)
            analog_info = parse_chroinf(os.path.join(file, f))
            print(analog_info)

    pattern = r"_chro(\d+)"

    for f in files:
        f_base, f_ext = os.path.splitext(f)

        if "chro" in f_base.lower():
            match = re.match(pattern, f_base)
            if match:
                # Extract and print the number
                number = int(match.group(1))
                print(f"File: {f} -> Number: {number}")

            print(f)
            if f_ext.lower() == ".dat":
                x, y = parse_chrodat(os.path.join(file, f))
                # pp.pprint([x, y])

                new_list = zip(x.tolist(), y.tolist())

                csv_filename = (
                    f"{os.path.join(parent_path,analog_info[number-1][0])}.csv"
                )

                with open(csv_filename, "w+") as csvfile:
                    filewriter = csv.writer(csvfile)
                    filewriter.writerows(new_list)

    sys.exit()
    extern_file = [f for f in files if "_extern.inf" in f.lower()]
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
                    function_string, function_number = format_function_number(line)

                    logger.info(f"Lockmass Reference found: {function_string}")
                    break

    # print(two_pass)
    # if two_pass:
    #     # Identify the function files
    #     func_files = [f for f in files if "_FUNC" in f]

    #     old_function_file = None
    #     old_function_file = None
    #     for func_file in func_files:
    #         if function_string in func_file and (
    #             ".dat" in func_file or ".DAT" in func_file
    #         ):
    #             logger.info(f"Function file found: {func_file}")
    #             old_function_file = os.path.join(file, func_file)
    #             new_function_file = os.path.join(file, func_file + ".tmp")

    #     logger.info("Renaming lockmass function file")
    #     if old_function_file and new_function_file:
    #         os.rename(old_function_file, new_function_file)

    #     logger.info("Processing Waters file First Pass")
    #     outfile = msconvert(
    #         file, index=False, sortbyscan=True, peak_picking=True, remove_zeros=True
    #     )
    #     outfile_temp = (
    #         os.path.splitext(outfile)[0] + "_tmp" + os.path.splitext(outfile)[1]
    #     )

    #     os.rename(outfile, outfile_temp)

    #     logger.info("Processing Waters scan headers")
    #     process_waters_scan_headers(outfile_temp)

    #     logger.info("Processing Waters mzML (adding index)")
    #     outfile = msconvert(
    #         outfile_temp,
    #         outfile=outfile,
    #         index=True,
    #         peak_picking=True,
    #         scan_event=
    #         remove_zeros=True,
    #     )

    #     os.remove(outfile_temp)
    #     if new_function_file and old_function_file:
    #         logger.info("Restoring lockmass function file")
    #         os.rename(new_function_file, old_function_file)

    # else:

    if True:
        logger.info("Processing Waters file")

        logger.info("Processing Waters file First Pass")

        outfile = msconvert(
            file,
            index=True,
            sortbyscan=True,
            peak_picking=True,
            remove_zeros=True,
            scan_event=function_number,
            lockmass=True,
            lockmass_value=554.2615,
        )

        process_waters_scan_headers(outfile)

    return outfile


def convert_raw_file(file, vendor):
    """
    Convert the raw file to mzML format based on the vendor.
    """
    logger.info(f"Converting {vendor} file: {file}")
    match vendor.lower():
        case "thermo":
            outfile = msconvert(file)
            return outfile

        case "agilent":
            outfile = msconvert(file)
            return outfile

        case "waters":
            outfile = waters_convert(file)
            return outfile

        case "bruker":
            outfile = msconvert(file)
            return outfile

        case "unspecified":
            logger.error("Vendor not supported, trying msconvert.")
            outfile = msconvert(file)
            return outfile


def msconvert(
    file,
    outfile=None,
    type="mzml",
    index=True,
    scan_event=None,
    lockmass=False,
    lockmass_value=None,
    peak_picking=True,
    remove_zeros=True,
    sortbyscan=False,
):
    """
    Converts the given file to the mzML format using the msconvert tool.
    """
    params = ""
    path = os.path.abspath(file)
    directory = os.path.dirname(path)
    filename = os.path.basename(file)

    logger.info(f"File path = {path}")
    logger.info(f"Converting {file} to {type} format.")
    logger.info(f"Input directory: {directory}")
    logger.info(f"Input filename: {filename}")

    if outfile is not None:
        outfilename = os.path.basename(outfile)
        base = os.path.splitext(outfilename)[0]
    else:
        base = os.path.splitext(filename)[0]

    if type == "mzxml":
        params += " --mzXML"
        outfile = base + ".mzXML"
    elif type == "mgf":
        params += " --mgf"
        outfile = base + ".mgf"
    else:
        params += " --mzML"
        outfile = base + ".mzML"

    logger.info(f"Output file: {outfile}")
    params += f' --outfile "/data/{outfile}"'

    if index is False:
        params += " --noindex"

    if lockmass is True:
        if lockmass_value is not None:
            params += f" --filter 'lockmassRefiner mz=556.2771 mzNegIons=554.2615'"
        else:
            logger.error("Lockmass value not provided. Ignoring lockmass.")
            lockmass = False

    if peak_picking is True:
        params += " --filter 'peakPicking true 1-'"

    if scan_event is not None:
        params += f" --filter 'scanEvent  1-{scan_event-1} {scan_event+1}-'"

    if sortbyscan is True:
        params += " --filter 'sortByScanTime'"

    if remove_zeros is True:
        params += " --filter 'zeroSamples removeExtra'"

    cmd = "docker run --rm -v '{}':/data {} wine msconvert '/data/{}' {}".format(
        directory, docker_image, filename, params
    )

    logger.info("Running msconvert")

    _output = run_cmd(cmd)

    logger.info("Conversion complete.")

    return os.path.join(directory, outfile)
