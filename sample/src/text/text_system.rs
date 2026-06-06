//! # Framewise Text System (Swash Migration)
//!
//! ## Rationale for choosing Swash
//!
//! Originally, Framewise used `fontdue` for text rendering. While `fontdue` is extremely fast and
//! lightweight, it has several major limitations that prevent a high-end, premium UI look:
//! - **No Variable Font Support:** It cannot dynamically adapt font weights, widths, or optical sizes,
//!   which is critical for clean typography across headings and micro-labels.
//! - **No Subpixel Positioning:** It does not support rendering glyphs at fractional offsets, causing
//!   either blurry text (with bilinear filtering) or uneven letter spacing/kerning (with pixel snapping).
//! - **No Autohinting:** It lacks sophisticated hinting routines, making text shapes look slightly soft
//!   and uneven at small pixel heights.
//!
//! We evaluated alternative pure-Rust options:
//! - **`ab_glyph`:** Good for basic GUIs, but has limited shaping capabilities (no ligatures or OpenType features)
//!   and a simpler scaler.
//! - **`cosmic-text`:** A massive library that handles layout, shaping, bi-directional text, and fallbacks.
//!   However, it is too heavy and would require discarding our custom line-wrapping, spacing, and layout policies,
//!   which are optimized specifically for Framewise.
//! - **`swash` (Selected):** Provides state-of-the-art OpenType shaping, variable font axis matching,
//!   and a premium scaler with autohinting and fractional subpixel rendering. It lets us retain full control
//!   over Framewise's custom line-wrapping and manual/wrapped layout algorithms.
//!
//! ## Design Selection
//! - **Grayscale Anti-Aliasing (Alpha coverage):** We use grayscale AA rather than RGB subpixel rendering.
//!   Grayscale is standard across modern operating systems (iOS, Android, macOS, and modern Windows Store apps)
//!   because it is robust under high-DPI scaling, transformations, subpixel layouts, and OLED/rotation changes.
//! - **Horizontal Subpixel Snapping (4 Bins):** Snaps glyphs to 4 horizontal fractional bins (0.0, 0.25, 0.5, 0.75).
//!   Since vertical baselines in standard UIs are always snapped to integer pixels, we do not perform vertical
//!   subpixel positioning, conserving texture atlas space.
//! - **Nearest-Neighbor Filtering:** In `renderer.rs`, we sample the font atlas using Nearest-neighbor filtering.
//!   By rendering pixel-aligned quads with pre-shifted subpixel glyphs, we map every font pixel 1-to-1 with screen
//!   pixels for maximum crispness.

use crate::text::types::{CachedLayout, GlyphInfo, GlyphKey};
use framewise::{
    CaretGeom, FontId, Rect, TextBounds, TextFlow, TextLayout, TextMetrics, TextSystem, Vec2,
};
use std::collections::HashMap;
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use swash::FontRef;

pub struct SampleTextSystem {
    pub fonts: Vec<FontRef<'static>>,
    pub font_opsz_ranges: Vec<(f32, f32)>, // (min, max) for each font's opsz axis
    pub font_weights: Vec<u16>, // Current weight setting for each font (for variable fonts)
    pub shape_context: ShapeContext,
    pub scale_context: ScaleContext,
    pub runs: Vec<CachedLayout>,

    // Atlas data
    pub glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    pub atlas_data: Vec<u8>,
    pub atlas_size: u32,

    // Simple shelf allocator
    pub current_x: u32,
    pub current_y: u32,
    pub row_height: u32,

    pub atlas_dirty: bool,
}

