//! Finnigan RAW container: file header, sequence row, autosampler info,
//! raw-file info (controller table) and per-controller run headers.
//!
//! Layout follows the public `unfinnigan` documentation (Finnigan::*), checked
//! against RAW v66 files. Only v66 is accepted; anything else is unsupported.

use anyhow::{bail, ensure, Context, Result};

use crate::bytes::Cursor;

pub const SUPPORTED_VERSION: u32 = 66;

#[derive(Debug, Clone)]
pub struct FileHeader {
    pub version: u32,
    /// Windows FILETIME (100 ns ticks since 1601-01-01) of the audit-start tag.
    pub created_filetime: u64,
}

#[derive(Debug, Clone)]
pub struct ControllerEntry {
    /// Device kind as stored in the controller table (not the SDK `Device` enum).
    pub kind: u32,
    pub index: u32,
    pub run_header_addr: u64,
}

#[derive(Debug, Clone)]
pub struct RunHeader {
    pub addr: u64,
    pub first_scan: u32,
    pub last_scan: u32,
    pub inst_log_length: u32,
    pub error_log_length: u32,
    pub max_ion_current: f64,
    pub low_mz: f64,
    pub high_mz: f64,
    pub start_time: f64,
    pub end_time: f64,
    pub tags: [String; 3],
    pub file_names: Vec<String>,
    pub nsegs: u32,
    pub scan_index_addr: u64,
    pub data_addr: u64,
    pub inst_log_addr: u64,
    pub error_log_addr: u64,
    pub unknown_addr: u64,
    pub scan_trailer_addr: u64,
    pub scan_params_addr: u64,
    pub unknown_longs: Vec<u32>,
    pub inst_id: InstId,
    /// File position right after the InstID block (status-log header follows).
    pub after_inst_id: usize,
}

#[derive(Debug, Clone, Default)]
pub struct InstId {
    pub models: Vec<String>,
    pub serial_number: String,
    pub software_version: String,
    pub tags: Vec<String>,
}

#[derive(Debug)]
pub struct RawFile {
    pub bytes: Vec<u8>,
    pub header: FileHeader,
    pub data_addr: u64,
    pub controllers: Vec<ControllerEntry>,
    pub run_headers: Vec<RunHeader>,
    pub sample_id: String,
    pub method: Option<MethodFile>,
}

/// Byte offset of the file-header checksum (the last word of the audit-start tag).
pub const CHECKSUM_OFFSET: usize = 148;
/// The checksum covers this many leading bytes of the file.
pub const CHECKSUM_SPAN: usize = 10 * 1024 * 1024;

/// The file-header checksum as stored.
pub fn stored_checksum(bytes: &[u8]) -> Option<u32> {
    let w = bytes.get(CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4)?;
    Some(u32::from_le_bytes(w.try_into().unwrap()))
}

/// The file-header checksum recomputed: Adler-32 with an initial value of 0
/// over the first 10 MiB, with the checksum field read as zero. Inferred from
/// the corpus (matches all 7 RAW files) and confirmed by msconvert rejecting
/// files whose covered bytes were edited ("IsError").
pub fn compute_checksum(bytes: &[u8]) -> u32 {
    const M: u32 = 65521;
    let (mut a, mut b) = (0u32, 0u32);
    for (i, &x) in bytes.iter().take(CHECKSUM_SPAN).enumerate() {
        let x = if (CHECKSUM_OFFSET..CHECKSUM_OFFSET + 4).contains(&i) {
            0
        } else {
            x
        };
        a = (a + x as u32) % M;
        b = (b + a) % M;
    }
    (b << 16) | a
}

impl RawFile {
    pub fn open(path: &std::path::Path) -> Result<Self> {
        let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Self::parse(bytes)
    }

    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let mut c = Cursor::new(&bytes, 0);
        let header = file_header(&mut c)?;
        let sample_id = seq_row(&mut c)?;
        as_info(&mut c)?;
        let (data_addr, controllers) = raw_file_info(&mut c)?;
        let method = method_file(&mut c)?;
        let mut run_headers = Vec::new();
        for ctl in &controllers {
            let mut rc = Cursor::new(&bytes, ctl.run_header_addr as usize);
            run_headers.push(run_header(&mut rc).with_context(|| {
                format!(
                    "run header for controller {} at {}",
                    ctl.index, ctl.run_header_addr
                )
            })?);
        }
        Ok(Self {
            bytes,
            header,
            data_addr,
            controllers,
            run_headers,
            sample_id,
            method,
        })
    }

    /// `IRawDataPlus.GetInstrumentMethod(i)`: the `Text` stream of each
    /// instrument storage in the embedded method (UTF-16LE).
    pub fn instrument_methods(&self) -> Result<Vec<String>> {
        let Some(m) = &self.method else {
            return Ok(vec![]);
        };
        let ole = std::io::Cursor::new(&self.bytes[m.ole.clone()]);
        let mut cf = cfb::CompoundFile::open(ole).context("embedded method file")?;
        let mut out = Vec::new();
        for (_, storage) in &m.instruments {
            let mut s = cf
                .open_stream(format!("/{storage}/Text"))
                .with_context(|| format!("method stream {storage}/Text"))?;
            let mut b = Vec::new();
            std::io::Read::read_to_end(&mut s, &mut b)?;
            out.push(crate::bytes::utf16_until_nul(&b));
        }
        Ok(out)
    }
}

