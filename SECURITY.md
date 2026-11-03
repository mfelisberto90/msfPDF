# Security Policy & Threat Model

## Reporting a Vulnerability

Please do **not** open public GitHub issues for suspected security vulnerabilities. Use [GitHub Private Vulnerability Reporting](https://github.com/mfelisberto90/msfPDF/security/advisories/new) on this repository so we can validate, patch, and publish a regression test before disclosure.

We aim to acknowledge reports within 48 hours.

## Supported Versions

| Version | Rust MSRV | Node / TypeScript / Edge | Supported |
| :--- | :--- | :--- | :---: |
| `0.2.x` | Rust `1.80+` | Node.js `22.6+` / Bun / Deno / Workers / Browser ES2022 | Yes |
| `0.1.x` | Rust `1.80+` | Node.js `22.6+` / Bun / Deno / Workers | Security fixes |

## Threat Model & Built-in Mitigations

Most server-side PDF incidents trace back to headless browser SSRF (`<iframe src="http://169.254.169.254/...">`), local file inclusion (`file:///etc/passwd`), memory-safety bugs in C/C++ OpenType font parsers (`FreeType` / `HarfBuzz`), unescaped PDF stream operators, or predictable file URLs leaking multi-tenant documents. **msfPDF** eliminates those classes structurally:

1. **Zero Network / Zero HTML Surface (SSRF & LFI Immunity)**: Neither `msfpdf-core` (Rust) nor `msfpdf` (TypeScript) parses HTML, CSS, SVG, JavaScript, or remote URLs. All output is compiled in-memory from typed `DocumentSpec` blocks into static `%PDF-1.4` vector primitives.
2. **Zero Untrusted Font-Binary Parsing (Type 3 Compile-Time Vector Table)**: Multilingual Russian Cyrillic and connected Arabic RTL glyphs are emitted from a compile-time constant table of 273 sanitized cubic Bezier `CharProcs` (`2048 UPM`) paired with deterministic UTF-16BE `/ToUnicode` CMaps. No `.ttf`, `.otf`, or `.woff2` binary is ever parsed at runtime.
3. **PDF Stream Operator Injection Neutralisation**: Every dynamic WinAnsi run passes through `pdf_escape_win_ansi` / `pdfEscapeWinAnsi` (`\\`, `\(`, `\)` + 3-digit octal escapes), while Type 3 runs emit strictly 2-digit hexadecimal glyph indices (`<HEX> Tj`). Active PDF dictionaries (`/JavaScript`, `/OpenAction`, `/Launch`, `/RichMedia`, `/SubmitForm`) are banned in CI by `scripts/security-lint.py`.
4. **Pagination & Payload DoS Ceilings (`SecurityLimits`)**: Untrusted callers cannot trigger memory exhaustion or infinite pagination loops. Default bounds reject JSON payloads above `256 KiB`, block counts above `500`, table row counts above `1,000`, and rendered page counts above `50` before allocating unbounded buffers.
5. **Bounded Header-Only JPEG Validation (`/DCTDecode`)**: Optional embedded logos (`logo_jpeg_base64`) enforce a `256 KiB` byte ceiling, verify `FFD8` SOI magic bytes and `SOF0..SOF2` 8-bit frame headers (`1..=4096 px`, Grayscale or RGB only), and wrap bytes inside 7-bit `/ASCIIHexDecode` streams without invoking unsafe C image decoders.
6. **Fail-Closed Multi-Tenant Vault (`TenantPdfVault`)**: When persisting generated documents on disk, `TenantPdfVault` enforces strict `tenant_id` validation, generates `160-bit` (`40` hex char) CSPRNG tokens, creates `0700` per-tenant directories, writes to dot-prefixed `.tmp` files with `0600` permissions + `fsync`, and atomically renames into place. Cross-tenant token reads and `..` / `/` / `\0` traversal inputs fail closed immediately.
7. **Cryptographic Signing & Tamper-Evidence (Ed25519 / RFC 8032)**: Native Ed25519 signing binds the SHA-256 digest of the base PDF to a cryptographic identity without relying on bloated X.509 PKI or heavy native libraries. Signatures can be rendered visually in the document via `audit_manifest` or embedded into `%MsfAuditManifest: {...}\n` trailing comments post-`%%EOF`, preserving `xref` table offsets for PDF readers while enabling 1-line mathematical tamper-evidence.
8. **Tokenless Supply Chain Security (OIDC Trusted Publishing)**: Package publishing to `npm` (`npm publish --provenance`) and `crates.io` (`cargo publish`) runs exclusively via short-lived GitHub Actions OpenID Connect (OIDC) identity tokens (`id-token: write`). No static, long-lived API tokens or credentials are stored as repository secrets, meeting SLSA Level 2 supply-chain provenance standards.
9. **Hardened Interactive Showcase (`docs/`)**: The browser compiler studio enforces a hash-locked `Content-Security-Policy` (`object-src 'none'; base-uri 'none'; form-action 'self'`), uses `textContent` / `createElementNS` exclusively (zero `innerHTML` / `eval`), and publishes a clean `robots.txt` with zero internal-route `Disallow:` reconnaissance leaks.

## Data Classification Tiers

| Tier | Examples in msfPDF | Handling Standard |
| :--- | :--- | :--- |
| `public` | Theme presets, page geometry (`A4`, `Letter`, `Thermal80mm`), Public Domain glyph outlines | Pure compile-time constants |
| `internal` | Block layout structure, column weights, pagination counts | Validated against `SecurityLimits` |
| `personal` / `sensitive` | Counterparty names, multilingual clauses, minor-unit invoice totals | Escaped in memory; persisted exclusively with `0600` permissions; never echoed in `PdfError` messages |
| `highly_sensitive` | `160-bit` vault capability tokens | Generated via OS CSPRNG (`getrandom` / `node:crypto`); enumerable tokens rejected |

## Automated CI Invariant Gate

Every pull request runs `python3 scripts/security-lint.py` alongside `cargo clippy -- -D warnings`, `cargo test`, `tsc --noEmit`, `node --test`, and `npm run build:showcase` to enforce zero runtime NPM dependencies, zero binary floats in monetary formatting, explicit `/Encoding /WinAnsiEncoding` and `/Subtype /Type3` + `/ToUnicode` declarations, zero dangerous PDF action dictionaries, zero DOM XSS sinks in `docs/`, and zero host path leaks.
