#!/usr/bin/env python3
"""Deterministic TrueType-to-%PDF-1.4 Type 3 vector glyph table generator for msfPDF.

Extracts Public Domain Cyrillic (Russian & Slavic) and Arabic (Connected Presentation Forms,
Lam-Alef ligatures, Arabic-Indic digits, and punctuation) outlines from DejaVuSans (2048 UPM)
and emits byte-identical static tables for both Rust (`glyphs_data.rs`) and TypeScript (`glyphsData.ts`).
"""

from __future__ import annotations

import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
REGULAR_TTF = Path("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf")
BOLD_TTF = Path("/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf")

# Canonical /ToUnicode normalization map from Arabic Presentation Forms back to NFC Unicode hex (UTF-16BE)
PRESENTATION_TO_UNICODE: dict[int, str] = {
    0xFE80: "0621",
    0xFE81: "0622", 0xFE82: "0622",
    0xFE83: "0623", 0xFE84: "0623",
    0xFE85: "0624", 0xFE86: "0624",
    0xFE87: "0625", 0xFE88: "0625",
    0xFE89: "0626", 0xFE8A: "0626", 0xFE8B: "0626", 0xFE8C: "0626",
    0xFE8D: "0627", 0xFE8E: "0627",
    0xFE8F: "0628", 0xFE90: "0628", 0xFE91: "0628", 0xFE92: "0628",
    0xFE93: "0629", 0xFE94: "0629",
    0xFE95: "062A", 0xFE96: "062A", 0xFE97: "062A", 0xFE98: "062A",
    0xFE99: "062B", 0xFE9A: "062B", 0xFE9B: "062B", 0xFE9C: "062B",
    0xFE9D: "062C", 0xFE9E: "062C", 0xFE9F: "062C", 0xFEA0: "062C",
    0xFEA1: "062D", 0xFEA2: "062D", 0xFEA3: "062D", 0xFEA4: "062D",
    0xFEA5: "062E", 0xFEA6: "062E", 0xFEA7: "062E", 0xFEA8: "062E",
    0xFEA9: "062F", 0xFEAA: "062F",
    0xFEAB: "0630", 0xFEAC: "0630",
    0xFEAD: "0631", 0xFEAE: "0631",
    0xFEAF: "0632", 0xFEB0: "0632",
    0xFEB1: "0633", 0xFEB2: "0633", 0xFEB3: "0633", 0xFEB4: "0633",
    0xFEB5: "0634", 0xFEB6: "0634", 0xFEB7: "0634", 0xFEB8: "0634",
    0xFEB9: "0635", 0xFEBA: "0635", 0xFEBB: "0635", 0xFEBC: "0635",
    0xFEBD: "0636", 0xFEBE: "0636", 0xFEBF: "0636", 0xFEC0: "0636",
    0xFEC1: "0637", 0xFEC2: "0637", 0xFEC3: "0637", 0xFEC4: "0637",
    0xFEC5: "0638", 0xFEC6: "0638", 0xFEC7: "0638", 0xFEC8: "0638",
    0xFEC9: "0639", 0xFECA: "0639", 0xFECB: "0639", 0xFECC: "0639",
    0xFECD: "063A", 0xFECE: "063A", 0xFECF: "063A", 0xFED0: "063A",
    0xFED1: "0641", 0xFED2: "0641", 0xFED3: "0641", 0xFED4: "0641",
    0xFED5: "0642", 0xFED6: "0642", 0xFED7: "0642", 0xFED8: "0642",
    0xFED9: "0643", 0xFEDA: "0643", 0xFEDB: "0643", 0xFEDC: "0643",
    0xFEDD: "0644", 0xFEDE: "0644", 0xFEDF: "0644", 0xFEE0: "0644",
    0xFEE1: "0645", 0xFEE2: "0645", 0xFEE3: "0645", 0xFEE4: "0645",
    0xFEE5: "0646", 0xFEE6: "0646", 0xFEE7: "0646", 0xFEE8: "0646",
    0xFEE9: "0647", 0xFEEA: "0647", 0xFEEB: "0647", 0xFEEC: "0647",
    0xFEED: "0648", 0xFEEE: "0648",
    0xFEEF: "0649", 0xFEF0: "0649",
    0xFEF1: "064A", 0xFEF2: "064A", 0xFEF3: "064A", 0xFEF4: "064A",
    # Mandatory Lam-Alef ligatures -> 2 UTF-16BE code units!
    0xFEF5: "06440622", 0xFEF6: "06440622",
    0xFEF7: "06440623", 0xFEF8: "06440623",
    0xFEF9: "06440625", 0xFEFA: "06440625",
    0xFEFB: "06440627", 0xFEFC: "06440627",
    # Persian / Urdu Presentation Forms-A
    0xFB56: "067E", 0xFB57: "067E", 0xFB58: "067E", 0xFB59: "067E",
    0xFB7A: "0686", 0xFB7B: "0686", 0xFB7C: "0686", 0xFB7D: "0686",
    0xFB8A: "0698", 0xFB8B: "0698",
    0xFB8E: "06A9", 0xFB8F: "06A9", 0xFB90: "06A9", 0xFB91: "06A9",
    0xFB92: "06AF", 0xFB93: "06AF", 0xFB94: "06AF", 0xFB95: "06AF",
    0xFBFC: "06CC", 0xFBFD: "06CC", 0xFBFE: "06CC", 0xFBFF: "06CC",
}


