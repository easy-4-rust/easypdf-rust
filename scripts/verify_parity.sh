#!/usr/bin/env bash
# Parity verification gate for CI.
#
# Usage: ./scripts/verify_parity.sh
#
# This script runs all parity tests and verifies that golden files
# have not been modified without running the tests. Any failure blocks
# the merge.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Parity Verification Gate ==="
echo ""

# Step 1: Verify sample PDFs exist
echo "[1/3] Checking sample PDFs..."
MISSING_SAMPLES=0
for sample in parity/samples/minimal.pdf parity/samples/multipage.pdf parity/samples/with-metadata.pdf; do
    if [[ ! -f "$sample" ]]; then
        echo "  MISSING: $sample" >&2
        MISSING_SAMPLES=1
    fi
done
if [[ "$MISSING_SAMPLES" -eq 1 ]]; then
    echo "ERROR: Missing sample PDFs. Run: cargo run -p easypdf-parity --bin generate_samples" >&2
    exit 1
fi
echo "  All sample PDFs present."

# Step 2: Verify golden files exist
echo "[2/3] Checking golden files..."
MISSING_GOLDEN=0
for golden in \
    parity/golden/text-extraction/minimal.pdf.txt \
    parity/golden/text-extraction/multipage.pdf.txt \
    parity/golden/text-extraction/with-metadata.pdf.txt \
    parity/golden/metadata/minimal.pdf.json \
    parity/golden/metadata/multipage.pdf.json \
    parity/golden/metadata/with-metadata.pdf.json \
    parity/golden/structure/minimal.pdf.json \
    parity/golden/structure/multipage.pdf.json \
    parity/golden/structure/with-metadata.pdf.json \
; do
    if [[ ! -f "$golden" ]]; then
        echo "  MISSING: $golden" >&2
        MISSING_GOLDEN=1
    fi
done
if [[ "$MISSING_GOLDEN" -eq 1 ]]; then
    echo "ERROR: Missing golden files. Run: ./scripts/generate_golden.sh" >&2
    exit 1
fi
echo "  All golden files present."

# Step 3: Run parity tests
echo "[3/3] Running parity tests..."
cargo test -p easypdf-parity --release

echo ""
echo "=== Parity gate PASSED ==="
