use super::cluster_layout::{
    append_empty_after_terminal_soft_wrap_boundary, logical_cluster_line_start,
    logical_cluster_line_width, make_source_line, wrap_clusters, wrap_clusters_at_words,
};
use super::text_overflow::apply_ellipsis_x;
use super::{
    EllipsisFallback, LayoutGlyph, LineEndKind, LineMetrics, OverflowX, OverflowY, ShapedText,
    TextBackend, TextBounds, TextCluster, TextLayout, TextLine, TextLineAlign, TextMetrics,
    TextStyle,
};
use crate::{
    draw::DrawCommands,
    types::{Color, Rect, Vec2},
};
use std::hash::Hash;
use std::rc::Rc;

#[allow(dead_code)]
pub(super) struct WorkingRun<G> {
    pub(super) shaped: Rc<ShapedText<G>>,
    pub(super) segment_start: usize,
}

#[derive(Debug, Clone)]
pub(super) enum WorkingClusterSource<G> {
    #[allow(dead_code)]
    Shaped {
        run_index: usize,
        cluster_index: usize,
    },
    Empty,
    SyntheticGlyphs {
        glyphs: Vec<LayoutGlyph<G>>,
    },
}

/// Mutable source-line representation used while applying wrapping and overflow.
pub(super) struct WorkingSourceLine<G> {
    pub(super) clusters: Vec<WorkingCluster<G>>,
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) baseline_y: f32,
}

/// Mutable visual line representation before final block positioning.
pub(super) struct WorkingProcessedLine<G> {
    pub(super) clusters: Vec<WorkingCluster<G>>,
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) baseline_y: f32,
    pub(super) end_kind: LineEndKind,
}

#[derive(Debug, Clone)]
pub(super) struct WorkingCluster<G> {
    pub(super) source: WorkingClusterSource<G>,
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) x: f32,
    pub(super) advance: f32,
    pub(super) is_hard_break: bool,
    pub(super) is_whitespace: bool,
    pub(super) is_soft_wrap_boundary: bool,
    pub(super) glyphs_visible: bool,
}

impl<G> WorkingCluster<G> {
    pub(super) fn end_x(&self) -> f32 {
        self.x + self.advance
    }

    pub(super) fn shift_x(&mut self, dx: f32) {
        self.x += dx;
    }

    pub(super) fn collapse_soft_wrap_boundary(&mut self) {
        self.advance = 0.0;
        self.is_soft_wrap_boundary = true;
        self.glyphs_visible = false;
    }
}

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
) -> TextLayout<B::ShapedGlyphId> {
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
) -> TextLayout<B::ShapedGlyphId> {
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
) -> TextLayout<B::ShapedGlyphId> {
    let layout = layout_text_in_rect(backend, text, style, rect);
    layout.emit_glyphs(
        commands,
        backend,
        Vec2::new(rect.x, rect.y),
        style,
        color,
        z,
    );
    layout
}

