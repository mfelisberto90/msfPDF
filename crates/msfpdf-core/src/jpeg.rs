use crate::error::PdfError;

pub const MAX_JPEG_BYTES: usize = 262_144; // 256 KiB ceiling per embedded JPEG

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedJpeg {
    pub width_px: u16,
    pub height_px: u16,
    pub color_space: &'static str,
    pub ascii_hex_stream: String,
}

fn decode_base64(input: &str) -> Result<Vec<u8>, PdfError> {
    let stripped = input
        .trim()
        .strip_prefix("data:image/jpeg;base64,")
        .or_else(|| input.trim().strip_prefix("data:image/jpg;base64,"))
        .unwrap_or(input.trim());

    let mut out = Vec::with_capacity((stripped.len() * 3) / 4);
    let mut buf = 0u32;
    let mut bits = 0u8;

    for b in stripped.bytes() {
        if b.is_ascii_whitespace() {
            continue;
        }
        if b == b'=' {
            break;
        }
        let val = match b {
            b'A'..=b'Z' => b - b'A',
            b'a'..=b'z' => b - b'a' + 26,
            b'0'..=b'9' => b - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return Err(PdfError::InvalidImage("invalid base64 byte in JPEG asset")),
        };
        buf = (buf << 6) | (val as u32);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push(((buf >> bits) & 0xFF) as u8);
            if out.len() > MAX_JPEG_BYTES {
                return Err(PdfError::LimitExceeded(
                    "embedded JPEG exceeds 256 KiB byte ceiling",
                ));
            }
        }
    }
    Ok(out)
}

fn encode_ascii_hex_eod(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2 + 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out.push('>'); // PDF ASCIIHexDecode EOD marker
    out
}

/// Validates JPEG magic bytes (`FF D8 ... FF D9`), extracts intrinsic SOF dimensions without full raster decoding,
/// enforces a strict `256 KiB` ceiling, and returns a 7-bit ASCIIHex stream ready for `/DCTDecode`.
pub fn parse_and_validate_jpeg_base64(b64: &str) -> Result<ValidatedJpeg, PdfError> {
    let bytes = decode_base64(b64)?;
    if bytes.len() < 24 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        return Err(PdfError::InvalidImage(
            "missing JPEG SOI magic bytes (FFD8)",
        ));
    }

    let mut pos = 2usize;
    while pos + 4 <= bytes.len() {
        if bytes[pos] != 0xFF {
            pos += 1;
            continue;
        }
        while pos < bytes.len() && bytes[pos] == 0xFF {
            pos += 1;
        }
        if pos >= bytes.len() {
            break;
        }
        let marker = bytes[pos];
        pos += 1;

        if marker == 0xD9 || marker == 0xDA {
            break;
        }
        if marker == 0x00 || (0xD0..=0xD7).contains(&marker) || marker == 0x01 {
            continue;
        }
        if pos + 2 > bytes.len() {
            return Err(PdfError::InvalidImage("truncated JPEG segment header"));
        }
        let seg_len = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]) as usize;
        if seg_len < 2 || pos + seg_len > bytes.len() {
            return Err(PdfError::InvalidImage("malformed JPEG segment length"));
        }

        // Baseline (SOF0 = 0xC0), Extended Sequential (SOF1 = 0xC1), Progressive (SOF2 = 0xC2)
        if matches!(marker, 0xC0..=0xC2) {
            if seg_len < 8 {
                return Err(PdfError::InvalidImage("truncated JPEG SOF frame"));
            }
            let precision = bytes[pos + 2];
            if precision != 8 {
                return Err(PdfError::InvalidImage(
                    "only 8-bit precision JPEGs are supported",
                ));
            }
            let height_px = u16::from_be_bytes([bytes[pos + 3], bytes[pos + 4]]);
            let width_px = u16::from_be_bytes([bytes[pos + 5], bytes[pos + 6]]);
            let components = bytes[pos + 7];
            if width_px == 0 || height_px == 0 || width_px > 4096 || height_px > 4096 {
                return Err(PdfError::InvalidImage("JPEG dimensions out of safe bounds"));
            }
            let color_space = match components {
                1 => "/DeviceGray",
                3 => "/DeviceRGB",
                _ => {
                    return Err(PdfError::InvalidImage(
                        "only Grayscale (1ch) and RGB (3ch) JPEGs are permitted",
                    ))
                }
            };
            return Ok(ValidatedJpeg {
                width_px,
                height_px,
                color_space,
                ascii_hex_stream: encode_ascii_hex_eod(&bytes),
            });
        }

        pos += seg_len;
    }

    Err(PdfError::InvalidImage(
        "no valid SOF0/SOF2 frame header found in JPEG",
    ))
}