class TtfReader:
    def __init__(self, path: Path) -> None:
        self.data = path.read_bytes()
        num_tables = struct.unpack_from(">H", self.data, 4)[0]
        self.tables: dict[str, tuple[int, int]] = {}
        for i in range(num_tables):
            tag, _, off, length = struct.unpack_from(">4sIII", self.data, 12 + i * 16)
            self.tables[tag.decode("latin1")] = (off, length)

        head_off = self.tables["head"][0]
        self.upm = struct.unpack_from(">H", self.data, head_off + 18)[0]
        self.loca_fmt = struct.unpack_from(">h", self.data, head_off + 50)[0]
        hhea_off = self.tables["hhea"][0]
        self.num_hmetrics = struct.unpack_from(">H", self.data, hhea_off + 34)[0]
        maxp_off = self.tables["maxp"][0]
        self.num_glyphs = struct.unpack_from(">H", self.data, maxp_off + 4)[0]
        self.cmap = self._read_cmap4()
        self.loca = self._read_loca()
        self.hmtx = self._read_hmtx()

    def _read_cmap4(self) -> dict[int, int]:
        cmap_off = self.tables["cmap"][0]
        num_sub = struct.unpack_from(">H", self.data, cmap_off + 2)[0]
        for i in range(num_sub):
            pid, eid, sub_off = struct.unpack_from(">HHI", self.data, cmap_off + 4 + i * 8)
            off = cmap_off + sub_off
            fmt = struct.unpack_from(">H", self.data, off)[0]
            if fmt == 4 and ((pid == 3 and eid == 1) or pid == 0):
                seg_count = struct.unpack_from(">H", self.data, off + 6)[0] // 2
                end_codes = struct.unpack_from(f">{seg_count}H", self.data, off + 14)
                start_codes = struct.unpack_from(f">{seg_count}H", self.data, off + 16 + 2 * seg_count)
                id_deltas = struct.unpack_from(f">{seg_count}h", self.data, off + 16 + 4 * seg_count)
                id_range_pos = off + 16 + 6 * seg_count
                id_range_offs = struct.unpack_from(f">{seg_count}H", self.data, id_range_pos)
                mapping: dict[int, int] = {}
                for s in range(seg_count):
                    for c in range(start_codes[s], end_codes[s] + 1):
                        if c == 0xFFFF:
                            continue
                        ro = id_range_offs[s]
                        if ro == 0:
                            gid = (c + id_deltas[s]) & 0xFFFF
                        else:
                            g_pos = id_range_pos + 2 * s + ro + 2 * (c - start_codes[s])
                            gid = struct.unpack_from(">H", self.data, g_pos)[0]
                            if gid != 0:
                                gid = (gid + id_deltas[s]) & 0xFFFF
                        if gid != 0:
                            mapping[c] = gid
                return mapping
        raise RuntimeError("Missing cmap format 4")

    def _read_loca(self) -> list[int]:
        off = self.tables["loca"][0]
        if self.loca_fmt == 0:
            vals = struct.unpack_from(f">{self.num_glyphs + 1}H", self.data, off)
            return [v * 2 for v in vals]
        return list(struct.unpack_from(f">{self.num_glyphs + 1}I", self.data, off))

    def _read_hmtx(self) -> list[int]:
        off = self.tables["hmtx"][0]
        advs: list[int] = []
        for i in range(self.num_hmetrics):
            adv, _ = struct.unpack_from(">Hh", self.data, off + i * 4)
            advs.append(adv)
        last = advs[-1]
        while len(advs) < self.num_glyphs:
            advs.append(last)
        return advs

    def get_contours(self, gid: int) -> tuple[int, tuple[int, int, int, int], list[list[tuple[int, int, bool]]]]:
        adv = self.hmtx[gid]
        glyf_off = self.tables["glyf"][0]
        g_start = self.loca[gid]
        g_end = self.loca[gid + 1]
        if g_start == g_end:
            return adv, (0, 0, 0, 0), []

        pos = glyf_off + g_start
        num_contours, xmin, ymin, xmax, ymax = struct.unpack_from(">hhhhh", self.data, pos)
        pos += 10

        if num_contours >= 0:
            end_pts = struct.unpack_from(f">{num_contours}H", self.data, pos)
            pos += 2 * num_contours
            instr_len = struct.unpack_from(">H", self.data, pos)[0]
            pos += 2 + instr_len
            num_pts = end_pts[-1] + 1 if num_contours > 0 else 0

            flags: list[int] = []
            while len(flags) < num_pts:
                f = self.data[pos]
                pos += 1
                flags.append(f)
                if f & 0x08:
                    rep = self.data[pos]
                    pos += 1
                    flags.extend([f] * rep)

            xs: list[int] = []
            cur_x = 0
            for f in flags:
                if f & 0x02:
                    dx = self.data[pos]
                    pos += 1
                    cur_x += dx if (f & 0x10) else -dx
                elif not (f & 0x10):
                    dx = struct.unpack_from(">h", self.data, pos)[0]
                    pos += 2
                    cur_x += dx
                xs.append(cur_x)

            ys: list[int] = []
            cur_y = 0
            for f in flags:
                if f & 0x04:
                    dy = self.data[pos]
                    pos += 1
                    cur_y += dy if (f & 0x20) else -dy
                elif not (f & 0x20):
                    dy = struct.unpack_from(">h", self.data, pos)[0]
                    pos += 2
                    cur_y += dy
                ys.append(cur_y)

            contours: list[list[tuple[int, int, bool]]] = []
            start = 0
            for end in end_pts:
                contours.append([(xs[i], ys[i], bool(flags[i] & 0x01)) for i in range(start, end + 1)])
                start = end + 1
            return adv, (xmin, ymin, xmax, ymax), contours

        all_contours: list[list[tuple[int, int, bool]]] = []
        while True:
            comp_flags, sub_gid = struct.unpack_from(">HH", self.data, pos)
            pos += 4
            if comp_flags & 0x0001:
                dx, dy = struct.unpack_from(">hh", self.data, pos) if (comp_flags & 0x0002) else (0, 0)
                pos += 4
            else:
                dx, dy = struct.unpack_from(">bb", self.data, pos) if (comp_flags & 0x0002) else (0, 0)
                pos += 2

            a, b, c, d = 1.0, 0.0, 0.0, 1.0
            if comp_flags & 0x0008:
                a = d = struct.unpack_from(">h", self.data, pos)[0] / 16384.0
                pos += 2
            elif comp_flags & 0x0040:
                a = struct.unpack_from(">h", self.data, pos)[0] / 16384.0
                d = struct.unpack_from(">h", self.data, pos + 2)[0] / 16384.0
                pos += 4
            elif comp_flags & 0x0080:
                a = struct.unpack_from(">h", self.data, pos)[0] / 16384.0
                b = struct.unpack_from(">h", self.data, pos + 2)[0] / 16384.0
                c = struct.unpack_from(">h", self.data, pos + 4)[0] / 16384.0
                d = struct.unpack_from(">h", self.data, pos + 6)[0] / 16384.0
                pos += 8

            _, _, sub_contours = self.get_contours(sub_gid)
            for cont in sub_contours:
                all_contours.append(
                    [(int(round(a * x + c * y + dx)), int(round(b * x + d * y + dy)), on) for (x, y, on) in cont]
                )
            if not (comp_flags & 0x0020):
                break
        return adv, (xmin, ymin, xmax, ymax), all_contours


