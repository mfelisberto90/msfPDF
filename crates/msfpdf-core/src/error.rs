use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdfError {
    LimitExceeded(&'static str),
    InvalidSpec(String),
    InvalidImage(&'static str),
    QrCapacityExceeded(usize),
    VaultSecurityViolation(&'static str),
    IoError(String),
}

impl fmt::Display for PdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LimitExceeded(msg) => write!(f, "security limit exceeded: {msg}"),
            Self::InvalidSpec(msg) => write!(f, "invalid document specification: {msg}"),
            Self::InvalidImage(msg) => write!(f, "invalid embedded JPEG asset: {msg}"),
            Self::QrCapacityExceeded(len) => {
                write!(
                    f,
                    "QR payload length ({len} bytes) exceeds version 6 capacity"
                )
            }
            Self::VaultSecurityViolation(msg) => write!(f, "vault security violation: {msg}"),
            Self::IoError(msg) => write!(f, "vault I/O failure: {msg}"),
        }
    }
}

impl std::error::Error for PdfError {}
