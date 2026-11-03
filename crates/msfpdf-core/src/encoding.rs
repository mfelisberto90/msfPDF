use std::borrow::Cow;

use crate::geometry::FontFace;
use crate::glyphs_data::{lookup_vector_glyph, VECTOR_UPM};
use crate::shaping::{has_arabic_or_rtl, shape_arabic_logical};

/// Maps CP1252 printable glyphs in 0x80..=0x9F to their 7-bit PDF octal escapes.
fn cp1252_special_octal(ch: char) -> Option<&'static str> {
    match ch {
        '€' => Some("\\200"),
        '‚' => Some("\\202"),
        'ƒ' => Some("\\203"),
        '„' => Some("\\204"),
        '…' => Some("\\205"),
        '†' => Some("\\206"),
        '‡' => Some("\\207"),
        'ˆ' => Some("\\210"),
        '‰' => Some("\\211"),
        'Š' => Some("\\212"),
        '‹' => Some("\\213"),
        'Œ' => Some("\\214"),
        'Ž' => Some("\\216"),
        '‘' => Some("\\221"),
        '’' => Some("\\222"),
        '“' => Some("\\223"),
        '”' => Some("\\224"),
        '•' => Some("\\225"),
        '–' => Some("\\226"),
        '—' => Some("\\227"),
        '˜' => Some("\\230"),
        '™' => Some("\\231"),
        'š' => Some("\\232"),
        '›' => Some("\\233"),
        'œ' => Some("\\234"),
        'ž' => Some("\\236"),
        'Ÿ' => Some("\\237"),
        _ => None,
    }
}

fn fold_unicode_fallback(ch: char) -> &'static str {
    match ch {
        'Ā' | 'Ă' | 'Ą' => "A",
        'ā' | 'ă' | 'ą' => "a",
        'Ć' | 'Ĉ' | 'Ċ' | 'Č' => "C",
        'ć' | 'ĉ' | 'ċ' | 'č' => "c",
        'Ď' | 'Đ' => "D",
        'ď' | 'đ' => "d",
        'Ē' | 'Ĕ' | 'Ė' | 'Ę' | 'Ě' => "E",
        'ē' | 'ĕ' | 'ė' | 'ę' | 'ě' => "e",
        'Ĝ' | 'Ğ' | 'Ġ' | 'Ģ' => "G",
        'ĝ' | 'ğ' | 'ġ' | 'ģ' => "g",
        'Ī' | 'Ĭ' | 'Į' | 'İ' => "I",
        'ī' | 'ĭ' | 'į' | 'ı' => "i",
        'Ł' => "L",
        'ł' => "l",
        'Ń' | 'Ņ' | 'Ň' => "N",
        'ń' | 'ņ' | 'ň' => "n",
        'Ō' | 'Ŏ' | 'Ő' => "O",
        'ō' | 'ŏ' | 'ő' => "o",
        'Ŕ' | 'Ŗ' | 'Ř' => "R",
        'ŕ' | 'ŗ' | 'ř' => "r",
        'Ś' | 'Ŝ' | 'Ş' => "S",
        'ś' | 'ŝ' | 'ş' => "s",
        'Ţ' | 'Ť' => "T",
        'ţ' | 'ť' => "t",
        'Ū' | 'Ŭ' | 'Ů' | 'Ű' | 'Ų' => "U",
        'ū' | 'ŭ' | 'ů' | 'ű' | 'ų' => "u",
        'Ź' | 'Ż' => "Z",
        'ź' | 'ż' => "z",
        '✓' | '✔' => "[x]",
        '✗' | '✘' => "[ ]",
        '→' => "->",
        '←' => "<-",
        _ => " ",
    }
}

/// Encodes arbitrary UTF-8 strings into 100% 7-bit ASCII PDF string literals using `/WinAnsiEncoding` octal escapes.
///
/// Security guarantees:
/// 1. Neutralizes PDF stream operator injection by escaping `\`, `(`, and `)`.
/// 2. Collapses ASCII control bytes (`0x00..=0x1F`, `0x7F`) into single spaces.
/// 3. Emits strictly 1-byte ASCII characters (`0x20..=0x7E`), ensuring exact byte-offset determinism in `xref` and `/Length`.
pub fn pdf_escape_win_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '(' => out.push_str("\\("),
            ')' => out.push_str("\\)"),
            '\r' | '\n' | '\t' => out.push(' '),
            c if ('\x00'..='\x1F').contains(&c) || c == '\x7F' => out.push(' '),
            c if ('\x20'..='\x7E').contains(&c) => out.push(c),
            c => {
                if let Some(oct) = cp1252_special_octal(c) {
                    out.push_str(oct);
                } else if ('\u{00A0}'..='\u{00FF}').contains(&c) {
                    let code = c as u32;
                    let d2 = ((code >> 6) & 7) as u8 + b'0';
                    let d1 = ((code >> 3) & 7) as u8 + b'0';
                    let d0 = (code & 7) as u8 + b'0';
                    out.push('\\');
                    out.push(d2 as char);
                    out.push(d1 as char);
                    out.push(d0 as char);
                } else {
                    out.push_str(fold_unicode_fallback(c));
                }
            }
        }
    }
    out
}

