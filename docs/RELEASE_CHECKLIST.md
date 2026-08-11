# 0.1.0 Release Checklist (Dry-Run Report)

Generated: 2026-08-12 (updated after metadata fixes)

## Summary

| Item | Status |
|------|--------|
| Version | `0.1.0` (workspace-consistent) |
| Metadata complete | **YES** -- all blocking issues fixed |
| Dry-run leaf crates | **PASS** -- easypdf-core, easypdf-derive |
| Dry-run dependent crates | Expected pre-publish failure (see note below) |
| Publishable | **YES** -- publish in dependency order (Section 4) |

**Note on dependent crate dry-run**: Crates that depend on `easypdf-core` fail
dry-run with "no matching package named `easypdf-core` found" because it is not
yet published on crates.io. This is the expected pre-publish state. The original
blocking error ("all dependencies must have a version requirement specified") is
**resolved**. After publishing `easypdf-core` first and waiting for crates.io
propagation (~45s), all dependent crates will pass dry-run.

---

## 1. Metadata Check (per crate)

### Common workspace-inherited fields

All 8 crates inherit from `[workspace.package]`:

- `version` = `0.1.0`
- `edition` = `2024`
- `rust-version` = `1.88`
- `license` = `Apache-2.0`
- `repository` = `https://github.com/easy-4-rust/easypdf-rust`
- `keywords` = `["pdf", "ocr", "markdown", "parser", "writer"]`
- `categories` = `["parser-implementations", "encoding"]`

### easypdf-core

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-core` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Core types, traits, enums, converters, and errors for easypdf-rust" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |

### easypdf-derive

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-derive` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Proc-macro derive for PdfModel trait in easypdf-rust" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |

### easypdf-reader

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-reader` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "PDF reading and text extraction for easypdf-rust (lopdf backend)" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core` via workspace (path + version) | OK |

### easypdf-writer

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-writer` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "PDF creation and writing for easypdf-rust (printpdf backend)" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core` via workspace (path + version) | OK |

### easypdf-markdown

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-markdown` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Deterministic PDF to Markdown conversion for easypdf-rust (includes render, table detection, and OCR)" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core`, `easypdf-reader` via workspace (path + version) | OK |

### easypdf-ocr

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-ocr` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Cloud OCR engine collection (GLM / HunyuanOCR / Baidu) for easypdf" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) -- **crate-level README created** | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core`, `easypdf-markdown` via workspace (path + version) | OK |

### easypdf-runtime

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf-runtime` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Runtime layer for easypdf: MCP server + resident daemon" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) -- **crate-level README created** | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core`, `easypdf-reader`, `easypdf-writer`, `easypdf-markdown` via workspace (path + version) | OK |

### easypdf (facade)

| Field | Value | Status |
|-------|-------|--------|
| name | `easypdf` | OK |
| version | `0.1.0` (workspace) | OK |
| edition | `2024` (workspace) | OK |
| rust-version | `1.88` (workspace) | OK |
| description | "Easy PDF manipulation library for Rust -- builder API for creating, reading, manipulating, and filling PDFs" | OK |
| license | `Apache-2.0` (workspace) | OK |
| repository | inherited from workspace | OK |
| readme | `README.md` (workspace) | OK |
| keywords | `["pdf", "ocr", "markdown", "parser", "writer"]` (workspace) | OK |
| categories | `["parser-implementations", "encoding"]` (workspace) | OK |
| publish | not set (defaults to true) | OK |
| internal deps | `easypdf-core`, `easypdf-derive`, `easypdf-reader`, `easypdf-writer` via workspace (mandatory); `easypdf-markdown`, `easypdf-ocr`, `easypdf-runtime` via workspace (optional) | OK |

### easypdf-test (skip -- not published)

| Field | Value | Status |
|-------|-------|--------|
| publish | `false` | OK (skipped) |

---

## 2. Dry-Run Results

### easypdf-core -- PASSED

```
Packaged 45 files, 341.1KiB (79.5KiB compressed)
Uploading easypdf-core v0.1.0
warning: aborting upload due to dry run
```

### easypdf-derive -- PASSED

```
Packaged 13 files, 77.7KiB (18.5KiB compressed)
Uploading easypdf-derive v0.1.0
warning: aborting upload due to dry run
```

### easypdf-reader -- EXPECTED PRE-PUBLISH FAILURE

```
error: no matching package named `easypdf-core` found
```

**Reason**: `easypdf-core` is not yet published on crates.io. Will pass after
easypdf-core is published and propagated.

### easypdf-writer -- EXPECTED PRE-PUBLISH FAILURE

Same as easypdf-reader.

### easypdf-markdown -- EXPECTED PRE-PUBLISH FAILURE

Same as easypdf-reader.

### easypdf-ocr -- EXPECTED PRE-PUBLISH FAILURE

Same as easypdf-reader.

### easypdf-runtime -- EXPECTED PRE-PUBLISH FAILURE

Same as easypdf-reader.

### easypdf -- EXPECTED PRE-PUBLISH FAILURE

Same as easypdf-reader.

---

## 3. Issues Found

### BLOCKING (must fix before publish)

#### B1: Path dependencies missing `version` field -- FIXED

