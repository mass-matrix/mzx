//! Scan filter: decoded from a v66 scan event, formatted like the Thermo SDK's
//! `IScanFilter.ToString()`.
//!
//! Only the event shapes present in the proof corpus are accepted, per
//! instrument [`Family`]. Every preamble byte that is not decoded for that
//! family must equal the family's reference pattern, so an instrument setting
//! we have not seen (polarity, analyzer, ionization, scan type, ...) is
//! rejected instead of guessed.

use anyhow::{bail, Result};

use crate::events::{ScanEvent, PREAMBLE_LEN};
use crate::Unsupported;

/// Preamble of an `FTMS + c ESI Full lock ms` scan (Q Exactive Focus corpus file).
const QE_REFERENCE: [u8; PREAMBLE_LEN] = [
    1, 0, 2, 1, 1, 0, 1, 0, 1, 2, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 255, 255,
    255, 255, 2, 0, 0, 0, 2, 0, 0, 0, 4, 2, 0, 2, 2, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 2, 4,
    2, 2, 2, 2, 2, 2, 2, 0, 0, 0, 0, 0,
];

/// Preamble of an `ITMS - p ESI Full ms` scan (LTQ XL corpus file).
const LTQ_REFERENCE: [u8; PREAMBLE_LEN] = [
    1, 0, 2, 1, 0, 1, 1, 0, 1, 1, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 255, 255,
    255, 255, 0, 0, 0, 0, 2, 0, 0, 0, 0, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0,
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 4,
    2, 2, 2, 2, 2, 0, 2, 0, 0, 0, 0, 0,
];

/// 0 = centroid, 1 = profile.
const SCAN_DATA: usize = 5;
const MS_POWER: usize = 6;
const DEPENDENT: usize = 10;
/// TriState: 0 = On, 1 = Off, 2 = Any (not shown).
const LOCK: usize = 42;
const PRIMARY_FLAG: usize = 104;
const PREAMBLE_ENERGY: usize = 112;

/// `Reaction::flags`: bit 0 = collision energy valid, bits 1.. = activation.
const REACTION_CID: u32 = 0x01;
const REACTION_HCD: u32 = 0x0b;

/// Instrument family whose scan-event layout is proven.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Q Exactive Focus: FTMS, positive, centroid, HCD.
    QExactive,
    /// LTQ XL: ITMS, negative, profile MS1 + centroid CID MS2.
    LtqXl,
}

