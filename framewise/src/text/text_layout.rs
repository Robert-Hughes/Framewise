use super::cluster_layout::{
    logical_cluster_line_start, logical_cluster_line_width, make_source_line,
    wrap_clusters_at_words_into_processed_lines, wrap_clusters_into_processed_lines,
};
use super::text_overflow::apply_ellipsis_x;
use super::{
    union_approx_ink_bounds, EllipsisFallback, LayoutGlyph, LineEndKind, LineMetrics, OverflowX,
    OverflowY, TextBackend, TextBounds, TextLayout, TextLineAlign, TextMetrics, TextStyle,
    WorkingCluster, WorkingClusterSource, WorkingProcessedLine, WorkingRun,
};
use crate::{
    draw::DrawCommands,
    types::{Color, Rect, Vec2},
};
use std::hash::Hash;

/// Lay out `text` with Framewise-owned text layout policy.
///
/// Framewise owns hard newline handling, wrapping, overflow, line records,
/// logical metrics, caret geometry, hit-testing, and glyph-run emission. The
/// backend owns font selection, shaping, glyph rasterisation, glyph caching, and
/// renderer resource handles.
///
/// All positions in the returned layout are in block-local coordinates: the
/// origin is the block's top-left corner, with y increasing downward. The caller
/// translates the block to its final screen position when emitting glyphs.
pub fn layout_text<B: TextBackend>(
    backend: &mut B,
    text: &str,
    style: TextStyle,
    bounds: TextBounds,
) -> TextLayout<B::ShapedGlyphToken> {
    TextLayout::from_backend(backend, text, style, bounds)
}

/// Measure `text` without preparing backend glyph resources for drawing.
///
/// Used by widgets' intrinsic-sizing companions to learn how large a piece of
/// text wants to be inside a given space, before the final rect is resolved.
/// The returned [`TextMetrics`] reflect the style's flow policy applied against
/// `bounds`; see [`TextBounds`] for how bounded and unbounded axes drive reflow.
///
/// The returned `logical_size` represents logical layout geometry:
/// advance-based line width and line-height-based block height after the
/// selected overflow policy has been applied. It is not a tight ink box.
///
/// With strict overflow policies the logical size should fit within bounded
/// input axes. Policies that explicitly keep overflowing content may return a
/// logical size larger than the supplied bounds. `approx_ink_bounds` reports
/// approximate visible bounds, which may protrude outside the logical size due
/// to font metrics and glyph placement.
///
/// `style.flow.line_align` has no effect on logical sizing, wrapping, or
/// truncation: those decisions are made in logical line space. It may affect
/// `approx_ink_bounds`, because alignment shifts the admitted glyphs within the
/// available line width.
///
/// For empty `text`, this returns the empty-text metrics described on
/// [`TextLayout`]: one normal-height line, zero width, and empty ink bounds.
pub fn measure_text<B: TextBackend>(
    backend: &mut B,
    text: &str,
    style: TextStyle,
    bounds: TextBounds,
) -> TextMetrics {
    layout_text(backend, text, style, bounds).metrics().clone()
}

pub fn layout_text_in_rect<B: TextBackend>(
    backend: &mut B,
    text: &str,
    style: TextStyle,
    rect: Rect,
) -> TextLayout<B::ShapedGlyphToken> {
    layout_text(
        backend,
        text,
        style,
        TextBounds {
            max_width: Some(rect.w),
            max_height: Some(rect.h),
        },
    )
}

