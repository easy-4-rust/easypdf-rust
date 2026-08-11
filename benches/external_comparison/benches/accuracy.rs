//! Criterion benchmark: measure text extraction accuracy by comparing
//! easypdf-reader output against pdftotext (Poppler) as ground truth.
//!
//! Metrics:
//! - Character count ratio: min(easypdf, pdftotext) / max(easypdf, pdftotext)
//! - Jaccard character similarity: |A intersect B| / |A union B|

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use easypdf_reader::PdfReader;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

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

fn pdftotext_extract(pdf_path: &Path) -> String {
    let output = std::process::Command::new("pdftotext")
        .arg(pdf_path)
        .arg("-")
        .output()
        .expect("failed to run pdftotext");
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn jaccard_chars(a: &str, b: &str) -> f64 {
    let a_set: HashSet<char> = a.chars().collect();
    let b_set: HashSet<char> = b.chars().collect();
    if a_set.is_empty() && b_set.is_empty() {
        return 1.0;
    }
    let intersection = a_set.intersection(&b_set).count();
    let union = a_set.union(&b_set).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

fn char_count_ratio(a: &str, b: &str) -> f64 {
    let a_len = a.chars().count();
    let b_len = b.chars().count();
    if a_len == 0 && b_len == 0 {
        return 1.0;
    }
    let max = a_len.max(b_len);
    let min = a_len.min(b_len);
    min as f64 / max as f64
}

fn bench_accuracy(c: &mut Criterion) {
    let pdfs = corpus_pdfs();
    if pdfs.is_empty() {
        eprintln!("WARNING: no PDFs found in benchmark_corpus; skipping accuracy benchmarks");
        return;
    }

    let check = std::process::Command::new("pdftotext").arg("-v").output();
    if check.is_err() {
        eprintln!("WARNING: pdftotext not found; skipping accuracy benchmarks");
        return;
    }

    let mut group = c.benchmark_group("accuracy_vs_pdftotext");
    group.sample_size(10);

    for pdf_path in &pdfs {
        let name = pdf_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown");

        group.bench_with_input(BenchmarkId::new("compare", name), pdf_path, |b, path| {
            b.iter(|| {
                let easypdf_text = PdfReader::open(path)
                    .and_then(|r| r.extract_text())
                    .unwrap_or_default();
                let pt_text = pdftotext_extract(path);

                let jaccard = jaccard_chars(&easypdf_text, &pt_text);
                let ratio = char_count_ratio(&easypdf_text, &pt_text);

                criterion::black_box((jaccard, ratio));
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_accuracy);
criterion_main!(benches);
