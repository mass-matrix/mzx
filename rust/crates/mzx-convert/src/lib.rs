//! The conversion pipeline mzx ships: `read → filters → brand → write`.
//!
//! The reader proof (`mzx-verify/tests/thermo_reader.rs`) and `mzx-verify
//! convert` call [`convert_thermo`], so the code that is proven against the
//! msconvert goldens is the code the Python package runs.
//!
//! Anything outside proven coverage fails with an error for which
//! [`is_unsupported`] is true; callers fall back to msconvert.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{Context, Result};
use mzx_mzml::model::{CvParam, MsData, Params, Software, UserParam};

/// Version recorded in the converter `<software>` block of mzx output.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Id of mzx's converter `<software>` block (replaces pwiz's `pwiz`).
pub const SOFTWARE_ID: &str = "mzx";

/// userParam name marking mzx's converter block; the contract normalization
/// recognizes it (`mzx_verify::MZX_CONVERTER_USERPARAM`).
pub const CONVERTER_USERPARAM: &str = "mzx converter";

/// The reader emits pwiz's converter block under this id; [`brand`] replaces it.
const PWIZ_SOFTWARE_ID: &str = "pwiz";

/// A contract option set (`CONTRACT.md`, "Inputs").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionSet {
    /// `--mzML --noindex --filter "peakPicking true 2-" --filter "zeroSamples removeExtra"`
    Default,
    /// `--mzML --noindex`
    NoFilter,
}

impl OptionSet {
    pub fn name(self) -> &'static str {
        match self {
            OptionSet::Default => "default",
            OptionSet::NoFilter => "nofilter",
        }
    }

    /// The msconvert arguments this option set stands for.
    pub fn msconvert_args(self) -> &'static str {
        match self {
            OptionSet::Default => {
                r#"--mzML --noindex --filter "peakPicking true 2-" --filter "zeroSamples removeExtra""#
            }
            OptionSet::NoFilter => "--mzML --noindex",
        }
    }

    pub fn thermo_filters(self) -> Vec<mzx_filters::Filter> {
        match self {
            OptionSet::Default => mzx_filters::mzx_default_thermo(),
            OptionSet::NoFilter => vec![],
        }
    }
}

impl FromStr for OptionSet {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "default" => Ok(OptionSet::Default),
            "nofilter" => Ok(OptionSet::NoFilter),
            o => Err(Unsupported(format!("option set {o:?}")).into()),
        }
    }
}

/// An option set or input outside the proven coverage (not a Thermo- or
/// filter-specific refusal; see [`is_unsupported`]).
#[derive(Debug)]
pub struct Unsupported(pub String);

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported: {}", self.0)
    }
}

impl std::error::Error for Unsupported {}

/// True when `e` is a fail-closed refusal (from the reader, the filters or
/// this crate) rather than an I/O or data error.
pub fn is_unsupported(e: &anyhow::Error) -> bool {
    e.chain().any(|c| {
        c.is::<Unsupported>()
            || c.is::<mzx_thermo::Unsupported>()
            || c.is::<mzx_filters::Unsupported>()
    })
}

/// Thermo reader options matching a filter chain: vendor peak picking makes
/// pwiz read those MS levels with `preferCentroid` (which also changes the
/// precursor peak intensities summed from the precursor scan).
pub fn thermo_reader_options(filters: &[mzx_filters::Filter]) -> mzx_thermo::reader::Options {
    mzx_thermo::reader::Options {
        centroid_ms_levels: mzx_filters::vendor_centroid_levels(filters).map(|s| {
            Box::new(move |level: u32| s.contains(level as i64)) as Box<dyn Fn(u32) -> bool>
        }),
    }
}

/// Read a Thermo RAW file and apply an option set's filters. `command_line`
/// goes into cvParam MS:1001885, where msconvert records its argv.
pub fn convert_thermo(raw: &Path, options: OptionSet, command_line: &str) -> Result<MsData> {
    let filters = options.thermo_filters();
    let mut msd = mzx_thermo::reader::read(raw, &thermo_reader_options(&filters))?;
    let method = msd
        .data_processing
        .first_mut()
        .and_then(|d| d.methods.first_mut())
        .context("reader produced no processing method")?;
    method
        .params
        .cv
        .push(CvParam::new("MS:1001885", command_line));
    mzx_filters::apply(&mut msd, &filters)?;
    brand(&mut msd)?;
    Ok(msd)
}

