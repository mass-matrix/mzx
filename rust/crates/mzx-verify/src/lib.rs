//! Contract verification (see `rust/CONTRACT.md`).
//!
//! [`normalize`] is the *only* transformation allowed before byte comparison.
//! It is applied identically to the ProteoWizard golden and to mzx output and
//! touches exactly:
//!   1. converter `<software>` blocks (pwiz: has cvParam MS:1000615
//!      "ProteoWizard software"; mzx: has userParam `mzx converter`) are
//!      collapsed to `<software id="@CONVERTERn@" version="@VERSION@"/>`, and
//!      attribute values exactly equal to such an id (softwareRef, ref) are
//!      rewritten to the token;
//!   2. the `location` attribute of `<sourceFile>`;
//!   3. the `value` of cvParam MS:1001885 "command-line parameters" (pwiz 4
//!      records its own argv, including container paths and `--outfile`).
//!
//! Everything else, including every base64 payload, must already be identical.
//! Works on either line ending (pwiz 3 `\n`, pwiz 4 `\r\n`); collapsed lines keep
//! the ending of the block they replace.

use anyhow::Result;
use mzx_mzml::encode::{self, Compression, Precision};

pub const MZX_CONVERTER_USERPARAM: &str = mzx_convert::CONVERTER_USERPARAM;

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// Value of attribute `key` in a single-element line.
fn attr<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!(" {key}=\"");
    let start = line.find(&pat)? + pat.len();
    let end = line[start..].find('"')? + start;
    Some(&line[start..end])
}

fn is_converter_block(block: &[&str]) -> bool {
    block.iter().any(|l| {
        let t = l.trim_start();
        (t.starts_with("<cvParam ") && t.contains("accession=\"MS:1000615\""))
            || (t.starts_with("<userParam ")
                && t.contains(&format!("name=\"{MZX_CONVERTER_USERPARAM}\"")))
    })
}

/// Apply the contract normalization to a pwiz-style (one element per line) mzML document.
pub fn normalize(doc: &str) -> String {
    let lines: Vec<&str> = doc.split_inclusive('\n').collect();
    let mut out: Vec<String> = Vec::with_capacity(lines.len());
    let mut converter_ids: Vec<(String, String)> = Vec::new();

    // pass 1: collapse converter software blocks
    let mut i = 0;
    while i < lines.len() {
        let t = lines[i].trim_start();
        if t.starts_with("<software ") && !t.trim_end().ends_with("/>") {
            let end = (i..lines.len()).find(|&j| lines[j].trim_start().starts_with("</software>"));
            if let Some(end) = end {
                let block = &lines[i..=end];
                if is_converter_block(block) {
                    let id = attr(lines[i], "id").unwrap_or("").to_string();
                    let token = format!("@CONVERTER{}@", converter_ids.len() + 1);
                    let last = lines[end];
                    let eol = &last[last.trim_end_matches(['\r', '\n']).len()..];
                    out.push(format!(
                        "{}<software id=\"{}\" version=\"@VERSION@\"/>{eol}",
                        indent_of(lines[i]),
                        token
                    ));
                    converter_ids.push((id, token));
                    i = end + 1;
                    continue;
                }
            }
        }
        out.push(lines[i].to_string());
        i += 1;
    }

    // pass 2: references to converter ids, sourceFile location, command line
    for line in out.iter_mut() {
        let t = line.trim_start();
        if t.starts_with("<binary>") {
            continue;
        }
        let is_source_file = t.starts_with("<sourceFile ");
        let is_command_line = t.starts_with("<cvParam ") && t.contains(" accession=\"MS:1001885\"");
        if is_source_file {
            if let Some(loc) = attr(line, "location") {
                *line = line.replacen(
                    &format!(" location=\"{loc}\""),
                    " location=\"@LOCATION@\"",
                    1,
                );
            }
        }
        if is_command_line {
            if let Some(v) = attr(line, "value") {
                *line = line.replacen(&format!(" value=\"{v}\""), " value=\"@COMMAND_LINE@\"", 1);
            }
        }
        for (id, token) in &converter_ids {
            if id.is_empty() || !line.contains(id.as_str()) {
                continue;
            }
            *line = line.replace(&format!("=\"{id}\""), &format!("=\"{token}\""));
        }
    }
    out.concat()
}

