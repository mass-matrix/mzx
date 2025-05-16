from typing import Literal, TypedDict, Union

# Vendors
TBruker = Literal["bruker"]
TWaters = Literal["waters"]
TThermo = Literal["Thermo"]
TAgilent = Literal["Agilent"]
TUnspecified = Literal["unspecified"]

TVendor = Union[TAgilent, TBruker, TThermo, TWaters, TUnspecified]


class TConfig(TypedDict):
    infile: str
    index: bool
    sortbyscan: bool
    peak_picking: Literal["all", "off"]
    remove_zeros: bool
    vendor: TVendor
    outfile: str | None
    type: Literal["mzml"]
    overwrite: bool
    debug: bool
    verbose: bool
