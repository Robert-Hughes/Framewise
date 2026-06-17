use super::{
    EllipsisFallback, TextBackend, TextStyle, WorkingCluster, WorkingClusterSource, WorkingRun,
};
use std::marker::PhantomData;

const ELLIPSIS_MARKER: &str = "\u{2026}";

pub(super) fn apply_ellipsis_x<B: TextBackend>(
    backend: &mut B,
    runs: &mut Vec<WorkingRun<B::ShapedGlyphId>>,
    clusters: Vec<WorkingCluster<B::ShapedGlyphId>>,
    w: f32,
    style: TextStyle,
    fallback: EllipsisFallback,
) -> Vec<WorkingCluster<B::ShapedGlyphId>> {
    let shaped = backend.shape_text(ELLIPSIS_MARKER, style);
    let run_index = runs.len();
    let ell_w = shaped
        .clusters
        .iter()
        .map(|cluster| cluster.advance)
        .sum::<f32>();
    let insert_byte = clusters.last().map(|cluster| cluster.byte_end).unwrap_or(0);
    runs.push(WorkingRun {
        shaped,
        segment_start: insert_byte,
    });
    let mut ell_cluster = WorkingCluster {
        source: WorkingClusterSource::Shaped {
            run_index,
            cluster_index: 0,
        },
        byte_start: insert_byte,
        byte_end: insert_byte,
        x: 0.0,
        advance: ell_w,
        is_hard_break: false,
        is_whitespace: false,
        is_soft_wrap_boundary: false,
        glyphs_visible: true,
        _marker: PhantomData,
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