/// Lay out `text` in `rect`, emit its drawable glyphs, and return the owned
/// layout for further metrics, caret, or hit-testing queries.
///
/// `rect` is the fully concrete logical layout rect: its width is the
/// wrap/alignment width, its height is the vertical layout or clip extent, and
/// its origin is the block origin used for rendering.
///
/// The final screen position (`rect.x`, `rect.y`) is passed to the backend when
/// glyphs are emitted so it can apply subpixel offsets/positioning at the
/// absolute draw location.
///
/// The text backend may produce ink that extends outside this rect. A caller
/// that needs hard containment must apply clipping or provide padding.
///
/// The returned [`TextLayout::metrics`] equal what [`measure_text`] would report
/// for the same `text` and `style`, with
/// `TextBounds { max_width: Some(rect.w), max_height: Some(rect.h) }`.
pub fn emit_text_in_rect<B: TextBackend>(
    commands: &mut DrawCommands,
    backend: &mut B,
    text: &str,
    style: TextStyle,
    rect: Rect,
    color: Color,
    z: u32,
) -> TextLayout<B::ShapedGlyphToken> {
    let layout = layout_text_in_rect(backend, text, style, rect);
    layout.emit_glyphs(commands, backend, Vec2::new(rect.x, rect.y), color, z);
    layout
}

fn working_cluster_ink<G: Copy>(
    cluster: &WorkingCluster,
    runs: &[WorkingRun<G>],
    baseline_y: f32,
) -> Option<Rect> {
    if !cluster.glyphs_visible {
        return None;
    }

    match &cluster.source {
        WorkingClusterSource::Shaped {
            run_index,
            cluster_index,
        } => {
            let run = &runs[*run_index];
            let shaped_cluster = &run.shaped.clusters[*cluster_index];
            debug_assert_eq!(
                cluster.byte_start,
                run.segment_start + shaped_cluster.byte_start
            );
            let rect = shaped_cluster.approx_ink_bounds;
            (rect.w > 0.0 && rect.h > 0.0)
                .then(|| Rect::new(cluster.x + rect.x, baseline_y + rect.y, rect.w, rect.h))
        }
        WorkingClusterSource::Empty => None,
    }
}

