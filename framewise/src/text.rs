use crate::{
    draw::{DrawCommands, DrawGlyph},
    layout::Align,
    types::{Color, Rect, Vec2},
};
use std::hash::Hash;

/// A lightweight application-owned font handle.
///
/// Framewise never loads or owns font files. It only passes this handle to the
/// application's `TextSystem`, which decides how the handle maps to real font
/// data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontId(pub u16);

/// Policy for resolving the visual height of text lines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineHeight {
    /// Use the font's natural typographic metrics (typically 1.2x - 1.4x of size).
    Normal,
    /// Override the line height as a multiplier of the font size (e.g. 1.55).
    Relative(f32),
}

/// Groups typography attributes together for reuse across the text system and widgets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextStyle {
    /// The lightweight font handle.
    pub font: FontId,
    /// The font size in logical pixels.
    pub size: f32,
    /// Font weight (typically 100-900).
    pub weight: u16,
    /// How the text flows and handles overflow.
    pub flow: TextFlow,
    /// Whether the text should be rendered in italics.
    pub italic: bool,
    /// Custom spacing between letters, in em units. Defaults to 0.0.
    pub letter_spacing: f32,
    /// Custom line height policy. Defaults to LineHeight::Normal.
    pub line_height: LineHeight,
}

impl TextStyle {
    pub fn new(font: FontId, size: f32, weight: u16, flow: TextFlow) -> Self {
        Self {
            font,
            size,
            weight,
            flow,
            italic: false,
            letter_spacing: 0.0,
            line_height: LineHeight::Normal,
        }
    }

    pub fn with_italic(mut self, italic: bool) -> Self {
        self.italic = italic;
        self
    }

    pub fn with_flow(mut self, flow: TextFlow) -> Self {
        self.flow = flow;
        self
    }

    pub fn with_letter_spacing(mut self, spacing: f32) -> Self {
        self.letter_spacing = spacing;
        self
    }

    pub fn with_line_height(mut self, line_height: LineHeight) -> Self {
        self.line_height = line_height;
        self
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: FontId::default(),
            size: 14.0,
            weight: 400,
            flow: TextFlow::single_line(),
            italic: false,
            letter_spacing: 0.0,
            line_height: LineHeight::Normal,
        }
    }
}

/// Semantic font roles used by themes and builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontRole {
    Sans,
    Mono,
}

/// An opaque handle to a text layout prepared by the application's text system.
///
/// Framewise does not know how text is shaped or rasterised. It just passes this
/// handle to the renderer via `DrawCmd::Text`.
///
/// A handle is produced by [`TextSystem::prepare`] and is valid only until the
/// text system's next frame reset (the implementation clears its run table each
/// frame). Handles must not be retained across frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextHandle(pub usize);

/// Backend-to-Framewise shaped text output.
///
/// This is a logical shaping result only. It contains no renderer resources and
/// no final line layout.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText<G> {
    pub clusters: Vec<ShapedCluster<G>>,
}

/// One indivisible shaped cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedCluster<G> {
    pub byte_start: usize,
    pub byte_end: usize,
    pub advance: f32,
    pub is_whitespace: bool,
    pub glyphs: Vec<ShapedGlyph<G>>,
}

/// One shaped glyph inside a cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph<G> {
    pub id: G,
    /// Position relative to the shaped text run before Framewise line layout.
    pub x: f32,
    /// Position relative to the line baseline before Framewise line layout.
    pub y: f32,
    pub advance: f32,
}

/// Request for a backend-owned glyph preparation/rasterisation step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrepareGlyphRequest<G> {
    pub glyph: G,
    pub style: TextStyle,

    /// Final logical-pixel origin of the shaped glyph.
    ///
    /// This is after layout, wrapping, line alignment, baseline placement, and
    /// caller draw origin have all been applied. The backend may use this for
    /// subpixel bin selection and returns a [`DrawGlyph`] with bitmap placement
    /// applied.
    pub glyph_origin: Vec2,
}

/// Low-level text backend contract used by Framewise-owned text layout.
///
/// Framewise owns layout policy; the backend owns font selection, shaping,
/// glyph rasterisation, glyph caching, and renderer resource handles.
pub trait TextBackend {
    type ShapedGlyphId: Copy + Eq + Hash;

    fn line_height(&mut self, style: TextStyle) -> f32;

    fn shape_text(&mut self, text: &str, style: TextStyle) -> ShapedText<Self::ShapedGlyphId>;

    fn shape_ellipsis(&mut self, style: TextStyle) -> ShapedText<Self::ShapedGlyphId>;

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph>;
}

// ── Flow & overflow policy ──────────────────────────────────────────────────

