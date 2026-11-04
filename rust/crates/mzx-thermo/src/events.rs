//! Per-scan scan events (v66) and the scan-event template hierarchy.

use anyhow::{ensure, Result};

use crate::bytes::Cursor;

pub const PREAMBLE_LEN: usize = 136;

/// One reaction (precursor / isolation) entry of a scan event.
#[derive(Debug, Clone, PartialEq)]
pub struct Reaction {
    pub precursor_mz: f64,
    pub isolation_width: f64,
    pub collision_energy: f64,
    /// Bit 0: collision energy valid; bits 1..: activation type
    /// (observed: 0x0b = valid + HCD).
    pub flags: u32,
    pub range_valid: u32,
    pub first_mz: f64,
    pub last_mz: f64,
    pub isolation_offset: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScanEvent {
    pub preamble: [u8; PREAMBLE_LEN],
    pub reactions: Vec<Reaction>,
    pub mass_ranges: Vec<(f64, f64)>,
    pub conversion_params: Vec<f64>,
    /// u32 n, n x 8 bytes, 2 x u32 (QE: n=1, LTQ: n=0): kept raw, never interpreted.
    pub tail: Vec<u32>,
}

impl ScanEvent {
    pub fn read(c: &mut Cursor) -> Result<Self> {
        let preamble: [u8; PREAMBLE_LEN] = c.take(PREAMBLE_LEN)?.try_into().unwrap();
        let n = c.u32()? as usize;
        ensure!(n <= 16, "implausible reaction count {n}");
        let mut reactions = Vec::with_capacity(n);
        for _ in 0..n {
            reactions.push(Reaction {
                precursor_mz: c.f64()?,
                isolation_width: c.f64()?,
                collision_energy: c.f64()?,
                flags: c.u32()?,
                range_valid: c.u32()?,
                first_mz: c.f64()?,
                last_mz: c.f64()?,
                isolation_offset: c.f64()?,
            });
        }
        let n = c.u32()? as usize;
        ensure!(n <= 64, "implausible mass range count {n}");
        let mut mass_ranges = Vec::with_capacity(n);
        for _ in 0..n {
            mass_ranges.push((c.f64()?, c.f64()?));
        }
        let n = c.u32()? as usize;
        ensure!(n <= 64, "implausible conversion parameter count {n}");
        let mut conversion_params = Vec::with_capacity(n);
        for _ in 0..n {
            conversion_params.push(c.f64()?);
        }
        let n = c.u32()? as usize;
        ensure!(n <= 64, "implausible scan event tail count {n}");
        let mut tail = vec![n as u32];
        for _ in 0..n * 2 + 2 {
            tail.push(c.u32()?);
        }
        Ok(Self {
            preamble,
            reactions,
            mass_ranges,
            conversion_params,
            tail,
        })
    }
}
