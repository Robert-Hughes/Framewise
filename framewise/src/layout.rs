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

/// The cross-axis alignment of a widget inside available layout space.
///
/// Default is [`Align::Start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    #[default]
    Start,
    Center,
    End,
}

/// Sizing policy for a sized widget on a single axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// Fixed pixel size.
    Fixed(f32),
    /// Size based on the widget's intrinsic preferred size.
    Auto,
}

/// Sizing and alignment policy for a widget on a single axis.
///
/// Under this model, alignment and filling are mutually exclusive at the type level:
/// a widget placed with [`Placement::Fill`] cannot have a separate alignment configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// Span the layout's available extent on that axis. Sizing and alignment are resolved
    /// automatically to fill the parent space.
    Fill,
    /// Sized widget with a specific alignment configuration.
    Sized { size: Size, align: Align },
}

impl Placement {
    /// Create a fixed size placement with default (`Start`) alignment.
    pub fn fixed(px: f32) -> Self {
        Placement::Sized {
            size: Size::Fixed(px),
            align: Align::Start,
        }
    }

    /// Create an auto size placement with default (`Start`) alignment.
    pub fn auto() -> Self {
        Placement::Sized {
            size: Size::Auto,
            align: Align::Start,
        }
    }

    /// Create a fill placement.
    pub fn fill() -> Self {
        Placement::Fill
    }

    /// Update the alignment policy for this placement.
    ///
    /// # Panics
    ///
    /// Panics if called on a `Placement::Fill`, as fill and alignment are mutually exclusive.
    pub fn align(self, align: Align) -> Self {
        match self {
            Placement::Fill => {
                panic!("Layout panic: cannot set alignment on Placement::Fill as align + fill is unrepresentable");
            }
            Placement::Sized { size, .. } => Placement::Sized { size, align },
        }
    }

    pub(crate) fn resolve_size(self, intrinsic: Option<f32>, avail: AxisBound) -> f32 {
        let preferred = || {
            intrinsic.unwrap_or_else(|| {
                panic!(
                    "Layout panic: Placement sizing needs an intrinsic measurement on this \
                     axis but none was reported. A child placed with Auto must report a preferred size."
                )
            })
        };
        match self {
            Placement::Sized {
                size: Size::Fixed(px),
                ..
            } => px,
            Placement::Sized {
                size: Size::Auto, ..
            } => match avail {
                AxisBound::Exact(_) => preferred(),
                AxisBound::AtMost(w) => preferred().min(w),
                AxisBound::Unbounded => preferred(),
            },
            Placement::Fill => match avail {
                AxisBound::Exact(w) => w,
                AxisBound::AtMost(_) => panic!(
                    "Layout panic: Placement::Fill on an AtMost axis is unsatisfiable — AtMost \
                     provides a ceiling but no committed frame to fill. Use Placement::auto() if \
                     you want the intrinsic size (clamped to the ceiling), or place this in \
                     a bounded (Exact) container."
                ),
                AxisBound::Unbounded => panic!(
                    "Layout panic: Placement::Fill on an Unbounded axis is unsatisfiable — \
                     there is no bounded extent to fill into. Use Placement::auto() if you want \
                     the intrinsic size, or place this in a bounded (Exact) container."
                ),
            },
        }
    }

    pub(crate) fn align_offset(self, resolved: f32, avail: AxisBound) -> f32 {
        match self {
            Placement::Fill => 0.0,
            Placement::Sized { align, .. } => match align {
                Align::Start => 0.0,
                Align::Center => match avail {
                    AxisBound::Exact(w) => (w - resolved) * 0.5,
                    AxisBound::AtMost(_) => panic!(
                        "Layout panic: Align::Center requires AxisBound::Exact available space on the cross axis, but width is AtMost"
                    ),
                    AxisBound::Unbounded => panic!(
                        "Layout panic: Align::Center requires AxisBound::Exact available space on the cross axis, but width is Unbounded"
                    ),
                },
                Align::End => match avail {
                    AxisBound::Exact(w) => w - resolved,
                    AxisBound::AtMost(_) => panic!(
                        "Layout panic: Align::End requires AxisBound::Exact available space on the cross axis, but width is AtMost"
                    ),
                    AxisBound::Unbounded => panic!(
                        "Layout panic: Align::End requires AxisBound::Exact available space on the cross axis, but width is Unbounded"
                    ),
                },
            },
        }
    }
}

