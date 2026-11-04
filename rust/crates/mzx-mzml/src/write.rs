//! Port of the mzML write path of pwiz (non-indexed output) in two dialects:
//! - [`Dialect::Pwiz3Cpp`]: `pwiz/data/msdata/IO.cpp` + `Serializer_mzML.cpp` (3.0.25182)
//! - [`Dialect::Pwiz4Net`]: `pwiz/data/msdata/MzmlWriter.cs` + `MzmlXml.cs` (4.0.26299, fb75aff)
//!
//! Function order and element/attribute order follow the reference sources so
//! they can be diffed by eye; deviations would break the byte contract. Where
//! the C# writer silently drops something the model can hold (precursor params,
//! scanSettings), the pwiz 4 path refuses instead.

use std::io::Write;

use anyhow::{anyhow, bail, Result};

use crate::cv;
use crate::dialect::Dialect;
use crate::encode::{self, Compression, EncoderConfig, Precision};
use crate::model::*;
use crate::xmlwriter::{encode_xml_id as enc, XmlWriter, STYLE_INLINE_INNER};

type W<'a, O> = &'a mut XmlWriter<O>;

fn a(k: &'static str, v: impl ToString) -> (&'static str, String) {
    (k, v.to_string())
}

fn write_cv_param<O: Write>(w: W<O>, p: &CvParam) -> Result<()> {
    let d = w.dialect();
    let name =
        cv::name(d, &p.accession).ok_or_else(|| anyhow!("unknown CV term {}", p.accession))?;
    let mut attrs = vec![
        a("cvRef", cv::prefix(&p.accession)),
        a("accession", &p.accession),
        a("name", name),
        a("value", &p.value),
    ];
    if let Some(u) = &p.unit {
        let uname = cv::name(d, u).ok_or_else(|| anyhow!("unknown unit term {u}"))?;
        attrs.push(a("unitCvRef", cv::prefix(u)));
        attrs.push(a("unitAccession", u));
        attrs.push(a("unitName", uname));
    }
    w.empty("cvParam", &attrs)?;
    Ok(())
}

fn write_user_param<O: Write>(w: W<O>, p: &UserParam) -> Result<()> {
    let mut attrs = vec![a("name", &p.name)];
    if !p.value.is_empty() {
        attrs.push(a("value", &p.value));
    }
    if !p.type_.is_empty() {
        attrs.push(a("type", &p.type_));
    }
    if let Some(u) = &p.unit {
        let uname = cv::name(w.dialect(), u).ok_or_else(|| anyhow!("unknown unit term {u}"))?;
        if w.dialect() == Dialect::Pwiz4Net {
            attrs.push(a("unitCvRef", cv::prefix(u))); // MzmlXml.WriteUserParam
        }
        attrs.push(a("unitAccession", u));
        attrs.push(a("unitName", uname));
    }
    w.empty("userParam", &attrs)?;
    Ok(())
}

fn write_params<O: Write>(w: W<O>, p: &Params) -> Result<()> {
    for r in &p.group_refs {
        // pwiz writeParamGroupRef: not id-encoded
        w.empty("referenceableParamGroupRef", &[a("ref", r)])?;
    }
    for c in &p.cv {
        write_cv_param(w, c)?;
    }
    for u in &p.user {
        write_user_param(w, u)?;
    }
    Ok(())
}

fn write_named_params<O: Write>(w: W<O>, tag: &'static str, p: &Params) -> Result<()> {
    w.start(tag, &[])?;
    write_params(w, p)?;
    w.end_element()?;
    Ok(())
}

fn write_list<O: Write, T>(
    w: W<O>,
    items: &[T],
    label: &'static str,
    f: impl Fn(W<O>, &T) -> Result<()>,
) -> Result<()> {
    if !items.is_empty() {
        w.start(label, &[a("count", items.len())])?;
        for it in items {
            f(w, it)?;
        }
        w.end_element()?;
    }
    Ok(())
}

