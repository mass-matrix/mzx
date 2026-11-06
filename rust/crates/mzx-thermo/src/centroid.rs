//! Reproduction of Thermo's ion-trap (LTQ) profile centroiding
//! (`Scan.ToCentroid`), which msconvert uses for vendor peak picking.
//!
//! The algorithm is closed. This model was inferred only from msconvert output
//! for real files and for synthetic probe files (profile intensities patched
//! into a copy of a RAW file). Thermo's RawFileReader was never decompiled or
//! inspected. It reproduces every centroid of the corpus bit for bit (three
//! LTQ XL files, MS1 and MS2, and five probe files). A closed algorithm can't
//! be proven in general: the guarantee is only as broad as that corpus.
//!
//! Model, for a profile with point `k` at `first + (k + 1) * step`:
//! 1. Runs are maximal stretches of points with intensity >= 1. Smaller
//!    points never count towards a centroid.
//! 2. A state machine cuts each run into sub-peaks ([`subpeaks`]). A sub-peak
//!    closed by a cut uses `mz[k - 1]` for each of its points (shifted by
//!    -1 bin). The last sub-peak of a run is shifted when its "half drop"
//!    happened before its last point, or when the run, extended by the
//!    following points, would cut at or after its end (the tail lookahead).
//! 3. Across a single sub-1 gap `g` followed by a point in `[1, 2g]`, a left
//!    sub-peak that is unshifted and at least 4 points wide continues into
//!    the next run: its half-drop state carries over, and the run is cut
//!    after its first point when the second point rises above twice the
//!    first or above the left sub-peak's last point. The tail lookahead sees
//!    the same carried state. When the next run then starts with a single
//!    shifted point, that point is fused into the left sub-peak.
//! 4. Sub-peak centroids (intensity sum, intensity-weighted m/z) are merged
//!    left to right into the running centroid while closer than 0.5 m/z.
//!
//! A scan with no point >= 1 has no centroids; the reader then returns the
//! profile, as pwiz does when `Scan.ToCentroid` finds no peaks.

/// Sub-peaks closer than this (in m/z) merge: 6 nominal bins of 1/12, not 6
/// times the file's step (`f32` 1/12, which is 1.5e-8 more). Pinned by
/// IPRP_ecoli scan 8571 (no merge at 6 steps - 7.8e-9) and probe 3 (merge at
/// 6 steps - 1.5e-8, none at 6 steps - 1.7e-9).
const MERGE_MZ: f64 = 0.5;
/// The only profile step the model is known for: LTQ normal scan rate, 1/12
/// m/z stored as `f32`.
pub const STEP: f64 = (1.0f32 / 12.0) as f64;
const THRESHOLD: f64 = 1.0;
/// Points at the end of the profile that are never part of a centroid.
const END_SKIP: usize = 2;

/// State carried from one run into the next across a single sub-1 gap.
#[derive(Default, Clone, Copy)]
struct Carry {
    /// The last point of the continued left sub-peak (`None`: the run
    /// starts fresh).
    last: Option<f64>,
    /// Position of the left sub-peak's half drop, in the next run's
    /// coordinates (negative).
    half_drop: Option<i64>,
}