fn union_rect(acc: Option<Rect>, rect: Rect) -> Option<Rect> {
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

fn translated_approx_ink<G>(
    glyph: &LayoutGlyph<G>,
    line_y_top: f32,
    line_height: f32,
) -> Option<Rect> {
    match glyph.approx_ink_bounds {
        Some(rect) if rect.w > 0.0 && rect.h > 0.0 => Some(Rect::new(
            glyph.origin.x + rect.x,
            glyph.origin.y + rect.y,
            rect.w,
            rect.h,
        )),
        Some(_) => None,
        None if glyph.advance > 0.0 => Some(Rect::new(
            glyph.origin.x,
            line_y_top,
            glyph.advance,
            line_height,
        )),
        None => None,
    }
}

fn materialize_working_cluster_glyphs<G: Copy>(
    cluster: &WorkingCluster<G>,
    baseline_y: f32,
) -> Vec<LayoutGlyph<G>> {
    if !cluster.glyphs_visible {
        return Vec::new();
    }

    match &cluster.source {
        WorkingClusterSource::SyntheticGlyphs { glyphs } => glyphs
            .iter()
            .map(|glyph| LayoutGlyph {
                id: glyph.id,
                origin: Vec2::new(cluster.x + glyph.origin.x, baseline_y + glyph.origin.y),
                advance: glyph.advance,
                byte_start: cluster.byte_start,
                approx_ink_bounds: glyph.approx_ink_bounds,
            })
            .collect(),
        WorkingClusterSource::Empty | WorkingClusterSource::Shaped { .. } => Vec::new(),
    }
}

impl<G: Copy + Eq + Hash> TextLayout<G> {
    fn from_backend<B: TextBackend<ShapedGlyphId = G>>(
        backend: &mut B,
        text: &str,
        style: TextStyle,
        bounds: TextBounds,
    ) -> Self {
        let flow = style.flow;
        let line_metrics = backend.line_metrics(style);
        let line_height = line_metrics.line_height.round().max(1.0);
        let baseline_offset = line_metrics.baseline_offset.round();
        let mut source_lines = Vec::new();
        let mut start_byte = 0;

        for (idx, ch) in text.char_indices() {
            if ch == '\n' {
                source_lines.push(make_source_line(
                    backend,
                    text,
                    style,
                    start_byte,
                    idx,
                    true,
                    source_lines.len(),
                    line_height,
                    baseline_offset,
                ));
                start_byte = idx + ch.len_utf8();
            }
        }
        if start_byte <= text.len() {
            source_lines.push(make_source_line(
                backend,
                text,
                style,
                start_byte,
                text.len(),
                false,
                source_lines.len(),
                line_height,
                baseline_offset,
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
        let mut processed_lines = Vec::new();

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

            let mut final_sublines = Vec::new();
            let mut overflow_line_end_kind = None;
            if let Some(w) = bounds.max_width {
                match flow.overflow_x {
                    OverflowX::WrapWord { fallback } => {
                        final_sublines.extend(wrap_clusters_at_words(seg, w, fallback));
                    }
                    OverflowX::WrapCluster { fallback } => {
                        final_sublines.extend(wrap_clusters(seg, w, fallback));
                    }
                    _ => {
                        if logical_line_w > w + 0.5 {
                            truncated_horizontal = true;
                            match flow.overflow_x {
                                OverflowX::Ellipsis { fallback } => {
                                    overflow_line_end_kind = Some(LineEndKind::EllipsisX);
                                    final_sublines.push(apply_ellipsis_x(
                                        backend,
                                        seg,
                                        w,
                                        style,
                                        fallback,
                                        line.baseline_y,
                                    ));
                                }
                                OverflowX::Keep => {
                                    overflow_line_end_kind = Some(LineEndKind::OverflowKeep);
                                    let mut out = Vec::new();
                                    for cluster in seg {
                                        let end_x = cluster.end_x();
                                        out.push(cluster);
                                        if end_x > w {
                                            break;
                                        }
                                    }
                                    final_sublines.push(out);
                                }
                                OverflowX::Drop => {
                                    overflow_line_end_kind = Some(LineEndKind::OverflowDrop);
                                    let mut out = Vec::new();
                                    for cluster in seg {
                                        if cluster.end_x() <= w {
                                            out.push(cluster);
                                        } else {
                                            break;
                                        }
                                    }
                                    final_sublines.push(out);
                                }
                                _ => unreachable!(),
                            }
                        } else {
                            final_sublines.push(seg);
                        }
                    }
                }
            } else {
                final_sublines.push(seg);
            }

            append_empty_after_terminal_soft_wrap_boundary(&mut final_sublines, line.byte_end);

            let mut sub_starts = Vec::new();
            let mut previous_end = line.byte_start;
            for (idx, sub_seg) in final_sublines.iter().enumerate() {
                let byte_start = if idx == 0 {
                    line.byte_start
                } else {
                    sub_seg
                        .first()
                        .map(|cluster| cluster.byte_start)
                        .unwrap_or(previous_end)
                };
                previous_end = sub_seg
                    .last()
                    .map(|cluster| cluster.byte_end)
                    .unwrap_or(byte_start);
                sub_starts.push(byte_start);
            }
            sub_starts.push(line.byte_end);

            for (idx, sub_seg) in final_sublines.into_iter().enumerate() {
                let end_kind = overflow_line_end_kind.unwrap_or_else(|| {
                    if sub_seg.last().is_some_and(|cluster| cluster.is_hard_break) {
                        LineEndKind::HardNewline
                    } else if sub_seg
                        .last()
                        .is_some_and(|cluster| cluster.is_soft_wrap_boundary)
                    {
                        LineEndKind::SoftWrapWhitespace
                    } else if idx + 1 < sub_starts.len() - 1 {
                        LineEndKind::SoftWrapNonWhitespace
                    } else {
                        LineEndKind::EndOfText
                    }
                });
                processed_lines.push(WorkingProcessedLine {
                    clusters: sub_seg,
                    byte_start: sub_starts[idx],
                    byte_end: sub_starts[idx + 1],
                    baseline_y: line.baseline_y,
                    end_kind,
                });
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
                            last_line_clusters,
                            w,
                            style,
                            fallback,
                            processed_lines[last_idx].baseline_y,
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
            processed_lines.push(WorkingProcessedLine {
                clusters: Vec::new(),
                byte_start: 0,
                byte_end: text.len(),
                baseline_y: baseline_offset,
                end_kind: LineEndKind::EndOfText,
            });
        }

        let mut glyphs = Vec::new();
        let mut clusters = Vec::new();
        let mut lines = Vec::new();
        let mut block_width = 0.0_f32;
        let mut block_ink: Option<Rect> = None;

        for (idx, mut line) in processed_lines.into_iter().enumerate() {
            let new_baseline_y = idx as f32 * line_height + baseline_offset;
            let y_top = idx as f32 * line_height;

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

            let glyph_start = glyphs.len();
            let cluster_start = clusters.len();
            for cluster in line.clusters {
                let cluster_glyph_start = glyphs.len();
                glyphs.extend(materialize_working_cluster_glyphs(
                    &cluster,
                    new_baseline_y.round(),
                ));
                clusters.push(TextCluster {
                    byte_start: cluster.byte_start,
                    byte_end: cluster.byte_end,
                    glyph_start: cluster_glyph_start,
                    glyph_end: glyphs.len(),
                    x: cluster.x,
                    advance: cluster.advance,
                    is_hard_break: cluster.is_hard_break,
                    is_whitespace: cluster.is_whitespace,
                    is_soft_wrap_boundary: cluster.is_soft_wrap_boundary,
                });
            }

            let line_ink = glyphs[glyph_start..]
                .iter()
                .filter_map(|glyph| translated_approx_ink(glyph, y_top, line_height))
                .fold(None, union_rect);
            block_ink = line_ink.into_iter().fold(block_ink, union_rect);
            let (ink_x, ink_width) = line_ink.map_or((align_off, 0.0), |rect| (rect.x, rect.w));

            lines.push(TextLine {
                y_top,
                height: line_height,
                glyph_start,
                glyph_end: glyphs.len(),
                cluster_start,
                cluster_end: clusters.len(),
                byte_start: line.byte_start,
                byte_end: line.byte_end,
                logical_width: logical_line_w,
                ink_width,
                logical_x: align_off,
                ink_x,
                end_kind: line.end_kind,
            });
        }

        let metrics_lines = lines
            .iter()
            .map(|line| LineMetrics {
                y_top: line.y_top,
                height: line.height,
                logical_width: line.logical_width,
                approx_ink_width: line.ink_width,
                logical_x: line.logical_x,
                approx_ink_x: line.ink_x,
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
            clusters,
            glyphs,
        }
    }

    pub fn metrics(&self) -> &TextMetrics {
        &self.metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CaretPosition, DrawGlyph, FontId, PrepareGlyphRequest, PreparedGlyphHandle, TextFlow,
        TextLineLayoutMetrics,
    };

    struct BaselineBackend;

    impl TextBackend for BaselineBackend {
        type ShapedGlyphId = u32;

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
                        id: ch as u32,
                        x: 0.0,
                        y: 0.0,
                        advance: 8.0,
                        approx_ink_bounds: Some(Rect::new(0.0, 0.0, 8.0, 16.0)),
                    }],
                })
                .collect();
            std::rc::Rc::new(super::super::ShapedText { clusters })
        }

        fn prepare_glyph(&mut self, request: PrepareGlyphRequest<u32>) -> Option<DrawGlyph> {
            Some(DrawGlyph {
                handle: PreparedGlyphHandle(request.glyph),
                top_left: request.glyph_origin,
            })
        }
    }

    #[test]
    fn layout_uses_backend_baseline_offset_not_style_size() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x", style, TextBounds::UNBOUNDED);

        assert_eq!(layout.glyphs[0].origin.y, 7.0);
    }

    #[test]
    fn multiline_baselines_use_line_height_plus_baseline_offset() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x\ny", style, TextBounds::UNBOUNDED);

        assert_eq!(layout.glyphs[0].origin.y, 7.0);
        assert_eq!(layout.glyphs[1].origin.y, 37.0);
    }

    #[test]
    fn caret_geometry_uses_line_height() {
        let mut backend = BaselineBackend;
        let style = TextStyle::new(FontId(0), 20.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut backend, "x\ny", style, TextBounds::UNBOUNDED);

        let caret = layout.caret_geom(CaretPosition::BeforeCluster {
            cluster_byte_index: 2,
        });
        assert_eq!(caret.y_top, 30.0);
        assert_eq!(caret.height, 30.0);
    }
}
