"""Minimal mzML 1.1 writer and reader using lxml.

No vendor libraries are used. Binary arrays are written as 64-bit float,
base64-encoded, optionally zlib-compressed.
"""

from __future__ import annotations

import base64
import struct
import zlib
from pathlib import Path
from typing import Iterable, Mapping, Optional, cast

from lxml import etree

from .base import Chromatogram, Polarity, Spectrum

MZML_NS = "http://psi.hupo.org/ms/mzml"
XSI_NS = "http://www.w3.org/2001/XMLSchema-instance"
# lxml uses None for the default xmlns; stubs only accept Mapping[str, str].
NSMAP = cast(Mapping[str, str], {None: MZML_NS, "xsi": XSI_NS})


def _cv(
    parent: etree._Element,
    accession: str,
    name: str,
    value: Optional[str] = None,
    unit_accession: Optional[str] = None,
    unit_name: Optional[str] = None,
) -> etree._Element:
    attrs = {
        "cvRef": "MS",
        "accession": accession,
        "name": name,
    }
    if value is not None:
        attrs["value"] = value
    if unit_accession is not None:
        attrs["unitCvRef"] = "UO" if unit_accession.startswith("UO:") else "MS"
        attrs["unitAccession"] = unit_accession
        if unit_name is not None:
            attrs["unitName"] = unit_name
    return etree.SubElement(parent, f"{{{MZML_NS}}}cvParam", attrs)


def _encode_binary(values: list[float], compress: bool = False) -> tuple[str, int]:
    """Encode float64 little-endian array as base64; return (text, decoded_byte_len)."""
    raw = struct.pack(f"<{len(values)}d", *values)
    if compress:
        raw = zlib.compress(raw)
    return base64.b64encode(raw).decode("ascii"), len(raw)


_COMPRESS_BINARY = False


def _binary_array(
    parent: etree._Element,
    values: list[float],
    accession: str,
    name: str,
) -> None:
    array_list = parent.find(f"{{{MZML_NS}}}binaryDataArrayList")
    if array_list is None:
        array_list = etree.SubElement(
            parent,
            f"{{{MZML_NS}}}binaryDataArrayList",
            {"count": "0"},
        )

    encoded, nbytes = _encode_binary(values, compress=_COMPRESS_BINARY)
    bda = etree.SubElement(
        array_list,
        f"{{{MZML_NS}}}binaryDataArray",
        {"encodedLength": str(len(encoded))},
    )
    _cv(bda, "MS:1000523", "64-bit float")
    if _COMPRESS_BINARY:
        _cv(bda, "MS:1000574", "zlib compression")
    else:
        _cv(bda, "MS:1000576", "no compression")
    _cv(bda, accession, name)
    binary = etree.SubElement(bda, f"{{{MZML_NS}}}binary")
    binary.text = encoded
    count = len(array_list.findall(f"{{{MZML_NS}}}binaryDataArray"))
    array_list.set("count", str(count))
    bda.set("arrayLength", str(len(values)))
    _ = nbytes