/// Embedded instrument method (OLE2 compound file) location.
#[derive(Debug, Clone)]
pub struct MethodFile {
    /// (instrument name, storage name) pairs.
    pub instruments: Vec<(String, String)>,
    pub ole: std::ops::Range<usize>,
}

/// MethodFile: FileHeader, u32 OLE size, pascal path, u32 n, n x (pascal
/// name, pascal storage), then the OLE2 container. Absent when no Finnigan
/// header follows RawFileInfo.
fn method_file(c: &mut Cursor) -> Result<Option<MethodFile>> {
    let mut probe = c.clone();
    if file_header(&mut probe).is_err() {
        return Ok(None);
    }
    *c = probe;
    let size = c.u32()? as usize;
    c.pascal()?;
    let n = c.u32()?;
    ensure!(n <= 16, "implausible method instrument count {n}");
    let mut instruments = Vec::new();
    for _ in 0..n {
        instruments.push((c.pascal()?, c.pascal()?));
    }
    let start = c.pos;
    c.skip(size)?;
    Ok(Some(MethodFile {
        instruments,
        ole: start..start + size,
    }))
}

fn file_header(c: &mut Cursor) -> Result<FileHeader> {
    let magic = c.u16()?;
    ensure!(
        magic == 0xa101,
        "not a Finnigan RAW file (magic {magic:#06x})"
    );
    let sig = c.utf16_fixed(18)?;
    ensure!(sig == "Finnigan", "unexpected signature {sig:?}");
    c.skip(16)?;
    let version = c.u32()?;
    if version != SUPPORTED_VERSION {
        bail!("unsupported RAW file version {version}");
    }
    // audit start: FILETIME, 2 x 50-byte tags, u32
    let created_filetime = c.u64()?;
    c.skip(104)?;
    c.skip(112)?; // audit end
    c.skip(4 + 60 + 1028)?;
    debug_assert_eq!(c.pos, 1356);
    Ok(FileHeader {
        version,
        created_filetime,
    })
}

/// Returns the sample id (third string; `SampleInformation.SampleId`).
fn seq_row(c: &mut Cursor) -> Result<String> {
    c.skip(4 + 4 + 4 + 12 + 5 * 8)?; // InjectionData
    let mut texts = Vec::new();
    for _ in 0..13 {
        texts.push(c.pascal()?);
    }
    for _ in 0..3 {
        c.pascal()?;
    }
    c.u32()?;
    for _ in 0..15 {
        c.pascal()?;
    }
    Ok(texts.swap_remove(2))
}

fn as_info(c: &mut Cursor) -> Result<()> {
    c.skip(24)?;
    c.pascal()?;
    Ok(())
}

/// RawFileInfoPreamble (fixed 1856 bytes in v66) plus five label headings and
/// one text. Returns the 64-bit data address and the controller table.
fn raw_file_info(c: &mut Cursor) -> Result<(u64, Vec<ControllerEntry>)> {
    let start = c.pos;
    c.skip(4 + 8 * 2)?; // method-file flag + SYSTEMTIME
    c.skip(8)?;
    let n_controllers = c.u32()?;
    let n_controllers2 = c.u32()?;
    ensure!(
        n_controllers == n_controllers2 && (1..=16).contains(&n_controllers),
        "unexpected controller count {n_controllers}/{n_controllers2}"
    );
    c.pos = start + 808;
    let data_addr = c.u64()?;
    let mut controllers = Vec::new();
    for _ in 0..n_controllers {
        let kind = c.u32()?;
        let index = c.u32()?;
        let run_header_addr = c.u64()?;
        controllers.push(ControllerEntry {
            kind,
            index,
            run_header_addr,
        });
    }
    c.pos = start + 1856;
    for _ in 0..6 {
        c.pascal()?;
    }
    Ok((data_addr, controllers))
}

