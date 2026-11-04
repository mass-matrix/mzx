//! Controlled-vocabulary table.
//!
//! pwiz writes a cvParam's `cvRef`/`name` (and unit `cvRef`/`name`) from its
//! compiled CV table (`cvTermInfo`), generated from the OBO files bundled with
//! that pwiz build. We embed the same OBO files (copied from the pwiz revision
//! the contract is pinned to) so names are reproduced, not copied from input.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::dialect::Dialect;

const PWIZ3_PSI_MS_OBO: &str = include_str!("../data/pwiz3/psi-ms.obo");
const PWIZ3_UNIT_OBO: &str = include_str!("../data/pwiz3/unit.obo");
const PWIZ4_PSI_MS_OBO: &str = include_str!("../data/pwiz4/psi-ms.obo");
const PWIZ4_UNIT_OBO: &str = include_str!("../data/pwiz4/unit.obo");

/// `MS:1000513` "binary data array".
const BINARY_DATA_ARRAY: &str = "MS:1000513";

struct Table {
    names: HashMap<String, String>,
    /// pwiz `CVID` enumerator name (e.g. `MS_Q_Exactive_Focus`) -> accession,
    /// from term names and EXACT synonyms, first definition wins.
    cvids: HashMap<String, String>,
    array_types: HashSet<String>,
}

fn parse_obo(
    text: &str,
    names: &mut HashMap<String, String>,
    parents: &mut HashMap<String, Vec<String>>,
    cvids: &mut HashMap<String, String>,
) {
    let mut id: Option<String> = None;
    let mut in_term = false;
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with('[') {
            in_term = line == "[Term]";
            id = None;
            continue;
        }
        if !in_term {
            continue;
        }
        if let Some(v) = line.strip_prefix("id: ") {
            id = Some(v.trim().to_string());
        } else if let Some(v) = line.strip_prefix("name: ") {
            if let Some(id) = &id {
                names.entry(id.clone()).or_insert_with(|| v.to_string());
                cvids.entry(cvid_name(id, v)).or_insert_with(|| id.clone());
            }
        } else if let Some(v) = line.strip_prefix("synonym: \"") {
            if let (Some(id), Some((syn, rest))) = (&id, v.split_once('"')) {
                if rest.trim_start().starts_with("EXACT") {
                    cvids
                        .entry(cvid_name(id, syn))
                        .or_insert_with(|| id.clone());
                }
            }
        } else if let Some(v) = line.strip_prefix("is_a: ") {
            if let Some(id) = &id {
                let parent = v.split_whitespace().next().unwrap_or("").to_string();
                parents.entry(id.clone()).or_default().push(parent);
            }
        }
    }
}

fn build(psi_ms: &str, unit: &str) -> Table {
    let mut names = HashMap::new();
    let mut parents: HashMap<String, Vec<String>> = HashMap::new();
    let mut cvids = HashMap::new();
    parse_obo(psi_ms, &mut names, &mut parents, &mut cvids);
    parse_obo(unit, &mut names, &mut parents, &mut cvids);
    let mut array_types = HashSet::new();
    for id in names.keys() {
        if is_descendant(id, BINARY_DATA_ARRAY, &parents, 0) {
            array_types.insert(id.clone());
        }
    }
    Table {
        names,
        cvids,
        array_types,
    }
}

fn table(dialect: Dialect) -> &'static Table {
    static PWIZ3: OnceLock<Table> = OnceLock::new();
    static PWIZ4: OnceLock<Table> = OnceLock::new();
    match dialect {
        Dialect::Pwiz3Cpp => PWIZ3.get_or_init(|| build(PWIZ3_PSI_MS_OBO, PWIZ3_UNIT_OBO)),
        Dialect::Pwiz4Net => PWIZ4.get_or_init(|| build(PWIZ4_PSI_MS_OBO, PWIZ4_UNIT_OBO)),
    }
}

fn is_descendant(
    id: &str,
    ancestor: &str,
    parents: &HashMap<String, Vec<String>>,
    depth: usize,
) -> bool {
    if depth > 64 {
        return false;
    }
    parents
        .get(id)
        .map(|ps| {
            ps.iter()
                .any(|p| p == ancestor || is_descendant(p, ancestor, parents, depth + 1))
        })
        .unwrap_or(false)
}

/// Term name, as pwiz's `cvTermInfo(cvid).name` in the given pwiz generation.
pub fn name(dialect: Dialect, accession: &str) -> Option<&'static str> {
    table(dialect).names.get(accession).map(|s| s.as_str())
}

/// pwiz cvgen enumerator name: prefix, `_`, then the term text with every
/// character outside `[A-Za-z0-9]` replaced by `_`.
fn cvid_name(id: &str, text: &str) -> String {
    let mut s = String::from(prefix(id));
    s.push('_');
    s.extend(
        text.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }),
    );
    s
}

/// Accession for a pwiz `CVID` enumerator name such as `MS_Q_Exactive_Focus`.
pub fn by_cvid(dialect: Dialect, cvid: &str) -> Option<&'static str> {
    table(dialect).cvids.get(cvid).map(|s| s.as_str())
}

/// CV prefix, as pwiz's `cvTermInfo(cvid).prefix()`.
pub fn prefix(accession: &str) -> &str {
    accession.split(':').next().unwrap_or(accession)
}

/// True for terms that are `is_a` descendants of "binary data array" in either
/// CV generation (the model itself is dialect-independent).
pub fn is_array_type(accession: &str) -> bool {
    table(Dialect::Pwiz4Net).array_types.contains(accession)
        || table(Dialect::Pwiz3Cpp).array_types.contains(accession)
}

/// Accessions the mzML writer generates from the encoder config (never stored in
/// a [`crate::model::BinaryDataArray`]'s params).
pub mod encoding {
    pub const FLOAT_32: &str = "MS:1000521";
    pub const FLOAT_64: &str = "MS:1000523";
    pub const INT_32: &str = "MS:1000519";
    pub const INT_64: &str = "MS:1000522";
    pub const NO_COMPRESSION: &str = "MS:1000576";
    pub const ZLIB: &str = "MS:1000574";

    pub fn is_encoding_term(accession: &str) -> bool {
        matches!(
            accession,
            FLOAT_32 | FLOAT_64 | INT_32 | INT_64 | NO_COMPRESSION | ZLIB
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_array_types() {
        for d in [Dialect::Pwiz3Cpp, Dialect::Pwiz4Net] {
            assert_eq!(name(d, "MS:1000514"), Some("m/z array"));
            assert_eq!(name(d, "UO:0000010"), Some("second"));
        }
        assert!(is_array_type("MS:1000514"));
        assert!(is_array_type("MS:1000515"));
        assert!(is_array_type("MS:1000786")); // non-standard data array
        assert!(!is_array_type("MS:1000523"));
        let d = Dialect::Pwiz4Net;
        assert_eq!(by_cvid(d, "MS_Q_Exactive_Focus"), Some("MS:1002993"));
        assert_eq!(by_cvid(d, "MS_TIC_chromatogram"), Some("MS:1000235"));
        assert_eq!(by_cvid(d, "MS_PDA"), Some("MS:1000621"));
        assert_eq!(by_cvid(d, "MS_EMR_spectrum"), Some("MS:1000804"));
        assert_eq!(by_cvid(d, "UO_minute"), Some("UO:0000031"));
    }
}
