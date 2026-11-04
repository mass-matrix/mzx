//! In-memory mzML model mirroring ProteoWizard's `pwiz::msdata::MSData`.
//!
//! Only what pwiz's mzML writer (`IO.cpp`) serializes is modelled. Values are
//! kept exactly as pwiz holds them: cvParam values are already-formatted strings
//! (pwiz formats numbers when the reader builds the param, not when it writes),
//! and binary arrays hold decoded numbers (pwiz holds `vector<double>` /
//! `vector<int64_t>` and encodes at write time).
//!
//! Cross references (softwareRef, sourceFileRef, ...) are stored as the
//! *decoded* id of the referenced object; the writer applies pwiz's
//! `encode_xml_id_copy` where pwiz does.

/// A controlled-vocabulary parameter. `name`, `cvRef` and the unit's name and
/// cvRef are not stored: pwiz derives them from its compiled CV table, and so
/// do we (see [`crate::cv`]).
#[derive(Debug, Clone, PartialEq)]
pub struct CvParam {
    pub accession: String,
    pub value: String,
    pub unit: Option<String>,
}

impl CvParam {
    pub fn new(accession: &str, value: impl Into<String>) -> Self {
        Self {
            accession: accession.to_string(),
            value: value.into(),
            unit: None,
        }
    }
    pub fn with_unit(accession: &str, value: impl Into<String>, unit: &str) -> Self {
        Self {
            accession: accession.to_string(),
            value: value.into(),
            unit: Some(unit.to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UserParam {
    pub name: String,
    pub value: String,
    pub type_: String,
    pub unit: Option<String>,
}

/// pwiz `ParamContainer`: written as referenceableParamGroupRefs, then
/// cvParams, then userParams, regardless of the order they were read in.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Params {
    pub group_refs: Vec<String>,
    pub cv: Vec<CvParam>,
    pub user: Vec<UserParam>,
}

impl Params {
    pub fn is_empty(&self) -> bool {
        self.group_refs.is_empty() && self.cv.is_empty() && self.user.is_empty()
    }
    pub fn has(&self, accession: &str) -> bool {
        self.cv.iter().any(|p| p.accession == accession)
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Cv {
    pub id: String,
    pub full_name: String,
    pub version: String,
    pub uri: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SourceFile {
    pub id: String,
    pub name: String,
    pub location: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FileDescription {
    pub file_content: Params,
    pub source_files: Vec<SourceFile>,
    pub contacts: Vec<Params>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParamGroup {
    pub id: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Sample {
    pub id: String,
    pub name: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Software {
    pub id: String,
    pub version: String,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ComponentKind {
    Source,
    Analyzer,
    Detector,
}

impl ComponentKind {
    pub fn tag(&self) -> &'static str {
        match self {
            ComponentKind::Source => "source",
            ComponentKind::Analyzer => "analyzer",
            ComponentKind::Detector => "detector",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    pub kind: ComponentKind,
    pub order: i32,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct InstrumentConfiguration {
    pub id: String,
    pub params: Params,
    pub components: Vec<Component>,
    pub software_ref: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProcessingMethod {
    pub order: i32,
    pub software_ref: Option<String>,
    pub params: Params,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DataProcessing {
    pub id: String,
    pub methods: Vec<ProcessingMethod>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScanSettings {
    pub id: String,
    pub source_file_refs: Vec<String>,
    pub targets: Vec<Params>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scan {
    pub spectrum_ref: String,
    pub source_file_ref: Option<String>,
    pub external_spectrum_id: String,
    pub instrument_configuration_ref: Option<String>,
    pub params: Params,
    pub scan_windows: Vec<Params>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScanList {
    pub params: Params,
    pub scans: Vec<Scan>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Precursor {
    pub spectrum_ref: String,
    pub source_file_ref: Option<String>,
    pub external_spectrum_id: String,
    pub params: Params,
    pub isolation_window: Params,
    pub selected_ions: Vec<Params>,
    pub activation: Params,
}

impl Precursor {
    /// pwiz `Precursor::empty()`.
    pub fn is_empty(&self) -> bool {
        self.spectrum_ref.is_empty()
            && self.source_file_ref.is_none()
            && self.external_spectrum_id.is_empty()
            && self.params.is_empty()
            && self.isolation_window.is_empty()
            && self.selected_ions.is_empty()
            && self.activation.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Product {
    pub isolation_window: Params,
}

/// Array payload as pwiz holds it in memory.
#[derive(Debug, Clone, PartialEq)]
pub enum ArrayData {
    /// `BinaryDataArray::data` (always double in memory).
    Float(Vec<f64>),
    /// `IntegerDataArray::data`.
    Int(Vec<i64>),
}

impl ArrayData {
    pub fn len(&self) -> usize {
        match self {
            ArrayData::Float(v) => v.len(),
            ArrayData::Int(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryDataArray {
    /// Params owned by the array (e.g. "m/z array"). Never contains the
    /// precision/compression terms: the writer emits those from the encoder config.
    pub params: Params,
    pub data_processing_ref: Option<String>,
    pub data: ArrayData,
}

impl BinaryDataArray {
    /// The array-type term, standing in for pwiz `cvParamChild(MS_binary_data_array)`.
    pub fn kind(&self) -> Option<&str> {
        self.params
            .cv
            .iter()
            .map(|p| p.accession.as_str())
            .find(|a| crate::cv::is_array_type(a))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Spectrum {
    pub index: usize,
    pub id: String,
    pub spot_id: String,
    pub default_array_length: usize,
    pub data_processing_ref: Option<String>,
    pub source_file_ref: Option<String>,
    pub params: Params,
    pub scan_list: ScanList,
    pub precursors: Vec<Precursor>,
    pub products: Vec<Product>,
    pub arrays: Vec<BinaryDataArray>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Chromatogram {
    pub index: usize,
    pub id: String,
    pub default_array_length: usize,
    pub data_processing_ref: Option<String>,
    pub params: Params,
    pub precursor: Precursor,
    pub product: Product,
    pub arrays: Vec<BinaryDataArray>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Run {
    pub id: String,
    pub default_instrument_configuration_ref: Option<String>,
    pub sample_ref: Option<String>,
    pub start_time_stamp: String,
    pub default_source_file_ref: Option<String>,
    pub params: Params,
    pub spectrum_default_data_processing_ref: Option<String>,
    pub spectra: Vec<Spectrum>,
    pub chromatogram_default_data_processing_ref: Option<String>,
    pub chromatograms: Vec<Chromatogram>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MsData {
    pub accession: String,
    pub id: String,
    pub version: String,
    pub cvs: Vec<Cv>,
    pub file_description: FileDescription,
    pub param_groups: Vec<ParamGroup>,
    pub samples: Vec<Sample>,
    pub software: Vec<Software>,
    pub scan_settings: Vec<ScanSettings>,
    pub instrument_configurations: Vec<InstrumentConfiguration>,
    pub data_processing: Vec<DataProcessing>,
    pub run: Run,
}
