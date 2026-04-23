import csv
import os
import struct

from mzx import (
    export_chromatograms,
    extract_tic_from_mzml,
    get_chromatogram_info,
    parse_chrodat,
    parse_chroinf,
    write_chrom_csv,
)


def _build_chroinf(records):
    """Build a synthetic _CHROMS.INF binary file."""
    # Header: 0x84 bytes of padding
    header = b"\x00" * 0x84
    body = b""
    for name, unit in records:
        # Format: "name,$CC$,1.000000,3,0,0, unit"
        entry = f"{name}\x00$CC$,1.000000,3,0,0,{unit}"
        # Pad to 0x55 bytes
        raw = entry.encode("latin-1")
        raw = raw.ljust(0x55, b"\x00")
        body += raw
    return header + body


def _build_chrodat(samples):
    """Build a synthetic _CHRO*.DAT binary file."""
    header = b"\x00" * 0x80
    data = b""
    for t, v in samples:
        data += struct.pack("<ff", t, v)
    return header + data


class TestParseChroinf:
    def test_parses_single_channel(self, tmp_path):
        inf_file = tmp_path / "_chroms.inf"
        inf_file.write_bytes(_build_chroinf([("TUV 260", " AU")]))
        result = parse_chroinf(str(inf_file))
        assert len(result) == 1
        assert result[0][0] == "TUV 260"
        assert result[0][1].strip() == "AU"

    def test_parses_multiple_channels(self, tmp_path):
        inf_file = tmp_path / "_chroms.inf"
        inf_file.write_bytes(
            _build_chroinf([("TUV 260", " AU"), ("BSM System Pressure", " psi")])
        )
        result = parse_chroinf(str(inf_file))
        assert len(result) == 2
        assert result[0][0] == "TUV 260"
        assert result[1][0] == "BSM System Pressure"

    def test_empty_file(self, tmp_path):
        inf_file = tmp_path / "_chroms.inf"
        inf_file.write_bytes(b"\x00" * 0x84)
        result = parse_chroinf(str(inf_file))
        assert result == []


class TestParseChrodat:
    def test_parses_samples(self, tmp_path):
        dat_file = tmp_path / "_chro001.dat"
        samples = [(0.5, 100.0), (1.0, 200.0), (1.5, 150.0)]
        dat_file.write_bytes(_build_chrodat(samples))
        result = parse_chrodat(str(dat_file))
        assert result is not None
        times, intensities = result
        assert len(times) == 3
        assert abs(times[0] - 0.5) < 1e-5
        assert abs(intensities[1] - 200.0) < 1e-5

    def test_empty_data(self, tmp_path):
        dat_file = tmp_path / "_chro001.dat"
        dat_file.write_bytes(b"\x00" * 0x80)  # header only
        result = parse_chrodat(str(dat_file))
        assert result is None


class TestWriteChromCsv:
    def test_writes_csv(self, tmp_path):
        csv_file = tmp_path / "test.csv"
        times = [0.1, 0.2, 0.3]
        intensities = [10.0, 20.0, 30.0]
        write_chrom_csv(str(csv_file), times, intensities)

        with open(csv_file) as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        assert len(rows) == 3
        assert rows[0]["time"] == "0.100000"
        assert rows[2]["intensity"] == "30.000000"


class TestGetChromatogramInfo:
    def test_finds_chroms_inf(self, tmp_path):
        raw_dir = tmp_path / "test.raw"
        raw_dir.mkdir()
        inf_file = raw_dir / "_chroms.inf"
        inf_file.write_bytes(_build_chroinf([("TUV 260", " AU")]))
        result = get_chromatogram_info(str(raw_dir))
        assert len(result) == 1

    def test_returns_empty_when_missing(self, tmp_path):
        raw_dir = tmp_path / "test.raw"
        raw_dir.mkdir()
        result = get_chromatogram_info(str(raw_dir))
        assert result == []


