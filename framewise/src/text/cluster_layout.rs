use super::{
    TextBackend, TextStyle, WorkingCluster, WorkingClusterSource, WorkingRun, WorkingSourceLine,
    WrapClusterFallback, WrapWordFallback,
};

#[allow(clippy::too_many_arguments)]
pub(super) fn make_source_line<B: TextBackend>(
    backend: &mut B,
    runs: &mut Vec<WorkingRun<B::ShapedGlyphId>>,
    text: &str,
    style: TextStyle,
    segment_start: usize,
    segment_end: usize,
    has_newline: bool,
    line_idx: usize,
    line_height: f32,
    baseline_offset: f32,
) -> WorkingSourceLine<B::ShapedGlyphId> {
    let segment = &text[segment_start..segment_end];
    let baseline_y = line_idx as f32 * line_height + baseline_offset;
    let mut clusters = Vec::new();

    if !segment.is_empty() {
        let shaped = backend.shape_text(segment, style);
        let run_index = runs.len();
        runs.push(WorkingRun {
            shaped,
            segment_start,
        });
        for (cluster_index, shaped_cluster) in runs[run_index].shaped.clusters.iter().enumerate() {
            let byte_start = segment_start + shaped_cluster.byte_start;
            let byte_end = segment_start + shaped_cluster.byte_end;
            let x = clusters.last().map(WorkingCluster::end_x).unwrap_or(0.0);
            clusters.push(WorkingCluster {
                source: WorkingClusterSource::Shaped {
                    run_index,
                    cluster_index,
                },
                byte_start,
                byte_end,
                x,
                advance: shaped_cluster.advance,
                is_hard_break: false,
                is_whitespace: shaped_cluster.is_whitespace,
                is_soft_wrap_boundary: false,
                glyphs_visible: true,
            });
        }
    }

    if has_newline {
        let x = clusters.last().map(WorkingCluster::end_x).unwrap_or(0.0);
        clusters.push(WorkingCluster {
            source: WorkingClusterSource::Empty,
            byte_start: segment_end,
            byte_end: segment_end + 1,
            x,
            advance: 0.0,
            is_hard_break: true,
            is_whitespace: true,
            is_soft_wrap_boundary: false,
            glyphs_visible: false,
        });
    }

    WorkingSourceLine {
        clusters,
        byte_start: segment_start,
        byte_end: if has_newline {
            segment_end + 1
        } else {
            segment_end
        },
        baseline_y,
    }
}

pub(super) fn logical_cluster_line_width<G>(clusters: &[WorkingCluster<G>]) -> f32 {
    let start = logical_cluster_line_start(clusters);
    clusters
        .iter()
        .map(WorkingCluster::end_x)
        .fold(start, f32::max)
        - start
}

pub(super) fn logical_cluster_line_start<G>(clusters: &[WorkingCluster<G>]) -> f32 {
    clusters
        .iter()
        .map(|cluster| cluster.x)
        .reduce(f32::min)
        .unwrap_or(0.0)
}

pub(super) fn append_empty_after_terminal_soft_wrap_boundary<G>(
    lines: &mut Vec<Vec<WorkingCluster<G>>>,
    source_byte_end: usize,
) {
    let has_terminal_boundary = lines
        .last()
        .and_then(|line| line.last())
        .is_some_and(|cluster| {
            cluster.is_soft_wrap_boundary && cluster.byte_end == source_byte_end
        });
    if has_terminal_boundary {
        lines.push(Vec::new());
    }
}

fn collapse_trailing_soft_wrap_space<G>(clusters: &mut [WorkingCluster<G>]) {
    let has_non_whitespace_content = clusters
        .iter()
        .rev()
        .skip(1)
        .any(|cluster| !cluster.is_whitespace && !cluster.is_hard_break);
    if has_non_whitespace_content {
        if let Some(cluster) = clusters
            .last_mut()
            .filter(|cluster| cluster.is_whitespace && !cluster.is_hard_break)
        {
            cluster.collapse_soft_wrap_boundary();
        }
    }
}

