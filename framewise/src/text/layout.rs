use super::cluster_layout::{
    append_empty_after_terminal_soft_wrap_boundary, logical_cluster_line_start,
    logical_cluster_line_width, make_source_line, wrap_clusters, wrap_clusters_at_words,
};
use super::overflow::apply_ellipsis_x;
use super::{
    CaretGeom, CaretPosition, EllipsisFallback, LayoutGlyph, LineEndKind, LineMetrics, OverflowX,
    OverflowY, TextBackend, TextBounds, TextCluster, TextHandle, TextLayout, TextLine,
    TextLineAlign, TextMetrics, TextStyle,
};
use crate::{
    draw::DrawCommands,
    types::{Color, Rect, Vec2},
};
use std::hash::Hash;

pub(super) struct SourceLine<G> {
    pub(super) clusters: Vec<OwnedCluster<G>>,
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) baseline_y: f32,
}

pub(super) struct ProcessedLine<G> {
    pub(super) clusters: Vec<OwnedCluster<G>>,
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) baseline_y: f32,
    pub(super) end_kind: LineEndKind,
}

#[derive(Debug, Clone)]
pub(super) struct OwnedCluster<G> {
    pub(super) byte_start: usize,
    pub(super) byte_end: usize,
    pub(super) x: f32,
    pub(super) advance: f32,
    pub(super) is_hard_break: bool,
    pub(super) is_whitespace: bool,
    pub(super) is_soft_wrap_boundary: bool,
    pub(super) glyphs: Vec<LayoutGlyph<G>>,
}

impl<G> OwnedCluster<G> {
    pub(super) fn end_x(&self) -> f32 {
        self.x + self.advance
    }

    pub(super) fn shift_x(&mut self, dx: f32) {
        self.x += dx;
        for glyph in &mut self.glyphs {
            glyph.origin.x += dx;
        }
    }

    pub(super) fn collapse_soft_wrap_boundary(&mut self) {
        self.advance = 0.0;
        self.is_soft_wrap_boundary = true;
        for glyph in &mut self.glyphs {
            glyph.advance = 0.0;
        }
    }
}

