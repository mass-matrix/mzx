//! Builds the `MsData` model the way pwiz 4.x `Reader_Thermo` +
//! `SpectrumList_Thermo` + `ChromatogramList_Thermo` (@ fb75aff) do, for the
//! proven subset of RAW files. Anything else is [`Unsupported`].

use std::path::Path;

use anyhow::{ensure, Context, Result};
use mzx_mzml::float::double12;
use mzx_mzml::model::*;
use sha1::{Digest, Sha1};

use crate::filter::{Activation, Family, ScanFilter};
use crate::generic::Value;
use crate::model::{accession, translate, GENERIC};
use crate::raw::{Peaks, RawFile, RunHeader, ScanIndexEntry};
use crate::Unsupported;

/// `MSData.PwizVersion` (pwiz/data/msdata/MSData.cs).
pub const PWIZ_VERSION: &str = "3.0.26056";

/// Controller-table kinds observed in the corpus.
const CONTROLLER_MS: u32 = 0;
const CONTROLLER_PDA: u32 = 3;
const CONTROLLER_OTHER: u32 = 5;

// CV accessions used below.
const MS_LEVEL: &str = "MS:1000511";
const MZ: &str = "MS:1000040";
const COUNTS: &str = "MS:1000131";
const NM: &str = "UO:0000018";
const MINUTE: &str = "UO:0000031";

pub struct Options {
    /// pwiz `msLevelsToCentroid`: MS levels for which vendor-centroided peaks
    /// are preferred (`peakPicking true <levels>`). `None` = nofilter.
    pub centroid_ms_levels: Option<Box<dyn Fn(u32) -> bool>>,
}

impl Options {
    pub fn nofilter() -> Self {
        Self {
            centroid_ms_levels: None,
        }
    }
}

fn unsupported(msg: impl Into<String>) -> anyhow::Error {
    Unsupported(msg.into()).into()
}

fn cv(acc: &str) -> CvParam {
    CvParam::new(acc, "")
}

fn cvv(acc: &str, v: impl Into<String>) -> CvParam {
    CvParam::new(acc, v)
}

fn cvu(acc: &str, v: f64, unit: &str) -> CvParam {
    CvParam::with_unit(acc, double12(v), unit)
}

/// One MS scan as `SpectrumList_Thermo`'s index entry sees it.
struct MsScan {
    scan: u32,
    index: ScanIndexEntry,
    filter: ScanFilter,
    trailer_pos: usize,
}

struct Ctx<'a> {
    raw: &'a RawFile,
    ms_rh: &'a RunHeader,
    trailer: crate::generic::Header,
    scans: Vec<MsScan>,
    method: crate::method::MethodWidths,
    /// Scan-event templates per time segment.
    events_per_segment: Vec<usize>,
}