/// Sub-peaks `(start, end, shifted)` of the run `y[..n]` (all points >= 1),
/// and the half-drop position of the last sub-peak. Points after `n` are only
/// looked at (the width cut's look-ahead; they matter where a run is cut off
/// by the profile's last two points). Width cuts are not made at or after
/// `no_width_from`.
fn subpeaks(
    y: &[f64],
    n: usize,
    tail: bool,
    carry: Carry,
    no_width_from: usize,
) -> (Vec<(usize, usize, bool)>, Option<i64>) {
    let mut out = vec![];
    let mut s = 0usize;
    // f: a half drop y[j] < y[j-1] / 2 happened at fj.
    let (mut f, mut fj) = match carry.half_drop {
        Some(v) => (true, v),
        None => (false, 0i64),
    };
    // a: the apex (first non-increase); np: the sub-peak is past its top.
    let (mut np, mut a) = (false, None::<usize>);
    let mut j = 1;
    while j < n {
        let ww = j - s;
        let next = |k: usize| y.get(k).copied();
        // np as it stands after this point (the width cut sees it early)
        let np_now = np
            || (j >= s + 3 && y[j] <= y[j - 3] && (a.is_some() || (y[j] <= y[j - 1] && j - 1 > s)));
        let width_cut = ww >= 13
            && j < no_width_from
            && !(f && j as i64 >= fj + 2)
            && (!np_now || !next(j + 1).is_some_and(|v| v > y[j - 2]))
            && !next(j + 1).is_some_and(|v| v > 2.0 * y[j]);
        // the second point of a run continuing a left sub-peak
        let continued = s == 0 && j == 1 && carry.last.is_some();
        let cut = width_cut
            || (f && j as i64 == fj + 3)
            || (y[j] > 2.0 * y[j - 1] && (ww >= 5 || continued))
            || (j >= s + 3 && y[j] > y[j - 3] && np && ww >= 5)
            || (continued && carry.last.is_some_and(|v| y[1] > v));
        if cut {
            out.push((s, j, true));
            s = j;
            f = false;
            np = false;
            a = None;
            j += 1;
            continue;
        }
        if j >= s + 3 && y[j] > y[j - 3] && np {
            // a valley seen while still narrower than 5 consumes np
            np = false;
        }
        if a.is_none() && y[j] <= y[j - 1] && j - 1 > s {
            a = Some(j - 1);
        }
        if j >= s + 3 && y[j] <= y[j - 3] && a.is_some() {
            np = true;
        }
        if !f && y[j] < 0.5 * y[j - 1] && j - 1 > s {
            f = true;
            fj = j as i64;
        }
        j += 1;
    }
    out.push((s, n, (f && fj + 1 < n as i64) || tail));
    (out, f.then_some(fj))
}

/// One sub-peak after fusion: its points `(index, shifted)`. A shifted point
/// uses the m/z of the point before it.
pub type SubPeak = Vec<(usize, bool)>;

/// The model's segmentation of an ion-trap profile scan: each centroid as
/// the sub-peaks merged into it, in m/z order.
pub fn segments(first: f64, step: f64, intensity: &[f32]) -> Vec<Vec<SubPeak>> {
    let y: Vec<f64> = intensity.iter().map(|&v| v as f64).collect();
    // The last two points never count; a run they cut off closes as a cut.
    let n = y.len().saturating_sub(END_SKIP);

    // Sub-peaks with: first of its run, fuse with the next run.
    let mut subs: Vec<(SubPeak, bool, bool)> = vec![];
    let mut carry = Carry::default();
    let mut i = 0;
    while i < n {
        if y[i] < THRESHOLD {
            i += 1;
            continue;
        }
        let mut e = i;
        while e < n && y[e] >= THRESHOLD {
            e += 1;
        }
        let run = &y[i..e];
        let l = run.len();

        // Tail lookahead: the run plus its gap point and, when it rises, the
        // point after (which may be one of the end points that never count);
        // a cut at or after the run's end shifts the last
        // sub-peak. A width cut does not count at that second point.
        let truncated = e == n && n < y.len();
        let tail = truncated
            || e < n && {
                let mut ext = run.to_vec();
                ext.push(y[e]);
                if e + 1 < y.len() && y[e + 1] > 0.0 && y[e + 1] > y[e] {
                    ext.push(y[e + 1]);
                }
                subpeaks(&ext, ext.len(), false, carry, l + 1)
                    .0
                    .iter()
                    .any(|sp| sp.0 >= l)
            };

        let (sps, last_f) = subpeaks(&y[i..], l, tail, carry, usize::MAX);
        carry = Carry::default();
        let mut fuse_next = false;
        if let Some(&(ls, lt, lsh)) = sps.last() {
            if !lsh
                && lt - ls >= 4
                && e + 1 < n
                && y[e] > 0.0
                && y[e] < THRESHOLD
                && y[e + 1] >= THRESHOLD
                && y[e + 1] <= 2.0 * y[e]
            {
                carry.last = Some(run[l - 1]);
                carry.half_drop = last_f.map(|fj| fj - (l as i64 + 1));
                fuse_next = true;
            }
        }
        let nsps = sps.len();
        for (si, (s, t, sh)) in sps.into_iter().enumerate() {
            let pts = (i + s..i + t).map(|g| (g, sh)).collect();
            // A run of one point shifted only because the profile's end cut it
            // off was not split by a cut, so it is not fused (merged instead;
            // IPRP_ecoli scan 8511, the only such case in the corpus).
            let fusable = si == 0 && !(truncated && nsps == 1);
            subs.push((pts, fusable, fuse_next && si + 1 == nsps));
        }
        i = e;
    }

    // Fuse a single shifted point that continues a carried sub-peak.
    let mut fused: Vec<SubPeak> = vec![];
    let mut prev_fuse = false;
    for (pts, first_of_run, fz) in subs {
        if prev_fuse && first_of_run && pts.len() == 1 && pts[0].1 {
            fused
                .last_mut()
                .expect("a fused sub-peak precedes")
                .extend(pts);
        } else {
            fused.push(pts);
        }
        prev_fuse = fz;
    }

    // Left-to-right merge into the running centroid.
    let mut out: Vec<Vec<SubPeak>> = vec![];
    let mut last: Option<(f64, f64)> = None;
    for sp in fused {
        let c = weighted(first, step, &y, std::slice::from_ref(&sp));
        match last {
            Some(p) if c.0 - p.0 < MERGE_MZ => {
                let ii = p.1 + c.1;
                last = Some(((p.0 * p.1 + c.0 * c.1) / ii, ii));
                out.last_mut().expect("running centroid").push(sp);
            }
            _ => {
                last = Some(c);
                out.push(vec![sp]);
            }
        }
    }
    out
}

