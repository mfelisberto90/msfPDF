pub mod canvas;
pub mod crypto;
pub mod encoding;
pub mod error;
pub mod geometry;
pub mod glyphs_data;
pub mod jpeg;
pub mod money;
pub mod qr;
pub mod renderer;
pub mod shaping;
pub mod spec;
pub mod vault;

pub use crypto::{
    compute_sha256_hex, decode_hex_exact, embed_audit_manifest, extract_audit_manifest,
    generate_keypair, hex_encode, sign_pdf, strip_embedded_audit, verify_manifest, verify_pdf,
    AuditManifest,
};

pub use encoding::{
    estimate_font_width_pt, pdf_escape_win_ansi, truncate_to_width_pt, wrap_text_to_width_pt,
};
pub use error::PdfError;
pub use geometry::{FontFace, PageFormat, Rgb, ThemePalette, ThemePreset};
pub use money::{format_money_minor, Currency};
pub use qr::{encode_qr, QrMatrix};
pub use renderer::{render_document, render_document_json};
pub use shaping::{
    has_arabic_or_rtl, is_rtl_dominant, shape_and_reorder_line, shape_arabic_logical,
};
pub use spec::{
    BannerTone, BentoCardItem, ContinuationHeaderSpec, DocumentBlock, DocumentMetadata,
    DocumentSpec, FinancialLineItem, KeyValueRow, SecurityLimits, SignerSpec, TableColumnSpec,
    TextAlign,
};
pub use vault::{maybe_trim_heap, IssuedVaultDocument, TenantPdfVault};

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_valid_xref_and_7bit_ascii(pdf: &[u8]) {
        assert!(pdf.starts_with(b"%PDF-1.4\n"), "missing %PDF-1.4 header");
        assert!(pdf.ends_with(b"%%EOF\n"), "missing %%EOF trailer");
        assert!(
            pdf.iter()
                .all(|&b| b == b'\n' || (0x20..=0x7E).contains(&b)),
            "stream must remain pure 7-bit ASCII so multi-byte UTF-8 never drifts xref offsets"
        );

        let text = std::str::from_utf8(pdf).expect("valid ASCII");
        let xref_pos = text.rfind("\nxref\n").expect("xref table present") + 1;
        let startxref_marker = text.rfind("startxref\n").expect("startxref present");
        let declared_xref: usize = text[startxref_marker + "startxref\n".len()..]
            .lines()
            .next()
            .unwrap()
            .trim()
            .parse()
            .expect("numeric startxref");
        assert_eq!(
            xref_pos, declared_xref,
            "startxref must match exact byte offset of xref"
        );

        let xref_lines: Vec<&str> = text[xref_pos..startxref_marker].lines().collect();
        // Skip "xref", "0 N", and free entry "0000000000 65535 f "
        for (idx, entry) in xref_lines.iter().skip(3).enumerate() {
            if entry.starts_with("trailer") {
                break;
            }
            let offset: usize = entry[0..10].parse().expect("10-digit xref offset");
            let obj_id = idx + 1;
            let expected_prefix = format!("{obj_id} 0 obj\n");
            assert!(
                text[offset..].starts_with(&expected_prefix),
                "xref entry {obj_id} at byte {offset} did not point to `{expected_prefix}`"
            );
        }
    }

    #[test]
    fn encodes_cp1252_currencies_accents_and_blocks_stream_injection() {
        let raw = "Client ) Tj ET BT /F2 48 Tf (PWNED — £1,250.00 • €99.00 • São Paulo m² nº 1ª";
        let escaped = pdf_escape_win_ansi(raw);
        assert!(escaped.contains("Client \\) Tj ET BT /F2 48 Tf \\(PWNED"));
        assert!(escaped.contains("\\227")); // em-dash
        assert!(escaped.contains("\\2431,250.00")); // £
        assert!(escaped.contains("\\20099.00")); // €
        assert!(escaped.contains("S\\343o Paulo")); // ã
        assert!(escaped.contains("m\\262")); // ²
        assert!(escaped.contains("n\\272")); // º
        assert!(escaped.contains("1\\252")); // ª
        assert!(escaped.bytes().all(|b| (0x20..=0x7E).contains(&b)));
    }

    #[test]
    fn formats_integer_minor_units_across_iso_currencies_without_floats() {
        assert_eq!(format_money_minor(125_050, Currency::GBP), "£1,250.50");
        assert_eq!(format_money_minor(-4_999, Currency::GBP), "-£49.99");
        assert_eq!(
            format_money_minor(20_000_000, Currency::BRL),
            "R$ 200.000,00"
        );
        assert_eq!(format_money_minor(98_765, Currency::EUR), "€987.65");
        assert_eq!(format_money_minor(100_000, Currency::USD), "$1,000.00");
        assert_eq!(format_money_minor(15_800, Currency::JPY), "¥15,800");
        assert_eq!(
            format_money_minor(45_000_050, Currency::RUB),
            "450 000,50 ₽"
        );
        assert_eq!(
            format_money_minor(1_942_500, Currency::AED),
            "AED 19,425.00"
        );
        assert_eq!(format_money_minor(750_000, Currency::SAR), "SAR 7,500.00");
    }

    #[test]
    fn shapes_arabic_ligatures_and_renders_russian_and_arabic_in_7bit_ascii_type3() {
        // Verify Lam-Alef ligature + contextual joining + BiDi number preservation
        let logical = "فاتورة للأنظمة رقم 8841";
        let shaped = shape_arabic_logical(logical);
        assert!(
            shaped.contains('\u{FEF8}'),
            "ل + أ after joining ل must form final Lam-Alef-Hamza ligature U+FEF8"
        );
        let visual = shape_and_reorder_line(logical);
        assert!(
            visual.starts_with("8841 "),
            "RTL visual line ending logically with 8841 must start visually on the left with LTR '8841 '"
        );

        let raw_json = include_bytes!("../../../examples/global-multilingual-invoice.json");
        let pdf_bytes =
            render_document_json(raw_json, SecurityLimits::default()).expect("renders trilingual");
        assert_valid_xref_and_7bit_ascii(&pdf_bytes);
        let text = std::str::from_utf8(&pdf_bytes).unwrap();
        assert!(
            text.contains("/Subtype /Type3"),
            "multilingual document must emit dynamic Type 3 vector glyph subsets"
        );
        assert!(
            text.contains("/ToUnicode"),
            "multilingual Type 3 subsets must include /ToUnicode CMap for searchable UTF-8 extraction"
        );
    }

    #[test]
    fn renders_multipage_editorial_spec_with_exact_xref_and_vector_qr() {
        let clauses: Vec<String> = (1..=26)
            .map(|n| {
                format!(
                    "Clause {n} — All commercial obligations, data processing schedules, and service credits remain governed by Scots law and ISO/IEC 27001 controls (£0.00 float drift)."
                )
            })
            .collect();

        let spec = DocumentSpec {
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
            continuation_header: Some(ContinuationHeaderSpec {
                title: "Caledonia Cloud Systems Ltd".into(),
                subtitle: "ENTERPRISE SERVICE SCHEDULE (CONTINUED)".into(),
            }),
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
                    ],
                    zebra: true,
                },
                DocumentBlock::FinancialSummary {
                    currency: Currency::GBP,
                    rows: vec![
                        FinancialLineItem {
                            label: "Subtotal (Excl. UK VAT)".into(),
                            amount_minor: Some(365_000),
                            currency: None,
                            text_value: None,
                            emphasize: false,
                        },
                        FinancialLineItem {
                            label: "UK Standard VAT (20.0%)".into(),
                            amount_minor: Some(73_000),
                            currency: None,
                            text_value: None,
                            emphasize: false,
                        },
                    ],
                },
                DocumentBlock::CalloutBanner {
                    title: "TOTAL AMOUNT DUE (GBP)".into(),
                    subtitle: "Payable via Faster Payments / CHAPS within 30 calendar days".into(),
                    right_value: Some("£4,380.00".into()),
                    tone: BannerTone::Accent,
                },
                DocumentBlock::QrVerification {
                    payload: "https://verify.example.co.uk/inv/2026-8841-9f8a7b6c".into(),
                    headline: "INSTANT CRYPTOGRAPHIC AUDIT VERIFICATION".into(),
                    details: "Scan this vector QR code to verify document SHA-256 integrity and settlement status.".into(),
                    code_display: Some("SHA256: 9F8A-7B6C-4410-EE29".into()),
                },
                DocumentBlock::SectionHeading {
                    title: "2. STATUTORY TERMS & SLA CLAUSES".into(),
                },
                DocumentBlock::ClauseList {
                    paragraphs: clauses,
                },
                DocumentBlock::SignatureGrid {
                    heading: Some("3. AUTHORISED SIGNATURES".into()),
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
        };

        let bytes = render_document(&spec, SecurityLimits::default()).expect("renders cleanly");
        assert_valid_xref_and_7bit_ascii(&bytes);
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(
            text.contains("/Count 2") || text.contains("/Count 3"),
            "26 clauses must paginate cleanly beyond page 1"
        );
        assert!(text.contains("\\2434,380.00"));
    }

    #[test]
    fn enforces_dos_limits_and_multi_tenant_vault_isolation() {
        let limits = SecurityLimits {
            max_pages: 2,
            ..SecurityLimits::default()
        };

        let huge_clauses: Vec<String> = (0..120)
            .map(|i| {
                format!("Line {i} repeated long clause text to trigger pagination ceiling safely.")
            })
            .collect();

        let spec = DocumentSpec {
            page_format: PageFormat::A4,
            theme: ThemePreset::SlateMinimal,
            accent_hex_override: None,
            metadata: DocumentMetadata {
                issuer_name: "Acme Corp".into(),
                document_code: "DOC-1".into(),
                issued_at_label: "2026-11-02".into(),
                footer_notice: None,
                page_counter_template: "Page {current} of {total}".into(),
            },
            continuation_header: None,
            blocks: vec![DocumentBlock::ClauseList {
                paragraphs: huge_clauses,
            }],
        };

        let err = render_document(&spec, limits).expect_err("must reject page bomb");
        assert!(matches!(err, PdfError::LimitExceeded(_)));

        // Multi-tenant vault isolation & traversal checks
        let tmp_dir =
            std::env::temp_dir().join(format!("msfpdf_vault_test_{}", std::process::id()));
        let vault = TenantPdfVault::new(&tmp_dir, "invoice").expect("valid vault");

        let small_spec = DocumentSpec {
            blocks: vec![DocumentBlock::CalloutBanner {
                title: "PAID".into(),
                subtitle: "Receipt".into(),
                right_value: Some("£10.00".into()),
                tone: BannerTone::Accent,
            }],
            ..spec
        };

        let issued = vault
            .issue_document("tenant_alpha_01", &small_spec)
            .expect("issues tenant doc");
        assert_eq!(issued.token.len(), 40);
        assert_valid_xref_and_7bit_ascii(&issued.bytes);

        // Positive read by owning tenant
        let loaded = vault
            .read_document("tenant_alpha_01", &issued.token)
            .expect("owning tenant reads");
        assert_eq!(loaded, issued.bytes);

        // Negative cross-tenant IDOR read (must fail closed)
        assert!(vault
            .read_document("tenant_beta_02", &issued.token)
            .is_err());

        // Negative path traversal & enumerable token rejections
        assert!(vault.read_document("../etc", &issued.token).is_err());
        assert!(vault.read_document("tenant_alpha_01", "../passwd").is_err());
        assert!(vault.read_document("tenant_alpha_01", "00001").is_err());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn signs_and_verifies_pdf_with_ed25519_and_audit_manifest() {
        let (seed, pubkey) = generate_keypair().expect("generates valid keypair");
        assert_eq!(seed.len(), 32);
        assert_eq!(pubkey.len(), 32);

        let spec = DocumentSpec {
            page_format: PageFormat::A4,
            theme: ThemePreset::NavyFintech,
            accent_hex_override: None,
            metadata: DocumentMetadata {
                issuer_name: "Meridian Clearing Ltd".into(),
                document_code: "INV-ED25519-01".into(),
                issued_at_label: "03 Nov 2026".into(),
                footer_notice: None,
                page_counter_template: "Page {current} of {total}".into(),
            },
            continuation_header: None,
            blocks: vec![
                DocumentBlock::HeroHeader {
                    kicker: "FINTECH AUDIT SPECIMEN".into(),
                    title: "Settlement Invoice".into(),
                    subtitle: "Edinburgh, Scotland, UK".into(),
                    badge_label: Some("AUDITED".into()),
                    badge_code: Some("#9042".into()),
                    badge_meta: None,
                    logo_jpeg_base64: None,
                },
                DocumentBlock::AuditManifest {
                    signer_identity: "Meridian Clearing House Ltd (UK-EDINBURGH-01)".into(),
                    public_key_hex: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
                    algorithm: "Ed25519".into(),
                    document_sha256: "553ef20b2245c7470c591cc93c724bef793b9894e29167cfe4546304f60ea006".into(),
                    signature_hex: "3a8f1234567890abcdef3a8f1234567890abcdef3a8f1234567890abcdef3a8f1234567890abcdef3a8f1234567890abcdef3a8f1234567890abcdef3a8f1234567890abcdef".into(),
                    timestamp_iso: "2026-11-03T12:00:00Z".into(),
                },
            ],
        };

        let pdf = render_document(&spec, SecurityLimits::default()).expect("renders signed spec");
        assert_valid_xref_and_7bit_ascii(&pdf);

        // Sign PDF with real Ed25519 keypair
        let manifest = sign_pdf(
            &pdf,
            &seed,
            "Meridian Clearing House Ltd",
            Some("2026-11-03T12:00:00Z"),
        )
        .expect("signs pdf");

        assert_eq!(manifest.algorithm, "Ed25519");
        assert_eq!(manifest.document_sha256.len(), 64);
        assert_eq!(manifest.public_key_hex.len(), 64);
        assert_eq!(manifest.signature_hex.len(), 128);

        // Verify valid signature
        let is_valid = verify_manifest(&pdf, &manifest).expect("verifies manifest");
        assert!(is_valid, "signature must verify over authentic PDF bytes");

        // Embed manifest in trailer and extract
        let embedded_pdf = embed_audit_manifest(&pdf, &manifest);
        assert!(embedded_pdf.starts_with(b"%PDF-1.4\n"));
        let extracted = extract_audit_manifest(&embedded_pdf).expect("extracts embedded manifest");
        assert_eq!(extracted, manifest);
        let is_embedded_valid =
            verify_manifest(&embedded_pdf, &extracted).expect("verifies embedded manifest");
        assert!(is_embedded_valid, "embedded manifest must verify base PDF");

        // Tamper test: flip a single byte in PDF stream, must fail verification
        let mut tampered_pdf = embedded_pdf.clone();
        tampered_pdf[120] ^= 0x01;
        let is_tampered_valid =
            verify_manifest(&tampered_pdf, &manifest).expect("verifies tampered");
        assert!(!is_tampered_valid, "tampered PDF must fail signature check");
    }
}
