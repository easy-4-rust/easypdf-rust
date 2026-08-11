# Golden text extraction files

Each `.txt` file contains the expected text output from
`EasyPdf::read("parity/samples/<name>.pdf").extract_text()`.

## Normalization

Golden files use LF line endings. The parity tests normalize both actual
and expected output by:
- Converting `\r\n` to `\n`
- Trimming trailing whitespace per line
- Trimming leading/trailing blank lines

This avoids platform-specific differences while still catching content changes.

## Regeneration

```bash
./scripts/generate_golden.sh
```
