#!/usr/bin/env bash
# Generate ProteoWizard golden outputs for the mzx byte contract (see rust/CONTRACT.md).
#
# usage: rust/scripts/make_goldens.sh <raw-file-or-dir> <option-set> [out-dir]
#        rust/scripts/make_goldens.sh --resolve        # print the digest of the local image
#
# MZX_PWIZ_IMAGE must be pinned by digest (repo@sha256:...). Goldens made with a
# different image are a different contract.
#
# Option sets:
#   default   what `mzx <file>` runs today
#   nofilter  no filters (used to prove the filter ports in isolation)
#   pickall   peak picking on every MS level (mzx GUI/CLI peak picking "all")
#   pickall_rz  pickall plus zeroSamples removeExtra
#   vendorcentroid  research name for the probe files; same arguments as pickall
set -euo pipefail

REPO="chambm/pwiz-skyline-i-agree-to-the-vendor-licenses"

if [[ "${1:-}" == "--resolve" ]]; then
    docker image inspect --format '{{index .RepoDigests 0}}' "$REPO"
    exit 0
fi

if [[ $# -lt 2 ]]; then
    sed -n '2,17p' "$0" >&2
    exit 2
fi

IMAGE="${MZX_PWIZ_IMAGE:-}"
if [[ "$IMAGE" != *@sha256:* ]]; then
    echo "MZX_PWIZ_IMAGE must be pinned by digest, e.g. $REPO@sha256:<digest>" >&2
    echo "Run '$0 --resolve' to print the digest of the image you have locally." >&2
    exit 2
fi

RAW="$(cd "$(dirname "$1")" && pwd)/$(basename "$1")"
OPTSET="$2"
OUT="$(mkdir -p "${3:-goldens}" && cd "${3:-goldens}" && pwd)"
NAME="$(basename "$RAW")"
STEM="${NAME%.*}"

case "$OPTSET" in
    default)  ARGS=(--mzML --noindex --filter "peakPicking true 2-" --filter "zeroSamples removeExtra") ;;
    nofilter) ARGS=(--mzML --noindex) ;;
    pickall|vendorcentroid) ARGS=(--mzML --noindex --filter "peakPicking true 1-") ;;
    pickall_rz) ARGS=(--mzML --noindex --filter "peakPicking true 1-" --filter "zeroSamples removeExtra") ;;
    *) echo "unknown option set: $OPTSET" >&2; exit 2 ;;
esac

OUTNAME="$STEM.$OPTSET.mzML"
# Same mount layout as mzx: the input directory is /data (it appears in sourceFile/@location).
docker run --rm \
    -v "$(dirname "$RAW")":/data:ro \
    -v "$OUT":/out \
    "$IMAGE" wine msconvert "/data/$NAME" "${ARGS[@]}" -o /out --outfile "$OUTNAME"
# NB: --outfile only names the file; the directory comes from -o (default: cwd = /data).

[[ -s "$OUT/$OUTNAME" ]] || { echo "msconvert did not produce $OUT/$OUTNAME" >&2; exit 1; }

# Whole line, e.g. "ProteoWizard release: 4.0.26299 (abc1234)": the hash pins the pwiz source.
PWIZ_VERSION="$(docker run --rm "$IMAGE" wine msconvert --help 2>&1 | tr -d '\r' | grep -m1 -o 'ProteoWizard release: .*' || true)"

if [[ -d "$RAW" ]]; then
    RAW_SHA1="$(cd "$RAW" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha1sum | sha1sum | cut -d' ' -f1)"
else
    RAW_SHA1="$(sha1sum "$RAW" | cut -d' ' -f1)"
fi
OUT_SHA256="$(sha256sum "$OUT/$OUTNAME" | cut -d' ' -f1)"

printf '{"raw":"%s","raw_sha1":"%s","option_set":"%s","args":"%s","image":"%s","pwiz":"%s","output":"%s","output_sha256":"%s"}\n' \
    "$NAME" "$RAW_SHA1" "$OPTSET" "${ARGS[*]}" "$IMAGE" "$PWIZ_VERSION" "$OUTNAME" "$OUT_SHA256" \
    >> "$OUT/manifest.jsonl"

echo "wrote $OUT/$OUTNAME"