/// How a block of text flows and fills the logical space it is measured or
/// drawn against.
///
/// Covers line breaking, per-line horizontal alignment, and overflow handling.
/// This struct carries **policy only** — never dimensions. The available space is
/// supplied separately: as [`TextBounds`] when measuring, or as the concrete
/// `Rect` when preparing for draw. Keeping size out of here is deliberate: the
/// same policy applies whether an axis is bounded, unbounded, or fixed.
///
/// Text overflow is modelled independently on the inline axis (`overflow_x`) and
/// block axis (`overflow_y`) because they answer different questions:
///
/// - X overflow asks what to do when the next logical text unit would not fit
///   within the current line's horizontal layout bounds.
/// - Y overflow asks what to do when the next visual line would not fit within
///   the block's vertical layout bounds.
///
/// This makes wrapping just one possible X-axis overflow response, rather than
/// a separate boolean. Hard line breaks (`'\n'`) are always respected before
/// X-overflow handling is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextFlow {
    /// Inline-axis overflow policy.
    ///
    /// Applied independently to each hard-break source line and to each visual
    /// line produced by wrapping.
    pub overflow_x: OverflowX,

    /// Block-axis overflow policy.
    ///
    /// Applied when the next visual line would not fit wholly within the
    /// available height.
    pub overflow_y: OverflowY,

    /// How lines are positioned horizontally within the available width.
    ///
    /// Alignment only affects placement of lines that are admitted by the
    /// overflow policies. It does not change measurement, wrapping decisions, or
    /// truncation decisions. If an over-wide line has no room to align, the text
    /// system should clamp alignment so the line starts at the leading edge.
    pub line_align: TextLineAlign,
}

impl TextFlow {
    /// Single-line-ish label/input default.
    ///
    /// Hard `'\n'` still creates additional source lines, but no soft wrapping is
    /// performed. Horizontally overflowing clusters are dropped, and vertically
    /// overflowing lines are dropped.
    pub fn single_line() -> Self {
        Self {
            overflow_x: OverflowX::Drop,
            overflow_y: OverflowY::Drop,
            line_align: TextLineAlign::Start,
        }
    }

    /// Paragraph/caption default.
    ///
    /// Wraps at word boundaries first, falls back to cluster wrapping for over-long
    /// words, drops a cluster only if even a single cluster cannot fit on an empty
    /// line, and ellipsises vertical overflow.
    pub fn wrapped() -> Self {
        Self {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            line_align: TextLineAlign::Start,
        }
    }

    /// Renderer-clipped text viewport default.
    ///
    /// This policy may emit logical clusters/lines that exceed the layout bounds.
    /// It is intended for renderers that apply their own scissor/clipping and
    /// want edge text to be partially visible.
    pub fn clipped_viewport() -> Self {
        Self {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        }
    }
}

/// What to do when the next logical text unit would not fit within the current
/// line's horizontal layout bounds.
///
/// Policies are deliberately expressed as a fallback chain. This mirrors the
/// actual layout decision tree:
///
/// 1. Prefer a high-level behavior, such as word wrapping.
/// 2. If that cannot make progress, fall back to a lower-level behavior, such as
///    cluster wrapping.
/// 3. If even that cannot make progress, either drop the overflowing unit or keep
///    it and rely on downstream clipping.
///
/// The important contract is:
///
/// - `Drop`, successful wrapping, and successful ellipsis fitting keep the
///   reported logical line inside the X bounds.
/// - `Keep` may emit the first overflowing unit, then truncates the rest of that
///   line. The reported logical size may exceed the input constraint, and
///   visible ink may also spill outside the logical bounds. A renderer/scissor
///   may clip the visible pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowX {
    /// Prefer wrapping at word boundaries.
    ///
    /// A “word” is an implementation-defined unbreakable run, usually separated
    /// by whitespace or Unicode line-break opportunities. Hard `'\n'` is not
    /// handled here; it always breaks before this policy is considered.
    ///
    /// If the next word cannot fit on the current line, it is moved to a new
    /// line. If that word still cannot fit on an empty line, `fallback` decides
    /// what happens.
    ///
    /// Preserved whitespace participates in this hierarchy as single-cluster
    /// word-like units, using the same fallback sequence as any other word-like
    /// unit. The only whitespace-specific behavior is the soft-wrap boundary
    /// collapse described in the `TextSystem` contract below: when whitespace
    /// itself overflows after other clusters have already been admitted to the
    /// current visual line, or when whitespace is immediately before the unit
    /// that overflows, that boundary whitespace may be retained on the previous
    /// line with zero advance.
    WrapWord { fallback: WrapWordFallback },

    /// Wrap at cluster boundaries.
    ///
    /// If the next cluster does not fit logically on the current line, it
    /// is moved to a new line. If it still cannot fit on an empty line,
    /// `fallback` decides whether it is dropped or kept partially.
    ///
    /// A cluster is the smallest indivisible shaped text unit emitted by the
    /// text system. It should normally correspond to a shaping cluster, and it
    /// must not split combining marks, ligatures, or script-shaped units in a way
    /// that would corrupt shaping.
    WrapCluster { fallback: WrapClusterFallback },

    /// Replace the logically overflowing tail of the line with an ellipsis
    /// marker.
    ///
    /// The text system drops enough trailing units so the ellipsis itself fits
    /// logically within the X bounds. If even the ellipsis cannot fit,
    /// `fallback` decides what to do.
    Ellipsis { fallback: EllipsisFallback },

    /// Include the first unit that does not fit logically within the X bounds,
    /// then drop the remaining units on that line.
    ///
    /// This is the opt-in partial-cluster mode. It is useful when the renderer
    /// applies clipping and the caller wants edge text to be visibly sliced
    /// rather than removed entirely.
    Keep,

    /// Drop the first unit that does not fit logically within the X bounds, and
    /// drop the remaining units on that line.
    ///
    /// This is the strict fully-inside truncate behavior.
    Drop,
}