fn run_header(c: &mut Cursor) -> Result<RunHeader> {
    let addr = c.pos as u64;
    // SampleInfo
    c.u32()?;
    c.u32()?;
    let first_scan = c.u32()?;
    let last_scan = c.u32()?;
    let inst_log_length = c.u32()?;
    let error_log_length = c.u32()?;
    c.skip(4 * 6)?;
    let max_ion_current = c.f64()?;
    let low_mz = c.f64()?;
    let high_mz = c.f64()?;
    let start_time = c.f64()?;
    let end_time = c.f64()?;
    c.skip(56)?;
    let tags = [c.utf16_fixed(88)?, c.utf16_fixed(40)?, c.utf16_fixed(320)?];
    let mut file_names = Vec::new();
    for _ in 0..6 {
        file_names.push(c.utf16_fixed(520)?);
    }
    c.skip(16)?;
    for _ in 0..7 {
        file_names.push(c.utf16_fixed(520)?);
    }
    c.skip(4 * 4)?;
    let nsegs = c.u32()?;
    c.skip(4 * 5)?;
    let scan_index_addr = c.u64()?;
    let data_addr = c.u64()?;
    let inst_log_addr = c.u64()?;
    let error_log_addr = c.u64()?;
    let unknown_addr = c.u64()?;
    let scan_trailer_addr = c.u64()?;
    let scan_params_addr = c.u64()?;
    c.skip(8)?;
    c.u64()?; // own addr
    let mut unknown_longs = Vec::new();
    for _ in 0..24 {
        unknown_longs.push(c.u32()?);
    }
    let inst_id = inst_id(c)?;
    let after_inst_id = c.pos;
    Ok(RunHeader {
        after_inst_id,
        addr,
        first_scan,
        last_scan,
        inst_log_length,
        error_log_length,
        max_ion_current,
        low_mz,
        high_mz,
        start_time,
        end_time,
        tags,
        file_names,
        nsegs,
        scan_index_addr,
        data_addr,
        inst_log_addr,
        error_log_addr,
        unknown_addr,
        scan_trailer_addr,
        scan_params_addr,
        unknown_longs,
        inst_id,
    })
}

fn inst_id(c: &mut Cursor) -> Result<InstId> {
    c.u32()?;
    c.u32()?;
    let n3 = c.u32()?;
    let mut models = vec![c.pascal()?, c.pascal()?];
    if n3 > 0 {
        models.push(c.pascal()?);
    }
    let serial_number = c.pascal()?;
    let software_version = c.pascal()?;
    let mut tags = Vec::new();
    for _ in 0..4 {
        tags.push(c.pascal()?);
    }
    Ok(InstId {
        models,
        serial_number,
        software_version,
        tags,
    })
}

/// One MS scan index entry (v66, 88 bytes).
#[derive(Debug, Clone)]
pub struct ScanIndexEntry {
    pub index: u32,
    pub scan_event: u16,
    pub scan_segment: u16,
    pub next: u32,
    pub unknown: u32,
    pub data_size: u32,
    pub start_time: f64,
    pub total_current: f64,
    pub base_intensity: f64,
    pub base_mz: f64,
    pub low_mz: f64,
    pub high_mz: f64,
    pub offset: u64,
    pub unknown2: [u32; 2],
}

impl RawFile {
    pub fn ms_scan_index(&self, rh: &RunHeader) -> Result<Vec<ScanIndexEntry>> {
        let n = (rh.last_scan + 1).saturating_sub(rh.first_scan) as usize;
        let mut c = Cursor::new(&self.bytes, rh.scan_index_addr as usize);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            c.u32()?;
            v.push(ScanIndexEntry {
                index: c.u32()?,
                scan_event: c.u16()?,
                scan_segment: c.u16()?,
                next: c.u32()?,
                unknown: c.u32()?,
                data_size: c.u32()?,
                start_time: c.f64()?,
                total_current: c.f64()?,
                base_intensity: c.f64()?,
                base_mz: c.f64()?,
                low_mz: c.f64()?,
                high_mz: c.f64()?,
                offset: c.u64()?,
                unknown2: [c.u32()?, c.u32()?],
            });
        }
        Ok(v)
    }
}

/// Structures hanging off an MS controller's run header.
#[derive(Debug)]
pub struct MsStreams {
    pub status_header: crate::generic::Header,
    pub templates: Vec<Vec<crate::events::ScanEvent>>,
    pub trailer_header: crate::generic::Header,
    pub events: Vec<crate::events::ScanEvent>,
    /// File offset of the first trailer-extra record.
    pub trailer_records: usize,
}