fn write_cv<O: Write>(w: W<O>, c: &Cv) -> Result<()> {
    w.empty(
        "cv",
        &[
            a("id", enc(&c.id)),
            a("fullName", &c.full_name),
            a("version", &c.version),
            a("URI", &c.uri),
        ],
    )?;
    Ok(())
}

fn write_source_file<O: Write>(w: W<O>, sf: &SourceFile) -> Result<()> {
    w.start(
        "sourceFile",
        &[
            a("id", enc(&sf.id)),
            a("name", &sf.name),
            a("location", &sf.location),
        ],
    )?;
    write_params(w, &sf.params)?;
    w.end_element()?;
    Ok(())
}

fn write_file_description<O: Write>(w: W<O>, fd: &FileDescription) -> Result<()> {
    w.start("fileDescription", &[])?;
    write_named_params(w, "fileContent", &fd.file_content)?;
    write_list(w, &fd.source_files, "sourceFileList", write_source_file)?;
    for c in &fd.contacts {
        write_named_params(w, "contact", c)?;
    }
    w.end_element()?;
    Ok(())
}

fn write_param_group<O: Write>(w: W<O>, g: &ParamGroup) -> Result<()> {
    w.start("referenceableParamGroup", &[a("id", enc(&g.id))])?;
    write_params(w, &g.params)?;
    w.end_element()?;
    Ok(())
}

fn write_sample<O: Write>(w: W<O>, s: &Sample) -> Result<()> {
    w.start("sample", &[a("id", enc(&s.id)), a("name", &s.name)])?;
    write_params(w, &s.params)?;
    w.end_element()?;
    Ok(())
}

fn write_software<O: Write>(w: W<O>, s: &Software) -> Result<()> {
    w.start("software", &[a("id", enc(&s.id)), a("version", &s.version)])?;
    write_params(w, &s.params)?;
    w.end_element()?;
    Ok(())
}

fn write_scan_settings<O: Write>(w: W<O>, s: &ScanSettings) -> Result<()> {
    w.start("scanSettings", &[a("id", enc(&s.id))])?;
    if !s.source_file_refs.is_empty() {
        w.start("sourceFileRefList", &[a("count", s.source_file_refs.len())])?;
        for r in &s.source_file_refs {
            w.empty("sourceFileRef", &[a("ref", enc(r))])?;
        }
        w.end_element()?;
    }
    if !s.targets.is_empty() {
        w.start("targetList", &[a("count", s.targets.len())])?;
        for t in &s.targets {
            write_named_params(w, "target", t)?;
        }
        w.end_element()?;
    }
    w.end_element()?;
    Ok(())
}

fn write_instrument_configuration<O: Write>(w: W<O>, ic: &InstrumentConfiguration) -> Result<()> {
    w.start("instrumentConfiguration", &[a("id", enc(&ic.id))])?;
    write_params(w, &ic.params)?;
    if !ic.components.is_empty() {
        w.start("componentList", &[a("count", ic.components.len())])?;
        for c in &ic.components {
            w.start(c.kind.tag(), &[a("order", c.order)])?;
            write_params(w, &c.params)?;
            w.end_element()?;
        }
        w.end_element()?;
    }
    if let Some(r) = &ic.software_ref {
        w.empty("softwareRef", &[a("ref", enc(r))])?;
    }
    w.end_element()?;
    Ok(())
}

fn write_data_processing<O: Write>(w: W<O>, dp: &DataProcessing) -> Result<()> {
    w.start("dataProcessing", &[a("id", enc(&dp.id))])?;
    for m in &dp.methods {
        let mut attrs = vec![a("order", m.order)];
        if let Some(r) = &m.software_ref {
            attrs.push(a("softwareRef", enc(r)));
        }
        w.start("processingMethod", &attrs)?;
        write_params(w, &m.params)?;
        w.end_element()?;
    }
    w.end_element()?;
    Ok(())
}

