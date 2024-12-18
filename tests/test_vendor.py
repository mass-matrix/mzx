from src.mzx import get_vendor
import pytest
import os

vendor_test_data = [
    ("tests/example_data/bruker_empty.d", "bruker"),
    ("tests/example_data/thermo_empty.raw", "thermo"),
    ("tests/example_data/waters_empty.raw", "waters"),
    ("tests/example_data/agilent.d", "agilent"),
]


@pytest.mark.parametrize("datafile,expected", vendor_test_data)
def test_sequence_to_tokens_V0(datafile, expected):
    print(os.getcwd())
    vendor = get_vendor(datafile)

    # print(test_out)
    assert vendor == expected
    # assert sequences == expected
