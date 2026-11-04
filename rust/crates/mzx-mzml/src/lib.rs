//! mzML model, reader and a ProteoWizard-byte-compatible writer.
//!
//! The writer ports pwiz's mzML serializer in both of its generations (see
//! [`Dialect`] and `CONTRACT.md` for the pinned pwiz builds). [`read::read_mzml`]
//! exists to prove the writer: `write(read(pwiz_output)) == pwiz_output`, byte
//! for byte.

pub mod cv;
pub mod dialect;
pub mod encode;
pub mod float;
pub mod model;
pub mod read;
pub mod write;
pub mod xmlwriter;

pub use dialect::Dialect;
pub use encode::EncoderConfig;
pub use model::MsData;
pub use read::read_mzml;
pub use write::write_mzml;
