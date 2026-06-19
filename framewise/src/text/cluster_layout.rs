use super::{
    LineEndKind, TextBackend, TextStyle, WorkingCluster, WorkingClusterSource,
    WorkingProcessedLine, WorkingRun, WorkingSourceLine, WrapClusterFallback, WrapWordFallback,
};

pub(super) fn make_source_line<B: TextBackend>(
    backend: &mut B,
    runs: &mut Vec<WorkingRun<B::ShapedGlyphToken>>,
    text: &str,
    style: TextStyle,
    segment_start: usize,
    segment_end: usize,
    has_newline: bool,
) -> WorkingSourceLine {
    let segment = &text[segment_start..segment_end];
    let mut clusters = Vec::new();

    if !segment.is_empty() {
        let shaped = backend.shape_text(segment, style);
        clusters.reserve(shaped.clusters.len() + usize::from(has_newline));
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
    } else if has_newline {
        clusters.reserve(1);
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
    }
}

pub(super) fn logical_cluster_line_width(clusters: &[WorkingCluster]) -> f32 {
    let start = logical_cluster_line_start(clusters);
    clusters
        .iter()
        .map(WorkingCluster::end_x)
        .fold(start, f32::max)
        - start
}

pub(super) fn logical_cluster_line_start(clusters: &[WorkingCluster]) -> f32 {
    clusters
        .iter()
        .map(|cluster| cluster.x)
        .reduce(f32::min)
        .unwrap_or(0.0)
}

fn collapse_trailing_soft_wrap_space(clusters: &mut [WorkingCluster]) {
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

struct PendingWrappedLine {
    clusters: Vec<WorkingCluster>,
    byte_start: usize,
    end_kind: LineEndKind,
}

fn first_cluster_byte_start(clusters: &[WorkingCluster], fallback: usize) -> usize {
    clusters
        .first()
        .map(|cluster| cluster.byte_start)
        .unwrap_or(fallback)
}

fn previous_pending_byte_end(line: &PendingWrappedLine) -> usize {
    line.clusters
        .last()
        .map(|cluster| cluster.byte_end)
        .unwrap_or(line.byte_start)
}

fn default_wrapped_line_end_kind(
    clusters: &[WorkingCluster],
    has_following_visual_line: bool,
) -> LineEndKind {
    if clusters.last().is_some_and(|cluster| cluster.is_hard_break) {
        LineEndKind::HardNewline
    } else if clusters
        .last()
        .is_some_and(|cluster| cluster.is_soft_wrap_boundary)
    {
        LineEndKind::SoftWrapWhitespace
    } else if has_following_visual_line {
        LineEndKind::SoftWrapNonWhitespace
    } else {
        LineEndKind::EndOfText
    }
}

struct WrappedLineEmitter<'a> {
    pending: Option<PendingWrappedLine>,
    out: &'a mut Vec<WorkingProcessedLine>,
    source_byte_start: usize,
    source_byte_end: usize,
}

impl<'a> WrappedLineEmitter<'a> {
    fn new(
        out: &'a mut Vec<WorkingProcessedLine>,
        source_byte_start: usize,
        source_byte_end: usize,
    ) -> Self {
        Self {
            pending: None,
            out,
            source_byte_start,
            source_byte_end,
        }
    }

    fn emit_line(&mut self, clusters: Vec<WorkingCluster>, end_kind: LineEndKind) {
        let byte_start = if let Some(previous) = &self.pending {
            first_cluster_byte_start(&clusters, previous_pending_byte_end(previous))
        } else {
            self.source_byte_start
        };

        if let Some(previous) = self.pending.take() {
            self.out.push(WorkingProcessedLine::pending(
                previous.clusters,
                previous.byte_start,
                byte_start,
                previous.end_kind,
            ));
        }

        self.pending = Some(PendingWrappedLine {
            clusters,
            byte_start,
            end_kind,
        });
    }

    fn emit_default_line(
        &mut self,
        clusters: Vec<WorkingCluster>,
        has_following_visual_line: bool,
    ) {
        let end_kind = default_wrapped_line_end_kind(&clusters, has_following_visual_line);
        self.emit_line(clusters, end_kind);
    }

    fn append_terminal_empty_line_if_needed(&mut self) {
        let has_terminal_boundary = self
            .pending
            .as_ref()
            .and_then(|line| line.clusters.last())
            .is_some_and(|cluster| {
                cluster.is_soft_wrap_boundary && cluster.byte_end == self.source_byte_end
            });
        if has_terminal_boundary {
            self.emit_line(Vec::new(), LineEndKind::EndOfText);
        }
    }