pub(super) fn wrap_clusters<G: Clone>(
    clusters: Vec<WorkingCluster<G>>,
    w: f32,
    fallback: WrapClusterFallback,
) -> Vec<Vec<WorkingCluster<G>>> {
    let mut lines: Vec<Vec<WorkingCluster<G>>> = Vec::new();
    if clusters.is_empty() {
        return vec![Vec::new()];
    }
    let mut current_line = Vec::new();
    let mut current_line_start_x = clusters[0].x;

    for cluster in clusters {
        if cluster.is_hard_break {
            let mut moved = cluster;
            let mut appended = false;
            if current_line.is_empty() {
                if let Some(last_line) = lines.last_mut() {
                    if last_line
                        .last()
                        .map(|c: &WorkingCluster<G>| c.is_hard_break)
                        != Some(true)
                    {
                        moved.shift_x(-moved.x);
                        last_line.push(moved.clone());
                        appended = true;
                    }
                }
            }
            if !appended {
                moved.shift_x(-current_line_start_x);
                current_line.push(moved);
                lines.push(current_line);
                current_line = Vec::new();
            }
            continue;
        }

        let rel_start_x = cluster.x - current_line_start_x;
        let rel_end_x = rel_start_x + cluster.advance;

        if rel_end_x <= w {
            let mut moved = cluster;
            moved.shift_x(rel_start_x - moved.x);
            current_line.push(moved);
        } else if cluster.is_whitespace && !current_line.is_empty() {
            let next_line_start_x = cluster.x + cluster.advance;
            let mut moved = cluster;
            moved.shift_x(rel_start_x - moved.x);
            moved.collapse_soft_wrap_boundary();
            current_line.push(moved);
            lines.push(current_line);
            current_line = Vec::new();
            current_line_start_x = next_line_start_x;
        } else if current_line.is_empty() {
            match fallback {
                WrapClusterFallback::Keep => {
                    let mut moved = cluster;
                    moved.shift_x(rel_start_x - moved.x);
                    current_line.push(moved);
                    lines.push(current_line);
                    current_line = Vec::new();
                    current_line_start_x += rel_end_x;
                }
                WrapClusterFallback::Drop => break,
            }
        } else {
            collapse_trailing_soft_wrap_space(&mut current_line);
            lines.push(current_line);
            current_line = Vec::new();
            current_line_start_x = cluster.x;

            if cluster.advance <= w {
                let mut moved = cluster;
                moved.shift_x(-moved.x);
                current_line.push(moved);
            } else {
                match fallback {
                    WrapClusterFallback::Keep => {
                        let advance = cluster.advance;
                        let mut moved = cluster;
                        moved.shift_x(-moved.x);
                        current_line.push(moved);
                        lines.push(current_line);
                        current_line = Vec::new();
                        current_line_start_x += advance;
                    }
                    WrapClusterFallback::Drop => break,
                }
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}

pub(super) fn wrap_clusters_at_words<G: Clone>(
    clusters: Vec<WorkingCluster<G>>,
    w: f32,
    fallback: WrapWordFallback,
) -> Vec<Vec<WorkingCluster<G>>> {
    if clusters.is_empty() {
        return vec![Vec::new()];
    }

    struct Seg<G> {
        is_space: bool,
        clusters: Vec<WorkingCluster<G>>,
        logical_w: f32,
    }

    let mut segments: Vec<Seg<G>> = Vec::new();
    for cluster in clusters {
        let is_space = cluster.is_whitespace || cluster.is_hard_break;
        if !is_space {
            if let Some(last) = segments.last_mut() {
                if !last.is_space {
                    last.clusters.push(cluster);
                    continue;
                }
            }
        }
        segments.push(Seg {
            is_space,
            clusters: vec![cluster],
            logical_w: 0.0,
        });
    }

    let seg_starts = segments
        .iter()
        .map(|seg| {
            seg.clusters
                .iter()
                .map(|cluster| cluster.x)
                .reduce(f32::min)
                .unwrap_or(0.0)
        })
        .collect::<Vec<_>>();

    let seg_len = segments.len();
    for i in 0..seg_len {
        if segments[i].clusters.is_empty() {
            continue;
        }
        let seg_l = seg_starts[i];
        for cluster in &mut segments[i].clusters {
            cluster.shift_x(-seg_l);
        }
        segments[i].logical_w = if i + 1 < seg_len {
            seg_starts[i + 1] - seg_l
        } else {
            logical_cluster_line_width(&segments[i].clusters)
        };
    }

    let mut lines = Vec::new();
    let mut current_line = Vec::new();
    let mut current_logical_w = 0.0;

    for seg in segments {
        let seg_logical_w = seg.logical_w;
        let is_hard_break = seg.clusters.iter().any(|cluster| cluster.is_hard_break);
        if is_hard_break || current_logical_w + seg_logical_w <= w {
            for mut cluster in seg.clusters {
                cluster.shift_x(current_logical_w);
                current_line.push(cluster);
            }
            current_logical_w += seg_logical_w;
        } else {
            if seg.is_space && !current_line.is_empty() {
                for mut cluster in seg.clusters {
                    cluster.shift_x(current_logical_w);
                    cluster.collapse_soft_wrap_boundary();
                    current_line.push(cluster);
                }
                lines.push(current_line);
                current_line = Vec::new();
                current_logical_w = 0.0;
                continue;
            }

            if !current_line.is_empty() {
                collapse_trailing_soft_wrap_space(&mut current_line);
                lines.push(current_line);
                current_line = Vec::new();
                current_logical_w = 0.0;
            }

            if seg_logical_w <= w {
                for mut cluster in seg.clusters {
                    cluster.shift_x(current_logical_w);
                    current_line.push(cluster);
                }
                current_logical_w += seg_logical_w;
            } else {
                match fallback {
                    WrapWordFallback::WrapCluster { fallback } => {
                        let seg_len = seg.clusters.len();
                        let wrapped = wrap_clusters(seg.clusters, w, fallback);
                        let mut wrapped_count = 0;
                        if !wrapped.is_empty() {
                            lines.extend(wrapped[..wrapped.len() - 1].to_vec());
                            current_line = wrapped.last().expect("wrapped is non-empty").clone();
                            current_logical_w = current_line
                                .iter()
                                .map(WorkingCluster::end_x)
                                .fold(0.0, f32::max);
                            wrapped_count = wrapped.iter().map(Vec::len).sum();
                        }
                        if fallback == WrapClusterFallback::Drop && wrapped_count < seg_len {
                            break;
                        }
                    }
                    WrapWordFallback::Drop => {
                        for cluster in seg.clusters {
                            if cluster.end_x() <= w {
                                current_line.push(cluster);
                            } else {
                                break;
                            }
                        }
                        lines.push(current_line);
                        current_line = Vec::new();
                        break;
                    }
                    WrapWordFallback::Keep => {
                        for cluster in seg.clusters {
                            let end_x = cluster.end_x();
                            current_line.push(cluster);
                            if end_x > w {
                                break;
                            }
                        }
                        lines.push(current_line);
                        current_line = Vec::new();
                        break;
                    }
                }
            }
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines
}
