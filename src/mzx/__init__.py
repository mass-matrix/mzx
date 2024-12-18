__version__ = "0.2.2"

import csv
from loguru import logger
import os
from pathlib import Path
import re
import sys
import shlex
import subprocess
from typing import Optional

from pydantic import BaseModel

import pprint

pp = pprint.PrettyPrinter(indent=4)

docker_image = "chambm/pwiz-skyline-i-agree-to-the-vendor-licenses"


class ParameterModel(BaseModel):
    output_type: Optional[str] = None
    peak_piking_flag: Optional[bool] = None
    lockmass_flag: Optional[bool] = None
    lockmass_value: Optional[float] = None
    remove_zeros_flag: Optional[bool] = None
    sortbyscan_flag: Optional[bool] = None
    scan_event_filter_value: Optional[int] = None


def run_cmd(cmd: str) -> str:
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


def get_vendor_from_directory(datafile: str) -> str:
    """
    Determine the vendor of the file based on the filename.
    Currently only supports Thermo and Agilent.
    Returns the vendor name.
    If the vendor is not supported, returns None.
    """

    directory_psuedo_ext = datafile.lower().split(".")[-1].replace("/", "")
    # parse extension from directory example: data.raw -> .raw

    match directory_psuedo_ext:
        case "d":
            return "bruker"
        case "raw":
            return "waters"
        case _:
            return "unspecified"


def get_vendor_from_file(datafile: str) -> str:
    """
    Determine the vendor of the file based on the filename.
    Currently only supports Thermo and Agilent.
    Returns the vendor name.
    If the vendor is not supported, returns None.
    """

    base, ext = os.path.splitext(datafile)
    match ext.lower():
        case ".raw":
            return "thermo"
        case ".d":
            return "agilent"

        # TODO: Add support for other file types
        # case ".mzml":
        #     return "mzml"
        # case ".mzxml":
        #     return "mzxml"
        # case ".mgf":
        #     return "mgf"
        # case ".csv":
        #     return "csv"
        # case ".txt":
        #     return "txt"

        case _:
            return "unspecified"


def get_vendor(datafile: str) -> str:
    """
    Determine the vendor of the file.
    If the vendor is not supported, returns unspecified.
    """
    logger.info(f"Getting vendor for file: {datafile}")

    # determine if file is folder or directory

    if os.path.isdir(datafile):
        logger.info(f"File is a directory.")
        vendor = get_vendor_from_directory(datafile)
        logger.info(f"Vendor is {vendor}.")
        return vendor
    else:
        logger.info(f"File is a not a directory.")
        vendor = get_vendor_from_file(datafile)
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
    with open(path, "rb") as f:

        # header = re.sub(r"[\0-\x04]|\$CC\$|\([0-9]*\)", "", f.read(0x55)).strip()
        # print(header)

        f.seek(0x84)  # start offset
        while f.tell() < os.path.getsize(path):
            read_tmp = f.read(0x55).decode("latin-1")
            line = re.sub(r"[\0-\x04]|\$CC\$|\([0-9]*\)", "", read_tmp).strip()

            split = line.split(",")
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


def get_chromatogram_information(waters_files: str) -> list:

    waters_data = os.listdir(waters_files)
    chrmo_info = list()

    for f in waters_data:
        if "_chroms.inf" in f.lower():
            chrmo_info = parse_chroinf(os.path.join(waters_files, f))
    return chrmo_info


