use crate::types::{Rect, Vec2};

/// A lightweight application-owned font handle.
///
/// Framewise never loads or owns font files. It only passes this handle to the
/// application's `TextSystem`, which decides how the handle maps to real font
/// data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontId(pub u16);

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

/// How a block of text flows and fills the space it is measured or drawn against.
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
/// - X overflow asks what to do when the next glyph would not fit wholly within
///   the current line's horizontal bounds.
/// - Y overflow asks what to do when the next visual line would not fit wholly
///   within the block's vertical bounds.
///
/// This makes wrapping just one possible X-axis overflow response, rather than
/// a separate boolean. Hard line breaks (`'\n'`) are always respected before
/// X-overflow handling is applied.
#[derive(Debug, Clone, Copy, PartialEq)]
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
    pub horizontal_align: HorizontalAlign,
}

impl TextFlow {
    /// Single-line-ish label/input default.
    ///
    /// Hard `'\n'` still creates additional source lines, but no soft wrapping is
    /// performed. Horizontally overflowing glyphs are dropped, and vertically
    /// overflowing lines are dropped.
    pub fn single_line() -> Self {
        Self {
            overflow_x: OverflowX::Drop,
            overflow_y: OverflowY::Drop,
            horizontal_align: HorizontalAlign::Start,
        }
    }

    /// Paragraph/caption default.
    ///
    /// Wraps at word boundaries first, falls back to glyph wrapping for over-long
    /// words, drops a glyph only if even a single glyph cannot fit on an empty
    /// line, and ellipsises vertical overflow.
    pub fn wrapped() -> Self {
        Self {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Drop,
                },
            },
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            horizontal_align: HorizontalAlign::Start,
        }
    }

    /// Renderer-clipped text viewport default.
    ///
    /// This policy may emit glyphs/lines that intersect the bounds but are not
    /// wholly inside them. It is intended for renderers that apply their own
    /// scissor/clipping and want edge glyphs to be partially visible.
    pub fn clipped_viewport() -> Self {
        Self {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        }
    }
}

/// What to do when the next glyph would not fit wholly within the current line's
/// horizontal bounds.
///
/// Policies are deliberately expressed as a fallback chain. This mirrors the
/// actual layout decision tree:
///
/// 1. Prefer a high-level behavior, such as word wrapping.
/// 2. If that cannot make progress, fall back to a lower-level behavior, such as
///    glyph wrapping.
/// 3. If even that cannot make progress, either drop the overflowing unit or keep
///    it and rely on downstream clipping.
///
/// The important contract is:
///
/// - `Drop`, successful wrapping, and successful ellipsis fitting emit only
///   glyphs wholly inside the X bounds.
/// - `Keep` may emit the first overflowing glyph, then truncates the rest of that
///   line. A renderer/scissor may clip the visible pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    WrapWord { fallback: WrapWordFallback },

    /// Wrap at glyph/character-cluster boundaries.
    ///
    /// If the next glyph does not fit on the current line, it is moved to a new
    /// line. If it still cannot fit on an empty line, `fallback` decides whether
    /// it is dropped or kept partially.
    ///
    /// “Glyph” here should be read as the smallest drawable unit the text system
    /// can safely split without corrupting shaping. For complex scripts this may
    /// need to mean a grapheme cluster or shaped cluster rather than a literal
    /// font glyph.
    WrapGlyph { fallback: WrapGlyphFallback },

    /// Replace the overflowing tail of the line with an ellipsis marker.
    ///
    /// The text system drops enough trailing glyphs so the ellipsis itself fits
    /// wholly within the X bounds. If even the ellipsis cannot fit,
    /// `fallback` decides what to do.
    Ellipsis { fallback: EllipsisFallback },

    /// Include the first glyph that does not fit wholly within the X bounds, then
    /// drop the remaining glyphs on that line.
    ///
    /// This is the opt-in partial-glyph mode. It is useful when the renderer
    /// applies clipping and the caller wants edge glyphs to be visibly sliced
    /// rather than removed entirely.
    Keep,

    /// Drop the first glyph that does not fit wholly within the X bounds, and
    /// drop the remaining glyphs on that line.
    ///
    /// This is the strict fully-inside truncate behavior.
    Drop,
}

