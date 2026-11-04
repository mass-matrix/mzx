//! Port of `ThermoInstrumentModel.Translate` (pwiz/data/vendor_readers/Thermo/
//! ThermoInstrumentModel.cs @ fb75aff). The table is transcribed mechanically,
//! in order: the first matching row wins.

use mzx_mzml::{cv, Dialect};

#[derive(Clone, Copy)]
enum Match {
    Exact,
    ExactNoSpaces,
    Contains,
    ContainsNoSpaces,
}

const TABLE: &[(&str, &str, Match)] = &[
    ("MAT253", "MS_MAT253", Match::ExactNoSpaces),
    ("MAT900XP", "MS_MAT900XP", Match::ExactNoSpaces),
    ("MAT900XPTRAP", "MS_MAT900XP_Trap", Match::ExactNoSpaces),
    ("MAT95XP", "MS_MAT95XP", Match::ExactNoSpaces),
    ("MAT95XPTRAP", "MS_MAT95XP_Trap", Match::ExactNoSpaces),
    ("SSQ7000", "MS_SSQ_7000", Match::ExactNoSpaces),
    ("TSQ7000", "MS_TSQ_7000", Match::ExactNoSpaces),
    ("TSQ8000EVO", "MS_TSQ_8000_Evo", Match::ExactNoSpaces),
    ("TSQ9000", "MS_TSQ_9000", Match::ExactNoSpaces),
    ("TSQ", "MS_TSQ", Match::Exact),
    ("ELEMENT2", "MS_Element_2", Match::ExactNoSpaces),
    (
        "DELTA PLUSADVANTAGE",
        "MS_DELTA_plusAdvantage",
        Match::Exact,
    ),
    ("DELTAPLUSXP", "MS_DELTAplusXP", Match::Exact),
    ("DELTA PLUS IRMS", "MS_DeltaPlus_IRMS", Match::Exact),
    ("LCQ ADVANTAGE", "MS_LCQ_Advantage", Match::Exact),
    ("LCQ CLASSIC", "MS_LCQ_Classic", Match::Exact),
    ("LCQ DECA", "MS_LCQ_Deca", Match::Exact),
    ("LCQ DECA XP", "MS_LCQ_Deca_XP_Plus", Match::Exact),
    ("LCQ DECA XP PLUS", "MS_LCQ_Deca_XP_Plus", Match::Exact),
    ("NEPTUNE", "MS_neptune", Match::Exact),
    ("DSQ", "MS_DSQ", Match::Exact),
    ("POLARISQ", "MS_PolarisQ", Match::Exact),
    ("SURVEYOR MSQ", "MS_Surveyor_MSQ", Match::Exact),
    ("MSQ PLUS", "MS_Surveyor_MSQ", Match::Exact),
    ("TEMPUS TOF", "MS_TEMPUS_TOF", Match::Exact),
    ("TRACE DSQ", "MS_TRACE_DSQ", Match::Exact),
    ("TRITON", "MS_TRITON", Match::Exact),
    ("LTQ", "MS_LTQ", Match::Exact),
    ("LTQ XL", "MS_LTQ_XL", Match::Exact),
    ("LTQ FT", "MS_LTQ_FT", Match::Exact),
    ("LTQ-FT", "MS_LTQ_FT", Match::Exact),
    ("LTQ FT ULTRA", "MS_LTQ_FT_Ultra", Match::Exact),
    ("LTQ ORBITRAP", "MS_LTQ_Orbitrap", Match::Exact),
    (
        "LTQ ORBITRAP CLASSIC",
        "MS_LTQ_Orbitrap_Classic",
        Match::Exact,
    ),
    (
        "LTQ ORBITRAP DISCOVERY",
        "MS_LTQ_Orbitrap_Discovery",
        Match::Exact,
    ),
    ("LTQ ORBITRAP XL", "MS_LTQ_Orbitrap_XL", Match::Exact),
    (
        "ORBITRAP VELOS PRO",
        "MS_LTQ_Orbitrap_Velos_Pro",
        Match::Contains,
    ),
    (
        "LTQ ORBITRAP VELOS/ETD",
        "MS_LTQ_Orbitrap_Velos_ETD",
        Match::Exact,
    ),
    ("ORBITRAP VELOS", "MS_LTQ_Orbitrap_Velos", Match::Contains),
    ("ORBITRAP ELITE", "MS_LTQ_Orbitrap_Elite", Match::Contains),
    ("VELOS PLUS", "MS_Velos_Plus", Match::Contains),
    ("VELOS PRO", "MS_Velos_Pro", Match::Contains),
    ("LTQ VELOS", "MS_LTQ_Velos", Match::Exact),
    ("LTQ VELOS ETD", "MS_LTQ_Velos_ETD", Match::Exact),
    ("LXQ", "MS_LXQ", Match::Exact),
    ("LCQ FLEET", "MS_LCQ_Fleet", Match::Exact),
    ("ITQ 700", "MS_ITQ_700", Match::Exact),
    ("ITQ 900", "MS_ITQ_900", Match::Exact),
    ("ITQ 1100", "MS_ITQ_1100", Match::Exact),
    ("ITQ", "MS_ITQ", Match::Contains),
    ("GC QUANTUM", "MS_GC_Quantum", Match::Exact),
    ("LTQ XL ETD", "MS_LTQ_XL_ETD", Match::Exact),
    (
        "LTQ ORBITRAP XL ETD",
        "MS_LTQ_Orbitrap_XL_ETD",
        Match::Exact,
    ),
    ("DFS", "MS_DFS", Match::Exact),
    ("DSQ II", "MS_DSQ_II", Match::Exact),
    ("ISQ SERIES", "MS_ISQ", Match::Exact),
    ("ISQEC", "MS_ISQ", Match::ContainsNoSpaces),
    ("ISQEM", "MS_ISQ", Match::ContainsNoSpaces),
    ("ISQ 7000", "MS_ISQ_7000", Match::Exact),
    ("ISQ LT", "MS_ISQ_LT", Match::Exact),
    ("MALDI LTQ XL", "MS_MALDI_LTQ_XL", Match::Exact),
    ("MALDI LTQ ORBITRAP", "MS_MALDI_LTQ_Orbitrap", Match::Exact),
    ("TSQ QUANTUM", "MS_TSQ_Quantum", Match::Exact),
    (
        "TSQ QUANTUM ACCESS MAX",
        "MS_TSQ_Quantum_Access_MAX",
        Match::Contains,
    ),
    (
        "TSQ QUANTUM ACCESS",
        "MS_TSQ_Quantum_Access",
        Match::Contains,
    ),
    ("TSQ QUANTUM ULTRA", "MS_TSQ_Quantum_Ultra", Match::Exact),
    (
        "TSQ QUANTUM ULTRA AM",
        "MS_TSQ_Quantum_Ultra_AM",
        Match::Exact,
    ),
    ("TSQ VANTAGE STANDARD", "MS_TSQ_Vantage", Match::Exact),
    ("TSQ VANTAGE EMR", "MS_TSQ_Vantage", Match::Exact),
    ("TSQ VANTAGE AM", "MS_TSQ_Vantage", Match::Exact),
    ("TSQ QUANTIVA", "MS_TSQ_Quantiva", Match::Exact),
    ("TSQ ENDURA", "MS_TSQ_Endura", Match::Exact),
    ("TSQ ALTIS", "MS_TSQ_Altis", Match::Exact),
    ("TSQ ALTIS PLUS", "MS_TSQ_Altis_Plus", Match::Exact),
    ("TSQ QUANTIS", "MS_TSQ_Quantis", Match::Exact),
    ("TSQ CERTIS", "MS_TSQ_Certis", Match::Exact),
    ("TSQ QUANTUM XLS", "MS_TSQ_Quantum_XLS", Match::Exact),
    ("TSQ 8000", "MS_TSQ_8000", Match::Exact),
    ("ELEMENT XR", "MS_Element_XR", Match::Exact),
    ("ELEMENT GD", "MS_Element_GD", Match::Exact),
    ("GC ISOLINK", "MS_GC_IsoLink", Match::Exact),
    ("ORBITRAP ID-X", "MS_Orbitrap_ID_X", Match::Exact),
    ("ORBITRAP IQ-X", "MS_Orbitrap_IQ_X", Match::Exact),
    (
        "Q EXACTIVE GC ORBITRAP",
        "MS_Q_Exactive_GC_Orbitrap",
        Match::Contains,
    ),
    ("Q EXACTIVE PLUS", "MS_Q_Exactive_Plus", Match::Contains),
    ("Q EXACTIVE HF-X", "MS_Q_Exactive_HF_X", Match::Contains),
    ("Q EXACTIVE HF", "MS_Q_Exactive_HF", Match::Contains),
    ("Q EXACTIVE UHMR", "MS_Q_Exactive_UHMR", Match::Contains),
    ("Q EXACTIVE FOCUS", "MS_Q_Exactive_Focus", Match::Contains),
    ("Q EXACTIVE", "MS_Q_Exactive", Match::Contains),
    ("EXACTIVE PLUS", "MS_Exactive_Plus", Match::Contains),
    ("EXACTIVE", "MS_Exactive", Match::Contains),
    (
        "ORBITRAP EXPLORIS 120",
        "MS_Orbitrap_Exploris_120",
        Match::Exact,
    ),
    (
        "ORBITRAP EXPLORIS 240",
        "MS_Orbitrap_Exploris_240",
        Match::Exact,
    ),
    (
        "ORBITRAPEXPLORISGC240",
        "MS_Orbitrap_Exploris_GC_240",
        Match::ExactNoSpaces,
    ),
    (
        "ORBITRAPEXPLORIS240GC",
        "MS_Orbitrap_Exploris_GC_240",
        Match::ExactNoSpaces,
    ),
    (
        "ORBITRAPEXPLORISGC",
        "MS_Orbitrap_Exploris_GC_MS",
        Match::ContainsNoSpaces,
    ),
    (
        "ORBITRAP EXPLORIS 480",
        "MS_Orbitrap_Exploris_480",
        Match::Exact,
    ),
    (
        "ORBITRAP EXCEDION PRO",
        "MS_Orbitrap_Excedion_Pro",
        Match::Contains,
    ),
    ("ORBITRAP EXCEDION", "MS_Orbitrap_Excedion", Match::Contains),
    ("ORBITRAP GC", "MS_Orbitrap_Exploris_480", Match::Contains),
    ("ECLIPSE", "MS_Orbitrap_Eclipse", Match::Contains),
    ("TRIBRID APEX", "MS_Orbitrap_Tribrid_Apex", Match::Contains),
    ("ASTRAL ZOOM", "MS_Orbitrap_Astral_Zoom", Match::Contains),
    ("ASTRAL", "MS_Orbitrap_Astral", Match::Contains),
    (
        "MALDI LTQ ORBITRAP XL",
        "MS_MALDI_LTQ_Orbitrap_XL",
        Match::Exact,
    ),
    (
        "MALDI LTQ ORBITRAP DISCOVERY",
        "MS_MALDI_LTQ_Orbitrap_Discovery",
        Match::Exact,
    ),
    ("VOYAGER", "MS_ThermoQuest_Voyager", Match::Contains),
    ("FUSION ETD", "MS_Orbitrap_Fusion_ETD", Match::Contains),
    ("FUSION LUMOS", "MS_Orbitrap_Fusion_Lumos", Match::Contains),
    ("FUSION", "MS_Orbitrap_Fusion", Match::Contains),
    ("ASCEND", "MS_Orbitrap_Ascend", Match::Contains),
    ("SURVEYOR PDA", "MS_Surveyor_PDA", Match::Exact),
    ("ACCELA PDA", "MS_Accela_PDA", Match::Exact),
    ("STELLAR", "MS_Stellar", Match::Contains),
];

