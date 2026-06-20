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

// ── Size requests ─────────────────────────────────────────────────────────

/// A widget's requested size, reported to a request-aware layout.
///
/// This is not final geometry. A layout may clamp, align, stretch, or otherwise
/// resolve this request according to its own layout params and available space.
/// "Fill", "grow", alignment, and weights are caller/layout policy and live in
/// layout params, not here.  Every field is optional: a
/// widget may know one axis (or none) and not the other.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SizeRequest {
    /// Smallest size below which content clips (e.g. the longest unbreakable word).
    pub min: Option<Vec2>,
    /// The preferred requested size (e.g. one line of text plus padding).
    pub preferred: Option<Vec2>,
    /// The largest useful size.
    pub max: Option<Vec2>,
}

impl SizeRequest {
    /// No requested size known on any axis.
    pub const UNKNOWN: Self = Self {
        min: None,
        preferred: None,
        max: None,
    };

    /// A request that reports only its preferred size.
    pub fn preferred(size: Vec2) -> Self {
        Self {
            preferred: Some(size),
            ..Self::UNKNOWN
        }
    }
}

/// One-dimensional alignment inside an available extent.
///
/// Layout placement uses this to align a widget inside parent layout space.
/// Widget content placement can also use it to align content inside a widget's
/// own content rect.
///
/// Default is [`Align::Start`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    /// Size based on the widget's preferred requested size.
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

/// Outcome of resolving a sizing/alignment constraint.
///
/// NOT a `std::result::Result` alias. Unlike `Result`, the failure arm
/// (`Fallback`) still carries a usable `T` — every layout query yields a
/// value; `Fallback` additionally reports that the request was unsatisfiable.
///
/// Do not glob-import the variants (`use LayoutResult::*`): `Ok` collides with
/// the prelude's `Result::Ok`. Always qualify as `LayoutResult::Ok`.
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutResult<T> {
    /// Constraint satisfiable — `value` is exact.
    Ok(T),
    /// Constraint unsatisfiable — `value` is a safe fallback; `violation` says why.
    Fallback {
        value: T,
        violation: LayoutViolation,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutViolation {
    pub kind: LayoutViolationKind,
    /// Call site. Captured via `#[track_caller]`. NOTE: today this resolves to
    /// the layout-internal caller (e.g. column.rs), not the user's widget call;
    /// see "track_caller scope" below.
    pub location: &'static core::panic::Location<'static>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutViolationKind {
    /// Center/End alignment with no committed frame (`AtMost`/`Unbounded`) to anchor against.
    UnsatisfiableAlignment { align: Align, bound: AxisBound },
    /// `Fill` with no bounded extent (`AtMost`/`Unbounded`) to fill into.
    UnsatisfiableFill { bound: AxisBound },
    /// `Auto` sizing but the widget reported no preferred requested size.
    MissingPreferredSize,
    /// Placing children in a closed linear layout (i.e. after a MainAxisAlign::End child).
    LayoutClosed,
    /// MainAxisAlign::End with no committed frame (AtMost/Unbounded) to anchor against on the main axis.
    UnsatisfiableMainAxisEnd { bound: AxisBound },
}

impl std::fmt::Display for LayoutViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            LayoutViolationKind::LayoutClosed => {
                write!(
                    f,
                    "Layout panic: layout is closed, no more children can be placed after a MainAxisAlign::End child"
                )
            }
            LayoutViolationKind::UnsatisfiableMainAxisEnd { bound } => {
                let bound_str = match bound {
                    AxisBound::Exact(_) => "Exact",
                    AxisBound::AtMost(_) => "AtMost",
                    AxisBound::Unbounded => "Unbounded",
                };
                write!(
                    f,
                    "Layout panic: MainAxisAlign::End requires AxisBound::Exact available space on the main axis, but is {bound_str}"
                )
            }
            LayoutViolationKind::MissingPreferredSize => {
                write!(
                    f,
                    "Layout panic: Placement sizing needs a preferred size request on this \
                     axis but none was reported. A child placed with Auto must report a preferred requested size."
                )
            }
            LayoutViolationKind::UnsatisfiableFill { bound } => match bound {
                AxisBound::AtMost(_) => write!(
                    f,
                    "Layout panic: Placement::Fill on an AtMost axis is unsatisfiable — AtMost \
                     provides a ceiling but no committed frame to fill. Use Placement::auto() if \
                     you want the requested size (clamped to the ceiling), or place this in \
                     a bounded (Exact) container."
                ),
                AxisBound::Unbounded => write!(
                    f,
                    "Layout panic: Placement::Fill on an Unbounded axis is unsatisfiable — \
                     there is no bounded extent to fill into. Use Placement::auto() if you want \
                     the requested size, or place this in a bounded (Exact) container."
                ),
                AxisBound::Exact(_) => {
                    unreachable!("UnsatisfiableFill never carries an Exact bound")
                }
            },
            LayoutViolationKind::UnsatisfiableAlignment { align, bound } => {
                let bound_str = match bound {
                    AxisBound::Exact(_) => "Exact",
                    AxisBound::AtMost(_) => "AtMost",
                    AxisBound::Unbounded => "Unbounded",
                };
                write!(
                    f,
                    "Layout panic: Align::{align:?} requires AxisBound::Exact available space on the cross axis, but is {bound_str}"
                )
            }
        }
    }
}

