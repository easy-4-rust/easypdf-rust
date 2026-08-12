# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

Only the latest release in the `0.1.x` line receives security patches.

## Reporting a Vulnerability

If you discover a security vulnerability in easypdf-rust, **please do not open a public issue.**

Instead, report it responsibly via one of the following channels:

- **Email**: [security@easy-4-rust.com](mailto:security@easy-4-rust.com)
- **GitHub Security Advisories**: Use the [Report a vulnerability](https://github.com/easy-4-rust/easypdf-rust/security/advisories/new) button on the Security tab of this repository

Please include the following in your report:

1. A description of the vulnerability
2. Steps to reproduce or a proof of concept
3. The affected version(s)
4. Any potential impact you have identified

## Response Process

| Step | Timeline | Description |
|------|----------|-------------|
| Acknowledgment | Within 48 hours | We confirm receipt of your report |
| Assessment | Within 5 business days | We evaluate severity and scope |
| Patch | Target: within 30 days | We develop and test a fix |
| Advisory | After patch release | We publish a GitHub Security Advisory crediting you (unless you prefer anonymity) |

We follow [coordinated disclosure](https://en.wikipedia.org/wiki/Coordinated_vulnerability_disclosure). We ask that you give us reasonable time to address the vulnerability before public disclosure.

## Known Security Status

The following advisories have been assessed against this project:

### Fixed

| Advisory | Status | Notes |
|----------|--------|-------|
| [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071) (rsa Marvin Attack) | Fixed | Only affects dev-dependency (test usage), not in production code path |
| [RUSTSEC-2025-0055](https://rustsec.org/advisories/RUSTSEC-2025-0055) (tracing-subscriber ANSI escape) | Fixed | Upgraded to patched version |

### Accepted Risk

| Advisory | Status | Notes |
|----------|--------|-------|
| [RUSTSEC-2026-0253](https://rustsec.org/advisories/RUSTSEC-2026-0253) (lru 0.16.4 unsound) | Accepted | `azul-layout 0.0.13` hard-pins `lru = "^0.16.1"`. Cargo does not allow patching across semver boundaries. This advisory is classified as "unsound" (warning), not "vulnerability" (error). Will be re-evaluated when azul-layout releases a version depending on lru >= 0.17. |

## Security Best Practices for Users

- Always use the latest released version of easypdf-rust.
- Run `cargo audit` in your project to check for known vulnerabilities in your dependency tree.
- If you are processing untrusted PDF input, be aware that PDF parsing is inherently complex. Keep your dependencies up to date.

## Acknowledgments

We thank the following researchers and community members for responsibly disclosing vulnerabilities:

_(None yet -- be the first!)_