/// Fallback used by [`OverflowX::WrapWord`] when a word cannot fit on an empty
/// line. This fallback is evaluated only after the word has first been moved to
/// an empty line and still cannot fit there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapWordFallback {
    /// Try breaking the over-long word at cluster boundaries, see OverflowX::WrapCluster.
    WrapCluster { fallback: WrapClusterFallback },

    /// Keep the over-long word's first overflowing cluster, then truncate. See OverflowX::Keep.
    ///
    /// May emit geometry outside the X bounds.
    Keep,

    /// Keep the over-long word's clusters that fit within the X bounds, dropping
    /// the first overflowing cluster and the remaining clusters of the word.
    /// Note: this does *not* drop the whole word!
    /// See OverflowX::Drop.
    Drop,
}

/// Fallback used by cluster wrapping when even one cluster cannot fit on an
/// empty line. This fallback is evaluated only after the cluster has first been
/// moved to an empty line and still cannot fit there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WrapClusterFallback {
    /// Keep the first overflowing cluster, then truncate. See OverflowX::Keep.
    ///
    /// May emit geometry outside the X bounds.
    Keep,

    /// Drop the cluster. See OverflowX::Drop.
    Drop,
}

/// What to do when the next visual line would not fit within the block's
/// vertical layout bounds.
///
/// This policy operates on whole visual lines, not individual clusters. A visual
/// line may come from a hard break or from X-axis wrapping.
///
/// The same inside/outside contract applies:
///
/// - `Drop` and successful `Ellipsis` keep the reported logical block inside
///   the Y bounds.
/// - `Keep` may emit the first vertically overflowing line, then drops all later
///   lines. The reported logical size may exceed the input constraint. A
///   renderer/scissor may clip it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OverflowY {
    /// Indicate vertical truncation by ellipsising the previous visible line.
    ///
    /// The overflowing line itself is not emitted. Instead, the last line that
    /// fits wholly inside the Y bounds is modified using X-axis ellipsis fitting.
    ///
    /// If there is no previous visible line, or if the ellipsis cannot fit in the
    /// available X bounds, `fallback` decides what to do.
    Ellipsis { fallback: EllipsisFallback },

    /// Include the first line that does not fit within the Y bounds, then
    /// drop all later lines.
    ///
    /// This is useful for clipped text viewports where partially visible top or
    /// bottom lines should still render.
    Keep,

    /// Drop the first line that does not fit within the Y bounds, and drop
    /// all later lines.
    Drop,
}

/// Fallback used when an ellipsis marker cannot be fitted.
///
/// `Keep` is intentionally allowed even though ellipsis normally implies a
/// fully-inside marker. It is useful as a “show something rather than nothing”
/// policy for extremely small rectangles. Callers that require strict
/// fully-inside rendering should use `Drop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EllipsisFallback {
    /// Keep the first overflowing cluster or line, depending on whether the
    /// ellipsis failure happened in X or Y handling. See `OverflowX::Keep` and `OverflowY::Keep`.
    ///
    /// May emit geometry outside the bounds.
    Keep,

    /// Emit nothing for the overflowing unit. See `OverflowX::Drop` and `OverflowY::Drop`.
    Drop,
}

/// Horizontal positioning of each line within the available width.
///
/// `Start`/`End` are resolved against text direction; for left-to-right text
/// `Start` is left and `End` is right. Alignment only affects glyph X positions —
/// it never changes the measured block size or which glyphs are truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLineAlign {
    Start,
    Center,
    End,
}

/// Which measured text geometry a widget should align inside its content rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextContentBasis {
    /// Align the logical text block, based on shaped advances and line height.
    Logical,
    /// Align the visible ink bounds for optical/icon-like placement.
    Ink,
}

/// Horizontal or vertical placement policy for a prepared text block inside a containing box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContentPlacement {
    /// Span the layout's available extent in that direction, allowing line alignment to take effect.
    #[default]
    Fill,
    /// Shrink-wrap the content in that direction and align it.
    Align(Align),
}

/// Placement of a prepared text block inside a widget's own content rect.
///
/// This is widget-local content placement. It is distinct from
/// [`TextFlow::line_align`], which positions individual lines inside a text
/// layout block, and from layout [`Align`], which positions a whole widget
/// inside its parent layout space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextContentPlacement {
    pub x: ContentPlacement,
    pub y: ContentPlacement,
    pub basis: TextContentBasis,
}

impl TextContentPlacement {
    pub const TOP_LEFT: Self = Self::logical(
        ContentPlacement::Align(Align::Start),
        ContentPlacement::Align(Align::Start),
    );
    pub const CENTER: Self = Self::logical(
        ContentPlacement::Align(Align::Center),
        ContentPlacement::Align(Align::Center),
    );
    pub const INK_CENTER: Self = Self::ink(
        ContentPlacement::Align(Align::Center),
        ContentPlacement::Align(Align::Center),
    );

    pub const fn logical(x: ContentPlacement, y: ContentPlacement) -> Self {
        Self {
            x,
            y,
            basis: TextContentBasis::Logical,
        }
    }

    pub const fn ink(x: ContentPlacement, y: ContentPlacement) -> Self {
        Self {
            x,
            y,
            basis: TextContentBasis::Ink,
        }
    }

