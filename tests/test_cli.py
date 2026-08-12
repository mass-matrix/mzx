"""Tests for mzx.cli main()."""

import sys
from unittest import mock

import pytest

from mzx.cli import main


def test_cli_calls_convert_file_with_parsed_args(monkeypatch) -> None:
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
    with mock.patch("mzx.cli.convert_file", return_value="/out/file.mgf") as mock_conv:
        main()
    mock_conv.assert_called_once()
    params, native = mock_conv.call_args[0][0], mock_conv.call_args[1]["native"]
    assert native is False
    assert params["infile"] == "/data/file.raw"
    assert params["type"] == "mgf"
    assert params["index"] is True
    assert params["peak_picking"] == "all"


def test_cli_default_still_uses_docker_path(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "/data/file.raw"])
    with mock.patch("mzx.cli.convert_file", return_value="/out/file.mzML") as mock_conv:
        main()
    _, kwargs = mock_conv.call_args
    assert kwargs["native"] is False


def test_cli_logs_exception_on_conversion_failure(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "/nope.raw"])
    with mock.patch("mzx.cli.convert_file", side_effect=RuntimeError("boom")):
        with mock.patch("mzx.cli.logger") as log:
            main()
    log.error.assert_called()


def test_cli_passes_vendor_from_vendor_name_from_file(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "/x.raw"])
    with mock.patch("mzx.cli.convert_file", return_value="/out.mzML") as mock_conv:
        with mock.patch("mzx.cli.vendor.vendor_name_from_file", return_value="Thermo"):
            main()
    params = mock_conv.call_args[0][0]
    assert params["vendor"] == "Thermo"


def test_cli_native_routes_to_convert_file(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "--native", "/data/file.raw"])
    with mock.patch("mzx.cli.convert_file", return_value="/out/file.mzML") as mock_conv:
        with mock.patch("mzx.cli.detect_native_vendor", return_value="Thermo"):
            with mock.patch("mzx.cli.logger") as log:
                main()
    _, kwargs = mock_conv.call_args
    assert kwargs["native"] is True
    log.warning.assert_called_once()
    assert "Experimental native conversion" in log.warning.call_args[0][0]


def test_cli_native_logs_experimental_warning(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "--native", "/data/file.raw"])
    with mock.patch("mzx.cli.convert_file", return_value="/out/file.mzML"):
        with mock.patch("mzx.cli.detect_native_vendor", return_value="Thermo"):
            with mock.patch("mzx.cli.logger") as log:
                main()
    log.warning.assert_called_once()
    assert "Experimental native conversion" in log.warning.call_args[0][0]


def test_cli_native_rejects_non_mzml_type(monkeypatch) -> None:
    monkeypatch.setattr(
        sys,
        "argv",
        ["mzx", "--native", "--type", "mgf", "/data/file.raw"],
    )
    with mock.patch("mzx.cli.convert_file") as mock_conv:
        with mock.patch("mzx.cli.logger") as log:
            with pytest.raises(SystemExit) as exc:
                main()
    assert exc.value.code == 1
    mock_conv.assert_not_called()
    log.error.assert_called()


def test_cli_passes_output_and_vendor_overrides(monkeypatch) -> None:
    monkeypatch.setattr(
        sys,
        "argv",
        [
            "mzx",
            "--native",
            "--vendor",
            "waters",
            "--output",
            "/tmp/custom.mzML",
            "/data/sample.raw",
        ],
    )
    with mock.patch(
        "mzx.cli.convert_file", return_value="/tmp/custom.mzML"
    ) as mock_conv:
        with mock.patch("mzx.cli.logger"):
            main()
    params = mock_conv.call_args[0][0]
    assert params["vendor"] == "waters"
    assert params["outfile"] == "/tmp/custom.mzML"


def test_cli_native_exits_on_conversion_failure(monkeypatch) -> None:
    monkeypatch.setattr(sys, "argv", ["mzx", "--native", "/nope.raw"])
    with mock.patch(
        "mzx.cli.convert_file",
        side_effect=RuntimeError("native failed"),
    ):
        with mock.patch("mzx.cli.detect_native_vendor", return_value="Thermo"):
            with mock.patch("mzx.cli.logger"):
                with pytest.raises(SystemExit) as exc:
                    main()
    assert exc.value.code == 1