impl<T> LayoutResult<T> {
    /// Build from a value plus an optional violation (inverse of into_parts).
    pub fn from_parts(value: T, violation: Option<LayoutViolation>) -> Self {
        match violation {
            None => LayoutResult::Ok(value),
            Some(violation) => LayoutResult::Fallback { value, violation },
        }
    }

    /// Always returns the value, discarding any violation. The graceful path (steps 2–3).
    pub fn value(self) -> T {
        match self {
            LayoutResult::Ok(val) => val,
            LayoutResult::Fallback { value, .. } => value,
        }
    }

    /// Splits into (value, optional violation). For the reaction layer (steps 2–3).
    pub fn into_parts(self) -> (T, Option<LayoutViolation>) {
        match self {
            LayoutResult::Ok(val) => (val, None),
            LayoutResult::Fallback { value, violation } => (value, Some(violation)),
        }
    }

    pub fn violation(&self) -> Option<&LayoutViolation> {
        match self {
            LayoutResult::Ok(_) => None,
            LayoutResult::Fallback { violation, .. } => Some(violation),
        }
    }

    /// Panics on `Fallback`, rethrowing `LayoutViolation`'s Display. The step-1 path.
    #[track_caller]
    pub fn unwrap(self) -> T {
        match self {
            LayoutResult::Ok(val) => val,
            LayoutResult::Fallback { violation, .. } => {
                panic!("{}", violation);
            }
        }
    }

    /// Transform the value, preserving the variant/violation. For `space.x + offset`.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> LayoutResult<U> {
        match self {
            LayoutResult::Ok(val) => LayoutResult::Ok(f(val)),
            LayoutResult::Fallback { value, violation } => LayoutResult::Fallback {
                value: f(value),
                violation,
            },
        }
    }
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

    #[track_caller]
    pub(crate) fn resolve_size(
        self,
        intrinsic: Option<f32>,
        avail: AxisBound,
    ) -> LayoutResult<f32> {
        match self {
            Placement::Sized {
                size: Size::Fixed(px),
                ..
            } => LayoutResult::Ok(px),
            Placement::Sized {
                size: Size::Auto, ..
            } => match intrinsic {
                Some(preferred) => match avail {
                    AxisBound::Exact(_) => LayoutResult::Ok(preferred),
                    AxisBound::AtMost(w) => LayoutResult::Ok(preferred.min(w)),
                    AxisBound::Unbounded => LayoutResult::Ok(preferred),
                },
                None => LayoutResult::Fallback {
                    value: 0.0,
                    violation: LayoutViolation {
                        kind: LayoutViolationKind::MissingPreferredSize,
                        location: core::panic::Location::caller(),
                    },
                },
            },
            Placement::Fill => match avail {
                AxisBound::Exact(w) => LayoutResult::Ok(w),
                bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                    let val = match bound {
                        AxisBound::AtMost(w) => intrinsic.map(|i| i.min(w)).unwrap_or(0.0),
                        AxisBound::Unbounded => intrinsic.unwrap_or(0.0),
                        AxisBound::Exact(_) => unreachable!(),
                    };
                    LayoutResult::Fallback {
                        value: val,
                        violation: LayoutViolation {
                            kind: LayoutViolationKind::UnsatisfiableFill { bound },
                            location: core::panic::Location::caller(),
                        },
                    }
                }
            },
        }
    }

    #[track_caller]
    pub(crate) fn align_offset(self, resolved: f32, avail: AxisBound) -> LayoutResult<f32> {
        match self {
            Placement::Fill => LayoutResult::Ok(0.0),
            Placement::Sized { align, .. } => match align {
                Align::Start => LayoutResult::Ok(0.0),
                Align::Center => match avail {
                    AxisBound::Exact(w) => LayoutResult::Ok((w - resolved) * 0.5),
                    bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                        LayoutResult::Fallback {
                            value: 0.0,
                            violation: LayoutViolation {
                                kind: LayoutViolationKind::UnsatisfiableAlignment { align, bound },
                                location: core::panic::Location::caller(),
                            },
                        }
                    }
                },
                Align::End => match avail {
                    AxisBound::Exact(w) => LayoutResult::Ok(w - resolved),
                    bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                        LayoutResult::Fallback {
                            value: 0.0,
                            violation: LayoutViolation {
                                kind: LayoutViolationKind::UnsatisfiableAlignment { align, bound },
                                location: core::panic::Location::caller(),
                            },
                        }
                    }
                },
            },
        }
    }
}

