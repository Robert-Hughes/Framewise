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
    /// "you live in a box of this size" — provides a limit AND anchor frame (e.g. for right-aligning).
    Exact(f32),
    /// "choose your own size, but don't exceed this" — limit only, can't right-align.
    AtMost(f32),
    /// "no ceiling from me on this axis".
    Unbounded,
}

impl AxisBound {
    pub fn resolve(self, measured: f32) -> f32 {
        match self {
            AxisBound::Exact(w) => w,
            AxisBound::AtMost(max_w) => measured.min(max_w),
            AxisBound::Unbounded => measured,
        }
    }
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

    pub fn resolve(self, measured: Vec2) -> Rect {
        Rect::new(
            self.x,
            self.y,
            self.width.resolve(measured.x),
            self.height.resolve(measured.y),
        )
    }

    /// A space bounded in width but unbounded in height — the shape a vertically
    /// scrolling content region is laid out into.
    pub fn unbounded_height(x: f32, y: f32, width: f32) -> Self {
        Self {
            x,
            y,
            width: AxisBound::Exact(width),
            height: AxisBound::Unbounded,
        }
    }

    /// A space bounded in height but unbounded in width.
    pub fn unbounded_width(x: f32, y: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width: AxisBound::Unbounded,
            height: AxisBound::Exact(height),
        }
    }

    /// Subtract margins from a space constraint (analogous to Rect::inset).
    pub fn inset(self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            width: match self.width {
                AxisBound::Exact(w) => AxisBound::Exact((w - amount * 2.0).max(0.0)),
                AxisBound::AtMost(w) => AxisBound::AtMost((w - amount * 2.0).max(0.0)),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
            height: match self.height {
                AxisBound::Exact(h) => AxisBound::Exact((h - amount * 2.0).max(0.0)),
                AxisBound::AtMost(h) => AxisBound::AtMost((h - amount * 2.0).max(0.0)),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        }
    }
}

