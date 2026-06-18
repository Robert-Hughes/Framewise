use crate::types::Rect;
use std::rc::Rc;

/// Shared immutable shaping output returned by [`TextBackend`](super::TextBackend).
///
/// Layouts may hold references after the backend cache entry has been reused or
/// evicted. Framewise treats the contents as immutable.
pub type SharedShapedText<G> = Rc<ShapedText<G>>;

/// Backend-to-Framewise shaped text output.
///
/// This is a logical shaping result only. It contains no renderer resources and
/// no final line layout. Framewise stores backend glyph tokens inside clusters
/// and later returns them to the same backend during glyph preparation.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText<G> {
    /// Shaped clusters in source order.
    pub clusters: Vec<ShapedCluster<G>>,
}

/// One indivisible shaped cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedCluster<G> {
    /// Byte range of the original string represented by this cluster.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Logical advance used by wrapping, caret placement, and hit-testing.
    pub advance: f32,
    /// True for Unicode whitespace clusters.
    pub is_whitespace: bool,
    /// Approximate raster-independent ink bounds in cluster/baseline-local coordinates.
    ///
    /// This is the union of visible glyph ink bounds translated by each shaped
    /// glyph's cluster-local offset. `Rect::ZERO` means known no visible ink.
    pub approx_ink_bounds: Rect,
    /// Glyph tokens and metrics belonging to this cluster.
    pub glyphs: Vec<ShapedGlyph<G>>,
}

/// One shaped glyph inside a cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph<G> {
    /// Opaque backend glyph token produced during shaping.
    ///
    /// Framewise stores this value and returns it to the same backend during
    /// glyph preparation. Production backends should include any
    /// origin-independent resource identity needed to prepare or rasterise the
    /// glyph efficiently.
    pub token: G,
    /// Position relative to the cluster/glyph run before Framewise wrapping and
    /// final line placement.
    pub x: f32,
    /// Position relative to the line baseline before Framewise wrapping and
    /// final line placement.
    pub y: f32,
    /// Shaped advance used by text flow.
    pub advance: f32,
    /// Approximate raster-independent ink bounds relative to this glyph origin.
    ///
    /// Backends should normally derive this from glyph outline/control bounds
    /// after applying font size, variations, and style choices. It must not
    /// depend on final draw origin, subpixel bins, hinting, atlas allocation, or
    /// renderer resource size.
    ///
    /// `Rect::ZERO` means the glyph is known to draw no ink. Backends that
    /// cannot get outline bounds must synthesize a conservative
    /// raster-independent estimate for visible glyphs.
    pub approx_ink_bounds: Rect,
}

pub fn union_approx_ink_bounds(acc: Option<Rect>, rect: Rect) -> Option<Rect> {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return acc;
    }

    Some(match acc {
        Some(existing) => {
            let left = existing.x.min(rect.x);
            let top = existing.y.min(rect.y);
            let right = existing.right().max(rect.right());
            let bottom = existing.bottom().max(rect.bottom());
            Rect::from_ltrb(left, top, right, bottom)
        }
        None => rect,
    })
}

pub fn cluster_approx_ink_bounds<G>(glyphs: &[ShapedGlyph<G>]) -> Rect {
    glyphs
        .iter()
        .filter_map(|glyph| {
            let rect = glyph.approx_ink_bounds;
            (rect.w > 0.0 && rect.h > 0.0)
                .then(|| Rect::new(glyph.x + rect.x, glyph.y + rect.y, rect.w, rect.h))
        })
        .fold(None, union_approx_ink_bounds)
        .unwrap_or(Rect::ZERO)
}
