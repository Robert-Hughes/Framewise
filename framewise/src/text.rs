use crate::{
    layout::Align,
    types::{Rect, Vec2},
};

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
    /// word-like units. A whitespace cluster may fit on the current line, move
    /// to the next line, or, if it cannot fit on an empty line, follow
    /// `fallback`. When whitespace itself is the unit that overflows a
    /// non-empty line, the `TextSystem` whitespace wrapping contract below
    /// applies.
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
/// line.
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
/// empty line.
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

/// Placement of a prepared text block inside a widget's own content rect.
///
/// This is widget-local content placement. It is distinct from
/// [`TextFlow::line_align`], which positions individual lines inside a text
/// layout block, and from layout [`Align`], which positions a whole widget
/// inside its parent layout space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextContentPlacement {
    pub x: Align,
    pub y: Align,
    pub basis: TextContentBasis,
}

impl TextContentPlacement {
    pub const TOP_LEFT: Self = Self::logical(Align::Start, Align::Start);
    pub const CENTER: Self = Self::logical(Align::Center, Align::Center);
    pub const INK_CENTER: Self = Self::ink(Align::Center, Align::Center);

    pub const fn logical(x: Align, y: Align) -> Self {
        Self {
            x,
            y,
            basis: TextContentBasis::Logical,
        }
    }

    pub const fn ink(x: Align, y: Align) -> Self {
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

        let x = content_rect.x + align_offset(content_rect.w, basis_w, self.x) - basis_x;
        let y = content_rect.y + align_offset(content_rect.h, basis_h, self.y) - basis_y;
        Rect::new(x, y, logical.x, logical.y)
    }
}

fn align_offset(available: f32, content: f32, align: Align) -> f32 {
    match align {
        Align::Start => 0.0,
        Align::Center => (available - content) * 0.5,
        Align::End => available - content,
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

/// The measured logical geometry of a single visual line of laid-out text, independent of where it is
/// drawn.
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
    /// Byte start index of the line in the original string.
    pub byte_start: usize,
    /// Byte end index of the line in the original string (exclusive).
    ///
    /// If the line ends with a hard newline (`\n`), this is the byte index
    /// immediately *after* the `\n` character, so the range `byte_start..byte_end`
    /// includes the newline.
    pub byte_end: usize,
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
pub struct TextLayout {
    /// The opaque handle to give to the renderer via `DrawCmd::Text`.
    pub handle: TextHandle,
    /// The block's measured geometry, identical to what [`TextSystem::measure`]
    /// would return for the same text, flow policy, and the draw rect's logical
    /// size as bounds.
    pub metrics: TextMetrics,
}

/// The geometry of a text caret at a given byte position, in block-local
/// coordinates (origin at the block's top-left, y increasing downward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretGeom {
    /// X offset of the caret (the leading edge of the cluster at the queried byte,
    /// or the trailing edge of the last cluster when the byte is at end-of-text).
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
///
/// ### Whitespace Wrapping
///
/// Preserved whitespace characters are individually wrap-capable. Under word
/// wrapping, each whitespace cluster is treated as a single-cluster word-like
/// unit for overflow and fallback purposes: if it cannot fit on an empty line,
/// the selected `WrapWordFallback`/`WrapClusterFallback` policy applies.
///
/// When such a whitespace character is the overflowing unit that causes a soft
/// wrap from a non-empty line, that one character is assigned to the end of the
/// previous visual line with zero visual advance. It remains part of the line's
/// byte range and caret/selection model, like a hard newline, but is excluded
/// from `logical_width`. Adjacent whitespace remains preserved and participates
/// in wrapping normally.
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
    /// The handle is valid until the next frame reset (see [`TextHandle`]).
    fn prepare(&mut self, text: &str, style: TextStyle, rect: Rect) -> TextLayout;

    /// Caret geometry for the character boundary at `byte_index`, in block-local
    /// coordinates. See [`CaretGeom`].
    ///
    /// Caret positions are in the same logical block coordinate system used by
    /// `prepare`. They should follow shaped advances and line metrics, not the
    /// tight ink box of the surrounding text.
    ///
    /// `byte_index` must fall on a UTF-8 char boundary of the prepared string.
    /// An index at or past the end returns the caret after the final cluster. If
    /// the cluster at that index was dropped by overflow, the caret clamps to the
    /// nearest laid-out boundary.
    ///
    /// For lines that end with a hard newline (`\n`):
    /// - Querying the index of the `\n` character itself (i.e. `byte_end - 1` for
    ///   that line) places the caret at the trailing edge of the last visible
    ///   character on that line.
    /// - Querying the index immediately after the `\n` (i.e. `byte_end`, which
    ///   equals the `byte_start` of the next line) places the caret at the start
    ///   of the next line.
    ///
    /// For soft-wrapped lines (where the visual line ends at a visual boundary rather
    /// than a hard newline `\n` character):
    /// - Querying `caret_geom` at the visual line's `byte_end` (which equals the `byte_start`
    ///   of the next visual line) resolves to the start of the next line (returning `x` aligned
    ///   to the start of the next line).
    /// - To obtain the trailing edge coordinate of the soft-wrapped line, widgets should
    ///   inspect the line's logical width (`line.logical_width`) instead of querying `caret_geom(line.byte_end)`.
    fn caret_geom(&self, handle: TextHandle, byte_index: usize) -> CaretGeom;

    /// Hit-test a point (block-local coordinates) to the nearest character
    /// boundary, returning a byte index into the prepared string.
    ///
    /// The coordinates `pos` are in the logical block coordinate system used by
    /// `prepare`. Hit testing should compare against the shaped logical cluster
    /// positions in the cached run.
    ///
    /// The point is resolved to a line by `y` first, then to the nearest gap
    /// between clusters by `x`:
    /// - Points above the block clamp to the first line; points below clamp to
    ///   the last line.
    /// - Points to the left of a line clamp to that line's `byte_start`.
    /// - Points to the right of a line clamp to the end of the *visible* content
    ///   on that line.  If the line ends with a hard newline (`\n`), the result
    ///   must be the index of the `\n` character itself (i.e. `byte_end - 1` for
    ///   that line), **not** `byte_end`.  This ensures that clicking in the right
    ///   margin keeps the caret on the clicked line rather than jumping to the
    ///   beginning of the next one.
    ///
    /// The result is always a valid UTF-8 char boundary.
    fn hit_test_caret(&self, handle: TextHandle, pos: Vec2) -> usize;

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
    /// - If the line ends with a hard newline (`\n`), a hit to the right of the line
    ///   or on the newline itself must return the index of the `\n` character itself.
    fn hit_test_cluster(&self, handle: TextHandle, pos: Vec2) -> usize;
}
