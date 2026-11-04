//! Port of the mzML path of `pwiz/data/msdata/BinaryDataEncoder.{cpp,cs}`:
//! down-convert to the configured precision, little-endian bytes, optional
//! zlib, then base64 (standard alphabet, padded, no line breaks). Numpress is
//! not supported (msconvert does not use it by default).
//!
//! The zlib *stream bytes* depend on the implementation:
//! - pwiz 3 (C++): boost `zlib_compressor` defaults == zlib `Z_DEFAULT_COMPRESSION`,
//!   15 window bits, memLevel 8 (`flate2` with the zlib backend).
//! - pwiz 4 (C#): .NET 10 `ZLibStream(CompressionLevel.Optimal)` == zlib-ng 2.2.5
//!   driven through 8 KiB output buffers ([`mzx_zng::deflate_dotnet`]).
//!
//! Empty arrays are never compressed by either (C++ skips; .NET's
//! `DeflateStream` writes nothing when no bytes were written).

use std::collections::HashMap;
use std::io::{Read, Write};

use anyhow::{bail, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;

use crate::dialect::Dialect;
use crate::model::ArrayData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Precision {
    P32,
    P64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Zlib,
}

/// pwiz `BinaryDataEncoder::Config` (the subset mzML uses), plus the
/// serializer generation the output must match.
#[derive(Debug, Clone, PartialEq)]
pub struct EncoderConfig {
    /// Which pwiz serializer (and zlib implementation) to reproduce.
    pub dialect: Dialect,
    pub precision: Precision,
    pub compression: Compression,
    /// Per array-type overrides, keyed by the array-type accession
    /// (pwiz `precisionOverrides[cvParamChild(MS_binary_data_array)]`).
    pub precision_overrides: HashMap<String, Precision>,
}

impl EncoderConfig {
    /// msconvert's defaults for that pwiz generation.
    ///
    /// - pwiz 3 (msconvert.cpp "precision defaults"): 64-bit, m/z 64-bit,
    ///   intensity 32-bit, uncompressed.
    /// - pwiz 4 (MsConvert/src/Config.cs `WriteConfig`): 64-bit, intensity
    ///   32-bit, zlib.
    pub fn msconvert_default(dialect: Dialect) -> Self {
        let mut precision_overrides = HashMap::new();
        precision_overrides.insert("MS:1000515".to_string(), Precision::P32); // intensity array
        let compression = match dialect {
            Dialect::Pwiz3Cpp => {
                precision_overrides.insert("MS:1000514".to_string(), Precision::P64); // m/z array
                Compression::None
            }
            Dialect::Pwiz4Net => Compression::Zlib,
        };
        Self {
            dialect,
            precision: Precision::P64,
            compression,
            precision_overrides,
        }
    }

    pub fn precision_for(&self, kind: Option<&str>) -> Precision {
        kind.and_then(|k| self.precision_overrides.get(k).copied())
            .unwrap_or(self.precision)
    }
}

/// Encode `data` to the base64 text pwiz writes inside `<binary>`.
pub fn encode(
    data: &ArrayData,
    precision: Precision,
    compression: Compression,
    dialect: Dialect,
) -> Result<String> {
    let mut bytes: Vec<u8> = Vec::with_capacity(data.len() * 8);
    match (data, precision) {
        (ArrayData::Float(v), Precision::P64) => v
            .iter()
            .for_each(|x| bytes.extend_from_slice(&x.to_le_bytes())),
        (ArrayData::Float(v), Precision::P32) => v
            .iter()
            .for_each(|x| bytes.extend_from_slice(&(*x as f32).to_le_bytes())),
        (ArrayData::Int(v), Precision::P64) => v
            .iter()
            .for_each(|x| bytes.extend_from_slice(&x.to_le_bytes())),
        (ArrayData::Int(v), Precision::P32) => v
            .iter()
            .for_each(|x| bytes.extend_from_slice(&(*x as i32).to_le_bytes())),
    }
    if compression == Compression::Zlib && !bytes.is_empty() {
        bytes = match dialect {
            Dialect::Pwiz3Cpp => {
                let mut enc =
                    flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
                enc.write_all(&bytes)?;
                enc.finish()?
            }
            Dialect::Pwiz4Net => mzx_zng::deflate_dotnet(&bytes),
        };
    }
    Ok(STANDARD.encode(bytes))
}

/// Decode `<binary>` text into the in-memory representation pwiz would hold.
pub fn decode(
    text: &str,
    precision: Precision,
    compression: Compression,
    integer: bool,
) -> Result<ArrayData> {
    let mut bytes = STANDARD.decode(text.trim()).context("base64 decode")?;
    if compression == Compression::Zlib && !bytes.is_empty() {
        let mut out = Vec::new();
        flate2::read::ZlibDecoder::new(&bytes[..])
            .read_to_end(&mut out)
            .context("zlib decode")?;
        bytes = out;
    }
    let width = if precision == Precision::P64 { 8 } else { 4 };
    if bytes.len() % width != 0 {
        bail!(
            "binary length {} is not a multiple of {}",
            bytes.len(),
            width
        );
    }
    let chunks = bytes.chunks_exact(width);
    Ok(match (integer, precision) {
        (false, Precision::P64) => ArrayData::Float(
            chunks
                .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
                .collect(),
        ),
        (false, Precision::P32) => ArrayData::Float(
            chunks
                .map(|c| f32::from_le_bytes(c.try_into().unwrap()) as f64)
                .collect(),
        ),
        (true, Precision::P64) => ArrayData::Int(
            chunks
                .map(|c| i64::from_le_bytes(c.try_into().unwrap()))
                .collect(),
        ),
        (true, Precision::P32) => ArrayData::Int(
            chunks
                .map(|c| i32::from_le_bytes(c.try_into().unwrap()) as i64)
                .collect(),
        ),
    })
}