impl<G: Copy + Eq + Hash> TextLayout<G> {
    fn from_backend<B: TextBackend<ShapedGlyphToken = G>>(
        backend: &mut B,
        text: &str,
        style: TextStyle,
        bounds: TextBounds,
    ) -> Self {
        let flow = style.flow;
        let line_metrics = backend.line_metrics(style);
        let line_height = line_metrics.line_height.round().max(1.0);
        let baseline_offset = line_metrics.baseline_offset.round();
        let source_line_count = text.as_bytes().iter().filter(|&&b| b == b'\n').count() + 1;
        let mut working_runs = Vec::with_capacity(source_line_count);
        let mut source_lines = Vec::with_capacity(source_line_count);
        let mut start_byte = 0;

        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                source_lines.push(make_source_line(
                    backend,
                    &mut working_runs,
                    text,
                    style,
                    start_byte,
                    idx,
                    true,
                ));
                start_byte = idx + ch.len_utf8();
            }
        }
        if start_byte <= text.len() {
            source_lines.push(make_source_line(
                backend,
                &mut working_runs,
                text,
                style,
                start_byte,
                text.len(),
                false,
            ));
        }

        let global_line_start = if source_lines.iter().all(|line| line.clusters.is_empty()) {
            0.0
        } else {
            source_lines
                .iter()
                .flat_map(|line| line.clusters.iter().map(|cluster| cluster.x))
                .fold(f32::INFINITY, f32::min)
        };

        let mut truncated_horizontal = false;
        let mut processed_lines = Vec::with_capacity(source_line_count);

        for line in source_lines {
            let mut seg = line.clusters;
            let line_start = logical_cluster_line_start(&seg);
            let logical_line_w = logical_cluster_line_width(&seg);
            let base_shift = match flow.line_align {
                TextLineAlign::Start => global_line_start,
                TextLineAlign::Center | TextLineAlign::End => line_start,
            };
            if base_shift != 0.0 {
                for cluster in &mut seg {
                    cluster.shift_x(-base_shift);
                }
            }

            if let Some(w) = bounds.max_width {
                match flow.overflow_x {
                    OverflowX::WrapWord { fallback } => {
                        wrap_clusters_at_words_into_processed_lines(
                            seg,
                            line.byte_start,
                            line.byte_end,
                            w,
                            fallback,
                            &mut processed_lines,
                        );
                    }
                    OverflowX::WrapCluster { fallback } => {
                        wrap_clusters_into_processed_lines(
                            seg,
                            line.byte_start,
                            line.byte_end,
                            w,
                            fallback,
                            &mut processed_lines,
                        );
                    }
                    _ => {
                        if logical_line_w > w + 0.5 {
                            truncated_horizontal = true;
                            match flow.overflow_x {
                                OverflowX::Ellipsis { fallback } => {
                                    let clusters = apply_ellipsis_x(
                                        backend,
                                        &mut working_runs,
                                        seg,
                                        w,
                                        style,
                                        fallback,
                                    );
                                    processed_lines.push(WorkingProcessedLine::pending(
                                        clusters,
                                        line.byte_start,
                                        line.byte_end,
                                        LineEndKind::EllipsisX,
                                    ));
                                }
                                OverflowX::Keep => {
                                    let mut out = Vec::with_capacity(seg.len());
                                    for cluster in seg {
                                        let end_x = cluster.end_x();
                                        out.push(cluster);
                                        if end_x > w {
                                            break;
                                        }
                                    }
                                    processed_lines.push(WorkingProcessedLine::pending(
                                        out,
                                        line.byte_start,
                                        line.byte_end,
                                        LineEndKind::OverflowKeep,
                                    ));
                                }
                                OverflowX::Drop => {
                                    let mut out = Vec::with_capacity(seg.len());
                                    for cluster in seg {
                                        if cluster.end_x() <= w {
                                            out.push(cluster);
                                        } else {
                                            break;
                                        }
                                    }
                                    processed_lines.push(WorkingProcessedLine::pending(
                                        out,
                                        line.byte_start,
                                        line.byte_end,
                                        LineEndKind::OverflowDrop,
                                    ));
                                }
                                _ => unreachable!(),
                            }
                        } else {
                            let end_kind =
                                if seg.last().is_some_and(|cluster| cluster.is_hard_break) {
                                    LineEndKind::HardNewline
                                } else {
                                    LineEndKind::EndOfText
                                };
                            processed_lines.push(WorkingProcessedLine::pending(
                                seg,
                                line.byte_start,
                                line.byte_end,
                                end_kind,
                            ));
                        }
                    }
                }
            } else {
                let end_kind = if seg.last().is_some_and(|cluster| cluster.is_hard_break) {
                    LineEndKind::HardNewline
                } else {
                    LineEndKind::EndOfText
                };
                processed_lines.push(WorkingProcessedLine::pending(
                    seg,
                    line.byte_start,
                    line.byte_end,
                    end_kind,
                ));
            }
        }

        let max_lines = bounds
            .max_height
            .map(|h| (h / line_height).floor() as usize)
            .unwrap_or(processed_lines.len());
        let mut truncated_vertical = false;
        if processed_lines.len() > max_lines {
            truncated_vertical = true;
            match flow.overflow_y {
                OverflowY::Drop => processed_lines.truncate(max_lines),
                OverflowY::Keep => processed_lines.truncate(max_lines + 1),
                OverflowY::Ellipsis { fallback } => {
                    if max_lines > 0 {
                        let last_idx = max_lines - 1;
                        let last_line_clusters =
                            std::mem::take(&mut processed_lines[last_idx].clusters);
                        let w = bounds.max_width.unwrap_or(f32::INFINITY);
                        processed_lines[last_idx].clusters = apply_ellipsis_x(
                            backend,
                            &mut working_runs,
                            last_line_clusters,
                            w,
                            style,
                            fallback,
                        );
                        processed_lines[last_idx].end_kind = LineEndKind::EllipsisY;
                        processed_lines.truncate(max_lines);
                    } else {
                        match fallback {
                            EllipsisFallback::Keep => {
                                processed_lines.truncate(1);
                                if let Some(line) = processed_lines.first_mut() {
                                    line.end_kind = LineEndKind::EllipsisY;
                                }
                            }
                            EllipsisFallback::Drop => processed_lines.clear(),
                        }
                    }
                }
            }
        }

        if processed_lines.is_empty() {
            processed_lines.push(WorkingProcessedLine::pending(
                Vec::new(),
                0,
                text.len(),
                LineEndKind::EndOfText,
            ));
        }

        let mut lines = Vec::with_capacity(processed_lines.len());
        let mut block_width = 0.0_f32;
        let mut block_ink: Option<Rect> = None;
        let mut visible_glyph_count = 0;

        for (idx, mut line) in processed_lines.into_iter().enumerate() {
            line.y_top = idx as f32 * line_height;
            line.baseline_y = line.y_top + baseline_offset;
            line.height = line_height;

            let align_off = match bounds.max_width {
                Some(w) => {
                    let logical_line_w = logical_cluster_line_width(&line.clusters);
                    match flow.line_align {
                        TextLineAlign::Start => 0.0,
                        TextLineAlign::Center => ((w - logical_line_w) * 0.5).max(0.0),
                        TextLineAlign::End => (w - logical_line_w).max(0.0),
                    }
                }
                None => 0.0,
            };
            if align_off != 0.0 {
                for cluster in &mut line.clusters {
                    cluster.shift_x(align_off);
                }
            }

            let logical_line_w = logical_cluster_line_width(&line.clusters);
            block_width = block_width.max(logical_line_w);

            let mut line_ink = None;
            for cluster in &line.clusters {
                if cluster.glyphs_visible {
                    if let WorkingClusterSource::Shaped {
                        run_index,
                        cluster_index,
                    } = cluster.source
                    {
                        visible_glyph_count += working_runs[run_index].shaped.clusters
                            [cluster_index]
                            .glyphs
                            .len();
                    }
                }

                let line_ink_from_cluster =
                    working_cluster_ink(cluster, &working_runs, line.baseline_y);
                line_ink = line_ink_from_cluster
                    .into_iter()
                    .fold(line_ink, union_approx_ink_bounds);
            }

            block_ink = line_ink
                .into_iter()
                .fold(block_ink, union_approx_ink_bounds);
            let (approx_ink_x, approx_ink_width) =
                line_ink.map_or((align_off, 0.0), |rect| (rect.x, rect.w));

            line.logical_width = logical_line_w;
            line.approx_ink_width = approx_ink_width;
            line.logical_x = align_off;
            line.approx_ink_x = approx_ink_x;

            lines.push(line);
        }

        let metrics_lines = lines
            .iter()
            .map(|line| LineMetrics {
                y_top: line.y_top,
                height: line.height,
                logical_width: line.logical_width,
                approx_ink_width: line.approx_ink_width,
                logical_x: line.logical_x,
                approx_ink_x: line.approx_ink_x,
                byte_start: line.byte_start,
                byte_end: line.byte_end,
                end_kind: line.end_kind,
            })
            .collect::<Vec<_>>();

        let metrics = TextMetrics {
            logical_size: Vec2::new(block_width.ceil(), lines.len() as f32 * line_height),
            approx_ink_bounds: block_ink.unwrap_or(Rect::ZERO),
            line_count: lines.len() as u32,
            truncated_horizontal,
            truncated_vertical,
            lines: metrics_lines,
        };

        Self {
            metrics,
            lines,
            runs: working_runs,
            visible_glyph_count,
        }
    }

    pub fn metrics(&self) -> &TextMetrics {
        &self.metrics
    }
}

