//! High-performance cryptographic signing and verification (Ed25519 + SHA-256)
//! for msfPDF vector documents. Enables tamper-evident, PKI-free commercial auditability.

use ed25519_compact::{KeyPair, PublicKey, Seed, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::PdfError;

pub const AUDIT_MARKER_PREFIX: &str = "\n%MsfAuditManifest: ";

/// Cryptographic audit manifest binding a `%PDF-1.4` document to an Ed25519 identity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditManifest {
    pub algorithm: String,
    pub document_sha256: String,
    pub signer_identity: String,
    pub public_key_hex: String,
    pub signature_hex: String,
    pub timestamp_iso: String,
}

/// Computes the uppercase 64-character hexadecimal SHA-256 digest of a byte slice.
pub fn compute_sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(64);
    for b in digest {
        hex.push_str(&format!("{b:02x}"));
    }
    hex
}

/// Generates a fresh random Ed25519 keypair `(seed_32, public_key_32)`.
pub fn generate_keypair() -> Result<([u8; 32], [u8; 32]), PdfError> {
    let mut seed = [0u8; 32];
    getrandom::getrandom(&mut seed)
        .map_err(|_| PdfError::VaultSecurityViolation("OS CSPRNG entropy failure"))?;
    let kp = KeyPair::from_seed(Seed::new(seed));
    let pub_bytes = *kp.pk;
    Ok((seed, pub_bytes))
}

/// Strips an appended `%MsfAuditManifest: ...` trailer comment to recover the exact base PDF bytes.
pub fn strip_embedded_audit(pdf_bytes: &[u8]) -> &[u8] {
    let marker = AUDIT_MARKER_PREFIX.as_bytes();
    if let Some(pos) = pdf_bytes.windows(marker.len()).rposition(|w| w == marker) {
        &pdf_bytes[..pos]
    } else {
        pdf_bytes
    }
}

/// Signs a PDF buffer with an Ed25519 private key seed, producing a standalone [`AuditManifest`].
pub fn sign_pdf(
    pdf_bytes: &[u8],
    private_key_seed: &[u8; 32],
    signer_identity: &str,
    timestamp_iso: Option<&str>,
) -> Result<AuditManifest, PdfError> {
    let base_pdf = strip_embedded_audit(pdf_bytes);
    let doc_hash_hex = compute_sha256_hex(base_pdf);

    let keypair = KeyPair::from_seed(Seed::new(*private_key_seed));
    let pub_key_bytes = *keypair.pk;

    // Sign the 64-character document SHA-256 hexadecimal string
    let sig = keypair.sk.sign(doc_hash_hex.as_bytes(), None);

    let mut pub_hex = String::with_capacity(64);
    for b in pub_key_bytes {
        pub_hex.push_str(&format!("{b:02x}"));
    }

    let mut sig_hex = String::with_capacity(128);
    for b in sig.iter() {
        sig_hex.push_str(&format!("{b:02x}"));
    }

    let ts = timestamp_iso
        .map(|s| s.to_string())
        .unwrap_or_else(|| "2026-11-03T00:00:00Z".to_string());

    Ok(AuditManifest {
        algorithm: "Ed25519".to_string(),
        document_sha256: doc_hash_hex,
        signer_identity: signer_identity.to_string(),
        public_key_hex: pub_hex,
        signature_hex: sig_hex,
        timestamp_iso: ts,
    })
}

/// Verifies that a given signature over the base PDF matches the Ed25519 public key.
pub fn verify_pdf(
    pdf_bytes: &[u8],
    public_key_bytes: &[u8; 32],
    signature_bytes: &[u8; 64],
) -> Result<bool, PdfError> {
    let base_pdf = strip_embedded_audit(pdf_bytes);
    let doc_hash_hex = compute_sha256_hex(base_pdf);

    let pk = PublicKey::from_slice(public_key_bytes)
        .map_err(|e| PdfError::InvalidSpec(format!("invalid Ed25519 public key: {e}")))?;
    let sig = Signature::from_slice(signature_bytes)
        .map_err(|e| PdfError::InvalidSpec(format!("invalid Ed25519 signature: {e}")))?;

    Ok(pk.verify(doc_hash_hex.as_bytes(), &sig).is_ok())
}

/// Verifies an [`AuditManifest`] against a PDF byte buffer.
pub fn verify_manifest(pdf_bytes: &[u8], manifest: &AuditManifest) -> Result<bool, PdfError> {
    if manifest.algorithm != "Ed25519" {
        return Ok(false);
    }

    let base_pdf = strip_embedded_audit(pdf_bytes);
    let actual_hash_hex = compute_sha256_hex(base_pdf);
    if actual_hash_hex != manifest.document_sha256 {
        return Ok(false);
    }

    let pub_bytes = decode_hex_exact::<32>(&manifest.public_key_hex)
        .ok_or_else(|| PdfError::InvalidSpec("invalid public_key_hex length or hex".into()))?;
    let sig_bytes = decode_hex_exact::<64>(&manifest.signature_hex)
        .ok_or_else(|| PdfError::InvalidSpec("invalid signature_hex length or hex".into()))?;

    verify_pdf(base_pdf, &pub_bytes, &sig_bytes)
}

/// Embeds an [`AuditManifest`] into the PDF buffer as a trailing comment block.
pub fn embed_audit_manifest(pdf_bytes: &[u8], manifest: &AuditManifest) -> Vec<u8> {
    let base_pdf = strip_embedded_audit(pdf_bytes);
    let json_bytes = serde_json::to_vec(manifest).unwrap_or_default();
    let mut out = Vec::with_capacity(base_pdf.len() + json_bytes.len() + 32);
    out.extend_from_slice(base_pdf);
    out.extend_from_slice(AUDIT_MARKER_PREFIX.as_bytes());
    out.extend_from_slice(&json_bytes);
    out.push(b'\n');
    out
}

/// Extracts an embedded [`AuditManifest`] from a PDF buffer if present.
pub fn extract_audit_manifest(pdf_bytes: &[u8]) -> Option<AuditManifest> {
    let marker = AUDIT_MARKER_PREFIX.as_bytes();
    let pos = pdf_bytes.windows(marker.len()).rposition(|w| w == marker)?;
    let manifest_bytes = &pdf_bytes[pos + marker.len()..];
    let end_pos = manifest_bytes
        .iter()
        .position(|&b| b == b'\n')
        .unwrap_or(manifest_bytes.len());
    serde_json::from_slice(&manifest_bytes[..end_pos]).ok()
}

/// Encodes a byte slice as a lowercase hexadecimal string.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{b:02x}");
    }
    s
}

/// Decodes an exact N-byte hexadecimal string into `[u8; N]`.
pub fn decode_hex_exact<const N: usize>(hex: &str) -> Option<[u8; N]> {
    let trimmed = hex.trim();
    if trimmed.len() != N * 2 {
        return None;
    }
    let mut out = [0u8; N];
    for i in 0..N {
        out[i] = u8::from_str_radix(&trimmed[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
