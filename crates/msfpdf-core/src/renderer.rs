use crate::canvas::{assemble_pdf14, PdfPageOps};
use crate::encoding::{truncate_to_width_pt, wrap_text_to_width_pt};
use crate::error::PdfError;
use crate::geometry::{FontFace, PageFormat, Rgb, ThemePalette};
use crate::jpeg::{parse_and_validate_jpeg_base64, ValidatedJpeg};
use crate::money::format_money_minor;
use crate::qr::encode_qr;
use crate::shaping::is_rtl_dominant;
use crate::spec::{BannerTone, DocumentBlock, DocumentSpec, SecurityLimits, TextAlign};

struct FlowEngine<'a> {
    spec: &'a DocumentSpec,
    limits: SecurityLimits,
    palette: ThemePalette,
    page_w: f64,
    page_h: f64,
    margin_x: f64,
    content_w: f64,
    safe_bottom_y: f64,
    pages: Vec<PdfPageOps>,
    jpegs: Vec<ValidatedJpeg>,
    cursor_y: f64,
}

impl<'a> FlowEngine<'a> {
    fn new(spec: &'a DocumentSpec, limits: SecurityLimits) -> Result<Self, PdfError> {
        if spec.blocks.len() > limits.max_blocks {
            return Err(PdfError::LimitExceeded(
                "document block count exceeds max_blocks ceiling",
            ));
        }

        let mut palette = ThemePalette::from_preset(spec.theme);
        if let Some(ref hex) = spec.accent_hex_override {
            palette.accent = Rgb::from_hex(hex)?;
        }

        let (page_w, page_h) = spec.page_format.dimensions_pt();
        let margin_x = spec.page_format.margin_x_pt();
        let content_w = page_w - margin_x * 2.0;
        let safe_bottom_y = spec.page_format.footer_top_pt() - 10.0;

        let first_page = PdfPageOps::new(page_w, page_h, margin_x);
        let initial_y = if matches!(spec.blocks.first(), Some(DocumentBlock::HeroHeader { .. })) {
            0.0
        } else if spec.page_format == PageFormat::Thermal80mm {
            16.0
        } else {
            36.0
        };

        Ok(Self {
            spec,
            limits,
            palette,
            page_w,
            page_h,
            margin_x,
            content_w,
            safe_bottom_y,
            pages: vec![first_page],
            jpegs: Vec::new(),
            cursor_y: initial_y,
        })
    }

    fn current_page(&mut self) -> &mut PdfPageOps {
        self.pages.last_mut().expect("at least one page exists")
    }

    fn push_continuation_page(&mut self) -> Result<(), PdfError> {
        if self.pages.len() >= self.limits.max_pages {
            return Err(PdfError::LimitExceeded(
                "document pagination exceeded max_pages ceiling",
            ));
        }

        let mut page = PdfPageOps::new(self.page_w, self.page_h, self.margin_x);
        if self.spec.page_format == PageFormat::Thermal80mm {
            page.fill_rect(0.0, 0.0, self.page_w, 28.0, self.palette.header_bg);
            page.fill_rect(0.0, 26.0, self.page_w, 2.0, self.palette.accent);
            let title = truncate_to_width_pt(
                &self.spec.metadata.issuer_name,
                FontFace::Bold,
                8.2,
                self.content_w,
            );
            page.text(
                &title,
                self.margin_x,
                17.0,
                FontFace::Bold,
                8.2,
                self.palette.white,
            );
            self.pages.push(page);
            self.cursor_y = 38.0;
            return Ok(());
        }

        let cont_title = self
            .spec
            .continuation_header
            .as_ref()
            .map(|c| c.title.as_str())
            .unwrap_or(&self.spec.metadata.issuer_name);
        let cont_sub = self
            .spec
            .continuation_header
            .as_ref()
            .map(|c| c.subtitle.as_str())
            .unwrap_or("CONTINUATION SHEET");

        page.fill_rect(0.0, 0.0, self.page_w, 50.0, self.palette.header_bg);
        page.fill_rect(0.0, 47.5, self.page_w, 2.5, self.palette.accent);
        page.text(
            &truncate_to_width_pt(cont_title, FontFace::Bold, 11.0, self.content_w * 0.68),
            self.margin_x,
            23.5,
            FontFace::Bold,
            11.0,
            self.palette.white,
        );
        page.text(
            &truncate_to_width_pt(cont_sub, FontFace::Regular, 7.6, self.content_w * 0.68),
            self.margin_x,
            37.5,
            FontFace::Regular,
            7.6,
            self.palette.accent_bright,
        );
        page.text_right(
            &truncate_to_width_pt(
                &self.spec.metadata.document_code,
                FontFace::Mono,
                8.6,
                self.content_w * 0.30,
            ),
            self.page_w - self.margin_x,
            30.5,
            FontFace::Mono,
            8.6,
            self.palette.white,
        );

        self.pages.push(page);
        self.cursor_y = 68.0;
        Ok(())
    }

