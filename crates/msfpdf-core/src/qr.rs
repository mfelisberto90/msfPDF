use crate::error::PdfError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrMatrix {
    pub size: usize,
    pub modules: Vec<bool>,
}

impl QrMatrix {
    #[inline]
    pub fn get(&self, row: usize, col: usize) -> bool {
        self.modules[row * self.size + col]
    }
}

struct VersionSpec {
    version: usize,
    data_codewords: usize,
    ec_codewords_per_block: usize,
    num_blocks: usize,
    align_centers: &'static [usize],
}

const VERSIONS_ECC_L: [VersionSpec; 6] = [
    VersionSpec {
        version: 1,
        data_codewords: 19,
        ec_codewords_per_block: 7,
        num_blocks: 1,
        align_centers: &[],
    },
    VersionSpec {
        version: 2,
        data_codewords: 34,
        ec_codewords_per_block: 10,
        num_blocks: 1,
        align_centers: &[6, 18],
    },
    VersionSpec {
        version: 3,
        data_codewords: 55,
        ec_codewords_per_block: 15,
        num_blocks: 1,
        align_centers: &[6, 22],
    },
    VersionSpec {
        version: 4,
        data_codewords: 80,
        ec_codewords_per_block: 20,
        num_blocks: 1,
        align_centers: &[6, 26],
    },
    VersionSpec {
        version: 5,
        data_codewords: 108,
        ec_codewords_per_block: 26,
        num_blocks: 1,
        align_centers: &[6, 30],
    },
    VersionSpec {
        version: 6,
        data_codewords: 136,
        ec_codewords_per_block: 18,
        num_blocks: 2,
        align_centers: &[6, 34],
    },
];

fn gf_mul(mut x: u8, mut y: u8) -> u8 {
    let mut z = 0u8;
    for _ in 0..8 {
        if (y & 1) != 0 {
            z ^= x;
        }
        let hi = (x & 0x80) != 0;
        x <<= 1;
        if hi {
            x ^= 0x1D; // x^8 + x^4 + x^3 + x^2 + 1 (0x11D)
        }
        y >>= 1;
    }
    z
}

fn rs_divisor(degree: usize) -> Vec<u8> {
    let mut result = vec![0u8; degree];
    result[degree - 1] = 1;
    let mut root = 1u8;
    for _ in 0..degree {
        for j in 0..degree {
            result[j] = gf_mul(result[j], root);
            if j + 1 < degree {
                result[j] ^= result[j + 1];
            }
        }
        root = gf_mul(root, 0x02);
    }
    result
}

fn rs_remainder(data: &[u8], divisor: &[u8]) -> Vec<u8> {
    let mut result = vec![0u8; divisor.len()];
    for &byte in data {
        let factor = byte ^ result[0];
        result.remove(0);
        result.push(0);
        for (r, &d) in result.iter_mut().zip(divisor.iter()) {
            *r ^= gf_mul(d, factor);
        }
    }
    result
}

fn push_bits(buf: &mut Vec<bool>, val: u32, len: usize) {
    for i in (0..len).rev() {
        buf.push(((val >> i) & 1) != 0);
    }
}

fn mask_bit(mask: u8, r: usize, c: usize) -> bool {
    match mask {
        0 => (r + c).is_multiple_of(2),
        1 => r.is_multiple_of(2),
        2 => c.is_multiple_of(3),
        3 => (r + c).is_multiple_of(3),
        4 => ((r / 2) + (c / 3)).is_multiple_of(2),
        5 => ((r * c) % 2) + ((r * c) % 3) == 0,
        6 => (((r * c) % 2) + ((r * c) % 3)).is_multiple_of(2),
        _ => (((r + c) % 2) + ((r * c) % 3)).is_multiple_of(2),
    }
}

fn format_bits_ecc_l(mask: u8) -> u16 {
    // ECC Level L indicator = 0b01
    let data = (0b01u16 << 3) | (mask as u16 & 0x07);
    let mut rem = data << 10;
    for i in (0..5).rev() {
        if ((rem >> (i + 10)) & 1) != 0 {
            rem ^= 0x537 << i;
        }
    }
    ((data << 10) | rem) ^ 0x5412
}

