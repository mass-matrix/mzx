//! Port of `Pwiz.Util.Misc.IntegerSet.Parse` for the forms msconvert filters
//! accept: `"2"`, `"2-3"`, `"2-"` (open-ended), and comma/space separated lists.

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntegerSet {
    intervals: Vec<(i64, i64)>,
}

impl IntegerSet {
    /// Every positive integer (`IntegerSet.Positive`).
    pub fn positive() -> Self {
        Self {
            intervals: vec![(1, i64::from(i32::MAX))],
        }
    }

    pub fn parse(spec: &str) -> Result<Self> {
        let mut intervals = Vec::new();
        for tok in spec.split([',', ' ', '\t']).filter(|t| !t.is_empty()) {
            let parse = |s: &str| -> Result<i64> {
                s.trim()
                    .parse()
                    .with_context(|| format!("bad integer in set {spec:?}"))
            };
            let iv = match tok.split_once('-') {
                None => {
                    let v = parse(tok)?;
                    (v, v)
                }
                Some((a, "")) => (parse(a)?, i64::from(i32::MAX)),
                Some((a, b)) => (parse(a)?, parse(b)?),
            };
            if iv.0 > iv.1 {
                bail!("empty interval {tok:?} in set {spec:?}");
            }
            intervals.push(iv);
        }
        if intervals.is_empty() {
            bail!("empty integer set {spec:?}");
        }
        Ok(Self { intervals })
    }

    pub fn contains(&self, v: i64) -> bool {
        self.intervals.iter().any(|&(a, b)| a <= v && v <= b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forms() {
        let s = IntegerSet::parse("2-").unwrap();
        assert!(!s.contains(1) && s.contains(2) && s.contains(10));
        let s = IntegerSet::parse("1,3-4").unwrap();
        assert!(s.contains(1) && !s.contains(2) && s.contains(4) && !s.contains(5));
        assert!(IntegerSet::positive().contains(1));
        assert!(IntegerSet::parse("").is_err());
    }
}
