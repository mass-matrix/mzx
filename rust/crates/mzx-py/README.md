# mzx-rs

The pure-Rust conversion backend of [mzx](https://github.com/mass-matrix/mzx).
It converts Thermo RAW files to mzML without Docker, and only when its output
is proven byte-identical to the pinned ProteoWizard `msconvert` (after
normalizing the converter's name, version and command line and the input file
location). Everything else raises `mzx_rs.Unsupported`, and mzx falls back to
msconvert. The contract and what is proven are in
[`rust/CONTRACT.md`](https://github.com/mass-matrix/mzx/blob/main/rust/CONTRACT.md)
and [`rust/support_matrix.toml`](https://github.com/mass-matrix/mzx/blob/main/rust/support_matrix.toml).

Install it through mzx:

```console
pip install "mzx[rust]"
```

There are no prebuilt wheels yet, so pip compiles it from source. That needs
a C compiler (gcc or clang on Linux, the Xcode command line tools on macOS,
Visual Studio Build Tools on Windows). The Rust toolchain and CMake are
fetched automatically when they are missing; the first build takes a few
minutes.

```python
import mzx_rs

mzx_rs.convert("run.raw", "run.mzML", "default", "mzx-rs run.raw --options default")
```

`mzx_rs.OPTION_SETS` lists the option sets (`default`, `nofilter`, `pickall`,
`pickall_rz`); their msconvert arguments are in `rust/CONTRACT.md`.
