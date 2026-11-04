//! Which ProteoWizard serializer the output must be byte-identical to.
//!
//! pwiz replaced its C++ code base with a C# port (pwiz-sharp) in Oct 2026
//! (commit 6dd40d8, "Retire the C++ tree"). The two write the same mzML
//! information but differ at the byte level, so the writer models both:
//!
//! | | [`Dialect::Pwiz3Cpp`] | [`Dialect::Pwiz4Net`] |
//! |---|---|---|
//! | reference | `pwiz/data/msdata/IO.cpp` + `XMLWriter.cpp` @ 4100417b (3.0.25182) | `pwiz/data/msdata/MzmlWriter.cs` @ fb75aff (4.0.26299), .NET `XmlWriter` |
//! | line ending | `\n` after every line | `\r\n` *between* lines (none at EOF) |
//! | empty element | `<x a="1"/>` | `<x a="1" />` |
//! | attribute escaping | `& " ' < >` | `& " < >`, plus `\t \n \r` as char refs |
//! | zlib | zlib 1.x `Z_DEFAULT_COMPRESSION` | zlib-ng 2.2.5 via `ZLibStream` ([`mzx_zng`]) |
//! | CV tables | psi-ms 4.1.182 / unit 09:04:2014 | psi-ms 4.1.257 / unit releases/2026-01-16 |
//!
//! plus a handful of element/attribute order differences handled in `write.rs`.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dialect {
    /// pwiz 3.x C++ msconvert.
    Pwiz3Cpp,
    /// pwiz 4.x C# msconvert (pwiz-sharp on .NET 10). The mzx contract target.
    #[default]
    Pwiz4Net,
}

impl Dialect {
    /// Identify which serializer wrote `xml` from its first line ending:
    /// .NET's `XmlWriter` uses `Environment.NewLine` (CRLF under Windows/Wine).
    pub fn sniff(xml: &[u8]) -> Dialect {
        let head = &xml[..xml.len().min(256)];
        match head.iter().position(|&b| b == b'\n') {
            Some(i) if i > 0 && head[i - 1] == b'\r' => Dialect::Pwiz4Net,
            _ => Dialect::Pwiz3Cpp,
        }
    }
}