/// Replace the reader's ProteoWizard converter `<software>` block with mzx's
/// own and repoint every reference to it. The contract normalization treats
/// both blocks as the converter (`CONTRACT.md`, "Equality" 1), so this keeps
/// the output provably identical without claiming to be ProteoWizard.
pub fn brand(msd: &mut MsData) -> Result<()> {
    let sw = msd
        .software
        .iter_mut()
        .find(|s| s.id == PWIZ_SOFTWARE_ID)
        .context("no pwiz converter software block to replace")?;
    *sw = Software {
        id: SOFTWARE_ID.into(),
        version: VERSION.into(),
        params: Params {
            user: vec![UserParam {
                name: CONVERTER_USERPARAM.into(),
                value: format!("mzx-rs {VERSION}"),
                type_: "xsd:string".into(),
                unit: None,
            }],
            ..Default::default()
        },
    };
    let repoint = |r: &mut Option<String>| {
        if r.as_deref() == Some(PWIZ_SOFTWARE_ID) {
            *r = Some(SOFTWARE_ID.into());
        }
    };
    for dp in &mut msd.data_processing {
        dp.methods
            .iter_mut()
            .for_each(|m| repoint(&mut m.software_ref));
    }
    for ic in &mut msd.instrument_configurations {
        repoint(&mut ic.software_ref);
    }
    Ok(())
}

/// Serialize with msconvert 4.x's default encoding.
pub fn write<O: Write>(out: O, msd: &MsData) -> Result<O> {
    let cfg = mzx_mzml::EncoderConfig::msconvert_default(mzx_mzml::Dialect::Pwiz4Net);
    mzx_mzml::write_mzml(out, msd, &cfg)
}

/// Write `msd` to `out` atomically: a sibling temporary file renamed into
/// place, so a failed conversion never leaves a partial mzML behind.
pub fn write_file(msd: &MsData, out: &Path) -> Result<()> {
    let tmp = tmp_path(out);
    let result = (|| -> Result<()> {
        let f =
            std::fs::File::create(&tmp).with_context(|| format!("creating {}", tmp.display()))?;
        let mut w = write(std::io::BufWriter::new(f), msd)?;
        w.flush()?;
        w.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        std::fs::rename(&tmp, out).with_context(|| format!("renaming to {}", out.display()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result
}

fn tmp_path(out: &Path) -> PathBuf {
    let mut name = out.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".mzx-{}.tmp", std::process::id()));
    out.with_file_name(name)
}

/// Convert `raw` to an mzML file at `out` (the full shipped path).
pub fn convert_thermo_file(
    raw: &Path,
    out: &Path,
    options: OptionSet,
    command_line: &str,
) -> Result<()> {
    let msd = convert_thermo(raw, options, command_line)?;
    write_file(&msd, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzx_mzml::model::{DataProcessing, ProcessingMethod};

    #[test]
    fn option_sets_parse_and_refuse() {
        assert_eq!("default".parse::<OptionSet>().unwrap(), OptionSet::Default);
        assert_eq!(
            "nofilter".parse::<OptionSet>().unwrap(),
            OptionSet::NoFilter
        );
        let e = "mgf".parse::<OptionSet>().unwrap_err();
        assert!(is_unsupported(&e));
        assert!(!is_unsupported(&anyhow::anyhow!("disk full")));
        let wrapped = anyhow::Error::from(mzx_thermo::Unsupported("x".into())).context("reading");
        assert!(is_unsupported(&wrapped));
    }

    #[test]
    fn brand_replaces_converter_and_refs() {
        let mut msd = MsData::default();
        msd.software.push(Software {
            id: "Xcalibur".into(),
            ..Default::default()
        });
        msd.software.push(Software {
            id: "pwiz".into(),
            version: "4.0".into(),
            params: Params {
                cv: vec![CvParam::new("MS:1000615", "")],
                ..Default::default()
            },
        });
        msd.data_processing.push(DataProcessing {
            id: "dp".into(),
            methods: vec![
                ProcessingMethod {
                    software_ref: Some("pwiz".into()),
                    ..Default::default()
                },
                ProcessingMethod {
                    software_ref: Some("Xcalibur".into()),
                    ..Default::default()
                },
            ],
        });
        brand(&mut msd).unwrap();
        assert_eq!(msd.software[0].id, "Xcalibur");
        assert_eq!(msd.software[1].id, SOFTWARE_ID);
        assert!(msd.software[1].params.cv.is_empty());
        assert_eq!(msd.software[1].params.user[0].name, "mzx converter");
        let refs: Vec<_> = msd.data_processing[0]
            .methods
            .iter()
            .map(|m| m.software_ref.as_deref().unwrap())
            .collect();
        assert_eq!(refs, ["mzx", "Xcalibur"]);
        assert!(brand(&mut msd).is_err(), "no pwiz block left to replace");
    }
}
