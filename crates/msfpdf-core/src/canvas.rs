use crate::encoding::{estimate_font_width_pt, pdf_escape_win_ansi};
use crate::geometry::{FontFace, Rgb};
use crate::glyphs_data::{lookup_vector_glyph, VECTOR_FONT_BBOX, VECTOR_FONT_MATRIX, VECTOR_UPM};
use crate::jpeg::ValidatedJpeg;
use crate::qr::QrMatrix;
use crate::shaping::shape_and_reorder_line;

#[derive(Debug, Clone)]
pub enum PageOp {
    Raw(String),
    VectorText {
        bold: bool,
        size: f64,
        color: Rgb,
        x: f64,
        y: f64,
        cps: Vec<u32>,
    },
}

#[derive(Debug, Clone)]
pub struct PdfPageOps {
    pub page_w: f64,
    pub page_h: f64,
    pub margin_x: f64,
    pub ops: Vec<PageOp>,
}

enum MixedRun {
    WinAnsi(String),
    Vector(Vec<u32>),
}

fn split_mixed_runs(visual: &str) -> Vec<MixedRun> {
    let mut runs: Vec<MixedRun> = Vec::new();
    for ch in visual.chars() {
        let cp = ch as u32;
        if lookup_vector_glyph(cp).is_some() {
            match runs.last_mut() {
                Some(MixedRun::Vector(cps)) => cps.push(cp),
                _ => runs.push(MixedRun::Vector(vec![cp])),
            }
        } else {
            match runs.last_mut() {
                Some(MixedRun::WinAnsi(s)) => s.push(ch),
                _ => {
                    let mut s = String::new();
                    s.push(ch);
                    runs.push(MixedRun::WinAnsi(s));
                }
            }
        }
    }
    runs
}

impl PdfPageOps {
    pub fn new(page_w: f64, page_h: f64, margin_x: f64) -> Self {
        Self {
            page_w,
            page_h,
            margin_x,
            ops: Vec::with_capacity(128),
        }
    }

    #[inline]
    fn push_raw(&mut self, op: String) {
        self.ops.push(PageOp::Raw(op));
    }