fn spectrum_ref_attrs(
    spectrum_ref: &str,
    external_id: &str,
    source_file_ref: &Option<String>,
) -> Result<Vec<(&'static str, String)>> {
    let mut attrs = Vec::new();
    if spectrum_ref.is_empty() {
        if !external_id.is_empty() {
            let sf = source_file_ref.as_ref().ok_or_else(|| {
                anyhow!("[IO::write] External spectrum references must refer to a source file")
            })?;
            attrs.push(a("sourceFileRef", enc(sf)));
            attrs.push(a("externalSpectrumID", external_id));
        }
    } else {
        attrs.push(a("spectrumRef", spectrum_ref)); // not an XML:IDREF
    }
    Ok(attrs)
}

/// MzmlWriter.cs writes these three independently, in this order.
fn spectrum_ref_attrs_net(
    spectrum_ref: &str,
    external_id: &str,
    source_file_ref: &Option<String>,
) -> Vec<(&'static str, String)> {
    let mut attrs = Vec::new();
    if let Some(sf) = source_file_ref {
        attrs.push(a("sourceFileRef", enc(sf)));
    }
    if !external_id.is_empty() {
        attrs.push(a("externalSpectrumID", external_id));
    }
    if !spectrum_ref.is_empty() {
        attrs.push(a("spectrumRef", spectrum_ref));
    }
    attrs
}

fn write_precursor<O: Write>(w: W<O>, p: &Precursor) -> Result<()> {
    match w.dialect() {
        Dialect::Pwiz3Cpp => {
            let attrs =
                spectrum_ref_attrs(&p.spectrum_ref, &p.external_spectrum_id, &p.source_file_ref)?;
            w.start("precursor", &attrs)?;
            write_params(w, &p.params)?;
        }
        Dialect::Pwiz4Net => {
            if !p.params.is_empty() {
                bail!("pwiz 4 MzmlWriter does not write precursor-level params");
            }
            let attrs = spectrum_ref_attrs_net(
                &p.spectrum_ref,
                &p.external_spectrum_id,
                &p.source_file_ref,
            );
            w.start("precursor", &attrs)?;
        }
    }
    if !p.isolation_window.is_empty() {
        write_named_params(w, "isolationWindow", &p.isolation_window)?;
    }
    if !p.selected_ions.is_empty() {
        w.start("selectedIonList", &[a("count", p.selected_ions.len())])?;
        for si in &p.selected_ions {
            write_named_params(w, "selectedIon", si)?;
        }
        w.end_element()?;
    }
    write_named_params(w, "activation", &p.activation)?;
    w.end_element()?;
    Ok(())
}

fn write_product<O: Write>(w: W<O>, p: &Product) -> Result<()> {
    w.start("product", &[])?;
    if !p.isolation_window.is_empty() {
        write_named_params(w, "isolationWindow", &p.isolation_window)?;
    }
    w.end_element()?;
    Ok(())
}

fn write_scan<O: Write>(w: W<O>, s: &Scan, msd: &MsData) -> Result<()> {
    let ic_attr = s
        .instrument_configuration_ref
        .as_ref()
        .filter(|ic| msd.run.default_instrument_configuration_ref.as_deref() != Some(ic.as_str()))
        .map(|ic| a("instrumentConfigurationRef", enc(ic)));
    let attrs = match w.dialect() {
        Dialect::Pwiz3Cpp => {
            let mut attrs =
                spectrum_ref_attrs(&s.spectrum_ref, &s.external_spectrum_id, &s.source_file_ref)?;
            attrs.extend(ic_attr);
            attrs
        }
        Dialect::Pwiz4Net => {
            let mut attrs: Vec<_> = ic_attr.into_iter().collect();
            attrs.extend(spectrum_ref_attrs_net(
                &s.spectrum_ref,
                &s.external_spectrum_id,
                &s.source_file_ref,
            ));
            attrs
        }
    };
    w.start("scan", &attrs)?;
    write_params(w, &s.params)?;
    if !s.scan_windows.is_empty() {
        w.start("scanWindowList", &[a("count", s.scan_windows.len())])?;
        for sw in &s.scan_windows {
            write_named_params(w, "scanWindow", sw)?;
        }
        w.end_element()?;
    }
    w.end_element()?;
    Ok(())
}

