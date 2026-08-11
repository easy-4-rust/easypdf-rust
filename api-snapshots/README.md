# API Snapshots

This directory contains public API snapshots for easypdf-rust crates, used to detect unintentional breaking changes.

## Files

| File | Crate |
|------|-------|
| `easypdf.txt` | `easypdf` (public facade) |
| `easypdf-core.txt` | `easypdf-core` (traits, types, error) |
| `easypdf-reader.txt` | `easypdf-reader` (PDF reading) |
| `easypdf-writer.txt` | `easypdf-writer` (PDF writing) |
| `easypdf-markdown.txt` | `easypdf-markdown` (Markdown conversion) |
| `workspace.txt` | Combined snapshot of all above |

## Generate Snapshots

Requires a nightly toolchain (for rustdoc JSON backend):

```bash
cargo install cargo-public-api

# Per crate
cargo public-api -p easypdf > api-snapshots/easypdf.txt
cargo public-api -p easypdf-core > api-snapshots/easypdf-core.txt
cargo public-api -p easypdf-reader > api-snapshots/easypdf-reader.txt
cargo public-api -p easypdf-writer > api-snapshots/easypdf-writer.txt
cargo public-api -p easypdf-markdown > api-snapshots/easypdf-markdown.txt

# Combined workspace snapshot
cat api-snapshots/easypdf-core.txt \
    api-snapshots/easypdf.txt \
    api-snapshots/easypdf-reader.txt \
    api-snapshots/easypdf-writer.txt \
    api-snapshots/easypdf-markdown.txt \
    > api-snapshots/workspace.txt
```

## CI Check

CI compares the current public API against the committed snapshot.
If the diff is non-empty, the `api-check` job fails.

## Update Snapshots (Intentional Breaking Change)

When you intentionally change the public API:

```bash
# Regenerate
cargo public-api -p easypdf > api-snapshots/easypdf.txt
# ... (repeat for other crates, then regenerate workspace.txt)

git add api-snapshots/
git commit -m "chore: update API snapshot for <version>"
```
