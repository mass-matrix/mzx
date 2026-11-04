//! Self-describing "generic data" records: the trailer extra (scan
//! parameters), tune data and status log share this header + record format.

use anyhow::{bail, Result};

use crate::bytes::Cursor;

#[derive(Debug, Clone)]
pub struct Field {
    pub kind: u32,
    pub length: u32,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Type 0: section heading / gap, no bytes in the record.
    None,
    I8(i8),
    /// Types 2-4: bool rendered true/false, yes/no, on/off.
    Bool(u32, bool),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    F32(f32),
    F64(f64),
    Str(String),
}

impl Field {
    pub fn size(&self) -> Result<usize> {
        Ok(match self.kind {
            0 => 0,
            1..=5 => 1,
            6 | 7 => 2,
            8..=10 => 4,
            11 => 8,
            12 => self.length as usize,
            13 => self.length as usize * 2,
            k => bail!("unknown generic field type {k} ({:?})", self.label),
        })
    }

    fn decode(&self, c: &mut Cursor) -> Result<Value> {
        Ok(match self.kind {
            0 => Value::None,
            1 => Value::I8(c.u8()? as i8),
            2..=4 => Value::Bool(self.kind, c.u8()? != 0),
            5 => Value::U8(c.u8()?),
            6 => Value::I16(c.u16()? as i16),
            7 => Value::U16(c.u16()?),
            8 => Value::I32(c.i32()?),
            9 => Value::U32(c.u32()?),
            10 => Value::F32(c.f32()?),
            11 => Value::F64(c.f64()?),
            12 => {
                let b = c.take(self.length as usize)?;
                let end = b.iter().position(|&x| x == 0).unwrap_or(b.len());
                Value::Str(b[..end].iter().map(|&x| x as char).collect())
            }
            13 => Value::Str(crate::bytes::utf16_until_nul(
                c.take(self.length as usize * 2)?,
            )),
            k => bail!("unknown generic field type {k}"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct Header {
    pub fields: Vec<Field>,
    pub record_size: usize,
}

impl Header {
    pub fn read(c: &mut Cursor) -> Result<Self> {
        let n = c.u32()? as usize;
        if n > 10_000 {
            bail!("implausible generic header field count {n}");
        }
        let mut fields = Vec::with_capacity(n);
        let mut record_size = 0;
        for _ in 0..n {
            let f = Field {
                kind: c.u32()?,
                length: c.u32()?,
                label: c.pascal()?,
            };
            record_size += f.size()?;
            fields.push(f);
        }
        Ok(Self {
            fields,
            record_size,
        })
    }

    pub fn index_of(&self, label: &str) -> Option<usize> {
        self.fields.iter().position(|f| f.label == label)
    }

    /// Decode the record starting at `pos`.
    pub fn record(&self, buf: &[u8], pos: usize) -> Result<Vec<Value>> {
        let mut c = Cursor::new(buf, pos);
        self.fields.iter().map(|f| f.decode(&mut c)).collect()
    }

    /// Decode a single field of the record at `pos`.
    pub fn value(&self, buf: &[u8], pos: usize, i: usize) -> Result<Value> {
        let off: usize = self.fields[..i].iter().map(|f| f.size().unwrap_or(0)).sum();
        let mut c = Cursor::new(buf, pos + off);
        self.fields[i].decode(&mut c)
    }
}