    fn ensure_space(&mut self, needed_h: f64) -> Result<(), PdfError> {
        if self.cursor_y + needed_h > self.safe_bottom_y {
            self.push_continuation_page()?;
        }
        Ok(())
    }

    fn draw_section_heading(
        &mut self,
        title: &str,
        keep_with_next_pt: f64,
    ) -> Result<(), PdfError> {
        self.ensure_space(18.0 + keep_with_next_pt)?;
        let y = self.cursor_y + 9.0;
        let max_w = self.content_w - 10.0;
        let label = truncate_to_width_pt(title, FontFace::Bold, 8.8, max_w);
        let pal = self.palette;
        let mx = self.margin_x;
        let p = self.current_page();
        p.fill_rect(mx, y - 8.2, 3.2, 10.2, pal.accent);
        p.text(&label, mx + 8.5, y, FontFace::Bold, 8.8, pal.ink_strong);
        self.cursor_y += 17.5;
        Ok(())
    }

    fn render_block(&mut self, block: &DocumentBlock) -> Result<(), PdfError> {
        match block {
            DocumentBlock::HeroHeader {
                kicker,
                title,
                subtitle,
                badge_label,
                badge_code,
                badge_meta,
                logo_jpeg_base64,
            } => {
                let is_thermal = self.spec.page_format == PageFormat::Thermal80mm;
                let pal = self.palette;
                let mx = self.margin_x;
                let pw = self.page_w;
                let cw = self.content_w;

                let mut logo_alias: Option<(String, f64, f64)> = None;
                if let Some(ref b64) = logo_jpeg_base64 {
                    if self.jpegs.len() >= 8 {
                        return Err(PdfError::LimitExceeded(
                            "maximum of 8 embedded JPEG images per document",
                        ));
                    }
                    let img = parse_and_validate_jpeg_base64(b64)?;
                    let target_h = if is_thermal { 22.0 } else { 28.0 };
                    let aspect = (img.width_px as f64) / (img.height_px as f64);
                    let target_w = (target_h * aspect).clamp(16.0, 90.0);
                    self.jpegs.push(img);
                    logo_alias = Some((format!("Im{}", self.jpegs.len()), target_w, target_h));
                }

                if is_thermal {
                    let banner_h = if badge_code.is_some() { 76.0 } else { 60.0 };
                    let p = self.current_page();
                    p.fill_rect(0.0, 0.0, pw, banner_h, pal.header_bg);
                    p.fill_rect(0.0, banner_h - 2.5, pw, 2.5, pal.accent);
                    p.text_center(
                        &truncate_to_width_pt(kicker, FontFace::Bold, 6.4, cw),
                        pw / 2.0,
                        15.0,
                        FontFace::Bold,
                        6.4,
                        pal.accent_bright,
                    );
                    p.text_center(
                        &truncate_to_width_pt(title, FontFace::Bold, 11.2, cw),
                        pw / 2.0,
                        31.0,
                        FontFace::Bold,
                        11.2,
                        pal.white,
                    );
                    p.text_center(
                        &truncate_to_width_pt(subtitle, FontFace::Regular, 7.2, cw),
                        pw / 2.0,
                        44.0,
                        FontFace::Regular,
                        7.2,
                        pal.header_mist,
                    );
                    if let Some(ref code) = badge_code {
                        p.text_center(
                            &truncate_to_width_pt(code, FontFace::Mono, 7.4, cw),
                            pw / 2.0,
                            61.0,
                            FontFace::Mono,
                            7.4,
                            pal.white,
                        );
                    }
                    self.cursor_y = banner_h + 12.0;
                } else {
                    let banner_h = 88.0;
                    let has_badge = badge_label.is_some() || badge_code.is_some();
                    let badge_w = if has_badge { 172.0 } else { 0.0 };
                    let left_max_w = cw - badge_w - 16.0;

                    let p = self.current_page();
                    p.fill_rect(0.0, 0.0, pw, banner_h, pal.header_bg);
                    p.fill_rect(0.0, banner_h - 3.0, pw, 3.0, pal.accent);

                    let mut text_x = mx;
                    if let Some((alias, lw, lh)) = logo_alias {
                        p.draw_jpeg(&alias, mx, 18.0, lw, lh);
                        text_x += lw + 10.0;
                    }
                    let adj_w = (left_max_w - (text_x - mx)).max(120.0);

                    p.text(
                        &truncate_to_width_pt(kicker, FontFace::Bold, 7.2, adj_w),
                        text_x,
                        23.0,
                        FontFace::Bold,
                        7.2,
                        pal.accent_bright,
                    );
                    p.text(
                        &truncate_to_width_pt(title, FontFace::Bold, 15.2, adj_w),
                        text_x,
                        45.0,
                        FontFace::Bold,
                        15.2,
                        pal.white,
                    );
                    p.text(
                        &truncate_to_width_pt(subtitle, FontFace::Regular, 8.6, adj_w),
                        text_x,
                        63.0,
                        FontFace::Regular,
                        8.6,
                        pal.header_mist,
                    );

                    if has_badge {
                        let bx = pw - mx - badge_w;
                        p.fill_and_stroke_rect(
                            bx,
                            15.0,
                            badge_w,
                            55.0,
                            pal.badge_bg,
                            0.7,
                            pal.badge_border,
                        );
                        if let Some(ref lbl) = badge_label {
                            p.text(
                                &truncate_to_width_pt(lbl, FontFace::Bold, 6.8, badge_w - 18.0),
                                bx + 10.0,
                                28.0,
                                FontFace::Bold,
                                6.8,
                                pal.accent_bright,
                            );
                        }
                        if let Some(ref code) = badge_code {
                            p.text(
                                &truncate_to_width_pt(code, FontFace::Mono, 8.5, badge_w - 18.0),
                                bx + 10.0,
                                44.0,
                                FontFace::Mono,
                                8.5,
                                pal.white,
                            );
                        }
                        if let Some(ref meta) = badge_meta {
                            p.text(
                                &truncate_to_width_pt(meta, FontFace::Regular, 7.4, badge_w - 18.0),
                                bx + 10.0,
                                58.5,
                                FontFace::Regular,
                                7.4,
                                pal.header_mist,
                            );
                        }
                    }

                    self.cursor_y = banner_h + 16.0;
                }
            }

            DocumentBlock::SectionHeading { title } => {
                self.draw_section_heading(title, 36.0)?;
            }

            DocumentBlock::BentoGrid { columns, cards } => {
                if cards.is_empty() {
                    return Ok(());
                }
                let cols = if self.spec.page_format == PageFormat::Thermal80mm {
                    1usize
                } else {
                    (*columns).clamp(1, 3) as usize
                };
                let gap = 12.0;
                let card_w = (self.content_w - gap * ((cols - 1) as f64)) / (cols as f64);

                for row_cards in cards.chunks(cols) {
                    let has_secondary = row_cards
                        .iter()
                        .any(|c| c.secondary_label.is_some() || c.secondary_value.is_some());
                    let card_h = if has_secondary { 70.0 } else { 44.0 };
                    self.ensure_space(card_h + 10.0)?;

                    let y = self.cursor_y;
                    let mx = self.margin_x;
                    let pal = self.palette;
                    let p = self.current_page();

                    for (c_idx, card) in row_cards.iter().enumerate() {
                        let cx = mx + (c_idx as f64) * (card_w + gap);
                        let inner_w = card_w - 20.0;
                        p.fill_and_stroke_rect(cx, y, card_w, card_h, pal.card_bg, 0.6, pal.border);
                        p.fill_rect(cx, y, card_w, 2.2, pal.accent);
                        p.text(
                            &truncate_to_width_pt(
                                &card.primary_label,
                                FontFace::Bold,
                                6.8,
                                inner_w,
                            ),
                            cx + 10.0,
                            y + 15.0,
                            FontFace::Bold,
                            6.8,
                            pal.ink_muted,
                        );
                        p.text(
                            &truncate_to_width_pt(
                                &card.primary_value,
                                FontFace::Bold,
                                10.0,
                                inner_w,
                            ),
                            cx + 10.0,
                            y + 30.5,
                            FontFace::Bold,
                            10.0,
                            pal.ink_strong,
                        );
                        if let Some(ref sec_lbl) = card.secondary_label {
                            p.text(
                                &truncate_to_width_pt(sec_lbl, FontFace::Bold, 6.6, inner_w),
                                cx + 10.0,
                                y + 47.0,
                                FontFace::Bold,
                                6.6,
                                pal.ink_muted,
                            );
                        }
                        if let Some(ref sec_val) = card.secondary_value {
                            p.text(
                                &truncate_to_width_pt(sec_val, FontFace::Regular, 8.4, inner_w),
                                cx + 10.0,
                                y + 60.0,
                                FontFace::Regular,
                                8.4,
                                pal.ink_body,
                            );
                        }
                    }
                    self.cursor_y += card_h + 12.0;
                }
            }

            DocumentBlock::DataTable {
                columns,
                rows,
                zebra,
            } => {
                if columns.is_empty() {
                    return Err(PdfError::InvalidSpec(
                        "data_table requires at least one column".to_string(),
                    ));
                }
                if rows.len() > self.limits.max_table_rows {
                    return Err(PdfError::LimitExceeded(
                        "table row count exceeds max_table_rows ceiling",
                    ));
                }

                let total_weight: f64 = columns.iter().map(|c| c.weight.max(0.2)).sum();
                let mut col_widths = Vec::with_capacity(columns.len());
                let mut col_starts = Vec::with_capacity(columns.len());
                let mut acc_x = self.margin_x;
                for col in columns {
                    col_starts.push(acc_x);
                    let w = (col.weight.max(0.2) / total_weight) * self.content_w;
                    col_widths.push(w);
                    acc_x += w;
                }

                let head_h = 19.0;
                let row_h = 20.5;
                self.ensure_space(head_h + row_h + 8.0)?;

                let mut row_idx = 0usize;
                while row_idx < rows.len() || (rows.is_empty() && row_idx == 0) {
                    let avail = (self.safe_bottom_y - self.cursor_y - head_h).max(row_h);
                    let max_rows_on_page = ((avail / row_h).floor() as usize).max(1);
                    let take_n = if rows.is_empty() {
                        0
                    } else {
                        (rows.len() - row_idx).min(max_rows_on_page)
                    };
                    let seg_h = head_h + (take_n as f64) * row_h;

                    let ty = self.cursor_y;
                    let mx = self.margin_x;
                    let cw = self.content_w;
                    let pal = self.palette;
                    let p = self.current_page();

                    p.fill_rect(mx, ty, cw, head_h, pal.table_head_bg);
                    for (ci, col) in columns.iter().enumerate() {
                        let cx = col_starts[ci];
                        let w = col_widths[ci];
                        let hdr = truncate_to_width_pt(
                            &col.header,
                            FontFace::Bold,
                            6.9,
                            (w - 14.0).max(12.0),
                        );
                        match col.align {
                            TextAlign::Left => p.text(
                                &hdr,
                                cx + 7.0,
                                ty + 12.6,
                                FontFace::Bold,
                                6.9,
                                pal.ink_muted,
                            ),
                            TextAlign::Center => p.text_center(
                                &hdr,
                                cx + w / 2.0,
                                ty + 12.6,
                                FontFace::Bold,
                                6.9,
                                pal.ink_muted,
                            ),
                            TextAlign::Right => p.text_right(
                                &hdr,
                                cx + w - 7.0,
                                ty + 12.6,
                                FontFace::Bold,
                                6.9,
                                pal.ink_muted,
                            ),
                        }
                    }

                    for local_i in 0..take_n {
                        let r_data = &rows[row_idx + local_i];
                        let ry = ty + head_h + (local_i as f64) * row_h;
                        if *zebra && local_i % 2 == 1 {
                            p.fill_rect(mx + 0.4, ry + 0.4, cw - 0.8, row_h - 0.8, pal.card_bg);
                        }
                        p.line(mx, ry, mx + cw, ry, 0.5, pal.border);

                        for (ci, col) in columns.iter().enumerate() {
                            let raw_cell = r_data.get(ci).map(String::as_str).unwrap_or("");
                            let cx = col_starts[ci];
                            let w = col_widths[ci];
                            let face = if col.bold_values {
                                FontFace::Bold
                            } else {
                                FontFace::Regular
                            };
                            let color = if col.bold_values {
                                pal.ink_strong
                            } else {
                                pal.ink_body
                            };
                            let val =
                                truncate_to_width_pt(raw_cell, face, 8.5, (w - 14.0).max(12.0));
                            match col.align {
                                TextAlign::Left => {
                                    p.text(&val, cx + 7.0, ry + 13.6, face, 8.5, color)
                                }
                                TextAlign::Center => {
                                    p.text_center(&val, cx + w / 2.0, ry + 13.6, face, 8.5, color)
                                }
                                TextAlign::Right => {
                                    p.text_right(&val, cx + w - 7.0, ry + 13.6, face, 8.5, color)
                                }
                            }
                        }
                    }

                    for &cx in col_starts.iter().skip(1) {
                        p.line(cx, ty, cx, ty + seg_h, 0.45, pal.border);
                    }
                    p.stroke_rect(mx, ty, cw, seg_h, 0.65, pal.border);

                    self.cursor_y += seg_h + 12.0;
                    if rows.is_empty() {
                        break;
                    }
                    row_idx += take_n;
                    if row_idx < rows.len() {
                        self.push_continuation_page()?;
                    }
                }
            }

            DocumentBlock::FinancialSummary { currency, rows } => {
                if rows.is_empty() {
                    return Ok(());
                }
                if rows.len() > self.limits.max_table_rows {
                    return Err(PdfError::LimitExceeded(
                        "financial_summary rows exceed max_table_rows ceiling",
                    ));
                }

                let row_h = 21.5;
                let total_h = row_h * (rows.len() as f64);
                self.ensure_space(total_h.min(120.0))?;

                let mut idx = 0usize;
                while idx < rows.len() {
                    let avail = (self.safe_bottom_y - self.cursor_y).max(row_h);
                    let max_here = ((avail / row_h).floor() as usize).max(1);
                    let take_n = (rows.len() - idx).min(max_here);
                    let seg_h = (take_n as f64) * row_h;

                    let ty = self.cursor_y;
                    let mx = self.margin_x;
                    let cw = self.content_w;
                    let pal = self.palette;
                    let p = self.current_page();

                    for local_i in 0..take_n {
                        let item = &rows[idx + local_i];
                        let ry = ty + (local_i as f64) * row_h;
                        if local_i % 2 == 0 {
                            p.fill_rect(mx + 0.4, ry + 0.4, cw - 0.8, row_h - 0.8, pal.card_bg);
                        }
                        if local_i > 0 {
                            p.line(mx, ry, mx + cw, ry, 0.5, pal.border);
                        }

                        let formatted_val = if let Some(minor) = item.amount_minor {
                            format_money_minor(minor, item.currency.unwrap_or(*currency))
                        } else {
                            item.text_value.clone().unwrap_or_else(|| "-".to_string())
                        };

                        let lbl =
                            truncate_to_width_pt(&item.label, FontFace::Regular, 8.8, cw * 0.64);
                        let val_face = if item.emphasize {
                            FontFace::Bold
                        } else {
                            FontFace::Regular
                        };
                        let val_size = if item.emphasize { 9.6 } else { 8.8 };
                        let val_str =
                            truncate_to_width_pt(&formatted_val, val_face, val_size, cw * 0.32);

                        p.text(
                            &lbl,
                            mx + 10.0,
                            ry + 14.2,
                            FontFace::Regular,
                            8.8,
                            pal.ink_body,
                        );
                        p.text_right(
                            &val_str,
                            mx + cw - 10.0,
                            ry + 14.2,
                            val_face,
                            val_size,
                            pal.ink_strong,
                        );
                    }
                    p.stroke_rect(mx, ty, cw, seg_h, 0.65, pal.border);

                    self.cursor_y += seg_h + 10.0;
                    idx += take_n;
                    if idx < rows.len() {
                        self.push_continuation_page()?;
                    }
                }
            }

            DocumentBlock::CalloutBanner {
                title,
                subtitle,
                right_value,
                tone,
            } => {
                let is_thermal = self.spec.page_format == PageFormat::Thermal80mm;
                let box_h = if is_thermal && right_value.is_some() {
                    54.0
                } else {
                    44.0
                };
                self.ensure_space(box_h + 10.0)?;

                let pal = self.palette;
                let (bg, bar) = match tone {
                    BannerTone::Accent => (pal.callout_bg, pal.accent),
                    BannerTone::Warning => (pal.warning_bg, pal.warning_accent),
                    BannerTone::Danger => (pal.danger_bg, pal.danger_accent),
                    BannerTone::Neutral => (pal.card_bg, pal.ink_muted),
                };

                let cy = self.cursor_y;
                let mx = self.margin_x;
                let cw = self.content_w;
                let p = self.current_page();

                p.fill_and_stroke_rect(mx, cy, cw, box_h, bg, 0.85, bar);
                p.fill_rect(mx, cy, 4.0, box_h, bar);

                if is_thermal {
                    p.text(
                        &truncate_to_width_pt(title, FontFace::Bold, 7.6, cw - 18.0),
                        mx + 10.0,
                        cy + 14.0,
                        FontFace::Bold,
                        7.6,
                        bar,
                    );
                    p.text(
                        &truncate_to_width_pt(subtitle, FontFace::Regular, 7.2, cw - 18.0),
                        mx + 10.0,
                        cy + 26.5,
                        FontFace::Regular,
                        7.2,
                        pal.ink_body,
                    );
                    if let Some(ref rv) = right_value {
                        p.text(
                            &truncate_to_width_pt(rv, FontFace::Bold, 10.5, cw - 18.0),
                            mx + 10.0,
                            cy + 43.0,
                            FontFace::Bold,
                            10.5,
                            pal.ink_strong,
                        );
                    }
                } else {
                    let left_limit = if right_value.is_some() {
                        cw * 0.60
                    } else {
                        cw - 24.0
                    };
                    p.text(
                        &truncate_to_width_pt(title, FontFace::Bold, 8.2, left_limit),
                        mx + 13.0,
                        cy + 16.5,
                        FontFace::Bold,
                        8.2,
                        bar,
                    );
                    p.text(
                        &truncate_to_width_pt(subtitle, FontFace::Regular, 7.8, left_limit),
                        mx + 13.0,
                        cy + 31.0,
                        FontFace::Regular,
                        7.8,
                        pal.ink_muted,
                    );
                    if let Some(ref rv) = right_value {
                        p.text_right(
                            &truncate_to_width_pt(rv, FontFace::Bold, 12.4, cw * 0.36),
                            mx + cw - 12.0,
                            cy + 27.0,
                            FontFace::Bold,
                            12.4,
                            pal.ink_strong,
                        );
                    }
                }

                self.cursor_y += box_h + 12.0;
            }

            DocumentBlock::ClauseList { paragraphs } => {
                let inner_w = self.content_w - 20.0;
                let font_sz = if self.spec.page_format == PageFormat::Thermal80mm {
                    7.4
                } else {
                    8.4
                };
                let line_step = if self.spec.page_format == PageFormat::Thermal80mm {
                    11.0
                } else {
                    13.0
                };

                let mut wrapped: Vec<(String, bool)> = Vec::new();
                for para in paragraphs {
                    let rtl = is_rtl_dominant(para);
                    for ln in wrap_text_to_width_pt(para, FontFace::Regular, font_sz, inner_w) {
                        wrapped.push((ln, rtl));
                    }
                }
                if wrapped.is_empty() {
                    return Ok(());
                }

                self.ensure_space(line_step * 2.0 + 18.0)?;
                let mut idx = 0usize;
                while idx < wrapped.len() {
                    let avail = (self.safe_bottom_y - self.cursor_y - 16.0).max(line_step);
                    let max_lines = ((avail / line_step).floor() as usize).max(1);
                    let take_n = (wrapped.len() - idx).min(max_lines);
                    let box_h = (take_n as f64) * line_step + 14.0;

                    let cy = self.cursor_y;
                    let mx = self.margin_x;
                    let cw = self.content_w;
                    let pal = self.palette;
                    let p = self.current_page();

                    p.fill_and_stroke_rect(mx, cy, cw, box_h, pal.card_bg, 0.6, pal.border);
                    for (offset, (line_str, rtl)) in wrapped[idx..idx + take_n].iter().enumerate() {
                        let ly = cy + 12.5 + (offset as f64) * line_step;
                        if *rtl {
                            p.text_right(
                                line_str,
                                mx + cw - 10.0,
                                ly,
                                FontFace::Regular,
                                font_sz,
                                pal.ink_body,
                            );
                        } else {
                            p.text(
                                line_str,
                                mx + 10.0,
                                ly,
                                FontFace::Regular,
                                font_sz,
                                pal.ink_body,
                            );
                        }
                    }

                    self.cursor_y += box_h + 12.0;
                    idx += take_n;
                    if idx < wrapped.len() {
                        self.push_continuation_page()?;
                    }
                }
            }

            DocumentBlock::QrVerification {
                payload,
                headline,
                details,
                code_display,
            } => {
                let qr = encode_qr(payload)?;
                let is_thermal = self.spec.page_format == PageFormat::Thermal80mm;
                let pal = self.palette;
                let mx = self.margin_x;
                let cw = self.content_w;

                if is_thermal {
                    let qr_box = 86.0;
                    let detail_lines =
                        wrap_text_to_width_pt(details, FontFace::Regular, 7.0, cw - 16.0);
                    let total_h = 28.0 + qr_box + (detail_lines.len() as f64) * 10.0 + 14.0;
                    self.ensure_space(total_h)?;

                    let cy = self.cursor_y;
                    let pw = self.page_w;
                    let p = self.current_page();
                    p.fill_and_stroke_rect(mx, cy, cw, total_h, pal.card_bg, 0.6, pal.border);
                    p.text_center(
                        &truncate_to_width_pt(headline, FontFace::Bold, 7.6, cw - 12.0),
                        pw / 2.0,
                        cy + 14.0,
                        FontFace::Bold,
                        7.6,
                        pal.ink_strong,
                    );
                    p.draw_qr_matrix(
                        &qr,
                        (pw - qr_box) / 2.0,
                        cy + 20.0,
                        qr_box,
                        pal.ink_strong,
                        pal.white,
                    );
                    for (li, ln) in detail_lines.iter().enumerate() {
                        p.text_center(
                            ln,
                            pw / 2.0,
                            cy + 24.0 + qr_box + 8.0 + (li as f64) * 10.0,
                            FontFace::Regular,
                            7.0,
                            pal.ink_muted,
                        );
                    }
                    self.cursor_y += total_h + 10.0;
                } else {
                    let card_h = 74.0;
                    let qr_size = 58.0;
                    self.ensure_space(card_h + 10.0)?;

                    let cy = self.cursor_y;
                    let text_x = mx + qr_size + 18.0;
                    let text_w = (cw - qr_size - 28.0).max(80.0);
                    let detail_lines =
                        wrap_text_to_width_pt(details, FontFace::Regular, 7.8, text_w);

                    let p = self.current_page();
                    p.fill_and_stroke_rect(mx, cy, cw, card_h, pal.card_bg, 0.6, pal.border);
                    p.stroke_rect(mx + 8.0, cy + 8.0, qr_size, qr_size, 0.5, pal.border);
                    p.draw_qr_matrix(
                        &qr,
                        mx + 8.5,
                        cy + 8.5,
                        qr_size - 1.0,
                        pal.ink_strong,
                        pal.white,
                    );

                    p.text(
                        &truncate_to_width_pt(headline, FontFace::Bold, 8.6, text_w),
                        text_x,
                        cy + 20.0,
                        FontFace::Bold,
                        8.6,
                        pal.ink_strong,
                    );
                    for (li, ln) in detail_lines.iter().take(2).enumerate() {
                        p.text(
                            ln,
                            text_x,
                            cy + 34.0 + (li as f64) * 11.2,
                            FontFace::Regular,
                            7.8,
                            pal.ink_body,
                        );
                    }
                    if let Some(ref code) = code_display {
                        p.text(
                            &truncate_to_width_pt(code, FontFace::Mono, 7.8, text_w),
                            text_x,
                            cy + 61.0,
                            FontFace::Mono,
                            7.8,
                            pal.accent,
                        );
                    }

                    self.cursor_y += card_h + 12.0;
                }
            }

            DocumentBlock::SignatureGrid { heading, signers } => {
                if signers.is_empty() {
                    return Ok(());
                }
                let needed = if heading.is_some() { 76.0 } else { 56.0 };
                self.ensure_space(needed)?;

                if let Some(ref h) = heading {
                    self.draw_section_heading(h, 46.0)?;
                }

                let cols = if self.spec.page_format == PageFormat::Thermal80mm {
                    1usize
                } else {
                    signers.len().clamp(1, 4)
                };

                for row_signers in signers.chunks(cols) {
                    self.ensure_space(52.0)?;
                    let gap = 16.0;
                    let col_w = (self.content_w - gap * ((row_signers.len() - 1) as f64))
                        / (row_signers.len() as f64);
                    let line_y = self.cursor_y + 26.0;
                    let mx = self.margin_x;
                    let pal = self.palette;
                    let p = self.current_page();

                    for (si, signer) in row_signers.iter().enumerate() {
                        let cx = mx + (si as f64) * (col_w + gap);
                        p.line(cx, line_y, cx + col_w, line_y, 0.7, pal.signature_line);
                        p.text(
                            &truncate_to_width_pt(
                                &signer.role_label,
                                FontFace::Bold,
                                7.0,
                                col_w - 4.0,
                            ),
                            cx,
                            line_y + 11.5,
                            FontFace::Bold,
                            7.0,
                            pal.ink_muted,
                        );
                        p.text(
                            &truncate_to_width_pt(
                                &signer.party_name,
                                FontFace::Regular,
                                8.2,
                                col_w - 4.0,
                            ),
                            cx,
                            line_y + 23.0,
                            FontFace::Regular,
                            8.2,
                            pal.ink_strong,
                        );
                    }
                    self.cursor_y += 54.0;
                }
            }

            DocumentBlock::KeyValueList {
                rows,
                dashed_dividers,
            } => {
                if rows.is_empty() {
                    return Ok(());
                }
                if rows.len() > self.limits.max_table_rows {
                    return Err(PdfError::LimitExceeded(
                        "key_value_list rows exceed max_table_rows ceiling",
                    ));
                }

                let row_h = 16.5;
                for (ri, item) in rows.iter().enumerate() {
                    self.ensure_space(row_h + 4.0)?;
                    let ry = self.cursor_y;
                    let mx = self.margin_x;
                    let cw = self.content_w;
                    let pal = self.palette;
                    let p = self.current_page();

                    if ri > 0 && *dashed_dividers {
                        p.dashed_line(mx, ry, mx + cw, ry, 0.45, pal.border);
                    }
                    let face = if item.emphasize {
                        FontFace::Bold
                    } else {
                        FontFace::Regular
                    };
                    let val_trunc = truncate_to_width_pt(&item.value, face, 8.2, cw * 0.38);
                    let val_w = crate::encoding::estimate_font_width_pt(&val_trunc, face, 8.2);
                    let lbl_max_w = (cw - val_w - 10.0).max(cw * 0.52);
                    p.text(
                        &truncate_to_width_pt(&item.label, face, 8.0, lbl_max_w),
                        mx,
                        ry + 11.5,
                        face,
                        8.0,
                        pal.ink_body,
                    );
                    p.text_right(&val_trunc, mx + cw, ry + 11.5, face, 8.2, pal.ink_strong);
                    self.cursor_y += row_h;
                }
                self.cursor_y += 8.0;
            }

            DocumentBlock::Divider => {
                self.ensure_space(12.0)?;
                let y = self.cursor_y + 4.0;
                let mx = self.margin_x;
                let cw = self.content_w;
                let border = self.palette.border;
                self.current_page().line(mx, y, mx + cw, y, 0.6, border);
                self.cursor_y += 10.0;
            }

            DocumentBlock::Spacer { height_pt } => {
                let h = height_pt.clamp(2.0, 200.0);
                self.ensure_space(h)?;
                self.cursor_y += h;
            }
        }

        Ok(())
    }