    /// Resolve the logical text block rect to pass to [`TextSystem::prepare`].
    pub fn resolve_rect(self, content_rect: Rect, metrics: TextMetrics) -> Rect {
        let logical = metrics.logical_size;
        let ink = metrics.ink_bounds;

        let (basis_x, basis_w) = match self.basis {
            TextContentBasis::Logical => (0.0, logical.x),
            TextContentBasis::Ink if ink.w > 0.0 => (ink.x, ink.w),
            TextContentBasis::Ink => (0.0, logical.x),
        };
        let (basis_y, basis_h) = match self.basis {
            TextContentBasis::Logical => (0.0, logical.y),
            TextContentBasis::Ink if ink.h > 0.0 => (ink.y, ink.h),
            TextContentBasis::Ink => (0.0, logical.y),
        };

        let (x, w) = match self.x {
            ContentPlacement::Fill => (content_rect.x, content_rect.w),
            ContentPlacement::Align(align) => {
                let x = content_rect.x + align_offset(content_rect.w, basis_w, align) - basis_x;
                let w = logical.x.min(content_rect.w);
                (x, w)
            }
        };

        let (y, h) = match self.y {
            ContentPlacement::Fill => (content_rect.y, content_rect.h),
            ContentPlacement::Align(align) => {
                let y = content_rect.y + align_offset(content_rect.h, basis_h, align) - basis_y;
                let h = logical.y.min(content_rect.h);
                (y, h)
            }
        };

        Rect::new(x, y, w, h)
    }
}

fn align_offset(available: f32, content: f32, align: Align) -> f32 {
    match align {
        Align::Start => 0.0,
        Align::Center => (available - content) * 0.5,
        Align::End => available - content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_content_placement_keeps_prepare_rect_inside_content_size() {
        let content_rect = Rect::new(10.0, 20.0, 4.0, 12.0);
        let overflow_metrics = TextMetrics {
            logical_size: Vec2::new(9.0, 18.0),
            ink_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
            line_count: 2,
            truncated_horizontal: true,
            truncated_vertical: true,
            lines: Vec::new(),
        };

        assert_eq!(
            TextContentPlacement::TOP_LEFT.resolve_rect(content_rect, overflow_metrics),
            Rect::new(10.0, 20.0, 4.0, 12.0)
        );
    }

    #[test]
    fn text_content_placement_fill_uses_full_content_size() {
        let content_rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let metrics = TextMetrics {
            logical_size: Vec2::new(40.0, 16.0),
            ink_bounds: Rect::new(0.0, 0.0, 40.0, 16.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };

        let placement = TextContentPlacement {
            x: ContentPlacement::Fill,
            y: ContentPlacement::Fill,
            basis: TextContentBasis::Logical,
        };

        assert_eq!(
            placement.resolve_rect(content_rect, metrics),
            Rect::new(10.0, 20.0, 100.0, 50.0)
        );
    }

    #[test]
    fn text_content_placement_fill_x_align_y() {
        let content_rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let metrics = TextMetrics {
            logical_size: Vec2::new(40.0, 16.0),
            ink_bounds: Rect::new(0.0, 0.0, 40.0, 16.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };

        let placement = TextContentPlacement {
            x: ContentPlacement::Fill,
            y: ContentPlacement::Align(Align::Center),
            basis: TextContentBasis::Logical,
        };

        assert_eq!(
            placement.resolve_rect(content_rect, metrics),
            Rect::new(10.0, 37.0, 100.0, 16.0)
        );
    }
}

// ── Measurement inputs & outputs ────────────────────────────────────────────

/// Logical layout constraints available to text, used by
/// [`TextSystem::measure`].
///
/// Each axis is `Some(px)` for a finite ceiling, or `None` for unbounded. This
/// is the reduction of the layout's `AxisBound`: both `Exact(w)` and `AtMost(w)`
/// become `Some(w)` (the distinction between a committed frame and a bare ceiling
/// does not matter for measurement — only the limit value does), while
/// `Unbounded` becomes `None`.
///
/// These are logical constraints, not pixel-containment guarantees. They drive
/// advances, wrapping, alignment, ellipsis, line admission, caret geometry, and
/// hit-testing. The visible ink may still protrude outside these bounds due to
/// glyph bearings, overhangs, accents, combining marks, symbol placement, or
/// custom font behavior.
///
/// Measurement is **symmetric**: text is reflowable, so its logical size is a
/// curve, not a point. Whichever axis is bounded constrains the flow; the
/// unbounded axis is the answer:
/// - `max_width: Some, max_height: None` → wrap to width, height grows down
///   (auto-height label in a column).
/// - `max_width: None, max_height: Some` → pack to a fixed height, width grows
///   sideways (fixed-height block that extends horizontally).
/// - both `Some` → wrap to width and clip/ellipsis to height (fixed box).
/// - both `None` → natural single line (plus any hard `'\n'` breaks).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextBounds {
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
}

impl TextBounds {
    pub const UNBOUNDED: Self = Self {
        max_width: None,
        max_height: None,
    };

    pub fn width(max_width: f32) -> Self {
        Self {
            max_width: Some(max_width),
            max_height: None,
        }
    }
}

/// Why a visual line ends where it does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LineEndKind {
    /// The source text ended normally on this visual line.
    EndOfText,
    /// The line ends with an explicit hard newline cluster.
    HardNewline,
    /// The line ends at collapsed soft-wrap boundary whitespace.
    SoftWrapWhitespace,
    /// The line ended because non-whitespace content wrapped to the next line.
    SoftWrapNonWhitespace,
    /// X overflow kept the first overflowing unit on this line.
    OverflowKeep,
    /// X overflow dropped content after this line.
    OverflowDrop,
    /// X overflow replaced omitted inline content with an ellipsis.
    EllipsisX,
    /// Y overflow replaced omitted later lines with an ellipsis on this line.
    EllipsisY,
}

/// The measured logical geometry of a single visual line of laid-out text, independent of where it is
/// drawn.
///
/// Hard trailing newlines create a following empty visual line with a zero-length
/// byte range at the end of the text. Preserved whitespace at the end of the
/// text does the same when it overflows and is collapsed at a soft-wrap
/// boundary: the collapsed whitespace remains part of the previous visual
/// line's byte range, while the following empty line exists for caret
/// positioning, hit-testing, selection, and editor feedback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineMetrics {
    /// Top Y offset of the line in block-local coordinates.
    pub y_top: f32,
    /// Height of the line.
    pub height: f32,
    /// Logical width of the line.
    pub logical_width: f32,
    /// Ink width of the line.
    pub ink_width: f32,
    /// X offset of the line's logical start in block-local coordinates.
    pub logical_x: f32,
    /// X offset of the line's ink start in block-local coordinates.
    pub ink_x: f32,
    /// Byte start index of the line in the original string.
    pub byte_start: usize,
    /// Byte end index of the line in the original string (exclusive).
    ///
    /// `byte_start..byte_end` is the source range represented by this visual
    /// line. A line's `byte_end` is not, by itself, a complete caret position:
    /// at soft-wrap boundaries, the previous line's end and the next line's
    /// start may share an insertion boundary while corresponding to different
    /// [`CaretPosition`] values.
    ///
    /// If the line ends with a hard newline (`\n`), this is the byte index
    /// immediately *after* the `\n` character, so the range `byte_start..byte_end`
    /// includes the newline.
    ///
    /// If the line ends with collapsed soft-wrap boundary whitespace, that
    /// whitespace remains included in this line's byte range even though it has
    /// zero visual advance and is excluded from `logical_width`. If that
    /// collapsed boundary whitespace is terminal, the following empty visual
    /// line has `byte_start == byte_end == text.len()`.
    pub byte_end: usize,
    /// The semantic reason this visual line ends here.
    pub end_kind: LineEndKind,
}