impl Ctx<'_> {
    /// `GetTrailerExtraValue` for a label, `None` when the label is absent.
    fn trailer(&self, s: &MsScan, label: &str) -> Result<Option<Value>> {
        match self.trailer.index_of(label) {
            None => Ok(None),
            Some(i) => Ok(Some(self.trailer.value(
                &self.raw.bytes,
                s.trailer_pos,
                i,
            )?)),
        }
    }

    /// `Convert.ToDouble(object)` for the value types proven so far (f32 and
    /// the integer types widen exactly).
    fn trailer_f64(&self, s: &MsScan, label: &str) -> Result<Option<f64>> {
        Ok(match self.trailer(s, label)? {
            None => None,
            Some(Value::F64(v)) => Some(v),
            Some(Value::F32(v)) => Some(v as f64),
            Some(Value::I32(v)) => Some(v as f64),
            Some(Value::I16(v)) => Some(v as f64),
            Some(v) => return Err(unsupported(format!("trailer {label:?} as double: {v:?}"))),
        })
    }

    /// `Convert.ToInt64(object)` for the value types proven so far.
    fn trailer_i64(&self, s: &MsScan, label: &str) -> Result<Option<i64>> {
        Ok(match self.trailer(s, label)? {
            None => None,
            Some(Value::I32(v)) => Some(v as i64),
            Some(Value::I16(v)) => Some(v as i64),
            Some(v) => return Err(unsupported(format!("trailer {label:?} as integer: {v:?}"))),
        })
    }

    /// `object.ToString()` for the value types proven so far.
    fn trailer_string(&self, s: &MsScan, label: &str) -> Result<Option<String>> {
        Ok(match self.trailer(s, label)? {
            None => None,
            Some(Value::I32(v)) => Some(v.to_string()),
            Some(Value::I16(v)) => Some(v.to_string()),
            Some(Value::Str(v)) => Some(v),
            Some(v) => return Err(unsupported(format!("trailer {label:?} as string: {v:?}"))),
        })
    }

    /// `ThermoRawFile.GetPeaks(scan, preferCentroid)`.
    fn peaks(&self, s: &MsScan, prefer_centroid: bool) -> Result<(Vec<f64>, Vec<f64>)> {
        // mzx models Thermo's ion-trap centroider only
        let itms = s.filter.analyzer == crate::filter::Analyzer::Itms;
        match self.raw.ms_peaks(self.ms_rh, &s.index)? {
            // FTMS centroid packets: the segmented scan and the centroid
            // stream are the same peak list, so preferCentroid does not
            // change the data. LR centroid scans fall through to the
            // segmented scan.
            Peaks::Centroid { mz, intensity } => Ok((
                mz.into_iter().map(f64::from).collect(),
                intensity.into_iter().map(f64::from).collect(),
            )),
            Peaks::Profile { .. } if prefer_centroid && !itms => Err(unsupported(
                "vendor centroiding (Scan.ToCentroid) of a profile scan that is not ion-trap",
            )),
            Peaks::Profile {
                first,
                step,
                intensity,
            } => {
                // pwiz centroids only a scan whose segment has at least two
                // points (CanFragmentEverySegment), and falls back to the
                // segmented scan (the profile) when ToCentroid finds no peaks.
                if prefer_centroid && intensity.len() >= 2 {
                    if step != crate::centroid::STEP {
                        return Err(unsupported(format!(
                            "ion-trap centroiding of a profile with step {step} \
                             (only 1/12 m/z is modeled)"
                        )));
                    }
                    let (mz, y) = crate::centroid::centroid(first, step, &intensity);
                    if !mz.is_empty() {
                        return Ok((mz, y));
                    }
                }
                Ok((
                    (1..=intensity.len())
                        .map(|k| first + k as f64 * step)
                        .collect(),
                    intensity.into_iter().map(f64::from).collect(),
                ))
            }
        }
    }
}