impl RawFile {
    pub fn ms_streams(&self, rh: &RunHeader) -> Result<MsStreams> {
        use crate::events::ScanEvent;
        use crate::generic::Header;
        let b = &self.bytes;
        let status_header = Header::read(&mut Cursor::new(b, rh.after_inst_id))?;

        let mut c = Cursor::new(b, rh.error_log_addr as usize);
        let n_err = c.u32()?;
        ensure!(n_err == rh.error_log_length, "error log length mismatch");
        for _ in 0..n_err {
            c.f32()?;
            c.pascal()?;
        }
        let nsegs = c.u32()?;
        ensure!(
            nsegs == rh.nsegs,
            "segment count mismatch ({nsegs} vs {})",
            rh.nsegs
        );
        let mut templates = Vec::new();
        for _ in 0..nsegs {
            let n = c.u32()?;
            let mut seg = Vec::new();
            for _ in 0..n {
                // template: preamble, u32 (0 in the corpus), mass ranges,
                // 16 bytes (zero in the corpus).
                let preamble: [u8; crate::events::PREAMBLE_LEN] =
                    c.take(crate::events::PREAMBLE_LEN)?.try_into().unwrap();
                let z = c.u32()?;
                ensure!(z == 0, "scan event template field {z}");
                let nr = c.u32()? as usize;
                ensure!(nr <= 64, "implausible template mass range count {nr}");
                let mut mass_ranges = Vec::with_capacity(nr);
                for _ in 0..nr {
                    mass_ranges.push((c.f64()?, c.f64()?));
                }
                c.skip(16)?;
                seg.push(ScanEvent {
                    preamble,
                    reactions: vec![],
                    mass_ranges,
                    conversion_params: vec![],
                    tail: vec![],
                });
            }
            templates.push(seg);
        }
        let trailer_header = Header::read(&mut c)?;

        let n = (rh.last_scan + 1).saturating_sub(rh.first_scan) as usize;
        let mut c = Cursor::new(b, rh.scan_trailer_addr as usize);
        let prefix = c.u32()?;
        ensure!(prefix == 0, "unexpected scan event stream prefix {prefix}");
        let mut events = Vec::with_capacity(n);
        for _ in 0..n {
            events.push(ScanEvent::read(&mut c)?);
        }
        ensure!(
            c.pos as u64 == rh.scan_params_addr,
            "scan event stream ends at {} but trailer records start at {}",
            c.pos,
            rh.scan_params_addr
        );
        let trailer_records = rh.scan_params_addr as usize;
        ensure!(
            trailer_records + n * trailer_header.record_size <= b.len(),
            "trailer records run past end of file"
        );
        Ok(MsStreams {
            status_header,
            templates,
            trailer_header,
            events,
            trailer_records,
        })
    }
}

/// One PDA scan index entry (72 bytes).
#[derive(Debug, Clone)]
pub struct PdaIndexEntry {
    pub scan: u32,
    pub kind: u32,
    pub start_time: f64,
    pub low: f64,
    pub high: f64,
    pub total: f64,
    /// Offset (from the controller data address) of the packet header; the
    /// intensity block precedes it.
    pub header_offset: u64,
    pub raw_head: [u32; 6],
    pub unknown_double: f64,
}

/// A decoded PDA scan.
#[derive(Debug, Clone)]
pub struct PdaScan {
    pub start: f64,
    pub step: f64,
    pub wl_first: f64,
    pub offset: f64,
    pub scale: f64,
    pub intensities: Vec<i32>,
    pub header_ints: [u32; 4],
}

impl RawFile {
    pub fn pda_index(&self, rh: &RunHeader) -> Result<Vec<PdaIndexEntry>> {
        let n = (rh.last_scan + 1).saturating_sub(rh.first_scan) as usize;
        let mut c = Cursor::new(&self.bytes, rh.scan_index_addr as usize);
        let mut v = Vec::with_capacity(n);
        for _ in 0..n {
            let raw_head = [c.u32()?, c.u32()?, c.u32()?, c.u32()?, c.u32()?, c.u32()?];
            v.push(PdaIndexEntry {
                scan: raw_head[1],
                kind: raw_head[2],
                unknown_double: c.f64()?,
                start_time: c.f64()?,
                low: c.f64()?,
                high: c.f64()?,
                total: c.f64()?,
                header_offset: c.u64()?,
                raw_head,
            });
        }
        Ok(v)
    }

