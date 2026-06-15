use crate::text::types::{GlyphPosition, GlyphRasterConfig, LineRec, TextCluster};
use crate::text::SampleTextSystem;
use framewise::{
    EllipsisFallback, FontId, LineEndKind, LineHeight, OverflowX, OverflowY, Rect, ShapedCluster,
    ShapedGlyph, ShapedText, TextFlow, TextLineAlign, TextMetrics, TextStyle, Vec2,
    WrapClusterFallback, WrapWordFallback,
};

struct Line {
    clusters: Vec<OwnedCluster>,
    byte_start: usize,
    byte_end: usize,
    baseline_y: f32,
}

#[derive(Debug, Clone)]
struct OwnedCluster {
    byte_start: usize,
    byte_end: usize,
    x: f32,
    advance: f32,
    is_hard_break: bool,
    is_whitespace: bool,
    is_soft_wrap_boundary: bool,
    glyphs: Vec<GlyphPosition>,
}

impl OwnedCluster {
    fn end_x(&self) -> f32 {
        self.x + self.advance
    }

    fn shift_x(&mut self, dx: f32) {
        self.x += dx;
        for g in &mut self.glyphs {
            g.x += dx;
        }
    }

    fn collapse_soft_wrap_boundary(&mut self) {
        self.advance = 0.0;
        self.is_soft_wrap_boundary = true;
        for glyph in &mut self.glyphs {
            glyph.advance = 0.0;
        }
    }
}

impl SampleTextSystem {
    /// Get the weight for a given font ID.
    /// For variable fonts, this returns the currently set weight.
    pub fn weight_for_font(&self, font_id: FontId) -> u16 {
        self.font_weights
            .get(font_id.0 as usize)
            .copied()
            .unwrap_or(400)
    }

    /// Get the optical size value for a given pixel size and font.
    /// Returns 0.0 if the font doesn't have opsz axis.
    pub fn opsz_for_size(&self, size: f32, font_id: FontId) -> f32 {
        let (min_opsz, max_opsz) = self.font_opsz_ranges[font_id.0 as usize];
        if max_opsz == 0.0 {
            // Font has no opsz axis
            return 0.0;
        }
        // Clamp size to the font's opsz range
        size.clamp(min_opsz, max_opsz)
    }

    pub fn line_height(&self, size: f32, font_id: FontId, line_height_style: LineHeight) -> f32 {
        match line_height_style {
            LineHeight::Normal => {
                let font = self.fonts[font_id.0 as usize];

                // For now, get metrics without variations - they should be similar enough
                // TODO: Consider if we need to normalize coords for metrics
                let metrics = font.metrics(&[]);
                let units_per_em = metrics.units_per_em as f32;
                let scale = size / units_per_em;
                let ascent = metrics.ascent * scale;
                let descent = (metrics.descent * scale).abs();
                let line_gap = metrics.leading * scale;
                ascent + descent + line_gap
            }
            LineHeight::Relative(mult) => size * mult,
        }
    }

    pub fn ellipsis(&mut self, size: f32, font_id: FontId) -> (Vec<GlyphPosition>, f32, f32) {
        let flow = TextFlow::single_line();
        let style = TextStyle::new(font_id, size, self.weight_for_font(font_id), flow);
        let (glyphs, _clusters, _lines, _metrics) =
            self.shape_internal("…", style, None, None, Some(0.0));

        let logical_w = Self::logical_line_width(&glyphs);

        let font = self.fonts[font_id.0 as usize];
        let metrics = font.metrics(&[]);
        let baseline = (metrics.ascent * size / metrics.units_per_em as f32).round();
        (glyphs, logical_w, baseline)
    }

