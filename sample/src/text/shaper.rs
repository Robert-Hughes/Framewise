use crate::text::types::{GlyphPosition, GlyphRasterConfig, LineRec};
use crate::text::SampleTextSystem;
use framewise::{
    EllipsisFallback, FontId, HorizontalAlign, OverflowX, OverflowY, TextFlow, TextMetrics, Vec2,
    WrapGlyphFallback, WrapWordFallback,
};

struct Line {
    glyph_start: usize,
    glyph_end: usize,
    byte_start: usize,
    byte_end: usize,
    baseline_y: f32,
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

    pub fn line_height(&self, size: f32, font_id: FontId) -> f32 {
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

    pub fn ellipsis(&mut self, size: f32, font_id: FontId) -> (Vec<GlyphPosition>, f32, f32) {
        let flow = TextFlow::single_line();
        let (glyphs, lines, _metrics) =
            self.shape_internal("…", size, font_id, flow, None, None, Some(0.0));

        let width = if glyphs.is_empty() {
            0.0
        } else {
            let l = glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
            let r = glyphs
                .iter()
                .map(|g| g.x + g.width as f32)
                .fold(f32::NEG_INFINITY, f32::max);
            r - l
        };

        let baseline = lines.first().map(|l| l.y_top + l.height).unwrap_or(0.0);
        (glyphs, width, baseline)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn shape_internal(
        &mut self,
        text: &str,
        size: f32,
        font_id: FontId,
        flow: TextFlow,
        max_w: Option<f32>,
        max_h: Option<f32>,
        absolute_x: Option<f32>,
    ) -> (Vec<GlyphPosition>, Vec<LineRec>, TextMetrics) {
        let line_height = self.line_height(size, font_id);
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
        if text.is_empty() {
            lines_raw.push(("", false, 0, 0));
        }

        let mut glyphs0 = Vec::new();
        let mut lines = Vec::new();

        for (i, &(segment, has_newline, segment_start, segment_end)) in lines_raw.iter().enumerate()
        {
            let line_start_index = glyphs0.len();
            let mut pen_x = 0.0_f32;

            if !segment.is_empty() {
                let mut temp_glyphs = Vec::new();
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
                    let byte_offset = segment_start + cluster.source.start as usize;
                    let parent_char = segment[cluster.source.to_range()]
                        .chars()
                        .next()
                        .unwrap_or(' ');

                    for glyph in cluster.glyphs {
                        temp_glyphs.push((
                            parent_char,
                            glyph.id,
                            pen_x + glyph.x,
                            glyph.y,
                            byte_offset,
                            glyph.advance,
                        ));
                        pen_x += glyph.advance;
                    }
                });

                for (parent_char, glyph_index, gx, gy, byte_offset, advance) in temp_glyphs {
                    // Calculate absolute X coordinate to resolve subpixel binning
                    let abs_x = absolute_x.unwrap_or(0.0) + gx;
                    let subpixel_x = (abs_x.fract() * 4.0).round() as u8 % 4;

                    let (w, h) = self.get_glyph_metrics(
                        font_id.0,
                        glyph_index,
                        size,
                        subpixel_x,
                        weight,
                        opsz as u16,
                    );

                    // Calculate the baseline position for this line
                    let baseline_y = i as f32 * line_height_snapped + baseline_offset;

                    glyphs0.push(GlyphPosition {
                        parent: parent_char,
                        key: GlyphRasterConfig {
                            glyph_index,
                            px: size,
                        },
                        x: gx,
                        y: baseline_y + gy, // Store as local baseline position
                        width: w as usize,  // bitmap width for now
                        height: h as usize,
                        byte_offset,
                        subpixel_x,
                        advance, // shaped advance for text flow
                        weight,
                        opsz: opsz as u16,
                    });
                }
            }

            if has_newline {
                glyphs0.push(GlyphPosition {
                    parent: '\n',
                    key: GlyphRasterConfig {
                        glyph_index: 0,
                        px: size,
                    },
                    x: pen_x,
                    y: i as f32 * line_height_snapped + baseline_offset,
                    width: 0,
                    height: 0,
                    byte_offset: segment_end,
                    subpixel_x: 0,
                    advance: 0.0,
                    weight,
                    opsz: opsz as u16,
                });
            }

            let byte_end = if has_newline {
                segment_end + 1
            } else {
                segment_end
            };

            lines.push(Line {
                glyph_start: line_start_index,
                glyph_end: glyphs0.len(),
                byte_start: segment_start,
                byte_end,
                baseline_y: i as f32 * line_height_snapped + baseline_offset,
            });
        }

        // ── Per-line: align, clip / ellipsis, rebuild glyph vec ─────────────
        let global_l = if glyphs0.is_empty() {
            0.0
        } else {
            glyphs0.iter().map(|g| g.x).fold(f32::INFINITY, f32::min)
        };

        struct ProcessedLine {
            glyphs: Vec<GlyphPosition>,
            byte_start: usize,
            byte_end: usize,
            baseline_y: f32,
        }

        let mut truncated_horizontal = false;
        let mut processed_lines: Vec<ProcessedLine> = Vec::new();

        for line in lines {
            let mut seg = glyphs0[line.glyph_start..line.glyph_end].to_vec();

            let (line_l, line_r) = if seg.is_empty() {
                (0.0, 0.0)
            } else {
                let l = seg.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
                let r = seg
                    .iter()
                    .map(|g| g.x + g.width as f32)
                    .fold(f32::NEG_INFINITY, f32::max);
                (l, r)
            };
            let line_w = line_r - line_l;

            let base_shift = match flow.horizontal_align {
                HorizontalAlign::Start => global_l,
                HorizontalAlign::Center | HorizontalAlign::End => line_l,
            };
            if base_shift != 0.0 {
                for g in &mut seg {
                    g.x -= base_shift;
                }
            }

            let mut final_sublines = Vec::new();

            if let Some(w) = max_w {
                match flow.overflow_x {
                    OverflowX::WrapWord { fallback } => {
                        let wrapped = Self::wrap_glyphs_at_words(seg, w, fallback);
                        final_sublines.extend(wrapped);
                    }
                    OverflowX::WrapGlyph { fallback } => {
                        let wrapped = Self::wrap_glyphs_at_glyphs(seg, w, fallback);
                        final_sublines.extend(wrapped);
                    }
                    _ => {
                        let overflows_w = line_w > w + 0.5;
                        if overflows_w {
                            truncated_horizontal = true;
                            match flow.overflow_x {
                                OverflowX::Ellipsis { fallback } => {
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
                                    let mut out = Vec::new();
                                    for g in seg {
                                        let end_x = g.x + g.width as f32;
                                        out.push(g);
                                        if end_x > w {
                                            break;
                                        }
                                    }
                                    final_sublines.push(out);
                                }
                                OverflowX::Drop => {
                                    let mut out = Vec::new();
                                    for g in seg {
                                        if g.x + g.width as f32 <= w {
                                            out.push(g);
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

            let mut sub_infos = Vec::new();
            for (j, sub_seg) in final_sublines.iter().enumerate() {
                let b_start = if j == 0 {
                    line.byte_start
                } else {
                    sub_seg
                        .first()
                        .map(|g| g.byte_offset)
                        .unwrap_or(line.byte_start)
                };
                sub_infos.push(b_start);
            }
            sub_infos.push(line.byte_end);

            for (j, sub_seg) in final_sublines.into_iter().enumerate() {
                processed_lines.push(ProcessedLine {
                    glyphs: sub_seg,
                    byte_start: sub_infos[j],
                    byte_end: sub_infos[j + 1],
                    baseline_y: line.baseline_y,
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
                        let last_line_glyphs =
                            std::mem::take(&mut processed_lines[last_idx].glyphs);
                        let w = max_w.unwrap_or(f32::INFINITY);
                        let ellipsised = self.apply_ellipsis_x(
                            last_line_glyphs,
                            w,
                            size,
                            font_id,
                            fallback,
                            processed_lines[last_idx].baseline_y,
                        );
                        processed_lines.truncate(max_lines);
                        processed_lines[last_idx].glyphs = ellipsised;
                    } else {
                        match fallback {
                            EllipsisFallback::Keep => {
                                processed_lines.truncate(1);
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
                glyphs: Vec::new(),
                byte_start: 0,
                byte_end: text.len(),
                baseline_y: line_height_snapped,
            });
        }

        let mut out: Vec<GlyphPosition> = Vec::new();
        let mut rec: Vec<LineRec> = Vec::new();
        let mut block_width = 0.0_f32;

        let line_count = processed_lines.len();
        for (i, mut line) in processed_lines.into_iter().enumerate() {
            let new_baseline_y = i as f32 * line_height_snapped + baseline_offset;
            let new_y_top = i as f32 * line_height_snapped;

            // Glyphs already have baseline-relative y from original shaping
            // We need to adjust them to the new baseline position
            for g in &mut line.glyphs {
                // Extract the original baseline-relative offset (gy from swash)
                // Since g.y was set to (old_baseline_y + gy), we need to:
                // 1. Extract gy = g.y - old_baseline_y
                // 2. Calculate new absolute y = new_baseline_y + gy
                // 3. Round for snapping
                let baseline_relative_y = g.y - line.baseline_y;
                g.y = (new_baseline_y + baseline_relative_y).round();
            }

            let align_off = match max_w {
                Some(w) => {
                    let line_w = if line.glyphs.is_empty() {
                        0.0
                    } else {
                        let l = line
                            .glyphs
                            .iter()
                            .map(|g| g.x)
                            .fold(f32::INFINITY, f32::min);
                        let r = line
                            .glyphs
                            .iter()
                            .map(|g| g.x + g.width as f32)
                            .fold(f32::NEG_INFINITY, f32::max);
                        r - l
                    };
                    match flow.horizontal_align {
                        HorizontalAlign::Start => 0.0,
                        HorizontalAlign::Center => ((w - line_w) * 0.5).max(0.0),
                        HorizontalAlign::End => (w - line_w).max(0.0),
                    }
                }
                None => 0.0,
            };
            if align_off != 0.0 {
                for g in &mut line.glyphs {
                    g.x += align_off;
                }
            }

            let line_w = if line.glyphs.is_empty() {
                0.0
            } else {
                let l = line
                    .glyphs
                    .iter()
                    .map(|g| g.x)
                    .fold(f32::INFINITY, f32::min);
                let r = line
                    .glyphs
                    .iter()
                    .map(|g| g.x + g.width as f32)
                    .fold(f32::NEG_INFINITY, f32::max);
                r - l
            };
            block_width = block_width.max(line_w);

            let glyph_start = out.len();
            out.extend(line.glyphs);
            rec.push(LineRec {
                y_top: new_y_top,
                height: line_height_snapped,
                glyph_start,
                glyph_end: out.len(),
                byte_start: line.byte_start,
                byte_end: line.byte_end,
            });
        }

        // Round final metrics block size
        let metrics = framewise::TextMetrics {
            size: Vec2::new(block_width.round(), line_count as f32 * line_height_snapped),
            line_count: line_count as u32,
            truncated_horizontal,
            truncated_vertical,
        };
        (out, rec, metrics)
    }

    fn wrap_glyphs_at_glyphs(
        glyphs: Vec<GlyphPosition>,
        w: f32,
        fallback: WrapGlyphFallback,
    ) -> Vec<Vec<GlyphPosition>> {
        let mut lines = Vec::new();
        if glyphs.is_empty() {
            return lines;
        }
        let mut current_line = Vec::new();
        let mut current_line_start_x = glyphs[0].x;

        for g in glyphs {
            if g.parent == '\n' {
                let mut g_moved = g;
                let mut appended = false;
                if current_line.is_empty() {
                    if let Some(last_line) = lines.last_mut() {
                        if last_line.last().map(|gl| gl.parent) != Some('\n') {
                            g_moved.x = 0.0;
                            last_line.push(g_moved);
                            appended = true;
                        }
                    }
                }
                if !appended {
                    g_moved.x = g.x - current_line_start_x;
                    current_line.push(g_moved);
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_start_x = g.x;
                }
                continue;
            }

            let rel_start_x = g.x - current_line_start_x;
            let rel_end_x = rel_start_x + g.width as f32;

            if rel_end_x <= w {
                let mut g_moved = g;
                g_moved.x = rel_start_x;
                current_line.push(g_moved);
            } else {
                if current_line.is_empty() {
                    match fallback {
                        WrapGlyphFallback::Keep => {
                            let mut g_moved = g;
                            g_moved.x = rel_start_x;
                            current_line.push(g_moved);
                            lines.push(current_line);
                            current_line = Vec::new();
                            current_line_start_x = g.x + g.width as f32;
                        }
                        WrapGlyphFallback::Drop => {
                            break;
                        }
                    }
                } else {
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_start_x = g.x;

                    let new_rel_start_x = 0.0;
                    let new_rel_end_x = g.width as f32;
                    if new_rel_end_x <= w {
                        let mut g_moved = g;
                        g_moved.x = new_rel_start_x;
                        current_line.push(g_moved);
                    } else {
                        match fallback {
                            WrapGlyphFallback::Keep => {
                                let mut g_moved = g;
                                g_moved.x = new_rel_start_x;
                                current_line.push(g_moved);
                                lines.push(current_line);
                                current_line = Vec::new();
                                current_line_start_x = g.x + g.width as f32;
                            }
                            WrapGlyphFallback::Drop => {
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

    fn wrap_glyphs_at_words(
        glyphs: Vec<GlyphPosition>,
        w: f32,
        fallback: WrapWordFallback,
    ) -> Vec<Vec<GlyphPosition>> {
        if glyphs.is_empty() {
            return Vec::new();
        }

        struct Seg {
            is_space: bool,
            glyphs: Vec<GlyphPosition>,
            width: f32,
        }

        let mut segments: Vec<Seg> = Vec::new();
        for g in glyphs {
            let is_space = g.parent == ' ' || g.parent == '\n';
            if let Some(last) = segments.last_mut() {
                if last.is_space == is_space {
                    last.glyphs.push(g);
                    continue;
                }
            }
            segments.push(Seg {
                is_space,
                glyphs: vec![g],
                width: 0.0,
            });
        }

        let mut seg_starts = Vec::with_capacity(segments.len());
        for seg in &segments {
            let seg_l = if seg.glyphs.is_empty() {
                0.0
            } else {
                seg.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min)
            };
            seg_starts.push(seg_l);
        }

        let seg_len = segments.len();
        for i in 0..seg_len {
            if segments[i].glyphs.is_empty() {
                continue;
            }
            let seg_l = seg_starts[i];

            for g in &mut segments[i].glyphs {
                g.x -= seg_l;
            }

            if i + 1 < seg_len {
                segments[i].width = seg_starts[i + 1] - seg_l;
            } else {
                segments[i].width = segments[i]
                    .glyphs
                    .iter()
                    .map(|g| g.x + g.width as f32)
                    .fold(0.0, f32::max);
            }
        }

        let mut lines = Vec::new();
        let mut current_line = Vec::new();
        let mut current_w = 0.0;

        for seg in segments {
            if seg.is_space {
                if !current_line.is_empty() {
                    for g in seg.glyphs {
                        let mut g_moved = g;
                        g_moved.x += current_w;
                        current_line.push(g_moved);
                    }
                    current_w += seg.width;
                }
            } else {
                let word_w = seg.width;
                if current_w + word_w <= w {
                    for g in seg.glyphs {
                        let mut g_moved = g;
                        g_moved.x += current_w;
                        current_line.push(g_moved);
                    }
                    current_w += word_w;
                } else {
                    if !current_line.is_empty() {
                        lines.push(current_line);
                        current_line = Vec::new();
                        current_w = 0.0;
                    }

                    if word_w <= w {
                        for g in seg.glyphs {
                            let mut g_moved = g;
                            g_moved.x += current_w;
                            current_line.push(g_moved);
                        }
                        current_w += word_w;
                    } else {
                        match &fallback {
                            WrapWordFallback::WrapGlyph { fallback: gf } => {
                                let seg_len = seg.glyphs.len();
                                let wrapped = Self::wrap_glyphs_at_glyphs(seg.glyphs, w, *gf);
                                let mut wrapped_count = 0;
                                if !wrapped.is_empty() {
                                    lines.extend(wrapped[..wrapped.len() - 1].to_vec());
                                    current_line = wrapped.last().unwrap().clone();
                                    current_w = current_line
                                        .iter()
                                        .map(|g| g.x + g.width as f32)
                                        .fold(0.0, f32::max);
                                    wrapped_count = wrapped.iter().map(|line| line.len()).sum();
                                }
                                if *gf == WrapGlyphFallback::Drop && wrapped_count < seg_len {
                                    break;
                                }
                            }
                            WrapWordFallback::Drop => {
                                for g in seg.glyphs {
                                    if g.x + g.width as f32 <= w {
                                        current_line.push(g);
                                    } else {
                                        break;
                                    }
                                }
                                lines.push(current_line);
                                current_line = Vec::new();
                                break;
                            }
                            WrapWordFallback::Keep => {
                                for g in seg.glyphs {
                                    let end_x = g.x + g.width as f32;
                                    current_line.push(g);
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
        }
        if !current_line.is_empty() {
            lines.push(current_line);
        }
        lines
    }

    fn apply_ellipsis_x(
        &mut self,
        glyphs: Vec<GlyphPosition>,
        w: f32,
        size: f32,
        font_id: FontId,
        fallback: EllipsisFallback,
        line_baseline_y: f32,
    ) -> Vec<GlyphPosition> {
        let (ell_glyphs, ell_w, ell_baseline) = self.ellipsis(size, font_id);
        if ell_w > w {
            match fallback {
                EllipsisFallback::Keep => {
                    let mut out = Vec::new();
                    for g in ell_glyphs {
                        let rel_start_x = g.x;
                        let rel_end_x = g.x + g.width as f32;
                        if rel_start_x < w {
                            out.push(g);
                            if rel_end_x > w {
                                break;
                            }
                        } else {
                            if out.is_empty() {
                                out.push(g);
                            }
                            break;
                        }
                    }
                    out
                }
                EllipsisFallback::Drop => Vec::new(),
            }
        } else {
            let limit = w - ell_w;
            let mut trimmed = Vec::new();
            for g in glyphs {
                if g.x + g.width as f32 <= limit {
                    trimmed.push(g);
                } else {
                    break;
                }
            }
            let pen_x = trimmed.last().map(|g| g.x + g.width as f32).unwrap_or(0.0);
            let dy = line_baseline_y - ell_baseline;
            for mut eg in ell_glyphs {
                eg.x += pen_x;
                eg.y += dy;
                trimmed.push(eg);
            }
            trimmed
        }
    }
}
