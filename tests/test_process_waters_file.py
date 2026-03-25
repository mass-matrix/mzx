"""Tests for in-place Waters mzML scan header rewriting."""

from pathlib import Path

from mzx import process_waters_scan_headers


def test_process_waters_scan_headers_rewrites_file(tmp_path: Path) -> None:
    p = tmp_path / "small.mzML"
    lines = [
        '        <spectrum index="0" id="function=1 process=0 scan=1" />\n',
        "        <other />\n",
    ]
    p.write_text("".join(lines), encoding="utf8")
    process_waters_scan_headers(str(p))
    out = p.read_text(encoding="utf8")
    assert "fscan=1" in out
    assert "scan=1 fscan=1" in out
