//! `mzx_rs._mzx_rs`: the Python face of `mzx_convert`.
//!
//! `convert` either writes output that the corpus proof covers or raises
//! `Unsupported` (mzx falls back to msconvert). Any other failure, including
//! a Rust panic, raises `ConversionError`; nothing is ever left at `outfile`.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;

create_exception!(
    _mzx_rs,
    Unsupported,
    PyException,
    "Input or options outside the proven coverage; convert with msconvert instead."
);
create_exception!(
    _mzx_rs,
    ConversionError,
    PyException,
    "The Rust backend failed on a supported input."
);

/// Convert a Thermo RAW file to mzML at `outfile` with a contract option set
/// (`"default"` or `"nofilter"`). `command_line` is recorded in cvParam
/// MS:1001885 where msconvert records its argv.
#[pyfunction]
fn convert(
    py: Python<'_>,
    infile: PathBuf,
    outfile: PathBuf,
    option_set: &str,
    command_line: &str,
) -> PyResult<()> {
    let options = option_set.parse().map_err(|e| to_py(&e))?;
    let result = py.allow_threads(|| {
        catch_unwind(AssertUnwindSafe(|| {
            mzx_convert::convert_thermo_file(&infile, &outfile, options, command_line)
        }))
    });
    match result {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(to_py(&e)),
        Err(panic) => {
            let msg = panic
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| panic.downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "unknown panic".into());
            Err(ConversionError::new_err(format!("mzx-rs panicked: {msg}")))
        }
    }
}

fn to_py(e: &anyhow::Error) -> PyErr {
    let msg = format!("{e:#}");
    if mzx_convert::is_unsupported(e) {
        Unsupported::new_err(msg)
    } else {
        ConversionError::new_err(msg)
    }
}

#[pymodule]
fn _mzx_rs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", mzx_convert::VERSION)?;
    m.add("OPTION_SETS", ("default", "nofilter"))?;
    m.add("Unsupported", m.py().get_type::<Unsupported>())?;
    m.add("ConversionError", m.py().get_type::<ConversionError>())?;
    m.add_function(wrap_pyfunction!(convert, m)?)?;
    Ok(())
}
