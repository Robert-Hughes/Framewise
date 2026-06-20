use super::{
    EllipsisFallback, TextBackend, TextStyle, WorkingCluster, WorkingClusterSource, WorkingRun,
};

const ELLIPSIS_MARKER: &str = "\u{2026}";

pub(super) fn apply_ellipsis_x<B: TextBackend>(
    backend: &mut B,
    runs: &mut Vec<WorkingRun<B::ShapedGlyphToken>>,
    clusters: Vec<WorkingCluster>,
    w: f32,
    style: TextStyle,
    fallback: EllipsisFallback,
) -> Vec<WorkingCluster> {
    let shaped = backend.shape_text(ELLIPSIS_MARKER, style);
    let run_index = runs.len();
    let ell_w = shaped
        .clusters
        .iter()
        .map(|cluster| cluster.advance)
        .sum::<f32>();
    let source_start_byte = clusters
        .first()
        .map(|cluster| cluster.byte_start)
        .unwrap_or(0);

    if ell_w > w {
        match fallback {
            EllipsisFallback::Keep => {
                runs.reserve(1);
                runs.push(WorkingRun {
                    shaped,
                    segment_start: source_start_byte,
                });
                vec![ellipsis_cluster(run_index, source_start_byte, ell_w)]
            }
            EllipsisFallback::Drop => Vec::new(),
        }
    } else {
        let limit = w - ell_w;
        let mut trimmed = Vec::with_capacity(clusters.len().saturating_add(1));
        for cluster in clusters {
            if cluster.end_x() <= limit {
                trimmed.push(cluster);
            } else {
                break;
            }
        }
        let pen_x = trimmed.last().map(WorkingCluster::end_x).unwrap_or(0.0);
        let insert_byte = trimmed
            .last()
            .map(|cluster| cluster.byte_end)
            .unwrap_or(source_start_byte);
        runs.reserve(1);
        runs.push(WorkingRun {
            shaped,
            segment_start: insert_byte,
        });
        let mut ell_cluster = ellipsis_cluster(run_index, insert_byte, ell_w);
        ell_cluster.shift_x(pen_x);
        trimmed.push(ell_cluster);
        trimmed
    }
}

fn ellipsis_cluster(run_index: usize, insert_byte: usize, advance: f32) -> WorkingCluster {
    WorkingCluster {
        source: WorkingClusterSource::Shaped {
            run_index,
            cluster_index: 0,
        },
        byte_start: insert_byte,
        byte_end: insert_byte,
        x: 0.0,
        advance,
        is_hard_break: false,
        is_whitespace: false,
        is_soft_wrap_boundary: false,
        glyphs_visible: true,
    }
}