/// The measured logical geometry of a block of text, independent of where it is
/// drawn.
#[derive(Debug, Clone, PartialEq)]
pub struct TextMetrics {
    /// Logical size of the laid-out block in logical pixels.
    ///
    /// - `x` is the widest line's used advance width (shrink-wrapped — it is `≤`
    ///   `max_width` when a width bound was given and the overflow policy keeps
    ///   content inside, *not* the bound itself).
    /// - `y` is `visible_line_count × line_height`, where `line_height` is the
    ///   font's line spacing at this size.
    ///
    /// The size may exceed the input constraints when the selected overflow
    /// policy explicitly keeps overflowing content (`Keep` fallbacks or
    /// `OverflowY::Keep`). Separately, the actual ink may extend outside this
    /// logical size even when the logical size fits.
    ///
    /// This field is not a tight ink box. Use [`ink_bounds`](Self::ink_bounds)
    /// when a caller needs the bounds of the drawn pixels.
    ///
    /// "Visible" means after any vertical overflow has been applied: a block
    /// clipped to a height reports the size it actually occupies, not the size
    /// the full string would have needed.
    pub logical_size: Vec2,

    /// Tight visual bounds of the ink produced by the laid-out text, in the
    /// same block-local coordinate system as `prepare`, before the caller's draw
    /// rect translation is applied.
    ///
    /// This may be smaller than, larger than, or offset from
    /// [`logical_size`](Self::logical_size). It may also be empty, for example
    /// for whitespace-only text. Callers that need optical centering or visual
    /// alignment should use `ink_bounds`.
    pub ink_bounds: Rect,

    /// Number of lines actually laid out (after wrapping, hard breaks, and
    /// vertical overflow). Always `≥ 1`, even for empty input.
    pub line_count: u32,

    /// `true` if any line was cut on the inline axis — a text run was wider than
    /// the available width and got clipped/ellipsised. With `wrap: true` this is
    /// rare (over-long words force-break instead) but can still occur when the
    /// width is narrower than a single cluster.
    pub truncated_horizontal: bool,

    /// `true` if whole lines were dropped because the content exceeded the
    /// available height.
    pub truncated_vertical: bool,

    /// Metrics for each laid-out line.
    pub lines: Vec<LineMetrics>,
}

/// The geometry and handle for a piece of text prepared for drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout<G = TextHandle> {
    /// The opaque handle to give to the renderer via `DrawCmd::Text`.
    pub handle: TextHandle,
    /// The block's measured geometry, identical to what [`TextSystem::measure`]
    /// would return for the same text, flow policy, and the draw rect's logical
    /// size as bounds.
    pub metrics: TextMetrics,
    /// Owned line records for Framewise-owned text layouts.
    pub lines: Vec<TextLine>,
    /// Owned text clusters for Framewise-owned text layouts.
    pub clusters: Vec<TextCluster>,
    /// Owned layout glyphs for Framewise-owned text layouts.
    pub glyphs: Vec<LayoutGlyph<G>>,
}

