use super::{
    EllipsisFallback, LayoutGlyph, TextBackend, TextStyle, WorkingCluster, WorkingClusterSource,
};
use crate::types::Vec2;

const ELLIPSIS_MARKER: &str = "\u{2026}";

pub(super) fn apply_ellipsis_x<B: TextBackend>(
    backend: &mut B,
    clusters: Vec<WorkingCluster<B::ShapedGlyphId>>,
    w: f32,
    style: TextStyle,
    fallback: EllipsisFallback,
    _line_baseline_y: f32,
) -> Vec<WorkingCluster<B::ShapedGlyphId>> {
    let shaped = backend.shape_text(ELLIPSIS_MARKER, style);
    let ell_w = shaped
        .clusters
        .iter()
        .map(|cluster| cluster.advance)
        .sum::<f32>();
    let insert_byte = clusters.last().map(|cluster| cluster.byte_end).unwrap_or(0);
    let mut ell_glyphs = Vec::new();
    let mut pen_x = 0.0;
    for cluster in &shaped.clusters {
        for glyph in &cluster.glyphs {
            ell_glyphs.push(LayoutGlyph {
                id: glyph.id,
                origin: Vec2::new(pen_x + glyph.x, glyph.y),
                advance: glyph.advance,
                byte_start: insert_byte,
                approx_ink_bounds: glyph.approx_ink_bounds,
            });
        }
        pen_x += cluster.advance;
    }
    let mut ell_cluster = WorkingCluster {
        source: WorkingClusterSource::SyntheticGlyphs { glyphs: ell_glyphs },
        byte_start: insert_byte,
        byte_end: insert_byte,
        x: 0.0,
        advance: ell_w,
        is_hard_break: false,
        is_whitespace: false,
        is_soft_wrap_boundary: false,
        glyphs_visible: true,
    };

    if ell_w > w {
        match fallback {
            EllipsisFallback::Keep => vec![ell_cluster],
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
        let pen_x = trimmed.last().map(WorkingCluster::end_x).unwrap_or(0.0);
        ell_cluster.shift_x(pen_x);
        trimmed.push(ell_cluster);
        trimmed
    }
}