def _add_spectrum(spectrum_list: etree._Element, spectrum: Spectrum) -> None:
    el = etree.SubElement(
        spectrum_list,
        f"{{{MZML_NS}}}spectrum",
        {
            "index": str(spectrum.index),
            "id": spectrum.scan_id,
            "defaultArrayLength": str(len(spectrum.mz)),
        },
    )
    _cv(el, "MS:1000511", "ms level", str(spectrum.ms_level))
    if spectrum.ms_level == 1:
        _cv(el, "MS:1000579", "MS1 spectrum")
    else:
        _cv(el, "MS:1000580", "MSn spectrum")
    if spectrum.is_centroid:
        _cv(el, "MS:1000127", "centroid spectrum")
    else:
        _cv(el, "MS:1000128", "profile spectrum")
    if spectrum.polarity == "positive":
        _cv(el, "MS:1000130", "positive scan")
    elif spectrum.polarity == "negative":
        _cv(el, "MS:1000129", "negative scan")
    if spectrum.total_ion_current is not None:
        _cv(el, "MS:1000285", "total ion current", f"{spectrum.total_ion_current}")
    if spectrum.base_peak_mz is not None:
        _cv(
            el,
            "MS:1000504",
            "base peak m/z",
            f"{spectrum.base_peak_mz}",
            unit_accession="MS:1000040",
            unit_name="m/z",
        )
    if spectrum.base_peak_intensity is not None:
        _cv(
            el,
            "MS:1000505",
            "base peak intensity",
            f"{spectrum.base_peak_intensity}",
            unit_accession="MS:1000131",
            unit_name="number of detector counts",
        )

    scan_list = etree.SubElement(el, f"{{{MZML_NS}}}scanList", {"count": "1"})
    _cv(scan_list, "MS:1000795", "no combination")
    scan = etree.SubElement(scan_list, f"{{{MZML_NS}}}scan")
    _cv(
        scan,
        "MS:1000016",
        "scan start time",
        f"{spectrum.retention_time_sec / 60.0}",
        unit_accession="UO:0000031",
        unit_name="minute",
    )
    if spectrum.filter_string is not None:
        _cv(scan, "MS:1000512", "filter string", spectrum.filter_string)
    if spectrum.ion_injection_time_ms is not None:
        _cv(
            scan,
            "MS:1000927",
            "ion injection time",
            f"{spectrum.ion_injection_time_ms}",
            unit_accession="UO:0000028",
            unit_name="millisecond",
        )
    if spectrum.scan_window_lower is not None or spectrum.scan_window_upper is not None:
        scan_window_list = etree.SubElement(
            scan, f"{{{MZML_NS}}}scanWindowList", {"count": "1"}
        )
        scan_window = etree.SubElement(scan_window_list, f"{{{MZML_NS}}}scanWindow")
        if spectrum.scan_window_lower is not None:
            _cv(
                scan_window,
                "MS:1000501",
                "scan window lower limit",
                f"{spectrum.scan_window_lower}",
                unit_accession="MS:1000040",
                unit_name="m/z",
            )
        if spectrum.scan_window_upper is not None:
            _cv(
                scan_window,
                "MS:1000500",
                "scan window upper limit",
                f"{spectrum.scan_window_upper}",
                unit_accession="MS:1000040",
                unit_name="m/z",
            )

    if spectrum.ms_level >= 2 and spectrum.precursor_mz is not None:
        precursor_list = etree.SubElement(
            el, f"{{{MZML_NS}}}precursorList", {"count": "1"}
        )
        precursor = etree.SubElement(precursor_list, f"{{{MZML_NS}}}precursor")
        selected = etree.SubElement(
            precursor, f"{{{MZML_NS}}}selectedIonList", {"count": "1"}
        )
        ion = etree.SubElement(selected, f"{{{MZML_NS}}}selectedIon")
        _cv(
            ion,
            "MS:1000744",
            "selected ion m/z",
            f"{spectrum.precursor_mz}",
            unit_accession="MS:1000040",
            unit_name="m/z",
        )
        if spectrum.precursor_charge is not None:
            _cv(ion, "MS:1000041", "charge state", str(spectrum.precursor_charge))
        if spectrum.collision_energy is not None:
            activation = etree.SubElement(precursor, f"{{{MZML_NS}}}activation")
            _cv(
                activation,
                "MS:1000045",
                "collision energy",
                f"{spectrum.collision_energy}",
                unit_accession="UO:0000266",
                unit_name="electronvolt",
            )

    etree.SubElement(el, f"{{{MZML_NS}}}binaryDataArrayList", {"count": "0"})
    _binary_array(el, spectrum.mz, "MS:1000514", "m/z array")
    _binary_array(el, spectrum.intensity, "MS:1000515", "intensity array")


def _add_chromatogram(chrom_list: etree._Element, chrom: Chromatogram) -> None:
    el = etree.SubElement(
        chrom_list,
        f"{{{MZML_NS}}}chromatogram",
        {
            "index": str(len(chrom_list)),
            "id": chrom.id,
            "defaultArrayLength": str(len(chrom.times)),
        },
    )
    _cv(el, "MS:1000235", "total ion current chromatogram")
    etree.SubElement(el, f"{{{MZML_NS}}}binaryDataArrayList", {"count": "0"})
    _binary_array(el, chrom.times, "MS:1000595", "time array")
    _binary_array(el, chrom.intensities, "MS:1000515", "intensity array")


