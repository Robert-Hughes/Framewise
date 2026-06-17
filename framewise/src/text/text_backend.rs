use super::{SharedShapedText, TextStyle};
use crate::{draw::DrawGlyph, types::Vec2};
use std::hash::Hash;

/// Request for a backend-owned glyph preparation/rasterisation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrepareGlyphRequest<G> {
    /// Backend-shaped glyph identifier to prepare.
    pub glyph: G,
    /// Text style used when this glyph was shaped and laid out.
    pub style: TextStyle,

    /// Final logical-pixel origin of the shaped glyph.
    ///
    /// This is after layout, wrapping, line alignment, baseline placement, and
    /// caller draw origin have all been applied. The backend may use this for
    /// subpixel bin selection and returns a [`DrawGlyph`] with bitmap placement
    /// applied.
    ///
    /// The final screen position must be known at this stage because modern text
    /// shapers and font rasterizers may use absolute physical screen coordinates
    /// to apply subpixel offsets and positioning. This ensures crisp glyph
    /// rasterization at fractional pixel boundaries and prevents blurriness.
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
///
/// The backend must account for every source character in shaped cluster byte
/// ranges. Framewise may omit clusters from the final layout only when the
/// selected overflow policy explicitly truncates content, such as `Drop`,
/// ellipsis fitting, or a `Drop` fallback.
///
/// Shaping and glyph preparation are intentionally separate. `shape_text`
/// returns immutable shared shaping output used for stable logical layout and
/// measurement; Framewise must not mutate it. Backend cache entries may outlive
/// the cache through `Rc` references held by layouts. `prepare_glyph` is called
/// later with the final draw origin included so the backend can choose subpixel
/// bins, hinting, and renderer resources using absolute logical-pixel position.
/// Returned [`DrawGlyph`] values may extend outside the logical layout bounds;
/// callers that require hard pixel containment should clip or add padding.
pub trait TextBackend {
    type ShapedGlyphId: Copy + Eq + Hash;

    /// Backend-provided vertical metrics for laying out one text line.
    ///
    /// The default baseline is `style.size` for simple backends. Font-aware
    /// backends should override this to use real typographic baseline metrics.
    fn line_metrics(&mut self, style: TextStyle) -> TextLineLayoutMetrics {
        TextLineLayoutMetrics {
            line_height: self.line_height(style),
            baseline_offset: style.size,
        }
    }

    /// Distance between consecutive line tops for this style.
    fn line_height(&mut self, style: TextStyle) -> f32;

    /// Shape text into indivisible clusters.
    ///
    /// The backend must account for every source character in cluster byte
    /// ranges. Clusters should normally correspond to shaping clusters, and must
    /// not split combining marks, ligatures, or script-shaped units in a way
    /// that would corrupt shaping. Framewise may also use this API for
    /// Framewise-owned synthetic UI marker text, such as an overflow ellipsis,
    /// then remap those marker byte ranges internally to source text
    /// coordinates.
    fn shape_text(&mut self, text: &str, style: TextStyle)
        -> SharedShapedText<Self::ShapedGlyphId>;

    /// Prepare a laid-out glyph for rendering.
    ///
    /// The backend may return `None` for non-drawable glyphs such as spaces,
    /// newlines, zero-area glyphs, or failed rasterisation.
    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph>;
}