/// First point at which two normalized documents differ.
#[derive(Debug)]
pub struct Mismatch {
    pub line: usize,
    pub expected: String,
    pub actual: String,
    pub detail: Option<String>,
}

fn truncate(s: &str, n: usize) -> String {
    if s.len() <= n {
        s.trim_end().to_string()
    } else {
        let mut k = n;
        while !s.is_char_boundary(k) {
            k -= 1;
        }
        format!("{}… ({} bytes)", &s[..k], s.len())
    }
}

/// Look back from a `<binary>` line to find the encoding terms of its array.
fn array_encoding(lines: &[&str], binary_line: usize) -> (Precision, bool, Compression) {
    let (mut p, mut int, mut c) = (Precision::P64, false, Compression::None);
    for l in lines[..binary_line].iter().rev() {
        let t = l.trim_start();
        if t.starts_with("<binaryDataArray ") {
            break;
        }
        if t.contains("\"MS:1000521\"") {
            p = Precision::P32;
        } else if t.contains("\"MS:1000519\"") {
            (p, int) = (Precision::P32, true);
        } else if t.contains("\"MS:1000522\"") {
            int = true;
        } else if t.contains("\"MS:1000574\"") {
            c = Compression::Zlib;
        }
    }
    (p, int, c)
}

fn binary_detail(exp_lines: &[&str], act_lines: &[&str], idx: usize) -> Option<String> {
    let strip = |l: &str| {
        l.trim()
            .trim_start_matches("<binary>")
            .trim_end_matches("</binary>")
            .to_string()
    };
    let (pe, ie, ce) = array_encoding(exp_lines, idx);
    let (pa, ia, ca) = array_encoding(act_lines, idx);
    if (pe, ie, ce) != (pa, ia, ca) {
        return Some("array encoding differs".into());
    }
    let e = encode::decode(&strip(exp_lines[idx]), pe, ce, ie).ok()?;
    let a = encode::decode(&strip(act_lines[idx]), pa, ca, ia).ok()?;
    use mzx_mzml::model::ArrayData::*;
    match (e, a) {
        (Float(e), Float(a)) => {
            if e.len() != a.len() {
                return Some(format!("array length {} vs {}", e.len(), a.len()));
            }
            let (k, (x, y)) = e
                .iter()
                .zip(a.iter())
                .enumerate()
                .find(|(_, (x, y))| x.to_bits() != y.to_bits())?;
            let ulps = if pe == Precision::P32 {
                ((*x as f32).to_bits() as i64 - (*y as f32).to_bits() as i64).abs()
            } else {
                (x.to_bits() as i64 - y.to_bits() as i64).abs()
            };
            Some(format!(
                "first differing value at index {k}: expected {x:?}, actual {y:?} ({ulps} ULP)"
            ))
        }
        (Int(e), Int(a)) => {
            let k = e.iter().zip(a.iter()).position(|(x, y)| x != y)?;
            Some(format!(
                "first differing value at index {k}: expected {}, actual {}",
                e[k], a[k]
            ))
        }
        _ => Some("array type differs".into()),
    }
}

/// Byte comparison of two already-normalized documents.
pub fn compare(expected: &str, actual: &str) -> Option<Mismatch> {
    if expected == actual {
        return None;
    }
    let e: Vec<&str> = expected.split_inclusive('\n').collect();
    let a: Vec<&str> = actual.split_inclusive('\n').collect();
    for i in 0..e.len().max(a.len()) {
        let (x, y) = (
            e.get(i).copied().unwrap_or(""),
            a.get(i).copied().unwrap_or(""),
        );
        if x != y {
            let detail = if x.trim_start().starts_with("<binary>")
                && y.trim_start().starts_with("<binary>")
            {
                binary_detail(&e, &a, i)
            } else {
                None
            };
            return Some(Mismatch {
                line: i + 1,
                expected: truncate(x, 300),
                actual: truncate(y, 300),
                detail,
            });
        }
    }
    Some(Mismatch {
        line: 0,
        expected: "(documents differ only in line endings)".into(),
        actual: String::new(),
        detail: None,
    })
}

/// The contract check: `normalize(expected) == normalize(actual)`.
pub fn contract_compare(expected: &str, actual: &str) -> Option<Mismatch> {
    compare(&normalize(expected), &normalize(actual))
}

