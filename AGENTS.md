# AGENTS.md — Instructions for AI Coding Assistants & Autonomous Agents

> **Quick Context:** `msfPDF` is a sub-millisecond (`0.16–0.52 ms`), zero-rendering-dependency ISO 32000-1 (`%PDF-1.4`) vector document engine with 1:1 byte-identical parity across **Rust** (`crates/msfpdf-core`), **Web-Standard TypeScript** (`packages/ts`, `0` runtime `npm` dependencies — runs in Node 22+, Bun, Deno, Cloudflare Workers, and browsers), and a **standalone `< 1 MB` CLI** (`crates/msfpdf-cli`).
> It supports **Latin / Western European** (`/WinAnsiEncoding` octal escapes), **Russian & Slavic Cyrillic** (`А-Я`, `а-я`, `Ё`, `ё`, `₽`, `№`), and **Connected Arabic RTL** (`العربية` contextual Isolated/Initial/Medial/Final shaping, mandatory `لا`/`لأ`/`لإ`/`لآ` ligatures, and UAX #9 BiDi visual reordering) with searchable `/ToUnicode` CMaps.

## Machine-Readable Context & Schemas
- **Concise LLM Index:** [`llms.txt`](./llms.txt)
- **Single-Fetch Full Documentation:** [`llms-full.txt`](./llms-full.txt)
- **JSON Schema (Structured Outputs):** [`schemas/document-spec.schema.json`](./schemas/document-spec.schema.json)
- **Installable Agent Skill:** [`skills/msfpdf/SKILL.md`](./skills/msfpdf/SKILL.md)

## Build, Test & Security Gate Commands
Run all gates from the repository root before opening or updating any pull request:

```bash
python3 scripts/security-lint.py
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --workspace
npm run typecheck
npm run test:ts
npm run build:showcase
```

Hardened release binary build (strips local host paths via `--remap-path-prefix`):
```bash
./scripts/build-release.sh -p msfpdf-cli
```

## Non-Negotiable Architectural & Security Invariants
1. **Zero Runtime Dependencies in TypeScript:** `package.json` `"dependencies"` must remain `{}`. `scripts/security-lint.py` fails CI if any runtime dependency is added.
2. **Exact Integer Monetary Arithmetic:** `crates/msfpdf-core/src/money.rs` and `formatMoneyMinor` accept integer minor units (`pence`, `cents`, `centavos`, `kopecks`, `fils`, `halalas`, `yen`) across `GBP`, `EUR`, `USD`, `BRL`, `JPY`, `RUB`, `AED`, and `SAR`. Never introduce `f32`/`f64` inside `money.rs` or pass floats for currency values.
3. **100% 7-Bit ASCII `%PDF-1.4` Output (`0` `xref` Drift):** Every byte emitted by `assemble_pdf14` / `assemblePdf14` must be `\n` (`0x0A`) or printable 7-bit ASCII (`0x20..=0x7E`). Latin-1/CP1252 glyphs use `/WinAnsiEncoding` octal escapes; Cyrillic and shaped Arabic glyphs emit `<HEX> Tj` against dynamically subsetted `2048-UPM` Type 3 vector fonts (`/MsfVectorRegular*`, `/MsfVectorBold*`) with `/ToUnicode` CMaps.
4. **Rust ↔ TypeScript Byte Parity:** Any change to layout coordinates, AFM widths, QR mask scoring, Arabic shaping, BiDi ordering, or PDF object numbering in `crates/msfpdf-core` **must** be mirrored identically in `packages/ts/src` and regenerated into `docs/msfpdf-browser.js` via `npm run build:showcase`.
5. **No Active PDF Actions or Remote Fetch Surface:** Tokens `/JavaScript`, `/JS`, `/Launch`, `/OpenAction`, `/RichMedia`, and `/SubmitForm` are banned across the codebase to eliminate SSRF, LFI, and reader-exploit vectors.