/// One laid-out visual line in a Framewise-owned text layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLine {
    pub y_top: f32,
    pub height: f32,
    pub glyph_start: usize,
    pub glyph_end: usize,
    pub cluster_start: usize,
    pub cluster_end: usize,
    pub byte_start: usize,
    pub byte_end: usize,
    pub logical_width: f32,
    pub ink_width: f32,
    pub logical_x: f32,
    pub ink_x: f32,
    pub end_kind: LineEndKind,
}

/// One indivisible laid-out cluster in a Framewise-owned text layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TextCluster {
    pub byte_start: usize,
    pub byte_end: usize,
    pub glyph_start: usize,
    pub glyph_end: usize,
    pub x: f32,
    pub advance: f32,
    pub is_hard_break: bool,
    pub is_whitespace: bool,
    pub is_soft_wrap_boundary: bool,
}

/// One glyph after Framewise line layout, before caller draw origin is added.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutGlyph<G> {
    pub id: G,
    pub origin: Vec2,
    pub advance: f32,
    pub byte_start: usize,
}

struct SourceLine<G> {
    clusters: Vec<OwnedCluster<G>>,
    byte_start: usize,
    byte_end: usize,
    baseline_y: f32,
}

struct ProcessedLine<G> {
    clusters: Vec<OwnedCluster<G>>,
    byte_start: usize,
    byte_end: usize,
    baseline_y: f32,
    end_kind: LineEndKind,
}

#[derive(Debug, Clone)]
struct OwnedCluster<G> {
    byte_start: usize,
    byte_end: usize,
    x: f32,
    advance: f32,
    is_hard_break: bool,
    is_whitespace: bool,
    is_soft_wrap_boundary: bool,
    glyphs: Vec<LayoutGlyph<G>>,
}

impl<G> OwnedCluster<G> {
    fn end_x(&self) -> f32 {
        self.x + self.advance
    }

    fn shift_x(&mut self, dx: f32) {
        self.x += dx;
        for glyph in &mut self.glyphs {
            glyph.origin.x += dx;
        }
    }

