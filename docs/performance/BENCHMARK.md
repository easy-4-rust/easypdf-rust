# Performance Baseline Report: easypdf vs pdftotext

**Date**: 2026-08-11
**Status**: Baseline established

## Test Environment

| Item | Value |
|------|-------|
| OS | macOS Darwin 25.5.0 arm64 |
| CPU | Apple M4 Pro |
| Rust | 1.97.1 (stable, 2026-07-14) |
| Rust edition | 2024 (main crate) / 2021 (bench crate) |
| Profile | release (optimized) |
| pdftotext | 26.02.0 (Poppler) |
| qpdf | NOT INSTALLED (TODO: install for memory comparison refinement) |

## Test Corpus

8 PDFs from `easypdf-test/samples/benchmark_corpus/`, symlinked from the main samples directory.

| PDF | Size (bytes) | Description |
|-----|-------------|-------------|
| minimal.pdf | 1,295 | Single page, minimal content |
| with_acroform.pdf | 860 | PDF with form fields |
| with-metadata.pdf | 1,378 | PDF with metadata fields |
| with_table_text.pdf | 1,852 | Table layout with text |
| multi_column_heuristic.pdf | 2,077 | Multi-column layout |
| multipage.pdf | 2,147 | Multiple pages |
| nested_objects.pdf | 3,797 | Deeply nested PDF objects |
| large_100page.pdf | 72,118 | 100-page stress test |

**Note**: `corrupted_xref.pdf`, `encrypted_dummy.pdf`, and `image_only.pdf` were excluded (no extractable text or invalid for comparison).

---

## Benchmark 1: Text Extraction Speed (Criterion)

Measured with Criterion 0.5 (10 samples per benchmark, release mode).

### Wall time (absolute)

| PDF | Size | easypdf median | 95% CI |
|-----|------|---------------|--------|
| with_acroform.pdf | 860 B | 96.6 us | [93.6, 101.3] |
| minimal.pdf | 1.3 KB | 94.6 us | [89.7, 99.3] |
| with-metadata.pdf | 1.4 KB | 102.6 us | [101.4, 104.5] |
| with_table_text.pdf | 1.8 KB | 120.8 us | [119.7, 122.2] |
| multi_column_heuristic.pdf | 2.0 KB | 121.8 us | [119.5, 123.8] |
| multipage.pdf | 2.1 KB | 128.9 us | [122.1, 134.8] |
| nested_objects.pdf | 3.7 KB | 146.7 us | [138.7, 153.2] |
| large_100page.pdf | 70.4 KB | 2,439 us | [2,360, 2,515] |

### Throughput (large_100page.pdf)

| Metric | Value |
|--------|-------|
| Throughput | 28.7 MiB/s |
| 95% CI | [28.2, 29.1] MiB/s |

---

## Benchmark 2: Speed Comparison (compare.sh)

Median of 3 runs per PDF. Both tools in cold-start mode.

| PDF | Size | easypdf (ms) | pdftotext (ms) | Speedup |
|-----|------|-------------|----------------|---------|
| minimal.pdf | 1,295 B | 0 | 12 | pdftotext slower |
| with_acroform.pdf | 860 B | 0 | 12 | pdftotext slower |
| with-metadata.pdf | 1,378 B | 0 | 12 | pdftotext slower |
| with_table_text.pdf | 1,852 B | 0 | 13 | pdftotext slower |
| multi_column_heuristic.pdf | 2,077 B | 0 | 12 | pdftotext slower |
| multipage.pdf | 2,147 B | 0 | 12 | pdftotext slower |
| nested_objects.pdf | 3,797 B | 0 | 12 | pdftotext slower |
| large_100page.pdf | 72,118 B | 3 | 17 | ~5.7x faster |

**Note**: Sub-millisecond easypdf times round to 0 ms in the shell timer (millisecond resolution). Criterion provides precise microsecond measurements above. The shell comparison primarily shows that easypdf's process startup + extraction is dominated by pdftotext's process startup overhead (~12 ms minimum for pdftotext). For the 100-page PDF, easypdf is approximately 5.7x faster.

---

## Benchmark 3: Text Extraction Accuracy

Comparison of extracted text: easypdf vs pdftotext (ground truth).

