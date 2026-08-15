//! Binary to generate sample PDFs for parity testing.
//!
//! Usage: `cargo run -p easypdf-parity --bin generate_samples`

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let samples_dir = manifest_dir.join("samples");

    println!("Generating sample PDFs to {}...", samples_dir.display());

    easypdf_test::generate_all_samples(&samples_dir).expect("failed to generate sample PDFs");

    println!("Done. Generated:");
    for entry in std::fs::read_dir(&samples_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "pdf") {
            let size = std::fs::metadata(&path).unwrap().len();
            println!(
                "  {} ({} bytes)",
                path.file_name().unwrap().to_string_lossy(),
                size
            );
        }
    }
}
