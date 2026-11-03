use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::time::Instant;

use msfpdf_core::{
    compute_sha256_hex, decode_hex_exact, embed_audit_manifest, extract_audit_manifest,
    generate_keypair, hex_encode, render_document, render_document_json, sign_pdf,
    strip_embedded_audit, verify_manifest, verify_pdf, AuditManifest, BannerTone, BentoCardItem,
    Currency, DocumentBlock, DocumentMetadata, DocumentSpec, FinancialLineItem, PageFormat,
    SecurityLimits, SignerSpec, TableColumnSpec, TenantPdfVault, TextAlign, ThemePreset,
};

fn tune_linux_allocator() {
    #[cfg(target_os = "linux")]
    unsafe {
        let _ = libc::mallopt(libc::M_ARENA_MAX, 2);
        let _ = libc::mallopt(libc::M_MMAP_THRESHOLD, 64 * 1024);
    }
}

fn read_bounded_input(path_or_dash: &str, max_bytes: usize) -> Result<Vec<u8>, String> {
    if path_or_dash == "-" {
        let stdin = io::stdin();
        let mut buf = Vec::with_capacity(8192);
        stdin
            .take((max_bytes + 1) as u64)
            .read_to_end(&mut buf)
            .map_err(|e| format!("failed reading stdin: {e}"))?;
        if buf.len() > max_bytes {
            return Err(format!(
                "stdin payload exceeds {max_bytes} byte security ceiling"
            ));
        }
        Ok(buf)
    } else {
        let meta = fs::metadata(path_or_dash)
            .map_err(|e| format!("cannot stat input '{path_or_dash}': {e}"))?;
        if meta.len() > max_bytes as u64 {
            return Err(format!(
                "input file exceeds {max_bytes} byte security ceiling"
            ));
        }
        fs::read(path_or_dash).map_err(|e| format!("cannot read '{path_or_dash}': {e}"))
    }
}

fn print_usage() {
    eprintln!(
        "msfpdf v{} — Sub-millisecond Zero-Dependency %PDF-1.4 Engine\n\
         Usage:\n\
         \x20 msfpdf render --in <spec.json | -> --out <output.pdf | -> [--max-pages N]\n\
         \x20 msfpdf vault-issue --vault <dir> --tenant <id> [--prefix doc] --in <spec.json | ->\n\
         \x20 msfpdf bench [--iterations 5000]\n\
         \x20 msfpdf keygen [--out <keys.json>]\n\
         \x20 msfpdf sign --in <doc.pdf | -> --key <seed_hex> [--identity <name>] [--timestamp <iso>] [--embed] [--out <out | ->]\n\
         \x20 msfpdf verify --in <doc.pdf | -> --pubkey <pub_hex> [--sig <manifest.json | sig_hex>]\n\
         \x20 msfpdf --version",
        env!("CARGO_PKG_VERSION")
    );
}

fn current_utc_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let days = secs / 86400;
    let rem_secs = secs % 86400;
    let hours = rem_secs / 3600;
    let mins = (rem_secs % 3600) / 60;
    let s = rem_secs % 60;
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1024 + doe / 1461 - doe / 142024) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{s:02}Z")
}

