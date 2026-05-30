use crate::types::{Rect, Vec2};

// ── Available space ────────────────────────────────────────────────────────

/// The extent of one axis of a [`LayoutSpace`].
///
/// Position is always concrete (a layout always knows *where* a child starts),
/// so only the *extent* can be unknown. `Unbounded` means "as much as the
/// content wants" — the space a deferred scroll area lays its content into, or a
/// panel told to grow without an enclosing limit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AxisBound {
    /// A known extent in pixels.
    Bounded(f32),
    /// No limit on this axis. A child still resolves to a concrete `Rect`; the
    /// running cursor stays a concrete `f32`, so the accumulated extent is
    /// `Bounded` even though the space it grew into was not (see the two
    /// unbounded rules in the layout design).
    Unbounded,
}

/// The available space a parent hands **down** to a layout. Carries an
/// [`AxisBound`] per axis: the origin (`x`, `y`) is always concrete, but either
/// extent may be [`AxisBound::Unbounded`].
///
/// A plain [`Rect`] converts via [`From`] to a fully-`Bounded` space — this is
/// the common case, and is why `Layout::begin` accepts `impl Into<LayoutSpace>`
/// (every `begin(some_rect)` call keeps working unchanged).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutSpace {
    pub x: f32,
    pub y: f32,
    pub width: AxisBound,
    pub height: AxisBound,
}

impl LayoutSpace {
    pub fn new(x: f32, y: f32, width: AxisBound, height: AxisBound) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// A space bounded in width but unbounded in height — the shape a vertically
    /// scrolling content region is laid out into.
    pub fn unbounded_height(x: f32, y: f32, width: f32) -> Self {
        Self {
            x,
            y,
            width: AxisBound::Bounded(width),
            height: AxisBound::Unbounded,
        }
    }

    /// A space bounded in height but unbounded in width.
    pub fn unbounded_width(x: f32, y: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width: AxisBound::Unbounded,
            height: AxisBound::Bounded(height),
        }
    }
}

/// A fully-specified `Rect` is a fully-`Bounded` space.
impl From<Rect> for LayoutSpace {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: AxisBound::Bounded(r.w),
            height: AxisBound::Bounded(r.h),
        }
    }
}

// ── Intrinsic sizing ──────────────────────────────────────────────────────

/// A widget's own size measurement, reported up to an intrinsic-aware layout.
///
/// Measurement only — never layout policy. "Fill", "grow", and weights are caller
/// intent and live in a layout's `Params`, not here. Every field is optional: a
/// widget may know one axis (or none) and not the other.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct IntrinsicSize {
    /// Smallest size below which content clips (e.g. the longest unbreakable word).
    pub min: Option<Vec2>,
    /// The natural, unconstrained size (e.g. one line of text plus padding).
    pub preferred: Option<Vec2>,
    /// The largest useful size.
    pub max: Option<Vec2>,
}

impl IntrinsicSize {
    /// No measurement known on any axis.
    pub const UNKNOWN: Self = Self {
        min: None,
        preferred: None,
        max: None,
    };

    /// A measurement that reports only its preferred size.
    pub fn preferred(size: Vec2) -> Self {
        Self {
            preferred: Some(size),
            ..Self::UNKNOWN
        }
    }
}

/// Size an intrinsic-aware layout falls back to when it needs a measurement that
/// was never reported (e.g. `Auto` sizing of a widget that returned no
/// `preferred` size). Deliberately large and obvious so missing measurements are
/// visible during development; a future version may log when this is hit.
pub const LAYOUT_FALLBACK_SIZE: Vec2 = Vec2::new(96.0, 96.0);

/// How a child is sized along one axis by an intrinsic-aware layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    /// Exactly this many pixels.
    Fixed(f32),
    /// The widget's intrinsic preferred size on this axis. Falls back to the
    /// corresponding [`LAYOUT_FALLBACK_SIZE`] axis if the widget reports none.
    Auto,
    /// Fill the layout's available space on this axis (its bounds extent).
    ///
    /// Meaningful on the *cross* axis (e.g. a column child filling the panel
    /// width). On the *main* axis of a sequential layout it simply claims the
    /// full bounds extent — leftover/weighted distribution is a later tier.
    Fill,
}