/// A fully-specified `Rect` is a fully-`Bounded` space.
impl From<Rect> for LayoutSpace {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: AxisBound::Exact(r.w),
            height: AxisBound::Exact(r.h),
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

/// How a child is sized along one axis by an intrinsic-aware layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    /// Exactly this many pixels.
    Fixed(f32),
    /// The widget's intrinsic preferred size on this axis. Panics if the widget
    /// reports no preferred size on this axis (an unsatisfiable request).
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
    /// on this axis (if any) and the layout's fillable extent.
    ///
    /// Panics if a measurement is needed but none was reported (e.g. `Auto`, or
    /// `Fill` on a non-`Exact` axis, with no intrinsic `preferred` size). This is
    /// consistent with the other layout panics (e.g. `CrossAlign` on a non-`Exact`
    /// cross axis): an unsatisfiable sizing request is a bug at the call site, so we
    /// fail loudly rather than silently substituting an arbitrary size.
    pub(crate) fn resolve(self, intrinsic_axis: Option<f32>, fill: AxisBound) -> f32 {
        // Obtain the intrinsic preferred size on this axis, or panic if it was never
        // measured. Only the branches that actually need it call this.
        let preferred = || {
            intrinsic_axis.unwrap_or_else(|| {
                panic!(
                    "Layout panic: {self:?} sizing needs an intrinsic measurement on this \
                     axis but none was reported. A child placed with Auto (or Fill on a \
                     non-Exact axis) must report a preferred size."
                )
            })
        };
        match self {
            Extent::Fixed(px) => px,
            Extent::Auto => match fill {
                AxisBound::Exact(_w) => preferred(),
                AxisBound::AtMost(w) => preferred().min(w),
                AxisBound::Unbounded => preferred(),
            },
            Extent::Fill => match fill {
                AxisBound::Exact(w) => w,
                // Position & distribution policies — fill, right-align, center,
                // space-between — require Exact: a committed frame with a far edge.
                // AtMost and Unbounded permit only measurement / shrink-wrap decisions,
                // so we fall back to Auto.
                AxisBound::AtMost(w) => preferred().min(w),
                AxisBound::Unbounded => preferred(),
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

#[must_use = "a LayoutToken must be finished with end_layout() to advance the parent layout"]
pub struct LayoutToken<'a, LS: LayoutState> {
    pub state: &'a mut LS,
    pub params: LS::Params,
}

impl<'a, LS: LayoutState> LayoutToken<'a, LS> {
    pub fn end_layout(self, extent: Vec2) -> Rect {
        self.state.end_layout(self.params, extent)
    }
}

pub trait LayoutState {
    type Params: Clone;

    /// Calculate the screen-space rectangle for a widget given the caller's
    /// `layout_params` (intent) and the widget's `intrinsic` measurement.
    ///
    /// Layouts that don't size from content (e.g. `ManualLayout`) ignore
    /// `intrinsic`; intrinsic-aware layouts (column/row/wrap) read it.
    fn layout(&mut self, layout_params: Self::Params, intrinsic: IntrinsicSize) -> Rect;

    /// Begin a deferred layout (for fit-to-children containers).
    /// Returns a provisional [`LayoutSpace`] and a [`LayoutToken`] that borrows this layout state.
    ///
    /// `intrinsic` mirrors the same parameter on [`LayoutState::layout`]; layout
    /// implementations may use it (e.g. to enforce a minimum size floor) or ignore it.
    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
        intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>)
    where
        Self: Sized;

    /// End a deferred layout, providing the actual final accumulated content extent.
    /// Returns the resolved concrete Rect of the container and advances the layout state.
    fn end_layout(&mut self, layout_params: Self::Params, extent: Vec2) -> Rect;

    /// The resolved concrete space Rect occupied by the entire layout (which might be
    /// bounded or accumulated), measured relative to parent coordinates but independent
    /// of any temporary offsets.
    fn resolve_space(&self) -> Rect;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAlign {
    #[default]
    Start,
    Center,
    End,
}

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
    }

    #[test]
    fn test_rect_converts_to_bounded_space() {
        let space: LayoutSpace = Rect::new(1.0, 2.0, 30.0, 40.0).into();
        assert_eq!(
            space,
            LayoutSpace {
                x: 1.0,
                y: 2.0,
                width: AxisBound::Exact(30.0),
                height: AxisBound::Exact(40.0),
            }
        );
    }

    #[test]
    fn test_at_most_resolution() {
        // Extent::Fixed is always fixed
        assert_eq!(
            Extent::Fixed(50.0).resolve(Some(30.0), AxisBound::AtMost(100.0)),
            50.0
        );
        assert_eq!(
            Extent::Fixed(150.0).resolve(Some(30.0), AxisBound::AtMost(100.0)),
            150.0
        );

        // Extent::Auto uses preferred, but caps at AtMost
        assert_eq!(
            Extent::Auto.resolve(Some(40.0), AxisBound::AtMost(100.0)),
            40.0
        );
        assert_eq!(
            Extent::Auto.resolve(Some(120.0), AxisBound::AtMost(100.0)),
            100.0
        );

        // Extent::Fill acts as Auto under AtMost
        assert_eq!(
            Extent::Fill.resolve(Some(40.0), AxisBound::AtMost(100.0)),
            40.0
        );
        assert_eq!(
            Extent::Fill.resolve(Some(120.0), AxisBound::AtMost(100.0)),
            100.0
        );
    }

    #[test]
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_auto_resolve_without_intrinsic_panics() {
        // Auto with no measured preferred size is unsatisfiable.
        let _ = Extent::Auto.resolve(None, AxisBound::AtMost(80.0));
    }

    #[test]
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_fill_resolve_without_intrinsic_panics() {
        // Fill under AtMost degrades to Auto, so it too needs a measurement.
        let _ = Extent::Fill.resolve(None, AxisBound::AtMost(80.0));
    }
}
