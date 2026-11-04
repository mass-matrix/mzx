//! Pure-Rust Thermo RAW reader (proof of concept, RAW v66 only).

pub mod bytes;
pub mod events;
pub mod filter;
pub mod generic;
pub mod method;
pub mod model;
pub mod raw;
pub mod reader;

/// The input is outside the proven coverage; the caller must fall back to
/// msconvert.
#[derive(Debug)]
pub struct Unsupported(pub String);

impl std::fmt::Display for Unsupported {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported Thermo input: {}", self.0)
    }
}

impl std::error::Error for Unsupported {}