fn write_scan_list<O: Write>(w: W<O>, sl: &ScanList, msd: &MsData) -> Result<()> {
    w.start("scanList", &[a("count", sl.scans.len())])?;
    write_params(w, &sl.params)?;
    for s in &sl.scans {
        write_scan(w, s, msd)?;
    }
    w.end_element()?;
    Ok(())
}

const MS_MZ_ARRAY: &str = "MS:1000514";
const MS_INTENSITY_ARRAY: &str = "MS:1000515";
const MS_TIME_ARRAY: &str = "MS:1000595";

fn write_binary_data_array<O: Write>(
    w: W<O>,
    b: &BinaryDataArray,
    config: &EncoderConfig,
) -> Result<()> {
    let is_integer = matches!(b.data, ArrayData::Int(_));
    let net = config.dialect == Dialect::Pwiz4Net;
    // MzmlWriter.WriteIntegerDataArray: always int64 ("BinaryPrecision is ignored")
    let precision = if net && is_integer {
        Precision::P64
    } else {
        config.precision_for(b.kind())
    };
    let encoded = encode::encode(&b.data, precision, config.compression, config.dialect)?;

    let mut attrs = Vec::new();
    // primary array types can never override the default array length
    // (pwiz 4 writes it on every integer array regardless)
    if (net && is_integer)
        || (!b.params.has(MS_MZ_ARRAY)
            && !b.params.has(MS_TIME_ARRAY)
            && !b.params.has(MS_INTENSITY_ARRAY))
    {
        attrs.push(a("arrayLength", b.data.len()));
    }
    attrs.push(a("encodedLength", encoded.len()));
    if let Some(dp) = &b.data_processing_ref {
        attrs.push(a("dataProcessingRef", enc(dp)));
    }
    w.start("binaryDataArray", &attrs)?;
    let precision_term = match (precision, is_integer) {
        (Precision::P32, true) => cv::encoding::INT_32,
        (Precision::P32, false) => cv::encoding::FLOAT_32,
        (Precision::P64, true) => cv::encoding::INT_64,
        (Precision::P64, false) => cv::encoding::FLOAT_64,
    };
    write_cv_param(w, &CvParam::new(precision_term, ""))?;
    let compression_term = match config.compression {
        Compression::None => cv::encoding::NO_COMPRESSION,
        Compression::Zlib => cv::encoding::ZLIB,
    };
    write_cv_param(w, &CvParam::new(compression_term, ""))?;
    write_params(w, &b.params)?;
    w.push_style(STYLE_INLINE_INNER);
    w.start("binary", &[])?;
    w.characters(&encoded, false)?;
    w.end_element()?;
    w.pop_style();
    w.end_element()?;
    Ok(())
}

fn write_arrays<O: Write>(
    w: W<O>,
    arrays: &[BinaryDataArray],
    config: &EncoderConfig,
) -> Result<()> {
    if arrays.is_empty() {
        return Ok(());
    }
    w.start("binaryDataArrayList", &[a("count", arrays.len())])?;
    // pwiz writes all BinaryDataArrays, then all IntegerDataArrays
    for b in arrays
        .iter()
        .filter(|b| matches!(b.data, ArrayData::Float(_)))
    {
        write_binary_data_array(w, b, config)?;
    }
    for b in arrays
        .iter()
        .filter(|b| matches!(b.data, ArrayData::Int(_)))
    {
        write_binary_data_array(w, b, config)?;
    }
    w.end_element()?;
    Ok(())
}