/// A per-axis sizing and alignment request a caller hands to an intrinsic-aware layout
/// (column/row/wrap). Axes are absolute (width/height), not main/cross, so the
/// same request reads identically regardless of layout orientation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement2D {
    pub width: Placement,
    pub height: Placement,
}

impl Placement2D {
    /// Both axes fixed to explicit pixels.
    pub fn fixed(w: f32, h: f32) -> Self {
        Self {
            width: Placement::fixed(w),
            height: Placement::fixed(h),
        }
    }

    /// Both axes sized to the widget's intrinsic preferred size.
    pub fn auto() -> Self {
        Self {
            width: Placement::auto(),
            height: Placement::auto(),
        }
    }

    /// Set alignment on the horizontal (width) axis.
    pub fn align_x(mut self, align: Align) -> Self {
        self.width = self.width.align(align);
        self
    }

    /// Set alignment on the vertical (height) axis.
    pub fn align_y(mut self, align: Align) -> Self {
        self.height = self.height.align(align);
        self
    }
}

/// Back-compat: a plain size is treated as fixed on both axes.
impl From<Vec2> for Placement2D {
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
        // Placement::fixed is always fixed
        assert_eq!(
            Placement::fixed(50.0).resolve_size(Some(30.0), AxisBound::AtMost(100.0)),
            50.0
        );
        assert_eq!(
            Placement::fixed(150.0).resolve_size(Some(30.0), AxisBound::AtMost(100.0)),
            150.0
        );

        // Placement::auto uses preferred, but caps at AtMost
        assert_eq!(
            Placement::auto().resolve_size(Some(40.0), AxisBound::AtMost(100.0)),
            40.0
        );
        assert_eq!(
            Placement::auto().resolve_size(Some(120.0), AxisBound::AtMost(100.0)),
            100.0
        );
    }

    #[test]
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_auto_resolve_without_intrinsic_panics() {
        // Auto with no measured preferred size is unsatisfiable.
        let _ = Placement::auto().resolve_size(None, AxisBound::AtMost(80.0));
    }

    #[test]
    #[should_panic(expected = "Fill on an AtMost axis is unsatisfiable")]
    fn test_fill_resolve_at_most_panics() {
        let _ = Placement::fill().resolve_size(Some(40.0), AxisBound::AtMost(80.0));
    }

    #[test]
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_fill_resolve_unbounded_panics() {
        let _ = Placement::fill().resolve_size(Some(18.0), AxisBound::Unbounded);
    }

    #[test]
    fn test_align_offset_exact() {
        assert_eq!(
            Placement::fill().align_offset(40.0, AxisBound::Exact(100.0)),
            0.0
        );
        assert_eq!(
            Placement::fixed(40.0).align_offset(40.0, AxisBound::Exact(100.0)),
            0.0
        );
        assert_eq!(
            Placement::fixed(40.0)
                .align(Align::Center)
                .align_offset(40.0, AxisBound::Exact(100.0)),
            30.0
        );
        assert_eq!(
            Placement::fixed(40.0)
                .align(Align::End)
                .align_offset(40.0, AxisBound::Exact(100.0)),
            60.0
        );
    }

    #[test]
    #[should_panic(expected = "Align::Center requires AxisBound::Exact")]
    fn test_align_center_panic_on_at_most() {
        let _ = Placement::fixed(40.0)
            .align(Align::Center)
            .align_offset(40.0, AxisBound::AtMost(100.0));
    }

    #[test]
    #[should_panic(expected = "Align::End requires AxisBound::Exact")]
    fn test_align_end_panic_on_unbounded() {
        let _ = Placement::fixed(40.0)
            .align(Align::End)
            .align_offset(40.0, AxisBound::Unbounded);
    }
}