    fn pending_line_mut(&mut self) -> Option<&mut PendingWrappedLine> {
        self.pending.as_mut()
    }

    fn finish(mut self) {
        if let Some(mut last) = self.pending.take() {
            if last.end_kind == LineEndKind::SoftWrapNonWhitespace {
                last.end_kind = default_wrapped_line_end_kind(&last.clusters, false);
            }
            self.out.push(WorkingProcessedLine::pending(
                last.clusters,
                last.byte_start,
                self.source_byte_end,
                last.end_kind,
            ));
        }
    }
}

pub(super) fn wrap_clusters_into_processed_lines(
    clusters: Vec<WorkingCluster>,
    source_byte_start: usize,
    source_byte_end: usize,
    w: f32,
    fallback: WrapClusterFallback,
    out: &mut Vec<WorkingProcessedLine>,
) {
    let mut emitter = WrappedLineEmitter::new(out, source_byte_start, source_byte_end);
    if clusters.is_empty() {
        emitter.emit_line(Vec::new(), LineEndKind::EndOfText);
        emitter.finish();
        return;
    }

    let cluster_count = clusters.len();
    let estimated_lines =
        estimated_wrapped_line_count(cluster_count, logical_cluster_line_width(&clusters), w);
    emitter.out.reserve(estimated_lines);
    let estimated_line_cap = estimated_clusters_per_line(cluster_count, estimated_lines);
    let mut current_line = Vec::with_capacity(estimated_line_cap);
    let mut current_line_start_x = clusters[0].x;

    for cluster in clusters {
        if cluster.is_hard_break {
            let mut moved = cluster;
            let mut appended = false;
            if current_line.is_empty() {
                if let Some(last_line) = emitter.pending_line_mut() {
                    if last_line
                        .clusters
                        .last()
                        .map(|c: &WorkingCluster| c.is_hard_break)
                        != Some(true)
                    {
                        moved.shift_x(-moved.x);
                        last_line.clusters.push(moved.clone());
                        last_line.end_kind = LineEndKind::HardNewline;
                        appended = true;
                    }
                }
            }
            if !appended {
                moved.shift_x(-current_line_start_x);
                current_line.push(moved);
                emitter.emit_line(current_line, LineEndKind::HardNewline);
                current_line = Vec::with_capacity(estimated_line_cap);
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
            emitter.emit_default_line(current_line, true);
            current_line = Vec::with_capacity(estimated_line_cap);
            current_line_start_x = next_line_start_x;
        } else if current_line.is_empty() {
            match fallback {
                WrapClusterFallback::Keep => {
                    let mut moved = cluster;
                    moved.shift_x(rel_start_x - moved.x);
                    current_line.push(moved);
                    emitter.emit_default_line(current_line, true);
                    current_line = Vec::with_capacity(estimated_line_cap);
                    current_line_start_x += rel_end_x;
                }
                WrapClusterFallback::Drop => break,
            }
        } else {
            collapse_trailing_soft_wrap_space(&mut current_line);
            emitter.emit_default_line(current_line, true);
            current_line = Vec::with_capacity(estimated_line_cap);
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
                        emitter.emit_default_line(current_line, true);
                        current_line = Vec::with_capacity(estimated_line_cap);
                        current_line_start_x += advance;
                    }
                    WrapClusterFallback::Drop => break,
                }
            }
        }
    }
    if !current_line.is_empty() {
        emitter.emit_default_line(current_line, false);
    }
    emitter.append_terminal_empty_line_if_needed();
    emitter.finish();
}