impl<G: Copy> TextLayout<G> {
    pub fn iter_resolved_glyphs(&self) -> impl Iterator<Item = LayoutGlyph<G>> + '_ {
        (0..self.lines.len()).flat_map(move |line_index| ResolvedGlyphIter::line(self, line_index))
    }

    pub fn iter_resolved_line_glyphs(
        &self,
        line_index: usize,
    ) -> impl Iterator<Item = LayoutGlyph<G>> + '_ {
        ResolvedGlyphIter::line(self, line_index)
    }

    pub fn resolved_glyphs(&self) -> Vec<LayoutGlyph<G>> {
        self.iter_resolved_glyphs().collect()
    }
}

struct ResolvedGlyphIter<'a, G> {
    layout: &'a TextLayout<G>,
    line_index: usize,
    cluster_index: usize,
    glyph_index: usize,
}

impl<'a, G> ResolvedGlyphIter<'a, G> {
    fn line(layout: &'a TextLayout<G>, line_index: usize) -> Self {
        Self {
            layout,
            line_index,
            cluster_index: 0,
            glyph_index: 0,
        }
    }
}

impl<G: Copy> Iterator for ResolvedGlyphIter<'_, G> {
    type Item = LayoutGlyph<G>;

    fn next(&mut self) -> Option<Self::Item> {
        let line = self.layout.lines.get(self.line_index)?;

        while self.cluster_index < line.clusters.len() {
            let cluster = &line.clusters[self.cluster_index];
            if !cluster.glyphs_visible {
                self.cluster_index += 1;
                self.glyph_index = 0;
                continue;
            }

            match cluster.source {
                WorkingClusterSource::Shaped {
                    run_index,
                    cluster_index,
                } => {
                    let shaped_cluster =
                        &self.layout.runs[run_index].shaped.clusters[cluster_index];
                    if let Some(glyph) = shaped_cluster.glyphs.get(self.glyph_index) {
                        self.glyph_index += 1;
                        return Some(LayoutGlyph {
                            id: glyph.token,
                            origin: Vec2::new(cluster.x + glyph.x, line.baseline_y + glyph.y),
                            advance: glyph.advance,
                            byte_start: cluster.byte_start,
                            approx_ink_bounds: glyph.approx_ink_bounds,
                        });
                    }
                }
                WorkingClusterSource::Empty => {}
            }

            self.cluster_index += 1;
            self.glyph_index = 0;
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CaretPosition, DrawGlyph, FontId, PrepareGlyphRequest, PreparedGlyphToken, TextFlow,
        TextLineLayoutMetrics,
    };

    struct BaselineBackend;

    impl TextBackend for BaselineBackend {
        type ShapedGlyphToken = u32;

        fn line_metrics(&mut self, _style: TextStyle) -> TextLineLayoutMetrics {
            TextLineLayoutMetrics {
                line_height: 30.0,
                baseline_offset: 7.0,
            }
        }

        fn line_height(&mut self, _style: TextStyle) -> f32 {
            30.0
        }

        fn shape_text(
            &mut self,
            text: &str,
            _style: TextStyle,
        ) -> super::super::SharedShapedText<u32> {
            let clusters = text
                .char_indices()
                .map(|(byte_start, ch)| super::super::ShapedCluster {
                    byte_start,
                    byte_end: byte_start + ch.len_utf8(),
                    advance: 8.0,
                    is_whitespace: ch.is_whitespace(),
                    glyphs: vec![crate::ShapedGlyph {
                        token: ch as u32,
                        x: 0.0,
                        y: 0.0,
                        advance: 8.0,
                        approx_ink_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
                    }],
                    approx_ink_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
                })
                .collect();
            std::rc::Rc::new(super::super::ShapedText { clusters })
        }

        fn prepare_glyph(&mut self, request: PrepareGlyphRequest<u32>) -> Option<DrawGlyph> {
            Some(DrawGlyph {
                token: PreparedGlyphToken(request.glyph as u64),
                top_left: request.glyph_origin,
            })
        }
    }

    #[test]
    fn layout_uses_backend_baseline_offset_not_style_size() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x", style, TextBounds::UNBOUNDED);

        let glyphs = layout.resolved_glyphs();
        assert_eq!(glyphs[0].origin.y, 7.0);
    }

    #[test]
    fn multiline_baselines_use_line_height_plus_baseline_offset() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x\ny", style, TextBounds::UNBOUNDED);

        let glyphs = layout.resolved_glyphs();
        assert_eq!(glyphs[0].origin.y, 7.0);
        assert_eq!(glyphs[1].origin.y, 37.0);
    }

    #[test]
    fn caret_geometry_uses_line_height() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x\ny", style, TextBounds::UNBOUNDED);

        let caret = layout.caret_geom(CaretPosition::BeforeCluster {
            cluster_byte_start: 2,
        });
        assert_eq!(caret.y_top, 30.0);
        assert_eq!(caret.height, 30.0);
    }
}
