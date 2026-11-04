//! Ports of the msconvert spectrum filters in mzx's `default` option set
//! (`--filter "peakPicking true 2-" --filter "zeroSamples removeExtra"`),
//! from pwiz 4.x (`pwiz/analysis` @ fb75aff; see `rust/CONTRACT.md`).
//!
//! Filters run in command-line order, each wrapping the previous one, exactly as
//! `SpectrumListFactory.Wrap` does. Anything the port cannot reproduce exactly
//! returns [`Unsupported`] so mzx can fall back to Docker.

mod integer_set;
pub mod peak_picking;
pub mod zero_samples;

pub use integer_set::IntegerSet;

use mzx_mzml::model::Spectrum;

pub const MS_LEVEL: &str = "MS:1000511";
pub const MS_MZ_ARRAY: &str = "MS:1000514";
pub const MS_INTENSITY_ARRAY: &str = "MS:1000515";
pub const MS_CENTROID_SPECTRUM: &str = "MS:1000127";
pub const MS_PROFILE_SPECTRUM: &str = "MS:1000128";

/// The input needs behaviour this port has not proven (fail closed).
#[derive(Debug)]
pub struct Unsupported(pub String);

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported: {}", self.0)
    }
}

impl std::error::Error for Unsupported {}

/// `spectrum.Params.CvParamValueOrDefault(CVID.MS_ms_level, 0)`.
pub fn ms_level(s: &Spectrum) -> i64 {
    s.params
        .cv
        .iter()
        .find(|p| p.accession == MS_LEVEL)
        .and_then(|p| p.value.trim().parse().ok())
        .unwrap_or(0)
}

/// `ParamContainer.HasCVParam` (direct params only, as the C# filters use it).
pub fn has_cv(s: &Spectrum, accession: &str) -> bool {
    s.params.cv.iter().any(|p| p.accession == accession)
}

/// One filter of mzx's `default` option set.
#[derive(Debug, Clone)]
pub enum Filter {
    /// `peakPicking true <msLevels>`: prefer vendor centroiding.
    PeakPickingVendor {
        ms_levels: IntegerSet,
        /// `IVendorCentroidingSpectrumList.VendorCentroidName` of the source reader.
        vendor_name: String,
    },
    /// `zeroSamples removeExtra [msLevels]`.
    ZeroSamplesRemoveExtra { ms_levels: IntegerSet },
}

/// mzx's `default` option set for a Thermo source.
pub fn mzx_default_thermo() -> Vec<Filter> {
    vec![
        Filter::PeakPickingVendor {
            ms_levels: IntegerSet::parse("2-").expect("static"),
            vendor_name: peak_picking::THERMO_VENDOR_CENTROID_NAME.to_string(),
        },
        Filter::ZeroSamplesRemoveExtra {
            ms_levels: IntegerSet::positive(),
        },
    ]
}

/// Apply `filters` in order to every spectrum, plus the list-level side effects
/// (DataProcessing methods) the corresponding pwiz wrappers have.
pub fn apply(msd: &mut mzx_mzml::MsData, filters: &[Filter]) -> anyhow::Result<()> {
    for f in filters {
        match f {
            Filter::PeakPickingVendor {
                ms_levels,
                vendor_name,
            } => peak_picking::apply_vendor_preferred(msd, ms_levels, vendor_name)?,
            Filter::ZeroSamplesRemoveExtra { ms_levels } => {
                for s in &mut msd.run.spectra {
                    zero_samples::remove_extra(s, ms_levels);
                }
            }
        }
    }
    Ok(())
}

/// MS levels for which the source reader must prefer vendor centroiding
/// (`SpectrumList_PeakPicker` asks the vendor list for centroided spectra at
/// these levels), or `None` when no vendor peak picking is requested.
pub fn vendor_centroid_levels(filters: &[Filter]) -> Option<IntegerSet> {
    filters.iter().find_map(|f| match f {
        Filter::PeakPickingVendor { ms_levels, .. } => Some(ms_levels.clone()),
        _ => None,
    })
}