/// Writer proof for one document: `write(read(doc)) == doc`.
pub fn roundtrip(doc: &[u8]) -> Result<Option<Mismatch>> {
    let (msd, config) = mzx_mzml::read_mzml(doc)?;
    let out = mzx_mzml::write_mzml(Vec::with_capacity(doc.len()), &msd, &config)?;
    let expected = String::from_utf8_lossy(doc);
    let actual = String::from_utf8_lossy(&out);
    Ok(compare(&expected, &actual))
}

/// cvParam `MS:1000042` "peak intensity" (on `<selectedIon>`).
pub const MS_PEAK_INTENSITY: &str = "MS:1000042";
const MS_COMMAND_LINE: &str = "MS:1001885";

/// Lines a filter proof may not hold the filter port responsible for: precursor
/// `peak intensity` values, which the Thermo reader computes differently when
/// peak picking prefers the vendor (summed over the vendor-centroided precursor
/// scan). Only the `value` attribute may differ.
fn reader_owned_value(e: &str, a: &str) -> bool {
    let is_pi = |l: &str| {
        l.trim_start().starts_with("<cvParam ")
            && l.contains(&format!(" accession=\"{MS_PEAK_INTENSITY}\""))
    };
    if !(is_pi(e) && is_pi(a)) {
        return false;
    }
    let strip = |l: &str| match attr(l, "value") {
        Some(v) => l.replacen(&format!(" value=\"{v}\""), "", 1),
        None => l.to_string(),
    };
    strip(e) == strip(a)
}

/// Result of [`prove_filters`].
#[derive(Debug)]
pub struct FilterProof {
    /// First difference not explained by [`reader_owned_value`], if any.
    pub mismatch: Option<Mismatch>,
    /// How many precursor `peak intensity` lines differed (reader-owned).
    pub reader_owned_differences: usize,
    /// How many spectra the filters changed (arrays or params).
    pub spectra_changed: usize,
}

/// Filter proof: `write(filters(read(nofilter))) == default`, byte for byte, except
/// reader-owned precursor intensities (counted, not ignored silently). The
/// command-line cvParam is msconvert's argv and is copied from `default`.
pub fn prove_filters(
    nofilter: &[u8],
    default: &[u8],
    filters: &[mzx_filters::Filter],
) -> Result<FilterProof> {
    let (mut msd, config) = mzx_mzml::read_mzml(nofilter)?;
    let (golden, _) = mzx_mzml::read_mzml(default)?;
    let before = msd.run.spectra.clone();
    mzx_filters::apply(&mut msd, filters)?;
    let spectra_changed = before
        .iter()
        .zip(&msd.run.spectra)
        .filter(|(a, b)| a != b)
        .count();

    // msconvert stamps its argv on the first method of the first DataProcessing.
    let argv = golden
        .data_processing
        .first()
        .and_then(|d| d.methods.first())
        .and_then(|m| m.params.cv.iter().find(|p| p.accession == MS_COMMAND_LINE))
        .map(|p| p.value.clone());
    if let (Some(argv), Some(m)) = (
        argv,
        msd.data_processing
            .first_mut()
            .and_then(|d| d.methods.first_mut()),
    ) {
        match m
            .params
            .cv
            .iter_mut()
            .find(|p| p.accession == MS_COMMAND_LINE)
        {
            Some(p) => p.value = argv,
            None => m
                .params
                .cv
                .push(mzx_mzml::model::CvParam::new(MS_COMMAND_LINE, argv)),
        }
    }

    let out = mzx_mzml::write_mzml(Vec::with_capacity(default.len()), &msd, &config)?;
    let expected = String::from_utf8_lossy(default);
    let actual = String::from_utf8_lossy(&out);
    let e: Vec<&str> = expected.split_inclusive('\n').collect();
    let a: Vec<&str> = actual.split_inclusive('\n').collect();
    let mut reader_owned_differences = 0;
    let mut mismatch = None;
    if e.len() != a.len() {
        mismatch = compare(&expected, &actual);
    } else {
        for (i, (x, y)) in e.iter().zip(&a).enumerate() {
            if x == y {
                continue;
            }
            if reader_owned_value(x, y) {
                reader_owned_differences += 1;
                continue;
            }
            // reuse compare() for the binary decoding detail
            let sub_e = e[..=i].concat();
            let sub_a = a[..=i].concat();
            mismatch = compare(&sub_e, &sub_a).map(|mut m| {
                m.line = i + 1;
                m
            });
            break;
        }
    }
    Ok(FilterProof {
        mismatch,
        reader_owned_differences,
        spectra_changed,
    })
}

