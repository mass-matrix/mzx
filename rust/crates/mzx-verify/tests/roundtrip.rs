//! Writer proof over real ProteoWizard output.
//!
//! Inputs: every `*.mzML` in `$MZX_CORPUS/goldens`, plus any paths in
//! `$MZX_ROUNDTRIP_FILES` (colon-separated). Skips when neither is set.

use std::path::PathBuf;

fn inputs() -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(dir) = std::env::var("MZX_CORPUS") {
        if let Ok(rd) = std::fs::read_dir(PathBuf::from(dir).join("goldens")) {
            v.extend(rd.flatten().map(|e| e.path()).filter(|p| {
                p.extension()
                    .is_some_and(|e| e.eq_ignore_ascii_case("mzml"))
            }));
        }
    }
    if let Ok(files) = std::env::var("MZX_ROUNDTRIP_FILES") {
        v.extend(
            files
                .split(':')
                .filter(|s| !s.is_empty())
                .map(PathBuf::from),
        );
    }
    v.sort();
    v
}

#[test]
fn writer_roundtrip_is_byte_identical() {
    let files = inputs();
    if files.is_empty() {
        eprintln!("skipping: set MZX_CORPUS or MZX_ROUNDTRIP_FILES");
        return;
    }
    let mut failures = Vec::new();
    for f in &files {
        let doc = std::fs::read(f).unwrap();
        match mzx_verify::roundtrip(&doc) {
            Ok(None) => {}
            Ok(Some(m)) => failures.push(format!(
                "{}: line {}\n  expected: {}\n  actual:   {}",
                f.display(),
                m.line,
                m.expected,
                m.actual
            )),
            Err(e) => failures.push(format!("{}: {e:#}", f.display())),
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
