from os import PathLike

__version__: str
OPTION_SETS: tuple[str, ...]

class Unsupported(Exception): ...
class ConversionError(Exception): ...

def convert(
    infile: str | PathLike[str],
    outfile: str | PathLike[str],
    option_set: str,
    command_line: str,
) -> None: ...
