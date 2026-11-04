//! Reader proof against the golden corpus (skipped without `MZX_CORPUS`):
//! for every `[[proven]]` row of `support_matrix.toml`,
//! `mzx_convert::convert_thermo(raw)` (read, filters, mzx branding) ==
//! `<stem>.<optset>.mzML` after the contract normalization.
//! `[[unsupported]]` rows must be refused with an `mzx_convert::is_unsupported`
//! error, never converted.
//!
//! Raw files are looked up in `$MZX_RAW` if set, else `$MZX_CORPUS/raw`.

use std::path::{Path, PathBuf};

/// `files = [...]` and `option_sets = [...]` of each table, in order. A
/// minimal reader for the flat format of support_matrix.toml.
fn matrix() -> Vec<(String, Vec<String>, Vec<String>)> {
    let text = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../support_matrix.toml"),
    )
    .unwrap();
    let list = |line: &str| -> Vec<String> {
        line.split_once('=')
            .unwrap()
            .1
            .trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .split(',')
            .map(|s| s.trim().trim_matches('"').to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };
    let mut rows: Vec<(String, Vec<String>, Vec<String>)> = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(t) = line.strip_prefix("[[").and_then(|l| l.strip_suffix("]]")) {
            rows.push((t.to_string(), vec![], vec![]));
        } else if line.starts_with("files") {
            rows.last_mut().unwrap().1 = list(line);
        } else if line.starts_with("option_sets") {
            rows.last_mut().unwrap().2 = list(line);
        }
    }
    rows
}

/// The shipped pipeline (`mzx_convert`), including mzx's converter branding.
fn convert(raw: &Path, option_set: &str) -> anyhow::Result<Vec<u8>> {
    let msd = mzx_convert::convert_thermo(raw, option_set.parse()?, "mzx-verify test")?;
    mzx_convert::write(Vec::new(), &msd)
}

#[test]
fn thermo_reader_reproduces_msconvert() {
    let Some(corpus) = std::env::var_os("MZX_CORPUS").map(PathBuf::from) else {
        eprintln!("MZX_CORPUS not set; skipping");
        return;
    };
    let raw_dir = std::env::var_os("MZX_RAW")
        .map(PathBuf::from)
        .unwrap_or_else(|| corpus.join("raw"));
    let mut proven = 0;
    for (kind, files, option_sets) in matrix() {
        for f in &files {
            let raw = raw_dir.join(f);
            assert!(raw.exists(), "{} missing", raw.display());
            let stem = f.strip_suffix(".raw").unwrap();
            match kind.as_str() {
                "proven" => {
                    assert!(!option_sets.is_empty());
                    for o in &option_sets {
                        let golden = std::fs::read_to_string(
                            corpus.join("goldens").join(format!("{stem}.{o}.mzML")),
                        )
                        .unwrap();
                        let out = String::from_utf8(convert(&raw, o).unwrap()).unwrap();
                        let m = mzx_verify::contract_compare(&golden, &out);
                        assert!(m.is_none(), "{stem}.{o}: {m:?}");
                        eprintln!("{stem}.{o}: IDENTICAL");
                        proven += 1;
                    }
                }
                "unsupported" => {
                    assert!(!option_sets.is_empty());
                    for o in &option_sets {
                        let e = convert(&raw, o)
                            .err()
                            .unwrap_or_else(|| panic!("{stem}.{o} converted but is not proven"));
                        assert!(
                            mzx_convert::is_unsupported(&e),
                            "{stem}.{o}: expected Unsupported, got {e:#}"
                        );
                        eprintln!("{stem}.{o}: refused ({e})");
                    }
                }
                k => panic!("unknown support matrix table {k}"),
            }
        }
    }
    assert!(proven > 0, "support matrix proves nothing");
}