/// Fallback used by [`OverflowX::WrapWord`] when a word cannot fit on an empty
/// line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapWordFallback {
    /// Try breaking the over-long word at glyph/cluster boundaries, see OverflowX::WrapGlyph.
    WrapGlyph { fallback: WrapGlyphFallback },

    /// Keep the over-long word's first overflowing glyph/cluster, then truncate. See OverflowX::Keep.
    ///
    /// May emit geometry outside the X bounds.
    Keep,

    /// Drop the over-long word when it cannot fit. See OverflowX::Drop.
    Drop,
}

/// Fallback used by glyph wrapping when even one glyph/cluster cannot fit on an
/// empty line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapGlyphFallback {
    /// Keep the first overflowing glyph/cluster, then truncate. See OverflowX::Keep.
    ///
    /// May emit geometry outside the X bounds.
    Keep,

    /// Drop the glyph/cluster. See OverflowX::Drop.
    Drop,
}

/// What to do when the next visual line would not fit wholly within the block's
/// vertical bounds.
///
/// This policy operates on whole visual lines, not individual glyphs. A visual
/// line may come from a hard break or from X-axis wrapping.
///
/// The same inside/outside contract applies:
///
/// - `Drop` and successful `Ellipsis` emit only lines wholly inside the Y bounds.
/// - `Keep` may emit the first vertically overflowing line, then drops all later
///   lines. A renderer/scissor may clip it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverflowY {
    /// Indicate vertical truncation by ellipsising the previous visible line.
    ///
    /// The overflowing line itself is not emitted. Instead, the last line that
    /// fits wholly inside the Y bounds is modified using X-axis ellipsis fitting.
    ///
    /// If there is no previous visible line, or if the ellipsis cannot fit in the
    /// available X bounds, `fallback` decides what to do.
    Ellipsis { fallback: EllipsisFallback },

    /// Include the first line that does not fit wholly within the Y bounds, then
    /// drop all later lines.
    ///
    /// This is useful for clipped text viewports where partially visible top or
    /// bottom lines should still render.
    Keep,

    /// Drop the first line that does not fit wholly within the Y bounds, and drop
    /// all later lines.
    Drop,
}

/// Fallback used when an ellipsis marker cannot be fitted.
///
/// `Keep` is intentionally allowed even though ellipsis normally implies a
/// fully-inside marker. It is useful as a “show something rather than nothing”
/// policy for extremely small rectangles. Callers that require strict
/// fully-inside rendering should use `Drop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EllipsisFallback {
    /// Keep the first overflowing glyph or line, depending on whether the
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalAlign {
    Start,
    Center,
    End,
}

// ── Measurement inputs & outputs ────────────────────────────────────────────

/// The space available to lay text into, used by [`TextSystem::measure`].
///
/// Each axis is `Some(px)` for a finite ceiling, or `None` for unbounded. This
/// is the reduction of the layout's `AxisBound`: both `Exact(w)` and `AtMost(w)`
/// become `Some(w)` (the distinction between a committed frame and a bare ceiling
/// does not matter for measurement — only the limit value does), while
/// `Unbounded` becomes `None`.
///
/// Measurement is **symmetric**: text is reflowable, so its size is a curve, not
/// a point. Whichever axis is bounded constrains the flow; the unbounded axis is
/// the answer:
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

/// The measured geometry of a block of text, independent of where it is drawn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextMetrics {
    /// Tight size of the laid-out block in logical pixels.
    ///
    /// - `x` is the widest line's used advance width (shrink-wrapped — it is `≤`
    ///   `max_width` when a width bound was given, *not* the bound itself).
    /// - `y` is `visible_line_count × line_height`, where `line_height` is the
    ///   font's line spacing at this size.
    ///
    /// "Visible" means after any vertical overflow has been applied: a block
    /// clipped to a height reports the size it actually occupies, not the size
    /// the full string would have needed.
    pub size: Vec2,

    /// Number of lines actually laid out (after wrapping, hard breaks, and
    /// vertical overflow). Always `≥ 1`, even for empty input.
    pub line_count: u32,

    /// `true` if any line was cut on the inline axis — a glyph run was wider than
    /// the available width and got clipped/ellipsised. With `wrap: true` this is
    /// rare (over-long words force-break instead) but can still occur when the
    /// width is narrower than a single glyph.
    pub truncated_horizontal: bool,

    /// `true` if whole lines were dropped because the content exceeded the
    /// available height.
    pub truncated_vertical: bool,
}