pub fn layout_text<B: TextBackend>(
    backend: &mut B,
    text: &str,
    style: TextStyle,
    bounds: TextBounds,
) -> TextLayout<B::ShapedGlyphId> {
    TextLayout::from_backend(backend, text, style, bounds)
}

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
                processed_lines.push(ProcessedLine {
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
            processed_lines.push(ProcessedLine {
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

        for (idx, mut line) in processed_lines.into_iter().enumerate() {
            let new_baseline_y = idx as f32 * line_height + baseline_offset;
            let y_top = idx as f32 * line_height;

            for cluster in &mut line.clusters {
                for glyph in &mut cluster.glyphs {
                    let baseline_relative_y = glyph.origin.y - line.baseline_y;
                    glyph.origin.y = (new_baseline_y + baseline_relative_y).round();
                }
            }

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
                glyphs.extend(cluster.glyphs);
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
                ink_width: logical_line_w,
                logical_x: align_off,
                ink_x: align_off,
                end_kind: line.end_kind,
            });
        }

        let metrics_lines = lines
            .iter()
            .map(|line| LineMetrics {
                y_top: line.y_top,
                height: line.height,
                logical_width: line.logical_width,
                ink_width: line.ink_width,
                logical_x: line.logical_x,
                ink_x: line.ink_x,
                byte_start: line.byte_start,
                byte_end: line.byte_end,
                end_kind: line.end_kind,
            })
            .collect::<Vec<_>>();

        let metrics = TextMetrics {
            logical_size: Vec2::new(block_width.ceil(), lines.len() as f32 * line_height),
            ink_bounds: if glyphs.is_empty() {
                Rect::ZERO
            } else {
                Rect::new(
                    0.0,
                    0.0,
                    block_width.ceil(),
                    lines.len() as f32 * line_height,
                )
            },
            line_count: lines.len() as u32,
            truncated_horizontal,
            truncated_vertical,
            lines: metrics_lines,
        };

        Self {
            handle: TextHandle(usize::MAX),
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

/// Implemented by the application to measure, shape, and cache text.
///
/// Framewise owns *policy* (whether to wrap, how much space is available, what to
/// do on overflow) and hands it down; the `TextSystem` owns *shaping* (where
/// lines actually break, how the ellipsis is fitted, how glyphs are positioned)
/// and hands geometry back. Framewise never inspects glyphs.
///
/// ### Character Preservation Contract
///
/// A text system must account for every source character in its layout, source
/// byte ranges, caret positions, hit-testing, and selection geometry. Characters
/// may be omitted from emitted geometry only when the selected overflow policy
/// explicitly truncates content, such as `Drop`, ellipsis fitting, or a `Drop`
/// fallback.
///
/// ### Newlines and Empty Lines
///
/// - Every hard newline (`\n`) must start a new visual line.
/// - Empty lines (such as a line consisting only of `\n`, or a trailing newline at
///   the end of the text) must produce a corresponding `LineMetrics` entry and
///   contribute to the vertical layout height.
/// - If preserved whitespace at the end of the text overflows and is collapsed at
///   a soft-wrap boundary, the layout must still create a following empty visual
///   line. This mirrors the behavior of a trailing hard newline: the boundary
///   character belongs to the previous line, while the caret position after it is
///   on the next empty line.
///
/// ### Whitespace Wrapping
///
/// Preserved whitespace characters are individually wrap-capable. Under word
/// wrapping, each whitespace cluster is treated as a single-cluster word-like
/// unit for overflow and fallback purposes. Under cluster wrapping, whitespace
/// is an ordinary cluster for wrapping and fallback purposes.
///
/// Whitespace does not have a separate fallback chain. It follows the selected
/// [`OverflowX`] policy normally. The special case is only how a whitespace
/// cluster is represented when it becomes a soft-wrap boundary after other
/// clusters have already been admitted to the current visual line: instead of
/// producing a whitespace-only visual line, that single boundary character is
/// kept on the previous line with zero advance.
///
/// At a soft-wrap boundary, exactly one preserved whitespace character is
/// collapsed if either that whitespace character is the overflowing unit, or
/// the overflowing unit immediately follows that whitespace character and the
/// line already contains non-whitespace content before the whitespace. The
/// non-whitespace requirement preserves leading indentation: leading whitespace
/// at the start of a visual line remains visible unless one of those whitespace
/// characters independently becomes the boundary character of a later soft
/// wrap.
///
/// The collapsed whitespace is kept in the previous visual line's byte range
/// and caret/selection model, like a hard newline, but is assigned zero visual
/// advance and excluded from that line's `logical_width`. Adjacent whitespace
/// remains preserved and participates in wrapping normally. A soft wrap
/// collapses only the single boundary whitespace character for that wrap; later
/// adjacent whitespace is not collapsed unless it independently becomes the
/// boundary character of a later soft wrap.
///
/// See `DESIGN.md` ("Text Wrapping And Whitespace") for rationale and examples.
///
/// ### Logical Bounds and Ink Bounds
///
/// The bounds passed into this trait are **logical layout bounds**. They constrain
/// text flow: advances, wrapping, ellipsis, alignment frames, line admission,
/// caret positions, and hit-testing. They are not a guarantee that every visible
/// pixel of ink will be contained inside the same rectangle.
///
/// **Ink bounds** are the visible bounds of the shaped/rasterized glyphs. They
/// are an output of shaping and rendering, not something the caller can know
/// before calling `measure` or `prepare`. Ink may sit inside the logical box,
/// protrude outside it, be empty for whitespace, or be offset by glyph bearings,
/// overhangs, accents, combining marks, or symbol placement.
///
/// [`TextMetrics`] reports both logical geometry and ink bounds. Callers that
/// require strict pixel containment should clip, add padding, or use a future
/// ink-fitting policy rather than assuming that the input bounds contain all
/// rendered pixels.
///
/// All positions returned by this trait are in **block-local coordinates**: the
/// origin is the block's top-left corner, with y increasing downward. The caller
/// translates the block to its final screen position via the `Rect` it passes to
/// [`prepare`](Self::prepare) and the rect on `DrawCmd::Text`.
///
/// ### Empty Text
///
/// Empty input (`""`) is a valid layout, not an error case. Implementations must
/// report one visible empty line with normal line metrics and zero text advance:
///
/// - `line_count == 1` and `lines.len() == 1`.
/// - `logical_size.x == 0.0`.
/// - `logical_size.y` is one line height, so empty editors can size and draw a
///   non-zero-height caret.
/// - `ink_bounds` is empty because no glyph ink is emitted.
/// - The single line has `byte_start == byte_end == 0`,
///   `end_kind == LineEndKind::EndOfText`, and a positive `height`.
///
/// Preparing empty text must allocate a valid handle whose cached layout may
/// contain zero clusters and zero glyphs, but must still contain the single
/// empty line record above. All caret, hit-testing, and navigation methods must
/// handle that handle without panicking.
pub trait TextSystem {
    /// Measure `text` without committing it for drawing (no handle is produced).
    ///
    /// Used by widgets' intrinsic-sizing companions to learn how large a piece of
    /// text wants to be inside a given space, before the final rect is resolved.
    /// The returned [`TextMetrics`] reflect `flow` applied against `bounds` — see
    /// [`TextBounds`] for how the bounded/unbounded axes drive reflow.
    ///
    /// The returned `logical_size` represents logical layout geometry: advance-based
    /// line width and line-height-based block height after the selected overflow
    /// policy has been applied. It is not a tight ink box.
    ///
    /// With strict overflow policies the logical size should fit within bounded
    /// input axes. Policies that explicitly keep overflowing content may return
    /// a logical size larger than the supplied bounds. `ink_bounds` reports the
    /// visible bounds of the emitted glyphs, which may protrude outside the
    /// logical size due to font metrics and glyph placement.
    ///
    /// `flow.line_align` has no effect on logical sizing, wrapping, or
    /// truncation: those decisions are made in logical line space. It may affect
    /// `ink_bounds`, because alignment shifts the admitted glyphs within the
    /// available line width.
    ///
    /// For empty `text`, this must return the empty-text metrics described in
    /// the trait-level contract: one normal-height line, zero width, and empty
    /// ink bounds.
    ///
    /// Must be free of observable side effects on the run table — calling
    /// `measure` does not allocate a [`TextHandle`].
    fn measure(&mut self, text: &str, style: TextStyle, bounds: TextBounds) -> TextMetrics;

    /// Shape `text` for drawing into `rect` and register it, returning a handle.
    ///
    /// `rect` is the fully concrete **logical layout rect** by the time this is
    /// called: its width is the wrap/alignment width, its height is the vertical
    /// layout or clip extent, and its origin is the block origin used for
    /// rendering.
    ///
    /// The screen position (`rect.x`, `rect.y`) must be known at this stage because
    /// modern text shapers and font rasterizers use the absolute physical screen coordinates
    /// to apply subpixel offsets/positioning. This ensures crisp glyph rasterization at
    /// fractional pixel boundaries and prevents blurriness.
    ///
    /// The text system may produce ink that extends outside this rect. A caller
    /// that needs hard containment must apply clipping or provide padding.
    ///
    /// The returned [`TextLayout::metrics`] equal what [`measure`](Self::measure)
    /// would report for the same `text` and `style`, with
    /// `TextBounds { max_width: Some(rect.w), max_height: Some(rect.h) }`.
    ///
    /// For empty `text`, this must still return a valid handle. The prepared run
    /// may have no clusters or glyphs, but it must have one empty line with
    /// positive line height so caret and hit-testing methods have stable line
    /// geometry.
    ///
    /// The handle is valid until the next frame reset (see [`TextHandle`]).
    fn prepare(&mut self, text: &str, style: TextStyle, rect: Rect) -> TextLayout;

    /// Caret geometry for a prepared visual caret position.
    ///
    /// Caret positions are in the same logical block coordinate system used by
    /// `prepare`. They should follow shaped advances and line metrics, not the
    /// tight ink box of the surrounding text.
    ///
    /// - `BeforeCluster { cluster_byte_index }` returns the leading visual edge
    ///   of the anchored cluster.
    /// - `AfterCluster { cluster_byte_index }` returns the trailing visual edge
    ///   of the anchored cluster.
    /// - `EmptyText` returns the start of the single empty line with a positive
    ///   height.
    ///
    /// If an empty prepared layout is queried with any cluster-anchored
    /// position, implementations should clamp to the same geometry as
    /// `EmptyText` instead of panicking.
    ///
    /// Hard newline clusters have newline-specific visual anchors:
    ///
    /// - `BeforeCluster` for the newline is the trailing text position before
    ///   the newline, on the previous visual line.
    /// - `AfterCluster` for the newline is the start of the following visual
    ///   line.
    ///
    /// Collapsed soft-wrap-boundary whitespace has the same shape:
    ///
    /// - `BeforeCluster` for the boundary whitespace is the end of the previous
    ///   visual line, with the boundary whitespace retained in that line's byte
    ///   range and caret/selection model.
    /// - `AfterCluster` for the boundary whitespace is the start of the
    ///   following visual line. If the boundary whitespace is terminal, this is
    ///   the following empty visual line created for editor feedback.
    fn caret_geom(&self, handle: TextHandle, position: CaretPosition) -> CaretGeom;

    /// Hit-test a point (block-local coordinates) to the nearest character
    /// boundary, returning a visual caret anchor.
    ///
    /// The coordinates `pos` are in the logical block coordinate system used by
    /// `prepare`. Hit testing should compare against the shaped logical cluster
    /// positions in the cached run.
    ///
    /// The point is resolved to a line by `y` first, then to the nearest gap
    /// between clusters by `x`:
    /// - Points above the block clamp to the first line; points below clamp to
    ///   the last line.
    /// - Points to the left of a non-empty line return `BeforeCluster` for that
    ///   line's first cluster.
    /// - Points to the right of a line clamp to the end of the *visible* content
    ///   on that line. If the line ends with a hard newline or collapsed
    ///   soft-wrap boundary, this returns a caret anchored to that boundary
    ///   cluster so the visual line is preserved.
    /// - Points on an empty line return the visual position for that empty line:
    ///   `EmptyText` for empty input, or `AfterCluster` for the previous hard
    ///   newline / terminal collapsed soft-wrap boundary when the empty line
    ///   exists because of such a boundary.
    /// - Points anywhere in an empty prepared layout return `EmptyText`.
    ///
    /// The returned cluster anchor can be converted to an insertion byte index
    /// with [`TextSystem::caret_insertion_byte`].
    fn hit_test_caret(&self, handle: TextHandle, pos: Vec2) -> CaretPosition;

    /// Convert a prepared visual caret position into the insertion byte index
    /// used by text editing operations.
    ///
    /// `BeforeCluster` returns the anchored cluster's `byte_start`;
    /// `AfterCluster` returns its `byte_end`; `EmptyText` returns `0`.
    ///
    /// For an empty prepared layout, every position must map to byte `0`,
    /// including stale or invalid cluster-anchored positions.
    fn caret_insertion_byte(&self, handle: TextHandle, position: CaretPosition) -> usize;

    /// Choose a canonical visual caret anchor for a programmatic insertion byte
    /// index.
    ///
    /// This is intended for non-hit-tested movement such as "go to byte 0",
    /// "go to end", or adapting existing byte-oriented editor state. It should
    /// return `BeforeCluster` for the first cluster at or after the byte, and
    /// `AfterCluster` for the last cluster when the byte is at or beyond the
    /// prepared text's end. Empty prepared text returns `EmptyText` for every
    /// requested byte index.
    fn caret_position_at_insertion_byte(
        &self,
        handle: TextHandle,
        byte_index: usize,
    ) -> CaretPosition;

    /// Move one shaped cluster boundary to the left.
    ///
    /// Implementations should move by the prepared text's cluster model, not by
    /// UTF-8 scalar boundaries. When movement is possible, the returned caret
    /// should map to a different insertion byte from `position`. At hard
    /// newlines and collapsed soft-wrap boundary whitespace, the returned
    /// [`CaretPosition`] should preserve the visual side reached by moving from
    /// the right, such as `AfterCluster` for the boundary character when landing
    /// immediately after it.
    ///
    /// The default implementation is a no-op for text systems used only by
    /// non-editing tests. Editable text systems should override it.
    ///
    /// Empty prepared text has no previous insertion boundary; editable
    /// implementations must return `EmptyText`.
    fn previous_caret_position(
        &self,
        _handle: TextHandle,
        position: CaretPosition,
    ) -> CaretPosition {
        position
    }

    /// Move one shaped cluster boundary to the right.
    ///
    /// Implementations should move by the prepared text's cluster model, not by
    /// UTF-8 scalar boundaries. When movement is possible, the returned caret
    /// should map to a different insertion byte from `position`. At hard
    /// newlines and collapsed soft-wrap boundary whitespace, the returned
    /// [`CaretPosition`] should preserve the visual side reached by moving from
    /// the left, such as `BeforeCluster` for the boundary character when landing
    /// immediately before it.
    ///
    /// The default implementation is a no-op for text systems used only by
    /// non-editing tests. Editable text systems should override it.
    ///
    /// Empty prepared text has no next insertion boundary; editable
    /// implementations must return `EmptyText`.
    fn next_caret_position(&self, _handle: TextHandle, position: CaretPosition) -> CaretPosition {
        position
    }

    /// Hit-test a point (block-local coordinates) to a shaped glyph cluster,
    /// returning the start byte index of the hit cluster.
    ///
    /// The coordinates `pos` are in the logical block coordinate system used by
    /// `prepare`. Hit testing compares against the shaped logical cluster
    /// positions in the cached run.
    ///
    /// The point is resolved to a line by `y` first, then to the cluster containing `x`:
    /// - Points above the block clamp to the first line; points below clamp to
    ///   the last line.
    /// - Points to the left of a line clamp to the first cluster of that line.
    /// - Points to the right of a line clamp to the last cluster of that line.
    /// - For multi-byte characters or complex clusters, this returns the starting
    ///   byte index of the cluster.
    /// - If the line ends with a boundary cluster that has no visual advance,
    ///   such as a hard newline (`\n`) or collapsed soft-wrap boundary
    ///   whitespace, a hit to the right of the line or on that boundary must
    ///   return the boundary cluster's start byte index.
    /// - Empty prepared text has no clusters, so every hit returns byte `0`.
    fn hit_test_cluster(&self, handle: TextHandle, pos: Vec2) -> usize;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        DrawGlyph, FontId, PrepareGlyphRequest, PreparedGlyphHandle, TextFlow,
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

        fn shape_text(&mut self, text: &str, _style: TextStyle) -> super::super::ShapedText<u32> {
            let clusters = text
                .char_indices()
                .map(|(byte_start, ch)| super::super::ShapedCluster {
                    byte_start,
                    byte_end: byte_start + ch.len_utf8(),
                    advance: 8.0,
                    is_whitespace: ch.is_whitespace(),
                    glyphs: vec![super::super::ShapedGlyph {
                        id: ch as u32,
                        x: 0.0,
                        y: 0.0,
                        advance: 8.0,
                    }],
                })
                .collect();
            super::super::ShapedText { clusters }
        }

        fn shape_ellipsis(&mut self, style: TextStyle) -> super::super::ShapedText<u32> {
            self.shape_text(".", style)
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
