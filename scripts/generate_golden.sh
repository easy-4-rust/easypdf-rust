#!/usr/bin/env bash
# Generate sample PDFs and golden files for parity testing.
#
# Usage: ./scripts/generate_golden.sh
#
# This script:
# 1. Builds easypdf-parity in release mode.
# 2. Generates sample PDFs to parity/samples/.
# 3. Generates golden files from those samples to parity/golden/.
#
# After running, review the diff and commit the changes.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

echo "=== Building easypdf-parity ==="
cargo build -p easypdf-parity --release

echo ""
echo "=== Generating sample PDFs ==="
cargo run -p easypdf-parity --release --bin generate_samples

echo ""
echo "=== Generating golden files ==="
cargo run -p easypdf-parity --release --bin generate_golden

echo ""
echo "=== Summary ==="
echo "Sample PDFs:"
ls -la parity/samples/*.pdf 2>/dev/null || echo "  (none found)"
echo ""
echo "Golden files:"
ls -la parity/golden/text-extraction/*.txt 2>/dev/null || echo "  (none found)"
ls -la parity/golden/metadata/*.json 2>/dev/null || echo "  (none found)"
ls -la parity/golden/structure/*.json 2>/dev/null || echo "  (none found)"
echo ""
echo "Review the diff, then commit if correct."
