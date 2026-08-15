//! Binary benchmark: extract text from a PDF and print timing + character count.
//!
//! Usage: text_extract <pdf-path>
//! Output: <elapsed_ms> <char_count> <byte_len>

use std::env;
use std::time::Instant;

use easypdf_reader::PdfReader;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: text_extract <pdf>");
        std::process::exit(1);
    }
    let path = &args[1];
    let start = Instant::now();
    let text = PdfReader::open(path)
        .and_then(|r| r.extract_text())
        .unwrap_or_default();
    let elapsed = start.elapsed();
    println!(
        "{} {} {}",
        elapsed.as_millis(),
        text.chars().count(),
        text.len()
    );
}