pub fn read(path: &Path, opts: &Options) -> Result<MsData> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let sha1 = Sha1::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    // Thermo's reader refuses a file whose header checksum does not match
    // ("IsError"), so msconvert produces nothing for it.
    if crate::raw::stored_checksum(&bytes) != Some(crate::raw::compute_checksum(&bytes)) {
        return Err(Unsupported("file header checksum mismatch".into()).into());
    }
    let raw = RawFile::parse(bytes)?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .context("file name")?
        .to_string();
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .context("file stem")?
        .to_string();
    let dir = std::fs::canonicalize(path)?
        .parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default();

    // ---- controllers ----
    let mut ms_i = None;
    let mut pda_i = None;
    for (i, c) in raw.controllers.iter().enumerate() {
        match c.kind {
            CONTROLLER_MS if ms_i.is_none() => ms_i = Some(i),
            CONTROLLER_PDA if pda_i.is_none() => pda_i = Some(i),
            CONTROLLER_OTHER if raw.run_headers[i].last_scan == 0 => {}
            k => return Err(unsupported(format!("controller kind {k} (#{i})"))),
        }
    }
    let ms_i = ms_i.ok_or_else(|| unsupported("no MS controller"))?;
    let ms_rh = &raw.run_headers[ms_i];

    // Instrument gate first: stream layouts are only proven for these models.
    let inst = &ms_rh.inst_id;
    let model_prop = inst.models.first().cloned().unwrap_or_default();
    let model_cvid = translate(&model_prop);
    if model_cvid == GENERIC {
        return Err(unsupported(format!("instrument model {model_prop:?}")));
    }
    let (family, recipe) = recipe(model_cvid)?;

    let streams = raw.ms_streams(ms_rh)?;
    let index = raw.ms_scan_index(ms_rh)?;
    ensure!(
        streams.events.len() == index.len(),
        "event/index count mismatch"
    );

    let mut scans = Vec::with_capacity(index.len());
    for (i, (e, ev)) in index.into_iter().zip(&streams.events).enumerate() {
        let filter = ScanFilter::decode(ev, family).with_context(|| format!("scan {}", i + 1))?;
        scans.push(MsScan {
            scan: ms_rh.first_scan + i as u32,
            index: e,
            filter,
            trailer_pos: streams.trailer_records + i * streams.trailer_header.record_size,
        });
    }
    // Instrument method text (+ status-log lines, which pwiz appends: refuse
    // labels that could feed its parser).
    for f in &streams.status_header.fields {
        let l = f.label.to_ascii_lowercase();
        if (l.trim().is_empty() && f.kind != 0)
            || [
                "segment",
                "scan event",
                "isolation",
                "isow",
                "instrument model",
                "data dependent",
                "diascan",
            ]
            .iter()
            .any(|k| l.contains(k))
        {
            return Err(unsupported(format!("status log label {:?}", f.label)));
        }
    }
    let method = crate::method::parse(&raw.instrument_methods()?.join("\n"));
    if !method.instrument_model.is_empty() {
        return Err(unsupported(format!(
            "instrument model from method text {:?}",
            method.instrument_model
        )));
    }
    let ctx = Ctx {
        raw: &raw,
        ms_rh,
        trailer: streams.trailer_header,
        scans,
        method,
        events_per_segment: streams.templates.iter().map(Vec::len).collect(),
    };

    let mut msd = MsData {
        id: stem.clone(),
        version: "1.1.0".into(),
        cvs: vec![
            Cv {
                id: "MS".into(),
                full_name: "Proteomics Standards Initiative Mass Spectrometry Ontology".into(),
                version: "4.1.257".into(),
                uri: "http://purl.obolibrary.org/obo/ms/psi-ms.obo".into(),
            },
            Cv {
                id: "UO".into(),
                full_name: "Unit Ontology".into(),
                version: "releases/2026-01-16".into(),
                uri: "http://purl.obolibrary.org/obo/uo.obo".into(),
            },
        ],
        ..Default::default()
    };

    // ---- source file ----
    msd.file_description.source_files.push(SourceFile {
        id: "RAW1".into(),
        name: file_name,
        location: format!("file:///{dir}"),
        params: Params {
            cv: vec![cv("MS:1000768"), cv("MS:1000563"), cvv("MS:1000569", sha1)],
            ..Default::default()
        },
    });

    // ---- sample: one entry named by the SampleId (pwiz cpp parity) ----
    if !raw.sample_id.is_empty() {
        msd.samples.push(Sample {
            id: raw.sample_id.clone(),
            name: String::new(),
            params: Params {
                cv: vec![cvv("MS:1000002", raw.sample_id.clone())],
                ..Default::default()
            },
        });
    }

    // ---- instrument (Reader_Thermo.FillInstrumentConfiguration) ----
    msd.software.push(Software {
        id: "Xcalibur".into(),
        version: inst.software_version.clone(),
        params: Params {
            cv: vec![cv("MS:1000532")],
            ..Default::default()
        },
    });
    let mut common = Params {
        cv: vec![cv(accession(model_cvid)?)],
        ..Default::default()
    };
    if !inst.serial_number.is_empty() {
        common
            .cv
            .push(cvv("MS:1000529", inst.serial_number.clone()));
    }
    msd.param_groups.push(ParamGroup {
        id: "CommonInstrumentParams".into(),
        params: common,
    });
    let group = Params {
        group_refs: vec!["CommonInstrumentParams".into()],
        ..Default::default()
    };

    // Source/inlet from the first scan's ionization: ESI is the only one proven.
    let mut components = vec![Component {
        kind: ComponentKind::Source,
        order: 1,
        params: Params {
            cv: vec![cv("MS:1000073"), cv("MS:1000057")],
            ..Default::default()
        },
    }];
    for (n, (kind, acc)) in recipe.iter().enumerate() {
        components.push(Component {
            kind: kind.clone(),
            order: n as i32 + 2,
            params: Params {
                cv: vec![cv(acc)],
                ..Default::default()
            },
        });
    }
    msd.instrument_configurations.push(InstrumentConfiguration {
        id: "IC1".into(),
        params: group.clone(),
        components,
        software_ref: Some("Xcalibur".into()),
    });
    if pda_i.is_some() {
        msd.instrument_configurations.push(InstrumentConfiguration {
            id: "IC2".into(),
            params: group.clone(),
            components: vec![Component {
                kind: ComponentKind::Detector,
                order: 1,
                params: Params {
                    cv: vec![cv("MS:1000621")],
                    ..Default::default()
                },
            }],
            software_ref: Some("Xcalibur".into()),
        });
    }

    msd.software.push(Software {
        id: "pwiz".into(),
        version: PWIZ_VERSION.into(),
        params: Params {
            cv: vec![cv("MS:1000615")],
            ..Default::default()
        },
    });
    msd.data_processing.push(DataProcessing {
        id: "pwiz_Reader_Thermo_conversion".into(),
        methods: vec![ProcessingMethod {
            order: 0,
            software_ref: Some("pwiz".into()),
            params: Params {
                cv: vec![cv("MS:1000544")],
                ..Default::default()
            },
        }],
    });

    // ---- run ----
    msd.run.id = stem;
    msd.run.default_instrument_configuration_ref = Some("IC1".into());
    msd.run.default_source_file_ref = Some("RAW1".into());
    msd.run.start_time_stamp = start_time_stamp(raw.header.created_filetime);
    msd.run.spectrum_default_data_processing_ref = Some("pwiz_Reader_Thermo_conversion".into());
    msd.run.chromatogram_default_data_processing_ref = Some("pwiz_Reader_Thermo_conversion".into());

    // ---- spectra ----
    let mut n_ms1 = 0;
    let mut n_ms2 = 0;
    for i in 0..ctx.scans.len() {
        let s = ms_spectrum(&ctx, i, opts)?;
        match ctx.scans[i].filter.ms_order {
            1 => n_ms1 += 1,
            _ => n_ms2 += 1,
        }
        msd.run.spectra.push(s);
    }
    let mut pda = None;
    if let Some(pi) = pda_i {
        let rh = &raw.run_headers[pi];
        let idx = raw.pda_index(rh)?;
        for e in &idx {
            let s = pda_spectrum(&raw, rh, e, msd.run.spectra.len())?;
            msd.run.spectra.push(s);
        }
        pda = Some((rh, idx));
    }

    // ---- fileContent (Reader_Thermo.FillFileContent) ----
    let fc = &mut msd.file_description.file_content.cv;
    if n_ms1 > 0 {
        fc.push(cv("MS:1000579"));
    }
    if n_ms2 > 0 {
        fc.push(cv("MS:1000580"));
    }
    if pda.as_ref().is_some_and(|(_, idx)| !idx.is_empty()) {
        fc.push(cv("MS:1000804"));
    }

    // ---- chromatograms ----
    msd.run.chromatograms.push(tic_chromatogram(&ctx)?);
    if let Some((rh, idx)) = &pda {
        msd.run.chromatograms.push(pda_chromatogram(&raw, rh, idx)?);
    }
    Ok(msd)
}

