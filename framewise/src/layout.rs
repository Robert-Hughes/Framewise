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
        let preferred = intrinsic_axis.unwrap_or(fallback);
        match self {
            Extent::Fixed(px) => px,
            Extent::Auto => match fill {
                AxisBound::Exact(_w) => preferred,
                AxisBound::AtMost(w) => preferred.min(w),
                AxisBound::Unbounded => preferred,
            },
            Extent::Fill => match fill {
                AxisBound::Exact(w) => w,
                // Position & distribution policies — fill, right-align, center,
                // space-between — require Exact: a committed frame with a far edge.
                // AtMost and Unbounded permit only measurement / shrink-wrap decisions,
                // so we fall back to Auto.
                AxisBound::AtMost(w) => preferred.min(w),
                AxisBound::Unbounded => preferred,
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

// ── ManualLayout ──────────────────────────────────────────────────────────

/// A layout that requires exact Rects for every widget.
pub struct ManualLayout;

impl Layout for ManualLayout {
    type Params = Rect;
    type State = ManualState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        ManualState {
            space: space.into(),
            content_extent: Vec2::ZERO,
        }
    }
}

pub struct ManualState {
    space: LayoutSpace,
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
            self.space.x + layout_params.x,
            self.space.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        )
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Rect,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let space = LayoutSpace::new(
            self.space.x + layout_params.x,
            self.space.y + layout_params.y,
            AxisBound::Exact(layout_params.w),
            AxisBound::Exact(layout_params.h),
        );
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: Rect, _extent: Vec2) -> Rect {
        self.content_extent.x = self.content_extent.x.max(layout_params.x + layout_params.w);
        self.content_extent.y = self.content_extent.y.max(layout_params.y + layout_params.h);
        Rect::new(
            self.space.x + layout_params.x,
            self.space.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        )
    }

    fn resolve_space(&self) -> Rect {
        self.space.resolve(self.content_extent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CrossAlign {
    #[default]
    Start,
    Center,
    End,
}

// ── ColumnLayout ──────────────────────────────────────────────────────────

pub struct ColumnLayout {
    pub spacing: f32,
    pub align: CrossAlign,
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
            align: self.align,
            content_w: 0.0,
            content_h: 0.0,
        }
    }
}

pub struct ColumnState {
    space: LayoutSpace,
    spacing: f32,
    current_y: f32,
    align: CrossAlign,
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
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        let x = match self.align {
            CrossAlign::Start => self.space.x,
            CrossAlign::Center => match self.space.width {
                AxisBound::Exact(width) => self.space.x + (width - w) * 0.5,
                AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but width is AtMost"),
                AxisBound::Unbounded => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but width is Unbounded"),
            },
            CrossAlign::End => match self.space.width {
                AxisBound::Exact(width) => self.space.x + width - w,
                AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but width is AtMost"),
                AxisBound::Unbounded => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but width is Unbounded"),
            },
        };

