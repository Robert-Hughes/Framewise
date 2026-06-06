use fontdue::{
    layout::{
        CoordinateSystem, GlyphPosition, HorizontalAlign as FdHAlign, Layout, LayoutSettings,
        TextStyle, VerticalAlign as FdVAlign, WrapStyle,
    },
    Font, FontSettings,
};
use framewise::{
    CaretGeom, EllipsisFallback, FontId, HorizontalAlign, OverflowX, OverflowY, Rect, TextBounds,
    TextFlow, TextHandle, TextLayout, TextMetrics, TextSystem, Vec2, WrapGlyphFallback,
    WrapWordFallback,
};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32, // store size as u32 (size * 10.0 as u32) for hashing
}

pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct GlyphInfo {
    pub atlas_rect: AtlasRect,
}

/// One laid-out line within a prepared run, in block-local coordinates.
pub struct LineRec {
    /// Top edge of the line.
    pub y_top: f32,
    /// Line height (advance to the next line).
    pub height: f32,
    /// Range into the run's `glyphs` vec: `[glyph_start, glyph_end)`.
    pub glyph_start: usize,
    pub glyph_end: usize,
    /// Byte range of the original string mapped to this line: `[byte_start, byte_end)`.
    pub byte_start: usize,
    pub byte_end: usize,
}

pub struct CachedLayout {
    pub font_id: FontId,
    pub glyphs: Vec<GlyphPosition>,
    pub lines: Vec<LineRec>,
}

pub struct SampleTextSystem {
    pub fonts: Vec<Font>,
    layout: Layout,
    pub runs: Vec<CachedLayout>,

    // Atlas data
    pub glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    pub atlas_data: Vec<u8>,
    pub atlas_size: u32,

    // Simple shelf allocator
    current_x: u32,
    current_y: u32,
    row_height: u32,

    pub atlas_dirty: bool,
}

/// Intermediate per-line record produced during shaping, before glyphs are
/// committed into the final per-run vec.
struct Line {
    glyph_start: usize,
    glyph_end: usize,
    byte_start: usize,
    byte_end: usize,
    baseline_y: f32,
}

impl SampleTextSystem {
    pub fn new() -> Self {
        let mono_data = include_bytes!("../assets/JetBrainsMono-Regular.ttf") as &[u8];
        let mono = Font::from_bytes(mono_data, FontSettings::default())
            .expect("failed to load JetBrainsMono font");
        let sans_data = include_bytes!("../assets/InterTight-Regular.ttf") as &[u8];
        let sans = Font::from_bytes(sans_data, FontSettings::default())
            .expect("failed to load InterTight font");
        let sans_bold_data = include_bytes!("../assets/InterTight-Bold.ttf") as &[u8];
        let sans_bold = Font::from_bytes(sans_bold_data, FontSettings::default())
            .expect("failed to load InterTight-Bold font");

        let atlas_size = 1024;
        Self {
            fonts: vec![mono, sans, sans_bold],
            layout: Layout::new(CoordinateSystem::PositiveYDown),
            runs: Vec::new(),
            glyph_cache: HashMap::new(),
            atlas_data: vec![0; (atlas_size * atlas_size) as usize],
            atlas_size,
            current_x: 0,
            current_y: 0,
            row_height: 0,
            atlas_dirty: false,
        }
    }

    /// Called at the start of each frame to reset the text layout handles.
    pub fn begin_frame(&mut self) {
        self.runs.clear();
        self.atlas_dirty = false;
    }

    fn line_height(&self, size: f32, font_id: FontId) -> f32 {
        self.fonts[font_id.0 as usize]
            .horizontal_line_metrics(size)
            .map(|m| m.new_line_size)
            .unwrap_or(size)
    }

    /// Lay out `…` on its own line to discover its glyphs, advance width, and
    /// baseline, so it can be repositioned onto a truncated line.
    fn ellipsis(&mut self, size: f32, font_id: FontId) -> (Vec<GlyphPosition>, f32, f32) {
        let font = &self.fonts[font_id.0 as usize];
        self.layout.reset(&LayoutSettings {
            line_height: 1.0,
            ..LayoutSettings::default()
        });
        self.layout.append(&[font], &TextStyle::new("…", size, 0));
        let glyphs = self.layout.glyphs().clone();
        let width = glyphs.last().map(|g| g.x + g.width as f32).unwrap_or(0.0);
        let baseline = self
            .layout
            .lines()
            .and_then(|l| l.first().map(|lp| lp.baseline_y))
            .unwrap_or(0.0);
        (glyphs, width, baseline)
    }

