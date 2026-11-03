//! Zero-dependency Arabic contextual glyph shaping (Isolated/Final/Initial/Medial + Lam-Alef ligatures)
//! and UAX #9 bidirectional (BiDi) visual line reordering (`levels 0..=2`).

/// Returns `(joins_left, [isolated, final_form, initial, medial])` for joining Arabic letters.
/// Every letter returned here joins on the right (`joins_right = true`).
fn arabic_joining_forms(cp: u32) -> Option<(bool, [u32; 4])> {
    match cp {
        // Right-only joiners (joins_left = false)
        0x0622 => Some((false, [0xFE81, 0xFE82, 0xFE81, 0xFE82])), // آ
        0x0623 => Some((false, [0xFE83, 0xFE84, 0xFE83, 0xFE84])), // أ
        0x0624 => Some((false, [0xFE85, 0xFE86, 0xFE85, 0xFE86])), // ؤ
        0x0625 => Some((false, [0xFE87, 0xFE88, 0xFE87, 0xFE88])), // إ
        0x0627 => Some((false, [0xFE8D, 0xFE8E, 0xFE8D, 0xFE8E])), // ا
        0x0629 => Some((false, [0xFE93, 0xFE94, 0xFE93, 0xFE94])), // ة
        0x062F => Some((false, [0xFEA9, 0xFEAA, 0xFEA9, 0xFEAA])), // د
        0x0630 => Some((false, [0xFEAB, 0xFEAC, 0xFEAB, 0xFEAC])), // ذ
        0x0631 => Some((false, [0xFEAD, 0xFEAE, 0xFEAD, 0xFEAE])), // ر
        0x0632 => Some((false, [0xFEAF, 0xFEB0, 0xFEAF, 0xFEB0])), // ز
        0x0648 => Some((false, [0xFEED, 0xFEEE, 0xFEED, 0xFEEE])), // و
        0x0649 => Some((false, [0xFEEF, 0xFEF0, 0xFEEF, 0xFEF0])), // ى
        0x0698 => Some((false, [0xFB8A, 0xFB8B, 0xFB8A, 0xFB8B])), // ژ
        // Dual joiners (joins_left = true)
        0x0626 => Some((true, [0xFE89, 0xFE8A, 0xFE8B, 0xFE8C])), // ئ
        0x0628 => Some((true, [0xFE8F, 0xFE90, 0xFE91, 0xFE92])), // ب
        0x062A => Some((true, [0xFE95, 0xFE96, 0xFE97, 0xFE98])), // ت
        0x062B => Some((true, [0xFE99, 0xFE9A, 0xFE9B, 0xFE9C])), // ث
        0x062C => Some((true, [0xFE9D, 0xFE9E, 0xFE9F, 0xFEA0])), // ج
        0x062D => Some((true, [0xFEA1, 0xFEA2, 0xFEA3, 0xFEA4])), // ح
        0x062E => Some((true, [0xFEA5, 0xFEA6, 0xFEA7, 0xFEA8])), // خ
        0x0633 => Some((true, [0xFEB1, 0xFEB2, 0xFEB3, 0xFEB4])), // س
        0x0634 => Some((true, [0xFEB5, 0xFEB6, 0xFEB7, 0xFEB8])), // ش
        0x0635 => Some((true, [0xFEB9, 0xFEBA, 0xFEBB, 0xFEBC])), // ص
        0x0636 => Some((true, [0xFEBD, 0xFEBE, 0xFEBF, 0xFEC0])), // ض
        0x0637 => Some((true, [0xFEC1, 0xFEC2, 0xFEC3, 0xFEC4])), // ط
        0x0638 => Some((true, [0xFEC5, 0xFEC6, 0xFEC7, 0xFEC8])), // ظ
        0x0639 => Some((true, [0xFEC9, 0xFECA, 0xFECB, 0xFECC])), // ع
        0x063A => Some((true, [0xFECD, 0xFECE, 0xFECF, 0xFED0])), // غ
        0x0640 => Some((true, [0x0640, 0x0640, 0x0640, 0x0640])), // ـ Tatweel
        0x0641 => Some((true, [0xFED1, 0xFED2, 0xFED3, 0xFED4])), // ف
        0x0642 => Some((true, [0xFED5, 0xFED6, 0xFED7, 0xFED8])), // ق
        0x0643 => Some((true, [0xFED9, 0xFEDA, 0xFEDB, 0xFEDC])), // ك
        0x0644 => Some((true, [0xFEDD, 0xFEDE, 0xFEDF, 0xFEE0])), // ل
        0x0645 => Some((true, [0xFEE1, 0xFEE2, 0xFEE3, 0xFEE4])), // م
        0x0646 => Some((true, [0xFEE5, 0xFEE6, 0xFEE7, 0xFEE8])), // ن
        0x0647 => Some((true, [0xFEE9, 0xFEEA, 0xFEEB, 0xFEEC])), // ه
        0x064A => Some((true, [0xFEF1, 0xFEF2, 0xFEF3, 0xFEF4])), // ي
        0x067E => Some((true, [0xFB56, 0xFB57, 0xFB58, 0xFB59])), // پ
        0x0686 => Some((true, [0xFB7A, 0xFB7B, 0xFB7C, 0xFB7D])), // چ
        0x06A9 => Some((true, [0xFB8E, 0xFB8F, 0xFB90, 0xFB91])), // ک
        0x06AF => Some((true, [0xFB92, 0xFB93, 0xFB94, 0xFB95])), // گ
        0x06CC => Some((true, [0xFBFC, 0xFBFD, 0xFBFE, 0xFBFF])), // ی
        _ => None,
    }
}

