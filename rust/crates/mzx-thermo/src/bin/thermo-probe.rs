//! Development probe: dump the decoded RAW structure.

use std::collections::BTreeMap;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: thermo-probe <file.raw> [what]");
    let what = args.next().unwrap_or_default();
    let raw = mzx_thermo::raw::RawFile::open(path.as_ref())?;
    let ms_i = raw.controllers.iter().position(|c| c.kind == 0).unwrap();
    let ms = &raw.run_headers[ms_i];
    let streams = || raw.ms_streams(ms);
    match what.as_str() {
        "trailer" => {
            let s = streams()?;
            for (i, f) in s.trailer_header.fields.iter().enumerate() {
                println!("{i}\t{}\t{}\t{:?}", f.kind, f.length, f.label);
            }
            println!("record size {}", s.trailer_header.record_size);
            for scan in [0usize, 1] {
                let v = s.trailer_header.record(
                    &raw.bytes,
                    s.trailer_records + scan * s.trailer_header.record_size,
                )?;
                println!("scan {}: {v:?}", scan + 1);
            }
        }
        "status" => {
            let s = streams()?;
            for f in &s.status_header.fields {
                println!("{}\t{}\t{:?}", f.kind, f.length, f.label);
            }
        }
        "events" => {
            let s = streams()?;
            let mut groups: BTreeMap<Vec<u8>, (usize, usize)> = BTreeMap::new();
            for (i, e) in s.events.iter().enumerate() {
                let g = groups.entry(e.preamble.to_vec()).or_insert((0, i));
                g.0 += 1;
            }
            for (p, (n, first)) in groups {
                let e = &s.events[first];
                println!("{n} events, first scan {}: preamble {:?}", first + 1, p);
                println!(
                    "   reactions {:?}\n   ranges {:?} params {:?} tail {:?}",
                    e.reactions, e.mass_ranges, e.conversion_params, e.tail
                );
            }
            for (si, seg) in s.templates.iter().enumerate() {
                for t in seg {
                    println!("template seg {si}: {:?}", t.preamble);
                }
            }
        }
        "index" => {
            let idx = raw.ms_scan_index(ms)?;
            let which: Vec<usize> = args.map(|a| a.parse::<usize>().unwrap() - 1).collect();
            for i in which {
                let e = &idx[i];
                println!("scan {}: {e:?}", i + 1);
                let start = (ms.data_addr + e.offset) as usize;
                let words: Vec<u32> = raw.bytes[start..start + 128]
                    .chunks(4)
                    .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
                    .collect();
                println!("  u32: {words:?}");
                let floats: Vec<f32> = words.iter().map(|w| f32::from_bits(*w)).collect();
                println!("  f32: {floats:?}");
            }
            let mut kinds = BTreeMap::new();
            for e in &idx {
                *kinds
                    .entry((e.unknown, e.scan_segment, e.scan_event))
                    .or_insert(0) += 1;
            }
            println!("(packet type, segment, event) counts: {kinds:?}");
        }
        "profile" => {
            let idx = raw.ms_scan_index(ms)?;
            let mut seen = std::collections::BTreeSet::new();
            for (i, e) in idx.iter().enumerate() {
                if e.unknown != 19 {
                    continue;
                }
                let start = (ms.data_addr + e.offset) as usize;
                let mut c = mzx_thermo::bytes::Cursor::new(&raw.bytes, start);
                let hdr: Vec<u32> = (0..8).map(|_| c.u32().unwrap()).collect();
                let (lo, hi) = (c.f32()?, c.f32()?);
                let first = c.f64()?;
                let step = c.f64()?;
                let nch = c.u32()?;
                let nb = c.u32()?;
                let mut ch = vec![];
                for _ in 0..nch {
                    let fb = c.u32()?;
                    let n = c.u32()?;
                    c.skip(4 * n as usize)?;
                    ch.push((fb, n));
                }
                let key =
                    format!("{hdr:?} {lo} {hi} first {first} step {step} nb {nb} chunks {ch:?}");
                if seen.insert(key.clone()) && seen.len() < 30 {
                    println!(
                        "scan {} seg {} used {}/{}: {key}",
                        i + 1,
                        e.scan_segment,
                        c.pos - start,
                        e.data_size
                    );
                }
            }
            println!("{} distinct", seen.len());
        }
        "method" => {
            println!("{:?}", raw.method);
            for m in raw.instrument_methods()? {
                println!("{m}");
            }
        }
        "pda" => {
            let pi = raw.controllers.iter().position(|c| c.kind == 3).unwrap();
            let rh = &raw.run_headers[pi];
            let idx = raw.pda_index(rh)?;
            for e in idx.iter().take(3) {
                let s = raw.pda_scan(rh, e)?;
                println!("{e:?}");
                println!(
                    "  start {} step {:?} first {} offset {} scale {} n {} ints {:?} head {:?}",
                    s.start,
                    s.step,
                    s.wl_first,
                    s.offset,
                    s.scale,
                    s.intensities.len(),
                    &s.intensities[..8],
                    s.header_ints
                );
                let wl: Vec<f64> = (0..4).map(|i| s.start + i as f64 * s.step).collect();
                println!(
                    "  wl {wl:?} last {:?}",
                    s.start + (s.intensities.len() - 1) as f64 * s.step
                );
            }
        }
        _ => {
            println!("header: {:?}", raw.header);
            for (ctl, rh) in raw.controllers.iter().zip(&raw.run_headers) {
                println!("--- controller {:?}", ctl);
                let mut rh = rh.clone();
                rh.file_names.retain(|s| !s.is_empty());
                println!("{rh:#?}");
            }
        }
    }
    Ok(())
}