def glyph_to_type3_charproc(ttf: TtfReader, cp: int) -> tuple[int, str]:
    gid = ttf.cmap[cp]
    adv, (xmin, ymin, xmax, ymax), contours = ttf.get_contours(gid)
    if not contours:
        return adv, f"{adv} 0 d0"

    parts = [f"{adv} 0 {xmin} {ymin} {xmax} {ymax} d1"]
    for pts in contours:
        if not pts:
            continue
        expanded: list[tuple[float, float, bool]] = []
        n = len(pts)
        for i in range(n):
            x, y, on = pts[i]
            nx, ny, non = pts[(i + 1) % n]
            expanded.append((float(x), float(y), on))
            if not on and not non:
                expanded.append(((x + nx) * 0.5, (y + ny) * 0.5, True))

        first_on = next((idx for idx, p in enumerate(expanded) if p[2]), 0)
        expanded = expanded[first_on:] + expanded[:first_on]

        sx, sy, _ = expanded[0]
        parts.append(f"{round(sx)} {round(sy)} m")
        i = 1
        m = len(expanded)
        cur_x, cur_y = sx, sy
        while i < m:
            px, py, on = expanded[i]
            if on:
                parts.append(f"{round(px)} {round(py)} l")
                cur_x, cur_y = px, py
                i += 1
            else:
                ex, ey, _ = expanded[(i + 1) % m]
                c1x = cur_x + (2.0 / 3.0) * (px - cur_x)
                c1y = cur_y + (2.0 / 3.0) * (py - cur_y)
                c2x = ex + (2.0 / 3.0) * (px - ex)
                c2y = ey + (2.0 / 3.0) * (py - ey)
                parts.append(
                    f"{round(c1x)} {round(c1y)} {round(c2x)} {round(c2y)} {round(ex)} {round(ey)} c"
                )
                cur_x, cur_y = ex, ey
                i += 2
        parts.append("h")
    parts.append("f")
    return adv, " ".join(parts)