fn wrap_clusters_into(
    clusters: Vec<WorkingCluster>,
    w: f32,
    fallback: WrapClusterFallback,
    lines: &mut Vec<Vec<WorkingCluster>>,
) -> usize {
    if clusters.is_empty() {
        return 0;
    }
    let cluster_count = clusters.len();
    let estimated_lines =
        estimated_wrapped_line_count(cluster_count, logical_cluster_line_width(&clusters), w);
    let estimated_line_cap = estimated_clusters_per_line(cluster_count, estimated_lines);
    let mut current_line = Vec::with_capacity(estimated_line_cap);
    let mut current_line_start_x = clusters[0].x;
    let mut wrapped_count = 0;

    for cluster in clusters {
        if cluster.is_hard_break {
            let mut moved = cluster;
            let mut appended = false;
            if current_line.is_empty() {
                if let Some(last_line) = lines.last_mut() {
                    if last_line.last().map(|c: &WorkingCluster| c.is_hard_break) != Some(true) {
                        moved.shift_x(-moved.x);
                        last_line.push(moved.clone());
                        appended = true;
                    }
                }
            }
            wrapped_count += 1;
            if !appended {
                moved.shift_x(-current_line_start_x);
                current_line.push(moved);
                lines.push(current_line);
                current_line = Vec::with_capacity(estimated_line_cap);
            }
            continue;
        }

        let rel_start_x = cluster.x - current_line_start_x;
        let rel_end_x = rel_start_x + cluster.advance;

        if rel_end_x <= w {
            let mut moved = cluster;
            moved.shift_x(rel_start_x - moved.x);
            current_line.push(moved);
            wrapped_count += 1;
        } else if cluster.is_whitespace && !current_line.is_empty() {
            let next_line_start_x = cluster.x + cluster.advance;
            let mut moved = cluster;
            moved.shift_x(rel_start_x - moved.x);
            moved.collapse_soft_wrap_boundary();
            current_line.push(moved);
            wrapped_count += 1;
            lines.push(current_line);
            current_line = Vec::with_capacity(estimated_line_cap);
            current_line_start_x = next_line_start_x;
        } else if current_line.is_empty() {
            match fallback {
                WrapClusterFallback::Keep => {
                    let mut moved = cluster;
                    moved.shift_x(rel_start_x - moved.x);
                    current_line.push(moved);
                    wrapped_count += 1;
                    lines.push(current_line);
                    current_line = Vec::with_capacity(estimated_line_cap);
                    current_line_start_x += rel_end_x;
                }
                WrapClusterFallback::Drop => break,
            }
        } else {
            collapse_trailing_soft_wrap_space(&mut current_line);
            lines.push(current_line);
            current_line = Vec::with_capacity(estimated_line_cap);
            current_line_start_x = cluster.x;

            if cluster.advance <= w {
                let mut moved = cluster;
                moved.shift_x(-moved.x);
                current_line.push(moved);
                wrapped_count += 1;
            } else {
                match fallback {
                    WrapClusterFallback::Keep => {
                        let advance = cluster.advance;
                        let mut moved = cluster;
                        moved.shift_x(-moved.x);
                        current_line.push(moved);
                        wrapped_count += 1;
                        lines.push(current_line);
                        current_line = Vec::with_capacity(estimated_line_cap);
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
    wrapped_count
}

pub(super) fn wrap_clusters_at_words_into_processed_lines(
    clusters: Vec<WorkingCluster>,
    source_byte_start: usize,
    source_byte_end: usize,
    w: f32,
    fallback: WrapWordFallback,
    out: &mut Vec<WorkingProcessedLine>,
) {
    let mut emitter = WrappedLineEmitter::new(out, source_byte_start, source_byte_end);
    if clusters.is_empty() {
        emitter.emit_line(Vec::new(), LineEndKind::EndOfText);
        emitter.finish();
        return;
    }

    let input_cluster_count = clusters.len();
    let logical_w = logical_cluster_line_width(&clusters);
    let estimated_lines = estimated_wrapped_line_count(input_cluster_count, logical_w, w);
    let estimated_line_cap = estimated_clusters_per_line(input_cluster_count, estimated_lines);
    emitter.out.reserve(estimated_lines);

    let mut state = WordWrapState {
        w,
        fallback,
        estimated_line_cap,
        current_line: Vec::with_capacity(estimated_line_cap),
        current_logical_w: 0.0,
        emitter,
    };
    let mut segment = Vec::new();
    let mut segment_is_space = false;

    for cluster in clusters {
        let is_space = cluster.is_whitespace || cluster.is_hard_break;
        if segment.is_empty() {
            segment_is_space = is_space;
            segment.push(cluster);
            continue;
        }

        let continues_segment = !segment_is_space && !is_space;
        if continues_segment {
            segment.push(cluster);
        } else {
            if !state.flush_segment(&mut segment, segment_is_space) {
                state.finish_emitter();
                return;
            }
            segment_is_space = is_space;
            segment.push(cluster);
        }
    }

    if !segment.is_empty() && !state.flush_segment(&mut segment, segment_is_space) {
        state.finish_emitter();
        return;
    }

    state.finish();
}

fn append_segment_to_line(
    line: &mut Vec<WorkingCluster>,
    segment: &mut Vec<WorkingCluster>,
    line_x: f32,
    collapse_as_boundary: bool,
) {
    let segment_start_x = logical_cluster_line_start(segment);
    for mut cluster in segment.drain(..) {
        let relative_x = cluster.x - segment_start_x;
        cluster.shift_x(line_x + relative_x - cluster.x);
        if collapse_as_boundary {
            cluster.collapse_soft_wrap_boundary();
        }
        line.push(cluster);
    }
}

struct WordWrapState<'a> {
    w: f32,
    fallback: WrapWordFallback,
    estimated_line_cap: usize,
    current_line: Vec<WorkingCluster>,
    current_logical_w: f32,
    emitter: WrappedLineEmitter<'a>,
}

impl WordWrapState<'_> {
    fn flush_segment(&mut self, segment: &mut Vec<WorkingCluster>, segment_is_space: bool) -> bool {
        let segment_start_x = logical_cluster_line_start(segment);
        let segment_logical_w = logical_cluster_line_width(segment);
        let is_hard_break = segment.iter().any(|cluster| cluster.is_hard_break);
        if is_hard_break || self.current_logical_w + segment_logical_w <= self.w {
            append_segment_to_line(
                &mut self.current_line,
                segment,
                self.current_logical_w,
                false,
            );
            self.current_logical_w += segment_logical_w;
            return true;
        }

        if segment_is_space && !self.current_line.is_empty() {
            append_segment_to_line(
                &mut self.current_line,
                segment,
                self.current_logical_w,
                true,
            );
            self.emit_current_line(true);
            return true;
        }

        if !self.current_line.is_empty() {
            collapse_trailing_soft_wrap_space(&mut self.current_line);
            self.emit_current_line(true);
        }

        if segment_logical_w <= self.w {
            append_segment_to_line(
                &mut self.current_line,
                segment,
                self.current_logical_w,
                false,
            );
            self.current_logical_w += segment_logical_w;
            return true;
        }

        for cluster in segment.iter_mut() {
            cluster.shift_x(-segment_start_x);
        }
        let seg_len = segment.len();
        let seg_clusters = std::mem::take(segment);
        match self.fallback {
            WrapWordFallback::WrapCluster { fallback } => {
                let mut wrapped_lines = Vec::new();
                let wrapped_count =
                    wrap_clusters_into(seg_clusters, self.w, fallback, &mut wrapped_lines);
                if let Some(last) = wrapped_lines.pop() {
                    for line in wrapped_lines {
                        self.emitter.emit_default_line(line, true);
                    }
                    self.current_line = last;
                    self.current_logical_w = self
                        .current_line
                        .iter()
                        .map(WorkingCluster::end_x)
                        .fold(0.0, f32::max);
                }
                fallback != WrapClusterFallback::Drop || wrapped_count >= seg_len
            }
            WrapWordFallback::Drop => {
                for cluster in seg_clusters {
                    if cluster.end_x() <= self.w {
                        self.current_line.push(cluster);
                    } else {
                        break;
                    }
                }
                self.emit_current_line(true);
                false
            }
            WrapWordFallback::Keep => {
                for cluster in seg_clusters {
                    let end_x = cluster.end_x();
                    self.current_line.push(cluster);
                    if end_x > self.w {
                        break;
                    }
                }
                self.emit_current_line(true);
                false
            }
        }
    }

    fn emit_current_line(&mut self, has_following_visual_line: bool) {
        let line = std::mem::replace(
            &mut self.current_line,
            Vec::with_capacity(self.estimated_line_cap),
        );
        self.emitter
            .emit_default_line(line, has_following_visual_line);
        self.current_logical_w = 0.0;
    }

    fn finish(mut self) {
        if !self.current_line.is_empty() {
            let line = std::mem::take(&mut self.current_line);
            self.emitter.emit_default_line(line, false);
        }
        self.finish_emitter();
    }

    fn finish_emitter(mut self) {
        self.emitter.append_terminal_empty_line_if_needed();
        self.emitter.finish();
    }
}

fn estimated_wrapped_line_count(cluster_count: usize, logical_w: f32, max_w: f32) -> usize {
    if cluster_count == 0 || !max_w.is_finite() || max_w <= 0.0 {
        return 1;
    }

    ((logical_w / max_w).ceil() as usize).clamp(1, cluster_count)
}

fn estimated_clusters_per_line(cluster_count: usize, line_count: usize) -> usize {
    cluster_count.div_ceil(line_count.max(1)).max(1)
}
