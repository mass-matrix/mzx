# mm_convert

## Installation

mmconvert is available on PyPi

```
pip install mmconvert
```

## Usage

```
mmconvert --type mgf /path/to/data.mgf
```

This will convert the `mgf` data to `mzml` by default.

## Developer Setup

If you have uv installed, then you can a venv setup by running:

```
make setup
```

While making changes to mmconvert, you can install/uninstall it from your local

```
make install
```

```
make uinstall
```

## Tests

```
make test
```