/// The geometry and handle for a piece of text prepared for drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayout {
    /// The opaque handle to give to the renderer via `DrawCmd::Text`.
    pub handle: TextHandle,
    /// The block's measured geometry, identical to what [`TextSystem::measure`]
    /// would return for the same text, flow policy, and the draw rect's size as
    /// bounds.
    pub metrics: TextMetrics,
}

/// The geometry of a text caret at a given byte position, in block-local
/// coordinates (origin at the block's top-left, y increasing downward).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaretGeom {
    /// X offset of the caret (the leading edge of the glyph at the queried byte,
    /// or the trailing edge of the last glyph when the byte is at end-of-text).
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
/// ### Optical Ink Bounds Alignment (Approach 2)
/// To support premium grid alignments in the GUI, all positions returned by this trait
/// are relative to the text's tight **ink bounds** rather than the typographic space.
/// The implementing `TextSystem` must shift shaped glyph horizontal positions (`g.x`)
/// by $-l$ (where $l$ is the leftmost horizontal pixel position across all glyphs in the segment)
/// so that the first visible pixel begins exactly at `x = 0.0` relative to the bounding box.
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
    /// The returned size represents the tight **ink bounds** ($r - l$) of the text run,
    /// excluding the typographic left-side bearing, ensuring the layout bounding box
    /// perfectly wraps the visible pixels.
    ///
    /// `flow.horizontal_align` has no effect on the result: alignment moves glyphs
    /// within a line but changes neither the block size nor what is truncated.
    ///
    /// Must be free of observable side effects on the run table — calling
    /// `measure` does not allocate a [`TextHandle`].
    fn measure(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        flow: TextFlow,
        bounds: TextBounds,
    ) -> TextMetrics;

    /// Shape `text` for drawing into `rect` and register it, returning a handle.
    ///
    /// `rect` is the fully concrete **ink-bounds** bounding box by the time this is called:
    /// its width is the wrap width and its height is the vertical clip extent.
    ///
    /// Because `rect` represents the ink bounds, the implementor must shift all shaped
    /// glyph coordinates by $-l$ (where $l$ is the leftmost horizontal coordinate of the unshifted
    /// shaped glyphs) so that the ink begins exactly at `x = 0.0` relative to the bounding box.
    ///
    /// The returned [`TextLayout::metrics`] equal what [`measure`](Self::measure)
    /// would report for the same `text`/`size`/`font`/`flow` and
    /// `TextBounds { max_width: Some(rect.w), max_height: Some(rect.h) }`.
    ///
    /// The handle is valid until the next frame reset (see [`TextHandle`]).
    fn prepare(
        &mut self,
        text: &str,
        size: f32,
        font: FontId,
        flow: TextFlow,
        rect: Rect,
    ) -> TextLayout;

    /// Caret geometry for the character boundary at `byte_index`, in block-local
    /// coordinates. See [`CaretGeom`].
    ///
    /// Because the cached layout is pre-shifted to ink bounds, caret positions
    /// must also align with the shifted coordinate system ($x = \text{typographic\_x} - l$).
    ///
    /// `byte_index` must fall on a UTF-8 char boundary of the prepared string.
    /// An index at or past the end returns the caret after the final glyph. If
    /// the glyph at that index was dropped by overflow, the caret clamps to the
    /// nearest laid-out boundary.
    fn caret_geom(&self, handle: TextHandle, byte_index: usize) -> CaretGeom;

    /// Hit-test a point (block-local coordinates) to the nearest character
    /// boundary, returning a byte index into the prepared string.
    ///
    /// The coordinates `pos` are in the pre-shifted ink-bounds space. Hit testing
    /// must compare against the shifted glyph positions in the cached run.
    ///
    /// The point is resolved to a line by `y` first, then to the nearest gap
    /// between glyphs by `x`. Points above/below the block clamp to the first/last
    /// line; points left/right of a line clamp to that line's start/end. The
    /// result is always a valid char boundary.
    fn hit_test(&self, handle: TextHandle, pos: Vec2) -> usize;
}
