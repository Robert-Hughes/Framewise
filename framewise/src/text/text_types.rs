use super::ShapedText;
use crate::types::{Rect, Vec2};
use std::rc::Rc;

/// A lightweight application-owned font handle.
///
/// Framewise never loads or owns font files. It only passes this handle to the
/// application's text backend, which decides how the handle maps to real font data.
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
    /// `WrapWord` treats each maximal run of non-whitespace, non-hard-break
    /// clusters as one word segment. Each whitespace cluster is an independent
    /// breakable segment. Unicode line-break opportunities other than
    /// whitespace are not currently recognised. Hard `'\n'` is handled before
    /// this policy is considered.
    ///
    /// If the next word cannot fit on the current line, it is moved to a new
    /// line. If that word still cannot fit on an empty line, `fallback` decides
    /// what happens.
    ///
    /// Preserved whitespace participates in this hierarchy as single-cluster
    /// word-like units, using the same fallback sequence as any other word-like
    /// unit. The only whitespace-specific behavior is the soft-wrap boundary
    /// collapse described in the Framewise text layout contract: when whitespace
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
///
/// When [`TextBounds::max_width`] is provided, lines align inside that width.
/// When no maximum width is provided, lines align inside the maximum logical
/// width of the laid-out lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextLineAlign {
    Start,
    Center,
    End,
}

// ── Measurement inputs & outputs ────────────────────────────────────────────

/// Logical layout constraints available to text, used by
/// [`measure_text`](crate::text::measure_text).
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
/// A caller that needs strict pixel containment should clip, add padding, or use
/// a future ink-fitting policy rather than assuming that these input bounds
/// contain all rendered pixels.
///
/// `max_width` controls inline reflow, wrapping, X overflow, and line
/// alignment. `max_height` never causes reflow; it only controls how many
/// already-produced visual lines are admitted on the block axis.
///
/// - `max_width: Some, max_height: None` -> wrap/overflow to width; height
///   grows down.
/// - `max_width: None, max_height: Some` -> no soft wrapping from height alone;
///   hard-break/natural visual lines are produced, then Y overflow is applied.
/// - both `Some` -> wrap/overflow to width, then apply Y overflow to the
///   resulting visual lines.
/// - both `None` -> natural width, plus any hard `'\n'` breaks.
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
    pub approx_ink_width: f32,
    /// X offset of the line's logical start in block-local coordinates.
    pub logical_x: f32,
    /// X offset of the line's ink start in block-local coordinates.
    pub approx_ink_x: f32,
    /// Byte start index of the line in the original string.
    pub byte_start: usize,
    /// Byte end index of the line in the original string (exclusive).
    ///
    /// `byte_start..byte_end` is the source range represented by this visual
    /// line. A line's `byte_end` is not, by itself, a complete caret position.
    /// At mid-word soft wraps, the previous line's trailing edge and the next
    /// line's leading edge may share one insertion byte but need distinct
    /// [`CaretPosition`] values to preserve visual affinity. At hard newlines
    /// and collapsed soft-wrap whitespace, the visual sides are source-distinct
    /// positions before and after the boundary cluster.
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
    /// - `x` is the widest admitted line’s logical advance width, rounded up to
    ///   a whole logical pixel. This is conservative for widget sizing.
    /// - `y` is `visible_line_count × line_height`, where `line_height` is the
    ///   font's line spacing at this size.
    ///
    /// The size may exceed the input constraints when the selected overflow
    /// policy explicitly keeps overflowing content (`Keep` fallbacks or
    /// `OverflowY::Keep`). Separately, the actual ink may extend outside this
    /// logical size even when the logical size fits.
    ///
    /// This field is stable and suitable for widget sizing. It is not a tight
    /// ink box.
    ///
    /// "Visible" means after any vertical overflow has been applied: a block
    /// clipped to a height reports the size it actually occupies, not the size
    /// the full string would have needed.
    pub logical_size: Vec2,

    /// Approximate raster-independent ink bounds in layout coordinates.
    ///
    /// These are computed during shaping/layout from backend-provided glyph
    /// outline/control bounds where available, or from a conservative fallback.
    /// They are suitable for optical placement such as
    /// [`TextContentPlacement::INK_CENTER`](crate::text::TextContentPlacement::INK_CENTER)
    /// and for diagnostics.
    ///
    /// They are not exact final drawn pixel bounds. Final raster ink may depend
    /// on draw origin, subpixel bin, hinting, rasterisation mode, glyph bearings,
    /// atlas/resource dimensions, and backend-specific preparation.
    ///
    /// Exact drawn bounds can only be derived from emitted `DrawGlyph`s plus the
    /// resolved image sizes for their `PreparedGlyphToken`s. `measure_text`
    /// does not promise exact pixel ink bounds. Callers that require strict
    /// pixel containment should clip, add padding, or use a future ink-fitting
    /// policy rather than assuming that input bounds contain all rendered
    /// pixels.
    pub approx_ink_bounds: Rect,

    /// Number of lines actually laid out (after wrapping, hard breaks, and
    /// vertical overflow). Always `≥ 1`, even for empty input.
    pub line_count: u32,

    /// `true` if any line was cut on the inline axis: content was dropped, kept
    /// only up to the first overflowing unit, or replaced with an ellipsis by
    /// the selected X-overflow policy or fallback.
    ///
    /// Pure successful wrapping without dropped/ellipsised inline content does
    /// not set this flag.
    pub truncated_horizontal: bool,

    /// `true` if whole lines were dropped because the content exceeded the
    /// available height.
    pub truncated_vertical: bool,

    /// Metrics for each laid-out line.
    pub lines: Vec<LineMetrics>,
}