/// pwiz `CVID` name of the generic fallback.
pub const GENERIC: &str = "MS_Thermo_Electron_instrument_model";

/// Returns the pwiz `CVID` enumerator name for a Thermo model string.
pub fn translate(model: &str) -> &'static str {
    if model.is_empty() {
        return GENERIC;
    }
    // .NET Trim() + ToUpperInvariant(); model strings are ASCII in practice.
    let upper = model.trim().to_uppercase();
    let no_spaces = upper.replace(' ', "");
    for &(name, cvid, m) in TABLE {
        let hit = match m {
            Match::Exact => upper == name,
            Match::ExactNoSpaces => no_spaces == name,
            Match::Contains => upper.contains(name),
            Match::ContainsNoSpaces => no_spaces.contains(name),
        };
        if hit {
            return cvid;
        }
    }
    GENERIC
}

/// Accession of a pwiz `CVID` name in the pwiz 4 CV.
pub fn accession(cvid: &str) -> anyhow::Result<&'static str> {
    cv::by_cvid(Dialect::Pwiz4Net, cvid)
        .ok_or_else(|| anyhow::anyhow!("no CV term for pwiz CVID {cvid}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_row_resolves_and_corpus_models_translate() {
        for &(_, cvid, _) in TABLE {
            accession(cvid).unwrap();
        }
        assert_eq!(
            translate("Q Exactive Focus Orbitrap"),
            "MS_Q_Exactive_Focus"
        );
        assert_eq!(translate("LTQ XL"), "MS_LTQ_XL");
        assert_eq!(translate(""), GENERIC);
    }
}