All internal path dependencies now use `workspace = true`, which resolves to
`{ path = "...", version = "0.1.0" }` from the workspace root's
`[workspace.dependencies]`. The original manifest validation error ("all
dependencies must have a version requirement specified") is resolved.

**Fix applied**: Added all 8 internal crates to `[workspace.dependencies]` in
the root `Cargo.toml` with both `path` and `version`. Changed all 6 sub-crates
to use `xxx.workspace = true`.

#### B2: `easypdf-ocr` missing `readme` -- FIXED

Added `readme.workspace = true` and created `crates/easypdf-ocr/README.md`.

#### B3: `easypdf-runtime` missing `readme` -- FIXED

Added `readme.workspace = true` and created `crates/easypdf-runtime/README.md`.

### RECOMMENDED (non-blocking)

#### R1: No `keywords` on any crate -- FIXED

Added `keywords = ["pdf", "ocr", "markdown", "parser", "writer"]` to
`[workspace.package]`. All 8 crates inherit via `keywords.workspace = true`.

#### R2: No `categories` on any crate -- FIXED

Added `categories = ["parser-implementations", "encoding"]` to
`[workspace.package]`. All 8 crates inherit via `categories.workspace = true`.

#### R3: `easypdf-derive` dev-dependency on `easypdf-core` is path-only

```toml
[dev-dependencies]
easypdf-core = { path = "../easypdf-core" }
```

**Intentionally kept as path-only**. Dev-dependencies are excluded from the
published crate, so this does not block `cargo publish`. Using `workspace = true`
for dev-deps causes Cargo to attempt crates.io resolution during packaging,
which would fail for unpublished internal crates.

#### R4: `easypdf-runtime` has `[[bin]]` with `required-features`

The crate defines a binary `easypdf-mcp` gated on `features = ["mcp"]`.
This is fine for crates.io, but note that the binary will only be
installable via `cargo install easypdf-runtime --features mcp`.

---

## 4. Publish Command Sequence (for real publish)

Run after all blocking issues are fixed. Each `cargo publish` must complete
before the next (crates.io index needs ~30-45s to propagate).

```bash
# Set your crates.io token (if not already configured)
# cargo login <your-token>

cd /Users/wandl/workspaces/workspace-github-easy-4-rust/easypdf-rust

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

### Notes on publish order

- `easypdf-core` MUST go first (all others depend on it).
- `easypdf-derive`, `easypdf-reader`, `easypdf-writer` can go in any order
  after core.
- `easypdf-markdown` needs core + reader to be live on crates.io.
- `easypdf-ocr` needs core + markdown.
- `easypdf-runtime` needs core + reader + writer + markdown.
- `easypdf` (facade) needs all of the above.
- `easypdf-test` is `publish = false` -- skipped.

---

## 5. Post-Publish

- [ ] Create git tag `v0.1.0`
- [ ] Push tag: `git push origin v0.1.0`
- [ ] Create GitHub Release with changelog
- [ ] Verify each crate appears on crates.io:
  - https://crates.io/crates/easypdf-core
  - https://crates.io/crates/easypdf-derive
  - https://crates.io/crates/easypdf-reader
  - https://crates.io/crates/easypdf-writer
  - https://crates.io/crates/easypdf-markdown
  - https://crates.io/crates/easypdf-ocr
  - https://crates.io/crates/easypdf-runtime
  - https://crates.io/crates/easypdf
- [ ] Verify docs.rs build: https://docs.rs/easypdf
- [ ] Announce (blog post, social, Discord, r/rust)

---

## 6. Changes Applied

### Root `Cargo.toml`

1. Added to `[workspace.package]`: `keywords`, `categories`
2. Added 8 internal crates to `[workspace.dependencies]` with `path` + `version`
3. Set `default-features = false` on `easypdf-runtime` workspace dep (facade controls features)

### Sub-crate `Cargo.toml` files (6 files)

| File | Changes |
|------|---------|
| `crates/easypdf-reader/Cargo.toml` | `easypdf-core` changed to `workspace = true`; added `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf-writer/Cargo.toml` | `easypdf-core` changed to `workspace = true`; added `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf-markdown/Cargo.toml` | `easypdf-core`, `easypdf-reader` changed to `workspace = true`; added `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf-ocr/Cargo.toml` | `easypdf-core`, `easypdf-markdown` changed to `workspace = true`; added `readme.workspace = true`, `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf-runtime/Cargo.toml` | `easypdf-core`, `easypdf-reader`, `easypdf-writer`, `easypdf-markdown` changed to `workspace = true`; added `readme.workspace = true`, `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf/Cargo.toml` | All 7 internal deps changed to `workspace = true` (3 optional); added `keywords.workspace = true`, `categories.workspace = true` |

### Also updated (non-blocking)

| File | Changes |
|------|---------|
| `crates/easypdf-core/Cargo.toml` | Added `keywords.workspace = true`, `categories.workspace = true` |
| `crates/easypdf-derive/Cargo.toml` | Added `keywords.workspace = true`, `categories.workspace = true` |

### New files

| File | Purpose |
|------|---------|
| `crates/easypdf-ocr/README.md` | Crate-level README for crates.io |
| `crates/easypdf-runtime/README.md` | Crate-level README for crates.io |

### Verification results

| Check | Result |
|-------|--------|
| `cargo check --workspace` | PASS |
| `cargo test --workspace` | PASS (all tests pass, 0 failures) |
| `cargo clippy --workspace --all-targets -D warnings` | PASS (0 warnings) |
| `cargo publish --dry-run` (leaf crates) | PASS (easypdf-core, easypdf-derive) |
| `cargo publish --dry-run` (dependent crates) | Expected pre-publish failure (deps not on crates.io) |