| PDF | easypdf chars | pdftotext chars | Char Ratio |
|-----|--------------|-----------------|------------|
| with_table_text.pdf | 186 | 189 | 0.9841 |
| large_100page.pdf | 20,183 | 18,992 | 0.9410 |
| with-metadata.pdf | 23 | 25 | 0.9200 |
| nested_objects.pdf | 20 | 22 | 0.9091 |
| with_acroform.pdf | 29 | 31 | 0.9355 |
| multipage.pdf | 23 | 27 | 0.8519 |
| minimal.pdf | 12 | 14 | 0.8571 |
| multi_column_heuristic.pdf | 496 | 363 | 0.7319 |

**Summary**:
- **Average char ratio**: 0.89 (89%)
- **Best**: with_table_text.pdf at 98.4%
- **Worst**: multi_column_heuristic.pdf at 73.2%
- **Large file (100 pages)**: 94.1% -- easypdf extracts more characters than pdftotext

**Note**: Char ratio = min(easypdf, pdftotext) / max(easypdf, pdftotext). A ratio below 1.0 does NOT necessarily mean incorrect extraction -- different tools may include different whitespace, page headers/footers, or column ordering. The `multi_column_heuristic.pdf` case shows easypdf extracting significantly more text (496 vs 363 chars), suggesting it may capture content that pdftotext's column heuristic misses or vice versa.

---

## Benchmark 4: Peak Memory (RSS)

Measured with `/usr/bin/time -l` on macOS (maximum resident set size).

| PDF | Size | easypdf RSS (KB) | pdftotext RSS (KB) | Ratio |
|-----|------|-----------------|--------------------|----|
| with_acroform.pdf | 860 B | 6,992 | 9,872 | 0.71 |
| minimal.pdf | 1,295 B | 7,040 | 9,760 | 0.72 |
| with-metadata.pdf | 1,378 B | 7,024 | 9,808 | 0.72 |
| with_table_text.pdf | 1,852 B | 7,104 | 10,000 | 0.71 |
| multi_column_heuristic.pdf | 2,077 B | 7,040 | 9,952 | 0.71 |
| multipage.pdf | 2,147 B | 7,088 | 9,840 | 0.72 |
| nested_objects.pdf | 3,797 B | 7,152 | 9,808 | 0.73 |
| large_100page.pdf | 72,118 B | 8,720 | 10,464 | 0.83 |

**Summary**:
- easypdf uses approximately **70-73% of pdftotext's memory** for small files
- For the 100-page file, the gap narrows to **83%**
- easypdf baseline RSS (~7 MB) reflects Rust runtime + tokio/allocator overhead
- pdftotext baseline RSS (~10 MB) reflects Poppler/C++ runtime overhead

---

## Key Findings

1. **Speed**: easypdf is fast. For the 100-page stress test, Criterion measures 2.4 ms wall time at 28.7 MiB/s throughput. pdftotext takes ~17 ms for the same file (7x slower, including process startup).

2. **Memory**: easypdf consistently uses less peak memory than pdftotext (29% less for small files, 17% less for the 100-page file).

3. **Accuracy**: Average char ratio is 89%. The main outlier is `multi_column_heuristic.pdf` where easypdf extracts 37% more text than pdftotext. For well-structured PDFs (tables, metadata, forms), accuracy is 92-98%.

4. **Criterion statistical notes**: Some benchmarks show high variance (10-59% change between runs) on small files where measurement noise dominates. The large_100page.pdf benchmark is the most statistically stable.

## Known Limitations

- **qpdf not installed**: Memory comparison only covers easypdf vs pdftotext. TODO: install qpdf via `brew install qpdf` for full 3-tool comparison.
- **Small corpus**: 8 PDFs total. The corpus should be expanded with real-world documents (scanned PDFs, large text PDFs, complex layouts) for production-grade baselines.
- **Accuracy metric**: Char-level ratio is a coarse metric. A proper diff/Levenshtein analysis would give more insight into where extraction diverges.
- **Single machine**: Results are specific to Apple M4 Pro. CI benchmarks should run on Linux x86_64 for cross-platform comparison.
- **easypdf-core::crypto compilation**: The `crypto` module in `easypdf-core` has pre-existing API incompatibilities with its `rsa`/`pkcs8` dependencies when compiled outside the main workspace's lockfile. This does not affect the reader functionality but prevents the bench crate from being a workspace member.

## Reproducing

```bash
# Build and run comparison
cd easypdf-rust/benches/external_comparison
./compare.sh ../../easypdf-test/samples/benchmark_corpus

# Build and run memory comparison
./compare_memory.sh ../../easypdf-test/samples/benchmark_corpus

# Run Criterion benchmarks
cargo bench --bench text_extraction
cargo bench --bench accuracy
```
