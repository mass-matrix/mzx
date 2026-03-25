"""Tests for mzx.cli main()."""

import sys
from unittest import mock

from mzx.cli import main


def test_cli_calls_convert_raw_file_with_parsed_args(monkeypatch) -> None:
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "mzx",
            "/data/file.raw",
            "--type",
            "mgf",
            "--index",
            "--peak_picking",
            "all",
        ],
    )
    with mock.patch("mzx.cli.convert_raw_file") as mock_conv:
        main()
    mock_conv.assert_called_once()
    params = mock_conv.call_args[0][0]
    assert params["infile"] == "/data/file.raw"
    assert params["type"] == "mgf"
    assert params["index"] is True
    assert params["peak_picking"] == "all"


def test_cli_logs_exception_on_conversion_failure(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "/nope.raw"])
    with mock.patch("mzx.cli.convert_raw_file", side_effect=RuntimeError("boom")):
        with mock.patch("mzx.cli.logger") as log:
            main()
    log.error.assert_called()


def test_cli_passes_vendor_from_vendor_name_from_file(monkeypatch) -> None:
    """CLI does not use --vendor today; vendor is inferred from the path."""
    monkeypatch.setattr(sys, "argv", ["mzx", "/x.raw"])
    with mock.patch("mzx.cli.convert_raw_file") as mock_conv:
        with mock.patch("mzx.cli.vendor.vendor_name_from_file", return_value="Thermo"):
            main()
    params = mock_conv.call_args[0][0]
    assert params["vendor"] == "Thermo"
