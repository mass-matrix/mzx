//! Port of `SpectrumList_PeakPicker` with `preferVendorPeakPicking = true`
//! (pwiz/analysis/SpectrumList_PeakPicker.cs @ fb75aff), restricted to what is
//! provable without the vendor library:
//!
//! - spectra outside the MS-level set pass through;
//! - spectra already marked `centroid spectrum` lose any `profile spectrum`
//!   term and are otherwise untouched (no dataProcessingRef is set);
//! - anything that would need vendor centroiding is [`Unsupported`].
//!
//! Caveat that is *not* a filter concern: with vendor preference the Thermo
//! reader is asked for spectra with `preferCentroid`, which also changes how it
//! computes MS2 precursor `peak intensity` (summed from the vendor-centroided
//! precursor scan). That belongs to the reader port; see `rust/CONTRACT.md`.

use anyhow::Result;
use mzx_mzml::model::{CvParam, ProcessingMethod, UserParam};
use mzx_mzml::MsData;

use crate::{has_cv, ms_level, IntegerSet, Unsupported, MS_CENTROID_SPECTRUM, MS_PROFILE_SPECTRUM};

/// `SpectrumList_Thermo.VendorCentroidName`.
pub const THERMO_VENDOR_CENTROID_NAME: &str = "Thermo/Xcalibur peak picking";

const MS_PEAK_PICKING: &str = "MS:1000035";

pub fn apply_vendor_preferred(
    msd: &mut MsData,
    ms_levels: &IntegerSet,
    vendor_name: &str,
) -> Result<()> {
    for s in &mut msd.run.spectra {
        if !ms_levels.contains(ms_level(s)) {
            continue;
        }
        if has_cv(s, MS_CENTROID_SPECTRUM) {
            // RemoveCv: first occurrence only
            if let Some(i) = s
                .params
                .cv
                .iter()
                .position(|p| p.accession == MS_PROFILE_SPECTRUM)
            {
                s.params.cv.remove(i);
            }
            continue;
        }
        return Err(Unsupported(format!(
            "spectrum {:?} needs vendor centroiding ({vendor_name})",
            s.id
        ))
        .into());
    }

    // BuildDataProcessing: a copy of the list's DataProcessing (same id) plus a
    // "peak picking" method; SpectrumListFactory.Wrap(msd, ...) replaces the
    // record in msd.DataProcessings.
    let dp_id = msd
        .run
        .spectrum_default_data_processing_ref
        .clone()
        .unwrap_or_else(|| "pwiz_Reader_conversion".to_string());
    let Some(dp) = msd.data_processing.iter_mut().find(|d| d.id == dp_id) else {
        return Err(Unsupported(format!("no dataProcessing {dp_id:?} to extend")).into());
    };
    let software_ref = dp.methods.first().and_then(|m| m.software_ref.clone());
    let mut method = ProcessingMethod {
        order: dp.methods.len() as i32,
        software_ref,
        ..Default::default()
    };
    method.params.cv.push(CvParam::new(MS_PEAK_PICKING, ""));
    method.params.user.push(UserParam {
        name: vendor_name.to_string(),
        ..Default::default()
    });
    dp.methods.push(method);
    Ok(())
}
