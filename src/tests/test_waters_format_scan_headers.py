import sys
import os

sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), "..")))
from src.mm_convert import *


def test_modify_waters_scan_header():

    input_str = '        <spectrum index="0" id="function=1 process=0 scan=1" defaultArrayLength="3510" dataProcessingRef="pwiz_Reader_Waters_conversion">'
    expected_output = '        <spectrum index="0" id="function=1 process=0 scan=1 fscan=1" defaultArrayLength="3510" dataProcessingRef="pwiz_Reader_Waters_conversion">'

    output_string = modify_waters_scan_header(input_str)

    assert output_string == expected_output
