use super::{ShapedText, TextStyle};
use crate::{draw::DrawGlyph, types::Vec2};
use std::hash::Hash;

/// Request for a backend-owned glyph preparation/rasterisation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrepareGlyphRequest<G> {
    pub glyph: G,
    pub style: TextStyle,

    /// Final logical-pixel origin of the shaped glyph.
    ///
    /// This is after layout, wrapping, line alignment, baseline placement, and
    /// caller draw origin have all been applied. The backend may use this for
    /// subpixel bin selection and returns a [`DrawGlyph`] with bitmap placement
    /// applied.
    pub glyph_origin: Vec2,
}

/// Backend-provided vertical metrics for laying out one text line.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLineLayoutMetrics {
    /// Distance between consecutive line tops.
    pub line_height: f32,
    /// Distance from the line top to the baseline.
    pub baseline_offset: f32,
}

/// Low-level text backend contract used by Framewise-owned text layout.
///
/// Framewise owns layout policy; the backend owns font selection, shaping,
/// glyph rasterisation, glyph caching, and renderer resource handles.
pub trait TextBackend {
    type ShapedGlyphId: Copy + Eq + Hash;

    fn line_metrics(&mut self, style: TextStyle) -> TextLineLayoutMetrics {
        TextLineLayoutMetrics {
            line_height: self.line_height(style),
            baseline_offset: style.size,
        }
    }

    fn line_height(&mut self, style: TextStyle) -> f32;

    fn shape_text(&mut self, text: &str, style: TextStyle) -> ShapedText<Self::ShapedGlyphId>;

    fn shape_ellipsis(&mut self, style: TextStyle) -> ShapedText<Self::ShapedGlyphId>;

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph>;
}