    fn collapse_soft_wrap_boundary(&mut self) {
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

impl<G: Copy + Eq + Hash> TextLayout<G> {
    fn from_backend<B: TextBackend<ShapedGlyphId = G>>(
        backend: &mut B,
        text: &str,
        style: TextStyle,
        bounds: TextBounds,
    ) -> Self {
        let flow = style.flow;
        let line_height = backend.line_height(style).round().max(1.0);
        let baseline_offset = style.size.round();
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

    pub fn caret_geom(&self, position: CaretPosition) -> CaretGeom {
        let Some((cluster_idx, cluster)) = self.find_caret_cluster(position) else {
            let line = self
                .lines
                .first()
                .expect("a text layout always has at least one line");
            return CaretGeom {
                x: line.logical_x,
                y_top: line.y_top,
                height: line.height,
            };
        };

        let line_idx = self.line_index_for_cluster(cluster_idx).unwrap_or(0);
        let line = &self.lines[line_idx];
        let (x, y_top, height) = match position {
            CaretPosition::BeforeCluster { .. } => (cluster.x, line.y_top, line.height),
            CaretPosition::AfterCluster { .. }
                if cluster.is_hard_break || cluster.is_soft_wrap_boundary =>
            {
                let next_line = self.lines.get(line_idx + 1).unwrap_or(line);
                let next_clusters = &self.clusters[next_line.cluster_start..next_line.cluster_end];
                let next_x = next_clusters
                    .first()
                    .map(|cluster| cluster.x)
                    .unwrap_or(next_line.logical_x);
                (next_x, next_line.y_top, next_line.height)
            }
            CaretPosition::AfterCluster { .. } => {
                (cluster.x + cluster.advance, line.y_top, line.height)
            }
            CaretPosition::EmptyText => unreachable!("handled by missing-cluster branch"),
        };

        CaretGeom { x, y_top, height }
    }

    pub fn hit_test_caret(&self, pos: Vec2) -> CaretPosition {
        let line_idx = self
            .lines
            .iter()
            .position(|line| pos.y < line.y_top + line.height)
            .unwrap_or_else(|| self.lines.len().saturating_sub(1));
        let line = &self.lines[line_idx];
        let clusters = &self.clusters[line.cluster_start..line.cluster_end];
        if clusters.is_empty() {
            return self.empty_line_caret_position(line_idx);
        }
        for cluster in clusters {
            let mid = cluster.x + cluster.advance * 0.5;
            if pos.x < mid {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }
        }
        match clusters.last() {
            Some(last) if last.is_hard_break || last.is_soft_wrap_boundary => {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: last.byte_start,
                }
            }
            Some(last) => CaretPosition::AfterCluster {
                cluster_byte_index: last.byte_start,
            },
            None => self.empty_line_caret_position(line_idx),
        }
    }

    pub fn caret_insertion_byte(&self, position: CaretPosition) -> usize {
        match self.find_caret_cluster(position) {
            Some((_, cluster)) => match position {
                CaretPosition::BeforeCluster { .. } => cluster.byte_start,
                CaretPosition::AfterCluster { .. } => cluster.byte_end,
                CaretPosition::EmptyText => 0,
            },
            None => 0,
        }
    }

    pub fn caret_position_at_insertion_byte(&self, byte_index: usize) -> CaretPosition {
        if self.clusters.is_empty() {
            return CaretPosition::EmptyText;
        }

        for (idx, cluster) in self.clusters.iter().enumerate() {
            if byte_index <= cluster.byte_start || byte_index < cluster.byte_end {
                return CaretPosition::BeforeCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }

            if byte_index == cluster.byte_end {
                if let Some(next) = self.clusters.get(idx + 1) {
                    if next.byte_start == byte_index {
                        return CaretPosition::BeforeCluster {
                            cluster_byte_index: next.byte_start,
                        };
                    }
                }
                return CaretPosition::AfterCluster {
                    cluster_byte_index: cluster.byte_start,
                };
            }
        }

        let last = self.clusters.last().expect("clusters is non-empty");
        CaretPosition::AfterCluster {
            cluster_byte_index: last.byte_start,
        }
    }

    pub fn previous_caret_position(&self, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(position);
        let Some(target_byte) = self.previous_insertion_boundary(byte_index) else {
            return self.caret_position_at_insertion_byte(0);
        };
        self.caret_position_for_movement_target(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

    pub fn next_caret_position(&self, position: CaretPosition) -> CaretPosition {
        let byte_index = self.caret_insertion_byte(position);
        let Some(target_byte) = self.next_insertion_boundary(byte_index) else {
            return self
                .clusters
                .last()
                .map(|cluster| CaretPosition::AfterCluster {
                    cluster_byte_index: cluster.byte_start,
                })
                .unwrap_or(CaretPosition::EmptyText);
        };
        self.caret_position_for_movement_target(target_byte)
            .unwrap_or_else(|| self.caret_position_at_insertion_byte(target_byte))
    }

    pub fn emit_glyphs<B>(
        &self,
        commands: &mut DrawCommands,
        backend: &mut B,
        origin: Vec2,
        style: TextStyle,
        color: Color,
        z: u32,
    ) where
        B: TextBackend<ShapedGlyphId = G>,
    {
        let glyphs = self.glyphs.iter().filter_map(|glyph| {
            backend.prepare_glyph(PrepareGlyphRequest {
                glyph: glyph.id,
                style,
                glyph_origin: Vec2::new(origin.x + glyph.origin.x, origin.y + glyph.origin.y),
            })
        });
        commands.push_glyph_run(glyphs, color, z);
    }

    fn find_caret_cluster(&self, position: CaretPosition) -> Option<(usize, &TextCluster)> {
        let cluster_byte_index = match position {
            CaretPosition::BeforeCluster { cluster_byte_index }
            | CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index,
            CaretPosition::EmptyText => return None,
        };

        self.clusters
            .iter()
            .enumerate()
            .find(|(_, cluster)| cluster.byte_start == cluster_byte_index)
            .or_else(|| {
                self.clusters.iter().enumerate().find(|(_, cluster)| {
                    cluster_byte_index <= cluster.byte_start
                        || cluster_byte_index < cluster.byte_end
                })
            })
            .or_else(|| self.clusters.iter().enumerate().next_back())
    }

    fn line_index_for_cluster(&self, cluster_idx: usize) -> Option<usize> {
        self.lines
            .iter()
            .position(|line| cluster_idx >= line.cluster_start && cluster_idx < line.cluster_end)
    }

    fn empty_line_caret_position(&self, line_idx: usize) -> CaretPosition {
        if self.clusters.is_empty() {
            return CaretPosition::EmptyText;
        }

        self.lines
            .get(..line_idx)
            .and_then(|lines| {
                lines
                    .iter()
                    .rev()
                    .find(|line| line.cluster_end > line.cluster_start)
            })
            .and_then(|line| self.clusters.get(line.cluster_end - 1))
            .map(|cluster| CaretPosition::AfterCluster {
                cluster_byte_index: cluster.byte_start,
            })
            .unwrap_or(CaretPosition::EmptyText)
    }

    fn previous_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.clusters
            .iter()
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte < byte_index)
            .max()
    }

    fn next_insertion_boundary(&self, byte_index: usize) -> Option<usize> {
        self.clusters
            .iter()
            .flat_map(|cluster| [cluster.byte_start, cluster.byte_end])
            .filter(|byte| *byte > byte_index)
            .min()
    }

    fn caret_position_for_movement_target(&self, target_byte: usize) -> Option<CaretPosition> {
        if let Some(cluster) = self.clusters.iter().find(|cluster| {
            (cluster.is_hard_break || cluster.is_soft_wrap_boundary)
                && cluster.byte_end == target_byte
        }) {
            return Some(CaretPosition::AfterCluster {
                cluster_byte_index: cluster.byte_start,
            });
        }

        if let Some(cluster) = self.clusters.iter().find(|cluster| {
            (cluster.is_hard_break || cluster.is_soft_wrap_boundary)
                && cluster.byte_start == target_byte
        }) {
            return Some(CaretPosition::BeforeCluster {
                cluster_byte_index: cluster.byte_start,
            });
        }

        None
    }
}

#[allow(clippy::too_many_arguments)]
fn make_source_line<B: TextBackend>(
    backend: &mut B,
    text: &str,
    style: TextStyle,
    segment_start: usize,
    segment_end: usize,
    has_newline: bool,
    line_idx: usize,
    line_height: f32,
    baseline_offset: f32,
) -> SourceLine<B::ShapedGlyphId> {
    let segment = &text[segment_start..segment_end];
    let baseline_y = line_idx as f32 * line_height + baseline_offset;
    let mut clusters = Vec::new();

    if !segment.is_empty() {
        let shaped = backend.shape_text(segment, style);
        for shaped_cluster in shaped.clusters {
            let byte_start = segment_start + shaped_cluster.byte_start;
            let byte_end = segment_start + shaped_cluster.byte_end;
            let x = clusters.last().map(OwnedCluster::end_x).unwrap_or(0.0);
            let glyphs = shaped_cluster
                .glyphs
                .into_iter()
                .map(|glyph| LayoutGlyph {
                    id: glyph.id,
                    origin: Vec2::new(x + glyph.x, baseline_y + glyph.y),
                    advance: glyph.advance,
                    byte_start,
                })
                .collect();
            clusters.push(OwnedCluster {
                byte_start,
                byte_end,
                x,
                advance: shaped_cluster.advance,
                is_hard_break: false,
                is_whitespace: shaped_cluster.is_whitespace,
                is_soft_wrap_boundary: false,
                glyphs,
            });
        }
    }

    if has_newline {
        let x = clusters.last().map(OwnedCluster::end_x).unwrap_or(0.0);
        clusters.push(OwnedCluster {
            byte_start: segment_end,
            byte_end: segment_end + 1,
            x,
            advance: 0.0,
            is_hard_break: true,
            is_whitespace: true,
            is_soft_wrap_boundary: false,
            glyphs: Vec::new(),
        });
    }

    SourceLine {
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

fn logical_cluster_line_width<G>(clusters: &[OwnedCluster<G>]) -> f32 {
    let start = logical_cluster_line_start(clusters);
    clusters
        .iter()
        .map(OwnedCluster::end_x)
        .fold(start, f32::max)
        - start
}

fn logical_cluster_line_start<G>(clusters: &[OwnedCluster<G>]) -> f32 {
    clusters
        .iter()
        .map(|cluster| cluster.x)
        .reduce(f32::min)
        .unwrap_or(0.0)
}

fn append_empty_after_terminal_soft_wrap_boundary<G>(
    lines: &mut Vec<Vec<OwnedCluster<G>>>,
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

fn collapse_trailing_soft_wrap_space<G>(clusters: &mut [OwnedCluster<G>]) {
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

fn wrap_clusters<G: Clone>(
    clusters: Vec<OwnedCluster<G>>,
    w: f32,
    fallback: WrapClusterFallback,
) -> Vec<Vec<OwnedCluster<G>>> {
    let mut lines: Vec<Vec<OwnedCluster<G>>> = Vec::new();
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
                    if last_line.last().map(|c: &OwnedCluster<G>| c.is_hard_break) != Some(true) {
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

fn wrap_clusters_at_words<G: Clone>(
    clusters: Vec<OwnedCluster<G>>,
    w: f32,
    fallback: WrapWordFallback,
) -> Vec<Vec<OwnedCluster<G>>> {
    if clusters.is_empty() {
        return vec![Vec::new()];
    }

    struct Seg<G> {
        is_space: bool,
        clusters: Vec<OwnedCluster<G>>,
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
                                .map(OwnedCluster::end_x)
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

fn apply_ellipsis_x<B: TextBackend>(
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

/// A visual caret anchor in prepared text.
///
/// This is deliberately richer than an insertion byte index. At hard line
/// breaks and soft-wrap boundaries, two visually distinct caret positions can
/// map to the same source insertion boundary: the trailing edge of the previous
/// visual line and the leading edge of the following visual line. A byte-only
/// API cannot preserve that distinction during hit-testing, caret movement, or
/// editor feedback.
///
/// `cluster_byte_index` identifies the cluster being anchored to. It is not
/// necessarily the same thing as the insertion byte index:
///
/// - [`BeforeCluster`](Self::BeforeCluster) inserts at the anchored cluster's
///   `byte_start`.
/// - [`AfterCluster`](Self::AfterCluster) inserts at the anchored cluster's
///   `byte_end`.
///
/// Use [`TextSystem::caret_insertion_byte`] to convert a prepared visual caret
/// position into an insertion byte index for editing operations. Use
/// [`TextSystem::caret_position_at_insertion_byte`] to choose a canonical visual
/// anchor for a programmatic byte position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretPosition {
    /// The caret is anchored to the leading visual edge of the cluster that
    /// starts at `cluster_byte_index`.
    BeforeCluster { cluster_byte_index: usize },
    /// The caret is anchored to the trailing visual edge of the cluster that
    /// starts at `cluster_byte_index`.
    AfterCluster { cluster_byte_index: usize },
    /// The prepared text contains no clusters. The caret sits at the start of
    /// the single empty visual line.
    EmptyText,
}

/// The geometry of a visual caret anchor, in block-local coordinates (origin at
/// the block's top-left, y increasing downward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretGeom {
    /// X offset of the caret.
    pub x: f32,
    /// Y offset of the top of the line the caret sits on.
    pub y_top: f32,
    /// Height of that line (the caret's drawn height).
    pub height: f32,
}

// ── The trait ───────────────────────────────────────────────────────────────

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