fn build_benchmark_spec() -> DocumentSpec {
    DocumentSpec {
        page_format: PageFormat::A4,
        theme: ThemePreset::NavyFintech,
        accent_hex_override: None,
        metadata: DocumentMetadata {
            issuer_name: "Caledonia Cloud Systems Ltd".into(),
            document_code: "#INV-2026-8841".into(),
            issued_at_label: "02 Nov 2026".into(),
            footer_notice: None,
            page_counter_template: "Page {current} of {total}".into(),
        },
        continuation_header: None,
        blocks: vec![
            DocumentBlock::HeroHeader {
                kicker: "ENTERPRISE SUBSCRIPTION INVOICE • UK VAT REG GB40928192".into(),
                title: "Caledonia Cloud Systems Ltd".into(),
                subtitle: "Dedicated Single-Tenant Infrastructure & Managed Compliance".into(),
                badge_label: Some("CRYPTOGRAPHIC REF".into()),
                badge_code: Some("#INV-2026-8841".into()),
                badge_meta: Some("Issued: 02/11/2026 • Net 30".into()),
                logo_jpeg_base64: None,
            },
            DocumentBlock::BentoGrid {
                columns: 2,
                cards: vec![
                    BentoCardItem {
                        primary_label: "BILLED CUSTOMER".into(),
                        primary_value: "Northbridge Logistics PLC".into(),
                        secondary_label: Some("BILLING CONTACT".into()),
                        secondary_value: Some("Eleanor Vance (Accounts Payable)".into()),
                    },
                    BentoCardItem {
                        primary_label: "SUPPLIER ENTITY".into(),
                        primary_value: "Caledonia Cloud Systems Ltd".into(),
                        secondary_label: Some("JURISDICTION".into()),
                        secondary_value: Some("Edinburgh, Scotland, United Kingdom".into()),
                    },
                ],
            },
            DocumentBlock::SectionHeading {
                title: "1. USAGE & SUBSCRIPTION SCHEDULE".into(),
            },
            DocumentBlock::DataTable {
                columns: vec![
                    TableColumnSpec {
                        header: "SERVICE DESCRIPTION".into(),
                        weight: 3.0,
                        align: TextAlign::Left,
                        bold_values: true,
                    },
                    TableColumnSpec {
                        header: "QTY".into(),
                        weight: 0.8,
                        align: TextAlign::Center,
                        bold_values: false,
                    },
                    TableColumnSpec {
                        header: "NET AMOUNT".into(),
                        weight: 1.2,
                        align: TextAlign::Right,
                        bold_values: true,
                    },
                ],
                rows: vec![
                    vec![
                        "Dedicated Cluster Nodes (eu-west-2)".into(),
                        "4".into(),
                        "£3,200.00".into(),
                    ],
                    vec![
                        "Automated Offsite DR Vault (10 TB)".into(),
                        "1".into(),
                        "£450.00".into(),
                    ],
                    vec![
                        "24/7 Incident Response Retainer".into(),
                        "1".into(),
                        "£850.00".into(),
                    ],
                ],
                zebra: true,
            },
            DocumentBlock::FinancialSummary {
                currency: Currency::GBP,
                rows: vec![
                    FinancialLineItem {
                        label: "Subtotal (Excl. UK VAT)".into(),
                        amount_minor: Some(450_000),
                        currency: None,
                        text_value: None,
                        emphasize: false,
                    },
                    FinancialLineItem {
                        label: "UK Standard VAT (20.0%)".into(),
                        amount_minor: Some(90_000),
                        currency: None,
                        text_value: None,
                        emphasize: false,
                    },
                ],
            },
            DocumentBlock::CalloutBanner {
                title: "TOTAL AMOUNT DUE (GBP)".into(),
                subtitle: "Payable via Faster Payments / CHAPS within 30 calendar days".into(),
                right_value: Some("£5,400.00".into()),
                tone: BannerTone::Accent,
            },
            DocumentBlock::QrVerification {
                payload: "https://verify.example.co.uk/inv/2026-8841-9f8a7b6c".into(),
                headline: "INSTANT CRYPTOGRAPHIC AUDIT VERIFICATION".into(),
                details:
                    "Scan this vector QR code to verify SHA-256 document integrity and settlement status."
                        .into(),
                code_display: Some("SHA256: 9F8A-7B6C-4410-EE29".into()),
            },
            DocumentBlock::ClauseList {
                paragraphs: vec![
                    "• Title to deliverables and SLA credits remain governed by the Master Services Agreement under Scots law.".into(),
                    "• Late commercial settlements accrue statutory interest pursuant to the Late Payment of Commercial Debts (Interest) Act 1998.".into(),
                ],
            },
            DocumentBlock::SignatureGrid {
                heading: Some("2. AUTHORISED APPROVALS".into()),
                signers: vec![
                    SignerSpec {
                        role_label: "FOR SUPPLIER".into(),
                        party_name: "Caledonia Cloud Systems Ltd".into(),
                    },
                    SignerSpec {
                        role_label: "FOR CUSTOMER".into(),
                        party_name: "Northbridge Logistics PLC".into(),
                    },
                ],
            },
        ],
    }
}