impl Extent {
    /// Resolve this extent to concrete pixels given the widget's intrinsic value
    /// on this axis (if any), the layout's fillable extent, and the fallback.
    fn resolve(self, intrinsic_axis: Option<f32>, fill: AxisBound, fallback: f32) -> f32 {
        match self {
            Extent::Fixed(px) => px,
            Extent::Auto => intrinsic_axis.unwrap_or(fallback),
            Extent::Fill => match fill {
                AxisBound::Bounded(px) => px,
                // Rule 1: filling an unbounded axis is undefined — there is no
                // extent to fill. Fall back to the intrinsic size (then the
                // global fallback), matching `Auto`.
                AxisBound::Unbounded => intrinsic_axis.unwrap_or(fallback),
            },
        }
    }
}

/// A per-axis sizing request a caller hands to an intrinsic-aware layout
/// (column/row/wrap). Axes are absolute (width/height), not main/cross, so the
/// same request reads identically regardless of layout orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SizeReq {
    pub width: Extent,
    pub height: Extent,
}

impl SizeReq {
    /// Both axes fixed to explicit pixels.
    pub fn fixed(width: f32, height: f32) -> Self {
        Self {
            width: Extent::Fixed(width),
            height: Extent::Fixed(height),
        }
    }

    /// Both axes sized to the widget's intrinsic preferred size.
    pub fn auto() -> Self {
        Self {
            width: Extent::Auto,
            height: Extent::Auto,
        }
    }
}

/// Back-compat: a plain size is treated as fixed on both axes.
impl From<Vec2> for SizeReq {
    fn from(v: Vec2) -> Self {
        Self::fixed(v.x, v.y)
    }
}

pub trait Layout {
    type Params;
    type State: LayoutState<Params = Self::Params>;

    /// Initializes the mutable layout state, given the space allocated by the
    /// parent. Accepts anything convertible to a [`LayoutSpace`]; a plain
    /// [`Rect`] is a fully-bounded space, so existing `begin(rect)` calls are
    /// unchanged.
    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State;
}

pub trait LayoutState {
    type Params;

    /// Calculate the screen-space rectangle for a widget given the caller's
    /// `layout_params` (intent) and the widget's `intrinsic` measurement.
    ///
    /// Layouts that don't size from content (e.g. `ManualLayout`) ignore
    /// `intrinsic`; intrinsic-aware layouts (column/row/wrap) read it.
    fn layout(&mut self, layout_params: Self::Params, intrinsic: IntrinsicSize) -> Rect;

    /// The total content extent consumed so far, measured from the layout's
    /// origin (so it is independent of any scroll offset). A deferred scroll area
    /// reads this at `finish` to discover how large its children turned out — the
    /// concrete `f32` end of an [`AxisBound::Unbounded`] axis (the "unbounded
    /// resolves to concrete at accumulation" rule).
    ///
    /// Returns the zero vector before any child is placed.
    fn content_extent(&self) -> Vec2;
}

// ── ManualLayout ──────────────────────────────────────────────────────────

/// A layout that requires exact Rects for every widget.
pub struct ManualLayout;

impl Layout for ManualLayout {
    type Params = Rect;
    type State = ManualState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = space.into();
        ManualState {
            origin: Vec2::new(space.x, space.y),
            content_extent: Vec2::ZERO,
        }
    }
}

pub struct ManualState {
    origin: Vec2,
    content_extent: Vec2,
}

impl LayoutState for ManualState {
    type Params = Rect;

    fn layout(&mut self, layout_params: Rect, _intrinsic: IntrinsicSize) -> Rect {
        // Offset the requested rect by the layout's origin. This ensures if
        // ManualLayout is nested inside a scroll view (or any other layout), the
        // explicit rects still shift correctly relative to the parent. The
        // explicit size is independent of the available extent, so ManualLayout
        // is unaffected by an unbounded axis.
        // The requested rect is origin-relative, so its far edge *is* the content
        // extent contribution (no need to subtract the origin back out).
        self.content_extent.x = self.content_extent.x.max(layout_params.x + layout_params.w);
        self.content_extent.y = self.content_extent.y.max(layout_params.y + layout_params.h);
        Rect::new(
            self.origin.x + layout_params.x,
            self.origin.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        )
    }

    fn content_extent(&self) -> Vec2 {
        self.content_extent
    }
}

// ── ColumnLayout ──────────────────────────────────────────────────────────