        let r = Rect::new(x, self.current_y, w, h);
        self.content_w = self.content_w.max((x + w) - self.space.x);
        self.content_h = (self.current_y + h) - self.space.y;
        self.current_y += h + self.spacing;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: SizeReq,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let width = match layout_params.width {
            Extent::Fixed(w) => AxisBound::Exact(w),
            Extent::Fill => self.space.width,
            Extent::Auto => match self.space.width {
                AxisBound::Exact(w) | AxisBound::AtMost(w) => AxisBound::AtMost(w),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let height = match layout_params.height {
            Extent::Fixed(h) => AxisBound::Exact(h),
            Extent::Fill => match self.space.height {
                AxisBound::Exact(h) => {
                    AxisBound::Exact((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::AtMost(h) => {
                    AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
            Extent::Auto => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => {
                    AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let w = match layout_params.width {
            Extent::Fixed(w) => Some(w),
            Extent::Fill => match self.space.width {
                AxisBound::Exact(w) => Some(w),
                AxisBound::AtMost(_) | AxisBound::Unbounded => None,
            },
            Extent::Auto => None,
        };

        let x = match self.align {
            CrossAlign::Start => self.space.x,
            CrossAlign::Center => match w {
                Some(val) => match self.space.width {
                    AxisBound::Exact(width) => self.space.x + (width - val) * 0.5,
                    AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but width is AtMost"),
                    AxisBound::Unbounded => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but width is Unbounded"),
                },
                None => panic!("Layout panic: CrossAlign::Center cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
            CrossAlign::End => match w {
                Some(val) => match self.space.width {
                    AxisBound::Exact(width) => self.space.x + width - val,
                    AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but width is AtMost"),
                    AxisBound::Unbounded => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but width is Unbounded"),
                },
                None => panic!("Layout panic: CrossAlign::End cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
        };

        let space = LayoutSpace::new(x, self.current_y, width, height);
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: SizeReq, extent: Vec2) -> Rect {
        let pref = Some(extent);
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        let x = match self.align {
            CrossAlign::Start => self.space.x,
            CrossAlign::Center => match self.space.width {
                AxisBound::Exact(width) => self.space.x + (width - w) * 0.5,
                _ => unreachable!("Panicked in begin_layout"),
            },
            CrossAlign::End => match self.space.width {
                AxisBound::Exact(width) => self.space.x + width - w,
                _ => unreachable!("Panicked in begin_layout"),
            },
        };

        let r = Rect::new(x, self.current_y, w, h);
        self.content_w = self.content_w.max((x + w) - self.space.x);
        self.content_h = (self.current_y + h) - self.space.y;
        self.current_y += h + self.spacing;
        r
    }

    fn resolve_space(&self) -> Rect {
        self.space
            .resolve(Vec2::new(self.content_w, self.content_h))
    }
}

// ── RowLayout ─────────────────────────────────────────────────────────────

pub struct RowLayout {
    pub spacing: f32,
    pub align: CrossAlign,
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
            align: self.align,
            content_w: 0.0,
            content_h: 0.0,
        }
    }
}

pub struct RowState {
    space: LayoutSpace,
    spacing: f32,
    current_x: f32,
    align: CrossAlign,
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
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        let y = match self.align {
            CrossAlign::Start => self.space.y,
            CrossAlign::Center => match self.space.height {
                AxisBound::Exact(height) => self.space.y + (height - h) * 0.5,
                AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but height is AtMost"),
                AxisBound::Unbounded => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but height is Unbounded"),
            },
            CrossAlign::End => match self.space.height {
                AxisBound::Exact(height) => self.space.y + height - h,
                AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but height is AtMost"),
                AxisBound::Unbounded => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but height is Unbounded"),
            },
        };

        let r = Rect::new(self.current_x, y, w, h);
        self.content_w = (self.current_x + w) - self.space.x;
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.current_x += w + self.spacing;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: SizeReq,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let width = match layout_params.width {
            Extent::Fixed(w) => AxisBound::Exact(w),
            Extent::Fill => match self.space.width {
                AxisBound::Exact(w) => {
                    AxisBound::Exact((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::AtMost(w) => {
                    AxisBound::AtMost((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
            Extent::Auto => match self.space.width {
                AxisBound::Exact(w) | AxisBound::AtMost(w) => {
                    AxisBound::AtMost((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let height = match layout_params.height {
            Extent::Fixed(h) => AxisBound::Exact(h),
            Extent::Fill => self.space.height,
            Extent::Auto => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => AxisBound::AtMost(h),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let h = match layout_params.height {
            Extent::Fixed(h) => Some(h),
            Extent::Fill => match self.space.height {
                AxisBound::Exact(h) => Some(h),
                AxisBound::AtMost(_) | AxisBound::Unbounded => None,
            },
            Extent::Auto => None,
        };

        let y = match self.align {
            CrossAlign::Start => self.space.y,
            CrossAlign::Center => match h {
                Some(val) => match self.space.height {
                    AxisBound::Exact(height) => self.space.y + (height - val) * 0.5,
                    AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but height is AtMost"),
                    AxisBound::Unbounded => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis, but height is Unbounded"),
                },
                None => panic!("Layout panic: CrossAlign::Center cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
            CrossAlign::End => match h {
                Some(val) => match self.space.height {
                    AxisBound::Exact(height) => self.space.y + height - val,
                    AxisBound::AtMost(_) => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but height is AtMost"),
                    AxisBound::Unbounded => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis, but height is Unbounded"),
                },
                None => panic!("Layout panic: CrossAlign::End cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
        };

        let space = LayoutSpace::new(self.current_x, y, width, height);
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: SizeReq, extent: Vec2) -> Rect {
        let pref = Some(extent);
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        let y = match self.align {
            CrossAlign::Start => self.space.y,
            CrossAlign::Center => match self.space.height {
                AxisBound::Exact(height) => self.space.y + (height - h) * 0.5,
                _ => unreachable!("Panicked in begin_layout"),
            },
            CrossAlign::End => match self.space.height {
                AxisBound::Exact(height) => self.space.y + height - h,
                _ => unreachable!("Panicked in begin_layout"),
            },
        };

        let r = Rect::new(self.current_x, y, w, h);
        self.content_w = (self.current_x + w) - self.space.x;
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.current_x += w + self.spacing;
        r
    }

    fn resolve_space(&self) -> Rect {
        self.space
            .resolve(Vec2::new(self.content_w, self.content_h))
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

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
        intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let (mut space, _) = self.inner.begin_layout(layout_params.clone(), intrinsic);
        space.x -= self.offset.x;
        space.y -= self.offset.y;
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: Self::Params, extent: Vec2) -> Rect {
        let mut r = self.inner.end_layout(layout_params, extent);
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }

    fn resolve_space(&self) -> Rect {
        let mut r = self.inner.resolve_space();
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
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
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        // Wrap before placing if this item would overflow the line — but never
        // wrap an item that is already at the start of a line (it just clips).
        // An unbounded width has no edge to overflow, so the flow never wraps.
        let at_line_start = self.current_x == self.space.x;
        let overflows = match self.space.width {
            AxisBound::Exact(width) | AxisBound::AtMost(width) => {
                self.current_x + w > self.space.x + width
            }
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

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: SizeReq,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let w = match layout_params.width {
            Extent::Fixed(w) => w,
            Extent::Fill => match self.space.width {
                AxisBound::Exact(w) => (w - (self.current_x - self.space.x)).max(0.0),
                AxisBound::AtMost(_) | AxisBound::Unbounded => panic!("Layout panic: WrapLayout cannot resolve Extent::Fill under non-Exact bounds in begin_layout"),
            },
            Extent::Auto => panic!("Layout panic: WrapLayout does not support Auto-sized deferred containers because wrapping must be resolved in begin_layout"),
        };

        let width = AxisBound::Exact(w);

        let height = match layout_params.height {
            Extent::Fixed(h) => AxisBound::Exact(h),
            Extent::Fill | Extent::Auto => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => {
                    AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let at_line_start = self.current_x == self.space.x;
        let overflows = match self.space.width {
            AxisBound::Exact(width) | AxisBound::AtMost(width) => {
                self.current_x + w > self.space.x + width
            }
            AxisBound::Unbounded => false,
        };
        if !at_line_start && overflows {
            self.current_x = self.space.x;
            self.current_y += self.line_height + self.line_spacing;
            self.line_height = 0.0;
        }

        let space = LayoutSpace::new(self.current_x, self.current_y, width, height);
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: SizeReq, extent: Vec2) -> Rect {
        let pref = Some(extent);
        let w = layout_params.width.resolve(
            pref.map(|p| p.x),
            self.space.width,
            LAYOUT_FALLBACK_SIZE.x,
        );
        let h = layout_params.height.resolve(
            pref.map(|p| p.y),
            self.space.height,
            LAYOUT_FALLBACK_SIZE.y,
        );

        let r = Rect::new(self.current_x, self.current_y, w, h);
        self.content_w = self.content_w.max((self.current_x + w) - self.space.x);
        self.current_x += w + self.spacing;
        self.line_height = self.line_height.max(h);
        r
    }

    fn resolve_space(&self) -> Rect {
        // Width: the widest line. Height: the bottom of the current (last) line.
        let measured = Vec2::new(
            self.content_w,
            (self.current_y + self.line_height) - self.space.y,
        );
        self.space.resolve(measured)
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
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_row_layout() {
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        let r2 = state.layout(Vec2::new(20.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    fn test_column_auto_uses_intrinsic_preferred() {
        let mut state = ColumnLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 200.0, 500.0));
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
        let mut state = ColumnLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(5.0, 0.0, 200.0, 500.0));
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
        let mut state = RowLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let r = state.layout(SizeReq::auto(), IntrinsicSize::UNKNOWN);
        // No intrinsic reported → both axes use the global fallback.
        assert_eq!(
            r,
            Rect::new(0.0, 0.0, LAYOUT_FALLBACK_SIZE.x, LAYOUT_FALLBACK_SIZE.y)
        );
    }

    #[test]
    fn test_row_auto_width_advances_cursor() {
        let mut state = RowLayout {
            spacing: 6.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 400.0, 50.0));
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
        let r = state.layout(
            SizeReq::auto(),
            IntrinsicSize::preferred(Vec2::new(80.0, 16.0)),
        );
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
                width: AxisBound::Exact(30.0),
                height: AxisBound::Exact(40.0),
            }
        );
    }

    #[test]
    fn test_column_unbounded_height_resolves_concrete() {
        // Rule 2: a child laid out in an unbounded main axis still resolves to a
        // concrete Rect, and the cursor advances by a concrete f32.
        let mut state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::Start,
        }
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
        let mut state = ColumnLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
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
        let mut state = RowLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(LayoutSpace::unbounded_width(0.0, 0.0, 40.0));
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
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(5.0, 7.0, 100.0, 500.0));
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(60.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // Width = resolved Exact = 100.0, height = resolved Exact = 500.0
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
    }

    #[test]
    fn test_row_content_extent() {
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(20.0, 40.0).into(), IntrinsicSize::UNKNOWN);
        // Width = resolved Exact = 400.0; height = resolved Exact = 100.0.
        assert_eq!(state.resolve_space(), Rect::new(0.0, 0.0, 400.0, 100.0));
    }

    #[test]
    fn test_manual_content_extent() {
        let parent_space =
            LayoutSpace::new(100.0, 100.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ManualLayout.begin(parent_space);
        state.layout(Rect::new(0.0, 0.0, 50.0, 20.0), IntrinsicSize::UNKNOWN);
        state.layout(Rect::new(80.0, 40.0, 30.0, 30.0), IntrinsicSize::UNKNOWN);
        // Resolved space origin is 100, 100; extent is max far edges (110, 70).
        assert_eq!(state.resolve_space(), Rect::new(100.0, 100.0, 110.0, 70.0));
    }

    #[test]
    fn test_manual_layout_resolve_space_axis_bounds() {
        let parent_space_exact =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(200.0), AxisBound::Exact(150.0));
        let parent_space_at_most = LayoutSpace::new(
            10.0,
            20.0,
            AxisBound::AtMost(200.0),
            AxisBound::AtMost(150.0),
        );
        let parent_space_unbounded =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Unbounded);

        // 1. Exact -> Expected: exact bounds even if widgets are smaller or larger.
        // The layout space determines the value entirely (Exact overrides any laid out widget size).
        {
            // Smaller: Placed child (50x40) is smaller than 200x150 bounds.
            // Layout space (Exact) determines the resolved size (forces 200x150).
            let mut state = ManualLayout.begin(parent_space_exact);
            state.layout(Rect::new(0.0, 0.0, 50.0, 40.0), IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));

            // Larger: Placed child far edge (300, 260) exceeds 200x150 bounds.
            // Layout space (Exact) determines the resolved size (clamps/forces 200x150).
            let mut state = ManualLayout.begin(parent_space_exact);
            state.layout(Rect::new(50.0, 60.0, 250.0, 200.0), IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));
        }

        // 2. AtMost -> Expected: shrink-wrapped to child boundaries if smaller, capped if larger.
        // Both the widget sizes and the layout space limit determine the final value.
        {
            // Smaller: Placed child far edge (60, 50) is within the 200x150 limits.
            // Placed widgets determine the value (shrink-wrapped).
            let mut state = ManualLayout.begin(parent_space_at_most);
            state.layout(Rect::new(10.0, 10.0, 50.0, 40.0), IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 60.0, 50.0));

            // Larger: Placed child far edge (300, 260) exceeds the 200x150 limits.
            // Layout space (AtMost limit) determines the value (clamps at limit ceilings).
            let mut state = ManualLayout.begin(parent_space_at_most);
            state.layout(Rect::new(50.0, 60.0, 250.0, 200.0), IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));
        }

        // 3. Unbounded -> Expected: shrink-wrapped to child boundaries (max far edges).
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            // Placed child far edge is (300, 260).
            // Placed widgets determine the value.
            let mut state = ManualLayout.begin(parent_space_unbounded);
            state.layout(Rect::new(50.0, 60.0, 250.0, 200.0), IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 300.0, 260.0));
        }
    }

    #[test]
    fn test_offset_content_extent_ignores_offset() {
        let offset = OffsetLayout {
            offset: Vec2::new(13.0, 27.0),
            inner: ColumnLayout {
                spacing: 0.0,
                align: CrossAlign::Start,
            },
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // resolved_space shifted by offset (origin: -13.0, -27.0)
        assert_eq!(state.resolve_space(), Rect::new(-13.0, -27.0, 100.0, 100.0));
    }

    #[test]
    fn test_offset_layout() {
        let offset = OffsetLayout {
            offset: Vec2::new(5.0, 15.0),
            inner: ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            },
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

    #[test]
    fn test_at_most_resolution() {
        let fallback = 96.0;

        // Extent::Fixed is always fixed
        assert_eq!(
            Extent::Fixed(50.0).resolve(Some(30.0), AxisBound::AtMost(100.0), fallback),
            50.0
        );
        assert_eq!(
            Extent::Fixed(150.0).resolve(Some(30.0), AxisBound::AtMost(100.0), fallback),
            150.0
        );

        // Extent::Auto uses preferred, but caps at AtMost
        assert_eq!(
            Extent::Auto.resolve(Some(40.0), AxisBound::AtMost(100.0), fallback),
            40.0
        );
        assert_eq!(
            Extent::Auto.resolve(Some(120.0), AxisBound::AtMost(100.0), fallback),
            100.0
        );
        assert_eq!(
            Extent::Auto.resolve(None, AxisBound::AtMost(80.0), fallback),
            80.0
        );

        // Extent::Fill acts as Auto under AtMost
        assert_eq!(
            Extent::Fill.resolve(Some(40.0), AxisBound::AtMost(100.0), fallback),
            40.0
        );
        assert_eq!(
            Extent::Fill.resolve(Some(120.0), AxisBound::AtMost(100.0), fallback),
            100.0
        );
        assert_eq!(
            Extent::Fill.resolve(None, AxisBound::AtMost(80.0), fallback),
            80.0
        );
    }

    #[test]
    fn test_column_cross_alignment_exact() {
        // Exact width layout space
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Exact(100.0), AxisBound::Unbounded);

        // Center alignment
        let mut center_state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::Center,
        }
        .begin(space);
        let r1 = center_state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
        // x = 10.0 + (100.0 - 40.0) * 0.5 = 40.0
        assert_eq!(r1, Rect::new(40.0, 10.0, 40.0, 20.0));

        // End alignment
        let mut end_state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::End,
        }
        .begin(space);
        let r2 = end_state.layout(SizeReq::fixed(30.0, 20.0), IntrinsicSize::UNKNOWN);
        // x = 10.0 + 100.0 - 30.0 = 80.0
        assert_eq!(r2, Rect::new(80.0, 10.0, 30.0, 20.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let mut state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::Center,
        }
        .begin(space);
        let _ = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::End,
        }
        .begin(space);
        let _ = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_row_cross_alignment_exact() {
        // Exact height layout space
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Exact(80.0));

        // Center alignment
        let mut center_state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Center,
        }
        .begin(space);
        let r1 = center_state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
        // y = 10.0 + (80.0 - 20.0) * 0.5 = 40.0
        assert_eq!(r1, Rect::new(10.0, 40.0, 40.0, 20.0));

        // End alignment
        let mut end_state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::End,
        }
        .begin(space);
        let r2 = end_state.layout(SizeReq::fixed(40.0, 30.0), IntrinsicSize::UNKNOWN);
        // y = 10.0 + 80.0 - 30.0 = 60.0
        assert_eq!(r2, Rect::new(10.0, 60.0, 40.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::AtMost(80.0));
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Center,
        }
        .begin(space);
        let _ = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::End,
        }
        .begin(space);
        let _ = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_deferred_layout_token_lifecycle() {
        let mut state = ColumnLayout {
            spacing: 8.0,
            align: CrossAlign::Center,
        }
        .begin(LayoutSpace::new(
            0.0,
            0.0,
            AxisBound::Exact(200.0),
            AxisBound::Unbounded,
        ));

        // 1. Begin layout for a fit container
        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Auto,
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Space should start at current_y = 0.0, with unbounded height and exact width
        assert_eq!(space.x, 0.0);
        assert_eq!(space.y, 0.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));
        assert_eq!(space.height, AxisBound::Unbounded);

        // 2. End layout with the child's computed extent (say 80x50)
        let resolved_rect = token.end_layout(Vec2::new(80.0, 50.0));

        // Center aligned: x = (200 - 200) * 0.5 = 0.0 because width is Extent::Fill which resolves to 200.0!
        // Height is Extent::Auto which resolves to the child's height 50.0.
        assert_eq!(resolved_rect, Rect::new(0.0, 0.0, 200.0, 50.0));

        // Cursor should have advanced by height (50.0) + spacing (8.0) = 58.0
        let next_rect = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(next_rect.y, 58.0);
    }

    #[test]
    fn test_deferred_manual_layout_lifecycle() {
        let parent_space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ManualLayout.begin(parent_space);
        let layout_param = Rect::new(20.0, 30.0, 50.0, 40.0);
        let (space, token) = state.begin_layout(layout_param, IntrinsicSize::UNKNOWN);

        // ManualLayout begins at logically shifted coordinate: (10.0+20.0, 10.0+30.0) = (30.0, 40.0)
        assert_eq!(space.x, 30.0);
        assert_eq!(space.y, 40.0);
        assert_eq!(space.width, AxisBound::Exact(50.0));
        assert_eq!(space.height, AxisBound::Exact(40.0));

        let resolved_rect = token.end_layout(Vec2::new(15.0, 15.0));
        // Resolved rect should be exactly the requested rect shifted by origin
        assert_eq!(resolved_rect, Rect::new(30.0, 40.0, 50.0, 40.0));

        // Resolved space origin is 10.0, 10.0; extent is max far edges (70.0, 70.0)
        assert_eq!(state.resolve_space(), Rect::new(10.0, 10.0, 70.0, 70.0));
    }

    #[test]
    fn test_deferred_row_layout_lifecycle() {
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Center,
        }
        .begin(LayoutSpace::new(
            10.0,
            10.0,
            AxisBound::Unbounded,
            AxisBound::Exact(100.0),
        ));

        let req = SizeReq {
            width: Extent::Auto,
            height: Extent::Fill,
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Space should start at current_x = 10.0, with unbounded width and exact height
        assert_eq!(space.x, 10.0);
        assert_eq!(space.y, 10.0);
        assert_eq!(space.width, AxisBound::Unbounded);
        assert_eq!(space.height, AxisBound::Exact(100.0));

        // Consume token with extent 60x40.
        // Width is Extent::Auto -> 60.0.
        // Height is Extent::Fill -> resolves to space.height (Exact 100.0).
        let resolved_rect = token.end_layout(Vec2::new(60.0, 40.0));
        assert_eq!(resolved_rect, Rect::new(10.0, 10.0, 60.0, 100.0));

        // Cursor should have advanced by width (60.0) + spacing (5.0) = 65.0, so next starts at 75.0
        let next_rect = state.layout(SizeReq::fixed(30.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(next_rect.x, 75.0);
        // Center aligned: y = 10.0 + (100.0 - 20.0) * 0.5 = 50.0
        assert_eq!(next_rect.y, 50.0);
    }

    #[test]
    fn test_deferred_wrap_layout_lifecycle() {
        let mut state = WrapLayout {
            spacing: 10.0,
            line_spacing: 5.0,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 200.0));

        // Place first item normally
        let r1 = state.layout(SizeReq::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 40.0, 20.0)); // cursor x is now 40 + 10 = 50

        // Place a deferred item of width 40.0.
        let req1 = SizeReq {
            width: Extent::Fixed(40.0),
            height: Extent::Auto,
        };
        let (space, token) = state.begin_layout(req1, IntrinsicSize::UNKNOWN);
        // provisional space starts at current_x (50) and current_y (0)
        assert_eq!(space.x, 50.0);
        assert_eq!(space.y, 0.0);

        // child measures 40x30.
        // Item is placed at current_x = 50. x + w = 50 + 40 = 90 <= 100, no overflow.
        let resolved_rect = token.end_layout(Vec2::new(40.0, 30.0));
        assert_eq!(resolved_rect, Rect::new(50.0, 0.0, 40.0, 30.0)); // cursor x now 50 + 40 + 10 = 100. line_height = max(20, 30) = 30.

        // Place next item of width 20.0. Under 100px width limit, cursor x = 100. 100 + 20 = 120 > 100, so it wraps.
        let req2 = SizeReq {
            width: Extent::Fixed(20.0),
            height: Extent::Auto,
        };
        let (space2, token2) = state.begin_layout(req2, IntrinsicSize::UNKNOWN);
        // Under WrapLayout's upfront wrap resolution, space2 wraps to start of next line: (0.0, 35.0)
        assert_eq!(space2.x, 0.0);
        assert_eq!(space2.y, 35.0);

        // This item is width 20.
        let resolved_rect2 = token2.end_layout(Vec2::new(20.0, 15.0));
        assert_eq!(resolved_rect2, Rect::new(0.0, 35.0, 20.0, 15.0));
    }

    #[test]
    fn test_deferred_offset_layout_lifecycle() {
        let offset = OffsetLayout {
            offset: Vec2::new(10.0, 20.0),
            inner: ColumnLayout {
                spacing: 5.0,
                align: CrossAlign::Start,
            },
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));

        let req = SizeReq {
            width: Extent::Fixed(50.0),
            height: Extent::Auto,
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Provisional space should be shifted by offset: space.x = 0.0 - 10.0 = -10.0
        assert_eq!(space.x, -10.0);
        assert_eq!(space.y, -20.0);

        let resolved_rect = token.end_layout(Vec2::new(50.0, 40.0));
        // Rect resolved in inner layout is at (0, 0, 50, 40), then shifted: (-10, -20, 50, 40)
        assert_eq!(resolved_rect, Rect::new(-10.0, -20.0, 50.0, 40.0));
    }

    #[test]
    fn test_manual_begin_layout_propagates_exact_bounds() {
        let mut state = ManualLayout.begin(Rect::new(10.0, 20.0, 300.0, 400.0));
        let (space, _token) =
            state.begin_layout(Rect::new(5.0, 10.0, 100.0, 150.0), IntrinsicSize::UNKNOWN);
        assert_eq!(space.width, AxisBound::Exact(100.0));
        assert_eq!(space.height, AxisBound::Exact(150.0));
    }

    #[test]
    fn test_column_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Start,
        }
        .begin(parent_space);

        // 1. Fixed child height
        let req_fixed = SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(50.0),
        };
        let (space_f, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        assert_eq!(space_f.width, AxisBound::Exact(200.0));
        assert_eq!(space_f.height, AxisBound::Exact(50.0));

        // Place a child to advance cursor by 50 + spacing(10) = 60
        state.layout(SizeReq::fixed(200.0, 50.0), IntrinsicSize::UNKNOWN);

        // Remaining parent height is 300 - 60 = 240.
        // 2. Fill child height
        let req_fill = SizeReq {
            width: Extent::Auto,
            height: Extent::Fill,
        };
        let (space_fill, _token) = state.begin_layout(req_fill, IntrinsicSize::UNKNOWN);
        assert_eq!(space_fill.width, AxisBound::AtMost(200.0));
        assert_eq!(space_fill.height, AxisBound::Exact(240.0));

        // 3. Auto child height
        let req_auto = SizeReq {
            width: Extent::Auto,
            height: Extent::Auto,
        };
        let (space_auto, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        assert_eq!(space_auto.width, AxisBound::AtMost(200.0));
        assert_eq!(space_auto.height, AxisBound::AtMost(240.0));
    }

    #[test]
    fn test_column_begin_layout_under_parent_at_most() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(150.0), AxisBound::AtMost(250.0));
        let mut state = ColumnLayout {
            spacing: 5.0,
            align: CrossAlign::Start,
        }
        .begin(parent_space);

        // 1. Fill child width, Auto child height
        let req1 = SizeReq {
            width: Extent::Fill,
            height: Extent::Auto,
        };
        let (space1, _token) = state.begin_layout(req1, IntrinsicSize::UNKNOWN);
        // Fill width under parent AtMost(150) should propagate AtMost(150)
        assert_eq!(space1.width, AxisBound::AtMost(150.0));
        // Auto height under parent AtMost(250) should propagate AtMost(250)
        assert_eq!(space1.height, AxisBound::AtMost(250.0));

        // Advance cursor by 40 + spacing(5) = 45
        state.layout(SizeReq::fixed(100.0, 40.0), IntrinsicSize::UNKNOWN);

        // 2. Fill child height (remaining = 250 - 45 = 205)
        let req2 = SizeReq {
            width: Extent::Auto,
            height: Extent::Fill,
        };
        let (space2, _token) = state.begin_layout(req2, IntrinsicSize::UNKNOWN);
        // Auto width under parent AtMost(150) should yield AtMost(150)
        assert_eq!(space2.width, AxisBound::AtMost(150.0));
        // Fill height under parent AtMost should yield AtMost(205)
        assert_eq!(space2.height, AxisBound::AtMost(205.0));
    }

    #[test]
    fn test_row_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(400.0), AxisBound::Exact(150.0));
        let mut state = RowLayout {
            spacing: 5.0,
            align: CrossAlign::Start,
        }
        .begin(parent_space);

        // Fixed width
        let req_fixed = SizeReq {
            width: Extent::Fixed(80.0),
            height: Extent::Fill,
        };
        let (space_f, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        assert_eq!(space_f.width, AxisBound::Exact(80.0));
        assert_eq!(space_f.height, AxisBound::Exact(150.0));

        // Advance cursor by 80 + spacing(5) = 85
        state.layout(SizeReq::fixed(80.0, 100.0), IntrinsicSize::UNKNOWN);

        // Remaining parent width is 400 - 85 = 315
        // Auto width, fill height
        let req_auto = SizeReq {
            width: Extent::Auto,
            height: Extent::Fill,
        };
        let (space_auto, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        assert_eq!(space_auto.width, AxisBound::AtMost(315.0));
        assert_eq!(space_auto.height, AxisBound::Exact(150.0));
    }

    #[test]
    fn test_wrap_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(250.0), AxisBound::Exact(200.0));
        let mut state = WrapLayout {
            spacing: 10.0,
            line_spacing: 5.0,
        }
        .begin(parent_space);

        // Place initial item of width 100, height 40 -> current_x is 110, line_height is 40
        state.layout(SizeReq::fixed(100.0, 40.0), IntrinsicSize::UNKNOWN);

        // Remaining width on this line is 250 - 110 = 140.
        // Fixed width, auto height child container.
        let req = SizeReq {
            width: Extent::Fixed(80.0),
            height: Extent::Auto,
        };
        let (space, _token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        assert_eq!(space.width, AxisBound::Exact(80.0));
        assert_eq!(space.height, AxisBound::AtMost(200.0));
    }

    #[test]
    fn test_deferred_column_center_align_fixed() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Center,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Fixed(80.0),
            height: Extent::Fixed(40.0),
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        // x = 10.0 + (200.0 - 80.0) * 0.5 = 70.0
        assert_eq!(space.x, 70.0);
        assert_eq!(space.width, AxisBound::Exact(80.0));

        let rect = token.end_layout(Vec2::new(80.0, 40.0));
        assert_eq!(rect, Rect::new(70.0, 10.0, 80.0, 40.0));
    }

    #[test]
    fn test_deferred_column_end_align_fill() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::End,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(40.0),
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        // w resolves to 200.0 (Fill under Exact 200)
        // x = 10.0 + 200.0 - 200.0 = 10.0
        assert_eq!(space.x, 10.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));

        let rect = token.end_layout(Vec2::new(200.0, 40.0));
        assert_eq!(rect, Rect::new(10.0, 10.0, 200.0, 40.0));
    }

    #[test]
    fn test_deferred_column_start_align_auto() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Start,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Auto,
            height: Extent::Fixed(40.0),
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        // Start alignment allows Auto size. x should be parent space x = 10.0.
        assert_eq!(space.x, 10.0);
        assert_eq!(space.width, AxisBound::AtMost(200.0));

        let rect = token.end_layout(Vec2::new(80.0, 40.0));
        // Under Start, w resolves to preferred size 80.0. x is 10.0.
        assert_eq!(rect, Rect::new(10.0, 10.0, 80.0, 40.0));
    }

    #[test]
    #[should_panic(expected = "cannot align dynamic")]
    fn test_deferred_column_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout {
            spacing: 10.0,
            align: CrossAlign::Center,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Auto,
            height: Extent::Fixed(40.0),
        };
        // Auto width under Center alignment should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    #[should_panic(expected = "cannot align dynamic")]
    fn test_deferred_row_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = RowLayout {
            spacing: 10.0,
            align: CrossAlign::Center,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Fixed(80.0),
            height: Extent::Auto,
        };
        // Auto height under Center alignment in RowLayout should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    #[should_panic(expected = "does not support Auto-sized")]
    fn test_deferred_wrap_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = WrapLayout {
            spacing: 10.0,
            line_spacing: 5.0,
        }
        .begin(parent_space);

        let req = SizeReq {
            width: Extent::Auto,
            height: Extent::Fixed(40.0),
        };
        // Auto width under WrapLayout should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_column_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(400.0), AxisBound::Unbounded);

        // 1. CrossAlign::Center (standard layout)
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Center,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(180.0, 32.0);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box right edge = 10.0 + (400.0 - 180.0)/2 + 180.0 = 300.0.
            // Under resolve_space, the Exact(400.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // 2. CrossAlign::End (standard layout)
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::End,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(180.0, 32.0);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box right edge = 10.0 + 400.0 - 180.0 + 180.0 = 410.0.
            // Relative right edge = 410.0 - 10.0 = 400.0.
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // 3. CrossAlign::Center (deferred begin/end layout)
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Center,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(180.0, 32.0);
            let (_, token) = state.begin_layout(req.clone(), IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(180.0, 32.0));

            // Under resolve_space, the Exact(400.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().w, 400.0);
        }
    }

    #[test]
    fn test_row_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Exact(300.0));

        // 1. CrossAlign::Center (standard layout)
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Center,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(80.0, 100.0);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box bottom edge = 20.0 + (300.0 - 100.0)/2 + 100.0 = 220.0.
            // Under resolve_space, the Exact(300.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // 2. CrossAlign::End (standard layout)
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::End,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(80.0, 100.0);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box bottom edge = 20.0 + 300.0 - 100.0 + 100.0 = 320.0.
            // Relative bottom edge = 320.0 - 20.0 = 300.0.
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // 3. CrossAlign::Center (deferred begin/end layout)
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Center,
            }
            .begin(parent_space);
            let req = SizeReq::fixed(80.0, 100.0);
            let (_, token) = state.begin_layout(req.clone(), IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(80.0, 100.0));

            // Under resolve_space, the Exact(300.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().h, 300.0);
        }
    }

