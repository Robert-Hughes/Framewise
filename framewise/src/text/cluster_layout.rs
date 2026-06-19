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
    let mut logical_start = f32::INFINITY;
    let mut logical_end = 0.0_f32;

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
            let cluster = WorkingCluster {
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
            };
            logical_start = logical_start.min(cluster.x);
            logical_end = logical_end.max(cluster.end_x());
            clusters.push(cluster);
        }
    } else if has_newline {
        clusters.reserve(1);
    }

    if has_newline {
        let x = clusters.last().map(WorkingCluster::end_x).unwrap_or(0.0);
        let cluster = WorkingCluster {
            source: WorkingClusterSource::Empty,
            byte_start: segment_end,
            byte_end: segment_end + 1,
            x,
            advance: 0.0,
            is_hard_break: true,
            is_whitespace: true,
            is_soft_wrap_boundary: false,
            glyphs_visible: false,
        };
        logical_start = logical_start.min(cluster.x);
        logical_end = logical_end.max(cluster.end_x());
        clusters.push(cluster);
    }

    if clusters.is_empty() {
        logical_start = 0.0;
        logical_end = 0.0;
    }
    let logical_width = logical_end - logical_start;

    WorkingSourceLine {
        clusters,
        byte_start: segment_start,
        byte_end: if has_newline {
            segment_end + 1
        } else {
            segment_end
        },
        logical_start,
        logical_width,
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

struct WorkingLineBuffer {
    clusters: Vec<WorkingCluster>,
    logical_start: f32,
    logical_width: f32,
}

impl WorkingLineBuffer {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            clusters: Vec::with_capacity(capacity),
            logical_start: 0.0,
            logical_width: 0.0,
        }
    }

    fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }

    fn clear_with_capacity(&mut self, capacity: usize) {
        self.clusters = Vec::with_capacity(capacity);
        self.logical_start = 0.0;
        self.logical_width = 0.0;
    }

    fn push_at(&mut self, mut cluster: WorkingCluster, x: f32) {
        cluster.x = x;
        self.push_rebased(cluster);
    }

    fn push_rebased(&mut self, cluster: WorkingCluster) {
        if self.clusters.is_empty() {
            self.logical_start = cluster.x;
            self.logical_width = cluster.end_x() - cluster.x;
        } else {
            let end_x = cluster.end_x();
            if cluster.x < self.logical_start {
                self.logical_width = self.logical_width.max(end_x - cluster.x);
                self.logical_start = cluster.x;
            } else {
                self.logical_width = self.logical_width.max(end_x - self.logical_start);
            }
        }
        self.clusters.push(cluster);
    }

    fn extend_segment_rebased(
        &mut self,
        segment: &mut Vec<WorkingCluster>,
        segment_start_x: f32,
        line_x: f32,
        collapse_as_boundary: bool,
    ) {
        for mut cluster in segment.drain(..) {
            let relative_x = cluster.x - segment_start_x;
            cluster.x = line_x + relative_x;
            if collapse_as_boundary {
                cluster.collapse_soft_wrap_boundary();
            }
            self.push_rebased(cluster);
        }
    }

    fn collapse_trailing_soft_wrap_space(&mut self) {
        let previous_width = self.logical_width;
        collapse_trailing_soft_wrap_space(&mut self.clusters);
        if self
            .clusters
            .last()
            .is_some_and(|cluster| cluster.is_soft_wrap_boundary)
        {
            self.refresh_geometry();
        } else {
            self.logical_width = previous_width;
        }
    }

    fn take_clusters(&mut self) -> Vec<WorkingCluster> {
        self.logical_start = 0.0;
        self.logical_width = 0.0;
        std::mem::take(&mut self.clusters)
    }

    fn logical_start(&self) -> f32 {
        self.logical_start
    }

    fn logical_width(&self) -> f32 {
        self.logical_width
    }

    fn refresh_geometry(&mut self) {
        self.logical_start = logical_cluster_line_start(&self.clusters);
        self.logical_width = logical_cluster_line_width(&self.clusters);
    }
}