pub struct ColumnLayout {
    pub spacing: f32,
}

impl Layout for ColumnLayout {
    type Params = SizeReq;
    type State = ColumnState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = space.into();
        ColumnState {
            current_y: space.y,
            space,
            spacing: self.spacing,
            content_w: 0.0,
            content_h: 0.0,
        }
    }
}

pub struct ColumnState {
    space: LayoutSpace,
    spacing: f32,
    current_y: f32,
    /// Widest child placed so far (cross axis).
    content_w: f32,
    /// Bottom edge of the last child relative to the origin (main axis), i.e. the
    /// consumed height excluding any trailing spacing.
    content_h: f32,
}

impl LayoutState for ColumnState {
    type Params = SizeReq;

    fn layout(&mut self, layout_params: SizeReq, intrinsic: IntrinsicSize) -> Rect {
        let pref = intrinsic.preferred;
        // Cross axis (width) fills the column space; main axis (height) stacks.
        // A `Fill` height (or unbounded height) falls back to intrinsic per Rule 1.
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width, LAYOUT_FALLBACK_SIZE.x);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height, LAYOUT_FALLBACK_SIZE.y);
        let r = Rect::new(self.space.x, self.current_y, w, h);
        self.content_w = self.content_w.max(w);
        self.content_h = (self.current_y + h) - self.space.y;
        self.current_y += h + self.spacing;
        r
    }

    fn content_extent(&self) -> Vec2 {
        Vec2::new(self.content_w, self.content_h)
    }
}

// ── RowLayout ─────────────────────────────────────────────────────────────

pub struct RowLayout {
    pub spacing: f32,
}

impl Layout for RowLayout {
    type Params = SizeReq;
    type State = RowState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = space.into();
        RowState {
            current_x: space.x,
            space,
            spacing: self.spacing,
            content_w: 0.0,
            content_h: 0.0,
        }
    }
}

pub struct RowState {
    space: LayoutSpace,
    spacing: f32,
    current_x: f32,
    /// Right edge of the last child relative to the origin (main axis), i.e. the
    /// consumed width excluding any trailing spacing.
    content_w: f32,
    /// Tallest child placed so far (cross axis).
    content_h: f32,
}

impl LayoutState for RowState {
    type Params = SizeReq;

    fn layout(&mut self, layout_params: SizeReq, intrinsic: IntrinsicSize) -> Rect {
        let pref = intrinsic.preferred;
        // Main axis (width) advances the cursor; cross axis (height) fills space.
        // A `Fill` width (or unbounded width) falls back to intrinsic per Rule 1.
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width, LAYOUT_FALLBACK_SIZE.x);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height, LAYOUT_FALLBACK_SIZE.y);
        let r = Rect::new(self.current_x, self.space.y, w, h);
        self.content_w = (self.current_x + w) - self.space.x;
        self.content_h = self.content_h.max(h);
        self.current_x += w + self.spacing;
        r
    }

    fn content_extent(&self) -> Vec2 {
        Vec2::new(self.content_w, self.content_h)
    }
}

// ── OffsetLayout ──────────────────────────────────────────────────────────

/// A pure decorator layout that shifts coordinates on the Y axis.
pub struct OffsetLayout<L> {
    pub offset: Vec2,
    pub inner: L,
}

impl<L: Layout> Layout for OffsetLayout<L> {
    type Params = L::Params;
    type State = OffsetState<L::State>;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        OffsetState {
            offset: self.offset,
            inner: self.inner.begin(space.into()),
        }
    }
}

pub struct OffsetState<InnerS> {
    offset: Vec2,
    inner: InnerS,
}

impl<InnerS: LayoutState> LayoutState for OffsetState<InnerS> {
    type Params = InnerS::Params;

    fn layout(&mut self, layout_params: Self::Params, intrinsic: IntrinsicSize) -> Rect {
        let mut r = self.inner.layout(layout_params, intrinsic);
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }

    fn content_extent(&self) -> Vec2 {
        // The content extent is offset-independent: it describes how large the
        // children are, not where they are scrolled to.
        self.inner.content_extent()
    }
}

// ── WrapLayout ─────────────────────────────────────────────────────────────

