//! Filter proof against the golden corpus (skipped without `MZX_CORPUS`):
//! `write(filters(read(<raw>.nofilter.mzML))) == <raw>.default.mzML`.

use std::path::PathBuf;

use mzx_filters::{mzx_default_thermo, Filter};

fn goldens() -> Option<Vec<(PathBuf, PathBuf)>> {
    let dir = PathBuf::from(std::env::var_os("MZX_CORPUS")?).join("goldens");
    let mut pairs = Vec::new();
    for e in std::fs::read_dir(&dir).ok()? {
        let p = e.ok()?.path();
        let name = p.file_name()?.to_str()?.to_string();
        if let Some(stem) = name.strip_suffix(".nofilter.mzML") {
            let d = dir.join(format!("{stem}.default.mzML"));
            if d.exists() {
                pairs.push((p, d));
            }
        }
    }
    pairs.sort();
    Some(pairs)
}

fn without(drop: fn(&Filter) -> bool) -> Vec<Filter> {
    mzx_default_thermo()
        .into_iter()
        .filter(|f| !drop(f))
        .collect()
}

#[test]
fn default_filters_reproduce_msconvert() {
    let Some(pairs) = goldens() else {
        eprintln!("MZX_CORPUS not set; skipping");
        return;
    };
    assert!(!pairs.is_empty(), "no nofilter/default golden pairs");
    let mut total_changed = 0;
    for (nofilter, default) in pairs {
        let (n, d) = (
            std::fs::read(&nofilter).unwrap(),
            std::fs::read(&default).unwrap(),
        );
        let p = mzx_verify::prove_filters(&n, &d, &mzx_default_thermo()).unwrap();
        assert!(
            p.mismatch.is_none(),
            "{}: {:?}",
            default.display(),
            p.mismatch
        );
        eprintln!(
            "{}: identical; {} spectra filtered, {} reader-owned peak-intensity lines",
            default.display(),
            p.spectra_changed,
            p.reader_owned_differences
        );
        total_changed += p.spectra_changed;

        // Negative controls: dropping a filter that had an effect must fail.
        // peakPicking always appends its processing method.
        let p2 = mzx_verify::prove_filters(
            &n,
            &d,
            &without(|f| matches!(f, Filter::PeakPickingVendor { .. })),
        )
        .unwrap();
        assert!(
            p2.mismatch.is_some(),
            "proof is vacuous without peakPicking"
        );
        // zeroSamples only matters where it changed spectra (Orbitrap
        // centroid data has no zero samples).
        if p.spectra_changed > 0 {
            let p3 = mzx_verify::prove_filters(
                &n,
                &d,
                &without(|f| matches!(f, Filter::ZeroSamplesRemoveExtra { .. })),
            )
            .unwrap();
            assert!(
                p3.mismatch.is_some(),
                "proof is vacuous without zeroSamples"
            );
        }
    }
    assert!(total_changed > 0, "corpus never exercises zeroSamples");
}