struct PendingWrappedLine {
    clusters: Vec<WorkingCluster>,
    byte_start: usize,
    end_kind: LineEndKind,
    logical_start: f32,
    logical_width: f32,
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

    fn emit_line_with_geometry(
        &mut self,
        clusters: Vec<WorkingCluster>,
        end_kind: LineEndKind,
        logical_start: f32,
        logical_width: f32,
    ) {
        let byte_start = if let Some(previous) = &self.pending {
            first_cluster_byte_start(&clusters, previous_pending_byte_end(previous))
        } else {
            self.source_byte_start
        };

        if let Some(previous) = self.pending.take() {
            self.out.push(WorkingProcessedLine::pending_with_geometry(
                previous.clusters,
                previous.byte_start,
                byte_start,
                previous.end_kind,
                previous.logical_start,
                previous.logical_width,
            ));
        }

        self.pending = Some(PendingWrappedLine {
            clusters,
            byte_start,
            end_kind,
            logical_start,
            logical_width,
        });
    }

    fn emit_line(&mut self, clusters: Vec<WorkingCluster>, end_kind: LineEndKind) {
        let logical_start = logical_cluster_line_start(&clusters);
        let logical_width = logical_cluster_line_width(&clusters);
        self.emit_line_with_geometry(clusters, end_kind, logical_start, logical_width);
    }

    fn emit_buffer(&mut self, line: &mut WorkingLineBuffer, end_kind: LineEndKind) {
        let logical_start = line.logical_start();
        let logical_width = line.logical_width();
        let clusters = line.take_clusters();
        self.emit_line_with_geometry(clusters, end_kind, logical_start, logical_width);
    }

    fn emit_default_buffer(
        &mut self,
        line: &mut WorkingLineBuffer,
        has_following_visual_line: bool,
    ) {
        let end_kind = default_wrapped_line_end_kind(&line.clusters, has_following_visual_line);
        self.emit_buffer(line, end_kind);
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
            self.out.push(WorkingProcessedLine::pending_with_geometry(
                last.clusters,
                last.byte_start,
                self.source_byte_end,
                last.end_kind,
                last.logical_start,
                last.logical_width,
            ));
        }
    }
}

pub(super) fn wrap_clusters_into_processed_lines(
    clusters: Vec<WorkingCluster>,
    source_byte_start: usize,
    source_byte_end: usize,
    source_logical_width: f32,
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
    let estimated_lines = estimated_wrapped_line_count(cluster_count, source_logical_width, w);
    emitter.out.reserve(estimated_lines);
    let estimated_line_cap = estimated_clusters_per_line(cluster_count, estimated_lines);
    let result = wrap_clusters_into_processed_lines_open_tail(
        clusters,
        w,
        fallback,
        estimated_line_cap,
        &mut emitter,
    );
    if let Some(mut open_line) = result.open_line {
        emitter.emit_default_buffer(&mut open_line, false);
    }
    emitter.append_terminal_empty_line_if_needed();
    emitter.finish();
}

struct ClusterWrapResult {
    open_line: Option<WorkingLineBuffer>,
    wrapped_count: usize,
}

fn append_hard_break_to_pending(
    emitter: &mut WrappedLineEmitter<'_>,
    mut cluster: WorkingCluster,
) -> bool {
    if let Some(last_line) = emitter.pending_line_mut() {
        if last_line
            .clusters
            .last()
            .map(|c: &WorkingCluster| c.is_hard_break)
            != Some(true)
        {
            cluster.x = 0.0;
            last_line.clusters.push(cluster);
            last_line.end_kind = LineEndKind::HardNewline;
            last_line.logical_start = logical_cluster_line_start(&last_line.clusters);
            last_line.logical_width = logical_cluster_line_width(&last_line.clusters);
            return true;
        }
    }
    false
}