def write_chrom_csv(filename, time, intensity):
    fieldnames = ["time", "intensity"]

    with open(filename, mode="w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=fieldnames)
        writer.writeheader()

        for idx, _ in enumerate(time):
            writer.writerow(
                {
                    "time": f"{time[idx]:.6f}",  # Format to 6 decimal places
                    "intensity": f"{intensity[idx]:.6f}",
                }
            )


def export_chromtatograms_csv(waters_file: str, chrmo_info: list):

    # get the list of files in the directory
    waters_files = os.listdir(waters_file)
    parent_path = Path(waters_file).parent.absolute()
    name = Path(waters_file).name

    # Test if _extern.inf file is present

    pattern = r"_chro(\d+)"

    for f in waters_files:

        f_base, f_ext = os.path.splitext(f)

        if "chro" in f_base.lower() and f_ext.lower() == ".dat":
            match = re.match(pattern, f_base)
            if match:
                # Extract and print the number
                number = int(match.group(1))

                x, y = parse_chrodat(os.path.join(waters_file, f))

                x = x * 60

                new_list = zip(x.tolist(), y.tolist())

                csv_name = f"{name}_{chrmo_info[number-1][0]}.csv"
                csv_filename = f"{os.path.join(parent_path,csv_name)}"

                write_chrom_csv(csv_filename, x, y)
                # with open(filename, "w+") as csvfile:
                #     filewriter = csv.writer(csvfile)
                #     filewriter.writerow(["time", "intensity"])
                #     filewriter.writerows(zip(x, y))

                # with open(csv_filename, "w+") as csvfile:

                #     filewriter = csv.writer(csvfile)
                #     filewriter.writerow(["time", "intensity"])
                #     filewriter.writerows(new_list)


def waters_convert(
    file, export_chrromatograms=True, lockmass=False, lockmass_value=None
):
    """
    Convert Waters raw file to mzML format.
    """

    logger.info(f"Converting Waters file: {file}")

    # get the list of files in the directory
    files = os.listdir(file)
    parent = Path(file).parent
    name = Path(file).name
    parent_path = Path(file).parent.absolute()
    # Test if _extern.inf file is present

    if export_chrromatograms:
        analog_info = get_chromatogram_information(file)
        export_chromtatograms_csv(file, analog_info)

    # print(analog_info)

    # pattern = r"_chro(\d+)"

    # for f in files:
    #     f_base, f_ext = os.path.splitext(f)

    #     if "chro" in f_base.lower():

    #         match = re.match(pattern, f_base.lower())
    #         if match:
    #             # Extract and print the number
    #             number = int(match.group(1))

    #         if f_ext.lower() == ".dat":
    #             x, y = parse_chrodat(os.path.join(file, f))
    #             # pp.pprint([x, y])

    #             new_list = zip(x.tolist(), y.tolist())
    #             csv_name = f"{name}_{analog_info[number-1][0]}.csv"
    #             csv_filename = f"{os.path.join(parent_path,csv_name)}"

    #             with open(csv_filename, "w+") as csvfile:
    #                 filewriter = csv.writer(csvfile)
    #                 filewriter.writerows(new_list)

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
                line = line.strip()

                if "REFERENCE" in line:
                    function_string, function_number = format_function_number(line)
                    logger.info(f"Lockmass Reference found: {function_string}")
                    lockmass = True
                elif "ReferenceMass" in line:
                    # get the lockmass value from a line formatted like this "ReferenceMass1	1,554.2620221"
                    # TODO check if the lockmass is a single value or multiple value
                    lockmass_value = float(line.split("\t")[1].split(",")[1])
                    lockmass = True
                    logger.info(f"Lockmass value: {lockmass_value}")
                elif "lockspray reference compound name" in line.lower():
                    if "leu neg" in line.lower():
                        lockmass = True
                        lockmass_value = 554.2620221

    if lockmass == True and lockmass_value == None:
        logger.error("Lockmass value not found.")
        return None

    logger.info("Processing Waters file")

    logger.info("Processing Waters file First Pass")

    parent = Path(file).parent
    name = Path(file).name

    outfile = msconvert(
        os.path.join(parent, name),
        index=True,
        sortbyscan=True,
        peak_picking=True,
        remove_zeros=True,
        scan_event=function_number,
        lockmass=lockmass,
        lockmass_value=lockmass_value,
    )

    # process_waters_scan_headers(outfile)

    return outfile


def convert_raw_file(datafile: str, parameters: ParameterModel) -> str:
    """
    Convert the raw file to mzML format based on the vendor.
    """

    out_type = parameters.output_type

    vendor = get_vendor(datafile)

    logger.info(f"Converting {vendor} file: {datafile}")

    match vendor:
        case "thermo":
            outfile = msconvert(datafile)

        case "agilent":
            outfile = msconvert(datafile)

        case "waters":
            outfile = waters_convert(datafile)

        case "bruker":
            outfile = msconvert(datafile)

        case "unspecified":
            logger.error("Vendor not supported, trying msconvert.")
            outfile = msconvert(datafile)

        case _:
            logger.error("Cannot covert file.")

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
    print(file)
    path = os.path.abspath(file)
    print(path)
    directory = os.path.dirname(path)
    filename = os.path.basename(file)
    print(file, filename)
    logger.info(f"File path = {path}")
    logger.info(f"Converting {file} to {type} format.")
    logger.info(f"Input directory: {directory}")
    logger.info(f"Input filename: {filename}")

    if outfile is not None:
        outfilename = os.path.basename(outfile)
        base = os.path.splitext(outfilename)[0]
    else:
        base = os.path.splitext(filename)[0]

    print(outfile)

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

    if peak_picking is True:
        params += " --filter 'peakPicking true 2-'"

    if lockmass is True:
        if int(lockmass_value) in [554, 556]:
            logger.info(f"Lockmass value recognized LeuEnk: {lockmass_value}")
            params += f" --filter 'lockmassRefiner mz=556.2771 mzNegIons=554.2615'"
        else:
            logger.error("Lockmass value not provided or not know. Ignoring lockmass.")
            lockmass = False

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