fn run() -> Result<(), String> {
    tune_linux_allocator();
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        print_usage();
        return Ok(());
    }
    if args[0] == "--version" || args[0] == "-V" {
        println!("msfpdf {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    match args[0].as_str() {
        "render" => {
            let mut in_path = "-";
            let mut out_path = "-";
            let mut limits = SecurityLimits::default();
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--in" if i + 1 < args.len() => {
                        in_path = &args[i + 1];
                        i += 2;
                    }
                    "--out" if i + 1 < args.len() => {
                        out_path = &args[i + 1];
                        i += 2;
                    }
                    "--max-pages" if i + 1 < args.len() => {
                        limits.max_pages = args[i + 1]
                            .parse()
                            .map_err(|_| "invalid --max-pages integer")?;
                        i += 2;
                    }
                    other => return Err(format!("unrecognised flag '{other}'")),
                }
            }

            let raw_json = read_bounded_input(in_path, limits.max_input_bytes)?;
            let pdf_bytes = render_document_json(&raw_json, limits).map_err(|e| e.to_string())?;
            if out_path == "-" {
                let mut stdout = io::stdout().lock();
                stdout
                    .write_all(&pdf_bytes)
                    .map_err(|e| format!("stdout write failed: {e}"))?;
            } else {
                fs::write(out_path, &pdf_bytes)
                    .map_err(|e| format!("cannot write '{out_path}': {e}"))?;
            }
            Ok(())
        }

        "vault-issue" => {
            let mut vault_dir: Option<&str> = None;
            let mut tenant_id: Option<&str> = None;
            let mut prefix = "document";
            let mut in_path = "-";
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--vault" if i + 1 < args.len() => {
                        vault_dir = Some(&args[i + 1]);
                        i += 2;
                    }
                    "--tenant" if i + 1 < args.len() => {
                        tenant_id = Some(&args[i + 1]);
                        i += 2;
                    }
                    "--prefix" if i + 1 < args.len() => {
                        prefix = &args[i + 1];
                        i += 2;
                    }
                    "--in" if i + 1 < args.len() => {
                        in_path = &args[i + 1];
                        i += 2;
                    }
                    other => return Err(format!("unrecognised flag '{other}'")),
                }
            }

            let v_dir = vault_dir.ok_or("missing --vault <dir>")?;
            let t_id = tenant_id.ok_or("missing --tenant <id>")?;
            let limits = SecurityLimits::default();
            let raw_json = read_bounded_input(in_path, limits.max_input_bytes)?;
            let spec: DocumentSpec = serde_json::from_slice(&raw_json)
                .map_err(|e| format!("invalid JSON specification: {e}"))?;
            let vault = TenantPdfVault::new(v_dir, prefix).map_err(|e| e.to_string())?;
            let issued = vault
                .issue_document(t_id, &spec)
                .map_err(|e| e.to_string())?;
            println!(
                "{{\"token\":\"{}\",\"file_name\":\"{}\",\"bytes\":{}}}",
                issued.token,
                issued.file_name,
                issued.bytes.len()
            );
            Ok(())
        }

        "bench" => {
            let mut iterations = 5000usize;
            if args.len() >= 3 && args[1] == "--iterations" {
                iterations = args[2]
                    .parse::<usize>()
                    .map_err(|_| "invalid --iterations value")?
                    .clamp(100, 100_000);
            }
            let spec = build_benchmark_spec();
            let limits = SecurityLimits::default();

            for _ in 0..300 {
                let _ = std::hint::black_box(render_document(&spec, limits));
            }

            let mut samples_us = Vec::with_capacity(iterations);
            let mut byte_len = 0usize;
            for _ in 0..iterations {
                let t0 = Instant::now();
                let out = std::hint::black_box(
                    render_document(&spec, limits).map_err(|e| e.to_string())?,
                );
                samples_us.push(t0.elapsed().as_nanos() as f64 / 1000.0);
                byte_len = out.len();
            }
            samples_us.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let avg_us = samples_us.iter().sum::<f64>() / (iterations as f64);
            let p50_us = samples_us[iterations / 2];
            let p99_us = samples_us[(iterations * 99) / 100];
            let docs_per_sec = (1_000_000.0 / avg_us).round() as u64;

            println!(
                "{{\"engine\":\"msfpdf-rust\",\"iterations\":{},\"p50_us\":{:.2},\"avg_us\":{:.2},\"p99_us\":{:.2},\"pdf_bytes\":{},\"docs_per_sec_single_core\":{}}}",
                iterations, p50_us, avg_us, p99_us, byte_len, docs_per_sec
            );
            Ok(())
        }

        "keygen" => {
            let mut out_path: Option<&str> = None;
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--out" if i + 1 < args.len() => {
                        out_path = Some(&args[i + 1]);
                        i += 2;
                    }
                    other => return Err(format!("unrecognised flag '{other}'")),
                }
            }
            let (seed, pubkey) = generate_keypair().map_err(|e| e.to_string())?;
            let json = serde_json::json!({
                "algorithm": "Ed25519",
                "private_key_seed_hex": hex_encode(&seed),
                "public_key_hex": hex_encode(&pubkey),
            });
            let formatted = serde_json::to_string_pretty(&json)
                .map_err(|e| format!("json serialization failed: {e}"))?;
            if let Some(path) = out_path {
                fs::write(path, formatted.as_bytes())
                    .map_err(|e| format!("cannot write '{path}': {e}"))?;
            } else {
                println!("{formatted}");
            }
            Ok(())
        }

        "sign" => {
            let mut in_path = "-";
            let mut out_path = "-";
            let mut key_hex: Option<&str> = None;
            let mut identity = "msfPDF Digital Signer";
            let mut timestamp: Option<String> = None;
            let mut embed = false;
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--in" if i + 1 < args.len() => {
                        in_path = &args[i + 1];
                        i += 2;
                    }
                    "--out" if i + 1 < args.len() => {
                        out_path = &args[i + 1];
                        i += 2;
                    }
                    "--key" if i + 1 < args.len() => {
                        key_hex = Some(&args[i + 1]);
                        i += 2;
                    }
                    "--identity" if i + 1 < args.len() => {
                        identity = &args[i + 1];
                        i += 2;
                    }
                    "--timestamp" if i + 1 < args.len() => {
                        timestamp = Some(args[i + 1].clone());
                        i += 2;
                    }
                    "--embed" => {
                        embed = true;
                        i += 1;
                    }
                    other => return Err(format!("unrecognised flag '{other}'")),
                }
            }
            let raw_key = key_hex.ok_or("missing required --key <seed_hex>")?;
            let seed_bytes: [u8; 32] = decode_hex_exact(raw_key).ok_or_else(|| {
                "invalid --key hex: must be 64 hexadecimal characters (32 bytes)".to_string()
            })?;

            let limits = SecurityLimits::default();
            let pdf_bytes = read_bounded_input(in_path, limits.max_input_bytes)?;
            let ts = timestamp.unwrap_or_else(current_utc_iso);
            let manifest = sign_pdf(&pdf_bytes, &seed_bytes, identity, Some(&ts))
                .map_err(|e| e.to_string())?;

            if embed {
                let signed_pdf = embed_audit_manifest(&pdf_bytes, &manifest);
                if out_path == "-" {
                    let mut stdout = io::stdout().lock();
                    stdout
                        .write_all(&signed_pdf)
                        .map_err(|e| format!("stdout write failed: {e}"))?;
                } else {
                    fs::write(out_path, &signed_pdf)
                        .map_err(|e| format!("cannot write '{out_path}': {e}"))?;
                }
            } else {
                let json = serde_json::to_string_pretty(&manifest)
                    .map_err(|e| format!("json serialization failed: {e}"))?;
                if out_path == "-" {
                    println!("{json}");
                } else {
                    fs::write(out_path, json.as_bytes())
                        .map_err(|e| format!("cannot write '{out_path}': {e}"))?;
                }
            }
            Ok(())
        }

        "verify" => {
            let mut in_path = "-";
            let mut pubkey_hex: Option<&str> = None;
            let mut sig_arg: Option<String> = None;
            let mut i = 1usize;
            while i < args.len() {
                match args[i].as_str() {
                    "--in" if i + 1 < args.len() => {
                        in_path = &args[i + 1];
                        i += 2;
                    }
                    "--pubkey" if i + 1 < args.len() => {
                        pubkey_hex = Some(&args[i + 1]);
                        i += 2;
                    }
                    "--sig" if i + 1 < args.len() => {
                        sig_arg = Some(args[i + 1].clone());
                        i += 2;
                    }
                    other => return Err(format!("unrecognised flag '{other}'")),
                }
            }

            let raw_pub = pubkey_hex.ok_or("missing required --pubkey <hex>")?;
            let pub_bytes: [u8; 32] = decode_hex_exact(raw_pub).ok_or_else(|| {
                "invalid --pubkey hex: must be 64 hexadecimal characters (32 bytes)".to_string()
            })?;

            let limits = SecurityLimits::default();
            let pdf_bytes = read_bounded_input(in_path, limits.max_input_bytes)?;

            let (manifest, is_valid) = if let Some(sig) = sig_arg {
                if let Ok(manifest_content) = fs::read_to_string(&sig) {
                    let parsed: AuditManifest = serde_json::from_str(&manifest_content)
                        .map_err(|e| format!("failed parsing manifest JSON file '{sig}': {e}"))?;
                    let valid = verify_manifest(&pdf_bytes, &parsed).map_err(|e| e.to_string())?;
                    (Some(parsed), valid)
                } else if let Ok(parsed) = serde_json::from_str::<AuditManifest>(&sig) {
                    let valid = verify_manifest(&pdf_bytes, &parsed).map_err(|e| e.to_string())?;
                    (Some(parsed), valid)
                } else if let Some(sig_bytes) = decode_hex_exact::<64>(&sig) {
                    let valid = verify_pdf(&pdf_bytes, &pub_bytes, &sig_bytes)
                        .map_err(|e| e.to_string())?;
                    (None, valid)
                } else {
                    return Err("unrecognised --sig format: expected manifest JSON file, inline JSON, or 128-hex signature".to_string());
                }
            } else {
                let extracted = extract_audit_manifest(&pdf_bytes).ok_or_else(|| {
                    "no embedded %MsfAuditManifest found in PDF; provide --sig <manifest.json>"
                        .to_string()
                })?;
                let valid = verify_manifest(&pdf_bytes, &extracted).map_err(|e| e.to_string())?;
                (Some(extracted), valid)
            };

            let base_pdf = strip_embedded_audit(&pdf_bytes);
            let sha256_hex = compute_sha256_hex(base_pdf);

            if !is_valid {
                eprintln!(
                    "VERIFICATION FAILED: Signature does not match document SHA-256 digest ({sha256_hex})"
                );
                return Err("cryptographic signature verification failed".to_string());
            }

            let result_json = serde_json::json!({
                "valid": true,
                "document_sha256": sha256_hex,
                "public_key_hex": raw_pub.trim(),
                "signer_identity": manifest.as_ref().map(|m| m.signer_identity.as_str()).unwrap_or("standalone"),
                "timestamp_iso": manifest.as_ref().map(|m| m.timestamp_iso.as_str()).unwrap_or(""),
                "tamper_evident": true
            });
            println!("{}", serde_json::to_string_pretty(&result_json).unwrap());
            Ok(())
        }

        other => Err(format!("unknown subcommand '{other}'")),
    }
}

fn main() -> ExitCode {
    if let Err(err) = run() {
        eprintln!("msfpdf error: {err}");
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