/// A per-axis sizing and alignment request a caller hands to a request-aware layout
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

    /// Both axes sized to the widget's preferred requested size.
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
    pub fn end_layout(self, extent: Vec2) -> LayoutResult<Rect> {
        self.state.end_layout(self.params, extent)
    }
}

pub trait LayoutState {
    type Params: Clone;

    /// Calculate the screen-space rectangle for a widget given the caller's
    /// `layout_params` (intent) and the widget's size `request`.
    ///
    /// Layouts that don't size from content (e.g. `ManualLayout`) ignore
    /// `request`; request-aware layouts (column/row/wrap) read it.
    fn layout(&mut self, layout_params: Self::Params, request: SizeRequest) -> LayoutResult<Rect>;

    /// Begin a deferred layout (for fit-to-children containers).
    /// Returns a provisional [`LayoutSpace`] and a [`LayoutToken`] that borrows this layout state.
    ///
    /// `request` mirrors the same parameter on [`LayoutState::layout`]; layout
    /// implementations may use it (e.g. to enforce a minimum size floor) or ignore it.
    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
        request: SizeRequest,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>)
    where
        Self: Sized;

    /// End a deferred layout, providing the actual final accumulated content extent.
    /// Returns the resolved concrete Rect of the container and advances the layout state.
    fn end_layout(&mut self, layout_params: Self::Params, extent: Vec2) -> LayoutResult<Rect>;

    /// The resolved concrete space Rect occupied by the entire layout (which might be
    /// bounded or accumulated), measured relative to parent coordinates but independent
    /// of any temporary offsets.
    fn resolve_space(&self) -> Rect;
}

pub trait SpacerLayoutState: LayoutState {
    type SpacerParams;
    fn spacer(&mut self, params: Self::SpacerParams);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_request_constructors() {
        assert_eq!(SizeRequest::UNKNOWN, SizeRequest::default());
        let i = SizeRequest::preferred(Vec2::new(10.0, 20.0));
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
            Placement::fixed(50.0)
                .resolve_size(Some(30.0), AxisBound::AtMost(100.0))
                .unwrap(),
            50.0
        );
        assert_eq!(
            Placement::fixed(150.0)
                .resolve_size(Some(30.0), AxisBound::AtMost(100.0))
                .unwrap(),
            150.0
        );

