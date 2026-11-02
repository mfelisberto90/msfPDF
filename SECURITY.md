# Security Policy & Threat Model

## Reporting a Vulnerability

Please do **not** open public GitHub issues for suspected security vulnerabilities. Use [GitHub Private Vulnerability Reporting](https://github.com/mfelisberto90/msfPDF/security/advisories/new) on this repository so we can validate, patch, and publish a regression test before disclosure.

We aim to acknowledge reports within 48 hours.

## Supported Versions

| Version | Rust MSRV | Node / TypeScript | Supported |
| :--- | :--- | :--- | :--- |
| `0.1.x` | Rust `1.80+` | Node.js `22.6+` / Bun / Deno / Workers | Yes |

## Threat Model & Built-in Mitigations

Most server-side PDF incidents trace back to headless browser SSRF (`<iframe src="http://169.254.169.254/...">`), local file inclusion (`file:///etc/passwd`), unescaped PDF stream operators, or predictable file URLs leaking multi-tenant documents. **msfPDF** eliminates those classes structurally:

1. **Zero Network / Zero HTML Surface (SSRF & LFI Immunity)**: Neither `msfpdf-core` (Rust) nor `msfpdf` (TypeScript) parses HTML, CSS, SVG, JavaScript, or remote URLs. All output is compiled in-memory from typed `DocumentSpec` blocks into static `%PDF-1.4` vector primitives.
2. **PDF Stream Operator Injection Neutralisation**: Every dynamic string passes through `pdf_escape_win_ansi` / `pdfEscapeWinAnsi` before entering `(...) Tj` text literals. Literal `\`, `(`, and `)` characters are escaped (`\\`, `\(`, `\)`), control bytes (`0x00..=0x1F`, `0x7F`) collapse to single spaces, and non-ASCII CP1252 characters map to 3-digit 7-bit ASCII octal escapes (`\243` for `£`, `\200` for `€`, `\343` for `ã`). Active PDF dictionaries (`/JavaScript`, `/OpenAction`, `/Launch`, `/RichMedia`, `/SubmitForm`) are banned in CI by `scripts/security-lint.py`.
3. **Pagination & Payload DoS Ceilings (`SecurityLimits`)**: Untrusted callers cannot trigger memory exhaustion or infinite pagination loops. Default bounds reject JSON payloads above `256 KiB`, block counts above `500`, table row counts above `1,000`, and rendered page counts above `50` before allocating unbounded buffers.
4. **Bounded Header-Only JPEG Validation (`/DCTDecode`)**: Optional embedded logos (`logo_jpeg_base64`) enforce a `256 KiB` byte ceiling, verify `FFD8` SOI magic bytes and `SOF0..SOF2` 8-bit frame headers (`1..=4096 px`, Grayscale or RGB only), and wrap bytes inside 7-bit `/ASCIIHexDecode` streams without invoking unsafe C image decoders.
5. **Fail-Closed Multi-Tenant Vault (`TenantPdfVault`)**: When persisting generated documents on disk, `TenantPdfVault` enforces strict `tenant_id` validation, generates `160-bit` (`40` hex char) CSPRNG tokens, creates `0700` per-tenant directories, writes to dot-prefixed `.tmp` files with `0600` permissions + `fsync`, and atomically renames into place. Cross-tenant token reads and `..` / `/` / `\0` traversal inputs fail closed immediately.

## Data Classification Tiers

| Tier | Examples in msfPDF | Handling Standard |
| :--- | :--- | :--- |
| `public` | Theme presets, page geometry (`A4`, `Letter`, `Thermal80mm`) | Pure compile-time constants |
| `internal` | Block layout structure, column weights, pagination counts | Validated against `SecurityLimits` |
| `personal` / `sensitive` | Billed party names, contract clauses, minor-unit invoice totals | Escaped in memory; persisted exclusively with `0600` permissions; never echoed in `PdfError` messages |
| `highly_sensitive` | `160-bit` vault capability tokens | Generated via OS CSPRNG (`getrandom` / `node:crypto`); enumerable tokens rejected |

## Automated CI Invariant Gate

Every pull request runs `python3 scripts/security-lint.py` alongside `cargo clippy -- -D warnings`, `cargo test`, `tsc --noEmit`, and `node --test` to enforce zero runtime NPM dependencies, zero binary floats in monetary formatting, explicit `/WinAnsiEncoding` font declarations, zero dangerous PDF action dictionaries, and zero host path leaks.