def collect_target_codepoints() -> list[int]:
    cps: set[int] = set()
    # Cyrillic (Russian, Ukrainian, Belarusian, Serbian, Bulgarian, Macedonian) + Ruble + Numero
    cps.update(range(0x0401, 0x045F + 1))
    cps.update([0x0490, 0x0491, 0x20BD, 0x2116])
    # Arabic punctuation, Tatweel, Arabic-Indic & Extended Arabic-Indic digits
    cps.update([0x060C, 0x061B, 0x061F, 0x0640, 0x066A, 0x066B, 0x066C])
    cps.update(range(0x0660, 0x066A))
    cps.update(range(0x06F0, 0x06FA))
    # Arabic Presentation Forms-B (all 125 contextual glyphs + 8 Lam-Alef ligatures)
    cps.update(range(0xFE80, 0xFEFD))
    # Persian / Urdu Presentation Forms-A
    cps.update([
        0xFB56, 0xFB57, 0xFB58, 0xFB59,
        0xFB7A, 0xFB7B, 0xFB7C, 0xFB7D,
        0xFB8A, 0xFB8B,
        0xFB8E, 0xFB8F, 0xFB90, 0xFB91,
        0xFB92, 0xFB93, 0xFB94, 0xFB95,
        0xFBFC, 0xFBFD, 0xFBFE, 0xFBFF,
    ])
    return sorted(cps)