fn draw_format_bits(modules: &mut [bool], is_func: &mut [bool], size: usize, mask: u8) {
    let bits = format_bits_ecc_l(mask);
    let get_b = |idx: u16| ((bits >> idx) & 1) != 0;

    // Top-left copy (MSB bit 14 -> LSB bit 0)
    let tl_coords: [(usize, usize, u16); 15] = [
        (8, 0, 14),
        (8, 1, 13),
        (8, 2, 12),
        (8, 3, 11),
        (8, 4, 10),
        (8, 5, 9),
        (8, 7, 8),
        (8, 8, 7),
        (7, 8, 6),
        (5, 8, 5),
        (4, 8, 4),
        (3, 8, 3),
        (2, 8, 2),
        (1, 8, 1),
        (0, 8, 0),
    ];
    for (r, c, b_idx) in tl_coords {
        modules[r * size + c] = get_b(b_idx);
        is_func[r * size + c] = true;
    }

    // Bottom-left vertical (bits 14..=8) + dark module
    for i in 0..7 {
        let r = size - 1 - i;
        let b_idx = 14 - (i as u16);
        modules[r * size + 8] = get_b(b_idx);
        is_func[r * size + 8] = true;
    }
    modules[(size - 8) * size + 8] = true;
    is_func[(size - 8) * size + 8] = true;

    // Top-right horizontal (bits 7..=0)
    for i in 0..8 {
        let c = size - 8 + i;
        let b_idx = 7 - (i as u16);
        modules[8 * size + c] = get_b(b_idx);
        is_func[8 * size + c] = true;
    }
}

fn penalty_score(modules: &[bool], size: usize) -> u32 {
    let mut score = 0u32;

    // N1: 5+ consecutive modules in row/col
    for r in 0..size {
        let mut run_val = modules[r * size];
        let mut run_len = 1usize;
        for c in 1..size {
            let v = modules[r * size + c];
            if v == run_val {
                run_len += 1;
            } else {
                if run_len >= 5 {
                    score += 3 + (run_len as u32 - 5);
                }
                run_val = v;
                run_len = 1;
            }
        }
        if run_len >= 5 {
            score += 3 + (run_len as u32 - 5);
        }
    }

    for c in 0..size {
        let mut run_val = modules[c];
        let mut run_len = 1usize;
        for r in 1..size {
            let v = modules[r * size + c];
            if v == run_val {
                run_len += 1;
            } else {
                if run_len >= 5 {
                    score += 3 + (run_len as u32 - 5);
                }
                run_val = v;
                run_len = 1;
            }
        }
        if run_len >= 5 {
            score += 3 + (run_len as u32 - 5);
        }
    }

    // N2: 2x2 solid blocks
    for r in 0..(size - 1) {
        for c in 0..(size - 1) {
            let v = modules[r * size + c];
            if modules[r * size + c + 1] == v
                && modules[(r + 1) * size + c] == v
                && modules[(r + 1) * size + c + 1] == v
            {
                score += 3;
            }
        }
    }

    score
}

