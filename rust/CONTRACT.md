# mzx ↔ msconvert output contract

mzx's Rust backend may only emit output that is proven identical to what
ProteoWizard `msconvert` produces for the same input and options. This
document defines "identical" and how it is proven.

## Reference implementation

- `msconvert` from `chambm/pwiz-skyline-i-agree-to-the-vendor-licenses`, **pinned by digest**:
  `sha256:8556d8c771c06f803f2fb54c280a293581a4a24a6acae8830d6147a6028323f3`,
  "ProteoWizard release: 4.0.26299". The digest and release string are recorded in
  each golden's `manifest.jsonl` row (`scripts/make_goldens.sh`). Changing the
  digest means regenerating every golden and re-running every proof.
- pwiz 4.x is the C# port (pwiz-sharp, .NET 10) that replaced the C++ tree in
  Oct 2026. The release number is the git commit date (`4.0.YYDDD`); 26299 is
  2026-10-26. Every commit that day (and back to 2026-10-24) has identical
  `pwiz/data/msdata`, `pwiz/data/common`, `pwiz/data/vendor_readers/Thermo`,
  `pwiz/analysis` and `MsConvert` sources, so the writer is ported from
  **`fb75aff`** (the day's last commit): `pwiz/data/msdata/MzmlWriter.cs`,
  `MzmlXml.cs`, `BinaryDataEncoder.cs`, `pwiz/data/common/PwizFloat.cs`.
  New goldens record the release hash (`4.0.NNNNN (hash)`), which pins this exactly.
- Serialization goes through .NET `System.Xml.XmlWriter` (`Indent`, two spaces),
  which under Wine means CRLF line breaks *between* lines and none at EOF,
  `<x a="1" />` empty elements, and .NET attribute escaping.
- zlib is .NET 10's `ZLibStream(CompressionLevel.Optimal)`: **zlib-ng 2.2.5**
  (dotnet/runtime `release/10.0` `src/native/external/zlib-ng`; its
  `cgmanifest.json` still says 2.2.1, which does *not* reproduce the goldens),
  level 6, window 15, memLevel 8, fed through `DeflateStream`'s 8 KiB output
  buffer. Vendored unmodified in `crates/mzx-zng`; any zlib-ng change is a
  contract change.
- CV tables (`crates/mzx-mzml/data/pwiz4/*.obo`: PSI-MS 4.1.257, UO
  releases/2026-01-16) are copied from `fb75aff`.
- The writer also keeps the pwiz 3 C++ dialect (commit `4100417b`, ≈3.0.25182;
  `data/pwiz3/*.obo`) as a regression target. See `crates/mzx-mzml/src/dialect.rs`
  for the byte-level differences between the two.

## Inputs

A contract case is `(raw input bytes, option set)`.

| Option set | msconvert arguments | Notes |
|---|---|---|
| `default` | `--mzML --noindex --filter "peakPicking true 2-" --filter "zeroSamples removeExtra"` | Exactly what `mzx <file>` runs today (CLI defaults: `--index` off, `peak_picking=msms`, `remove_zeros` on) |
| `nofilter` | `--mzML --noindex` | Test-only: isolates reader output from the filters |
| `pickall` | `--mzML --noindex --filter "peakPicking true 1-"` | mzx peak picking "all" without remove-zeros (also the `vendorcentroid` probe goldens) |
| `pickall_rz` | `pickall` + `--filter "zeroSamples removeExtra"` | mzx peak picking "all" (default remove-zeros) |

Encoding is msconvert 4.x's default (`MsConvert/src/Config.cs`): binary arrays
64-bit, intensity 32-bit, integer arrays always 64-bit, **zlib**.

## Equality

`normalize(mzx_output) == normalize(pwiz_output)`, compared byte for byte
(`mzx-verify cmp`). `normalize` (`crates/mzx-verify/src/lib.rs`) is applied to
both sides and changes exactly:

1. **Converter software.** Each `<software>` block that identifies the
   converter (pwiz: cvParam `MS:1000615` "ProteoWizard software"; mzx: userParam
   `mzx converter`) collapses to `<software id="@CONVERTERn@" version="@VERSION@"/>`.
   Attribute values exactly equal to that id (`softwareRef`, `ref`) become the
   same token. The number and position of converter blocks must still match.
2. **`sourceFile/@location`.** msconvert runs in Docker under Wine and sees
   `file:///Z:\data`.
3. **cvParam `MS:1001885` "command-line parameters" `value`.** pwiz 4 records
   its own argv (container paths, `-o`, `--outfile`). The cvParam itself must
   still be present in the same place.

Nothing else is normalized. Every other attribute, cvParam value string,
element order, whitespace and base64 payload must match, including the
input-file SHA-1 cvParam.

## Fail closed

The Rust backend returns `Unsupported` for anything outside proven coverage:
an unknown file-format version, an instrument family not in the corpus, an
unproven option set, or profile data whose centroiding isn't proven. mzx then
falls back to Docker. Unproven output is never written.

## Proof levels

| Level | Claim | Evidence |
|---|---|---|
| Writer | `write(read(golden)) == golden` for every golden | `mzx-verify roundtrip` |
| Filters | `filter(read(nofilter golden)) == default golden` | per-filter tests |
| Reader | `mzx-verify convert raw` passes `mzx-verify cmp` against the golden | `support_matrix.toml` rows, `tests/thermo_reader.rs` |

A proof covers the corpus files it names. It does not cover other files.
Coverage grows by adding files that differ along the dimensions recorded in
`corpus/README.md`.

## Status

- **Writer (pwiz 4.0.26299, `fb75aff` confirmed by the manifest):** proven
  byte-identical (2026-11-04) on all 6 goldens: `LR_p_LTQ_HILIC_ecoli`,
  `190218_AR_JH_A_pos`, `190218_AR_JH_Blank2_pos` × `{default,nofilter}`
  (≈1.3 GB; LTQ ion trap, Q Exactive Focus with PDA spectra and an absorption
  chromatogram; zlib via zlib-ng 2.2.5).
- **Writer (pwiz 3 dialect, regression only):** byte-identical on 4 msconvert
  3.0.25182 outputs (Bruker BAF, ≈190 MB) in `mzml_web/tmp`.
- Indexed output isn't implemented yet; it's outside the v1 option sets.
- **Filters (`default` option set, `crates/mzx-filters`):** proven 2026-11-04 on
  all 3 pairs: `write(filters(read(nofilter))) == default` byte for byte
  (`mzx-verify prove-filters`, `tests/filters.rs`, with negative controls for
  each filter that had an effect). LTQ: 1,589 spectra changed by `zeroSamples
  removeExtra`. Q Exactive: all data already centroided with no zero samples, so
  `default` differs from `nofilter` only in provenance and the appended
  peak-picking method. `peakPicking true 2-` is
  ported only for spectra that are already centroided (all LTQ MS2); a profile
  spectrum at MS level ≥ 2 is `Unsupported`.
  **Exception, owned by the reader:** 807 of 2,401 MS2 `selectedIon` "peak
  intensity" values differ between the goldens, because with vendor-preferred
  peak picking the Thermo reader sums the precursor window over the
  *vendor-centroided* MS1 scan (`SpectrumList_Thermo.PopulatePrecursor`,
  `preferCentroid`). The proof masks exactly those lines, checks only `value`
  differs, and reports the count. The Thermo reader (M3) must reproduce them,
  which means Thermo's ion-trap profile centroiding (`Scan.ToCentroid`) for MS1.
- **Thermo reader (`crates/mzx-thermo`):** proven 2026-11-04 on
  `190218_AR_JH_A_pos.raw` and `190218_AR_JH_Blank2_pos.raw` (Q Exactive Focus +
  Acquity PDA, RAW v66) × `{default,nofilter}`: `mzx-verify convert` output is
  identical to the golden under `contract_compare` (all 25,895 / 25,833 spectra
  including 19,201 PDA spectra, the TIC and `PDA 1` chromatograms, the
  instrument configurations and the SHA-1). Blank2 was not used during
  development and is the held-out check. Evidence:
  `crates/mzx-verify/tests/thermo_reader.rs`, driven by `support_matrix.toml`.
  The reader is written from the public RAW layout (unfinnigan) and the
  observed goldens; Thermo's RawFileReader is not used, decompiled or
  inspected (its EULA forbids that). It is fail-closed: an unknown controller
  kind, instrument recipe, scan-event preamble byte, activation, packet type,
  trailer value type, SPS/master-scan trailer or non-monotonic retention time
  returns `mzx_thermo::Unsupported`.
  So does a file whose header checksum doesn't match, which msconvert also
  rejects ("Thermo RAW file reports IsError"). The checksum is the u32 at file
  offset 148: Adler-32 with an initial value of 0 over the first 10 MiB, with
  the field itself read as zero. It was inferred from the corpus (all 7 RAW
  files match) and confirmed by msconvert's response to edited copies.
- **Thermo reader, LTQ XL:** `LR_p_LTQ_HILIC_ecoli.raw` × `nofilter` proven
  2026-11-04 (all 3,997 spectra: ITMS profile MS1, LR centroid CID MS2, sample
  list, TIC). Isolation widths come from the embedded instrument method
  (OLE2 `LTQ/Text` stream, parsed with pwiz's regexes); only scans of the
  first time segment receive them in the golden although every method segment
  lists a width, which fits the SDK numbering scan events across segments.
  Scans for which the plausible numberings would give different widths are
  refused. `default` needs Thermo's ion-trap centroiding of the profile MS1:
  its 807 differing precursor peak intensities are sums over the vendor
  centroids (`Scan.ToCentroid`), reproduced by mzx's ion-trap centroider
  (below). The reader is told about vendor peak picking via
  `mzx_verify::thermo_reader_options` (derived from the filter chain).
- **Thermo reader, LTQ XL with profile MS2:** `LR_p_LTQ_IPRP.raw` and
  `LR_p_LTQ_IPRP_ecoli.raw` × `nofilter` proven 2026-11-05 (MS1 and CID MS2
  both ITMS profile). `default` needs Thermo's ion-trap centroiding of every
  MS2 scan (below).
- **Ion-trap centroiding** (`crates/mzx-thermo/src/centroid.rs`): a model of
  Thermo's closed `Scan.ToCentroid` for ITMS profile scans, inferred only from
  msconvert output for the corpus and for six synthetic probe files
  (profile intensities patched into copies of `LR_p_LTQ_HILIC_ecoli.raw`;
  the probe builders are lab tools, not shipped). RawFileReader was not
  decompiled or inspected.
  Proven 2026-11-06: `LR_p_LTQ_HILIC_ecoli`, `LR_p_LTQ_IPRP`
  and `LR_p_LTQ_IPRP_ecoli` × `{default, pickall, pickall_rz}` are IDENTICAL
  (every centroid of 8,646 profile MS1 and 10,154 profile MS2 scans bit for
  bit, and all 823 precursor intensities). `pickall` of probes 2–6 (about 5.9
  million centroids, including 189,000 placed within 0.003 ULP of a rounding
  tie) is identical as well. When the model finds no point >= 1 the reader
  returns the profile, as pwiz does when `ToCentroid` returns no peaks; a
  profile whose step is not 1/12 m/z is refused, and so is vendor peak
  picking of a profile scan from any analyzer other than the ion trap. Being
  a model of a closed algorithm, it is only as general as this corpus. Until
  2026-11-06 it was opt-in (an "experimental centroiding" setting); with
  every corpus conversion byte-identical it is now always used.
- **Orbitrap Fusion Lumos** (`HR_OH_Orbi_HILIC`, `LR_OH_Orbi_HILIC`, with a
  Vanquish controller of kind 2): goldens exist for both option sets, but there
  is no reader for this family yet, so both files are refused.
- **Shipped pipeline (M4):** `crates/mzx-convert` (`convert_thermo`: read →
  filters → brand → atomic write) is what both the Python extension
  (`crates/mzx-py`, module `mzx_rs`) and the reader proof run, so the proof
  covers the shipped code. Branding replaces pwiz's converter `<software>`
  block with `<software id="mzx">` carrying userParam `mzx converter`
  (normalization 1) and records `mzx-rs <file> --options <set>` in
  MS:1001885 (normalization 3). The proof passed on this branded output
  (2026-11-04), and an end-to-end `mzx --backend rust` conversion of Blank2
  passed `mzx-verify cmp`. mzx (`src/mzx/rust_backend.py`) maps its options to
  `default`, `nofilter`, `pickall` or `pickall_rz` and sends anything else to
  msconvert; `--backend auto`
  also falls back on any Rust refusal or error.
- **Host assumption:** `run/@startTimeStamp` is the RAW creation FILETIME
  shifted by the converting host's UTC offset (`ReaderConfig.FormatStartTimeStamp`).
  The pinned msconvert container runs in UTC, so the Rust reader emits UTC.
