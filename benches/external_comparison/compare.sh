#!/usr/bin/env bash
# compare.sh — Run easypdf vs pdftotext text extraction and collect timing + accuracy data.
#
# Usage: ./compare.sh [corpus_dir]
#   corpus_dir: directory containing .pdf files (default: ../../easypdf-test/samples/benchmark_corpus)
#
# Output: CSV to stdout with columns:
#   pdf, size_bytes, easypdf_ms, easypdf_chars, pdftotext_ms, pdftotext_chars, char_ratio

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

# Check pdftotext
if ! command -v pdftotext &>/dev/null; then
    echo "ERROR: pdftotext not found" >&2
    exit 1
fi

echo "pdf,size_bytes,easypdf_ms,easypdf_chars,pdftotext_ms,pdftotext_chars,char_ratio"

for pdf in "$CORPUS"/*.pdf; do
    [ -f "$pdf" ] || continue
    name="$(basename "$pdf")"
    size="$(wc -c < "$pdf" | tr -d ' ')"

    # easypdf extraction: run 3 times, take median
    ep_times=()
    ep_chars_val=0
    for run in 1 2 3; do
        easypdf_out="$("$BINARY" "$pdf" 2>/dev/null)" || easypdf_out="0 0 0"
        ep_times+=("$(echo "$easypdf_out" | awk '{print $1}')")
        ep_chars_val="$(echo "$easypdf_out" | awk '{print $2}')"
    done
    # Sort to find median
    easypdf_ms="$(printf '%s\n' "${ep_times[@]}" | sort -n | sed -n '2p')"
    easypdf_chars="$ep_chars_val"

    # pdftotext extraction with timing (3 runs, take median)
    pt_times=()
    pt_chars_val=0
    for run in 1 2 3; do
        pt_tmp="$(mktemp)"
        pt_start="$(date +%s%N)"
        pdftotext "$pdf" - 2>/dev/null > "$pt_tmp" || true
        pt_end="$(date +%s%N)"
        pt_times+=("$(( (pt_end - pt_start) / 1000000 ))")
        pt_chars_val="$(wc -m < "$pt_tmp" | tr -d ' ')"
        rm -f "$pt_tmp"
    done
    pdftotext_ms="$(printf '%s\n' "${pt_times[@]}" | sort -n | sed -n '2p')"
    pdftotext_chars="$pt_chars_val"

    # Char ratio: min/max
    if [ "$pdftotext_chars" -gt 0 ] && [ "$easypdf_chars" -gt 0 ]; then
        char_ratio="$(python3 -c "
a, b = $easypdf_chars, $pdftotext_chars
print(f'{min(a,b)/max(a,b):.4f}')
")"
    else
        char_ratio="0.0000"
    fi

    echo "$name,$size,$easypdf_ms,$easypdf_chars,$pdftotext_ms,$pdftotext_chars,$char_ratio"
done