/// A flow layout: places children left-to-right, wrapping to the next line when
/// the next child would overflow the bounds width. Intrinsic-aware — children
/// are sized from their [`SizeReq`] and reported intrinsic size, exactly like
/// row/column.
pub struct WrapLayout {
    /// Horizontal gap between items on a line.
    pub spacing: f32,
    /// Vertical gap between lines.
    pub line_spacing: f32,
}

impl Layout for WrapLayout {
    type Params = SizeReq;
    type State = WrapState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = space.into();
        WrapState {
            current_x: space.x,
            current_y: space.y,
            space,
            spacing: self.spacing,
            line_spacing: self.line_spacing,
            line_height: 0.0,
            content_w: 0.0,
        }
    }
}

pub struct WrapState {
    space: LayoutSpace,
    spacing: f32,
    line_spacing: f32,
    current_x: f32,
    current_y: f32,
    /// Tallest item on the current line, used to advance to the next line.
    line_height: f32,
    /// Widest line right-edge reached relative to the origin (cross-line max).
    content_w: f32,
}

impl LayoutState for WrapState {
    type Params = SizeReq;

    fn layout(&mut self, layout_params: SizeReq, intrinsic: IntrinsicSize) -> Rect {
        let pref = intrinsic.preferred;
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width, LAYOUT_FALLBACK_SIZE.x);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height, LAYOUT_FALLBACK_SIZE.y);

        // Wrap before placing if this item would overflow the line — but never
        // wrap an item that is already at the start of a line (it just clips).
        // An unbounded width has no edge to overflow, so the flow never wraps.
        let at_line_start = self.current_x == self.space.x;
        let overflows = match self.space.width {
            AxisBound::Bounded(width) => self.current_x + w > self.space.x + width,
            AxisBound::Unbounded => false,
        };
        if !at_line_start && overflows {
            self.current_x = self.space.x;
            self.current_y += self.line_height + self.line_spacing;
            self.line_height = 0.0;
        }

        let r = Rect::new(self.current_x, self.current_y, w, h);
        self.content_w = self.content_w.max((self.current_x + w) - self.space.x);
        self.current_x += w + self.spacing;
        self.line_height = self.line_height.max(h);
        r
    }

    fn content_extent(&self) -> Vec2 {
        // Width: the widest line. Height: the bottom of the current (last) line.
        Vec2::new(self.content_w, (self.current_y + self.line_height) - self.space.y)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intrinsic_size_constructors() {
        assert_eq!(IntrinsicSize::UNKNOWN, IntrinsicSize::default());
        let i = IntrinsicSize::preferred(Vec2::new(10.0, 20.0));
        assert_eq!(i.preferred, Some(Vec2::new(10.0, 20.0)));
        assert_eq!(i.min, None);
        assert_eq!(i.max, None);
        assert_eq!(LAYOUT_FALLBACK_SIZE, Vec2::new(96.0, 96.0));
    }

    #[test]
    fn test_manual_layout() {
        let mut state = ManualLayout.begin(Rect::new(10.0, 10.0, 100.0, 100.0));
        let r = state.layout(Rect::new(5.0, 5.0, 20.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(r, Rect::new(15.0, 15.0, 20.0, 20.0));
    }

    #[test]
    fn test_column_layout() {
        let mut state = ColumnLayout { spacing: 10.0 }.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_row_layout() {
        let mut state = RowLayout { spacing: 5.0 }.begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        let r2 = state.layout(Vec2::new(20.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    fn test_column_auto_uses_intrinsic_preferred() {
        let mut state = ColumnLayout { spacing: 0.0 }.begin(Rect::new(0.0, 0.0, 200.0, 500.0));
        let req = SizeReq {
            width: Extent::Fixed(120.0),
            height: Extent::Auto,
        };
        let intrinsic = IntrinsicSize::preferred(Vec2::new(80.0, 24.0));
        let r = state.layout(req, intrinsic);
        // Auto height reads intrinsic.preferred.y; width stays fixed.
        assert_eq!(r, Rect::new(0.0, 0.0, 120.0, 24.0));
    }

    #[test]
    fn test_column_fill_cross_axis_uses_bounds_width() {
        let mut state = ColumnLayout { spacing: 0.0 }.begin(Rect::new(5.0, 0.0, 200.0, 500.0));
        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(30.0),
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN);
        // Fill width spans the column bounds width.
        assert_eq!(r, Rect::new(5.0, 0.0, 200.0, 30.0));
    }

    #[test]
    fn test_auto_without_intrinsic_falls_back() {
        let mut state = RowLayout { spacing: 0.0 }.begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let r = state.layout(SizeReq::auto(), IntrinsicSize::UNKNOWN);
        // No intrinsic reported → both axes use the global fallback.
        assert_eq!(
            r,
            Rect::new(0.0, 0.0, LAYOUT_FALLBACK_SIZE.x, LAYOUT_FALLBACK_SIZE.y)
        );
    }

    #[test]
    fn test_row_auto_width_advances_cursor() {
        let mut state = RowLayout { spacing: 6.0 }.begin(Rect::new(0.0, 0.0, 400.0, 50.0));
        let req = SizeReq {
            width: Extent::Auto,
            height: Extent::Fixed(40.0),
        };
        let r1 = state.layout(req, IntrinsicSize::preferred(Vec2::new(70.0, 16.0)));
        assert_eq!(r1, Rect::new(0.0, 0.0, 70.0, 40.0));
        let r2 = state.layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 16.0)));
        // Cursor advanced by first auto width (70) + spacing (6).
        assert_eq!(r2, Rect::new(76.0, 0.0, 50.0, 40.0));
    }

    #[test]
    fn test_wrap_layout_wraps_on_overflow() {
        // 100px-wide bounds, 40px items, no spacing: two per line, then wrap.
        let mut state = WrapLayout {
            spacing: 0.0,
            line_spacing: 5.0,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 500.0));
        let item = SizeReq {
            width: Extent::Fixed(40.0),
            height: Extent::Fixed(20.0),
        };
        let r1 = state.layout(item, IntrinsicSize::UNKNOWN);
        let r2 = state.layout(item, IntrinsicSize::UNKNOWN);
        let r3 = state.layout(item, IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 40.0, 20.0));
        assert_eq!(r2, Rect::new(40.0, 0.0, 40.0, 20.0));
        // Third item (would end at 120 > 100) wraps to the next line at
        // y = line_height(20) + line_spacing(5) = 25.
        assert_eq!(r3, Rect::new(0.0, 25.0, 40.0, 20.0));
    }

    #[test]
    fn test_wrap_layout_uses_intrinsic_and_does_not_wrap_first_item() {
        // A single item wider than the bounds stays on the first line (no wrap
        // at line start); auto width comes from the intrinsic preferred size.
        let mut state = WrapLayout {
            spacing: 0.0,
            line_spacing: 0.0,
        }
        .begin(Rect::new(0.0, 0.0, 30.0, 500.0));
        let r = state.layout(SizeReq::auto(), IntrinsicSize::preferred(Vec2::new(80.0, 16.0)));
        assert_eq!(r, Rect::new(0.0, 0.0, 80.0, 16.0));
    }

    #[test]
    fn test_rect_converts_to_bounded_space() {
        let space: LayoutSpace = Rect::new(1.0, 2.0, 30.0, 40.0).into();
        assert_eq!(
            space,
            LayoutSpace {
                x: 1.0,
                y: 2.0,
                width: AxisBound::Bounded(30.0),
                height: AxisBound::Bounded(40.0),
            }
        );
    }

    #[test]
    fn test_column_unbounded_height_resolves_concrete() {
        // Rule 2: a child laid out in an unbounded main axis still resolves to a
        // concrete Rect, and the cursor advances by a concrete f32.
        let mut state = ColumnLayout { spacing: 5.0 }
            .begin(LayoutSpace::unbounded_height(0.0, 0.0, 200.0));
        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Auto,
        };
        let r1 = state.layout(req, IntrinsicSize::preferred(Vec2::new(80.0, 24.0)));
        // Fill width uses the bounded width; Auto height uses intrinsic.
        assert_eq!(r1, Rect::new(0.0, 0.0, 200.0, 24.0));
        let r2 = state.layout(req, IntrinsicSize::preferred(Vec2::new(80.0, 30.0)));
        // Cursor advanced concretely by 24 + spacing(5) = 29.
        assert_eq!(r2, Rect::new(0.0, 29.0, 200.0, 30.0));
        assert!(r2.y.is_finite());
    }

    #[test]
    fn test_fill_on_unbounded_axis_falls_back_to_intrinsic() {
        // Rule 1: Fill on an unbounded axis is undefined — falls back to intrinsic.
        let mut state = ColumnLayout { spacing: 0.0 }
            .begin(LayoutSpace::unbounded_height(0.0, 0.0, 100.0));
        let req = SizeReq {
            width: Extent::Fixed(50.0),
            height: Extent::Fill,
        };
        let r = state.layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 18.0)));
        // Fill height has no extent to fill → intrinsic 18.
        assert_eq!(r, Rect::new(0.0, 0.0, 50.0, 18.0));
    }

    #[test]
    fn test_fill_on_unbounded_axis_without_intrinsic_uses_fallback() {
        let mut state =
            RowLayout { spacing: 0.0 }.begin(LayoutSpace::unbounded_width(0.0, 0.0, 40.0));
        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(40.0),
        };
        // Fill width on the unbounded axis, no intrinsic → global fallback.
        let r = state.layout(req, IntrinsicSize::UNKNOWN);
        assert_eq!(r, Rect::new(0.0, 0.0, LAYOUT_FALLBACK_SIZE.x, 40.0));
    }

    #[test]
    fn test_wrap_unbounded_width_never_wraps() {
        // An unbounded width has no edge to overflow: every item stays on line 0.
        let mut state = WrapLayout {
            spacing: 0.0,
            line_spacing: 5.0,
        }
        .begin(LayoutSpace::unbounded_width(0.0, 0.0, 500.0));
        let item = SizeReq {
            width: Extent::Fixed(40.0),
            height: Extent::Fixed(20.0),
        };
        let r1 = state.layout(item, IntrinsicSize::UNKNOWN);
        let r2 = state.layout(item, IntrinsicSize::UNKNOWN);
        let r3 = state.layout(item, IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 40.0, 20.0));
        assert_eq!(r2, Rect::new(40.0, 0.0, 40.0, 20.0));
        assert_eq!(r3, Rect::new(80.0, 0.0, 40.0, 20.0));
    }

    #[test]
    fn test_column_content_extent() {
        let mut state = ColumnLayout { spacing: 10.0 }.begin(Rect::new(5.0, 7.0, 100.0, 500.0));
        assert_eq!(state.content_extent(), Vec2::ZERO);
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(60.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // Width = widest child (60); height = bottom of last child = 20 + 10 + 30 = 60
        // (no trailing spacing counted).
        assert_eq!(state.content_extent(), Vec2::new(60.0, 60.0));
    }

    #[test]
    fn test_row_content_extent() {
        let mut state = RowLayout { spacing: 5.0 }.begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(20.0, 40.0).into(), IntrinsicSize::UNKNOWN);
        // Width = right of last child = 30 + 5 + 20 = 55; height = tallest child (40).
        assert_eq!(state.content_extent(), Vec2::new(55.0, 40.0));
    }

    #[test]
    fn test_manual_content_extent() {
        let mut state = ManualLayout.begin(Rect::new(100.0, 100.0, 0.0, 0.0));
        state.layout(Rect::new(0.0, 0.0, 50.0, 20.0), IntrinsicSize::UNKNOWN);
        state.layout(Rect::new(80.0, 40.0, 30.0, 30.0), IntrinsicSize::UNKNOWN);
        // Extent is origin-relative: max far edges = (80+30, 40+30) = (110, 70).
        assert_eq!(state.content_extent(), Vec2::new(110.0, 70.0));
    }

    #[test]
    fn test_offset_content_extent_ignores_offset() {
        let offset = OffsetLayout {
            offset: Vec2::new(13.0, 27.0),
            inner: ColumnLayout { spacing: 0.0 },
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // Content extent describes child size, not scroll position — offset ignored.
        assert_eq!(state.content_extent(), Vec2::new(40.0, 50.0));
    }

    #[test]
    fn test_offset_layout() {
        let offset = OffsetLayout {
            offset: Vec2::new(5.0, 15.0),
            inner: ColumnLayout { spacing: 10.0 },
        };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = offset.begin(bounds);

        let r1 = state.layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        // Logic X is 10.0. Actual X = 10.0 - 5.0 = 5.0
        assert_eq!(r1, Rect::new(5.0, -5.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // Logic Y is 10.0 + 20.0 + 10.0 = 40.0. Actual Y = 40.0 - 15.0 = 25.0
        assert_eq!(r2, Rect::new(5.0, 25.0, 40.0, 30.0));
    }
}
