# Contributing to easypdf-rust

Thank you for your interest in contributing to easypdf-rust! This document
provides guidelines and instructions for contributing.

## Code of Conduct

This project follows a Code of Conduct. By participating, you agree to
uphold a respectful and inclusive environment. See
[CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).

## Getting Started

### Prerequisites

- **Rust toolchain**: MSRV 1.88, Edition 2024
- **Git**: For version control

### Fork and Clone

```bash
# Fork the repository on GitHub, then:
git clone https://github.com/<your-username>/easypdf-rust.git
cd easypdf-rust
git remote add upstream https://github.com/easy-4-rust/easypdf-rust.git
```

### Build and Verify

```bash
# Build the workspace
cargo build

# Run all tests
cargo test --workspace

# Lint (zero warnings required)
cargo clippy --workspace --all-targets -- -D warnings

# Verify documentation builds cleanly
cargo doc --workspace --no-deps

# Run fuzz targets (requires nightly)
cargo +nightly fuzz run pdf_parse
cargo +nightly fuzz run streaming_scan
cargo +nightly fuzz run pdf_encrypt_decrypt
cargo +nightly fuzz run pdf_sign_verify
cargo +nightly fuzz run markdown_convert
cargo +nightly fuzz run ssrf_url
```

All of the above must pass before submitting a pull request.

## Development Workflow

### Branch Strategy

- **`dev`** is the primary development branch. Create feature branches from `dev`.
- **`main`** is the stable release branch. Do not commit directly to `main`.
- Feature branches: `feat/<short-description>`, `fix/<short-description>`,
  `docs/<short-description>`, etc.

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add streaming read strategy for large PDFs
fix: correct UTF-16BE metadata encoding in writer
docs: update architecture diagram for 9-crate layout
test: add fuzz target for PDF signature verification
refactor: consolidate model types into easypdf-core
chore: update lopdf dependency to 0.44.0
```

Prefix with scope when relevant: `feat(reader):`, `fix(writer):`, `docs(ocr):`.

### Code Style

- **Zero unsafe**: `#![deny(unsafe_code)]` is enforced workspace-wide.
- **Clippy pedantic**: `clippy::all = "warn"` and `clippy::pedantic = "warn"`
  are set in the workspace `Cargo.toml`.
- **Formatting**: Run `cargo fmt --all` before committing.
- **File size**: Keep individual files under 800 lines.

### Testing Requirements

The project maintains 1522+ tests with 91.61% code coverage. All changes must:

1. Include tests for new functionality.
2. Not reduce existing coverage.
3. Pass `cargo test --workspace` with zero failures.

## Pull Request Process

1. **Create a feature branch** from `dev`.
2. **Make your changes** following the guidelines above.
3. **Run all quality gates** (see below).
4. **Update CHANGELOG.md** if the change is user-visible.
5. **Submit a pull request** to the `dev` branch.
6. **At least 1 reviewer** must approve before merge.

### Quality Gates (CI must pass)

```bash
# Format check
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Build (no default features)
cargo check -p easypdf --no-default-features

# Build (all features)
cargo check -p easypdf --all-features

# Tests
cargo test --workspace

# Documentation
cargo doc --workspace --no-deps

# Security audit
cargo audit
cargo deny check
```

## Project Structure

The workspace contains 9 crates (consolidated from the original 22):

| Crate | Path | Role |
|-------|------|------|
| `easypdf` | `crates/easypdf/` | Facade crate -- `EasyPdf` entry point and builder API |
| `easypdf-core` | `crates/easypdf-core/` | Core types, traits, errors, crypto, model, IO, layout |
| `easypdf-derive` | `crates/easypdf-derive/` | `#[derive(PdfModel)]` proc-macro |
| `easypdf-reader` | `crates/easypdf-reader/` | PDF reading, text extraction, merge/split/rotate (lopdf backend) |
| `easypdf-writer` | `crates/easypdf-writer/` | PDF creation and writing (printpdf backend) |
| `easypdf-markdown` | `crates/easypdf-markdown/` | PDF-to-Markdown pipeline (table detection, render, OCR) |
| `easypdf-ocr` | `crates/easypdf-ocr/` | Cloud OCR engines (GLM / HunyuanOCR / Baidu) |
| `easypdf-runtime` | `crates/easypdf-runtime/` | Runtime layer: MCP server + resident daemon |
| `easypdf-test` | `easypdf-test/` | Integration tests and golden samples (not published) |

For detailed architecture, see
[docs/easypdf-rust-Architecture.md](docs/easypdf-rust-Architecture.md) and
[docs/PROJECT_FACTS.md](docs/PROJECT_FACTS.md).

## Testing

### Unit Tests

Unit tests live inside each crate in `#[cfg(test)] mod tests` blocks.

```bash
# Run tests for a specific crate
cargo test -p easypdf-core
cargo test -p easypdf-reader
```

### Integration Tests

Cross-crate integration tests are in `easypdf-test/tests/` with golden PDF
samples in `easypdf-test/golden/` and `easypdf-test/samples/`.

### Fuzz Testing

Fuzz targets are in `fuzz/` and require a nightly toolchain:

```bash
cargo +nightly fuzz run <target>
```

Available targets: `pdf_parse`, `streaming_scan`, `pdf_encrypt_decrypt`,
`pdf_sign_verify`, `markdown_convert`, `ssrf_url`.

## Release Process

Releases follow this sequence (see also
[docs/superpowers/version-plan.md](docs/superpowers/version-plan.md)):

1. **Bump version** in root `Cargo.toml` (`workspace.package.version`).
2. **Update CHANGELOG.md** with the new version section.
3. **Dry-run verification**:
   ```bash
   cargo publish -p easypdf-core --dry-run
   cargo publish -p easypdf-derive --dry-run
   ```
4. **Publish in dependency order** (each step waits ~45s for crates.io propagation):
   ```bash
   # Layer 1: no internal deps
   cargo publish -p easypdf-core && sleep 45

   # Layer 2: depends on core
   cargo publish -p easypdf-derive && sleep 45
   cargo publish -p easypdf-reader && sleep 45
   cargo publish -p easypdf-writer && sleep 45

   # Layer 3: depends on core + reader
   cargo publish -p easypdf-markdown && sleep 45

   # Layer 4: depends on markdown
   cargo publish -p easypdf-ocr && sleep 45

   # Layer 5: depends on reader + writer + markdown
   cargo publish -p easypdf-runtime && sleep 45

   # Layer 6: facade (depends on everything)
   cargo publish -p easypdf
   ```
5. **Tag and release**:
   ```bash
   git tag v<version>
   git push origin v<version>
   ```
6. **Create GitHub Release** with changelog content.
7. **Verify** crates.io listing and docs.rs build.

## Questions?

- Open a [GitHub Issue](https://github.com/easy-4-rust/easypdf-rust/issues)
  for bugs, feature requests, or questions.
- Check existing [documentation](docs/) for architecture and usage guides.
