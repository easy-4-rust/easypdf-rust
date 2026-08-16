//! Engine comparison benchmark.
//!
//! Compares `printpdf` and `krilla` engines on 3-page document output
//! size and generation time.
//!
//! Run: `cargo run --release -p easypdf-test --all-features --bin engine_bench`

#![allow(clippy::cast_precision_loss)]

use easypdf::prelude::*;
use std::time::{Duration, Instant};

/// Number of iterations per benchmark.
const ITERATIONS: u32 = 10;

/// Number of pages per document.
const PAGES: usize = 3;

/// Benchmark `printpdf` engine, returning `(avg_duration, last_output_size)`.
fn bench_printpdf(iterations: u32) -> (Duration, u64) {
    let start = Instant::now();
    let mut size = 0u64;
    for _ in 0..iterations {
        let path = std::env::temp_dir().join("bench_pp.pdf");
        let mut w = PdfWriter::new("Bench printpdf");
        for i in 0..PAGES {
            w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            w.write_text(
                &PdfText::new(format!(
                    "Page {} - Benchmark text content for engine comparison.",
                    i + 1
                ))
                .font(PdfFont::helvetica(14.0)),
                72.0,
                700.0,
            )
            .unwrap();
            w.write_text(
                &PdfText::new("Second line of benchmark text for volume comparison.")
                    .font(PdfFont::times_roman(12.0)),
                72.0,
                670.0,
            )
            .unwrap();
            w.draw_line(72.0, 650.0, 523.0, 650.0, 1.0);
            w.draw_rect_stroke(72.0, 600.0, 200.0, 40.0, 1.0);
        }
        w.finish(&path).unwrap();
        size = std::fs::metadata(&path).unwrap().len();
        let _ = std::fs::remove_file(&path);
    }
    (start.elapsed() / iterations, size)
}

/// Benchmark `krilla` engine, returning `(avg_duration, last_output_size)`.
fn bench_krilla(iterations: u32, font_data: Option<&[u8]>) -> (Duration, u64) {
    let has_font = font_data.is_some();
    let start = Instant::now();
    let mut size = 0u64;
    for _ in 0..iterations {
        let path = std::env::temp_dir().join("bench_kr.pdf");
        let mut w = PdfWriterBuilder::new("Bench krilla")
            .engine(WriteEngineKind::Krilla)
            .build()
            .unwrap();
        if let Some(d) = font_data {
            w.register_font_from_bytes("hf", d).unwrap();
        }
        for i in 0..PAGES {
            w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
            if has_font {
                let _ = w.write_text_with_custom_font(
                    &format!(
                        "Page {} - Benchmark text content for engine comparison.",
                        i + 1
                    ),
                    "hf",
                    14.0,
                    72.0,
                    700.0,
                );
                let _ = w.write_text_with_custom_font(
                    "Second line of benchmark text for volume comparison.",
                    "hf",
                    12.0,
                    72.0,
                    670.0,
                );
            }
            w.draw_line(72.0, 650.0, 523.0, 650.0, 1.0);
            w.draw_rect_stroke(72.0, 600.0, 200.0, 40.0, 1.0);
        }
        w.finish(&path).unwrap();
        size = std::fs::metadata(&path).unwrap().len();
        let _ = std::fs::remove_file(&path);
    }
    (start.elapsed() / iterations, size)
}

fn main() {
    let font_data = std::fs::read("/System/Library/Fonts/Helvetica.ttc").ok();
    let has_font = font_data.is_some();

    // Warm up.
    for _ in 0..3 {
        let path = std::env::temp_dir().join("warmup.pdf");
        let mut w = PdfWriter::new("warmup");
        w.add_page(PageSize::A4, Orientation::Portrait).unwrap();
        w.finish(&path).unwrap();
        let _ = std::fs::remove_file(&path);
    }

    let (pp_avg, pp_size) = bench_printpdf(ITERATIONS);
    let (kr_avg, kr_size) = bench_krilla(ITERATIONS, font_data.as_deref());

    println!("# Engine Comparison Benchmark Results");
    println!();
    println!(
        "Platform: {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH
    );
    println!("Iterations: {ITERATIONS}");
    println!("Font available: {has_font}");
    println!();
    println!("| Metric | printpdf | krilla |");
    println!("|---|---|---|");
    println!("| Avg generation time ({PAGES} pages) | {pp_avg:?} | {kr_avg:?} |");
    println!("| Output size (bytes) | {pp_size} | {kr_size} |");
    println!(
        "| Output size (KB) | {:.1} | {:.1} |",
        pp_size as f64 / 1024.0,
        kr_size as f64 / 1024.0
    );
    println!("| Pages | {PAGES} | {PAGES} |");
    println!("| Base14 built-in fonts | Supported | Not supported |");
    println!("| SVG | Supported | Not supported |");
    println!("| Font subsetting | No | Yes |");
    println!("| CJK optimization | No | Yes |");

    if pp_size > 0 && kr_size > 0 {
        let ratio = if pp_size > kr_size {
            format!("{:.1}x smaller", pp_size as f64 / kr_size as f64)
        } else {
            format!("{:.1}x larger", kr_size as f64 / pp_size as f64)
        };
        println!("| Size ratio (krilla vs printpdf) | baseline | {ratio} |");
    }
}