/// The geometry for a laid-out piece of text.
///
/// Framewise text layout uses two conversion boundaries:
///
/// 1. Backend shaping output -> Framewise working layout representation.
///    The backend returns cached immutable [`ShapedText`]. Framewise converts
///    shaped runs into its own working lines/clusters exactly once. These
///    working clusters store source byte ranges, logical x/advance, visibility,
///    wrapping state, and source references into shaped runs.
/// 2. Framewise working layout representation -> draw commands. When emitting
///    text, Framewise resolves final glyph origins from line baseline, cluster
///    x, and shaped glyph offsets, then asks the backend to prepare glyphs and
///    appends `DrawGlyph`s into `DrawCommands`.
///
/// Between those boundaries, layout mutates or moves the same working cluster
/// objects and derives metrics/caret/hit-test results. It should not copy
/// clusters into another intermediate representation.
///
/// `Working*` types are Framewise-owned layout-space records. They are not
/// backend shaping output, and they are not public API records. "Working" does
/// not mean "discarded before `TextLayout`"; some working records are stored
/// privately inside `TextLayout` after finalisation.
///
/// `TextLayout` keeps final working line and cluster records as an overlay over
/// shared immutable shaped runs. It does not store a flat glyph vector; final
/// glyph positions are resolved from line baselines, cluster positions, and
/// shaped glyph offsets only for approximate ink bounds, glyph emission, or
/// explicit materialisation through [`TextLayout::resolved_glyphs`].
///
/// All positions are in block-local coordinates: the origin is the text block's
/// top-left corner, with y increasing downward. Callers translate the layout to
/// screen space by passing a draw origin to [`TextLayout::emit_glyphs`].
///
/// Empty input is a valid layout, not an error case. It produces one visible
/// empty line with normal line metrics, zero advance, empty ink bounds, and a
/// positive height so editors can size and draw a non-zero-height caret.
///
/// For empty text:
/// - `metrics.line_count == 1` and `lines.len() == 1`.
/// - `metrics.logical_size.x == 0.0`.
/// - `metrics.logical_size.y` is one line height.
/// - `metrics.approx_ink_bounds` is empty because no glyph ink is emitted.
/// - The single line has `byte_start == byte_end == 0`,
///   `end_kind == LineEndKind::EndOfText`, and a positive `height`.
///
/// Every hard newline starts a new visual line. A trailing hard newline creates
/// a following empty visual line. Preserved whitespace at the end of the text
/// does the same when it overflows and is collapsed at a soft-wrap boundary:
/// the boundary character remains in the previous line's byte range while the
/// caret position after it is on the following empty line.
///
/// At a soft-wrap boundary, exactly one preserved whitespace character may be
/// collapsed to zero advance. Leading indentation remains visible unless one of
/// those whitespace characters independently becomes a later boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum WorkingClusterSource {
    Shaped {
        run_index: usize,
        cluster_index: usize,
    },
    Empty,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextLayout<G> {
    /// The block's measured logical geometry.
    pub(crate) metrics: TextMetrics,
    /// Final Framewise-owned visual line records in block-local coordinates.
    pub(crate) lines: Vec<WorkingProcessedLine>,
    pub(crate) runs: Vec<WorkingRun<G>>,
    pub(crate) visible_glyph_count: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkingRun<G> {
    pub(crate) shaped: Rc<ShapedText<G>>,
    pub(crate) segment_start: usize,
}

/// Temporary hard-line grouping before wrapping.
pub(crate) struct WorkingSourceLine {
    pub(crate) clusters: Vec<WorkingCluster>,
    pub(crate) byte_start: usize,
    pub(crate) byte_end: usize,
    pub(crate) logical_start: f32,
    pub(crate) logical_width: f32,
}

/// Framewise-owned visual line after wrapping/truncation/finalisation.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkingProcessedLine {
    pub y_top: f32,
    pub baseline_y: f32,
    pub height: f32,
    pub byte_start: usize,
    pub byte_end: usize,
    pub logical_width: f32,
    pub approx_ink_width: f32,
    pub logical_x: f32,
    pub approx_ink_x: f32,
    pub(crate) logical_geometry_valid: bool,
    pub end_kind: LineEndKind,
    pub clusters: Vec<WorkingCluster>,
}

impl WorkingProcessedLine {
    pub(crate) fn pending(
        clusters: Vec<WorkingCluster>,
        byte_start: usize,
        byte_end: usize,
        end_kind: LineEndKind,
    ) -> Self {
        Self {
            y_top: 0.0,
            baseline_y: 0.0,
            height: 0.0,
            byte_start,
            byte_end,
            logical_width: 0.0,
            approx_ink_width: 0.0,
            logical_x: 0.0,
            approx_ink_x: 0.0,
            logical_geometry_valid: false,
            end_kind,
            clusters,
        }
    }