fn wrap_clusters_into_processed_lines_open_tail(
    clusters: Vec<WorkingCluster>,
    w: f32,
    fallback: WrapClusterFallback,
    estimated_line_cap: usize,
    emitter: &mut WrappedLineEmitter<'_>,
) -> ClusterWrapResult {
    if clusters.is_empty() {
        return ClusterWrapResult {
            open_line: None,
            wrapped_count: 0,
        };
    }
    let mut current_line = WorkingLineBuffer::with_capacity(estimated_line_cap);
    let mut current_line_start_x = clusters[0].x;
    let mut wrapped_count = 0;

    for cluster in clusters {
        if cluster.is_hard_break {
            let mut appended = false;
            if current_line.is_empty() && append_hard_break_to_pending(emitter, cluster.clone()) {
                appended = true;
            }
            wrapped_count += 1;
            if !appended {
                let x = cluster.x - current_line_start_x;
                current_line.push_at(cluster, x);
                emitter.emit_buffer(&mut current_line, LineEndKind::HardNewline);
                current_line.clear_with_capacity(estimated_line_cap);
            }
            continue;
        }

        let rel_start_x = cluster.x - current_line_start_x;
        let rel_end_x = rel_start_x + cluster.advance;

        if rel_end_x <= w {
            current_line.push_at(cluster, rel_start_x);
            wrapped_count += 1;
        } else if cluster.is_whitespace && !current_line.is_empty() {
            let next_line_start_x = cluster.x + cluster.advance;
            let mut moved = cluster;
            moved.collapse_soft_wrap_boundary();
            current_line.push_at(moved, rel_start_x);
            wrapped_count += 1;
            emitter.emit_default_buffer(&mut current_line, true);
            current_line.clear_with_capacity(estimated_line_cap);
            current_line_start_x = next_line_start_x;
        } else if current_line.is_empty() {
            match fallback {
                WrapClusterFallback::Keep => {
                    current_line.push_at(cluster, rel_start_x);
                    wrapped_count += 1;
                }
                WrapClusterFallback::Drop => break,
            }
        } else {
            current_line.collapse_trailing_soft_wrap_space();
            emitter.emit_default_buffer(&mut current_line, true);
            current_line.clear_with_capacity(estimated_line_cap);
            current_line_start_x = cluster.x;

            if cluster.advance <= w {
                current_line.push_at(cluster, 0.0);
                wrapped_count += 1;
            } else {
                match fallback {
                    WrapClusterFallback::Keep => {
                        current_line.push_at(cluster, 0.0);
                        wrapped_count += 1;
                    }
                    WrapClusterFallback::Drop => break,
                }
            }
        }
    }
    let open_line = (!current_line.is_empty()).then_some(current_line);
    ClusterWrapResult {
        open_line,
        wrapped_count,
    }
}

pub(super) fn wrap_clusters_at_words_into_processed_lines(
    clusters: Vec<WorkingCluster>,
    source_byte_start: usize,
    source_byte_end: usize,
    source_logical_width: f32,
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
    let estimated_lines =
        estimated_wrapped_line_count(input_cluster_count, source_logical_width, w);
    let estimated_line_cap = estimated_clusters_per_line(input_cluster_count, estimated_lines);
    emitter.out.reserve(estimated_lines);

    let mut state = WordWrapState {
        w,
        fallback,
        estimated_line_cap,
        current_line: WorkingLineBuffer::with_capacity(estimated_line_cap),
        emitter,
    };
    let mut segment = Vec::new();
    let mut segment_is_space = false;
    let mut segment_geometry = SegmentGeometry::default();

    for cluster in clusters {
        let is_space = cluster.is_whitespace || cluster.is_hard_break;
        if segment.is_empty() {
            segment_is_space = is_space;
            segment_geometry = SegmentGeometry::from_cluster(&cluster);
            segment.push(cluster);
            continue;
        }

        let continues_segment = !segment_is_space && !is_space;
        if continues_segment {
            segment_geometry.push(&cluster);
            segment.push(cluster);
        } else {
            if !state.flush_segment(&mut segment, segment_is_space, segment_geometry) {
                state.finish_emitter();
                return;
            }
            segment_is_space = is_space;
            segment_geometry = SegmentGeometry::from_cluster(&cluster);
            segment.push(cluster);
        }
    }

    if !segment.is_empty() && !state.flush_segment(&mut segment, segment_is_space, segment_geometry)
    {
        state.finish_emitter();
        return;
    }

    state.finish();
}

