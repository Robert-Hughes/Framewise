use super::{EllipsisFallback, LayoutGlyph, OwnedCluster, TextBackend, TextStyle};
use crate::types::Vec2;

pub(super) fn apply_ellipsis_x<B: TextBackend>(
    backend: &mut B,
    clusters: Vec<OwnedCluster<B::ShapedGlyphId>>,
    w: f32,
    style: TextStyle,
    fallback: EllipsisFallback,
    line_baseline_y: f32,
) -> Vec<OwnedCluster<B::ShapedGlyphId>> {
    let shaped = backend.shape_ellipsis(style);
    let ell_w = shaped
        .clusters
        .iter()
        .map(|cluster| cluster.advance)
        .sum::<f32>();
    let insert_byte = clusters.last().map(|cluster| cluster.byte_end).unwrap_or(0);
    let mut ell_glyphs = Vec::new();
    let mut pen_x = 0.0;
    for cluster in shaped.clusters {
        for glyph in cluster.glyphs {
            ell_glyphs.push(LayoutGlyph {
                id: glyph.id,
                origin: Vec2::new(pen_x + glyph.x, line_baseline_y + glyph.y),
                advance: glyph.advance,
                byte_start: insert_byte,
            });
        }
        pen_x += cluster.advance;
    }
    let mut ell_cluster = OwnedCluster {
        byte_start: insert_byte,
        byte_end: insert_byte,
        x: 0.0,
        advance: ell_w,
        is_hard_break: false,
        is_whitespace: false,
        is_soft_wrap_boundary: false,
        glyphs: ell_glyphs,
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
        let pen_x = trimmed.last().map(OwnedCluster::end_x).unwrap_or(0.0);
        ell_cluster.shift_x(pen_x);
        trimmed.push(ell_cluster);
        trimmed
    }
}

// ── The trait ───────────────────────────────────────────────────────────────
