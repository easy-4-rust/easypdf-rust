# Contributing to easypdf-rust

## Development Setup

```bash
git clone https://github.com/easy-4-rust/easypdf-rust
cd easypdf-rust
cargo build
```

## Quality Gates

Before submitting a PR:

```bash
# Format
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets -- -D warnings

# Build (no default features)
cargo check -p easypdf --no-default-features

# Build (all features)
cargo check -p easypdf --all-features

# Test
cargo test --workspace --quiet

# Docs
cargo doc --workspace --no-deps
```

## Project Structure

```
easypdf-rust/
├── crates/
│   ├── easypdf/           facade — public API, EasyPdf, all Builders
│   ├── easypdf-core/      shared types, enums, traits, errors
│   ├── easypdf-model/     engine-neutral semantic IR
│   ├── easypdf-io/        resource limits, atomic output, input abstraction
│   ├── easypdf-derive/    #[derive(PdfModel)] proc-macro
│   ├── easypdf-layout/    backend-neutral layout (LayoutSink, FlowLayout)
│   ├── easypdf-reader/    PDF parsing and text extraction (lopdf)
│   ├── easypdf-writer/    PDF creation (printpdf)
│   ├── easypdf-manipulate/ merge, split, rotate, reorder (lopdf)
│   ├── easypdf-template/  AcroForm field filling (lopdf)
│   └── easypdf-markdown/  PDF → Markdown conversion
├── docs/
│   ├── easypdf-rust-Architecture.md       architecture (English)
│   ├── easypdf-rust-Architecture.zh_CN.md architecture (中文)
│   ├── roadmap.md          detailed roadmap
│   ├── usage-guide.md      user guide
│   ├── compatibility.md    feature compatibility matrix
│   └── implementation-plan.md
├── benches/                reproducible benchmarks
└── tests/                  workspace-level integration tests
```

## Design Principles

1. **Zero unsafe** — `#![forbid(unsafe_code)]` in every crate
2. **Fluent builders** — `mut self -> Self` with `#[must_use]`
3. **Multi-engine backend** — lopdf for read/manipulate, printpdf for create
4. **Engine-neutral IR** — `easypdf-model` has zero engine dependencies
5. **Trait extensibility** — `PdfModel`, `PdfReadListener`, `PdfWriteHandler`, `PdfConverter`, `LayoutSink`
6. **Single error type** — `PdfError` enum, `type Result<T> = ...`
7. **Atomic output** — temp file + rename for all save operations
8. **Structured warnings** — unimplemented capabilities emit warnings, not fake success

## Adding a New Feature

1. Define core types in `easypdf-core` (if needed)
2. Implement engine logic in the appropriate crate (reader/writer/manipulate/template/markdown)
3. Expose via `easypdf` facade
4. Add tests in the implementing crate's `#[cfg(test)]` module
5. Update documentation: README.md, README.zh-CN.md, docs/usage-guide.md, docs/roadmap.md

## Crate Dependency Rules

- `easypdf-core` has zero engine dependencies
- `easypdf-model` and `easypdf-io` have zero engine dependencies
- `easypdf-layout` does NOT depend on `easypdf-writer`
- Domain crates (reader, writer, manipulate, template, markdown) do NOT depend on each other
- Only the `easypdf` facade depends on all sub-crates

## Commit Convention

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `test:` tests
- `refactor:` code change without feature/fix
- `chore:` build, CI, dependencies
- `perf:` performance improvement

## Release Order

```text
easypdf-core → easypdf-model → easypdf-io → easypdf-derive → easypdf-layout
→ easypdf-reader → easypdf-writer → easypdf-manipulate → easypdf-template
→ easypdf-markdown → easypdf
```
