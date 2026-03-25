"""Tests for pure helpers in mzx.__init__."""

import pytest

from mzx import exclusion_string, format_function_number, modify_waters_scan_header


@pytest.mark.parametrize(
    "x, expected",
    [
        (1, "2-"),
        (2, "1 3-"),
        (3, "1-2 4-"),
        (5, "1-4 6-"),
        (10, "1-9 11-"),
    ],
)
def test_exclusion_string_examples(x: int, expected: str) -> None:
    assert exclusion_string(x) == expected


@pytest.mark.parametrize("bad", [0, -1, -100])
def test_exclusion_string_rejects_non_positive(bad: int) -> None:
    with pytest.raises(ValueError, match="positive integer"):
        exclusion_string(bad)


def test_format_function_number_match() -> None:
    s, n = format_function_number("foo REFERENCE Function 7 bar")
    assert s == "_FUNC007"
    assert n == 7


def test_format_function_number_no_match() -> None:
    assert format_function_number("no function here") is None


def test_modify_waters_scan_header_no_match_returns_line() -> None:
    line = "<spectrum index='0'/>"
    assert modify_waters_scan_header(line) is line


def test_modify_waters_scan_header_preserves_unmatched_spectrum_line() -> None:
    line = '  <spectrum index="99" id="other">'
    assert modify_waters_scan_header(line) == line