    pub fn shape_text_run(&mut self, text: &str, style: TextStyle) -> ShapedText<u16> {
        let font_id = style.font;
        let size = style.size;
        let weight = style.weight;
        let opsz = self.opsz_for_size(size, font_id);
        let letter_spacing_px = size * style.letter_spacing;
        let font = self.fonts[font_id.0 as usize];
        let mut shaper = self.shape_context.builder(font).size(size);

        let mut vars = Vec::new();
        if self.font_has_wght[font_id.0 as usize] {
            vars.push(("wght", weight as f32));
        }
        if self.font_has_opsz[font_id.0 as usize] && opsz > 0.0 {
            vars.push(("opsz", opsz));
        }
        if !vars.is_empty() {
            shaper = shaper.variations(&vars);
        }

        let mut shaper = shaper.build();
        shaper.add_str(text);

        let mut clusters = Vec::new();
        let mut pen_x = 0.0_f32;
        shaper.shape_with(|cluster| {
            let source = cluster.source.to_range();
            let source_text = &text[source.clone()];
            let byte_start = cluster.source.start as usize;
            let byte_end = cluster.source.end as usize;
            let is_whitespace = source_text.chars().all(char::is_whitespace);
            let cluster_x = pen_x;
            let mut cluster_advance = 0.0;
            let mut glyphs = Vec::new();

            for glyph in cluster.glyphs {
                let advance = glyph.advance + letter_spacing_px;
                glyphs.push(ShapedGlyph {
                    id: glyph.id,
                    x: pen_x - cluster_x + glyph.x,
                    y: glyph.y,
                    advance,
                });
                pen_x += advance;
                cluster_advance += advance;
            }

            if glyphs.is_empty() {
                glyphs.push(ShapedGlyph {
                    id: 0,
                    x: 0.0,
                    y: 0.0,
                    advance: 0.0,
                });
            }

            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance: cluster_advance,
                is_whitespace,
                glyphs,
            });
        });

        ShapedText { clusters }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_internal(
        &mut self,
        text: &str,
        style: TextStyle,
        max_w: Option<f32>,
        max_h: Option<f32>,
        absolute_x: Option<f32>,
    ) -> (
        Vec<GlyphPosition>,
        Vec<TextCluster>,
        Vec<LineRec>,
        TextMetrics,
    ) {
        let size = style.size;
        let font_id = style.font;
        let flow = style.flow;
        let line_height = self.line_height(size, font_id, style.line_height);
        let letter_spacing_px = size * style.letter_spacing;
        let font = self.fonts[font_id.0 as usize];
        let weight = self.weight_for_font(font_id);
        let opsz = self.opsz_for_size(size, font_id);

        let metrics = font.metrics(&[]);
        let scale = size / metrics.units_per_em as f32;
        let ascent = metrics.ascent * scale;

        // Snapping baseline offset to integer logical pixel
        let baseline_offset = ascent.round();
        let line_height_snapped = line_height.round();

        // ── Partition input by hard newlines ────────────────────────────────
        let mut lines_raw = Vec::new();
        let mut start_byte = 0;
        for (idx, c) in text.char_indices() {
            if c == '\n' {
                lines_raw.push((&text[start_byte..idx], true, start_byte, idx));
                start_byte = idx + 1;
            }
        }
        if start_byte <= text.len() {
            lines_raw.push((&text[start_byte..], false, start_byte, text.len()));
        }

        let mut lines = Vec::new();

        for (i, &(segment, has_newline, segment_start, segment_end)) in lines_raw.iter().enumerate()
        {
            let mut pen_x = 0.0_f32;
            let mut clusters = Vec::new();
            let baseline_y = i as f32 * line_height_snapped + baseline_offset;

            if !segment.is_empty() {
                let mut shaper = self.shape_context.builder(font).size(size);

                // Apply variation settings if font supports them
                let mut vars = Vec::new();
                if self.font_has_wght[font_id.0 as usize] {
                    vars.push(("wght", weight as f32));
                }
                if self.font_has_opsz[font_id.0 as usize] && opsz > 0.0 {
                    vars.push(("opsz", opsz));
                }
                if !vars.is_empty() {
                    shaper = shaper.variations(&vars);
                }

                let mut shaper = shaper.build();
                shaper.add_str(segment);

                shaper.shape_with(|cluster| {
                    let source = cluster.source.to_range();
                    let source_text = &segment[source.clone()];
                    let byte_start = segment_start + cluster.source.start as usize;
                    let byte_end = segment_start + cluster.source.end as usize;
                    let parent_char = source_text.chars().next().unwrap_or(' ');
                    let is_whitespace = source_text.chars().all(char::is_whitespace);
                    let cluster_x = pen_x;
                    let mut cluster_advance = 0.0;
                    let mut glyphs = Vec::new();

                    for glyph in cluster.glyphs {
                        let advance = glyph.advance + letter_spacing_px;
                        let gx = pen_x + glyph.x;

                        glyphs.push(GlyphPosition {
                            parent: parent_char,
                            key: GlyphRasterConfig {
                                glyph_index: glyph.id,
                                px: size,
                            },
                            x: gx,
                            y: baseline_y + glyph.y,
                            raster_w: 0,
                            raster_h: 0,
                            byte_offset: byte_start,
                            subpixel_x: 0,
                            advance,
                            weight,
                            opsz: opsz as u16,
                        });

                        pen_x += advance;
                        cluster_advance += advance;
                    }

                    if glyphs.is_empty() {
                        glyphs.push(GlyphPosition {
                            parent: parent_char,
                            key: GlyphRasterConfig {
                                glyph_index: 0,
                                px: size,
                            },
                            x: cluster_x,
                            y: baseline_y,
                            raster_w: 0,
                            raster_h: 0,
                            byte_offset: byte_start,
                            subpixel_x: 0,
                            advance: 0.0,
                            weight,
                            opsz: opsz as u16,
                        });
                    }

                    clusters.push(OwnedCluster {
                        byte_start,
                        byte_end,
                        x: cluster_x,
                        advance: cluster_advance,
                        is_hard_break: false,
                        is_whitespace,
                        is_soft_wrap_boundary: false,
                        glyphs,
                    });
                });

                for cluster in &mut clusters {
                    for glyph in &mut cluster.glyphs {
                        let abs_x = absolute_x.unwrap_or(0.0) + glyph.x;
                        glyph.subpixel_x = (abs_x.fract() * 4.0).round() as u8 % 4;
                        let (w, h) = self.get_glyph_metrics(
                            font_id.0,
                            glyph.key.glyph_index,
                            size,
                            glyph.subpixel_x,
                            weight,
                            opsz as u16,
                        );
                        glyph.raster_w = w as usize;
                        glyph.raster_h = h as usize;
                    }
                }
            }

            if has_newline {
                let newline_glyph = GlyphPosition {
                    parent: '\n',
                    key: GlyphRasterConfig {
                        glyph_index: 0,
                        px: size,
                    },
                    x: pen_x,
                    y: baseline_y,
                    raster_w: 0,
                    raster_h: 0,
                    byte_offset: segment_end,
                    subpixel_x: 0,
                    advance: 0.0,
                    weight,
                    opsz: opsz as u16,
                };
                clusters.push(OwnedCluster {
                    byte_start: segment_end,
                    byte_end: segment_end + 1,
                    x: pen_x,
                    advance: 0.0,
                    is_hard_break: true,
                    is_whitespace: true,
                    is_soft_wrap_boundary: false,
                    glyphs: vec![newline_glyph],
                });
            }

            let byte_end = if has_newline {
                segment_end + 1
            } else {
                segment_end
            };

            lines.push(Line {
                clusters,
                byte_start: segment_start,
                byte_end,
                baseline_y,
            });
        }

        // ── Per-line: align, clip / ellipsis, rebuild glyph vec ─────────────
        let global_line_start = if lines.iter().all(|line| line.clusters.is_empty()) {
            0.0
        } else {
            lines
                .iter()
                .flat_map(|line| line.clusters.iter().map(|cluster| cluster.x))
                .fold(f32::INFINITY, f32::min)
        };

        struct ProcessedLine {
            clusters: Vec<OwnedCluster>,
            byte_start: usize,
            byte_end: usize,
            baseline_y: f32,
            end_kind: LineEndKind,
        }

        let mut truncated_horizontal = false;
        let mut processed_lines: Vec<ProcessedLine> = Vec::new();

        for line in lines {
            let mut seg = line.clusters;

            let line_start = Self::logical_cluster_line_start(&seg);
            let logical_line_w = Self::logical_cluster_line_width(&seg);

            let base_shift = match flow.line_align {
                TextLineAlign::Start => global_line_start,
                TextLineAlign::Center | TextLineAlign::End => line_start,
            };
            if base_shift != 0.0 {
                for cluster in &mut seg {
                    cluster.shift_x(-base_shift);
                }
            }

            let mut final_sublines = Vec::new();
            let mut overflow_line_end_kind = None;

            if let Some(w) = max_w {
                match flow.overflow_x {
                    OverflowX::WrapWord { fallback } => {
                        let wrapped = Self::wrap_clusters_at_words(seg, w, fallback);
                        final_sublines.extend(wrapped);
                    }
                    OverflowX::WrapCluster { fallback } => {
                        let wrapped = Self::wrap_clusters(seg, w, fallback);
                        final_sublines.extend(wrapped);
                    }
                    _ => {
                        let overflows_w = logical_line_w > w + 0.5;
                        if overflows_w {
                            truncated_horizontal = true;
                            match flow.overflow_x {
                                OverflowX::Ellipsis { fallback } => {
                                    overflow_line_end_kind = Some(LineEndKind::EllipsisX);
                                    let ellipsised = self.apply_ellipsis_x(
                                        seg,
                                        w,
                                        size,
                                        font_id,
                                        fallback,
                                        line.baseline_y,
                                    );
                                    final_sublines.push(ellipsised);
                                }
                                OverflowX::Keep => {
                                    overflow_line_end_kind = Some(LineEndKind::OverflowKeep);
                                    let mut out = Vec::new();
                                    for cluster in seg {
                                        let end_x = cluster.end_x();
                                        out.push(cluster);
                                        if end_x > w {
                                            break;
                                        }
                                    }
                                    final_sublines.push(out);
                                }
                                OverflowX::Drop => {
                                    overflow_line_end_kind = Some(LineEndKind::OverflowDrop);
                                    let mut out = Vec::new();
                                    for cluster in seg {
                                        if cluster.end_x() <= w {
                                            out.push(cluster);
                                        } else {
                                            break;
                                        }
                                    }
                                    final_sublines.push(out);
                                }
                                _ => unreachable!(),
                            }
                        } else {
                            final_sublines.push(seg);
                        }
                    }
                }
            } else {
                final_sublines.push(seg);
            }

            Self::append_empty_after_terminal_soft_wrap_boundary(
                &mut final_sublines,
                line.byte_end,
            );

            let mut sub_infos = Vec::new();
            let mut previous_end = line.byte_start;
            for (j, sub_seg) in final_sublines.iter().enumerate() {
                let b_start = if j == 0 {
                    line.byte_start
                } else {
                    sub_seg
                        .first()
                        .map(|cluster| cluster.byte_start)
                        .unwrap_or(previous_end)
                };
                previous_end = sub_seg
                    .last()
                    .map(|cluster| cluster.byte_end)
                    .unwrap_or(b_start);
                sub_infos.push(b_start);
            }
            sub_infos.push(line.byte_end);

            for (j, sub_seg) in final_sublines.into_iter().enumerate() {
                let end_kind = overflow_line_end_kind.unwrap_or_else(|| {
                    if sub_seg.last().is_some_and(|cluster| cluster.is_hard_break) {
                        LineEndKind::HardNewline
                    } else if sub_seg
                        .last()
                        .is_some_and(|cluster| cluster.is_soft_wrap_boundary)
                    {
                        LineEndKind::SoftWrapWhitespace
                    } else if j + 1 < sub_infos.len() - 1 {
                        LineEndKind::SoftWrapNonWhitespace
                    } else {
                        LineEndKind::EndOfText
                    }
                });
                processed_lines.push(ProcessedLine {
                    clusters: sub_seg,
                    byte_start: sub_infos[j],
                    byte_end: sub_infos[j + 1],
                    baseline_y: line.baseline_y,
                    end_kind,
                });
            }
        }

        // ── Vertical overflow: cap visible line count ───────────────────────
        let max_lines = max_h
            .map(|h| (h / line_height_snapped).floor() as usize)
            .unwrap_or(processed_lines.len());
        let mut truncated_vertical = false;
        if processed_lines.len() > max_lines {
            truncated_vertical = true;
            match flow.overflow_y {
                OverflowY::Drop => {
                    processed_lines.truncate(max_lines);
                }
                OverflowY::Keep => {
                    processed_lines.truncate(max_lines + 1);
                }
                OverflowY::Ellipsis { fallback } => {
                    if max_lines > 0 {
                        let last_idx = max_lines - 1;
                        let last_line_clusters =
                            std::mem::take(&mut processed_lines[last_idx].clusters);
                        let w = max_w.unwrap_or(f32::INFINITY);
                        let ellipsised = self.apply_ellipsis_x(
                            last_line_clusters,
                            w,
                            size,
                            font_id,
                            fallback,
                            processed_lines[last_idx].baseline_y,
                        );
                        processed_lines.truncate(max_lines);
                        processed_lines[last_idx].clusters = ellipsised;
                        processed_lines[last_idx].end_kind = LineEndKind::EllipsisY;
                    } else {
                        match fallback {
                            EllipsisFallback::Keep => {
                                processed_lines.truncate(1);
                                if let Some(line) = processed_lines.first_mut() {
                                    line.end_kind = LineEndKind::EllipsisY;
                                }
                            }
                            EllipsisFallback::Drop => {
                                processed_lines.clear();
                            }
                        }
                    }
                }
            }
        }

        if processed_lines.is_empty() {
            processed_lines.push(ProcessedLine {
                clusters: Vec::new(),
                byte_start: 0,
                byte_end: text.len(),
                baseline_y: line_height_snapped,
                end_kind: LineEndKind::EndOfText,
            });
        }

        let mut out: Vec<GlyphPosition> = Vec::new();
        let mut out_clusters: Vec<TextCluster> = Vec::new();
        let mut rec: Vec<LineRec> = Vec::new();
        let mut block_width = 0.0_f32;
        let mut ink_l = f32::INFINITY;
        let mut ink_t = f32::INFINITY;
        let mut ink_r = f32::NEG_INFINITY;
        let mut ink_b = f32::NEG_INFINITY;

        let line_count = processed_lines.len();
        for (i, mut line) in processed_lines.into_iter().enumerate() {
            let new_baseline_y = i as f32 * line_height_snapped + baseline_offset;
            let new_y_top = i as f32 * line_height_snapped;

            // Glyphs already have baseline-relative y from original shaping
            // We need to adjust them to the new baseline position
            for cluster in &mut line.clusters {
                for g in &mut cluster.glyphs {
                    // Extract the original baseline-relative offset (gy from swash)
                    // Since g.y was set to (old_baseline_y + gy), we need to:
                    // 1. Extract gy = g.y - old_baseline_y
                    // 2. Calculate new absolute y = new_baseline_y + gy
                    // 3. Round for snapping
                    let baseline_relative_y = g.y - line.baseline_y;
                    g.y = (new_baseline_y + baseline_relative_y).round();
                }
            }

            let align_off = match max_w {
                Some(w) => {
                    let logical_line_w = Self::logical_cluster_line_width(&line.clusters);
                    match flow.line_align {
                        TextLineAlign::Start => 0.0,
                        TextLineAlign::Center => ((w - logical_line_w) * 0.5).max(0.0),
                        TextLineAlign::End => (w - logical_line_w).max(0.0),
                    }
                }
                None => 0.0,
            };
            if align_off != 0.0 {
                for cluster in &mut line.clusters {
                    cluster.shift_x(align_off);
                }
            }

            for cluster in &mut line.clusters {
                for g in &mut cluster.glyphs {
                    if g.raster_w == 0 && g.raster_h == 0 {
                        continue;
                    }

                    let abs_x = absolute_x.unwrap_or(0.0) + g.x;
                    let subpixel_x = (abs_x.fract() * 4.0).round() as u8 % 4;
                    if g.subpixel_x != subpixel_x {
                        g.subpixel_x = subpixel_x;
                        let (w, h) = self.get_glyph_metrics(
                            font_id.0,
                            g.key.glyph_index,
                            size,
                            subpixel_x,
                            g.weight,
                            g.opsz,
                        );
                        g.raster_w = w as usize;
                        g.raster_h = h as usize;
                    }
                }
            }

            let mut line_ink_l = f32::INFINITY;
            let mut line_ink_r = f32::NEG_INFINITY;

            for cluster in &line.clusters {
                for g in &cluster.glyphs {
                    if g.raster_w == 0 && g.raster_h == 0 {
                        continue;
                    }

                    let key = crate::text::GlyphKey {
                        font_id: font_id.0,
                        glyph_index: g.key.glyph_index,
                        size: (g.key.px * 10.0) as u32,
                        subpixel_x: g.subpixel_x,
                        weight: g.weight,
                        opsz: g.opsz,
                    };
                    if let Some(info) = self.glyph_cache.get(&key) {
                        if info.atlas_rect.w == 0 || info.atlas_rect.h == 0 {
                            continue;
                        }
                        let l = g.x + info.left as f32;
                        let t = g.y - info.top as f32;
                        let r = l + info.atlas_rect.w as f32;
                        let b = t + info.atlas_rect.h as f32;
                        ink_l = ink_l.min(l);
                        ink_t = ink_t.min(t);
                        ink_r = ink_r.max(r);
                        ink_b = ink_b.max(b);

                        line_ink_l = line_ink_l.min(l);
                        line_ink_r = line_ink_r.max(r);
                    }
                }
            }

            let line_ink_width = if line_ink_l.is_finite() {
                (line_ink_r - line_ink_l).max(0.0)
            } else {
                0.0
            };

            let logical_line_w = Self::logical_cluster_line_width(&line.clusters);
            block_width = block_width.max(logical_line_w);

            let glyph_start = out.len();
            let cluster_start = out_clusters.len();
            for cluster in line.clusters {
                let cluster_glyph_start = out.len();
                out.extend(cluster.glyphs);
                out_clusters.push(TextCluster {
                    byte_start: cluster.byte_start,
                    byte_end: cluster.byte_end,
                    glyph_start: cluster_glyph_start,
                    glyph_end: out.len(),
                    x: cluster.x,
                    advance: cluster.advance,
                    is_hard_break: cluster.is_hard_break,
                    is_whitespace: cluster.is_whitespace,
                    is_soft_wrap_boundary: cluster.is_soft_wrap_boundary,
                });
            }
            rec.push(LineRec {
                y_top: new_y_top,
                height: line_height_snapped,
                logical_width: logical_line_w,
                ink_width: line_ink_width,
                logical_x: align_off,
                ink_x: if line_ink_l.is_finite() {
                    line_ink_l
                } else {
                    align_off
                },
                glyph_start,
                glyph_end: out.len(),
                cluster_start,
                cluster_end: out_clusters.len(),
                byte_start: line.byte_start,
                byte_end: line.byte_end,
                end_kind: line.end_kind,
            });
        }

        // Keep reported bounds conservative so a measured width can safely be
        // reused as a hard prepare-time constraint.
        let ink_bounds = if ink_l.is_finite() {
            Rect::new(ink_l, ink_t, ink_r - ink_l, ink_b - ink_t)
        } else {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        };
        let lines = rec
            .iter()
            .map(|r| framewise::LineMetrics {
                y_top: r.y_top,
                height: r.height,
                logical_width: r.logical_width,
                ink_width: r.ink_width,
                logical_x: r.logical_x,
                ink_x: r.ink_x,
                byte_start: r.byte_start,
                byte_end: r.byte_end,
                end_kind: r.end_kind,
            })
            .collect();

        let metrics = framewise::TextMetrics {
            logical_size: Vec2::new(block_width.ceil(), line_count as f32 * line_height_snapped),
            ink_bounds,
            line_count: line_count as u32,
            truncated_horizontal,
            truncated_vertical,
            lines,
        };
        (out, out_clusters, rec, metrics)
    }

    fn logical_line_width(glyphs: &[GlyphPosition]) -> f32 {
        let start = Self::logical_line_start(glyphs);
        glyphs
            .iter()
            .map(Self::logical_glyph_end)
            .fold(start, f32::max)
            - start
    }

    fn logical_line_start(glyphs: &[GlyphPosition]) -> f32 {
        glyphs.iter().map(|g| g.x).fold(0.0, f32::min)
    }

    fn logical_glyph_end(g: &GlyphPosition) -> f32 {
        g.x + g.advance
    }

    fn logical_cluster_line_width(clusters: &[OwnedCluster]) -> f32 {
        let start = Self::logical_cluster_line_start(clusters);
        clusters
            .iter()
            .map(OwnedCluster::end_x)
            .fold(start, f32::max)
            - start
    }

    fn logical_cluster_line_start(clusters: &[OwnedCluster]) -> f32 {
        clusters
            .iter()
            .map(|cluster| cluster.x)
            .reduce(f32::min)
            .unwrap_or(0.0)
    }

    fn wrap_clusters(
        clusters: Vec<OwnedCluster>,
        w: f32,
        fallback: WrapClusterFallback,
    ) -> Vec<Vec<OwnedCluster>> {
        let mut lines: Vec<Vec<OwnedCluster>> = Vec::new();
        if clusters.is_empty() {
            return vec![Vec::new()];
        }
        let mut current_line = Vec::new();
        let mut current_line_start_x = clusters[0].x;

        for cluster in clusters {
            if cluster.is_hard_break {
                let mut moved = cluster;
                let mut appended = false;
                if current_line.is_empty() {
                    if let Some(last_line) = lines.last_mut() {
                        if last_line.last().map(|c: &OwnedCluster| c.is_hard_break) != Some(true) {
                            moved.shift_x(-moved.x);
                            last_line.push(moved.clone());
                            appended = true;
                        }
                    }
                }
                if !appended {
                    moved.shift_x(-current_line_start_x);
                    current_line.push(moved);
                    lines.push(current_line);
                    current_line = Vec::new();
                }
                continue;
            }

            let rel_start_x = cluster.x - current_line_start_x;
            let rel_end_x = rel_start_x + cluster.advance;

            if rel_end_x <= w {
                let mut moved = cluster;
                moved.shift_x(rel_start_x - moved.x);
                current_line.push(moved);
            } else {
                if cluster.is_whitespace && !current_line.is_empty() {
                    let next_line_start_x = cluster.x + cluster.advance;
                    let mut moved = cluster;
                    moved.shift_x(rel_start_x - moved.x);
                    moved.collapse_soft_wrap_boundary();
                    current_line.push(moved);
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_start_x = next_line_start_x;
                    continue;
                }

                if current_line.is_empty() {
                    match fallback {
                        WrapClusterFallback::Keep => {
                            let mut moved = cluster;
                            moved.shift_x(rel_start_x - moved.x);
                            current_line.push(moved);
                            lines.push(current_line);
                            current_line = Vec::new();
                            current_line_start_x += rel_end_x;
                        }
                        WrapClusterFallback::Drop => {
                            break;
                        }
                    }
                } else {
                    Self::collapse_trailing_soft_wrap_space(&mut current_line);
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_start_x = cluster.x;

                    let new_rel_start_x = 0.0;
                    let new_rel_end_x = cluster.advance;
                    if new_rel_end_x <= w {
                        let mut moved = cluster;
                        moved.shift_x(new_rel_start_x - moved.x);
                        current_line.push(moved);
                    } else {
                        match fallback {
                            WrapClusterFallback::Keep => {
                                let mut moved = cluster;
                                moved.shift_x(new_rel_start_x - moved.x);
                                current_line.push(moved);
                                lines.push(current_line);
                                current_line = Vec::new();
                                current_line_start_x += new_rel_end_x;
                            }
                            WrapClusterFallback::Drop => {
                                break;
                            }
                        }
                    }
                }
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    fn append_empty_after_terminal_soft_wrap_boundary(
        lines: &mut Vec<Vec<OwnedCluster>>,
        source_byte_end: usize,
    ) {
        let has_terminal_boundary =
            lines
                .last()
                .and_then(|line| line.last())
                .is_some_and(|cluster| {
                    cluster.is_soft_wrap_boundary && cluster.byte_end == source_byte_end
                });
        if has_terminal_boundary {
            lines.push(Vec::new());
        }
    }

    fn collapse_trailing_soft_wrap_space(clusters: &mut [OwnedCluster]) {
        let has_non_whitespace_content = clusters
            .iter()
            .rev()
            .skip(1)
            .any(|cluster| !cluster.is_whitespace && !cluster.is_hard_break);
        if has_non_whitespace_content {
            if let Some(cluster) = clusters
                .last_mut()
                .filter(|cluster| cluster.is_whitespace && !cluster.is_hard_break)
            {
                cluster.collapse_soft_wrap_boundary();
            }
        }
    }

    fn wrap_clusters_at_words(
        clusters: Vec<OwnedCluster>,
        w: f32,
        fallback: WrapWordFallback,
    ) -> Vec<Vec<OwnedCluster>> {
        if clusters.is_empty() {
            return vec![Vec::new()];
        }

        struct Seg {
            is_space: bool,
            clusters: Vec<OwnedCluster>,
            logical_w: f32,
        }

        let mut segments: Vec<Seg> = Vec::new();
        for cluster in clusters {
            let is_space = cluster.is_whitespace || cluster.is_hard_break;
            if !is_space {
                if let Some(last) = segments.last_mut() {
                    if !last.is_space {
                        last.clusters.push(cluster);
                        continue;
                    }
                }
            }
            segments.push(Seg {
                is_space,
                clusters: vec![cluster],
                logical_w: 0.0,
            });
        }

        let mut seg_starts = Vec::with_capacity(segments.len());
        for seg in &segments {
            let seg_l = if seg.clusters.is_empty() {
                0.0
            } else {
                seg.clusters
                    .iter()
                    .map(|cluster| cluster.x)
                    .fold(f32::INFINITY, f32::min)
            };
            seg_starts.push(seg_l);
        }

        let seg_len = segments.len();
        for i in 0..seg_len {
            if segments[i].clusters.is_empty() {
                continue;
            }
            let seg_l = seg_starts[i];

            for cluster in &mut segments[i].clusters {
                cluster.shift_x(-seg_l);
            }

            if i + 1 < seg_len {
                segments[i].logical_w = seg_starts[i + 1] - seg_l;
            } else {
                segments[i].logical_w = Self::logical_cluster_line_width(&segments[i].clusters);
            }
        }

        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_logical_w = 0.0;

        for seg in segments {
            let seg_logical_w = seg.logical_w;
            let is_hard_break = seg.clusters.iter().any(|c| c.is_hard_break);

            if is_hard_break || current_logical_w + seg_logical_w <= w {
                for mut cluster in seg.clusters {
                    cluster.shift_x(current_logical_w);
                    current_line.push(cluster);
                }
                current_logical_w += seg_logical_w;
            } else {
                if seg.is_space && !current_line.is_empty() {
                    for mut cluster in seg.clusters {
                        cluster.shift_x(current_logical_w);
                        cluster.collapse_soft_wrap_boundary();
                        current_line.push(cluster);
                    }
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_logical_w = 0.0;
                    continue;
                }

                if !current_line.is_empty() {
                    Self::collapse_trailing_soft_wrap_space(&mut current_line);
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_logical_w = 0.0;
                }

                if seg_logical_w <= w {
                    for mut cluster in seg.clusters {
                        cluster.shift_x(current_logical_w);
                        current_line.push(cluster);
                    }
                    current_logical_w += seg_logical_w;
                } else {
                    // Segment doesn't fit even on an empty line. Use fallback.
                    match &fallback {
                        WrapWordFallback::WrapCluster { fallback: gf } => {
                            let seg_len = seg.clusters.len();
                            let wrapped = Self::wrap_clusters(seg.clusters, w, *gf);
                            let mut wrapped_count = 0;
                            if !wrapped.is_empty() {
                                lines.extend(wrapped[..wrapped.len() - 1].to_vec());
                                current_line = wrapped.last().unwrap().clone();
                                current_logical_w = current_line
                                    .iter()
                                    .map(OwnedCluster::end_x)
                                    .fold(0.0, f32::max);
                                wrapped_count = wrapped.iter().map(|line| line.len()).sum();
                            }
                            if *gf == WrapClusterFallback::Drop && wrapped_count < seg_len {
                                break;
                            }
                        }
                        WrapWordFallback::Drop => {
                            for cluster in seg.clusters {
                                if cluster.end_x() <= w {
                                    current_line.push(cluster);
                                } else {
                                    break;
                                }
                            }
                            lines.push(current_line);
                            current_line = Vec::new();
                            break;
                        }
                        WrapWordFallback::Keep => {
                            for cluster in seg.clusters {
                                let end_x = cluster.end_x();
                                current_line.push(cluster);
                                if end_x > w {
                                    break;
                                }
                            }
                            lines.push(current_line);
                            current_line = Vec::new();
                            break;
                        }
                    }
                }
            }
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    fn apply_ellipsis_x(
        &mut self,
        clusters: Vec<OwnedCluster>,
        w: f32,
        size: f32,
        font_id: FontId,
        fallback: EllipsisFallback,
        line_baseline_y: f32,
    ) -> Vec<OwnedCluster> {
        let (ell_glyphs, ell_w, ell_baseline) = self.ellipsis(size, font_id);
        let insert_byte = clusters.last().map(|cluster| cluster.byte_end).unwrap_or(0);
        let mut ell_cluster = OwnedCluster {
            byte_start: insert_byte,
            byte_end: insert_byte,
            x: 0.0,
            advance: ell_w,
            is_hard_break: false,
            is_whitespace: false,
            is_soft_wrap_boundary: false,
            glyphs: ell_glyphs,
        };

        if ell_w > w {
            match fallback {
                EllipsisFallback::Keep => {
                    let dy = line_baseline_y - ell_baseline;
                    for g in &mut ell_cluster.glyphs {
                        g.y += dy;
                    }
                    vec![ell_cluster]
                }
                EllipsisFallback::Drop => Vec::new(),
            }
        } else {
            let limit = w - ell_w;
            let mut trimmed = Vec::new();
            for cluster in clusters {
                if cluster.end_x() <= limit {
                    trimmed.push(cluster);
                } else {
                    break;
                }
            }
            let pen_x = trimmed.last().map(OwnedCluster::end_x).unwrap_or(0.0);
            let dy = line_baseline_y - ell_baseline;
            ell_cluster.shift_x(pen_x);
            for eg in &mut ell_cluster.glyphs {
                eg.y += dy;
            }
            trimmed.push(ell_cluster);
            trimmed
        }
    }
}
