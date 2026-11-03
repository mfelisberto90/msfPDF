#!/usr/bin/env python3
"""Automated Security, Multilingual & AI-Discovery Invariant Gate for msfPDF."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

FORBIDDEN_PDF_ACTIONS = (
    "/JavaScript",
    "/JS",
    "/Launch",
    "/OpenAction",
    "/RichMedia",
    "/SubmitForm",
)


def fail(errors: list[str], msg: str) -> None:
    errors.append(msg)


def check_zero_npm_runtime_deps(errors: list[str]) -> None:
    pkg_path = ROOT / "package.json"
    data = json.loads(pkg_path.read_text(encoding="utf-8"))
    deps = data.get("dependencies", {})
    if deps:
        fail(
            errors,
            f"package.json must maintain zero runtime dependencies, found: {list(deps.keys())}",
        )


def check_integer_money_module(errors: list[str]) -> None:
    money_rs = (ROOT / "crates/msfpdf-core/src/money.rs").read_text(encoding="utf-8")
    if re.search(r"\bf32\b|\bf64\b", money_rs):
        fail(errors, "crates/msfpdf-core/src/money.rs must not use f32 or f64")


def check_pdf_stream_and_unsafe_hygiene(errors: list[str]) -> None:
    source_files = list((ROOT / "crates").rglob("*.rs")) + list(
        (ROOT / "packages/ts").rglob("*.ts")
    )

    win_ansi_count = 0
    type3_count = 0
    tounicode_count = 0
    for path in source_files:
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")

        for token in FORBIDDEN_PDF_ACTIONS:
            if token in text:
                fail(errors, f"{rel} contains forbidden active PDF dictionary token '{token}'")

        if "/Encoding /WinAnsiEncoding" in text:
            win_ansi_count += 1
        if "/Subtype /Type3" in text:
            type3_count += 1
        if "/ToUnicode" in text:
            tounicode_count += 1

        if path.suffix == ".rs" and re.search(r"\bunsafe\s*\{", text):
            allowed = rel in (
                "crates/msfpdf-core/src/vault.rs",
                "crates/msfpdf-cli/src/main.rs",
            )
            if not allowed or ("malloc_trim" not in text and "mallopt" not in text):
                fail(errors, f"{rel} contains unapproved unsafe block")

        if "/tests/" in f"/{rel}" or rel.endswith("lib.rs"):
            if re.search(r"assert!\s*\(\s*true\s*\)|assert\.ok\s*\(\s*true\s*\)", text):
                fail(errors, f"{rel} contains tautological test assertion")

    if win_ansi_count < 2:
        fail(
            errors,
            "both Rust and TypeScript assemblers must declare explicit /Encoding /WinAnsiEncoding",
        )
    if type3_count < 2 or tounicode_count < 2:
        fail(
            errors,
            "both Rust and TypeScript assemblers must emit /Subtype /Type3 with /ToUnicode CMaps",
        )


def check_showcase_and_ai_discovery_hygiene(errors: list[str]) -> None:
    required_files = (
        "llms.txt",
        "llms-full.txt",
        "AGENTS.md",
        "CITATION.cff",
        "codemeta.json",
        "context7.json",
        "THIRD_PARTY_NOTICES.md",
        "schemas/document-spec.schema.json",
        "skills/msfpdf/SKILL.md",
        "docs/index.html",
        "docs/showcase.js",
        "docs/msfpdf-browser.min.js",
        "docs/robots.txt",
        "docs/sitemap.xml",
    )
    for rel in required_files:
        if not (ROOT / rel).is_file():
            fail(errors, f"missing required showcase / AI-discovery artifact: {rel}")

    robots_path = ROOT / "docs/robots.txt"
    if robots_path.is_file():
        robots_txt = robots_path.read_text(encoding="utf-8")
        if re.search(r"^\s*Disallow\s*:\s*\S+", robots_txt, re.IGNORECASE | re.MULTILINE):
            fail(errors, "docs/robots.txt must not expose internal paths via non-empty Disallow:")

    showcase_js = ROOT / "docs/showcase.js"
    if showcase_js.is_file():
        js_text = showcase_js.read_text(encoding="utf-8")
        if re.search(r"\b(innerHTML|outerHTML|insertAdjacentHTML|eval)\b", js_text):
            fail(errors, "docs/showcase.js must not use dangerous DOM sinks (innerHTML/eval)")


def check_zero_local_path_leaks(errors: list[str]) -> None:
    tracked_dirs = [
        ROOT / "crates",
        ROOT / "packages",
        ROOT / "examples",
        ROOT / "docs",
        ROOT / "schemas",
        ROOT / "skills",
        ROOT / "scripts",
        ROOT / ".github",
    ]
    files = [p for d in tracked_dirs if d.exists() for p in d.rglob("*") if p.is_file()]
    for extra in (
        ROOT / "README.md",
        ROOT / "SECURITY.md",
        ROOT / "AGENTS.md",
        ROOT / "CITATION.cff",
        ROOT / "codemeta.json",
        ROOT / "context7.json",
        ROOT / "llms.txt",
        ROOT / "llms-full.txt",
        ROOT / "THIRD_PARTY_NOTICES.md",
        ROOT / "Cargo.toml",
        ROOT / "package.json",
    ):
        if extra.exists():
            files.append(extra)

    local_home_re = re.compile(r"/home/[a-zA-Z0-9_-]+/|" + r"C:\\Users\\", re.IGNORECASE)
    for path in files:
        if path.suffix in {".png", ".jpg", ".jpeg", ".pdf"}:
            continue
        rel = path.relative_to(ROOT).as_posix()
        if rel == "scripts/security-lint.py":
            continue
        text = path.read_text(encoding="utf-8", errors="ignore")
        if local_home_re.search(text):
            fail(errors, f"{rel} leaks a host filesystem path")


def main() -> int:
    errors: list[str] = []
    check_zero_npm_runtime_deps(errors)
    check_integer_money_module(errors)
    check_pdf_stream_and_unsafe_hygiene(errors)
    check_showcase_and_ai_discovery_hygiene(errors)
    check_zero_local_path_leaks(errors)

    if errors:
        for err in errors:
            print(f"[SECURITY-LINT FAIL] {err}", file=sys.stderr)
        return 1

    print("[SECURITY-LINT OK] All 5 architectural, multilingual & AI-discovery invariant suites passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