/// `Reader_Thermo_Detail` instrument-configuration recipe: analyzer/detector
/// components after the source, and the scan-event family whose layout is
/// proven for the model. Only models in the corpus.
fn recipe(model_cvid: &str) -> Result<(Family, Vec<(ComponentKind, &'static str)>)> {
    Ok(match model_cvid {
        // OrbitrapQuadHybridRecipe
        "MS_Q_Exactive_Focus" => (
            Family::QExactive,
            vec![
                (ComponentKind::Analyzer, "MS:1000081"), // quadrupole
                (ComponentKind::Analyzer, "MS:1000484"), // orbitrap
                (ComponentKind::Detector, "MS:1000624"), // inductive detector
            ],
        ),
        // LinearIonTrapRecipe
        "MS_LTQ_XL" => (
            Family::LtqXl,
            vec![
                (ComponentKind::Analyzer, "MS:1000083"), // radial ejection linear ion trap
                (ComponentKind::Detector, "MS:1000253"), // electron multiplier
            ],
        ),
        m => return Err(unsupported(format!("instrument recipe for {m}"))),
    })
}

/// `ReaderConfig.FormatStartTimeStamp` with the SDK's `CreationDate`
/// (FILETIME -> local wall clock) on a UTC host (the msconvert container).
fn start_time_stamp(filetime: u64) -> String {
    let secs = (filetime / 10_000_000) as i64 - 11_644_473_600;
    let days = secs.div_euclid(86_400);
    let tod = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        tod / 3600,
        tod / 60 % 60,
        tod % 60
    )
}

/// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn float_array(kind: &str, unit: &str, data: Vec<f64>) -> BinaryDataArray {
    BinaryDataArray {
        params: Params {
            cv: vec![CvParam::with_unit(kind, "", unit)],
            ..Default::default()
        },
        data_processing_ref: None,
        data: ArrayData::Float(data),
    }
}

fn ms_spectrum(ctx: &Ctx, i: usize, opts: &Options) -> Result<Spectrum> {
    let s = &ctx.scans[i];
    let f = &s.filter;
    let ms_level = f.ms_order;
    let prefer_centroid = opts
        .centroid_ms_levels
        .as_ref()
        .is_some_and(|p| p(ms_level));
    let mut spec = Spectrum {
        index: i,
        id: format!("controllerType=0 controllerNumber=1 scan={}", s.scan),
        ..Default::default()
    };
    let p = &mut spec.params.cv;
    p.push(cv(if ms_level == 1 {
        "MS:1000579"
    } else {
        "MS:1000580"
    }));
    p.push(cvv(MS_LEVEL, ms_level.to_string()));

    // ---- scan ----
    let mut scan = Scan {
        instrument_configuration_ref: Some("IC1".into()),
        ..Default::default()
    };
    let sp = &mut scan.params;
    sp.cv.push(cvu("MS:1000016", s.index.start_time, MINUTE));
    let mut resolving_power = 0;
    if let Some(v) = ctx
        .trailer_i64(s, "Orbitrap Resolution:")?
        .filter(|&v| v > 0)
    {
        resolving_power = v;
    } else if let Some(v) = ctx.trailer_i64(s, "FT Resolution:")?.filter(|&v| v > 0) {
        resolving_power = v;
    }
    if resolving_power > 0 {
        sp.cv.push(cvv("MS:1000800", resolving_power.to_string()));
    }
    let filter_string = f.to_string();
    sp.cv.push(cvv("MS:1000512", filter_string));
    if let Some(ev) = ctx
        .trailer_string(s, "Scan Event:")?
        .filter(|v| !v.is_empty())
    {
        sp.cv.push(cvv("MS:1000616", ev));
    }
    if ctx.trailer(s, "Scan Description:")?.is_some() {
        return Err(unsupported("Scan Description trailer"));
    }
    if ms_level > 1 {
        let mono = ctx.trailer_f64(s, "Monoisotopic M/Z:")?.unwrap_or(0.0);
        sp.user.push(UserParam {
            name: "[Thermo Trailer Extra]Monoisotopic M/Z:".into(),
            value: g17(mono),
            type_: "xsd:float".into(),
            unit: None,
        });
    }
    let inj = ctx
        .trailer_f64(s, "Ion Injection Time (ms):")?
        .unwrap_or(0.0);
    sp.cv.push(cvu("MS:1000927", inj, "UO:0000028"));

    // ---- polarity, centroid ----
    p.push(cv(if f.positive {
        "MS:1000130"
    } else {
        "MS:1000129"
    }));
    p.push(cv(if f.centroid || prefer_centroid {
        "MS:1000127"
    } else {
        "MS:1000128"
    }));

    // ---- scan stats ----
    let e = &s.index;
    p.push(cvu("MS:1000504", e.base_mz, MZ));
    p.push(cvu("MS:1000505", e.base_intensity, COUNTS));
    p.push(cvv("MS:1000285", double12(e.total_current)));
    scan.scan_windows.push(Params {
        cv: vec![
            cvu("MS:1000501", e.low_mz, MZ),
            cvu("MS:1000500", e.high_mz, MZ),
        ],
        ..Default::default()
    });
    spec.scan_list.params.cv.push(cv("MS:1000795"));
    spec.scan_list.scans.push(scan);

    // ---- precursor ----
    if ms_level > 1 {
        if ctx.trailer(s, "SPS Masses:")?.is_some() {
            return Err(unsupported("SPS trailer"));
        }
        populate_precursor(ctx, i, &mut spec, prefer_centroid)?;
    }

    // ---- data ----
    let (mz, y) = ctx.peaks(s, prefer_centroid)?;
    spec.default_array_length = mz.len();
    if let (Some(first), Some(last)) = (mz.first(), mz.last()) {
        spec.params.cv.push(cvu("MS:1000528", *first, MZ));
        spec.params.cv.push(cvu("MS:1000527", *last, MZ));
    }
    spec.arrays.push(float_array("MS:1000514", MZ, mz));
    spec.arrays.push(float_array("MS:1000515", COUNTS, y));
    Ok(spec)
}