class TestExportChromatograms:
    def test_exports_csv_files(self, tmp_path):
        raw_dir = tmp_path / "sample.raw"
        raw_dir.mkdir()

        # Create chroms.inf
        inf_file = raw_dir / "_chroms.inf"
        inf_file.write_bytes(_build_chroinf([("TUV 260", " AU")]))

        # Create chro data
        dat_file = raw_dir / "_chro001.dat"
        samples = [(0.5, 100.0), (1.0, 200.0)]
        dat_file.write_bytes(_build_chrodat(samples))

        chrom_info = get_chromatogram_info(str(raw_dir))
        exported = export_chromatograms(str(raw_dir), chrom_info)

        assert len(exported) == 1
        assert "TUV 260" in exported[0]
        assert os.path.exists(exported[0])

        with open(exported[0]) as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        assert len(rows) == 2
        # Times should be converted from minutes to seconds (0.5 * 60 = 30)
        assert abs(float(rows[0]["time"]) - 30.0) < 1e-4

    def test_skips_empty_dat_files(self, tmp_path):
        raw_dir = tmp_path / "sample.raw"
        raw_dir.mkdir()

        inf_file = raw_dir / "_chroms.inf"
        inf_file.write_bytes(_build_chroinf([("TUV 260", " AU")]))

        dat_file = raw_dir / "_chro001.dat"
        dat_file.write_bytes(b"\x00" * 0x80)

        chrom_info = get_chromatogram_info(str(raw_dir))
        exported = export_chromatograms(str(raw_dir), chrom_info)
        assert len(exported) == 0


def _build_mzml(spectra):
    """Build a minimal mzML XML string with given spectra.

    Each spectrum is a dict with keys: rt (minutes), tic.
    """
    lines = [
        '<?xml version="1.0" encoding="utf-8"?>',
        '<mzML xmlns="http://psi.hupo.org/ms/mzml">',
        "  <run>",
        f'    <spectrumList count="{len(spectra)}">',
    ]
    for i, s in enumerate(spectra):
        lines.append(f'      <spectrum index="{i}">')
        lines.append(f'        <cvParam accession="MS:1000285" value="{s["tic"]}"/>')
        lines.append("        <scanList>")
        lines.append("          <scan>")
        lines.append(
            f'            <cvParam accession="MS:1000016" value="{s["rt"]}"'
            f' unitName="minute"/>'
        )
        lines.append("          </scan>")
        lines.append("        </scanList>")
        lines.append("      </spectrum>")
    lines.append("    </spectrumList>")
    lines.append("  </run>")
    lines.append("</mzML>")
    return "\n".join(lines)


class TestExtractTicFromMzml:
    def test_extracts_tic(self, tmp_path):
        mzml_file = tmp_path / "test.mzML"
        mzml_file.write_text(
            _build_mzml(
                [
                    {"rt": 1.0, "tic": 1000.0},
                    {"rt": 2.0, "tic": 2000.0},
                    {"rt": 3.0, "tic": 1500.0},
                ]
            )
        )
        output = extract_tic_from_mzml(str(mzml_file))
        assert output == str(tmp_path / "test_TIC.csv")
        assert os.path.exists(output)

        with open(output) as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        assert len(rows) == 3
        # RT should be converted from minutes to seconds
        assert abs(float(rows[0]["time"]) - 60.0) < 1e-4
        assert abs(float(rows[1]["intensity"]) - 2000.0) < 1e-4

    def test_custom_output_path(self, tmp_path):
        mzml_file = tmp_path / "test.mzML"
        mzml_file.write_text(_build_mzml([{"rt": 1.0, "tic": 500.0}]))
        custom_out = str(tmp_path / "custom_tic.csv")
        output = extract_tic_from_mzml(str(mzml_file), output_csv=custom_out)
        assert output == custom_out
        assert os.path.exists(custom_out)

    def test_empty_mzml(self, tmp_path):
        mzml_file = tmp_path / "empty.mzML"
        mzml_file.write_text(_build_mzml([]))
        output = extract_tic_from_mzml(str(mzml_file))
        assert os.path.exists(output)
        with open(output) as f:
            reader = csv.DictReader(f)
            rows = list(reader)
        assert len(rows) == 0
