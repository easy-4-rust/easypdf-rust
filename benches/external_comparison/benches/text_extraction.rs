//! Criterion benchmark: measure easypdf-reader text extraction speed across PDF corpus.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use easypdf_reader::PdfReader;
use std::path::PathBuf;

/// Return all .pdf files in the benchmark corpus directory.
fn corpus_pdfs() -> Vec<PathBuf> {
    let corpus_dir = PathBuf::from("../../easypdf-test/samples/benchmark_corpus");
    if !corpus_dir.exists() {
        return Vec::new();
    }
    let mut pdfs: Vec<PathBuf> = std::fs::read_dir(corpus_dir)
        .expect("read benchmark_corpus directory")
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            if path.extension().and_then(|s| s.to_str()) == Some("pdf") {
                Some(path)
            } else {
                None
            }
        })
        .collect();
    pdfs.sort();
    pdfs
}

fn bench_text_extraction(c: &mut Criterion) {
    let pdfs = corpus_pdfs();
    if pdfs.is_empty() {
        eprintln!("WARNING: no PDFs found in benchmark_corpus; skipping benchmarks");
        return;
    }

    let mut group = c.benchmark_group("easypdf_text_extraction");
    group.sample_size(10);

    for pdf_path in &pdfs {
        let name = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        group.bench_with_input(BenchmarkId::new("extract_text", name), pdf_path, |b, path| {
            b.iter(|| {
                let text = PdfReader::open(path)
                    .and_then(|r| r.extract_text())
                    .unwrap_or_default();
                criterion::black_box(&text);
            });
        });
    }

    group.finish();
}

fn bench_text_extraction_with_size(c: &mut Criterion) {
    let pdfs = corpus_pdfs();
    if pdfs.is_empty() {
        return;
    }

    let mut group = c.benchmark_group("easypdf_text_by_size");
    group.sample_size(10);

    for pdf_path in &pdfs {
        let name = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");
        let size_bytes = std::fs::metadata(pdf_path)
            .map(|m| m.len())
            .unwrap_or(0);

        group.throughput(criterion::Throughput::Bytes(size_bytes));
        group.bench_with_input(
            BenchmarkId::new("extract_text", name),
            pdf_path,
            |b, path| {
                b.iter(|| {
                    let text = PdfReader::open(path)
                        .and_then(|r| r.extract_text())
                        .unwrap_or_default();
                    criterion::black_box(&text);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_text_extraction, bench_text_extraction_with_size);
criterion_main!(benches);
