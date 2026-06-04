use fontdue::{
    layout::{
        CoordinateSystem, GlyphPosition, HorizontalAlign as FdHAlign, Layout, LayoutSettings,
        TextStyle, VerticalAlign as FdVAlign, WrapStyle,
    },
    Font, FontSettings,
};
use framewise::{
    CaretGeom, FontId, HorizontalAlign, Overflow, Rect, TextBounds, TextFlow, TextHandle,
    TextLayout, TextMetrics, TextSystem, Vec2,
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
    y_top: f32,
    height: f32,
    baseline_y: f32,
    /// Natural (unaligned, pre-ellipsis) used width of the line.
    width: f32,
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
        // When wrapping is off we pass no max_width so fontdue never inserts a
        // soft break; hard '\n' breaks still apply. We do our own horizontal
        // alignment and overflow handling, so fontdue is always Left / Top.
        let wrap_width = if flow.wrap { max_w } else { None };
        {
            let font = &self.fonts[font_id.0 as usize];
            self.layout.reset(&LayoutSettings {
                max_width: wrap_width,
                max_height: None,
                horizontal_align: FdHAlign::Left,
                vertical_align: FdVAlign::Top,
                line_height: 1.0,
                wrap_style: WrapStyle::Word,
                wrap_hard_breaks: true,
                ..LayoutSettings::default()
            });
            self.layout.append(&[font], &TextStyle::new(text, size, 0));
        }
        let glyphs0 = self.layout.glyphs().clone();
        // Copy only the line fields we need so we drop the borrow on `layout`.
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
        // Use each line's glyph_start as the boundary to the next, which avoids
        // inclusive/empty-range ambiguity in fontdue's glyph_end.
        let mut lines: Vec<Line> = Vec::new();
        if fd_lines.is_empty() {
            lines.push(Line {
                glyph_start: 0,
                glyph_end: 0,
                byte_start: 0,
                byte_end: text.len(),
                y_top: 0.0,
                height: line_height,
                baseline_y: line_height, // approximate; unused for empty text
                width: 0.0,
            });
        } else {
            for (i, &(gs, baseline_y, max_ascent, new_line)) in fd_lines.iter().enumerate() {
                let ge = if i + 1 < fd_lines.len() {
                    fd_lines[i + 1].0
                } else {
                    glyphs0.len()
                };
                let byte_start = glyphs0.get(gs).map(|g| g.byte_offset).unwrap_or(text.len());
                let width = if ge > gs {
                    let g = &glyphs0[ge - 1];
                    g.x + g.width as f32
                } else {
                    0.0
                };
                lines.push(Line {
                    glyph_start: gs,
                    glyph_end: ge,
                    byte_start,
                    byte_end: text.len(), // patched below
                    y_top: baseline_y - max_ascent,
                    height: new_line,
                    baseline_y,
                    width,
                });
            }
            // byte_end of each line is the next line's byte_start.
            for i in 0..lines.len() - 1 {
                lines[i].byte_end = lines[i + 1].byte_start;
            }
        }

        // ── Vertical overflow: cap visible line count ───────────────────────
        let max_lines = max_h
            .map(|h| ((h / line_height).floor() as usize).max(1))
            .unwrap_or(usize::MAX);
        let mut truncated_vertical = false;
        if lines.len() > max_lines {
            truncated_vertical = true;
            lines.truncate(max_lines);
            // The last visible line now absorbs the remaining selectable bytes.
            if let Some(last) = lines.last_mut() {
                last.byte_end = text.len();
            }
        }

        // ── Per-line: align, clip / ellipsis, rebuild glyph vec ─────────────
        let mut truncated_horizontal = false;
        let mut out: Vec<GlyphPosition> = Vec::with_capacity(glyphs0.len());
        let mut rec: Vec<LineRec> = Vec::with_capacity(lines.len());
        let mut block_width = 0.0_f32;

        let line_count = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let is_last = i + 1 == line_count;
            let mut seg: Vec<GlyphPosition> = glyphs0[line.glyph_start..line.glyph_end].to_vec();

            // Horizontal alignment (we do it ourselves so it also works without
            // a max_width and uniformly across wrap modes).
            let align_off = match max_w {
                Some(w) => align_offset(flow.horizontal_align, w, line.width),
                None => 0.0,
            };
            if align_off != 0.0 {
                for g in &mut seg {
                    g.x += align_off;
                }
            }

            let overflows_w = max_w.is_some_and(|w| line.width > w + 0.5);
            if overflows_w {
                truncated_horizontal = true;
            }
            let ellipsis_here = flow.overflow == Overflow::Ellipsis
                && (overflows_w || (truncated_vertical && is_last));

            if ellipsis_here {
                let (ell_glyphs, ell_w, ell_baseline) = self.ellipsis(size, font_id);
                // Trim trailing glyphs so the content + ellipsis fits the width.
                if let Some(w) = max_w {
                    let limit = align_off + w - ell_w;
                    while let Some(last) = seg.last() {
                        if (last.x + last.width as f32) <= limit {
                            break;
                        }
                        seg.pop();
                    }
                }
                let pen_x = seg
                    .last()
                    .map(|g| g.x + g.width as f32)
                    .unwrap_or(align_off);
                let dy = line.baseline_y - ell_baseline;
                for mut eg in ell_glyphs {
                    eg.x += pen_x;
                    eg.y += dy;
                    seg.push(eg);
                }
            } else if flow.overflow == Overflow::Clip && overflows_w {
                let limit = align_off + max_w.unwrap();
                while let Some(last) = seg.last() {
                    if (last.x + last.width as f32) <= limit {
                        break;
                    }
                    seg.pop();
                }
            }

            // Metrics width is the real extent of what was laid out (after any
            // alignment shift, trim, and ellipsis), measured tightly.
            let seg_w = if seg.is_empty() {
                0.0
            } else {
                let l = seg.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
                let r = seg
                    .iter()
                    .map(|g| g.x + g.width as f32)
                    .fold(f32::NEG_INFINITY, f32::max);
                r - l
            };
            block_width = block_width.max(seg_w);

            let glyph_start = out.len();
            out.extend(seg);
            rec.push(LineRec {
                y_top: line.y_top,
                height: line.height,
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

fn align_offset(align: HorizontalAlign, avail: f32, line_w: f32) -> f32 {
    let off = match align {
        HorizontalAlign::Start => 0.0,
        HorizontalAlign::Center => (avail - line_w) * 0.5,
        HorizontalAlign::End => avail - line_w,
    };
    // A line wider than the box has no room to shift; clamp so it never starts
    // off the left edge (an overflowing line falls back to start-aligned).
    off.max(0.0)
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

    /// Reconstruct the visible glyph string (including any ellipsis) from a
    /// prepared run, via each glyph's source `parent` char.
    fn visible(sys: &SampleTextSystem, h: TextHandle) -> String {
        sys.runs[h.0].glyphs.iter().map(|g| g.parent).collect()
    }

    /// Rightmost rendered edge across all glyphs in a run.
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
            TextFlow::single_line(), // single_line uses Clip…
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        // …so use an explicit ellipsis flow instead:
        let layout = sys.prepare(
            "hello world this is long",
            16.0,
            FontId(1),
            TextFlow {
                wrap: false,
                overflow: Overflow::Ellipsis,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        let _ = layout;
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
                wrap: false,
                overflow: Overflow::Clip,
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
        let c_line2 = sys.caret_geom(layout.handle, 4); // 'd', first char of line 2
        assert!(
            c_line2.y_top > 1.0,
            "second-line caret should sit below the first"
        );
    }

    // ── Probes for known sample-impl limitations ────────────────────────────

    #[test]
    fn long_unbreakable_word_is_force_broken() {
        let mut sys = sys();
        let layout = sys.prepare(
            "supercalifragilisticexpialidocious",
            16.0,
            FontId(1),
            TextFlow {
                wrap: true,
                overflow: Overflow::Clip,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 200.0),
        );
        let lines = sys.runs[layout.handle.0].lines.len();
        // Trait contract: a word wider than the box force-breaks mid-word.
        assert!(
            lines > 1,
            "expected the long word to break across lines, got {lines}"
        );
    }

    #[test]
    fn metrics_width_matches_rendered_width_after_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            wrap: false,
            overflow: Overflow::Ellipsis,
            horizontal_align: HorizontalAlign::Start,
        };
        let rect = Rect::new(0.0, 0.0, 50.0, 30.0);
        let layout = sys.prepare("hello world this is long", 16.0, FontId(1), flow, rect);
        let reported = layout.metrics.size.x;
        let actual = rendered_width(&sys, layout.handle);
        // metrics should reflect what was actually laid out, not the box width.
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
                wrap: false,
                overflow: Overflow::Ellipsis,
                horizontal_align: HorizontalAlign::Center,
            },
            rect,
        );
        let left = sys.runs[layout.handle.0]
            .glyphs
            .iter()
            .map(|g| g.x)
            .fold(f32::INFINITY, f32::min);
        // A truncated line should still start at/after the box's left edge.
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
        // Start of second line "def" is byte 4 (after "abc\n").
        assert_eq!(on_line2, 4);
    }
}
