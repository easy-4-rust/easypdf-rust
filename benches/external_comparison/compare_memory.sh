#!/usr/bin/env bash
# compare_memory.sh — Measure peak RSS (memory) for easypdf vs pdftotext.
#
# Usage: ./compare_memory.sh [corpus_dir]
#
# macOS: uses /usr/bin/time -l to capture max RSS (bytes)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CORPUS="${1:-$SCRIPT_DIR/../../easypdf-test/samples/benchmark_corpus}"

if [ ! -d "$CORPUS" ]; then
    echo "ERROR: corpus directory not found: $CORPUS" >&2
    exit 1
fi

# Build the text_extract binary (standalone workspace)
echo "Building text_extract binary..." >&2
cargo build --release --bin text_extract --manifest-path "$SCRIPT_DIR/Cargo.toml" 2>&1 >&2
BINARY="$SCRIPT_DIR/target/release/text_extract"

if [ ! -f "$BINARY" ]; then
    echo "ERROR: text_extract binary not found at $BINARY" >&2
    exit 1
fi

echo "pdf,size_bytes,easypdf_peak_rss_kb,pdftotext_peak_rss_kb"

for pdf in "$CORPUS"/*.pdf; do
    [ -f "$pdf" ] || continue
    name="$(basename "$pdf")"
    size="$(wc -c < "$pdf" | tr -d ' ')"

    # easypdf memory (macOS /usr/bin/time -l)
    ep_out=$(/usr/bin/time -l "$BINARY" "$pdf" 2>&1) || true
    ep_rss="$(echo "$ep_out" | grep "maximum resident set size" | awk '{print $1}')"

    # pdftotext memory
    pt_out=$(/usr/bin/time -l pdftotext "$pdf" - 2>&1) || true
    pt_rss="$(echo "$pt_out" | grep "maximum resident set size" | awk '{print $1}')"

    # Convert bytes to KB
    ep_rss_kb="$(( ${ep_rss:-0} / 1024 ))"
    pt_rss_kb="$(( ${pt_rss:-0} / 1024 ))"

    echo "$name,$size,$ep_rss_kb,$pt_rss_kb"
done
