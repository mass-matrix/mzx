//! Development lab: test ion-trap centroiding hypotheses against golden
//! precursor peak intensities (`ms2 scan, ms1 scan, iso m/z, value` TSV).

use mzx_mzml::float::double12;
use mzx_thermo::raw::{Peaks, RawFile};

struct Profile {
    mz: Vec<f64>,
    y: Vec<f64>,
}

/// Candidate centroider: returns (m/z, intensity) peaks.
type Centroider = fn(&Profile) -> Vec<(f64, f64)>;

/// Maximal runs of non-zero points.
fn runs(p: &Profile) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut k = 0;
    while k < p.y.len() {
        if p.y[k] > 0.0 {
            let s = k;
            while k < p.y.len() && p.y[k] > 0.0 {
                k += 1;
            }
            out.push((s, k));
        } else {
            k += 1;
        }
    }
    out
}

/// Split [s, e) at interior local minima (strict).
fn split_minima(p: &Profile, s: usize, e: usize) -> Vec<(usize, usize)> {
    let mut out = vec![];
    let mut a = s;
    for k in s + 1..e.saturating_sub(1) {
        if p.y[k] < p.y[k - 1] && p.y[k] <= p.y[k + 1] {
            out.push((a, k + 1));
            a = k + 1;
        }
    }
    out.push((a, e));
    out
}

fn summarize(p: &Profile, s: usize, e: usize) -> (f64, f64) {
    let sum: f64 = p.y[s..e].iter().sum();
    let wm: f64 = (s..e).map(|k| p.mz[k] * p.y[k]).sum::<f64>() / sum;
    (wm, sum)
}

fn v_runs(p: &Profile) -> Vec<(f64, f64)> {
    runs(p)
        .into_iter()
        .map(|(s, e)| summarize(p, s, e))
        .collect()
}

fn v_minima(p: &Profile) -> Vec<(f64, f64)> {
    runs(p)
        .into_iter()
        .flat_map(|(s, e)| split_minima(p, s, e))
        .map(|(s, e)| summarize(p, s, e))
        .collect()
}

fn v_apex(p: &Profile) -> Vec<(f64, f64)> {
    let mut out = vec![];
    for k in 1..p.y.len().saturating_sub(1) {
        if p.y[k] > p.y[k - 1] && p.y[k] >= p.y[k + 1] {
            out.push((p.mz[k], p.y[k]));
        }
    }
    out
}

fn window_sum(peaks: &[(f64, f64)], iso: f64) -> f64 {
    let (lo, hi) = (iso - 1.5, iso + 1.5);
    let mut sorted = peaks.to_vec();
    sorted.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    sorted
        .iter()
        .filter(|(m, _)| *m >= lo && *m < hi)
        .map(|(_, y)| y)
        .sum()
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let raw_path = args
        .next()
        .expect("usage: centroid-lab <raw> <tsv> [detail-scan]");
    let tsv = args.next().unwrap();
    let detail: Option<u32> = args.next().map(|s| s.parse().unwrap());
    let raw = RawFile::open(raw_path.as_ref())?;
    let ms_i = raw.controllers.iter().position(|c| c.kind == 0).unwrap();
    let rh = &raw.run_headers[ms_i];
    let idx = raw.ms_scan_index(rh)?;
    let profile = |scan: u32| -> Profile {
        match raw.ms_peaks(rh, &idx[(scan - 1) as usize]).unwrap() {
            Peaks::Profile {
                first,
                step,
                intensity,
            } => Profile {
                mz: (1..=intensity.len())
                    .map(|k| first + k as f64 * step)
                    .collect(),
                y: intensity.into_iter().map(f64::from).collect(),
            },
            _ => panic!("scan {scan} is not profile"),
        }
    };
    let variants: [(&str, Centroider); 3] =
        [("runs", v_runs), ("minima", v_minima), ("apex", v_apex)];
    let mut hits = vec![0usize; variants.len()];
    let mut n = 0;
    for line in std::fs::read_to_string(tsv)?.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        let (ms2, ms1, iso, want): (u32, u32, f64, &str) =
            (f[0].parse()?, f[1].parse()?, f[2].parse()?, f[3]);
        let p = profile(ms1);
        n += 1;
        for (vi, (name, c)) in variants.iter().enumerate() {
            let got = double12(window_sum(&c(&p), iso));
            if got == want {
                hits[vi] += 1;
            } else if detail == Some(ms2) {
                println!("{name}: got {got} want {want}");
            }
        }
        if detail == Some(ms2) {
            let (lo, hi) = (iso - 1.5, iso + 1.5);
            let a = p.mz.partition_point(|&m| m < lo - 1.0);
            let b = p.mz.partition_point(|&m| m < hi + 1.0);
            for k in a..b {
                println!(
                    "{k}\t{:.6}\t{}\t{}",
                    p.mz[k],
                    p.y[k],
                    if p.mz[k] >= lo && p.mz[k] < hi {
                        "in"
                    } else {
                        ""
                    }
                );
            }
        }
    }
    for (vi, (name, _)) in variants.iter().enumerate() {
        println!("{name}: {}/{n}", hits[vi]);
    }
    Ok(())
}