def main() -> None:
    reg_ttf = TtfReader(REGULAR_TTF)
    bold_ttf = TtfReader(BOLD_TTF)
    assert reg_ttf.upm == 2048 and bold_ttf.upm == 2048

    codepoints = collect_target_codepoints()

    rows = []
    for cp in codepoints:
        reg_adv, reg_path = glyph_to_type3_charproc(reg_ttf, cp)
        bold_adv, bold_path = glyph_to_type3_charproc(bold_ttf, cp)
        to_uni = PRESENTATION_TO_UNICODE.get(cp, f"{cp:04X}")
        rows.append((cp, to_uni, reg_adv, reg_path, bold_adv, bold_path))

    # Emit Rust binary-searchable static slice
    rs_lines = [
        "// @generated by scripts/gen-vector-glyphs.py — DO NOT EDIT MANUALLY.",
        "// Public Domain Cyrillic & Connected Arabic vector outlines (2048 UPM).",
        "",
        "#[derive(Debug, Clone, Copy)]",
        "pub struct VectorGlyphRecord {",
        "    pub codepoint: u32,",
        "    pub to_unicode_hex: &'static str,",
        "    pub regular_adv: u16,",
        "    pub regular_proc: &'static str,",
        "    pub bold_adv: u16,",
        "    pub bold_proc: &'static str,",
        "}",
        "",
        "pub const VECTOR_UPM: f64 = 2048.0;",
        "pub const VECTOR_FONT_MATRIX: &str = \"[0.00048828125 0 0 0.00048828125 0 0]\";",
        "pub const VECTOR_FONT_BBOX: &str = \"[-520 -680 2800 2120]\";",
        "",
        f"pub static VECTOR_GLYPHS: [VectorGlyphRecord; {len(rows)}] = [",
    ]
    for cp, to_uni, r_adv, r_proc, b_adv, b_proc in rows:
        rs_lines.append(
            f"    VectorGlyphRecord {{ codepoint: 0x{cp:04X}, to_unicode_hex: \"{to_uni}\", "
            f"regular_adv: {r_adv}, regular_proc: \"{r_proc}\", "
            f"bold_adv: {b_adv}, bold_proc: \"{b_proc}\" }},"
        )
    rs_lines.extend([
        "];",
        "",
        "#[inline]",
        "pub fn lookup_vector_glyph(cp: u32) -> Option<&'static VectorGlyphRecord> {",
        "    VECTOR_GLYPHS",
        "        .binary_search_by_key(&cp, |g| g.codepoint)",
        "        .ok()",
        "        .map(|idx| &VECTOR_GLYPHS[idx])",
        "}",
        "",
    ])
    rs_out = ROOT / "crates/msfpdf-core/src/glyphs_data.rs"
    rs_out.write_text("\n".join(rs_lines), encoding="utf-8")

    # Emit TypeScript compact Map
    ts_lines = [
        "// @generated by scripts/gen-vector-glyphs.py — DO NOT EDIT MANUALLY.",
        "// Public Domain Cyrillic & Connected Arabic vector outlines (2048 UPM).",
        "",
        "export interface VectorGlyphRecord {",
        "  readonly toUnicodeHex: string;",
        "  readonly regularAdv: number;",
        "  readonly regularProc: string;",
        "  readonly boldAdv: number;",
        "  readonly boldProc: string;",
        "}",
        "",
        "export const VECTOR_UPM = 2048;",
        "export const VECTOR_FONT_MATRIX = '[0.00048828125 0 0 0.00048828125 0 0]';",
        "export const VECTOR_FONT_BBOX = '[-520 -680 2800 2120]';",
        "",
        "const RAW_GLYPHS: ReadonlyArray<readonly [number, string, number, string, number, string]> = [",
    ]
    for cp, to_uni, r_adv, r_proc, b_adv, b_proc in rows:
        ts_lines.append(
            f"  [0x{cp:04X}, '{to_uni}', {r_adv}, '{r_proc}', {b_adv}, '{b_proc}'],"
        )
    ts_lines.extend([
        "];",
        "",
        "const GLYPH_MAP: ReadonlyMap<number, VectorGlyphRecord> = new Map(",
        "  RAW_GLYPHS.map(([cp, toUnicodeHex, regularAdv, regularProc, boldAdv, boldProc]) => [",
        "    cp,",
        "    { toUnicodeHex, regularAdv, regularProc, boldAdv, boldProc },",
        "  ])",
        ");",
        "",
        "export function lookupVectorGlyph(cp: number): VectorGlyphRecord | undefined {",
        "  return GLYPH_MAP.get(cp);",
        "}",
        "",
    ])
    ts_out = ROOT / "packages/ts/src/glyphsData.ts"
    ts_out.write_text("\n".join(ts_lines), encoding="utf-8")

    print(
        f"Generated {len(rows)} glyphs -> "
        f"Rust: {rs_out.stat().st_size / 1024:.1f} KiB, "
        f"TS: {ts_out.stat().st_size / 1024:.1f} KiB"
    )


if __name__ == "__main__":
    main()