#[inline]
fn is_ignorable_arabic_mark(cp: u32) -> bool {
    (0x064B..=0x065F).contains(&cp)
        || cp == 0x0670
        || (0x06D6..=0x06ED).contains(&cp)
        || cp == 0x061C
        || cp == 0x200E
        || cp == 0x200F
        || (0x202A..=0x202E).contains(&cp)
        || (0x2066..=0x2069).contains(&cp)
}

#[inline]
fn lam_alef_ligature(alef_cp: u32, prev_joins_left: bool) -> Option<u32> {
    match alef_cp {
        0x0622 => Some(if prev_joins_left { 0xFEF6 } else { 0xFEF5 }),
        0x0623 => Some(if prev_joins_left { 0xFEF8 } else { 0xFEF7 }),
        0x0625 => Some(if prev_joins_left { 0xFEFA } else { 0xFEF9 }),
        0x0627 => Some(if prev_joins_left { 0xFEFC } else { 0xFEFB }),
        _ => None,
    }
}

#[inline]
pub fn has_arabic_or_rtl(input: &str) -> bool {
    input.chars().any(|c| {
        let cp = c as u32;
        (0x0590..=0x08FF).contains(&cp)
            || (0xFB50..=0xFDFF).contains(&cp)
            || (0xFE70..=0xFEFF).contains(&cp)
    })
}