/// cpp `lexical_cast<string>(double)`: 17 significant digits ("G17").
fn g17(v: f64) -> String {
    if v == 0.0 {
        return "0".into();
    }
    // .NET "G17": scientific when exponent < -5 or >= 17.
    let exp = v.abs().log10().floor() as i32;
    if !(-5..17).contains(&exp) {
        return format!("{v:.16E}");
    }
    let decimals = (16 - exp).max(0) as usize;
    let s = format!("{v:.decimals$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

fn populate_precursor(
    ctx: &Ctx,
    i: usize,
    spec: &mut Spectrum,
    prefer_centroid: bool,
) -> Result<()> {
    let s = &ctx.scans[i];
    let f = &s.filter;
    let ms_level = f.ms_order;
    ensure!(
        ms_level == 2 && f.precursors.len() == 1,
        "only single-precursor MS2 is proven"
    );
    let fp = &f.precursors[0];
    let isolation_mz = fp.mz;
    if isolation_mz <= 0.0 {
        return Ok(());
    }
    // Primary precursor: trailer width overrides the filter width unconditionally.
    let width = ctx
        .trailer_f64(s, &format!("MS{ms_level} Isolation Width:"))?
        .unwrap_or(0.0);
    let mut half = width / 2.0;
    if half == 0.0 {
        let w = method_isolation_width(ctx, s, ms_level)?;
        if w > 0.0 {
            half = w / 2.0;
        }
    }
    let mut pre = Precursor::default();
    let iw = &mut pre.isolation_window;
    iw.cv.push(cvu("MS:1000827", isolation_mz, MZ));
    if half > 0.0 {
        iw.cv.push(cvu("MS:1000828", half, MZ));
        iw.cv.push(cvu("MS:1000829", half, MZ));
    }
    iw.user.push(UserParam {
        name: "ms level".into(),
        value: (ms_level - 1).to_string(),
        ..Default::default()
    });

    let charge = ctx
        .trailer_i64(s, "Charge State:")?
        .filter(|&c| c > 0)
        .unwrap_or(0);
    let mut selected_mz = isolation_mz;
    if let Some(m) = ctx
        .trailer_f64(s, "Monoisotopic M/Z:")?
        .filter(|&m| m > 0.0)
    {
        selected_mz = m;
    }
    // No zoom scans in the proven corpus (FindPrecursorZoomScan returns -1).
    let (lo, hi) = if half <= 2.0 {
        (isolation_mz - 1.5 * 2.0, isolation_mz + 2.5)
    } else {
        (isolation_mz - half, isolation_mz + half)
    };
    if selected_mz < lo || selected_mz > hi {
        selected_mz = isolation_mz;
    }
    let mut ion = Params::default();
    if selected_mz > 0.0 {
        ion.cv.push(cvu("MS:1000744", selected_mz, MZ));
    }
    if charge > 0 {
        ion.cv.push(cvv("MS:1000041", charge.to_string()));
    }

    if ctx.trailer(s, "Master Scan Number:")?.is_some() {
        return Err(unsupported("Master Scan Number trailer"));
    }
    if let Some(j) = find_precursor_index(ctx, i, isolation_mz) {
        let ps = &ctx.scans[j];
        pre.spectrum_ref = format!("controllerType=0 controllerNumber=1 scan={}", ps.scan);
        let (mz, y) = ctx.peaks(ps, prefer_centroid)?;
        let q = if half > 0.0 { 1.5 } else { 0.0 };
        let (lo, hi) = (isolation_mz - q, isolation_mz + q);
        let start = mz.partition_point(|&m| m < lo);
        let mut sum = 0.0;
        for k in start..mz.len() {
            if mz[k] >= hi {
                break;
            }
            sum += y[k];
        }
        if sum > 0.0 {
            ion.cv.push(cvu("MS:1000042", sum, COUNTS));
        }
    }
    pre.selected_ions.push(ion);

    pre.activation.cv.push(cv(match fp.activation {
        Activation::Cid => "MS:1000133",
        Activation::Hcd => "MS:1000422",
    }));
    pre.activation
        .cv
        .push(cvu("MS:1000045", fp.energy, "UO:0000266"));
    spec.precursors.push(pre);
    Ok(())
}

/// Method fallback for the primary precursor's isolation width
/// (`GetMethodIsolationWidth`, then `GetMethodDefaultIsolationWidth`), keyed
/// by the SDK's 1-based `(SegmentNumber, ScanEventNumber)`.
///
/// The LTQ golden shows that only scans of the first time segment receive the
/// method width, although every method segment lists one: the SDK's scan
/// event number is evidently not the per-segment index-entry event. Two
/// readings fit the golden: the event number counts across segments (with the
/// segment number either the index segment or 0). Scans of the first segment
/// are the same under both; for later segments we compute both lookups and
/// refuse the scan if they disagree.
fn method_isolation_width(ctx: &Ctx, s: &MsScan, ms_level: u32) -> Result<f64> {
    let seg = s.index.scan_segment as usize;
    let evt = s.index.scan_event as i32;
    ensure!(
        seg < ctx.events_per_segment.len(),
        "scan segment {seg} out of range"
    );
    let global = ctx.events_per_segment[..seg].iter().sum::<usize>() as i32 + evt;
    let lookup = |segment: i32| {
        let w = ctx.method.event_width(segment, global + 1);
        if w != 0.0 {
            w
        } else {
            ctx.method.default_width(segment, ms_level as i32)
        }
    };
    let (a, b) = (lookup(seg as i32 + 1), lookup(1));
    if a != b {
        return Err(unsupported(format!(
            "method isolation width for segment {} event {} is ambiguous ({a} vs {b})",
            seg + 1,
            evt + 1
        )));
    }
    Ok(a)
}

/// `FindPrecursorIndex` without a master scan: the closest earlier MS1 whose
/// scan-statistics range brackets the isolation m/z.
fn find_precursor_index(ctx: &Ctx, from: usize, isolation_mz: f64) -> Option<usize> {
    (0..from).rev().find(|&j| {
        let p = &ctx.scans[j];
        p.filter.ms_order == 1 && isolation_mz >= p.index.low_mz && isolation_mz <= p.index.high_mz
    })
}

fn pda_spectrum(
    raw: &RawFile,
    rh: &RunHeader,
    e: &crate::raw::PdaIndexEntry,
    index: usize,
) -> Result<Spectrum> {
    let mut spec = Spectrum {
        index,
        id: format!("controllerType=4 controllerNumber=1 scan={}", e.scan),
        ..Default::default()
    };
    spec.params.cv.push(cv("MS:1000804"));
    spec.scan_list.params.cv.push(cv("MS:1000795"));
    let mut scan = Scan {
        instrument_configuration_ref: Some("IC2".into()),
        ..Default::default()
    };
    scan.params.cv.push(cvu("MS:1000016", e.start_time, MINUTE));
    // SDK scan statistics for PDA scans: no base peak, TIC = index total,
    // Low/HighMass = 0.
    spec.params.cv.push(cvv("MS:1000285", double12(e.total)));
    scan.scan_windows.push(Params {
        cv: vec![cvu("MS:1000501", 0.0, NM), cvu("MS:1000500", 0.0, NM)],
        ..Default::default()
    });
    spec.scan_list.scans.push(scan);

    let p = raw.pda_scan(rh, e)?;
    if p.offset != 0.0 || p.scale != 1.0 || p.wl_first != p.start {
        return Err(unsupported(format!(
            "PDA scan {} with offset {} scale {} first {}",
            e.scan, p.offset, p.scale, p.wl_first
        )));
    }
    let mut wl = Vec::with_capacity(p.intensities.len());
    let mut w = p.start;
    for k in 0..p.intensities.len() {
        if k > 0 {
            w += p.step;
        }
        wl.push(w);
    }
    let y: Vec<f64> = p.intensities.iter().map(|&v| v as f64).collect();
    spec.default_array_length = wl.len();
    if let (Some(first), Some(last)) = (wl.first(), wl.last()) {
        spec.params.cv.push(cvu("MS:1000619", *first, NM));
        spec.params.cv.push(cvu("MS:1000618", *last, NM));
    }
    if !wl.is_empty() {
        spec.arrays.push(float_array("MS:1000617", NM, wl));
        spec.arrays.push(float_array("MS:1000515", COUNTS, y));
    }
    Ok(spec)
}

fn tic_chromatogram(ctx: &Ctx) -> Result<Chromatogram> {
    let times: Vec<f64> = ctx.scans.iter().map(|s| s.index.start_time).collect();
    if times.windows(2).any(|w| w[1] <= w[0]) {
        return Err(unsupported("non-increasing MS retention times"));
    }
    let tic: Vec<f64> = ctx.scans.iter().map(|s| s.index.total_current).collect();
    let levels: Vec<i64> = ctx.scans.iter().map(|s| s.filter.ms_order as i64).collect();
    Ok(Chromatogram {
        index: 0,
        id: "TIC".into(),
        default_array_length: times.len(),
        params: Params {
            cv: vec![cv("MS:1000235")],
            ..Default::default()
        },
        arrays: vec![
            float_array("MS:1000595", MINUTE, times),
            float_array("MS:1000515", COUNTS, tic),
            BinaryDataArray {
                params: Params {
                    cv: vec![CvParam::with_unit("MS:1000786", "ms level", "UO:0000186")],
                    ..Default::default()
                },
                data_processing_ref: None,
                data: ArrayData::Int(levels),
            },
        ],
        ..Default::default()
    })
}

/// `TraceType.TotalAbsorbance`: the scan's total divided by its point count
/// (mean absorbance), at each PDA scan time.
fn pda_chromatogram(
    raw: &RawFile,
    rh: &RunHeader,
    idx: &[crate::raw::PdaIndexEntry],
) -> Result<Chromatogram> {
    let times: Vec<f64> = idx.iter().map(|e| e.start_time).collect();
    let mut y = Vec::with_capacity(idx.len());
    for e in idx {
        let n = raw.pda_scan(rh, e)?.intensities.len();
        ensure!(n > 0, "empty PDA scan {}", e.scan);
        y.push(e.total / n as f64);
    }
    Ok(Chromatogram {
        index: 1,
        id: "PDA 1".into(),
        default_array_length: times.len(),
        params: Params {
            cv: vec![cv("MS:1000812")],
            ..Default::default()
        },
        arrays: vec![
            float_array("MS:1000595", MINUTE, times),
            float_array("MS:1000515", "UO:0000269", y),
        ],
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_checksum_is_adler32_from_zero_without_its_field() {
        // zlib.adler32(data with bytes 148..152 zeroed, 0)
        let mut b: Vec<u8> = (0..=255u8).chain(0..=255u8).collect();
        assert_eq!(crate::raw::compute_checksum(&b), 0x927dfcaa);
        b[148..152].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        assert_eq!(crate::raw::compute_checksum(&b), 0x927dfcaa);
        assert_eq!(crate::raw::stored_checksum(&b), Some(0xdead_beef));
    }

    #[test]
    fn g17_matches_dotnet() {
        assert_eq!(g17(158.9614054609623), "158.96140546096231");
        assert_eq!(g17(0.0), "0");
    }

    #[test]
    fn timestamp() {
        assert_eq!(start_time_stamp(131949969478427609), "2019-02-18T20:55:47Z");
    }
}
