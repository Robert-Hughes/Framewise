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

use crate::text::types::{
    GlyphInfo, GlyphKey, PreparedGlyphImage, PreparedGlyphResources, SampleGlyphToken,
};
use framewise::{
    DrawGlyph, PrepareGlyphRequest, PreparedGlyphHandle, SharedShapedText, TextBackend,
    TextLineLayoutMetrics, Vec2,
};
use std::collections::HashMap;
use std::rc::Rc;
use swash::scale::ScaleContext;
use swash::shape::ShapeContext;
use swash::FontRef;

const MAX_SHAPE_CACHE_ENTRIES: usize = 4096;
const FLOAT_KEY_SCALE: f32 = 1024.0;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct ShapeCacheKey {
    text: String,
    font_id: u16,
    size: i64,
    weight: u16,
    letter_spacing_px: i64,
    opsz: i64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RawFontLineMetrics {
    pub units_per_em: f32,
    pub ascent: f32,
    pub descent: f32,
    pub leading: f32,
}

pub struct SampleTextBackend {
    pub fonts: Vec<FontRef<'static>>,
    pub(crate) raw_line_metrics: Vec<RawFontLineMetrics>,
    pub font_opsz_ranges: Vec<(f32, f32)>, // (min, max) for each font's opsz axis
    pub font_has_wght: Vec<bool>,          // Whether each font has a wght axis
    pub font_has_opsz: Vec<bool>,          // Whether each font has an opsz axis
    pub shape_context: ShapeContext,
    pub scale_context: ScaleContext,
    pub(crate) shape_cache: HashMap<ShapeCacheKey, SharedShapedText<SampleGlyphToken>>,
    #[cfg(test)]
    pub(crate) shape_text_run_count: usize,
    // Atlas data
    pub glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    pub prepared_glyph_keys: Vec<GlyphKey>,
    pub prepared_glyph_handles: HashMap<GlyphKey, PreparedGlyphHandle>,
    pub atlas_data: Vec<u8>,
    pub atlas_size: u32,

    // Simple shelf allocator
    pub current_x: u32,
    pub current_y: u32,
    pub row_height: u32,

    pub atlas_dirty: bool,
}

impl SampleTextBackend {
    pub fn new() -> Self {
        let jetbrains_mono_data =
            include_bytes!("../../assets/JetBrains_Mono/JetBrainsMono-VariableFont_wght.ttf")
                as &[u8];
        let jetbrains_mono = FontRef::from_index(jetbrains_mono_data, 0)
            .expect("failed to load JetBrainsMono variable font");

        // Load Inter variable font
        let inter_data =
            include_bytes!("../../assets/Inter/Inter-VariableFont_opsz,wght.ttf") as &[u8];
        let inter = FontRef::from_index(inter_data, 0).expect("failed to load Inter variable font");

        // Inter Tight - specialised for "Hero headings, landing pages, article titles". Slightly different!
        let inter_tight_data =
            include_bytes!("../../assets/Inter_Tight/InterTight-VariableFont_wght.ttf") as &[u8];
        let inter_tight = FontRef::from_index(inter_tight_data, 0)
            .expect("failed to load Inter Tight variable font");

        let fonts = vec![jetbrains_mono, inter, inter_tight];

        let raw_line_metrics = fonts
            .iter()
            .map(|font| {
                // For now, get metrics without variations - they should be similar enough.
                // TODO: Consider if we need to normalize coords for metrics.
                let metrics = font.metrics(&[]);
                RawFontLineMetrics {
                    units_per_em: metrics.units_per_em as f32,
                    ascent: metrics.ascent,
                    descent: metrics.descent,
                    leading: metrics.leading,
                }
            })
            .collect();

        // Detect supported variation axes for each font
        let mut font_has_wght = Vec::new();
        let mut font_has_opsz = Vec::new();
        for font in &fonts {
            let mut has_wght = false;
            let mut has_opsz = false;
            for var in font.variations() {
                let tag = var.tag();
                if tag == 0x77676874 {
                    // 'wght'
                    has_wght = true;
                }
                if tag == 0x6F70737A {
                    // 'opsz'
                    has_opsz = true;
                }
            }
            font_has_wght.push(has_wght);
            font_has_opsz.push(has_opsz);
        }

        // Extract opsz range from each font
        let mut font_opsz_ranges = Vec::new();

        // JetBrainsMono has no opsz axis
        font_opsz_ranges.push((0.0, 0.0));

        // Inter has opsz axis - extract its range
        let opsz_range = {
            let variations = fonts[1].variations();
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

        // Inter Tight has no opsz axis
        font_opsz_ranges.push((0.0, 0.0));

        let atlas_size = 1024;
        Self {
            fonts,
            raw_line_metrics,
            font_opsz_ranges,
            font_has_wght,
            font_has_opsz,
            shape_context: ShapeContext::new(),
            scale_context: ScaleContext::new(),
            shape_cache: HashMap::new(),
            #[cfg(test)]
            shape_text_run_count: 0,
            glyph_cache: HashMap::new(),
            prepared_glyph_keys: Vec::new(),
            prepared_glyph_handles: HashMap::new(),
            atlas_data: vec![0; (atlas_size * atlas_size) as usize],
            atlas_size,
            current_x: 0,
            current_y: 0,
            row_height: 0,
            atlas_dirty: false,
        }
    }

    pub fn begin_frame(&mut self) {
        self.atlas_dirty = false;
    }

    fn shape_cache_key(&self, text: &str, style: framewise::TextStyle) -> ShapeCacheKey {
        let opsz = self.opsz_for_size(style.size, style.font);
        ShapeCacheKey {
            text: text.to_owned(),
            font_id: style.font.0,
            size: quantize_float_key(style.size),
            weight: style.weight,
            letter_spacing_px: quantize_float_key(style.size * style.letter_spacing),
            opsz: quantize_float_key(opsz),
        }
    }

    pub fn prepare_glyph_handle(&mut self, key: GlyphKey) -> PreparedGlyphHandle {
        if let Some(handle) = self.prepared_glyph_handles.get(&key) {
            return *handle;
        }

        self.ensure_glyph(key);
        let handle = PreparedGlyphHandle(self.prepared_glyph_keys.len() as u32);
        self.prepared_glyph_keys.push(key);
        self.prepared_glyph_handles.insert(key, handle);
        handle
    }
}

impl Default for SampleTextBackend {
    fn default() -> Self {
        Self::new()
    }
}

fn quantize_float_key(value: f32) -> i64 {
    if !value.is_finite() {
        return if value.is_sign_negative() {
            i64::MIN
        } else {
            i64::MAX
        };
    }

    (value * FLOAT_KEY_SCALE).round() as i64
}

pub(crate) fn glyph_size_key(size: f32) -> u32 {
    (size * 10.0) as u32
}

pub(crate) fn glyph_opsz_key(opsz: f32) -> u16 {
    opsz as u16
}

fn subpixel_bin(x: f32) -> u8 {
    ((x * 4.0).round() as i32).rem_euclid(4) as u8
}

impl PreparedGlyphResources for SampleTextBackend {
    fn resolve_glyph(&self, handle: PreparedGlyphHandle) -> Option<PreparedGlyphImage> {
        let key = self.prepared_glyph_keys.get(handle.0 as usize)?;
        let info = self.glyph_cache.get(key)?;
        Some(PreparedGlyphImage {
            atlas_rect: info.atlas_rect,
        })
    }
}

impl TextBackend for SampleTextBackend {
    type ShapedGlyphToken = SampleGlyphToken;

    fn line_metrics(&mut self, style: framewise::TextStyle) -> TextLineLayoutMetrics {
        self.line_layout_metrics(style.size, style.font, style.line_height)
    }

    fn line_height(&mut self, style: framewise::TextStyle) -> f32 {
        Self::line_height(self, style.size, style.font, style.line_height)
    }

    fn shape_text(
        &mut self,
        text: &str,
        style: framewise::TextStyle,
    ) -> SharedShapedText<Self::ShapedGlyphToken> {
        // Cache whole hard-line segments: sample shaping depends on text plus
        // font/variation inputs, not Framewise layout bounds, wrapping,
        // alignment, overflow, or final draw origin. The final origin is still
        // handled by prepare_glyph for subpixel bins and atlas resources.
        let key = self.shape_cache_key(text, style);
        if let Some(shaped) = self.shape_cache.get(&key) {
            return Rc::clone(shaped);
        }

        let shaped = Rc::new(self.shape_text_run(text, style));
        if self.shape_cache.len() >= MAX_SHAPE_CACHE_ENTRIES {
            self.shape_cache.clear();
        }
        self.shape_cache.insert(key, Rc::clone(&shaped));
        shaped
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphToken>,
    ) -> Option<DrawGlyph> {
        let subpixel_x = subpixel_bin(request.glyph_origin.x);
        let key = GlyphKey {
            font_id: request.glyph.font_id,
            glyph_index: request.glyph.glyph_index,
            size: request.glyph.size,
            weight: request.glyph.weight,
            opsz: request.glyph.opsz,
            subpixel_x,
        };

        let handle = self.prepare_glyph_handle(key);
        let info = self.glyph_cache.get(&key)?;
        if info.atlas_rect.w == 0 || info.atlas_rect.h == 0 {
            return None;
        }

        let quantized_x = (request.glyph_origin.x * 4.0).round() / 4.0;
        Some(DrawGlyph {
            handle,
            top_left: Vec2::new(
                quantized_x.floor() + info.left as f32,
                request.glyph_origin.y.round() - info.top as f32,
            ),
        })
    }
}
