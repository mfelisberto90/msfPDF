use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::PdfError;
use crate::renderer::render_document;
use crate::spec::{DocumentSpec, SecurityLimits};

static TMP_SEQ: AtomicU64 = AtomicU64::new(1);

/// Returns unused heap arenas back to the Linux kernel after batch PDF bursts.
pub fn maybe_trim_heap() {
    #[cfg(target_os = "linux")]
    unsafe {
        let _ = libc::malloc_trim(0);
    }
}

fn is_valid_tenant_id(tenant_id: &str) -> bool {
    let len = tenant_id.len();
    (3..=64).contains(&len)
        && !tenant_id.starts_with('-')
        && tenant_id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn is_valid_prefix(prefix: &str) -> bool {
    let len = prefix.len();
    (2..=24).contains(&len)
        && prefix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn is_valid_token_hex(token: &str) -> bool {
    token.len() == 40
        && token
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn encode_hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

fn generate_token_160bit() -> Result<String, PdfError> {
    let mut raw = [0u8; 20];
    getrandom::getrandom(&mut raw)
        .map_err(|_| PdfError::IoError("OS CSPRNG entropy unavailable".to_string()))?;
    Ok(encode_hex_lower(&raw))
}

fn atomic_write_0600(target_path: &Path, bytes: &[u8]) -> Result<(), PdfError> {
    let parent = target_path
        .parent()
        .ok_or(PdfError::VaultSecurityViolation("missing tenant directory"))?;
    fs::create_dir_all(parent).map_err(|e| PdfError::IoError(format!("mkdir failed: {e}")))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
    }

    let seq = TMP_SEQ.fetch_add(1, Ordering::Relaxed);
    let file_name = target_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document.pdf");
    let tmp_path = parent.join(format!(".{file_name}.{}.{seq}.tmp", std::process::id()));

    let write_res = (|| -> Result<(), PdfError> {
        let mut opts = OpenOptions::new();
        opts.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut file = opts
            .open(&tmp_path)
            .map_err(|e| PdfError::IoError(format!("open tmp failed: {e}")))?;
        file.write_all(bytes)
            .map_err(|e| PdfError::IoError(format!("write tmp failed: {e}")))?;
        file.sync_all()
            .map_err(|e| PdfError::IoError(format!("fsync tmp failed: {e}")))?;
        drop(file);
        fs::rename(&tmp_path, target_path)
            .map_err(|e| PdfError::IoError(format!("atomic rename failed: {e}")))?;
        Ok(())
    })();

    if write_res.is_err() {
        let _ = fs::remove_file(&tmp_path);
    }
    write_res
}

#[derive(Debug, Clone)]
pub struct IssuedVaultDocument {
    pub token: String,
    pub file_name: String,
    pub bytes: Vec<u8>,
}

/// Fail-closed multi-tenant PDF storage vault enforcing per-tenant `0700` directory boundaries,
/// `160-bit` CSPRNG non-enumerable tokens, and `0600` atomic `.tmp` + `fsync` + `rename` writes.
#[derive(Debug, Clone)]
pub struct TenantPdfVault {
    vault_root: PathBuf,
    doc_prefix: String,
    limits: SecurityLimits,
}

impl TenantPdfVault {
    pub fn new(vault_root: impl AsRef<Path>, doc_prefix: &str) -> Result<Self, PdfError> {
        if !is_valid_prefix(doc_prefix) {
            return Err(PdfError::VaultSecurityViolation(
                "doc_prefix must match ^[a-z0-9-]{2,24}$",
            ));
        }
        let p = vault_root.as_ref();
        let abs = if p.is_absolute() {
            p.to_path_buf()
        } else {
            std::env::current_dir()
                .map_err(|e| PdfError::IoError(format!("cwd unavailable: {e}")))?
                .join(p)
        };
        Ok(Self {
            vault_root: abs,
            doc_prefix: doc_prefix.to_string(),
            limits: SecurityLimits::default(),
        })
    }

    pub fn with_limits(mut self, limits: SecurityLimits) -> Self {
        self.limits = limits;
        self
    }

    fn resolve_safe_path(
        &self,
        tenant_id: &str,
        token: &str,
    ) -> Result<(String, PathBuf), PdfError> {
        if !is_valid_tenant_id(tenant_id) {
            return Err(PdfError::VaultSecurityViolation("invalid tenant_id format"));
        }
        if !is_valid_token_hex(token) {
            return Err(PdfError::VaultSecurityViolation(
                "invalid or enumerable 160-bit token rejected",
            ));
        }

        let file_name = format!("{}-{token}.pdf", self.doc_prefix);
        let tenant_dir = self.vault_root.join(tenant_id);
        let full = tenant_dir.join(&file_name);

        if !tenant_dir.starts_with(&self.vault_root) || !full.starts_with(&tenant_dir) {
            return Err(PdfError::VaultSecurityViolation(
                "vault directory boundary violation",
            ));
        }
        Ok((file_name, full))
    }

    pub fn issue_document(
        &self,
        tenant_id: &str,
        spec: &DocumentSpec,
    ) -> Result<IssuedVaultDocument, PdfError> {
        let token = generate_token_160bit()?;
        let (file_name, full_path) = self.resolve_safe_path(tenant_id, &token)?;
        let bytes = render_document(spec, self.limits)?;
        atomic_write_0600(&full_path, &bytes)?;
        maybe_trim_heap();
        Ok(IssuedVaultDocument {
            token,
            file_name,
            bytes,
        })
    }

    pub fn read_document(&self, tenant_id: &str, token: &str) -> Result<Vec<u8>, PdfError> {
        let (_name, full_path) = self.resolve_safe_path(tenant_id, token)?;
        fs::read(&full_path)
            .map_err(|_| PdfError::VaultSecurityViolation("document not found in tenant scope"))
    }
}