/// Encodes a UTF-8 string (up to 134 bytes) into an ISO/IEC 18004 QR Code matrix (Versions 1..=6, ECC-L).
pub fn encode_qr(payload: &str) -> Result<QrMatrix, PdfError> {
    let raw = payload.as_bytes();
    if raw.is_empty() {
        return Err(PdfError::InvalidSpec(
            "QR payload cannot be empty".to_string(),
        ));
    }

    // Byte mode header = 4 bits (0100) + 8 bits character count (for versions 1..=9) = 12 bits
    let spec = VERSIONS_ECC_L
        .iter()
        .find(|s| (12 + raw.len() * 8) <= s.data_codewords * 8)
        .ok_or(PdfError::QrCapacityExceeded(raw.len()))?;

    let mut bits = Vec::with_capacity(spec.data_codewords * 8);
    push_bits(&mut bits, 0b0100, 4);
    push_bits(&mut bits, raw.len() as u32, 8);
    for &b in raw {
        push_bits(&mut bits, b as u32, 8);
    }

    let capacity_bits = spec.data_codewords * 8;
    let terminator = (capacity_bits - bits.len()).min(4);
    push_bits(&mut bits, 0, terminator);
    while !bits.len().is_multiple_of(8) {
        bits.push(false);
    }

    let mut data_bytes = Vec::with_capacity(spec.data_codewords);
    for base in (0..bits.len()).step_by(8) {
        let mut val = 0u8;
        for &bit in &bits[base..base + 8] {
            val = (val << 1) | u8::from(bit);
        }
        data_bytes.push(val);
    }

    let pad_bytes = [0xECu8, 0x11u8];
    let mut pad_idx = 0usize;
    while data_bytes.len() < spec.data_codewords {
        data_bytes.push(pad_bytes[pad_idx % 2]);
        pad_idx += 1;
    }

    // Reed-Solomon block encoding & interleaving
    let divisor = rs_divisor(spec.ec_codewords_per_block);
    let block_data_len = spec.data_codewords / spec.num_blocks;
    let mut data_blocks = Vec::with_capacity(spec.num_blocks);
    let mut ec_blocks = Vec::with_capacity(spec.num_blocks);

    for b in 0..spec.num_blocks {
        let slice = &data_bytes[b * block_data_len..(b + 1) * block_data_len];
        ec_blocks.push(rs_remainder(slice, &divisor));
        data_blocks.push(slice);
    }

    let mut all_codewords =
        Vec::with_capacity(spec.data_codewords + spec.ec_codewords_per_block * spec.num_blocks);
    for i in 0..block_data_len {
        for blk in &data_blocks {
            all_codewords.push(blk[i]);
        }
    }
    for i in 0..spec.ec_codewords_per_block {
        for ec in &ec_blocks {
            all_codewords.push(ec[i]);
        }
    }

    // Build matrix & function reservations
    let size = spec.version * 4 + 17;
    let mut base_modules = vec![false; size * size];
    let mut is_func = vec![false; size * size];

    let mut set_func = |r: usize, c: usize, dark: bool| {
        base_modules[r * size + c] = dark;
        is_func[r * size + c] = true;
    };

    // 3 Finder patterns + 1-module white separators
    let draw_finder = |top_r: isize, left_c: isize, set: &mut dyn FnMut(usize, usize, bool)| {
        for dr in -1..=7 {
            for dc in -1..=7 {
                let rr = top_r + dr;
                let cc = left_c + dc;
                if rr >= 0 && rr < size as isize && cc >= 0 && cc < size as isize {
                    let in_outer = (0..=6).contains(&dr) && (0..=6).contains(&dc);
                    let on_ring = dr == 0 || dr == 6 || dc == 0 || dc == 6;
                    let in_core = (2..=4).contains(&dr) && (2..=4).contains(&dc);
                    set(rr as usize, cc as usize, in_outer && (on_ring || in_core));
                }
            }
        }
    };

    draw_finder(0, 0, &mut set_func);
    draw_finder(0, (size - 7) as isize, &mut set_func);
    draw_finder((size - 7) as isize, 0, &mut set_func);

    // Timing patterns
    for i in 8..(size - 8) {
        let dark = i.is_multiple_of(2);
        set_func(6, i, dark);
        set_func(i, 6, dark);
    }

    // Alignment patterns (skipping finder corners)
    let n_align = spec.align_centers.len();
    for (ri, &cr) in spec.align_centers.iter().enumerate() {
        for (ci, &cc) in spec.align_centers.iter().enumerate() {
            if (ri == 0 && ci == 0)
                || (ri == 0 && ci == n_align - 1)
                || (ri == n_align - 1 && ci == 0)
            {
                continue;
            }
            for dr in -2isize..=2 {
                for dc in -2isize..=2 {
                    let r = (cr as isize + dr) as usize;
                    let c = (cc as isize + dc) as usize;
                    let dark = dr.abs().max(dc.abs()) != 1;
                    set_func(r, c, dark);
                }
            }
        }
    }

    // Reserve format areas temporarily before data placement
    draw_format_bits(&mut base_modules, &mut is_func, size, 0);

    // Zig-zag upward/downward 2-column bit placement
    let mut bit_idx = 0usize;
    let total_bits = all_codewords.len() * 8;
    let mut right_col = size as isize - 1;
    let mut upward = true;

    while right_col >= 1 {
        if right_col == 6 {
            right_col -= 1; // Skip vertical timing column
        }
        for step in 0..size {
            let r = if upward { size - 1 - step } else { step };
            for dc in 0..2 {
                let c = (right_col - dc) as usize;
                if !is_func[r * size + c] {
                    let dark = if bit_idx < total_bits {
                        let byte = all_codewords[bit_idx / 8];
                        ((byte >> (7 - (bit_idx % 8))) & 1) != 0
                    } else {
                        false
                    };
                    base_modules[r * size + c] = dark;
                    bit_idx += 1;
                }
            }
        }
        upward = !upward;
        right_col -= 2;
    }

    // Evaluate all 8 masks and select lowest penalty
    let mut best_modules = base_modules.clone();
    let mut best_score = u32::MAX;

    for mask in 0..8u8 {
        let mut candidate = base_modules.clone();
        for r in 0..size {
            for c in 0..size {
                let idx = r * size + c;
                if !is_func[idx] && mask_bit(mask, r, c) {
                    candidate[idx] = !candidate[idx];
                }
            }
        }
        let mut dummy_func = is_func.clone();
        draw_format_bits(&mut candidate, &mut dummy_func, size, mask);
        let score = penalty_score(&candidate, size);
        if score < best_score {
            best_score = score;
            best_modules = candidate;
        }
    }

    Ok(QrMatrix {
        size,
        modules: best_modules,
    })
}
