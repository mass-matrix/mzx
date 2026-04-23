__version__ = "0.3.2"

import csv
import os
import re
import shlex
import struct
import subprocess
from pathlib import Path

from lxml import etree
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


def parse_chroinf(path):
    """
    Parse a Waters _CHROMS.INF file.

    Reads channel metadata (name and unit) for each analog data file.

    Args:
        path: Path to the _CHROMS.INF file.

    Returns:
        List of [name, unit] pairs for each chromatogram channel.
    """
    analog_info = []
    file_size = os.path.getsize(path)
    with open(path, "rb") as f:
        f.seek(0x84)
        while f.tell() < file_size:
            raw = f.read(0x55)
            if not raw:
                break
            line = re.sub(
                r"[\0-\x04]|\$CC\$|\([0-9]*\)", "", raw.decode("latin-1")
            ).strip()
            parts = line.split(",")
            info = [parts[0]]
            if len(parts) == 6:
                info.append(parts[5])
            analog_info.append(info)
    return analog_info


def parse_chrodat(path):
    """
    Parse a Waters _CHRO*.DAT binary file.

    Each sample is 8 bytes: two little-endian 32-bit floats (time, intensity).
    Data starts at offset 0x80.

    Args:
        path: Path to the _CHRO*.DAT file.

    Returns:
        Tuple of (times, intensities) as lists of floats, or None if empty.
    """
    data_start = 0x80
    file_size = os.path.getsize(path)
    num_samples = (file_size - data_start) // 8
    if num_samples == 0:
        return None

    times = []
    intensities = []
    with open(path, "rb") as f:
        f.seek(data_start)
        for _ in range(num_samples):
            t, v = struct.unpack("<ff", f.read(8))
            times.append(t)
            intensities.append(v)
    return times, intensities


def get_chromatogram_info(raw_dir):
    """
    Locate and parse _CHROMS.INF from a Waters .raw directory.

    Args:
        raw_dir: Path to the Waters .raw directory.

    Returns:
        List of chromatogram channel metadata from parse_chroinf.
    """
    for f in os.listdir(raw_dir):
        if f.lower() == "_chroms.inf":
            return parse_chroinf(os.path.join(raw_dir, f))
    return []


def write_chrom_csv(filename, times, intensities):
    """
    Write chromatogram data to a CSV file.

    Args:
        filename: Output CSV file path.
        times: List of time values.
        intensities: List of intensity values.
    """
    with open(filename, mode="w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["time", "intensity"])
        writer.writeheader()
        for t, v in zip(times, intensities):
            writer.writerow({"time": f"{t:.6f}", "intensity": f"{v:.6f}"})


def export_chromatograms(raw_dir, chrom_info):
    """
    Extract and export all chromatogram channels from a Waters .raw directory to CSV.

    Output files are written to the parent directory of the .raw folder,
    named {raw_name}_{channel_name}.csv.

    Args:
        raw_dir: Path to the Waters .raw directory.
        chrom_info: Channel metadata from get_chromatogram_info().

    Returns:
        List of output CSV file paths.
    """
    parent_path = Path(raw_dir).parent.absolute()
    raw_name = Path(raw_dir).name
    pattern = re.compile(r"_chro(\d+)", re.IGNORECASE)
    output_files = []

    for f in sorted(os.listdir(raw_dir)):
        f_base, f_ext = os.path.splitext(f)
        if f_ext.lower() != ".dat":
            continue
        match = pattern.match(f_base)
        if not match:
            continue

        number = int(match.group(1))
        result = parse_chrodat(os.path.join(raw_dir, f))
        if result is None:
            logger.warning(f"Skipping empty chromatogram file: {f}")
            continue

        times, intensities = result
        # Convert times from minutes to seconds
        times = [t * 60 for t in times]

        if number <= len(chrom_info):
            channel_name = chrom_info[number - 1][0]
        else:
            channel_name = f"channel_{number}"

        csv_name = f"{raw_name}_{channel_name}.csv"
        csv_path = str(parent_path / csv_name)
        write_chrom_csv(csv_path, times, intensities)
        logger.info(f"Exported chromatogram: {csv_path}")
        output_files.append(csv_path)

    return output_files


def extract_tic_from_mzml(mzml_path, output_csv=None):
    """
    Extract the Total Ion Current (TIC) from an mzML file and write to CSV.

    Parses each spectrum element for scan start time and total ion current.
    Times are converted from minutes to seconds.

    Args:
        mzml_path: Path to the mzML file.
        output_csv: Output CSV path. Defaults to {mzml_base}_TIC.csv.

    Returns:
        Path to the output CSV file.
    """
    if output_csv is None:
        base = os.path.splitext(mzml_path)[0]
        output_csv = f"{base}_TIC.csv"

    times = []
    tics = []

    for event, elem in etree.iterparse(
        mzml_path, events=("end",), tag="{http://psi.hupo.org/ms/mzml}spectrum"
    ):
        rt = None
        tic = None
        # Check cvParams directly under spectrum and under scanList/scan
        for cv in elem.iterdescendants("{http://psi.hupo.org/ms/mzml}cvParam"):
            acc = cv.get("accession")
            if acc == "MS:1000016":  # scan start time
                rt = float(cv.get("value"))
                unit = cv.get("unitName", "minute")
                if unit == "minute":
                    rt *= 60.0
            elif acc == "MS:1000285":  # total ion current
                tic = float(cv.get("value"))
        if rt is not None and tic is not None:
            times.append(rt)
            tics.append(tic)
        elem.clear()

    write_chrom_csv(output_csv, times, tics)
    logger.info(f"Exported TIC: {output_csv} ({len(times)} scans)")
    return output_csv


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
