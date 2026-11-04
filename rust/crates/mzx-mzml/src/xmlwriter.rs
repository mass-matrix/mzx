//! Port of `pwiz/utility/minimxml/XMLWriter.cpp` (pwiz 3.0.25182 era), plus the
//! byte-level behaviour of the .NET `System.Xml.XmlWriter` (`Indent = true`,
//! `IndentChars = "  "`) that pwiz 4.x serializes through; see [`Dialect`].
//!
//! Byte-for-byte behaviour that matters: 2-space indentation per open element,
//! attribute and text escaping rules, empty-element handling, the
//! `InlineInner` style used for `<binary>`, line endings, and `encode_xml_id`.

use std::io::{self, Write};

use crate::dialect::Dialect;

pub const STYLE_INLINE_INNER: u32 = 0x01;
pub const STYLE_INLINE_OUTER: u32 = 0x02;

const INDENTATION_STEP: usize = 2;

pub type Attributes = Vec<(&'static str, String)>;

pub struct XmlWriter<W: Write> {
    out: W,
    position: u64,
    elements: Vec<&'static str>,
    styles: Vec<u32>,
    dialect: Dialect,
    /// .NET writes the line break *before* the next line, so the document's
    /// last line has none. Pending breaks are flushed by the next write.
    pending_newline: bool,
}

impl<W: Write> XmlWriter<W> {
    pub fn new(out: W, dialect: Dialect) -> Self {
        Self {
            out,
            position: 0,
            elements: Vec::new(),
            styles: vec![0],
            dialect,
            pending_newline: false,
        }
    }

    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    pub fn into_inner(self) -> W {
        self.out
    }

    /// Bytes written so far (pwiz `XMLWriter::position`).
    pub fn position(&self) -> u64 {
        self.position
    }

    /// pwiz `XMLWriter::positionNext`: where the next element's `<` will land.
    pub fn position_next(&self) -> u64 {
        let mut p = self.position;
        if self.pending_newline {
            p += 2;
        }
        if !self.style(STYLE_INLINE_OUTER) {
            p += (self.elements.len() * INDENTATION_STEP) as u64;
        }
        p
    }

    fn raw(&mut self, bytes: &[u8]) -> io::Result<()> {
        if self.pending_newline {
            self.pending_newline = false;
            self.position += 2;
            self.out.write_all(b"\r\n")?;
        }
        self.position += bytes.len() as u64;
        self.out.write_all(bytes)
    }

    fn newline(&mut self) -> io::Result<()> {
        match self.dialect {
            Dialect::Pwiz3Cpp => self.raw(b"\n"),
            Dialect::Pwiz4Net => {
                self.pending_newline = true;
                Ok(())
            }
        }
    }

    fn indent(&mut self, depth: usize) -> io::Result<()> {
        const SPACES: [u8; 256] = [b' '; 256];
        let mut n = depth * INDENTATION_STEP;
        while n > 0 {
            let k = n.min(SPACES.len());
            self.raw(&SPACES[..k])?;
            n -= k;
        }
        Ok(())
    }

    fn style(&self, flag: u32) -> bool {
        self.styles.last().copied().unwrap_or(0) & flag != 0
    }

    pub fn push_style(&mut self, flags: u32) {
        self.styles.push(flags);
    }

    pub fn pop_style(&mut self) {
        self.styles.pop();
        assert!(
            !self.styles.is_empty(),
            "[XMLWriter] Style stack underflow."
        );
    }

    pub fn processing_instruction(&mut self, name: &str, data: &str) -> io::Result<()> {
        self.indent(self.elements.len())?;
        let s = format!("<?{name} {data}?>");
        self.raw(s.as_bytes())?;
        self.newline()
    }

    pub fn start_element(
        &mut self,
        name: &'static str,
        attributes: &[(&'static str, String)],
        empty: bool,
    ) -> io::Result<()> {
        if !self.style(STYLE_INLINE_OUTER) {
            self.indent(self.elements.len())?;
        }
        let mut buf = Vec::with_capacity(64);
        buf.push(b'<');
        buf.extend_from_slice(name.as_bytes());
        for (k, v) in attributes {
            buf.push(b' ');
            buf.extend_from_slice(k.as_bytes());
            buf.extend_from_slice(b"=\"");
            escape_attribute(&mut buf, v, self.dialect);
            buf.push(b'"');
        }
        buf.extend_from_slice(match (empty, self.dialect) {
            (false, _) => b">",
            (true, Dialect::Pwiz3Cpp) => b"/>",
            (true, Dialect::Pwiz4Net) => b" />",
        });
        self.raw(&buf)?;
        if !self.style(STYLE_INLINE_INNER) || (!self.style(STYLE_INLINE_OUTER) && empty) {
            self.newline()?;
        }
        if !empty {
            self.elements.push(name);
        }
        Ok(())
    }

    pub fn start(
        &mut self,
        name: &'static str,
        attributes: &[(&'static str, String)],
    ) -> io::Result<()> {
        self.start_element(name, attributes, false)
    }

    pub fn empty(
        &mut self,
        name: &'static str,
        attributes: &[(&'static str, String)],
    ) -> io::Result<()> {
        self.start_element(name, attributes, true)
    }

    pub fn end_element(&mut self) -> io::Result<()> {
        let name = self
            .elements
            .pop()
            .expect("[XMLWriter] Element stack underflow.");
        if !self.style(STYLE_INLINE_INNER) {
            self.indent(self.elements.len())?;
        }
        let mut buf = Vec::with_capacity(name.len() + 3);
        buf.extend_from_slice(b"</");
        buf.extend_from_slice(name.as_bytes());
        buf.push(b'>');
        self.raw(&buf)?;
        if !self.style(STYLE_INLINE_OUTER) {
            self.newline()?;
        }
        Ok(())
    }

    pub fn characters(&mut self, text: &str, auto_escape: bool) -> io::Result<()> {
        if !self.style(STYLE_INLINE_INNER) {
            self.indent(self.elements.len())?;
        }
        if auto_escape {
            let mut buf = Vec::with_capacity(text.len());
            escape_text(&mut buf, text, self.dialect);
            self.raw(&buf)?;
        } else {
            self.raw(text.as_bytes())?;
        }
        if !self.style(STYLE_INLINE_INNER) {
            self.newline()?;
        }
        Ok(())
    }
}

/// C++ `XMLWriter::Attributes` escaping vs .NET `XmlEncodedRawTextWriter.WriteAttributeTextBlock`
/// (`NewLineHandling.Replace`: tab/CR/LF become character references; `'` is left alone).
fn escape_attribute(buf: &mut Vec<u8>, s: &str, dialect: Dialect) {
    for &c in s.as_bytes() {
        match (c, dialect) {
            (b'&', _) => buf.extend_from_slice(b"&amp;"),
            (b'"', _) => buf.extend_from_slice(b"&quot;"),
            (b'<', _) => buf.extend_from_slice(b"&lt;"),
            (b'>', _) => buf.extend_from_slice(b"&gt;"),
            (b'\'', Dialect::Pwiz3Cpp) => buf.extend_from_slice(b"&apos;"),
            (b'\t', Dialect::Pwiz4Net) => buf.extend_from_slice(b"&#x9;"),
            (b'\n', Dialect::Pwiz4Net) => buf.extend_from_slice(b"&#xA;"),
            (b'\r', Dialect::Pwiz4Net) => buf.extend_from_slice(b"&#xD;"),
            _ => buf.push(c),
        }
    }
}

/// Element text. pwiz only writes text inside `<binary>` (base64), so the .NET
/// newline normalization of text content never comes into play; refuse it
/// rather than guess.
fn escape_text(buf: &mut Vec<u8>, s: &str, dialect: Dialect) {
    for &c in s.as_bytes() {
        match c {
            b'&' => buf.extend_from_slice(b"&amp;"),
            b'<' => buf.extend_from_slice(b"&lt;"),
            b'>' => buf.extend_from_slice(b"&gt;"),
            b'\n' | b'\r' if dialect == Dialect::Pwiz4Net => {
                panic!("[XMLWriter] newline in element text is not modelled for pwiz 4.x")
            }
            _ => buf.push(c),
        }
    }
}

fn is_ncname_start(c: u8) -> bool {
    c.is_ascii_alphabetic() || c == b'_'
}

fn is_ncname_char(c: u8) -> bool {
    is_ncname_start(c) || c.is_ascii_digit() || c == b'.' || c == b'-'
}

/// pwiz `encode_xml_id_copy` (C++) / `XmlIdEncoding.Encode` (C#); identical output.
///
/// Each non-NCName byte `c` becomes `_x00HH_`.
pub fn encode_xml_id(id: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let src = id.as_bytes();
    assert!(
        !src.is_empty(),
        "[encode_xml_id] xml:IDs and xml:IDREFs cannot be empty strings"
    );
    let mut out: Vec<u8> = Vec::with_capacity(src.len() + 8);
    for (i, &c) in src.iter().enumerate() {
        let ok = if i == 0 {
            is_ncname_start(c)
        } else {
            is_ncname_char(c)
        };
        if ok {
            out.push(c);
        } else {
            out.extend_from_slice(b"_x00");
            out.push(HEX[((c & 0xF0) >> 4) as usize]);
            out.push(HEX[(c & 0x0F) as usize]);
            out.push(b'_');
        }
    }
    String::from_utf8(out).expect("encode_xml_id produced invalid UTF-8")
}

/// Inverse of [`encode_xml_id`] for ids pwiz wrote: `_x00HH_` -> byte.
pub fn decode_xml_id(id: &str) -> String {
    let b = id.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if i + 7 <= b.len() && &b[i..i + 4] == b"_x00" && b[i + 6] == b'_' {
            if let Ok(v) = u8::from_str_radix(&id[i + 4..i + 6], 16) {
                out.push(v);
                i += 7;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_matches_pwiz() {
        let raw = "Malat_peptide_neg_MSMS.d\\Analysis.baf";
        let enc = "Malat_peptide_neg_MSMS.d_x005c_Analysis.baf";
        assert_eq!(encode_xml_id(raw), enc);
        assert_eq!(decode_xml_id(enc), raw);
        assert_eq!(encode_xml_id("1abc"), "_x0031_abc");
        assert_eq!(encode_xml_id("a  b"), "a_x0020__x0020_b");
    }

    fn render(d: Dialect) -> String {
        let mut w = XmlWriter::new(Vec::new(), d);
        w.processing_instruction("xml", "version=\"1.0\" encoding=\"utf-8\"")
            .unwrap();
        w.start("a", &[("x", "it's \"q\"\t".to_string())]).unwrap();
        w.empty("b", &[]).unwrap();
        w.push_style(STYLE_INLINE_INNER);
        w.start("binary", &[]).unwrap();
        w.characters("AAAA", false).unwrap();
        w.end_element().unwrap();
        w.pop_style();
        w.end_element().unwrap();
        String::from_utf8(w.into_inner()).unwrap()
    }

    #[test]
    fn dialect_bytes() {
        assert_eq!(
            render(Dialect::Pwiz3Cpp),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<a x=\"it&apos;s &quot;q&quot;\t\">\n  <b/>\n  <binary>AAAA</binary>\n</a>\n"
        );
        assert_eq!(
            render(Dialect::Pwiz4Net),
            "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n<a x=\"it's &quot;q&quot;&#x9;\">\r\n  <b />\r\n  <binary>AAAA</binary>\r\n</a>"
        );
    }
}