/// Applies Arabic contextual joining (`Isolated`, `Final`, `Initial`, `Medial`) and mandatory
/// `Lam-Alef` ligatures in **logical** character order (preserving top-to-bottom word-wrap order).
pub fn shape_arabic_logical(input: &str) -> String {
    if !has_arabic_or_rtl(input) {
        return input.to_string();
    }

    // Expand Rial ligature (U+FDFC) and strip non-spacing harakat/control marks
    let mut cps: Vec<u32> = Vec::with_capacity(input.len());
    for ch in input.chars() {
        let cp = ch as u32;
        if cp == 0xFDFC {
            cps.extend_from_slice(&[0x0631, 0x064A, 0x0627, 0x0644]);
        } else if !is_ignorable_arabic_mark(cp) {
            cps.push(cp);
        }
    }

    let mut out = String::with_capacity(input.len() + 8);
    let mut prev_joins_left = false;
    let mut i = 0usize;
    let n = cps.len();

    while i < n {
        let cp = cps[i];

        // Zero-width non-joiner / joiner
        if cp == 0x200C {
            prev_joins_left = false;
            i += 1;
            continue;
        }
        if cp == 0x200D {
            prev_joins_left = true;
            i += 1;
            continue;
        }

        // Isolated Hamza
        if cp == 0x0621 {
            out.push(char::from_u32(0xFE80).unwrap_or(' '));
            prev_joins_left = false;
            i += 1;
            continue;
        }

        // Mandatory Lam + Alef ligatures
        if cp == 0x0644 && i + 1 < n {
            if let Some(lig) = lam_alef_ligature(cps[i + 1], prev_joins_left) {
                out.push(char::from_u32(lig).unwrap_or(' '));
                prev_joins_left = false;
                i += 2;
                continue;
            }
        }

        if let Some((cur_joins_left, forms)) = arabic_joining_forms(cp) {
            let next_joins_right = if i + 1 < n {
                let next_cp = cps[i + 1];
                next_cp == 0x200D || arabic_joining_forms(next_cp).is_some()
            } else {
                false
            };

            let form_idx = match (prev_joins_left, cur_joins_left && next_joins_right) {
                (false, false) => 0, // Isolated
                (true, false) => 1,  // Final
                (false, true) => 2,  // Initial
                (true, true) => 3,   // Medial
            };
            out.push(char::from_u32(forms[form_idx]).unwrap_or(' '));
            prev_joins_left = cur_joins_left;
        } else {
            out.push(char::from_u32(cp).unwrap_or(' '));
            prev_joins_left = false;
        }

        i += 1;
    }

    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BidiClass {
    L,
    R,
    Num,
    Neutral,
}

fn classify_bidi(cp: u32) -> BidiClass {
    if (0x0660..=0x0669).contains(&cp)
        || (0x06F0..=0x06F9).contains(&cp)
        || (0x0030..=0x0039).contains(&cp)
    {
        return BidiClass::Num;
    }
    if (0x0590..=0x08FF).contains(&cp)
        || (0xFB50..=0xFDFF).contains(&cp)
        || (0xFE70..=0xFEFF).contains(&cp)
    {
        return BidiClass::R;
    }
    if (0x0041..=0x005A).contains(&cp)
        || (0x0061..=0x007A).contains(&cp)
        || (0x00C0..=0x02AF).contains(&cp)
        || (0x0370..=0x052F).contains(&cp)
        || matches!(cp, 0x00A3 | 0x00A5 | 0x20AC | 0x20BD | 0x2116)
    {
        return BidiClass::L;
    }
    BidiClass::Neutral
}

/// Returns `true` when the first strong directional character of `text` is Right-to-Left (Arabic).
pub fn is_rtl_dominant(text: &str) -> bool {
    for ch in text.chars() {
        match classify_bidi(ch as u32) {
            BidiClass::R => return true,
            BidiClass::L => return false,
            BidiClass::Num | BidiClass::Neutral => {}
        }
    }
    false
}

fn mirror_rtl_bracket(ch: char) -> char {
    match ch {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        _ => ch,
    }
}

/// Shapes Arabic contextual forms and reorders a single line into visual Left-to-Right PDF rendering order
/// following UAX #9 embedding levels (`0..=2`), preserving embedded LTR words and numeric sequences.
pub fn shape_and_reorder_line(logical_line: &str) -> String {
    if !has_arabic_or_rtl(logical_line) {
        return logical_line.to_string();
    }

    let shaped = shape_arabic_logical(logical_line);
    let chars: Vec<char> = shaped.chars().collect();
    let n = chars.len();
    if n == 0 {
        return shaped;
    }

    let base_rtl = is_rtl_dominant(&shaped);
    let mut classes: Vec<BidiClass> = chars.iter().map(|&c| classify_bidi(c as u32)).collect();

    // W4 / W5: Numeric separators (.,٫٬:/%+-#) adjacent to numbers or LTR identifiers stay with them
    for i in 0..n {
        let ch = chars[i];
        if (matches!(ch, '.' | ',' | '٫' | '٬' | ':' | '/')
            && i > 0
            && i + 1 < n
            && classes[i - 1] == BidiClass::Num
            && classes[i + 1] == BidiClass::Num)
            || (matches!(ch, '%' | '٪') && i > 0 && classes[i - 1] == BidiClass::Num)
        {
            classes[i] = BidiClass::Num;
        } else if matches!(ch, '#' | '+')
            && i + 1 < n
            && matches!(classes[i + 1], BidiClass::Num | BidiClass::L)
            && (i == 0 || classes[i - 1] == BidiClass::Neutral)
        {
            classes[i] = classes[i + 1];
        } else if ch == '-'
            && i > 0
            && i + 1 < n
            && matches!(classes[i - 1], BidiClass::Num | BidiClass::L)
            && matches!(classes[i + 1], BidiClass::Num | BidiClass::L)
        {
            classes[i] = BidiClass::L;
        }
    }

    // Assign preliminary levels:
    // - Base LTR (0): L=0, R=1, Num=0 (or 2 if preceded by strong R)
    // - Base RTL (1): R=1, L=2, Num=2
    let base_level: u8 = if base_rtl { 1 } else { 0 };
    let mut levels = vec![base_level; n];
    let mut last_strong = if base_rtl { BidiClass::R } else { BidiClass::L };

    for i in 0..n {
        match classes[i] {
            BidiClass::L => {
                levels[i] = if base_rtl { 2 } else { 0 };
                last_strong = BidiClass::L;
            }
            BidiClass::R => {
                levels[i] = 1;
                last_strong = BidiClass::R;
            }
            BidiClass::Num => {
                levels[i] = if base_rtl || last_strong == BidiClass::R {
                    2
                } else {
                    0
                };
            }
            BidiClass::Neutral => {}
        }
    }

    // N1 / N2: Resolve neutral runs: if bounded on both sides by the same level, adopt that level;
    // otherwise adopt base_level.
    let mut i = 0;
    while i < n {
        if classes[i] != BidiClass::Neutral {
            i += 1;
            continue;
        }
        let start = i;
        while i < n && classes[i] == BidiClass::Neutral {
            i += 1;
        }
        let left_lvl = if start > 0 {
            levels[start - 1]
        } else {
            base_level
        };
        let right_lvl = if i < n { levels[i] } else { base_level };
        let resolved = if left_lvl == right_lvl {
            left_lvl
        } else {
            base_level
        };
        for slot in &mut levels[start..i] {
            *slot = resolved;
        }
    }

    // L4: Mirror brackets on odd (RTL) level 1
    let mut visual = chars;
    for idx in 0..n {
        if levels[idx] % 2 == 1 {
            visual[idx] = mirror_rtl_bracket(visual[idx]);
        }
    }

    // L2: Reverse contiguous runs from max level (2) down to 1
    for target_lvl in [2u8, 1u8] {
        let mut idx = 0;
        while idx < n {
            if levels[idx] < target_lvl {
                idx += 1;
                continue;
            }
            let start = idx;
            while idx < n && levels[idx] >= target_lvl {
                idx += 1;
            }
            visual[start..idx].reverse();
            levels[start..idx].reverse();
        }
    }

    visual.into_iter().collect()
}