fn write_spectrum<O: Write>(
    w: W<O>,
    s: &Spectrum,
    msd: &MsData,
    config: &EncoderConfig,
) -> Result<()> {
    let mut attrs = vec![a("index", s.index), a("id", &s.id)];
    if !s.spot_id.is_empty() {
        attrs.push(a("spotID", &s.spot_id));
    }
    attrs.push(a("defaultArrayLength", s.default_array_length));
    let dp = s
        .data_processing_ref
        .as_ref()
        .map(|r| a("dataProcessingRef", enc(r)));
    let sf = s
        .source_file_ref
        .as_ref()
        .map(|r| a("sourceFileRef", enc(r)));
    match w.dialect() {
        Dialect::Pwiz3Cpp => attrs.extend(dp.into_iter().chain(sf)),
        Dialect::Pwiz4Net => attrs.extend(sf.into_iter().chain(dp)),
    }
    w.start("spectrum", &attrs)?;
    write_params(w, &s.params)?;
    write_scan_list(w, &s.scan_list, msd)?;
    if !s.precursors.is_empty() {
        w.start("precursorList", &[a("count", s.precursors.len())])?;
        for p in &s.precursors {
            write_precursor(w, p)?;
        }
        w.end_element()?;
    }
    if !s.products.is_empty() {
        w.start("productList", &[a("count", s.products.len())])?;
        for p in &s.products {
            write_product(w, p)?;
        }
        w.end_element()?;
    }
    write_arrays(w, &s.arrays, config)?;
    w.end_element()?;
    Ok(())
}

fn write_chromatogram<O: Write>(w: W<O>, c: &Chromatogram, config: &EncoderConfig) -> Result<()> {
    let mut attrs = vec![
        a("index", c.index),
        a("id", &c.id),
        a("defaultArrayLength", c.default_array_length),
    ];
    if let Some(dp) = &c.data_processing_ref {
        attrs.push(a("dataProcessingRef", enc(dp)));
    }
    w.start("chromatogram", &attrs)?;
    write_params(w, &c.params)?;
    if !c.precursor.is_empty() {
        write_precursor(w, &c.precursor)?;
    }
    if !c.product.isolation_window.is_empty() {
        write_product(w, &c.product)?;
    }
    write_arrays(w, &c.arrays, config)?;
    w.end_element()?;
    Ok(())
}

fn write_run<O: Write>(w: W<O>, msd: &MsData, config: &EncoderConfig) -> Result<()> {
    let run = &msd.run;
    let net = w.dialect() == Dialect::Pwiz4Net;
    let mut attrs = vec![a("id", enc(&run.id))];
    let default_ic = match (
        &run.default_instrument_configuration_ref,
        msd.instrument_configurations.first(),
    ) {
        (Some(r), _) => Some(enc(r)),
        (None, _) if net => None, // C# writes it only when the run has one
        (None, Some(ic)) => Some(enc(&ic.id)),
        (None, None) => Some("IC".to_string()),
    };
    if let Some(ic) = default_ic {
        attrs.push(a("defaultInstrumentConfigurationRef", ic));
    }
    // C++ writes the list-level defaultDataProcessingRef raw; C# id-encodes it.
    let list_dp = |r: &String| if net { enc(r) } else { r.clone() };
    if let Some(s) = &run.sample_ref {
        attrs.push(a("sampleRef", enc(s)));
    }
    if !run.start_time_stamp.is_empty() {
        attrs.push(a("startTimeStamp", &run.start_time_stamp));
    }
    if let Some(sf) = &run.default_source_file_ref {
        attrs.push(a("defaultSourceFileRef", enc(sf)));
    }
    w.start("run", &attrs)?;
    write_params(w, &run.params)?;
    if !run.spectra.is_empty() {
        let mut attrs = vec![a("count", run.spectra.len())];
        if let Some(dp) = &run.spectrum_default_data_processing_ref {
            attrs.push(a("defaultDataProcessingRef", list_dp(dp)));
        }
        w.start("spectrumList", &attrs)?;
        for (i, s) in run.spectra.iter().enumerate() {
            if s.index != i {
                return Err(anyhow!("[IO::write(SpectrumList)] Bad index."));
            }
            write_spectrum(w, s, msd, config)?;
        }
        w.end_element()?;
    }
    if !run.chromatograms.is_empty() {
        let mut attrs = vec![a("count", run.chromatograms.len())];
        if let Some(dp) = &run.chromatogram_default_data_processing_ref {
            attrs.push(a("defaultDataProcessingRef", list_dp(dp)));
        }
        w.start("chromatogramList", &attrs)?;
        for c in &run.chromatograms {
            write_chromatogram(w, c, config)?;
        }
        w.end_element()?;
    }
    w.end_element()?;
    Ok(())
}

