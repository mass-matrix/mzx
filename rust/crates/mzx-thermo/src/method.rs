//! Isolation widths from the embedded instrument-method text: a port of
//! `ThermoRawFile.ParseInstrumentMethodText` (pwiz C# @ fb75aff, itself a port
//! of cpp `RawFile.cpp parseInstrumentMethod`). Regexes are copied verbatim.

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

macro_rules! re {
    ($name:ident, $pat:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pat).unwrap());
    };
}

re!(SEGMENT, r"^\s*Segment (\d+) Information\s*$");
re!(SCAN_EVENT, r"^\s*(\d+):.*$");
re!(ISOLATION_WIDTH, r"^\s*Isolation Width:\s*(\S+)\s*$");
re!(ISO_W, r"^\s*MS.*:.*\s+IsoW\s+(\S+)\s*$");
re!(
    REPEATED_EVENT,
    r"^\s*Scan Event (\d+) repeated for top (\d+)\s*$"
);
re!(
    DEFAULT_ISO_WIDTH,
    r"^\s*MS(\d+) Isolation Width:\s*(\S+)\s*$"
);
re!(INSTRUMENT_MODEL, r"^\s*Instrument model\s*-\s*(.+?)\s*$");
re!(
    DEFAULT_ISO_WINDOW,
    r"^\s*Isolation Window \(m/z\) =\s*(\S+)\s*$"
);

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MethodWidths {
    /// segment -> scan event -> width (1-based, as in the method text).
    pub by_event: HashMap<i32, HashMap<i32, f64>>,
    /// segment -> ms level -> default width.
    pub default_by_level: HashMap<i32, HashMap<i32, f64>>,
    /// "Instrument model - X" (last occurrence), empty if none.
    pub instrument_model: String,
}

impl MethodWidths {
    pub fn event_width(&self, segment: i32, event: i32) -> f64 {
        self.by_event
            .get(&segment)
            .and_then(|m| m.get(&event))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn default_width(&self, segment: i32, ms_level: i32) -> f64 {
        self.default_by_level
            .get(&segment)
            .and_then(|m| m.get(&ms_level))
            .copied()
            .unwrap_or(0.0)
    }
}

/// `TextReader.ReadLine` line splitting: `\r\n`, `\r` or `\n`.
fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let b = text.as_bytes();
    let (mut start, mut i) = (0, 0);
    while i < b.len() {
        match b[i] {
            b'\r' => {
                out.push(&text[start..i]);
                i += if b.get(i + 1) == Some(&b'\n') { 2 } else { 1 };
                start = i;
            }
            b'\n' => {
                out.push(&text[start..i]);
                i += 1;
                start = i;
            }
            _ => i += 1,
        }
    }
    if start < b.len() {
        out.push(&text[start..]);
    }
    out
}

/// .NET `double.TryParse(s, NumberStyles.Float, InvariantCulture)` for the
/// plain decimal forms method text uses.
fn parse_f64(s: &str) -> Option<f64> {
    if s.bytes()
        .all(|c| c.is_ascii_digit() || matches!(c, b'.' | b'-' | b'+' | b'e' | b'E'))
    {
        s.parse().ok()
    } else {
        None
    }
}

fn contains_ci(line: &str, needle: &str) -> bool {
    line.to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

pub fn parse(text: &str) -> MethodWidths {
    let mut w = MethodWidths::default();
    let (mut segment, mut scan_event) = (1i32, 0i32);
    let mut in_details = false;
    let mut in_dd = false;
    for line in lines(text) {
        if let Some(m) = SEGMENT.captures(line) {
            if let Ok(s) = m[1].parse() {
                segment = s;
            }
            continue;
        }
        if let Some(m) = INSTRUMENT_MODEL.captures(line) {
            w.instrument_model = m[1].to_string();
            continue;
        }
        if contains_ci(line, "Scan Event Details") {
            in_details = true;
            continue;
        }
        if in_details {
            if let Some(m) = SCAN_EVENT.captures(line) {
                if let Ok(e) = m[1].parse() {
                    scan_event = e;
                }
                continue;
            }
            if let Some(m) = ISOLATION_WIDTH
                .captures(line)
                .or_else(|| ISO_W.captures(line))
            {
                if let Some(v) = parse_f64(&m[1]) {
                    w.by_event.entry(segment).or_default().insert(scan_event, v);
                }
                continue;
            }
            if let Some(m) = REPEATED_EVENT.captures(line) {
                let (base, count): (i32, i32) =
                    (m[1].parse().unwrap_or(0), m[2].parse().unwrap_or(0));
                if let Some(d) = w.by_event.get_mut(&segment) {
                    if let Some(&bw) = d.get(&base) {
                        for i in base + 1..base + count {
                            d.insert(i, bw);
                        }
                    }
                }
                continue;
            }
            if line.trim().is_empty() {
                in_details = false;
            }
        }
        if contains_ci(line, "Data Dependent Settings") || contains_ci(line, "Scan DIAScan") {
            in_dd = true;
            continue;
        }
        if in_dd {
            if let Some(m) = DEFAULT_ISO_WIDTH.captures(line) {
                if let (Ok(l), Some(v)) = (m[1].parse(), parse_f64(&m[2])) {
                    w.default_by_level.entry(segment).or_default().insert(l, v);
                }
                continue;
            }
            if let Some(m) = DEFAULT_ISO_WINDOW.captures(line) {
                if let Some(v) = parse_f64(&m[1]) {
                    w.default_by_level.entry(segment).or_default().insert(2, v);
                }
                continue;
            }
            if line.trim().is_empty() {
                in_dd = false;
            }
        }
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_segments_events_and_defaults() {
        let t = "Segment 1 Information\r\n\r\nScan Event Details:\r\n 1:  ITMS - p norm\r\n \
                 2:  ITMS - c norm Dep MS/MS\r\n       Isolation Width:         3.00\r\n\r\n\
                 Segment 2 Information\r\nScan Event Details:\r\n 2:  x\r\n  Isolation Width: 2.0\r\n\
                 Scan Event 2 repeated for top 3\r\n\r\nData Dependent Settings:\r\n  MS2 Isolation Width: 1.6\r\n\r\n";
        let w = parse(t);
        assert_eq!(w.event_width(1, 2), 3.0);
        assert_eq!(w.event_width(1, 1), 0.0);
        assert_eq!(w.event_width(2, 2), 2.0);
        assert_eq!(w.event_width(2, 4), 2.0);
        assert_eq!(w.event_width(2, 5), 0.0);
        assert_eq!(w.default_width(2, 2), 1.6);
        assert_eq!(w.default_width(1, 2), 0.0);
    }
}