    pub fn fill_rect(&mut self, x: f64, y_top: f64, w: f64, h: f64, color: Rgb) {
        let y = self.page_h - y_top - h;
        self.push_raw(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
        self.push_raw(format!("{x:.2} {y:.2} {w:.2} {h:.2} re f"));
    }

    pub fn stroke_rect(&mut self, x: f64, y_top: f64, w: f64, h: f64, line_w: f64, color: Rgb) {
        let y = self.page_h - y_top - h;
        self.push_raw(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.push_raw(format!("{x:.2} {y:.2} {w:.2} {h:.2} re S"));
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fill_and_stroke_rect(
        &mut self,
        x: f64,
        y_top: f64,
        w: f64,
        h: f64,
        fill: Rgb,
        line_w: f64,
        stroke: Rgb,
    ) {
        self.fill_rect(x, y_top, w, h, fill);
        self.stroke_rect(x, y_top, w, h, line_w, stroke);
    }

    pub fn line(&mut self, x1: f64, y_top1: f64, x2: f64, y_top2: f64, line_w: f64, color: Rgb) {
        let y1 = self.page_h - y_top1;
        let y2 = self.page_h - y_top2;
        self.push_raw(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.push_raw(format!("{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S"));
    }

    pub fn dashed_line(
        &mut self,
        x1: f64,
        y_top1: f64,
        x2: f64,
        y_top2: f64,
        line_w: f64,
        color: Rgb,
    ) {
        let y1 = self.page_h - y_top1;
        let y2 = self.page_h - y_top2;
        self.push_raw("q [2.5 2.5] 0 d".to_string());
        self.push_raw(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.push_raw(format!("{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S Q"));
    }

    pub fn text(
        &mut self,
        content: &str,
        x: f64,
        y_top: f64,
        font: FontFace,
        size: f64,
        color: Rgb,
    ) {
        let y = self.page_h - y_top;
        let visual = shape_and_reorder_line(content);

        if !visual
            .chars()
            .any(|c| lookup_vector_glyph(c as u32).is_some())
        {
            self.push_raw("BT".to_string());
            self.push_raw(format!("/{} {size:.2} Tf", font.pdf_alias()));
            self.push_raw(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
            self.push_raw(format!("1 0 0 1 {x:.2} {y:.2} Tm"));
            self.push_raw(format!("({}) Tj", pdf_escape_win_ansi(&visual)));
            self.push_raw("ET".to_string());
            return;
        }

        let is_bold = font == FontFace::Bold;
        let mut cur_x = x;
        for run in split_mixed_runs(&visual) {
            match run {
                MixedRun::WinAnsi(seg) => {
                    let seg_w = estimate_font_width_pt(&seg, font, size);
                    self.push_raw("BT".to_string());
                    self.push_raw(format!("/{} {size:.2} Tf", font.pdf_alias()));
                    self.push_raw(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
                    self.push_raw(format!("1 0 0 1 {cur_x:.2} {y:.2} Tm"));
                    self.push_raw(format!("({}) Tj", pdf_escape_win_ansi(&seg)));
                    self.push_raw("ET".to_string());
                    cur_x += seg_w;
                }
                MixedRun::Vector(cps) => {
                    let mut funits: u64 = 0;
                    for &cp in &cps {
                        if let Some(rec) = lookup_vector_glyph(cp) {
                            funits += (if is_bold {
                                rec.bold_adv
                            } else {
                                rec.regular_adv
                            }) as u64;
                        }
                    }
                    let seg_w = ((funits as f64) / VECTOR_UPM) * size;
                    self.ops.push(PageOp::VectorText {
                        bold: is_bold,
                        size,
                        color,
                        x: cur_x,
                        y,
                        cps,
                    });
                    cur_x += seg_w;
                }
            }
        }
    }

    pub fn text_right(
        &mut self,
        content: &str,
        x_right: f64,
        y_top: f64,
        font: FontFace,
        size: f64,
        color: Rgb,
    ) {
        let est = estimate_font_width_pt(content, font, size);
        let x = (x_right - est).max(self.margin_x);
        self.text(content, x, y_top, font, size, color);
    }

    pub fn text_center(
        &mut self,
        content: &str,
        x_center: f64,
        y_top: f64,
        font: FontFace,
        size: f64,
        color: Rgb,
    ) {
        let est = estimate_font_width_pt(content, font, size);
        let x = (x_center - est / 2.0).max(self.margin_x);
        self.text(content, x, y_top, font, size, color);
    }

    /// Renders a crisp vector QR matrix using run-length merged horizontal rectangles (`re f`)
    /// so adjacent dark modules never exhibit sub-pixel anti-aliasing seams in PDF viewers.
    pub fn draw_qr_matrix(
        &mut self,
        qr: &QrMatrix,
        x_left: f64,
        y_top: f64,
        box_size_pt: f64,
        fg: Rgb,
        bg: Rgb,
    ) {
        self.fill_rect(x_left, y_top, box_size_pt, box_size_pt, bg);
        let quiet_modules = 2usize;
        let total_grid = (qr.size + quiet_modules * 2) as f64;
        let mod_pt = box_size_pt / total_grid;

        self.push_raw(format!("{:.3} {:.3} {:.3} rg", fg.0, fg.1, fg.2));

        for r in 0..qr.size {
            let mut c = 0usize;
            while c < qr.size {
                if !qr.get(r, c) {
                    c += 1;
                    continue;
                }
                let start_c = c;
                while c < qr.size && qr.get(r, c) {
                    c += 1;
                }
                let run_len = (c - start_c) as f64;
                let rx = x_left + ((start_c + quiet_modules) as f64) * mod_pt;
                let ry_top = y_top + ((r + quiet_modules) as f64) * mod_pt;
                let ry_pdf = self.page_h - ry_top - mod_pt;
                let rw = run_len * mod_pt;
                self.push_raw(format!("{rx:.3} {ry_pdf:.3} {rw:.3} {mod_pt:.3} re"));
            }
        }
        self.push_raw("f".to_string());
    }

    pub fn draw_jpeg(&mut self, xobj_alias: &str, x: f64, y_top: f64, w: f64, h: f64) {
        let y = self.page_h - y_top - h;
        self.push_raw(format!(
            "q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /{xobj_alias} Do Q"
        ));
    }
}

struct VectorFontBank {
    alias: String,
    pdf_name: String,
    bold: bool,
    cps: Vec<u32>,
}

impl VectorFontBank {
    fn object_count(&self) -> usize {
        // 1 Font dict + 1 ToUnicode CMap stream + N CharProc streams
        2 + self.cps.len()
    }
}

struct VectorSubsetPool {
    banks: Vec<VectorFontBank>,
}

impl VectorSubsetPool {
    fn new() -> Self {
        Self { banks: Vec::new() }
    }

    fn resolve_slot(&mut self, bold: bool, cp: u32) -> (String, u8) {
        for bank in &self.banks {
            if bank.bold == bold {
                if let Some(pos) = bank.cps.iter().position(|&x| x == cp) {
                    return (bank.alias.clone(), (pos + 1) as u8);
                }
            }
        }

        for bank in &mut self.banks {
            if bank.bold == bold && bank.cps.len() < 254 {
                bank.cps.push(cp);
                let code = bank.cps.len() as u8;
                return (bank.alias.clone(), code);
            }
        }

        let same_weight_idx = self.banks.iter().filter(|b| b.bold == bold).count();
        let base_alias = if bold { "F5" } else { "F4" };
        let alias = if same_weight_idx == 0 {
            base_alias.to_string()
        } else {
            format!("{base_alias}_{same_weight_idx}")
        };
        let pdf_name = if bold {
            format!("MsfVectorBold{same_weight_idx}")
        } else {
            format!("MsfVectorRegular{same_weight_idx}")
        };
        self.banks.push(VectorFontBank {
            alias: alias.clone(),
            pdf_name,
            bold,
            cps: vec![cp],
        });
        (alias, 1u8)
    }
}

/// Assembles an ISO 32000-1 `%PDF-1.4` byte buffer with deterministic 7-bit ASCII `xref` byte offsets,
/// dynamically subsetting Cyrillic & Arabic Type 3 vector glyphs (`0` overhead on pure Latin documents).
pub fn assemble_pdf14(pages: &[PdfPageOps], jpegs: &[ValidatedJpeg]) -> Vec<u8> {
    let mut pool = VectorSubsetPool::new();
    let mut resolved_page_streams: Vec<String> = Vec::with_capacity(pages.len().max(1));

    for page in pages {
        let mut lines: Vec<String> = Vec::with_capacity(page.ops.len() + 8);
        for op in &page.ops {
            match op {
                PageOp::Raw(s) => lines.push(s.clone()),
                PageOp::VectorText {
                    bold,
                    size,
                    color,
                    x,
                    y,
                    cps,
                } => {
                    let mut cur_x = *x;
                    let mut idx = 0usize;
                    while idx < cps.len() {
                        let (alias, first_code) = pool.resolve_slot(*bold, cps[idx]);
                        let mut hex = format!("{first_code:02X}");
                        let mut run_funits: u64 = lookup_vector_glyph(cps[idx])
                            .map(|r| if *bold { r.bold_adv } else { r.regular_adv } as u64)
                            .unwrap_or(0);
                        idx += 1;

                        while idx < cps.len() {
                            let (next_alias, next_code) = pool.resolve_slot(*bold, cps[idx]);
                            if next_alias != alias {
                                break;
                            }
                            hex.push_str(&format!("{next_code:02X}"));
                            run_funits += lookup_vector_glyph(cps[idx])
                                .map(|r| if *bold { r.bold_adv } else { r.regular_adv } as u64)
                                .unwrap_or(0);
                            idx += 1;
                        }

                        lines.push("BT".to_string());
                        lines.push(format!("/{alias} {size:.2} Tf"));
                        lines.push(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
                        lines.push(format!("1 0 0 1 {cur_x:.2} {y:.2} Tm"));
                        lines.push(format!("<{hex}> Tj"));
                        lines.push("ET".to_string());
                        cur_x += ((run_funits as f64) / VECTOR_UPM) * (*size);
                    }
                }
            }
        }
        resolved_page_streams.push(format!("{}\n", lines.join("\n")));
    }

    let page_count = pages.len().max(1);
    let img_count = jpegs.len();
    let vector_obj_count: usize = pool.banks.iter().map(VectorFontBank::object_count).sum();
    let first_page_id = 6 + img_count + vector_obj_count;
    let total_objects = 5 + img_count + vector_obj_count + page_count * 2;
    let mut objects: Vec<String> = Vec::with_capacity(total_objects);

    let kids_refs: Vec<String> = (0..page_count)
        .map(|idx| format!("{} 0 R", first_page_id + idx * 2))
        .collect();

    objects.push("1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n".to_string());
    objects.push(format!(
        "2 0 obj\n<< /Type /Pages /Kids [{}] /Count {} >>\nendobj\n",
        kids_refs.join(" "),
        page_count
    ));
    objects.push(
        "3 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica /Encoding /WinAnsiEncoding >>\nendobj\n"
            .to_string(),
    );
    objects.push(
        "4 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica-Bold /Encoding /WinAnsiEncoding >>\nendobj\n"
            .to_string(),
    );
    objects.push(
        "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Courier /Encoding /WinAnsiEncoding >>\nendobj\n"
            .to_string(),
    );

    let mut xobj_dict = String::new();
    if !jpegs.is_empty() {
        xobj_dict.push_str(" /XObject <<");
        for (idx, img) in jpegs.iter().enumerate() {
            let obj_id = 6 + idx;
            xobj_dict.push_str(&format!(" /Im{} {} 0 R", idx + 1, obj_id));
            let stream_body = format!("{}\n", img.ascii_hex_stream);
            let stream_len = stream_body.len();
            objects.push(format!(
                "{obj_id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace {} /BitsPerComponent 8 /Filter [/ASCIIHexDecode /DCTDecode] /Length {stream_len} >>\nstream\n{stream_body}endstream\nendobj\n",
                img.width_px, img.height_px, img.color_space
            ));
        }
        xobj_dict.push_str(" >>");
    }

    let mut extra_font_refs = String::new();
    let mut next_obj_id = 6 + img_count;
    for bank in &pool.banks {
        let font_obj_id = next_obj_id;
        let cmap_obj_id = next_obj_id + 1;
        let first_proc_id = next_obj_id + 2;
        next_obj_id += bank.object_count();

        extra_font_refs.push_str(&format!(" /{} {font_obj_id} 0 R", bank.alias));

        let mut char_procs = Vec::with_capacity(bank.cps.len());
        let mut diffs = Vec::with_capacity(bank.cps.len() + 1);
        diffs.push("1".to_string());
        let mut widths = Vec::with_capacity(bank.cps.len());
        let mut bfchars = Vec::with_capacity(bank.cps.len());

        for (i, &cp) in bank.cps.iter().enumerate() {
            let code = i + 1;
            let proc_id = first_proc_id + i;
            let rec = lookup_vector_glyph(cp).expect("registered vector glyph exists");
            let adv = if bank.bold {
                rec.bold_adv
            } else {
                rec.regular_adv
            };
            char_procs.push(format!("/g{code} {proc_id} 0 R"));
            diffs.push(format!("/g{code}"));
            widths.push(adv.to_string());
            bfchars.push(format!("<{code:02X}> <{}>", rec.to_unicode_hex));
        }

        objects.push(format!(
            "{font_obj_id} 0 obj\n<< /Type /Font /Subtype /Type3 /Name /{} /FontBBox {VECTOR_FONT_BBOX} /FontMatrix {VECTOR_FONT_MATRIX} /CharProcs << {} >> /Encoding << /Type /Encoding /Differences [{}] >> /FirstChar 1 /LastChar {} /Widths [{}] /ToUnicode {cmap_obj_id} 0 R >>\nendobj\n",
            bank.pdf_name,
            char_procs.join(" "),
            diffs.join(" "),
            bank.cps.len(),
            widths.join(" ")
        ));

        let cmap_stream = format!(
            "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n1 begincodespacerange\n<01> <FE>\nendcodespacerange\n{} beginbfchar\n{}\nendbfchar\nendcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n",
            bfchars.len(),
            bfchars.join("\n")
        );
        objects.push(format!(
            "{cmap_obj_id} 0 obj\n<< /Length {} >>\nstream\n{cmap_stream}endstream\nendobj\n",
            cmap_stream.len()
        ));

        for (i, &cp) in bank.cps.iter().enumerate() {
            let proc_id = first_proc_id + i;
            let rec = lookup_vector_glyph(cp).expect("registered vector glyph exists");
            let proc_body = format!(
                "{}\n",
                if bank.bold {
                    rec.bold_proc
                } else {
                    rec.regular_proc
                }
            );
            objects.push(format!(
                "{proc_id} 0 obj\n<< /Length {} >>\nstream\n{proc_body}endstream\nendobj\n",
                proc_body.len()
            ));
        }
    }

    for (idx, page) in pages.iter().enumerate() {
        let page_obj_id = first_page_id + idx * 2;
        let content_obj_id = page_obj_id + 1;
        let stream_body = &resolved_page_streams[idx];
        let stream_len = stream_body.len();

        objects.push(format!(
            "{page_obj_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R{extra_font_refs} >>{xobj_dict} >> /Contents {content_obj_id} 0 R >>\nendobj\n",
            page.page_w, page.page_h
        ));
        objects.push(format!(
            "{content_obj_id} 0 obj\n<< /Length {stream_len} >>\nstream\n{stream_body}endstream\nendobj\n"
        ));
    }

    let mut pdf = String::with_capacity(4096 * page_count + vector_obj_count * 256);
    pdf.push_str("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);
    for obj in &objects {
        offsets.push(pdf.len());
        pdf.push_str(obj);
    }

    let xref_offset = pdf.len();
    pdf.push_str(&format!(
        "xref\n0 {}\n0000000000 65535 f \n",
        objects.len() + 1
    ));
    for off in offsets.iter().skip(1) {
        pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
        objects.len() + 1,
        xref_offset
    ));
    pdf.into_bytes()
}