    pub fn pda_scan(&self, rh: &RunHeader, e: &PdaIndexEntry) -> Result<PdaScan> {
        let hdr = (rh.data_addr + e.header_offset) as usize;
        let mut c = Cursor::new(&self.bytes, hdr);
        let header_ints = [c.u32()?, c.u32()?, c.u32()?, c.u32()?];
        let start = c.f64()?;
        let _end = c.f64()?;
        let step = c.f64()?;
        let wl_first = c.f64()?;
        let offset = c.f64()?;
        let scale = c.f64()?;
        let n = c.u32()? as usize;
        c.u32()?;
        let data_offset = c.u64()?;
        ensure!(
            data_offset + (n as u64) * 4 == e.header_offset,
            "PDA scan {}: data block does not end at the packet header",
            e.scan
        );
        let mut d = Cursor::new(&self.bytes, (rh.data_addr + data_offset) as usize);
        let intensities = (0..n).map(|_| d.i32()).collect::<Result<Vec<_>>>()?;
        Ok(PdaScan {
            start,
            step,
            wl_first,
            offset,
            scale,
            intensities,
            header_ints,
        })
    }
}

/// Packet types stored in [`ScanIndexEntry::unknown`] (corpus: Q Exactive FT
/// centroid, LTQ low-resolution centroid and profile).
pub const PACKET_LR_CENTROID: u32 = 18;
pub const PACKET_LR_PROFILE: u32 = 19;
pub const PACKET_FT_CENTROID: u32 = 20;

/// What a scan packet holds.
#[derive(Debug, Clone, PartialEq)]
pub enum Peaks {
    /// Peak list as stored (f32).
    Centroid { mz: Vec<f32>, intensity: Vec<f32> },
    /// Uniformly binned profile of one chunk starting at bin 0.
    Profile {
        first: f64,
        step: f64,
        intensity: Vec<f32>,
    },
}

impl RawFile {
    /// Decode an MS scan packet. Only the packet shapes in the corpus are
    /// accepted: FT centroid (layout 0x80), LR centroid and single-chunk LR
    /// profile (layout 0).
    pub fn ms_peaks(&self, rh: &RunHeader, e: &ScanIndexEntry) -> Result<Peaks> {
        let unsupported = |m: String| -> anyhow::Error { crate::Unsupported(m).into() };
        let start = (rh.data_addr + e.offset) as usize;
        let end = start + e.data_size as usize;
        let mut c = Cursor::new(&self.bytes, start);
        let _one = c.u32()?;
        let profile_size = c.u32()? as usize;
        let peak_list_size = c.u32()?;
        let layout = c.u32()?;
        // descriptors, unknown stream, triplets, unknown: FT packets carry
        // values here that do not affect the peak list; LR packets in the
        // corpus have zeros only.
        let words = [c.u32()?, c.u32()?, c.u32()?, c.u32()?];
        if e.unknown != PACKET_FT_CENTROID && words != [0; 4] {
            return Err(unsupported(format!(
                "LR scan packet header words {words:?}"
            )));
        }
        c.f32()?;
        c.f32()?;
        let shape = (e.unknown, profile_size != 0, peak_list_size != 0, layout);
        match shape {
            (PACKET_FT_CENTROID, false, true, 0x80) | (PACKET_LR_CENTROID, false, true, 0) => {}
            (PACKET_LR_PROFILE, true, false, 0) => {
                let first = c.f64()?;
                let step = c.f64()?;
                let nchunks = c.u32()?;
                let nbins = c.u32()? as usize;
                let first_bin = c.u32()?;
                let n = c.u32()? as usize;
                if nchunks != 1 || first_bin != 0 || n != nbins || profile_size != 8 + n {
                    return Err(unsupported(format!(
                        "profile packet with {nchunks} chunks (first bin {first_bin}, {n}/{nbins} bins)"
                    )));
                }
                let intensity = (0..n).map(|_| c.f32()).collect::<Result<Vec<_>>>()?;
                ensure!(c.pos == end, "profile packet size mismatch");
                return Ok(Peaks::Profile {
                    first,
                    step,
                    intensity,
                });
            }
            _ => {
                return Err(unsupported(format!(
                    "scan packet type {} (profile {profile_size}, peaks {peak_list_size}, layout {layout:#x})",
                    e.unknown
                )))
            }
        }
        let n = c.u32()? as usize;
        ensure!(
            peak_list_size as usize == 2 * n + 1,
            "peak list size mismatch"
        );
        let mut mz = Vec::with_capacity(n);
        let mut intensity = Vec::with_capacity(n);
        for _ in 0..n {
            mz.push(c.f32()?);
            intensity.push(c.f32()?);
        }
        ensure!(c.pos <= end, "peak list overruns packet");
        Ok(Peaks::Centroid { mz, intensity })
    }
}
