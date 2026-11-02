#!/usr/bin/env python3
"""Automated Security & Architectural Invariant Gate for msfPDF."""

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
    for path in source_files:
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8")

        for token in FORBIDDEN_PDF_ACTIONS:
            if token in text:
                fail(errors, f"{rel} contains forbidden active PDF dictionary token '{token}'")

        if "/Encoding /WinAnsiEncoding" in text:
            win_ansi_count += 1

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


def check_zero_local_path_leaks(errors: list[str]) -> None:
    tracked_dirs = [
        ROOT / "crates",
        ROOT / "packages",
        ROOT / "examples",
        ROOT / ".github",
    ]
    files = [p for d in tracked_dirs if d.exists() for p in d.rglob("*") if p.is_file()]
    for extra in (ROOT / "README.md", ROOT / "SECURITY.md", ROOT / "Cargo.toml", ROOT / "package.json"):
        if extra.exists():
            files.append(extra)

    local_home_re = re.compile(r"/home/[a-zA-Z0-9_-]+/|C:\\Users\\", re.IGNORECASE)
    for path in files:
        if path.suffix in {".png", ".jpg", ".jpeg", ".pdf"}:
            continue
        rel = path.relative_to(ROOT).as_posix()
        text = path.read_text(encoding="utf-8", errors="ignore")
        if local_home_re.search(text):
            fail(errors, f"{rel} leaks a host filesystem path")


def main() -> int:
    errors: list[str] = []
    check_zero_npm_runtime_deps(errors)
    check_integer_money_module(errors)
    check_pdf_stream_and_unsafe_hygiene(errors)
    check_zero_local_path_leaks(errors)

    if errors:
        for err in errors:
            print(f"[SECURITY-LINT FAIL] {err}", file=sys.stderr)
        return 1

    print("[SECURITY-LINT OK] All 4 architectural & security invariant suites passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
