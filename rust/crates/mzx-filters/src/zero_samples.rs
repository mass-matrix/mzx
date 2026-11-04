//! Port of `SpectrumListZeroSamplesFilter.GetSpectrum`, mode `RemoveExtra`
//! (pwiz/analysis/SpectrumListZeroSamplesFilter.cs @ fb75aff).

use mzx_mzml::model::{ArrayData, Spectrum};

use crate::{has_cv, ms_level, IntegerSet, MS_CENTROID_SPECTRUM, MS_INTENSITY_ARRAY, MS_MZ_ARRAY};

/// `Spectrum.GetMZArray` / `GetIntensityArray`: the first (float)
/// BinaryDataArray of that type. Integer arrays live in a separate list in pwiz.
fn float_array(s: &Spectrum, kind: &str) -> Option<usize> {
    s.arrays
        .iter()
        .position(|a| matches!(a.data, ArrayData::Float(_)) && a.kind() == Some(kind))
}

fn floats(s: &Spectrum, i: usize) -> &[f64] {
    match &s.arrays[i].data {
        ArrayData::Float(v) => v,
        ArrayData::Int(_) => unreachable!("checked by float_array"),
    }
}

/// Drop zero-intensity samples. Profile spectra (no `centroid spectrum` term)
/// keep one zero on each side of every non-zero run; arrays of length <= 3 are
/// left as they are.
pub fn remove_extra(s: &mut Spectrum, ms_levels: &IntegerSet) {
    if !ms_levels.contains(ms_level(s)) {
        return;
    }
    let (Some(mz_i), Some(int_i)) = (
        float_array(s, MS_MZ_ARRAY),
        float_array(s, MS_INTENSITY_ARRAY),
    ) else {
        return;
    };
    let preserve_flanking_zeros = !has_cv(s, MS_CENTROID_SPECTRUM);
    let (mz, y) = (floats(s, mz_i), floats(s, int_i));
    let n = mz.len().min(y.len());
    let mut new_mz = Vec::with_capacity(n);
    let mut new_y = Vec::with_capacity(n);

    if !preserve_flanking_zeros {
        for i in 0..n {
            if y[i] != 0.0 {
                new_mz.push(mz[i]);
                new_y.push(y[i]);
            }
        }
    } else if n <= 3 {
        new_mz.extend_from_slice(&mz[..n]);
        new_y.extend_from_slice(&y[..n]);
    } else {
        let last = n - 1;
        for i in 0..last {
            if y[i] != 0.0 || y[i + 1] != 0.0 || (i > 0 && y[i - 1] != 0.0) {
                new_mz.push(mz[i]);
                new_y.push(y[i]);
            }
        }
        if y[last] != 0.0 || y[last - 1] != 0.0 {
            new_mz.push(mz[last]);
            new_y.push(y[last]);
        }
    }

    // Spectrum.SetMZIntensityArrays: replace contents, set defaultArrayLength.
    s.default_array_length = new_mz.len();
    s.arrays[mz_i].data = ArrayData::Float(new_mz);
    s.arrays[int_i].data = ArrayData::Float(new_y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzx_mzml::model::{BinaryDataArray, CvParam, Params};

    fn spectrum(centroid: bool, y: &[f64]) -> Spectrum {
        let arr = |kind: &str, v: Vec<f64>| BinaryDataArray {
            params: Params {
                cv: vec![CvParam::new(kind, "")],
                ..Default::default()
            },
            data_processing_ref: None,
            data: ArrayData::Float(v),
        };
        let mut s = Spectrum {
            default_array_length: y.len(),
            arrays: vec![
                arr(MS_MZ_ARRAY, (0..y.len()).map(|i| i as f64).collect()),
                arr(MS_INTENSITY_ARRAY, y.to_vec()),
            ],
            ..Default::default()
        };
        s.params.cv.push(CvParam::new(crate::MS_LEVEL, "1"));
        s.params.cv.push(CvParam::new(
            if centroid {
                MS_CENTROID_SPECTRUM
            } else {
                crate::MS_PROFILE_SPECTRUM
            },
            "",
        ));
        s
    }

    fn run(centroid: bool, y: &[f64]) -> (Vec<f64>, usize) {
        let mut s = spectrum(centroid, y);
        remove_extra(&mut s, &IntegerSet::positive());
        match &s.arrays[0].data {
            ArrayData::Float(mz) => (mz.clone(), s.default_array_length),
            _ => unreachable!(),
        }
    }

    #[test]
    fn profile_keeps_one_flanking_zero() {
        // indices:        0    1    2    3    4    5    6    7
        let y = [0.0, 0.0, 0.0, 5.0, 0.0, 0.0, 0.0, 0.0];
        assert_eq!(run(false, &y), (vec![2.0, 3.0, 4.0], 3));
        // first sample has no left neighbour; last only looks left
        let y = [1.0, 0.0, 0.0, 0.0, 0.0, 2.0];
        assert_eq!(run(false, &y).0, vec![0.0, 1.0, 4.0, 5.0]);
    }

    #[test]
    fn profile_short_arrays_untouched() {
        assert_eq!(run(false, &[0.0, 0.0, 0.0]).0, vec![0.0, 1.0, 2.0]);
    }

    #[test]
    fn centroid_drops_every_zero() {
        assert_eq!(run(true, &[0.0, 3.0, 0.0, 0.0, 4.0, 0.0]).0, vec![1.0, 4.0]);
    }

    #[test]
    fn ms_level_gate() {
        let mut s = spectrum(true, &[0.0, 1.0]);
        remove_extra(&mut s, &IntegerSet::parse("2-").unwrap());
        assert_eq!(s.default_array_length, 2);
    }
}
