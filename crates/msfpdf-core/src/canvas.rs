use crate::encoding::{estimate_font_width_pt, pdf_escape_win_ansi};
use crate::geometry::{FontFace, Rgb};
use crate::jpeg::ValidatedJpeg;
use crate::qr::QrMatrix;

#[derive(Debug, Clone)]
pub struct PdfPageOps {
    pub page_w: f64,
    pub page_h: f64,
    pub margin_x: f64,
    pub ops: Vec<String>,
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

    pub fn fill_rect(&mut self, x: f64, y_top: f64, w: f64, h: f64, color: Rgb) {
        let y = self.page_h - y_top - h;
        self.ops
            .push(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
        self.ops.push(format!("{x:.2} {y:.2} {w:.2} {h:.2} re f"));
    }

    pub fn stroke_rect(&mut self, x: f64, y_top: f64, w: f64, h: f64, line_w: f64, color: Rgb) {
        let y = self.page_h - y_top - h;
        self.ops.push(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.ops.push(format!("{x:.2} {y:.2} {w:.2} {h:.2} re S"));
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
        self.ops.push(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.ops
            .push(format!("{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S"));
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
        self.ops.push("q [2.5 2.5] 0 d".to_string());
        self.ops.push(format!(
            "{line_w:.2} w {:.3} {:.3} {:.3} RG",
            color.0, color.1, color.2
        ));
        self.ops
            .push(format!("{x1:.2} {y1:.2} m {x2:.2} {y2:.2} l S Q"));
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
        self.ops.push("BT".to_string());
        self.ops.push(format!("/{} {size:.2} Tf", font.pdf_alias()));
        self.ops
            .push(format!("{:.3} {:.3} {:.3} rg", color.0, color.1, color.2));
        self.ops.push(format!("1 0 0 1 {x:.2} {y:.2} Tm"));
        self.ops
            .push(format!("({}) Tj", pdf_escape_win_ansi(content)));
        self.ops.push("ET".to_string());
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

        self.ops
            .push(format!("{:.3} {:.3} {:.3} rg", fg.0, fg.1, fg.2));

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
                self.ops
                    .push(format!("{rx:.3} {ry_pdf:.3} {rw:.3} {mod_pt:.3} re"));
            }
        }
        self.ops.push("f".to_string());
    }

    pub fn draw_jpeg(&mut self, xobj_alias: &str, x: f64, y_top: f64, w: f64, h: f64) {
        let y = self.page_h - y_top - h;
        self.ops.push(format!(
            "q {w:.2} 0 0 {h:.2} {x:.2} {y:.2} cm /{xobj_alias} Do Q"
        ));
    }
}

/// Assembles an ISO 32000-1 `%PDF-1.4` byte buffer with deterministic 7-bit ASCII `xref` byte offsets.
pub fn assemble_pdf14(pages: &[PdfPageOps], jpegs: &[ValidatedJpeg]) -> Vec<u8> {
    // Object numbering:
    // 1: Catalog
    // 2: Pages
    // 3: Font F1 (Helvetica /WinAnsiEncoding)
    // 4: Font F2 (Helvetica-Bold /WinAnsiEncoding)
    // 5: Font F3 (Courier /WinAnsiEncoding)
    // 6..(6 + K): Optional JPEG Image XObjects (Im1..ImK)
    // Remaining pairs: Page + Content Stream
    let page_count = pages.len().max(1);
    let img_count = jpegs.len();
    let first_page_id = 6 + img_count;
    let total_objects = 5 + img_count + page_count * 2;
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

    for (idx, page) in pages.iter().enumerate() {
        let page_obj_id = first_page_id + idx * 2;
        let content_obj_id = page_obj_id + 1;
        let stream_body = format!("{}\n", page.ops.join("\n"));
        let stream_len = stream_body.len();

        objects.push(format!(
            "{page_obj_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R >>{xobj_dict} >> /Contents {content_obj_id} 0 R >>\nendobj\n",
            page.page_w, page.page_h
        ));
        objects.push(format!(
            "{content_obj_id} 0 obj\n<< /Length {stream_len} >>\nstream\n{stream_body}endstream\nendobj\n"
        ));
    }

    let mut pdf = String::with_capacity(4096 * page_count);
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