def write_mzml(
    path: str | Path,
    spectra: Iterable[Spectrum],
    *,
    chromatograms: Optional[Iterable[Chromatogram]] = None,
    metadata: Optional[dict] = None,
    indexed: bool = False,
    compress: bool = False,
) -> str:
    """
    Write spectra (and optional chromatograms) to an mzML 1.1 file.

    Used by :func:`~mzx.convert.convert_native` to serialize vendor data without
    ProteoWizard. Retention times are stored in minutes per the mzML convention.

    Args:
        path: Output file path.
        spectra: Spectra to serialize.
        chromatograms: Optional chromatogram channels.
        metadata: Optional run metadata (``source_file``, ``vendor``,
            ``instrument_model``, ``software``).
        indexed: If True, wrap content in ``indexedmzML`` (offset placeholders).
        compress: If True, use zlib compression for binary arrays.

    Returns:
        Absolute path to the written file.

    Example::

        from mzx.convert.mzml_writer import write_mzml

        write_mzml("out.mzML", spectra, metadata={"source_file": "run.raw"})
    """
    global _COMPRESS_BINARY
    _COMPRESS_BINARY = compress

    metadata = metadata or {}
    spectra_list = list(spectra)
    chrom_list_data = list(chromatograms or [])

    mzml = etree.Element(f"{{{MZML_NS}}}mzML", nsmap=NSMAP, version="1.1.0")
    mzml.set(
        f"{{{XSI_NS}}}schemaLocation",
        f"{MZML_NS} http://psidev.info/ms/mzML/xsd/mzML1.1.0.xsd",
    )

    cv_list = etree.SubElement(mzml, f"{{{MZML_NS}}}cvList", {"count": "2"})
    etree.SubElement(
        cv_list,
        f"{{{MZML_NS}}}cv",
        {
            "id": "MS",
            "fullName": "Proteomics Standards Initiative Mass Spectrometry Ontology",
            "version": "4.1.0",
            "URI": "https://www.proteininformationresource.org/MS/",
        },
    )
    etree.SubElement(
        cv_list,
        f"{{{MZML_NS}}}cv",
        {
            "id": "UO",
            "fullName": "Unit Ontology",
            "version": "09:04:2014",
            "URI": "http://unit-ontology.googlecode.com/svn/trunk/unit.obo",
        },
    )

    file_desc = etree.SubElement(mzml, f"{{{MZML_NS}}}fileDescription")
    file_content = etree.SubElement(file_desc, f"{{{MZML_NS}}}fileContent")
    _cv(file_content, "MS:1000579", "MS1 spectrum")
    source_files = etree.SubElement(
        file_desc, f"{{{MZML_NS}}}sourceFileList", {"count": "1"}
    )
    source_name = Path(str(metadata.get("source_file", "unknown"))).name
    etree.SubElement(
        source_files,
        f"{{{MZML_NS}}}sourceFile",
        {
            "id": "SF1",
            "name": source_name,
            "location": f"file://{metadata.get('source_file', 'unknown')}",
        },
    )

    etree.SubElement(
        mzml,
        f"{{{MZML_NS}}}softwareList",
        {"count": "1"},
    ).append(
        etree.Element(
            f"{{{MZML_NS}}}software",
            {
                "id": "mzx",
                "version": str(metadata.get("software", "mzx-native")),
            },
        )
    )

    instrument_list = etree.SubElement(
        mzml, f"{{{MZML_NS}}}instrumentConfigurationList", {"count": "1"}
    )
    instrument = etree.SubElement(
        instrument_list,
        f"{{{MZML_NS}}}instrumentConfiguration",
        {"id": "IC1"},
    )
    model = metadata.get("instrument_model") or metadata.get("vendor") or "unknown"
    _cv(instrument, "MS:1000031", "instrument model", str(model))

    data_processing_list = etree.SubElement(
        mzml, f"{{{MZML_NS}}}dataProcessingList", {"count": "1"}
    )
    dp = etree.SubElement(
        data_processing_list, f"{{{MZML_NS}}}dataProcessing", {"id": "DP1"}
    )
    proc = etree.SubElement(
        dp,
        f"{{{MZML_NS}}}processingMethod",
        {"order": "1", "softwareRef": "mzx"},
    )
    _cv(proc, "MS:1000544", "Conversion to mzML")

    run = etree.SubElement(
        mzml,
        f"{{{MZML_NS}}}run",
        {
            "id": "run1",
            "defaultInstrumentConfigurationRef": "IC1",
        },
    )
    spectrum_list = etree.SubElement(
        run,
        f"{{{MZML_NS}}}spectrumList",
        {
            "count": str(len(spectra_list)),
            "defaultDataProcessingRef": "DP1",
        },
    )
    for spectrum in spectra_list:
        _add_spectrum(spectrum_list, spectrum)

    if chrom_list_data:
        chromatogram_list = etree.SubElement(
            run,
            f"{{{MZML_NS}}}chromatogramList",
            {
                "count": str(len(chrom_list_data)),
                "defaultDataProcessingRef": "DP1",
            },
        )
        for chrom in chrom_list_data:
            _add_chromatogram(chromatogram_list, chrom)

    out_path = Path(path).resolve()
    out_path.parent.mkdir(parents=True, exist_ok=True)

    if indexed:
        root = etree.Element(
            f"{{{MZML_NS}}}indexedmzML",
            nsmap=NSMAP,
        )
        root.append(mzml)
        index_list = etree.SubElement(root, f"{{{MZML_NS}}}indexList", {"count": "1"})
        index = etree.SubElement(
            index_list, f"{{{MZML_NS}}}index", {"name": "spectrum"}
        )
        for spectrum in spectra_list:
            offset = etree.SubElement(
                index,
                f"{{{MZML_NS}}}offset",
                {"idRef": spectrum.scan_id},
            )
            offset.text = "0"
        tree = etree.ElementTree(root)
    else:
        tree = etree.ElementTree(mzml)

    tree.write(
        str(out_path),
        xml_declaration=True,
        encoding="utf-8",
        pretty_print=True,
    )
    return str(out_path)


