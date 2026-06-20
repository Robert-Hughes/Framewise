use super::{SharedShapedText, TextStyle};
use crate::{draw::DrawGlyph, types::Vec2};
use std::hash::Hash;

/// Request for a backend-owned glyph preparation/rasterisation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrepareGlyphRequest<G> {
    /// Opaque backend glyph token produced during shaping.
    ///
    /// The token must contain any origin-independent information required by
    /// the backend to identify the glyph resource, such as raw glyph id, font
    /// face, quantised size, weight, optical size, raster mode, or equivalent
    /// backend-specific data.
    pub glyph: G,

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
/// `shape_text` receives one hard-line source segment or Framewise-owned marker
/// string at a time. Backend cluster byte ranges are UTF-8 ranges into that
/// exact input string, not into the original full source text. Framewise may
/// omit clusters from the final layout only when the selected overflow policy
/// explicitly truncates content, such as `Drop`, ellipsis fitting, or a `Drop`
/// fallback.
///
/// Shaping and glyph preparation are intentionally separate. `shape_text`
/// returns immutable shared shaping output used for stable logical layout and
/// measurement; Framewise must not mutate it. Backend cache entries may outlive
/// the cache through `Rc` references held by layouts.
///
/// A shaped glyph token is opaque to Framewise. It is produced by the backend
/// during shaping and later returned to the same backend during glyph
/// preparation. The token should include all origin-independent information
/// needed to identify the glyph resource. `prepare_glyph` receives that token
/// plus the final draw origin, and should only need to add origin-dependent
/// choices such as subpixel binning before looking up or preparing renderer
/// resources. Returned [`DrawGlyph`] values may extend outside the logical
/// layout bounds; callers that require hard pixel containment should clip or add
/// padding.
pub trait TextBackend {
    /// Opaque backend glyph token stored by Framewise after shaping.
    type ShapedGlyphToken: Copy + Eq + Hash;

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

    /// Shape text into indivisible clusters containing backend glyph tokens.
    ///
    /// `text` is a single hard-line segment or marker string. Cluster ranges
    /// are UTF-8 byte ranges into this exact input. For ordinary source text,
    /// ranges must be sorted, non-overlapping, non-empty, cover `0..text.len()`,
    /// and represent each source character exactly once.
    ///
    /// Clusters should normally correspond to shaping clusters, and must not
    /// split combining marks, ligatures, or script-shaped units in a way that
    /// would corrupt shaping. Framewise handles hard newlines before shaping by
    /// splitting source text into hard-line segments and creating its own
    /// hard-newline layout clusters.
    ///
    /// Framewise may also call this for marker text such as an overflow
    /// ellipsis; those marker ranges are later remapped internally to source
    /// text coordinates.
    fn shape_text(
        &mut self,
        text: &str,
        style: TextStyle,
    ) -> SharedShapedText<Self::ShapedGlyphToken>;

    /// Prepare a laid-out glyph token for rendering.
    ///
    /// The backend may return `None` for non-drawable glyphs such as spaces,
    /// newlines, zero-area glyphs, or failed rasterisation. The request includes
    /// the shaped token and final glyph origin; origin-independent style and
    /// resource identity should already be carried by the token.
    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphToken>,
    ) -> Option<DrawGlyph>;
}