/// `(m/z, intensity)` of sub-peaks, combined by running intensity-weighted
/// means `(m_a I_a + m_b I_b) / (I_a + I_b)`. A sub-peak's m/z is
/// `mz[e + 1] + (q - 1) * step`, where `e` is the bin after its last point
/// and `q = sum(I (k - e)) / sum(I)` its intensity-weighted mean offset from
/// `e` (negative). This exact operation order reproduces Thermo's doubles bit
/// for bit, including 189,000 probe peaks whose exact mean lies within 0.003
/// ULP of a rounding tie; simpler orders (`sum(I m) / sum(I)`, offsets from
/// the first point or from `e + 1` directly) miss 1 near-tie in 30 to 50.
fn weighted(first: f64, step: f64, y: &[f64], parts: &[SubPeak]) -> (f64, f64) {
    let mut acc: Option<(f64, f64)> = None;
    for sp in parts {
        let bin = |&(g, sh): &(usize, bool)| g + 1 - sh as usize;
        // the mean offset from the bin after the last point, moved one bin
        // further before it is scaled
        let end = bin(sp.last().expect("a sub-peak has points")) + 1;
        let (mut inten, mut w) = (0f64, 0f64);
        for p in sp {
            inten += y[p.0];
            w += y[p.0] * (bin(p) as f64 - end as f64);
        }
        let q = w / inten - 1.0;
        let c = ((first + (end + 1) as f64 * step) + q * step, inten);
        acc = Some(match acc {
            None => c,
            Some(p) => {
                let ii = p.1 + c.1;
                ((p.0 * p.1 + c.0 * c.1) / ii, ii)
            }
        });
    }
    acc.expect("a centroid has a sub-peak")
}