#[derive(Clone, Copy, Default)]
struct SegmentGeometry {
    logical_start: f32,
    logical_width: f32,
    has_hard_break: bool,
}

impl SegmentGeometry {
    fn from_cluster(cluster: &WorkingCluster) -> Self {
        Self {
            logical_start: cluster.x,
            logical_width: cluster.advance,
            has_hard_break: cluster.is_hard_break,
        }
    }

    fn push(&mut self, cluster: &WorkingCluster) {
        let end_x = cluster.end_x();
        if cluster.x < self.logical_start {
            self.logical_width = self.logical_width.max(end_x - cluster.x);
            self.logical_start = cluster.x;
        } else {
            self.logical_width = self.logical_width.max(end_x - self.logical_start);
        }
        self.has_hard_break |= cluster.is_hard_break;
    }
}

struct WordWrapState<'a> {
    w: f32,
    fallback: WrapWordFallback,
    estimated_line_cap: usize,
    current_line: WorkingLineBuffer,
    emitter: WrappedLineEmitter<'a>,
}

impl WordWrapState<'_> {
    fn flush_segment(
        &mut self,
        segment: &mut Vec<WorkingCluster>,
        segment_is_space: bool,
        segment_geometry: SegmentGeometry,
    ) -> bool {
        let segment_start_x = segment_geometry.logical_start;
        let segment_logical_w = segment_geometry.logical_width;
        if segment_geometry.has_hard_break
            || self.current_line.logical_width() + segment_logical_w <= self.w
        {
            self.current_line.extend_segment_rebased(
                segment,
                segment_start_x,
                self.current_line.logical_width(),
                false,
            );
            return true;
        }

        if segment_is_space && !self.current_line.is_empty() {
            self.current_line.extend_segment_rebased(
                segment,
                segment_start_x,
                self.current_line.logical_width(),
                true,
            );
            self.emit_current_line(true);
            return true;
        }

        if !self.current_line.is_empty() {
            self.current_line.collapse_trailing_soft_wrap_space();
            self.emit_current_line(true);
        }

        if segment_logical_w <= self.w {
            self.current_line.extend_segment_rebased(
                segment,
                segment_start_x,
                self.current_line.logical_width(),
                false,
            );
            return true;
        }

        let seg_len = segment.len();
        let seg_clusters = std::mem::take(segment);
        match self.fallback {
            WrapWordFallback::WrapCluster { fallback } => {
                let result = wrap_clusters_into_processed_lines_open_tail(
                    seg_clusters,
                    self.w,
                    fallback,
                    self.estimated_line_cap,
                    &mut self.emitter,
                );
                self.current_line = result
                    .open_line
                    .unwrap_or_else(|| WorkingLineBuffer::with_capacity(self.estimated_line_cap));
                fallback != WrapClusterFallback::Drop || result.wrapped_count >= seg_len
            }
            WrapWordFallback::Drop => {
                for cluster in seg_clusters {
                    let x = cluster.x - segment_start_x;
                    if x + cluster.advance <= self.w {
                        self.current_line.push_at(cluster, x);
                    } else {
                        break;
                    }
                }
                self.emit_current_line(true);
                false
            }
            WrapWordFallback::Keep => {
                for cluster in seg_clusters {
                    let x = cluster.x - segment_start_x;
                    let end_x = x + cluster.advance;
                    self.current_line.push_at(cluster, x);
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
        self.emitter
            .emit_default_buffer(&mut self.current_line, has_following_visual_line);
        self.current_line
            .clear_with_capacity(self.estimated_line_cap);
    }

    fn finish(mut self) {
        if !self.current_line.is_empty() {
            self.emitter
                .emit_default_buffer(&mut self.current_line, false);
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
