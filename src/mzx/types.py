from typing import Literal, Optional, TypedDict, Union

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
    peak_picking: Literal["off", "all", "msms", "ms1"]
    remove_zeros: bool
    vendor: TVendor
    outfile: str | None
    type: Literal["mzml"]
    overwrite: bool
    debug: bool
    verbose: bool
    lockmass_disabled: Optional[bool]
    lockmass: Optional[bool]
    neg_lockmass: Optional[float]
    pos_lockmass: Optional[float]
    lockmass_tolerance: Optional[float]
    lockmass_function_exclude: Optional[int]