/// Centroids `(m/z, intensity)` of an ion-trap profile scan whose point `k`
/// (0-based) has m/z `first + (k + 1) * step`, in m/z order.
pub fn centroid(first: f64, step: f64, intensity: &[f32]) -> (Vec<f64>, Vec<f64>) {
    let y: Vec<f64> = intensity.iter().map(|&v| v as f64).collect();
    segments(first, step, intensity)
        .iter()
        .map(|parts| weighted(first, step, &y, parts))
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bins(first: f64, step: f64, mz: &[f64]) -> Vec<f64> {
        mz.iter()
            .map(|m| ((m - first) / step * 1e6).round() / 1e6)
            .collect()
    }

    #[test]
    fn isolated_peak_is_its_weighted_mean() {
        let step = 1.0 / 12.0;
        let (mz, i) = centroid(100.0, step, &[0.0, 2.0, 4.0, 2.0, 0.0, 0.0, 0.0]);
        assert_eq!(i, vec![8.0]);
        assert_eq!(bins(100.0, step, &mz), vec![3.0]); // point 2 is bin 3
    }

    #[test]
    fn sub_threshold_points_do_not_count() {
        let (_, i) = centroid(0.0, 1.0 / 12.0, &[0.5, 3.0, 0.9, 0.0, 0.0, 0.0]);
        assert_eq!(i, vec![3.0]);
    }

    #[test]
    fn half_drop_with_points_after_it_shifts() {
        // probe 5: left peak 4 wide with a half drop, gap 0.996, one point:
        // the right point is fused into the left peak and shifted -1 bin
        let step = 1.0 / 12.0;
        let y = [
            0.0, 4.38483, 7.2166924, 5.073693, 1.21014, 0.995871, 1.5239351, 0.0, 0.0, 0.0,
        ];
        let (mz, i) = centroid(0.0, step, &y);
        assert_eq!(i.len(), 1);
        let unshifted: f64 = [(1, y[1]), (2, y[2]), (3, y[3]), (4, y[4]), (6, y[6])]
            .iter()
            .map(|&(k, v)| (k as f64 + 1.0) * v as f64)
            .sum::<f64>()
            / i[0];
        let got = mz[0] / step;
        assert!((unshifted - got - y[6] as f64 / i[0]).abs() < 1e-6);
    }

    #[test]
    fn last_two_points_never_count_and_cut_off_runs_shift() {
        // IPRP_ecoli scan 2250: 2.47 9.78 4.27 are the last three points;
        // Thermo keeps only 2.47, shifted by -1 bin.
        let step = 1.0 / 12.0;
        let (mz, i) = centroid(0.0, step, &[0.0, 0.0, 2.47, 9.78, 4.27]);
        assert_eq!(i, vec![2.47f32 as f64]);
        assert_eq!(bins(0.0, step, &mz), vec![2.0]); // point 2, shifted: bin 2
    }

    /// Indices of the shifted points of the model's segmentation of `y`
    /// (padded with zeros), relative to `y[0]`.
    fn shifted(y: &[f32]) -> Vec<usize> {
        let mut v = vec![0f32; 3];
        v.extend_from_slice(y);
        v.extend_from_slice(&[0.0; 4]);
        let mut out: Vec<usize> = segments(0.0, 1.0 / 12.0, &v)
            .iter()
            .flatten()
            .flatten()
            .filter(|p| p.1)
            .map(|p| p.0 - 3)
            .collect();
        out.sort();
        out
    }

    #[test]
    fn m_z_bits_match_thermo_at_a_rounding_near_tie() {
        // probe 6, a two-point peak whose exact mean is within 0.0002 ULP of a
        // rounding tie: only the offset-from-the-end order gives Thermo's bits
        let mut y = vec![0f32; 1100];
        y[1011] = 212.6774;
        y[1012] = 243.88754;
        let (mz, i) = centroid(600.0, (1.0f32 / 12.0) as f64, &y);
        assert_eq!(i.len(), 1);
        assert_eq!(mz[0].to_bits(), 4649231067056869058);
    }

    #[test]
    fn continued_run_cuts_on_a_rise_above_the_left_peaks_last_point() {
        // IPRP_ecoli scan 4383: 2.03 > 1.21 cuts after 1.36, which is fused
        // into the left peak, shifted
        let y = [2.81, 2.76, 2.23, 1.21, 0.92, 1.36, 2.03, 1.04];
        assert_eq!(shifted(&y), vec![5]);
    }

    #[test]
    fn continued_run_cuts_on_a_2x_rise_only_at_its_second_point() {
        // probe 6: 10.58 > 2 x 3.53 at the third point after the gap is no cut
        let y = [
            34.235783, 45.620155, 39.666645, 32.592434, 0.9882426, 1.838472, 3.5338225, 10.584775,
        ];
        assert_eq!(shifted(&y), Vec::<usize>::new());
    }

    #[test]
    fn width_cut_waits_for_a_valley_that_follows_the_top() {
        // IPRP scan 7343: 14 points, not 13, before the valley cut at 61.89
        let y = [
            1.03, 2.81, 2.96, 15.86, 14.43, 18.54, 26.68, 37.86, 40.87, 47.37, 58.29, 58.55, 52.54,
            54.25, 61.89, 0.0,
        ];
        let segs = segments(0.0, 1.0 / 12.0, &[&[0f32; 3][..], &y, &[0.0; 4]].concat());
        assert_eq!(segs[0][0].len(), 14);
    }
}