pub use mzx_convert::thermo_reader_options;

#[cfg(test)]
mod tests {
    use super::*;

    const PWIZ: &str = r#"  <softwareList count="2">
    <software id="Xcalibur" version="2.0">
      <cvParam cvRef="MS" accession="MS:1000532" name="Xcalibur" value=""/>
    </software>
    <software id="pwiz" version="3.0.25182">
      <cvParam cvRef="MS" accession="MS:1000615" name="ProteoWizard software" value=""/>
    </software>
  </softwareList>
      <sourceFile id="a.raw" name="a.raw" location="file:///data">
      <processingMethod order="0" softwareRef="pwiz">
"#;
    const MZX: &str = r#"  <softwareList count="2">
    <software id="Xcalibur" version="2.0">
      <cvParam cvRef="MS" accession="MS:1000532" name="Xcalibur" value=""/>
    </software>
    <software id="mzx" version="0.1.0">
      <userParam name="mzx converter"/>
    </software>
  </softwareList>
      <sourceFile id="a.raw" name="a.raw" location="file:///home/me/runs">
      <processingMethod order="0" softwareRef="mzx">
"#;

    #[test]
    fn normalization_equates_provenance_only() {
        assert!(
            contract_compare(PWIZ, MZX).is_none(),
            "{:?}",
            contract_compare(PWIZ, MZX)
        );
        // Anything else still differs.
        let tampered = MZX.replace("Xcalibur\" version=\"2.0", "Xcalibur\" version=\"2.1");
        assert!(contract_compare(PWIZ, &tampered).is_some());
    }

    /// pwiz 4.x shape: CRLF, `" />`, a command-line cvParam, a Wine path.
    const PWIZ4: &str = "    <software id=\"pwiz\" version=\"3.0.26056\">\r\n      <cvParam cvRef=\"MS\" accession=\"MS:1000615\" name=\"ProteoWizard software\" value=\"\" />\r\n    </software>\r\n      <sourceFile id=\"RAW1\" name=\"a.raw\" location=\"file:///Z:\\data\">\r\n      <processingMethod order=\"0\" softwareRef=\"pwiz\">\r\n        <cvParam cvRef=\"MS\" accession=\"MS:1001885\" name=\"command-line parameters\" value=\"/data/a.raw --mzML -o /out --outfile a.mzML\" />\r\n        <cvParam cvRef=\"MS\" accession=\"MS:1000035\" name=\"peak picking\" value=\"\" />\r\n      </processingMethod>";

    #[test]
    fn normalization_pwiz4_crlf_and_command_line() {
        let mzx = PWIZ4
            .replace("id=\"pwiz\" version=\"3.0.26056\"", "id=\"mzx\" version=\"0.1.0\"")
            .replace(
                "<cvParam cvRef=\"MS\" accession=\"MS:1000615\" name=\"ProteoWizard software\" value=\"\" />",
                "<userParam name=\"mzx converter\" />",
            )
            .replace("softwareRef=\"pwiz\"", "softwareRef=\"mzx\"")
            .replace("file:///Z:\\data", "file:///home/me")
            .replace("/data/a.raw --mzML -o /out --outfile a.mzML", "mzx a.raw");
        let n = normalize(PWIZ4);
        assert!(n.contains("<software id=\"@CONVERTER1@\" version=\"@VERSION@\"/>\r\n"));
        assert!(n.contains("value=\"@COMMAND_LINE@\""));
        assert!(
            contract_compare(PWIZ4, &mzx).is_none(),
            "{:?}",
            contract_compare(PWIZ4, &mzx)
        );
        // other cvParams on the same element are untouched
        let tampered = mzx.replace("name=\"peak picking\"", "name=\"peak  picking\"");
        assert!(contract_compare(PWIZ4, &tampered).is_some());
    }
}