#[inline]
fn afm_win_ansi_unit(ch: char) -> f64 {
    match ch {
        ' ' | '.' | ',' | ':' | ';' | '!' | '|' | '/' | '\'' | '‘' | '’' | 'i' | 'l' | 'I'
        | 'í' | 'ì' | 'î' | 'ï' => 0.278,
        'j' => 0.222,
        'f' | 'r' | 't' | '(' | ')' | '[' | ']' | '-' | '–' => 0.333,
        '•' => 0.350,
        '"' | '“' | '”' => 0.400,
        's' | 'z' | 'c' | 'ç' | 'J' => 0.500,
        'L' | '1' => 0.556,
        'm' | 'w' => 0.833,
        'M' | 'W' | '%' | '‰' => 0.889,
        '@' | '—' | 'Œ' | 'œ' => 0.980,
        'G' | 'O' | 'Q' | 'Ó' | 'Ô' | 'Õ' | 'Ò' => 0.778,
        'C' | 'D' | 'H' | 'N' | 'R' | 'U' | 'Ç' | 'Ñ' | 'Ú' => 0.722,
        c if c.is_ascii_uppercase() || ('\u{00C0}'..='\u{00DD}').contains(&c) => 0.667,
        _ => 0.556,
    }
}

/// Estimates glyph advance width in PDF points (`1/72 inch`) using Adobe Font Metrics (AFM) proportions
/// for Standard Type 1 `Helvetica`/`Courier` and exact `2048-UPM` `hmtx` advance widths for Cyrillic & shaped Arabic glyphs.
pub fn estimate_font_width_pt(text: &str, font: FontFace, size_pt: f64) -> f64 {
    let effective: Cow<'_, str> = if has_arabic_or_rtl(text) {
        Cow::Owned(shape_arabic_logical(text))
    } else {
        Cow::Borrowed(text)
    };

    let mut win_ansi_units = 0.0;
    let mut win_ansi_count = 0usize;
    let mut vector_funits: u64 = 0;

    for ch in effective.chars() {
        if let Some(rec) = lookup_vector_glyph(ch as u32) {
            let adv = if font == FontFace::Bold {
                rec.bold_adv
            } else {
                rec.regular_adv
            };
            vector_funits += adv as u64;
        } else {
            win_ansi_count += 1;
            if font != FontFace::Mono {
                win_ansi_units += afm_win_ansi_unit(ch);
            }
        }
    }

    let win_pt = if font == FontFace::Mono {
        (win_ansi_count as f64) * 0.600 * size_pt
    } else {
        let bold_scale = if font == FontFace::Bold { 1.045 } else { 1.0 };
        win_ansi_units * size_pt * bold_scale
    };

    if vector_funits == 0 {
        win_pt
    } else {
        win_pt + ((vector_funits as f64) / VECTOR_UPM) * size_pt
    }
}

/// Truncates text cleanly with a single CP1252 ellipsis (`…`) when its rendered width exceeds `max_width_pt`.
pub fn truncate_to_width_pt(text: &str, font: FontFace, size_pt: f64, max_width_pt: f64) -> String {
    let clean = text.trim();
    if estimate_font_width_pt(clean, font, size_pt) <= max_width_pt {
        return clean.to_string();
    }
    let ellipsis_w = estimate_font_width_pt("…", font, size_pt);
    let target = (max_width_pt - ellipsis_w).max(size_pt);
    let mut acc = String::new();
    for ch in clean.chars() {
        let mut candidate = acc.clone();
        candidate.push(ch);
        if estimate_font_width_pt(&candidate, font, size_pt) > target {
            break;
        }
        acc.push(ch);
    }
    format!("{}…", acc.trim_end())
}

/// Word-wraps multi-paragraph UTF-8 text to fit inside `max_width_pt` without mid-word clipping,
/// falling back to character-level splitting only for unbroken tokens wider than an entire line.
pub fn wrap_text_to_width_pt(
    text: &str,
    font: FontFace,
    size_pt: f64,
    max_width_pt: f64,
) -> Vec<String> {
    let bound = max_width_pt.max(size_pt * 3.0);
    let space_w = estimate_font_width_pt(" ", font, size_pt);
    let mut lines = Vec::new();

    for raw_para in text.split('\n') {
        let trimmed = raw_para.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut current = String::new();
        let mut current_w = 0.0;

        for word in trimmed.split_whitespace() {
            let word_w = estimate_font_width_pt(word, font, size_pt);
            if word_w > bound {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                let mut chunk = String::new();
                let mut chunk_w = 0.0;
                for ch in word.chars() {
                    let mut buf = [0u8; 4];
                    let ch_w = estimate_font_width_pt(ch.encode_utf8(&mut buf), font, size_pt);
                    if !chunk.is_empty() && chunk_w + ch_w > bound {
                        lines.push(std::mem::take(&mut chunk));
                        chunk_w = 0.0;
                    }
                    chunk.push(ch);
                    chunk_w += ch_w;
                }
                current = chunk;
                current_w = chunk_w;
                continue;
            }

            if current.is_empty() {
                current.push_str(word);
                current_w = word_w;
            } else if current_w + space_w + word_w <= bound {
                current.push(' ');
                current.push_str(word);
                current_w += space_w + word_w;
            } else {
                lines.push(std::mem::take(&mut current));
                current.push_str(word);
                current_w = word_w;
            }
        }

        if !current.is_empty() {
            lines.push(current);
        }
    }

    lines
}