impl SampleTextSystem {
    pub fn new() -> Self {
        let mono_data = include_bytes!("../../assets/JetBrainsMono-Regular.ttf") as &[u8];
        let mono = FontRef::from_index(mono_data, 0).expect("failed to load JetBrainsMono font");

        // Load Inter variable font (replaces InterTight-Regular and InterTight-Bold)
        let sans_data =
            include_bytes!("../../assets/Inter/Inter-VariableFont_opsz,wght.ttf") as &[u8];
        let sans = FontRef::from_index(sans_data, 0).expect("failed to load Inter variable font");

        // Extract opsz range from each font
        let mut font_opsz_ranges = Vec::new();

        // JetBrainsMono has no opsz axis
        font_opsz_ranges.push((0.0, 0.0));

        // Inter has opsz axis - extract its range
        let opsz_range = {
            let variations = sans.variations();
            let mut range = (14.0, 32.0); // fallback to documented range
            for var in variations {
                let tag = var.tag();
                // 'opsz' = 0x6F70737A
                if tag == 0x6F70737A {
                    // Note: swash Variation doesn't expose min/max directly in all versions
                    // Use documented Inter range for now
                    range = (14.0, 32.0);
                    break;
                }
            }
            range
        };
        font_opsz_ranges.push(opsz_range);

        let atlas_size = 1024;
        Self {
            fonts: vec![mono, sans],
            font_opsz_ranges,
            font_weights: vec![400, 400], // Default weights for each font
            shape_context: ShapeContext::new(),
            scale_context: ScaleContext::new(),
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

    /// Set the weight to use for a specific font. Used to control variable font weight axis.
    pub fn set_font_weight(&mut self, font_id: FontId, weight: u16) {
        if (font_id.0 as usize) < self.font_weights.len() {
            self.font_weights[font_id.0 as usize] = weight;
        }
    }

    pub fn begin_frame(&mut self) {
        self.runs.clear();
        self.atlas_dirty = false;
    }
}

impl TextSystem for SampleTextSystem {
    fn measure(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        weight: u16,
        flow: TextFlow,
        bounds: TextBounds,
    ) -> TextMetrics {
        // Temporarily set the weight for this font before shaping
        let old_weight = self
            .font_weights
            .get(font.0 as usize)
            .copied()
            .unwrap_or(400);
        self.set_font_weight(font, weight);

        let (_glyphs, _lines, metrics) = self.shape_internal(
            text,
            size,
            font,
            flow,
            bounds.max_width,
            bounds.max_height,
            None,
        );

        // Restore old weight
        self.set_font_weight(font, old_weight);
        metrics
    }

    fn prepare(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        weight: u16,
        flow: TextFlow,
        rect: Rect,
    ) -> TextLayout {
        // Temporarily set the weight for this font before shaping
        let old_weight = self
            .font_weights
            .get(font.0 as usize)
            .copied()
            .unwrap_or(400);
        self.set_font_weight(font, weight);

        // Pass the absolute X coordinate (rect.x) to internal shaper to compute correct subpixel offsets
        let (glyphs, lines, metrics) = self.shape_internal(
            text,
            size,
            font,
            flow,
            Some(rect.w),
            Some(rect.h),
            Some(rect.x),
        );

        let opsz = self.opsz_for_size(size, font);

        for g in &glyphs {
            let key = GlyphKey {
                font_id: font.0,
                glyph_index: g.key.glyph_index,
                size: (g.key.px * 10.0) as u32,
                subpixel_x: g.subpixel_x,
                weight,
                opsz: opsz as u16,
            };
            self.ensure_glyph(key);
        }

        let handle_id = self.runs.len();
        self.runs.push(CachedLayout {
            font_id: font,
            glyphs,
            lines,
        });

        // Restore old weight
        self.set_font_weight(font, old_weight);

        TextLayout {
            handle: framewise::TextHandle(handle_id),
            metrics,
        }
    }

    fn caret_geom(&self, handle: framewise::TextHandle, byte_index: usize) -> CaretGeom {
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
                .map(|g| g.x + g.advance)
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

    fn hit_test(&self, handle: framewise::TextHandle, pos: Vec2) -> usize {
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

fn line_start_x(glyphs: &[crate::text::types::GlyphPosition]) -> f32 {
    glyphs.first().map(|g| g.x).unwrap_or(0.0)
}