    #[test]
    fn test_column_layout_resolve_space_axis_bounds() {
        let parent_space_exact =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(400.0), AxisBound::Unbounded);
        let parent_space_at_most =
            LayoutSpace::new(10.0, 20.0, AxisBound::AtMost(400.0), AxisBound::Unbounded);
        let parent_space_at_most_overflow =
            LayoutSpace::new(10.0, 20.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let parent_space_unbounded =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Unbounded);

        let req = SizeReq::fixed(180.0, 32.0);

        // 1. Exact(400.0) -> Expected: exact bounds (400.0) even if widgets are smaller (180.0)
        // The layout space determines the value (Exact constraint overrides any smaller child size).
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // 2. AtMost(400.0) -> Expected: shrink-wrapped to child's actual width (180.0)
        // Placed widgets determine the value (since the child's size is within parent bounds).
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 180.0);
        }

        // 2b. AtMost(100.0) (widgets larger than AtMost value) -> Expected: capped at AtMost value (100.0)
        // The layout space determines the value (clamped at parent limit ceiling).
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 100.0);
        }

        // 3. Unbounded -> Expected: child's actual width (180.0)
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            let mut state = ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 180.0);
        }
    }

    #[test]
    fn test_row_layout_resolve_space_axis_bounds() {
        let parent_space_exact =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Exact(300.0));
        let parent_space_at_most =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::AtMost(300.0));
        let parent_space_at_most_overflow =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::AtMost(50.0));
        let parent_space_unbounded =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Unbounded);

        let req = SizeReq::fixed(80.0, 100.0);

        // 1. Exact(300.0) -> Expected: exact bounds (300.0) even if widgets are smaller (100.0)
        // The layout space determines the value (Exact constraint overrides any smaller child size).
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // 2. AtMost(300.0) -> Expected: shrink-wrapped to child's actual height (100.0)
        // Placed widgets determine the value (since the child's size is within parent bounds).
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 100.0);
        }

        // 2b. AtMost(50.0) (widgets larger than AtMost value) -> Expected: capped at AtMost value (50.0)
        // The layout space determines the value (clamped at parent limit ceiling).
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 50.0);
        }

        // 3. Unbounded -> Expected: child's actual height (100.0)
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            let mut state = RowLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 100.0);
        }
    }
}
