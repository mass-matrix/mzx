//! `mzx-verify`: contract tooling.
//!
//!   mzx-verify roundtrip <pwiz.mzML>...      writer proof: write(read(f)) == f
//!   mzx-verify cmp <golden.mzML> <mzx.mzML>   contract check (normalized bytes)
//!   mzx-verify normalize <in.mzML>            print the normalized document
//!   mzx-verify prove-filters <nofilter> <default>   filter proof (mzx default option set)

use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    version,
    about = "Verify mzx output against the ProteoWizard byte contract"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Prove the mzML writer: parse each file and re-serialize it byte-identically.
    Roundtrip { files: Vec<PathBuf> },
    /// Contract comparison of a pwiz golden against mzx output.
    Cmp { expected: PathBuf, actual: PathBuf },
    /// Print the contract-normalized form of a document.
    Normalize { file: PathBuf },
    /// Prove the `default` filter ports: write(filters(read(nofilter))) == default.
    ProveFilters { nofilter: PathBuf, default: PathBuf },
    /// Convert a Thermo RAW file with the Rust reader (proof-of-concept path).
    Convert {
        raw: PathBuf,
        out: PathBuf,
        /// Option set: `default` or `nofilter`.
        #[arg(long, default_value = "default")]
        options: String,
    },
    /// Debug: print spectra (by index) with params and decoded arrays.
    Dump {
        file: PathBuf,
        indices: Vec<usize>,
        /// Print at most this many array values per array.
        #[arg(long, default_value_t = 20)]
        values: usize,
        /// Index chromatograms instead of spectra.
        #[arg(long)]
        chrom: bool,
    },
}

fn read(p: &PathBuf) -> Result<Vec<u8>> {
    std::fs::read(p).with_context(|| format!("reading {}", p.display()))
}

fn report(m: &mzx_verify::Mismatch) {
    eprintln!("  first difference at line {}", m.line);
    eprintln!("  expected: {}", m.expected);
    eprintln!("  actual:   {}", m.actual);
    if let Some(d) = &m.detail {
        eprintln!("  detail:   {d}");
    }
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Roundtrip { files } => {
            let mut failed = 0;
            for f in &files {
                match mzx_verify::roundtrip(&read(f)?) {
                    Ok(None) => println!("IDENTICAL  {}", f.display()),
                    Ok(Some(m)) => {
                        failed += 1;
                        println!("DIFFERENT  {}", f.display());
                        report(&m);
                    }
                    Err(e) => {
                        failed += 1;
                        println!("ERROR      {}: {e:#}", f.display());
                    }
                }
            }
            Ok(if failed == 0 {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            })
        }
        Cmd::Cmp { expected, actual } => {
            let e = String::from_utf8(read(&expected)?)?;
            let a = String::from_utf8(read(&actual)?)?;
            match mzx_verify::contract_compare(&e, &a) {
                None => {
                    println!("IDENTICAL (normalized: converter software, sourceFile.location)");
                    Ok(ExitCode::SUCCESS)
                }
                Some(m) => {
                    println!("DIFFERENT");
                    report(&m);
                    Ok(ExitCode::FAILURE)
                }
            }
        }
        Cmd::ProveFilters { nofilter, default } => {
            let p = mzx_verify::prove_filters(
                &read(&nofilter)?,
                &read(&default)?,
                &mzx_filters::mzx_default_thermo(),
            )?;
            println!(
                "spectra changed by filters: {}; reader-owned precursor peak-intensity lines: {}",
                p.spectra_changed, p.reader_owned_differences
            );
            match p.mismatch {
                None => {
                    println!("IDENTICAL (except reader-owned values above)");
                    Ok(ExitCode::SUCCESS)
                }
                Some(m) => {
                    println!("DIFFERENT");
                    report(&m);
                    Ok(ExitCode::FAILURE)
                }
            }
        }
        Cmd::Convert { raw, out, options } => {
            // msconvert records its own argv; mzx records the equivalent.
            let argv: Vec<String> = std::env::args().collect();
            mzx_convert::convert_thermo_file(&raw, &out, options.parse()?, &argv.join(" "))?;
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Dump {
            file,
            indices,
            values,
            chrom,
        } => {
            let (doc, _) = mzx_mzml::read_mzml(&read(&file)?)?;
            for i in indices {
                let arrays = if chrom {
                    let mut c = doc.run.chromatograms[i].clone();
                    let arrays = std::mem::take(&mut c.arrays);
                    println!("{c:#?}");
                    arrays
                } else {
                    let mut s = doc.run.spectra[i].clone();
                    let arrays = std::mem::take(&mut s.arrays);
                    println!("{s:#?}");
                    arrays
                };
                for a in arrays {
                    let kind = a.kind().unwrap_or("?").to_string();
                    match a.data {
                        mzx_mzml::model::ArrayData::Float(v) => {
                            println!("{kind} n={} {:?}", v.len(), &v[..v.len().min(values)])
                        }
                        mzx_mzml::model::ArrayData::Int(v) => {
                            println!("{kind} n={} {:?}", v.len(), &v[..v.len().min(values)])
                        }
                    }
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        Cmd::Normalize { file } => {
            print!(
                "{}",
                mzx_verify::normalize(&String::from_utf8(read(&file)?)?)
            );
            Ok(ExitCode::SUCCESS)
        }
    }
}