    fn finalize(mut self) -> Vec<u8> {
        let total = self.pages.len();
        let footer_y = self.spec.page_format.footer_top_pt();
        let is_thermal = self.spec.page_format == PageFormat::Thermal80mm;

        let default_notice = format!(
            "{}  •  Ref {}  •  {}",
            self.spec.metadata.issuer_name,
            self.spec.metadata.document_code,
            self.spec.metadata.issued_at_label
        );
        let notice = self
            .spec
            .metadata
            .footer_notice
            .as_deref()
            .unwrap_or(&default_notice);

        for (idx, page) in self.pages.iter_mut().enumerate() {
            page.line(
                self.margin_x,
                footer_y,
                self.page_w - self.margin_x,
                footer_y,
                0.55,
                self.palette.border,
            );

            let page_str = self
                .spec
                .metadata
                .page_counter_template
                .replace("{current}", &(idx + 1).to_string())
                .replace("{total}", &total.to_string());

            if is_thermal {
                page.text_center(
                    &truncate_to_width_pt(
                        &format!("{} • {}", self.spec.metadata.document_code, page_str),
                        FontFace::Regular,
                        6.8,
                        self.content_w,
                    ),
                    self.page_w / 2.0,
                    footer_y + 13.0,
                    FontFace::Regular,
                    6.8,
                    self.palette.ink_muted,
                );
            } else {
                page.text(
                    &truncate_to_width_pt(notice, FontFace::Regular, 7.3, self.content_w * 0.76),
                    self.margin_x,
                    footer_y + 14.0,
                    FontFace::Regular,
                    7.3,
                    self.palette.ink_muted,
                );
                page.text_right(
                    &truncate_to_width_pt(&page_str, FontFace::Bold, 7.5, self.content_w * 0.22),
                    self.page_w - self.margin_x,
                    footer_y + 14.0,
                    FontFace::Bold,
                    7.5,
                    self.palette.ink_muted,
                );
            }
        }

        assemble_pdf14(&self.pages, &self.jpegs)
    }
}

/// Renders a validated `DocumentSpec` into a standalone `%PDF-1.4` byte buffer.
pub fn render_document(spec: &DocumentSpec, limits: SecurityLimits) -> Result<Vec<u8>, PdfError> {
    let mut engine = FlowEngine::new(spec, limits)?;
    for block in &spec.blocks {
        engine.render_block(block)?;
    }
    Ok(engine.finalize())
}

/// Parses a JSON `DocumentSpec` enforcing `limits.max_input_bytes` before deserialization,
/// then compiles it into `%PDF-1.4` bytes.
pub fn render_document_json(
    json_bytes: &[u8],
    limits: SecurityLimits,
) -> Result<Vec<u8>, PdfError> {
    if json_bytes.len() > limits.max_input_bytes {
        return Err(PdfError::LimitExceeded(
            "JSON payload exceeds max_input_bytes ceiling",
        ));
    }
    let spec: DocumentSpec = serde_json::from_slice(json_bytes)
        .map_err(|e| PdfError::InvalidSpec(format!("malformed JSON specification: {e}")))?;
    render_document(&spec, limits)
}
