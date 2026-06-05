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
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextFlow {
    /// Soft-wrapping toggle.
    ///
    /// - `true` — break the text onto multiple lines to fit the available
    ///   width. Breaks are preferred at word boundaries (whitespace). A single
    ///   "word" that is itself wider than the available width is **force-broken**
    ///   mid-word so it never spills past the edge (think long URLs or hashes).
    /// - `false` — no soft breaks are ever introduced; the text is laid out as a
    ///   single run per paragraph and may exceed the available width (where it is
    ///   then subject to `overflow`).
    ///
    /// A literal `'\n'` in the input is a **hard break** and always starts a new
    /// line, independent of this flag. So `wrap: false` text can still occupy
    /// multiple lines if the string contains newlines — this flag governs only
    /// *automatic* (width-driven) breaking, not explicit ones.
    pub wrap: bool,

    /// What happens to content that does not fit the available space after
    /// wrapping has been applied. See [`Overflow`].
    pub overflow: Overflow,

    /// How lines are positioned **horizontally** within the available width.
    /// See [`HorizontalAlign`].
    ///
    /// Vertical placement of the block is *not* handled here — that is the
    /// caller's responsibility (it positions the block's rect). This only
    /// distributes each line across the inline axis.
    pub horizontal_align: HorizontalAlign,
}

impl TextFlow {
    /// A single visual line: no soft wrapping, truncate-and-clip on overflow,
    /// start-aligned. The classic label/button/field default.
    ///
    /// Note this does not *force* one line — a `'\n'` in the string still hard-
    /// breaks. Constrain the height (via the draw `Rect` or measure
    /// [`TextBounds`]) to cap the visible line count.
    pub fn single_line() -> Self {
        Self {
            wrap: false,
            overflow: Overflow::Clip,
            horizontal_align: HorizontalAlign::Start,
        }
    }

    /// Multi-line wrapped text: soft-wrap on, ellipsis on overflow,
    /// start-aligned. The classic paragraph/caption default.
    pub fn wrapped() -> Self {
        Self {
            wrap: true,
            overflow: Overflow::Ellipsis,
            horizontal_align: HorizontalAlign::Start,
        }
    }
}

/// What to do with text that cannot fit the available space.
///
/// Overflow applies on **both axes** and only takes effect when content actually
/// exceeds the space:
/// - **Horizontal** — a line is wider than the available width and no break was
///   taken there (e.g. `wrap: false`, or an unbreakable run on a wrapped line).
/// - **Vertical** — there are more lines than the available height admits.
///
/// In all cases truncation happens at a **character boundary** — whole glyphs
/// are kept or dropped, never sliced. (Sub-pixel clipping at the rect edge is a
/// separate, additional concern handled by the renderer's scissor, not here.)
///
/// # Examples
///
/// Width fits only `"Hello w"` of `"Hello world"` on a single line:
/// - `Clip` → renders `Hello w`
/// - `Ellipsis` → renders `Hello…` (drops *additional* glyphs so the `…` itself
///   fits within the width)
///
/// A wrapped paragraph needs 4 lines but the height admits only 2:
/// - `Clip` → renders lines 1–2, drops 3–4
/// - `Ellipsis` → renders lines 1–2 with the tail of line 2 replaced by `…`
///
/// When everything fits, `Clip` and `Ellipsis` are identical, no marker is
/// drawn, and both `truncated_*` flags are `false`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overflow {
    /// Drop overflowing glyphs/lines at the character boundary. No marker.
    Clip,
    /// Drop overflowing glyphs/lines, then place a `…` ellipsis at the cut point
    /// (end of the last visible line), re-fitting so the ellipsis stays inside
    /// the available width.
    Ellipsis,
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