    pub(crate) fn pending_with_geometry(
        clusters: Vec<WorkingCluster>,
        byte_start: usize,
        byte_end: usize,
        end_kind: LineEndKind,
        logical_x: f32,
        logical_width: f32,
    ) -> Self {
        Self {
            y_top: 0.0,
            baseline_y: 0.0,
            height: 0.0,
            byte_start,
            byte_end,
            logical_width,
            approx_ink_width: 0.0,
            logical_x,
            approx_ink_x: 0.0,
            logical_geometry_valid: true,
            end_kind,
            clusters,
        }
    }

    pub(crate) fn invalidate_logical_geometry(&mut self) {
        self.logical_x = 0.0;
        self.logical_width = 0.0;
        self.logical_geometry_valid = false;
    }
}

/// Framewise-owned mutable layout cluster overlay over `ShapedText`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct WorkingCluster {
    pub source: WorkingClusterSource,
    pub byte_start: usize,
    pub byte_end: usize,
    pub x: f32,
    pub advance: f32,
    pub is_hard_break: bool,
    pub is_whitespace: bool,
    pub is_soft_wrap_boundary: bool,
    pub glyphs_visible: bool,
}

impl WorkingCluster {
    pub(crate) fn end_x(&self) -> f32 {
        self.x + self.advance
    }

    pub(crate) fn shift_x(&mut self, dx: f32) {
        self.x += dx;
    }

    pub(crate) fn collapse_soft_wrap_boundary(&mut self) {
        self.advance = 0.0;
        self.is_soft_wrap_boundary = true;
        self.glyphs_visible = false;
    }
}

/// One glyph after Framewise line layout, before caller draw origin is added.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutGlyph<G> {
    /// Opaque backend glyph token produced during shaping.
    pub id: G,
    /// Final layout-space glyph origin before caller draw origin is added.
    pub origin: Vec2,
    /// Shaped advance used by text flow.
    pub advance: f32,
    /// Source byte index of the cluster that produced this glyph.
    pub byte_start: usize,
    /// Approximate raster-independent ink bounds relative to this glyph origin.
    pub approx_ink_bounds: Rect,
}

/// A visual caret anchor in prepared text with a cheap insertion-byte hint.
///
/// This is deliberately richer than an insertion byte index. Some visual line
/// boundaries need more information than a plain insertion byte can carry.
///
/// For a soft wrap inserted between clusters with no source boundary character,
/// the trailing edge of the previous visual line and the leading edge of the
/// following visual line can map to the same source insertion byte.
/// `CaretPosition` preserves that visual affinity.
///
/// For hard newlines and visually collapsed whitespace at soft-wrap
/// boundaries, the two visual sides are source-distinct positions around a real
/// cluster: before/after the newline or before/after the collapsed whitespace.
/// These are not the same insertion boundary, but they still require explicit
/// before/after cluster representation so hit-testing, caret geometry, and
/// editor feedback can choose the intended visual side.
///
/// `cluster_byte_start` identifies the cluster being visually anchored to. For
/// [`BeforeCluster`](Self::BeforeCluster), it is also the insertion byte. For
/// [`AfterCluster`](Self::AfterCluster), `cluster_byte_end` is stored so callers
/// can recover the insertion byte immediately after the anchored cluster
/// without consulting layout.
///
/// Use [`CaretPosition::insertion_byte_hint`] to get the insertion byte for
/// editing operations. Use
/// [`TextLayout::caret_position_at_insertion_byte`] to choose a canonical visual
/// anchor for a programmatic byte position. Layout-dependent operations such as
/// caret geometry, hit testing, and visual movement remain on `TextLayout`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaretPosition {
    /// The caret is anchored to the leading visual edge of the cluster that
    /// starts at `cluster_byte_start`.
    ///
    /// `cluster_byte_start` identifies the anchored cluster's start byte. It is
    /// both the visual anchor and the insertion byte.
    BeforeCluster { cluster_byte_start: usize },
    /// The caret is anchored to the trailing visual edge of the cluster that
    /// starts at `cluster_byte_start`.
    ///
    /// `cluster_byte_start` identifies the anchored cluster's start byte and is
    /// the visual anchor. `cluster_byte_end` is the insertion byte immediately
    /// after the anchored cluster.
    AfterCluster {
        cluster_byte_start: usize,
        cluster_byte_end: usize,
    },
    /// The prepared text contains no clusters. The caret sits at the start of
    /// the single empty visual line, with insertion byte `0`.
    EmptyText,
}

impl CaretPosition {
    /// Return the insertion byte carried by this visual caret anchor.
    pub fn insertion_byte_hint(self) -> usize {
        match self {
            CaretPosition::BeforeCluster { cluster_byte_start } => cluster_byte_start,
            CaretPosition::AfterCluster {
                cluster_byte_end, ..
            } => cluster_byte_end,
            CaretPosition::EmptyText => 0,
        }
    }
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
