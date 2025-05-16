import pytest
from unittest import mock

# Assuming your function is in a module named `vendor_utils.py`
from src.mzx import vendor


@pytest.mark.parametrize(
    "filename, expected_vendor",
    [
        ("test.raw", "Thermo"),
        ("test.d", "Agilent"),
        ("unknown.txt", "unspecified"),
    ],
)
def test_vendor_from_file_extensions(filename, expected_vendor):
    with mock.patch("os.path.isdir", return_value=False):
        assert vendor.vendor_name_from_file(filename) == expected_vendor


def test_directory_with_dot_d(tmp_path):
    path = tmp_path / "sample.d"
    path.mkdir()
    with mock.patch("os.path.isdir", return_value=True):
        assert vendor.vendor_name_from_file(str(path)) == "bruker"


def test_directory_with_dot_raw(tmp_path):
    path = tmp_path / "sample.raw"
    path.mkdir()
    with mock.patch("os.path.isdir", return_value=True):
        assert vendor.vendor_name_from_file(str(path)) == "waters"


def test_directory_with_func_file(tmp_path):
    path = tmp_path
    (path / "some_FUNC_file.txt").write_text("data")
    with mock.patch("os.path.isdir", return_value=True):
        assert vendor.vendor_name_from_file(str(path)) == "waters"


def test_directory_with_no_vendor_hint(tmp_path):
    path = tmp_path
    (path / "file1.txt").write_text("abc")
    (path / "file2.csv").write_text("def")
    with mock.patch("os.path.isdir", return_value=True):
        assert vendor.vendor_name_from_file(str(path)) == "unspecified"
