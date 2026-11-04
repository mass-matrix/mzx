# Contract corpus

Raw files and goldens are too large for git. Keep them in shared storage and
point `MZX_CORPUS` at a local copy laid out as:

```
$MZX_CORPUS/
  raw/<file>.raw                 vendor inputs
  goldens/<stem>.<optset>.mzML   from scripts/make_goldens.sh
  goldens/manifest.jsonl         one row per golden (image digest, sha1/sha256, args)
```

Corpus-backed tests skip with a message when `MZX_CORPUS` is unset. Set
`MZX_RAW` to read the raw files from another directory than `$MZX_CORPUS/raw`.

Run everything: `MZX_CORPUS=... cargo test --release --workspace`. The reader
proof (`tests/thermo_reader.rs`) checks every `support_matrix.toml` row.

## Coverage dimensions (record for every raw file)

| File | Vendor | Format version | Instrument | Polarity | MS levels | Profile/centroid | Activation | Notes |
|---|---|---|---|---|---|---|---|---|
| LR_p_LTQ_HILIC_ecoli.raw | Thermo | Finnigan RAW 66 | LTQ XL (ion trap) | negative | 1 (1,596), 2 (2,401) | MS1 profile, MS2 centroid | CID 35/40 | 112 MB; SHA-1 a8c66ad0…; TIC chromatogram only. 807 MS2 precursor intensities depend on vendor-centroided MS1 under `default` |
| 190218_AR_JH_A_pos.raw | Thermo | Finnigan RAW 66 | Q Exactive Focus (Orbitrap) + Acquity PDA | positive | 1 (1,674, 1,649 with lock mass), 2 (5,020) | all centroid | HCD 30 | SHA-1 a2680427…; 3 controllers; 19,201 PDA spectra + `PDA 1` absorption chromatogram; filters change no data |
| 190218_AR_JH_Blank2_pos.raw | Thermo | Finnigan RAW 66 | Q Exactive Focus (Orbitrap) + Acquity PDA | positive | 1, 2 (4,974) | all centroid | HCD 30 | SHA-1 089b4cec…; blank of the above; 25,833 spectra |