impl Family {
    fn reference(self) -> &'static [u8; PREAMBLE_LEN] {
        match self {
            Family::QExactive => &QE_REFERENCE,
            Family::LtqXl => &LTQ_REFERENCE,
        }
    }

    fn is_variable(self, i: usize) -> bool {
        let energy = (PREAMBLE_ENERGY..PREAMBLE_ENERGY + 8).contains(&i);
        match self {
            Family::QExactive => matches!(i, MS_POWER | DEPENDENT | LOCK | PRIMARY_FLAG) || energy,
            Family::LtqXl => matches!(i, SCAN_DATA | MS_POWER | DEPENDENT),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Analyzer {
    Itms,
    Ftms,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    Cid,
    Hcd,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FilterPrecursor {
    pub mz: f64,
    pub isolation_width: f64,
    pub activation: Activation,
    pub energy: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanFilter {
    /// 1 = MS, 2 = MS2, ...
    pub ms_order: u32,
    pub dependent: bool,
    /// `None` = TriState Any (no token).
    pub lock: Option<bool>,
    pub analyzer: Analyzer,
    pub positive: bool,
    pub centroid: bool,
    /// `IScanFilter` masses (one per precursor level).
    pub precursors: Vec<FilterPrecursor>,
    pub mass_ranges: Vec<(f64, f64)>,
}

fn unsupported(msg: String) -> anyhow::Error {
    Unsupported(msg).into()
}

impl ScanFilter {
    pub fn decode(e: &ScanEvent, family: Family) -> Result<Self> {
        let p = &e.preamble;
        for (i, (&got, &want)) in p.iter().zip(family.reference().iter()).enumerate() {
            if !family.is_variable(i) && got != want {
                return Err(unsupported(format!(
                    "scan event preamble byte {i} = {got} (only {want} is proven for {family:?})"
                )));
            }
        }
        let ms_order = p[MS_POWER] as u32;
        let dependent = match p[DEPENDENT] {
            0 => false,
            1 => true,
            v => return Err(unsupported(format!("dependent flag {v}"))),
        };
        let lock = match p[LOCK] {
            0 => Some(true),
            1 => Some(false),
            2 => None,
            v => return Err(unsupported(format!("lock tri-state {v}"))),
        };
        let centroid = match p[SCAN_DATA] {
            0 => true,
            1 => false,
            v => return Err(unsupported(format!("scan data type {v}"))),
        };
        let preamble_energy =
            f64::from_le_bytes(p[PREAMBLE_ENERGY..PREAMBLE_ENERGY + 8].try_into().unwrap());
        let shape = |ok: bool| -> Result<()> {
            if ok {
                Ok(())
            } else {
                Err(unsupported(format!(
                    "unproven ms{ms_order} scan event shape for {family:?}"
                )))
            }
        };
        let precursors = match (family, ms_order) {
            (Family::QExactive, 1) => {
                // The MS1 "reaction" is the quadrupole window: no filter mass.
                shape(
                    e.reactions.len() == 1
                        && e.reactions[0].range_valid == 1
                        && p[PRIMARY_FLAG] == 1
                        && preamble_energy == 0.0
                        && !dependent,
                )?;
                vec![]
            }
            (Family::QExactive, 2) => {
                shape(e.reactions.len() == 1)?;
                let r = &e.reactions[0];
                shape(
                    r.range_valid == 0
                        && p[PRIMARY_FLAG] == 0
                        && r.flags == REACTION_HCD
                        && preamble_energy == r.collision_energy
                        && lock == Some(false),
                )?;
                vec![FilterPrecursor {
                    mz: r.precursor_mz,
                    isolation_width: r.isolation_width,
                    activation: Activation::Hcd,
                    energy: r.collision_energy,
                }]
            }
            (Family::LtqXl, 1) => {
                shape(e.reactions.is_empty() && !centroid && !dependent)?;
                vec![]
            }
            (Family::LtqXl, 2) => {
                shape(e.reactions.len() == 1 && centroid && dependent)?;
                let r = &e.reactions[0];
                shape(r.range_valid == 0 && r.flags == REACTION_CID && r.isolation_offset == 0.0)?;
                vec![FilterPrecursor {
                    mz: r.precursor_mz,
                    isolation_width: r.isolation_width,
                    activation: Activation::Cid,
                    energy: r.collision_energy,
                }]
            }
            (_, n) => return Err(unsupported(format!("MS order {n} for {family:?}"))),
        };
        if e.mass_ranges.is_empty() {
            bail!("scan event without a mass range");
        }
        let (analyzer, positive) = match family {
            Family::QExactive => (Analyzer::Ftms, true),
            Family::LtqXl => (Analyzer::Itms, false),
        };
        Ok(Self {
            ms_order,
            dependent,
            lock,
            analyzer,
            positive,
            centroid,
            precursors,
            mass_ranges: e.mass_ranges.clone(),
        })
    }
}

impl std::fmt::Display for ScanFilter {
    /// `IScanFilter.ToString()` for the proven shapes, e.g.
    /// `FTMS + c ESI d Full ms2 158.9614@hcd30.00 [50.0000-180.0000]`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mass precision follows the analyzer: 4 decimals for FTMS, 2 for ITMS.
        let (analyzer, prec) = match self.analyzer {
            Analyzer::Ftms => ("FTMS", 4),
            Analyzer::Itms => ("ITMS", 2),
        };
        write!(
            f,
            "{analyzer} {} {} ESI ",
            if self.positive { '+' } else { '-' },
            if self.centroid { 'c' } else { 'p' }
        )?;
        if self.dependent {
            f.write_str("d ")?;
        }
        f.write_str("Full ")?;
        if self.lock == Some(true) {
            f.write_str("lock ")?;
        }
        if self.ms_order == 1 {
            f.write_str("ms")?;
        } else {
            write!(f, "ms{}", self.ms_order)?;
        }
        for p in &self.precursors {
            let act = match p.activation {
                Activation::Cid => "cid",
                Activation::Hcd => "hcd",
            };
            write!(f, " {:.prec$}@{act}{:.2}", p.mz, p.energy)?;
        }
        f.write_str(" [")?;
        for (i, (lo, hi)) in self.mass_ranges.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{lo:.prec$}-{hi:.prec$}")?;
        }
        f.write_str("]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Reaction;

    fn qe_event(ms_order: u8) -> ScanEvent {
        let mut preamble = QE_REFERENCE;
        preamble[MS_POWER] = ms_order;
        let reaction = if ms_order == 1 {
            Reaction {
                precursor_mz: 810.0,
                isolation_width: 1380.0,
                collision_energy: 0.0,
                flags: REACTION_HCD,
                range_valid: 1,
                first_mz: 120.0,
                last_mz: 1500.0,
                isolation_offset: 0.0,
            }
        } else {
            preamble[DEPENDENT] = 1;
            preamble[LOCK] = 1;
            preamble[PRIMARY_FLAG] = 0;
            preamble[PREAMBLE_ENERGY..PREAMBLE_ENERGY + 8].copy_from_slice(&30.0f64.to_le_bytes());
            Reaction {
                precursor_mz: 158.9614105224609,
                isolation_width: 1.0,
                collision_energy: 30.0,
                flags: REACTION_HCD,
                range_valid: 0,
                first_mz: 0.0,
                last_mz: 0.0,
                isolation_offset: 0.0,
            }
        };
        ScanEvent {
            preamble,
            reactions: vec![reaction],
            mass_ranges: vec![if ms_order == 1 {
                (120.0, 1500.0)
            } else {
                (50.0, 180.0)
            }],
            conversion_params: vec![],
            tail: vec![1, 0, 0, 0, 0],
        }
    }

    fn ltq_event(ms_order: u8) -> ScanEvent {
        let mut preamble = LTQ_REFERENCE;
        preamble[MS_POWER] = ms_order;
        let mut reactions = vec![];
        let mut range = (600.0, 2000.0);
        if ms_order == 2 {
            preamble[SCAN_DATA] = 0;
            preamble[DEPENDENT] = 1;
            reactions.push(Reaction {
                precursor_mz: 814.6354370117188,
                isolation_width: 3.0,
                collision_energy: 35.0,
                flags: REACTION_CID,
                range_valid: 0,
                first_mz: 0.0,
                last_mz: 0.0,
                isolation_offset: 0.0,
            });
            range = (210.0, 1640.0);
        }
        ScanEvent {
            preamble,
            reactions,
            mass_ranges: vec![range],
            conversion_params: vec![],
            tail: vec![0, 0, 0],
        }
    }

    #[test]
    fn formats_proven_shapes() {
        let s = |e, fam| ScanFilter::decode(&e, fam).unwrap().to_string();
        assert_eq!(
            s(qe_event(1), Family::QExactive),
            "FTMS + c ESI Full lock ms [120.0000-1500.0000]"
        );
        assert_eq!(
            s(qe_event(2), Family::QExactive),
            "FTMS + c ESI d Full ms2 158.9614@hcd30.00 [50.0000-180.0000]"
        );
        assert_eq!(
            s(ltq_event(1), Family::LtqXl),
            "ITMS - p ESI Full ms [600.00-2000.00]"
        );
        assert_eq!(
            s(ltq_event(2), Family::LtqXl),
            "ITMS - c ESI d Full ms2 814.64@cid35.00 [210.00-1640.00]"
        );
    }

    fn assert_unsupported(e: &ScanEvent, fam: Family, what: &str) {
        let err = ScanFilter::decode(e, fam).unwrap_err();
        assert!(err.downcast_ref::<Unsupported>().is_some(), "{what}: {err}");
    }

    #[test]
    fn unproven_bytes_are_refused() {
        // Every non-decoded byte, perturbed, must be Unsupported (polarity,
        // analyzer, ionization, scan type, ...).
        for (fam, ev) in [
            (Family::QExactive, qe_event as fn(u8) -> ScanEvent),
            (Family::LtqXl, ltq_event),
        ] {
            for i in 0..PREAMBLE_LEN {
                if fam.is_variable(i) {
                    continue;
                }
                for ms in [1, 2] {
                    let mut e = ev(ms);
                    e.preamble[i] = e.preamble[i].wrapping_add(1);
                    assert_unsupported(&e, fam, &format!("{fam:?} ms{ms} byte {i}"));
                }
            }
            let mut e = ev(2);
            e.reactions[0].flags = 0x09; // a different activation
            assert_unsupported(&e, fam, "activation");
            let mut e = ev(1);
            e.preamble[MS_POWER] = 3;
            assert_unsupported(&e, fam, "ms3");
        }
        // A family's events are not accepted as the other family's.
        assert_unsupported(&ltq_event(2), Family::QExactive, "ltq as qe");
        assert_unsupported(&qe_event(2), Family::LtqXl, "qe as ltq");
        // LTQ: centroid MS1 / profile MS2 are not proven.
        let mut e = ltq_event(1);
        e.preamble[SCAN_DATA] = 0;
        assert_unsupported(&e, Family::LtqXl, "centroid ms1");
        let mut e = ltq_event(2);
        e.preamble[SCAN_DATA] = 1;
        assert_unsupported(&e, Family::LtqXl, "profile ms2");
    }
}