def parse_spectra_from_mzml(path: str | Path) -> list[Spectrum]:
    """
    Parse spectra from an mzML file written by :func:`write_mzml`.

    Used by round-trip and equivalence tests. Only supports the encoding produced
    by this writer (64-bit float, no compression).

    Args:
        path: Path to an mzML or indexed mzML file.

    Returns:
        List of :class:`~mzx.convert.base.Spectrum` instances in file order.
    """
    path = Path(path)
    root = etree.parse(str(path)).getroot()
    # Handle indexedmzML wrapper
    if root.tag == f"{{{MZML_NS}}}indexedmzML":
        mzml = root.find(f"{{{MZML_NS}}}mzML")
        if mzml is None:
            return []
        root = mzml

    spectra: list[Spectrum] = []
    for el in root.findall(f".//{{{MZML_NS}}}spectrum"):
        index = int(el.get("index", "0"))
        scan_id = el.get("id", f"scan={index + 1}")
        ms_level = 1
        rt_sec = 0.0
        polarity: Polarity | None = None
        tic = None
        precursor_mz = None
        precursor_charge = None
        collision_energy = None
        is_centroid = False

        for cv in el.findall(f"{{{MZML_NS}}}cvParam"):
            acc = cv.get("accession")
            if acc == "MS:1000511":
                ms_level = int(float(cv.get("value", "1")))
            elif acc == "MS:1000285":
                tic = float(cv.get("value", "0"))
            elif acc == "MS:1000130":
                polarity = "positive"
            elif acc == "MS:1000129":
                polarity = "negative"
            elif acc == "MS:1000127":
                is_centroid = True

        for cv in el.findall(f".//{{{MZML_NS}}}cvParam"):
            acc = cv.get("accession")
            if acc == "MS:1000016":
                rt = float(cv.get("value", "0"))
                unit = cv.get("unitName", "minute")
                rt_sec = rt * 60.0 if unit == "minute" else rt
            elif acc == "MS:1000744":
                precursor_mz = float(cv.get("value", "0"))
            elif acc == "MS:1000041":
                precursor_charge = int(float(cv.get("value", "0")))
            elif acc == "MS:1000045":
                collision_energy = float(cv.get("value", "0"))

        mz: list[float] = []
        intensity: list[float] = []
        for bda in el.findall(f".//{{{MZML_NS}}}binaryDataArray"):
            accessions = {
                cv.get("accession") for cv in bda.findall(f"{{{MZML_NS}}}cvParam")
            }
            binary = bda.find(f"{{{MZML_NS}}}binary")
            if binary is None or not binary.text:
                continue
            raw = base64.b64decode(binary.text)
            n = len(raw) // 8
            values = list(struct.unpack(f"<{n}d", raw))
            if "MS:1000514" in accessions:
                mz = values
            elif "MS:1000515" in accessions:
                intensity = values

        spectra.append(
            Spectrum(
                index=index,
                scan_id=scan_id,
                ms_level=ms_level,
                retention_time_sec=rt_sec,
                mz=mz,
                intensity=intensity,
                polarity=polarity,
                precursor_mz=precursor_mz,
                precursor_charge=precursor_charge,
                collision_energy=collision_energy,
                total_ion_current=tic,
                is_centroid=is_centroid,
            )
        )
    return spectra