/// Serialize `msd` as non-indexed mzML exactly as pwiz `Serializer_mzML` does
/// with `indexed = false`.
pub fn write_mzml<O: Write>(out: O, msd: &MsData, config: &EncoderConfig) -> Result<O> {
    let net = config.dialect == Dialect::Pwiz4Net;
    let mut w = XmlWriter::new(out, config.dialect);
    w.processing_instruction("xml", "version=\"1.0\" encoding=\"utf-8\"")?;
    let version = if msd.version.is_empty() {
        "1.1.0"
    } else {
        msd.version.as_str()
    };
    const NS: &str = "http://psi.hupo.org/ms/mzml";
    let mut attrs = Vec::new();
    if !net {
        attrs.push(a("xmlns", NS));
    }
    attrs.push(a("xmlns:xsi", "http://www.w3.org/2001/XMLSchema-instance"));
    // C# hard-codes the 1.1.0 schema; C++ derives it from the version.
    let xsd = if net { "1.1.0" } else { version };
    attrs.push(a(
        "xsi:schemaLocation",
        format!("http://psi.hupo.org/ms/mzml http://psidev.info/files/ms/mzML/xsd/mzML{xsd}.xsd"),
    ));
    if !msd.accession.is_empty() {
        attrs.push(a("accession", &msd.accession));
    }
    if !net || !msd.id.is_empty() {
        attrs.push(a("id", &msd.id));
    }
    attrs.push(a("version", version));
    if net {
        // .NET XmlWriter emits the element's own namespace declaration last.
        attrs.push(a("xmlns", NS));
    }
    w.start("mzML", &attrs)?;
    write_list(&mut w, &msd.cvs, "cvList", write_cv)?;
    write_file_description(&mut w, &msd.file_description)?;
    write_list(
        &mut w,
        &msd.param_groups,
        "referenceableParamGroupList",
        write_param_group,
    )?;
    write_list(&mut w, &msd.samples, "sampleList", write_sample)?;
    write_list(&mut w, &msd.software, "softwareList", write_software)?;
    if net {
        if !msd.scan_settings.is_empty() {
            bail!("pwiz 4 MzmlWriter does not write scanSettingsList");
        }
    } else {
        write_list(
            &mut w,
            &msd.scan_settings,
            "scanSettingsList",
            write_scan_settings,
        )?;
    }
    if msd.instrument_configurations.is_empty() && !net {
        let ic = InstrumentConfiguration {
            id: "IC".into(),
            params: Params {
                cv: vec![CvParam::new("MS:1000031", "")],
                ..Default::default()
            },
            ..Default::default()
        };
        write_list(
            &mut w,
            std::slice::from_ref(&ic),
            "instrumentConfigurationList",
            write_instrument_configuration,
        )?;
    } else {
        write_list(
            &mut w,
            &msd.instrument_configurations,
            "instrumentConfigurationList",
            write_instrument_configuration,
        )?;
    }
    write_list(
        &mut w,
        &msd.data_processing,
        "dataProcessingList",
        write_data_processing,
    )?;
    write_run(&mut w, msd, config)?;
    w.end_element()?;
    Ok(w.into_inner())
}