        // Placement::auto uses preferred, but caps at AtMost
        assert_eq!(
            Placement::auto()
                .resolve_size(Some(40.0), AxisBound::AtMost(100.0))
                .unwrap(),
            40.0
        );
        assert_eq!(
            Placement::auto()
                .resolve_size(Some(120.0), AxisBound::AtMost(100.0))
                .unwrap(),
            100.0
        );
    }

    #[test]
    fn test_auto_resolve_without_intrinsic_falls_back() {
        let res = Placement::auto().resolve_size(None, AxisBound::AtMost(80.0));
        assert!(matches!(
            res,
            LayoutResult::Fallback {
                value: 0.0,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::MissingPreferredSize,
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_fill_resolve_at_most_falls_back() {
        let res = Placement::fill().resolve_size(Some(40.0), AxisBound::AtMost(80.0));
        assert!(matches!(
            res,
            LayoutResult::Fallback {
                value: 40.0,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::UnsatisfiableFill {
                        bound: AxisBound::AtMost(80.0)
                    },
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_fill_resolve_unbounded_falls_back() {
        let res = Placement::fill().resolve_size(Some(18.0), AxisBound::Unbounded);
        assert!(matches!(
            res,
            LayoutResult::Fallback {
                value: 18.0,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::UnsatisfiableFill {
                        bound: AxisBound::Unbounded
                    },
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_align_offset_exact() {
        assert_eq!(
            Placement::fill()
                .align_offset(40.0, AxisBound::Exact(100.0))
                .unwrap(),
            0.0
        );
        assert_eq!(
            Placement::fixed(40.0)
                .align_offset(40.0, AxisBound::Exact(100.0))
                .unwrap(),
            0.0
        );
        assert_eq!(
            Placement::fixed(40.0)
                .align(Align::Center)
                .align_offset(40.0, AxisBound::Exact(100.0))
                .unwrap(),
            30.0
        );
        assert_eq!(
            Placement::fixed(40.0)
                .align(Align::End)
                .align_offset(40.0, AxisBound::Exact(100.0))
                .unwrap(),
            60.0
        );
    }

    #[test]
    fn test_align_center_falls_back_on_at_most() {
        let res = Placement::fixed(40.0)
            .align(Align::Center)
            .align_offset(40.0, AxisBound::AtMost(100.0));
        assert!(matches!(
            res,
            LayoutResult::Fallback {
                value: 0.0,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::UnsatisfiableAlignment {
                        align: Align::Center,
                        bound: AxisBound::AtMost(100.0)
                    },
                    ..
                }
            }
        ));
    }

    #[test]
    fn test_align_end_falls_back_on_unbounded() {
        let res = Placement::fixed(40.0)
            .align(Align::End)
            .align_offset(40.0, AxisBound::Unbounded);
        assert!(matches!(
            res,
            LayoutResult::Fallback {
                value: 0.0,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::UnsatisfiableAlignment {
                        align: Align::End,
                        bound: AxisBound::Unbounded
                    },
                    ..
                }
            }
        ));
    }

    #[test]
    #[should_panic(expected = "Align::Center requires AxisBound::Exact")]
    fn test_unwrap_rethrow_lock() {
        let res = Placement::fixed(40.0)
            .align(Align::Center)
            .align_offset(40.0, AxisBound::AtMost(100.0));
        res.unwrap();
    }

    // ── Fallback-value sanity ───────────────────────────────────────────────
    // These lock the *values* carried by Fallback (consumed via `value()` in
    // steps 2–3), independent of the violation kind.

    #[test]
    fn test_fill_fallback_clamps_to_ceiling() {
        // Intrinsic exceeds the AtMost ceiling → fallback clamps to the ceiling,
        // never overflows it. (Distinct from the i < ceiling case, which would
        // pass even without the clamp.)
        let res = Placement::fill().resolve_size(Some(120.0), AxisBound::AtMost(80.0));
        let LayoutResult::Fallback { value, .. } = res else {
            panic!("expected Fallback, got {res:?}");
        };
        assert_eq!(value, 80.0);
    }

    #[test]
    fn test_fill_fallback_zero_without_intrinsic() {
        // No intrinsic to fall back on → 0.0 (finite, safe), not NaN/garbage.
        for bound in [AxisBound::AtMost(80.0), AxisBound::Unbounded] {
            let res = Placement::fill().resolve_size(None, bound);
            let LayoutResult::Fallback { value, .. } = res else {
                panic!("expected Fallback for {bound:?}, got {res:?}");
            };
            assert_eq!(value, 0.0, "bound {bound:?}");
        }
    }

    #[test]
    fn test_alignment_fallback_collapses_to_start() {
        // Alignment fallback is Start (offset 0) for every unsatisfiable
        // bound × align combo, regardless of the resolved child size.
        for bound in [AxisBound::AtMost(50.0), AxisBound::Unbounded] {
            for align in [Align::Center, Align::End] {
                let res = Placement::fixed(40.0)
                    .align(align)
                    .align_offset(999.0, bound);
                let LayoutResult::Fallback { value, .. } = res else {
                    panic!("expected Fallback for {align:?} on {bound:?}, got {res:?}");
                };
                assert_eq!(
                    value, 0.0,
                    "expected Start fallback for {align:?} on {bound:?}"
                );
            }
        }
    }
}