    /// Shape `text` against the given flow and per-axis limits, producing
    /// block-local glyphs, line records, and metrics. Does not touch the atlas
    /// or the run table.
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

        // Alternating word/space segments
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

        // Normalize segments to start at 0.0, and compute width
        for seg in &mut segments {
            if !seg.glyphs.is_empty() {
                let seg_l = seg.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
                for g in &mut seg.glyphs {
                    g.x -= seg_l;
                }
                seg.width = seg
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
                    // Space segment at the end of the line: only add if it fits or has 0 width
                    if seg.width == 0.0 || current_w + seg.width <= w {
                        for g in seg.glyphs {
                            let mut g_moved = g;
                            g_moved.x += current_w;
                            current_line.push(g_moved);
                        }
                        current_w += seg.width;
                    }
                }
            } else {
                let word_w = seg.width;
                if current_w + word_w <= w {
                    // Fits on current line
                    for g in seg.glyphs {
                        let mut g_moved = g;
                        g_moved.x += current_w;
                        current_line.push(g_moved);
                    }
                    current_w += word_w;
                } else {
                    // Wrap to new line
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
                        // Word does not fit even on the empty line! Apply fallback.
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
                                // Keep only characters that fit
                                for g in seg.glyphs {
                                    if g.x + g.width as f32 <= w {
                                        current_line.push(g);
                                    } else {
                                        break;
                                    }
                                }
                                lines.push(current_line);
                                current_line = Vec::new();
                                break; // Discard rest of paragraph
                            }
                            WrapWordFallback::Keep => {
                                // Keep characters that fit + first overflowing
                                for g in seg.glyphs {
                                    let end_x = g.x + g.width as f32;
                                    current_line.push(g);
                                    if end_x > w {
                                        break;
                                    }
                                }
                                lines.push(current_line);
                                current_line = Vec::new();
                                break; // Discard rest of paragraph
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

    fn shape(
        &mut self,
        text: &str,
        size: f32,
        font_id: FontId,
        flow: TextFlow,
        max_w: Option<f32>,
        max_h: Option<f32>,
    ) -> (Vec<GlyphPosition>, Vec<LineRec>, TextMetrics) {
        let line_height = self.line_height(size, font_id);

        // ── Base layout pass ────────────────────────────────────────────────
        // We do all wrapping ourselves in the shape function using custom
        // word-wrapping and glyph-wrapping algorithms, so Fontdue is run without max_width.
        let (wrap_width, wrap_style) = (None, WrapStyle::Word);
        {
            let font = &self.fonts[font_id.0 as usize];
            self.layout.reset(&LayoutSettings {
                max_width: wrap_width,
                max_height: None,
                horizontal_align: FdHAlign::Left,
                vertical_align: FdVAlign::Top,
                line_height: 1.0,
                wrap_style,
                wrap_hard_breaks: true,
                ..LayoutSettings::default()
            });
            self.layout.append(&[font], &TextStyle::new(text, size, 0));
        }
        let glyphs0 = self.layout.glyphs().clone();
        let fd_lines: Vec<(usize, f32, f32, f32)> = self
            .layout
            .lines()
            .map(|ls| {
                ls.iter()
                    .map(|l| {
                        (
                            l.glyph_start,
                            l.baseline_y,
                            l.max_ascent,
                            l.max_new_line_size,
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // ── Partition glyphs into lines ─────────────────────────────────────
        let mut lines: Vec<Line> = Vec::new();
        if fd_lines.is_empty() {
            lines.push(Line {
                glyph_start: 0,
                glyph_end: 0,
                byte_start: 0,
                byte_end: text.len(),
                baseline_y: line_height,
            });
        } else {
            for (i, &(gs, baseline_y, _max_ascent, _new_line)) in fd_lines.iter().enumerate() {
                let ge = if i + 1 < fd_lines.len() {
                    fd_lines[i + 1].0
                } else {
                    glyphs0.len()
                };
                let byte_start = glyphs0.get(gs).map(|g| g.byte_offset).unwrap_or(text.len());
                lines.push(Line {
                    glyph_start: gs,
                    glyph_end: ge,
                    byte_start,
                    byte_end: text.len(),
                    baseline_y,
                });
            }
            for i in 0..lines.len() - 1 {
                lines[i].byte_end = lines[i + 1].byte_start;
            }
        }

        let baseline_offset = if fd_lines.is_empty() {
            line_height
        } else {
            let (_, baseline_y, max_ascent, _) = fd_lines[0];
            let y_top = baseline_y - max_ascent;
            baseline_y - y_top
        };

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
            .map(|h| (h / line_height).floor() as usize)
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
                baseline_y: line_height,
            });
        }

        let mut out: Vec<GlyphPosition> = Vec::new();
        let mut rec: Vec<LineRec> = Vec::new();
        let mut block_width = 0.0_f32;

        let line_count = processed_lines.len();
        for (i, mut line) in processed_lines.into_iter().enumerate() {
            let new_baseline_y = i as f32 * line_height + baseline_offset;
            let new_y_top = i as f32 * line_height;

            for g in &mut line.glyphs {
                g.y = g.y - line.baseline_y + new_baseline_y;
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
                height: line_height,
                glyph_start,
                glyph_end: out.len(),
                byte_start: line.byte_start,
                byte_end: line.byte_end,
            });
        }

        let metrics = TextMetrics {
            size: Vec2::new(block_width, line_count as f32 * line_height),
            line_count: line_count as u32,
            truncated_horizontal,
            truncated_vertical,
        };
        (out, rec, metrics)
    }

    fn ensure_glyph(&mut self, key: GlyphKey) {
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        let font = self
            .fonts
            .get(key.font_id as usize)
            .expect("unknown font id");
        let (metrics, bitmap) = font.rasterize_indexed(key.glyph_index, key.size as f32 / 10.0);
        let w = metrics.width as u32;
        let h = metrics.height as u32;

        if w == 0 || h == 0 {
            self.glyph_cache.insert(
                key,
                GlyphInfo {
                    atlas_rect: AtlasRect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    },
                },
            );
            return;
        }

        // Shelf pack
        if self.current_x + w > self.atlas_size {
            self.current_y += self.row_height + 1;
            self.current_x = 0;
            self.row_height = 0;
        }

        if self.current_y + h > self.atlas_size {
            panic!("Atlas full!");
        }

        let x = self.current_x;
        let y = self.current_y;

        self.current_x += w + 1;
        self.row_height = self.row_height.max(h);

        // Copy bitmap to atlas
        for row in 0..h {
            let src_start = (row * w) as usize;
            let src_end = src_start + w as usize;

            let dst_start = ((y + row) * self.atlas_size + x) as usize;
            let dst_end = dst_start + w as usize;

            self.atlas_data[dst_start..dst_end].copy_from_slice(&bitmap[src_start..src_end]);
        }

        self.atlas_dirty = true;
        self.glyph_cache.insert(
            key,
            GlyphInfo {
                atlas_rect: AtlasRect { x, y, w, h },
            },
        );
    }
}

impl TextSystem for SampleTextSystem {
    fn measure(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        flow: TextFlow,
        bounds: TextBounds,
    ) -> TextMetrics {
        let (_glyphs, _lines, metrics) =
            self.shape(text, size, font, flow, bounds.max_width, bounds.max_height);
        metrics
    }

    fn prepare(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        flow: TextFlow,
        rect: Rect,
    ) -> TextLayout {
        let (glyphs, lines, metrics) =
            self.shape(text, size, font, flow, Some(rect.w), Some(rect.h));

        for g in &glyphs {
            let key = GlyphKey {
                font_id: font.0,
                glyph_index: g.key.glyph_index,
                size: (g.key.px * 10.0) as u32,
            };
            self.ensure_glyph(key);
        }

        let handle_id = self.runs.len();
        self.runs.push(CachedLayout {
            font_id: font,
            glyphs,
            lines,
        });

        TextLayout {
            handle: TextHandle(handle_id),
            metrics,
        }
    }

    fn caret_geom(&self, handle: TextHandle, byte_index: usize) -> CaretGeom {
        let run = &self.runs[handle.0];

        // Find the line the byte falls on (last line whose start is <= byte).
        let line = run
            .lines
            .iter()
            .rev()
            .find(|l| byte_index >= l.byte_start)
            .or_else(|| run.lines.first())
            .expect("a prepared run always has at least one line");

        // X within the line: leading edge of the glyph at/after byte_index,
        // else the trailing edge of the last glyph on the line.
        let glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
        let x = if byte_index >= line.byte_end {
            glyphs
                .last()
                .map(|g| g.x + g.width as f32)
                .unwrap_or_else(|| line_start_x(glyphs))
        } else {
            glyphs
                .iter()
                .find(|g| g.byte_offset >= byte_index)
                .map(|g| g.x)
                .unwrap_or_else(|| line_start_x(glyphs))
        };

        CaretGeom {
            x,
            y_top: line.y_top,
            height: line.height,
        }
    }

    fn hit_test(&self, handle: TextHandle, pos: Vec2) -> usize {
        let run = &self.runs[handle.0];

        // Resolve the line by Y (clamp above/below to first/last).
        let line = run
            .lines
            .iter()
            .find(|l| pos.y < l.y_top + l.height)
            .unwrap_or_else(|| run.lines.last().expect("at least one line"));

        let glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
        if glyphs.is_empty() {
            return line.byte_start;
        }
        for g in glyphs {
            let mid = g.x + g.width as f32 / 2.0;
            if pos.x < mid {
                return g.byte_offset;
            }
        }
        line.byte_end
    }
}

/// Leftmost glyph X on a line (its content start), used as a caret fallback.
fn line_start_x(glyphs: &[GlyphPosition]) -> f32 {
    glyphs.first().map(|g| g.x).unwrap_or(0.0)
}
#[cfg(test)]
mod tests {
    use super::*;

    fn sys() -> SampleTextSystem {
        SampleTextSystem::new()
    }

    #[test]
    fn glyph_cache_keys_include_font_id() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        let _ = sys.prepare("A", 12.0, FontId(0), TextFlow::single_line(), rect);
        let _ = sys.prepare("A", 12.0, FontId(1), TextFlow::single_line(), rect);

        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 0));
        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 1));
    }

    #[test]
    fn single_line_is_one_line() {
        let mut sys = sys();
        let m = sys.measure(
            "hello world",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 1);
        assert!(!m.truncated_horizontal && !m.truncated_vertical);
        assert!(m.size.x > 0.0);
    }

    #[test]
    fn hard_breaks_make_lines_without_wrap() {
        let mut sys = sys();
        let m = sys.measure(
            "a\nb\nc",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn wrapping_splits_a_long_line() {
        let mut sys = sys();
        let unwrapped = sys.measure(
            "the quick brown fox jumps over the lazy dog",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(unwrapped.line_count, 1);

        let wrapped = sys.measure(
            "the quick brown fox jumps over the lazy dog",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            TextBounds::width(80.0),
        );
        assert!(wrapped.line_count > 1);
        assert!(wrapped.size.x <= 80.0 + 0.5);
    }

    #[test]
    fn vertical_overflow_truncates_lines() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let m = sys.measure(
            "the quick brown fox jumps over the lazy dog again and again",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            TextBounds {
                max_width: Some(80.0),
                max_height: Some(lh * 2.0 + 1.0),
            },
        );
        assert_eq!(m.line_count, 2);
        assert!(m.truncated_vertical);
    }

    #[test]
    fn single_line_overflow_truncates_horizontally() {
        let mut sys = sys();
        let m = sys.measure(
            "hello world this is a long line",
            16.0,
            FontId(1),
            TextFlow::single_line(),
            TextBounds {
                max_width: Some(40.0),
                max_height: Some(100.0),
            },
        );
        assert_eq!(m.line_count, 1);
        assert!(m.truncated_horizontal);
        assert!(m.size.x <= 40.0 + 0.5);
    }

    #[test]
    fn caret_advances_along_single_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 40.0),
        );
        let c0 = sys.caret_geom(layout.handle, 0);
        let c3 = sys.caret_geom(layout.handle, 3);
        assert!(c3.x > c0.x);
        assert_eq!(c0.y_top, c3.y_top);
    }

    #[test]
    fn hit_test_round_trips_to_a_boundary() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 40.0),
        );
        let far = sys.hit_test(layout.handle, Vec2::new(1000.0, 5.0));
        assert_eq!(far, 3);
        let near = sys.hit_test(layout.handle, Vec2::new(-5.0, 5.0));
        assert_eq!(near, 0);
    }

    fn visible(sys: &SampleTextSystem, h: TextHandle) -> String {
        sys.runs[h.0].glyphs.iter().map(|g| g.parent).collect()
    }

    fn rendered_width(sys: &SampleTextSystem, h: TextHandle) -> f32 {
        sys.runs[h.0]
            .glyphs
            .iter()
            .map(|g| g.x + g.width as f32)
            .fold(0.0, f32::max)
    }

    #[test]
    fn ellipsis_is_appended_on_single_line_overflow() {
        let mut sys = sys();
        let layout = sys.prepare(
            "hello world this is long",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Ellipsis {
                    fallback: EllipsisFallback::Drop,
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(
            text.ends_with('…'),
            "expected trailing ellipsis, got {text:?}"
        );
    }

    #[test]
    fn ellipsis_on_last_line_when_height_clipped() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let layout = sys.prepare(
            "the quick brown fox jumps over the lazy dog and then keeps going",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            Rect::new(0.0, 0.0, 80.0, lh * 2.0 + 1.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert!(
            text.contains('…'),
            "expected an ellipsis somewhere, got {text:?}"
        );
    }

    #[test]
    fn center_align_centers_a_fitting_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "hi",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Drop,
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Center,
            },
            Rect::new(0.0, 0.0, 200.0, 30.0),
        );
        let first_x = sys.runs[layout.handle.0].glyphs[0].x;
        assert!(
            first_x > 50.0,
            "short line should be pushed right when centered, x={first_x}"
        );
    }

    #[test]
    fn caret_on_second_line_is_offset_in_y() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc\ndef",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let c_line2 = sys.caret_geom(layout.handle, 4);
        assert!(
            c_line2.y_top > 1.0,
            "second-line caret should sit below the first"
        );
    }

    #[test]
    fn long_unbreakable_word_is_force_broken() {
        let mut sys = sys();
        let layout = sys.prepare(
            "supercalifragilisticexpialidocious",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::WrapWord {
                    fallback: WrapWordFallback::WrapGlyph {
                        fallback: WrapGlyphFallback::Drop,
                    },
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 200.0),
        );
        let lines = sys.runs[layout.handle.0].lines.len();
        assert!(
            lines > 1,
            "expected the long word to break across lines, got {lines}"
        );
    }

    #[test]
    fn metrics_width_matches_rendered_width_after_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Drop,
            horizontal_align: HorizontalAlign::Start,
        };
        let rect = Rect::new(0.0, 0.0, 50.0, 30.0);
        let layout = sys.prepare("hello world this is long", 16.0, FontId(1), flow, rect);
        let reported = layout.metrics.size.x;
        let actual = rendered_width(&sys, layout.handle);
        assert!(
            (reported - actual).abs() < 1.0,
            "metrics width {reported} should match rendered width {actual}",
        );
    }

    #[test]
    fn center_align_keeps_overflowing_line_within_box() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 40.0, 30.0);
        let layout = sys.prepare(
            "hello world this is long",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Ellipsis {
                    fallback: EllipsisFallback::Drop,
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Center,
            },
            rect,
        );
        let left = sys.runs[layout.handle.0]
            .glyphs
            .iter()
            .map(|g| g.x)
            .fold(f32::INFINITY, f32::min);
        assert!(
            left >= -0.5,
            "centered overflow line starts off-box at x={left}"
        );
    }

    #[test]
    fn multiline_hit_test_picks_the_right_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc\ndef",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0));
        let on_line2 = sys.hit_test(layout.handle, Vec2::new(0.0, lh + lh * 0.5));
        assert_eq!(on_line2, 4);
    }

    #[test]
    fn test_optical_ink_bounds_alignment() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 500.0, 100.0);
        let layout = sys.prepare(
            "Hello World",
            16.0,
            FontId(1),
            TextFlow::single_line(),
            rect,
        );

        let run = &sys.runs[layout.handle.0];
        let l = run.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
        let r = run
            .glyphs
            .iter()
            .map(|g| g.x + g.width as f32)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(l, 0.0, "Leftmost ink pixel must be at 0.0");
        assert!(
            (layout.metrics.size.x - r).abs() < 0.001,
            "Metrics width must match tight ink width"
        );

        let caret = sys.caret_geom(layout.handle, 0);
        assert_eq!(caret.x, 0.0, "Caret at index 0 must be at x = 0.0");

        let idx = sys.hit_test(layout.handle, Vec2::new(0.0, 5.0));
        assert_eq!(idx, 0, "Hit testing near 0.0 must return index 0");
    }

    // ── Systematic unit tests ────────────────────────────────────────────────

    #[test]
    fn test_overflow_x_drop_y_drop() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Drop,
            overflow_y: OverflowY::Drop,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let run = &sys.runs[layout.handle.0];
        for g in &run.glyphs {
            assert!(g.x + g.width as f32 <= 25.0 + 0.1);
        }
        assert!(!run.glyphs.is_empty());
    }

    #[test]
    fn test_overflow_x_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        let mut line1_has_overflow = false;
        let mut line2_has_overflow = false;
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if line_glyphs
                .iter()
                .any(|g| g.x + g.width as f32 > 25.0 + 0.1)
            {
                if i == 0 {
                    line1_has_overflow = true;
                }
                if i == 1 {
                    line2_has_overflow = true;
                }
            }
        }
        assert!(line1_has_overflow);
        assert!(line2_has_overflow);
    }

    #[test]
    fn test_overflow_x_keep_y_ellipsis() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let text = visible(&sys, layout.handle);
        assert!(text.ends_with('…'));
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(last_glyph.x + last_glyph.width as f32 <= 25.0 + 0.1);
    }

    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_drop() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 1.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 1.5),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "…");
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(last_glyph.x + last_glyph.width as f32 > 8.0 + 0.1);
    }

    #[test]
    fn test_overflow_x_ellipsis_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 2.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert!(text.contains('…'));
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(last_g.x + last_g.width as f32 <= 23.0 + 0.1);
        }
    }

    #[test]
    fn test_overflow_x_ellipsis_fallback_drop_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 2.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    #[test]
    fn test_overflow_x_ellipsis_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 2.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "……");
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(last_g.x + last_g.width as f32 > 8.0 + 0.1);
        }
    }

    // Keep this test in sync with Card 1 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, 65.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello\nhello");
    }

    // Keep this test in sync with Card 2 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 6.0, 70.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 3 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 3.0, lh * 13.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 11);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello\nhello");
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            assert!(line_glyphs.len() <= 1);
        }
    }

    // Keep this test in sync with Card 4 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 48.0, lh * 4.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 5 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 10.0),
        );
        assert!(sys.runs[layout.handle.0].lines.len() > 4);
        let text = visible(&sys, layout.handle);
        let run = &sys.runs[layout.handle.0];
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                println!(
                    "line {}, char={:?}, x={}, width={}",
                    i, g.parent, g.x, g.width
                );
            }
        }
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 6 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_fallback_drop_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 3.0, lh * 10.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 7 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 3.0, lh * 25.0),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there\nhello there");
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 20);
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let visible_glyphs: Vec<_> = line_glyphs
                .iter()
                .filter(|g| g.parent != ' ' && g.parent != '\n')
                .collect();
            assert!(visible_glyphs.len() <= 1);
        }
    }

    // Keep this test in sync with Card 8 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_drop_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 5.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                assert!(g.x + g.width as f32 <= 23.0 + 0.1);
            }
        }
    }

    // Keep this test in sync with Card 9 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 5.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);
        let mut has_overflow = false;
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if let Some(last_g) = line_glyphs.last() {
                if last_g.parent != '\n'
                    && last_g.parent != ' '
                    && last_g.x + last_g.width as f32 > 23.0
                {
                    has_overflow = true;
                }
            }
        }
        assert!(has_overflow);
    }
}
