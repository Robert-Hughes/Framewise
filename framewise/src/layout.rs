use crate::types::{Rect, Vec2};

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

pub trait Layout {
    type Params;
    type State: LayoutState<Params = Self::Params>;

    /// Initializes the mutable layout state, given the bounds allocated by the parent.
    fn begin(self, bounds: Rect) -> Self::State;
}

pub trait LayoutState {
    type Params;

    /// Calculate the screen-space rectangle for a widget given the parameters.
    fn layout(&mut self, layout_params: Self::Params) -> Rect;
}

// ── ManualLayout ──────────────────────────────────────────────────────────

/// A layout that requires exact Rects for every widget.
pub struct ManualLayout;

impl Layout for ManualLayout {
    type Params = Rect;
    type State = ManualState;

    fn begin(self, bounds: Rect) -> Self::State {
        ManualState { bounds }
    }
}

pub struct ManualState {
    bounds: Rect,
}

impl LayoutState for ManualState {
    type Params = Rect;

    fn layout(&mut self, layout_params: Rect) -> Rect {
        // Offset the requested rect by the layout's bounding box top-left.
        // This ensures if ManualLayout is nested inside a scroll view (or any other layout), the explicit rects
        // still shift correctly relative to the parent.
        Rect::new(
            self.bounds.x + layout_params.x,
            self.bounds.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        )
    }
}

// ── ColumnLayout ──────────────────────────────────────────────────────────

pub struct ColumnLayout {
    pub spacing: f32,
}

impl Layout for ColumnLayout {
    type Params = Vec2;
    type State = ColumnState;

    fn begin(self, bounds: Rect) -> Self::State {
        ColumnState {
            bounds,
            spacing: self.spacing,
            current_y: bounds.y,
        }
    }
}

pub struct ColumnState {
    bounds: Rect,
    spacing: f32,
    current_y: f32,
}

impl LayoutState for ColumnState {
    type Params = Vec2;

    fn layout(&mut self, layout_params: Vec2) -> Rect {
        let r = Rect::new(
            self.bounds.x,
            self.current_y,
            layout_params.x,
            layout_params.y,
        );
        self.current_y += layout_params.y + self.spacing;
        r
    }
}

// ── RowLayout ─────────────────────────────────────────────────────────────

pub struct RowLayout {
    pub spacing: f32,
}

impl Layout for RowLayout {
    type Params = Vec2;
    type State = RowState;

    fn begin(self, bounds: Rect) -> Self::State {
        RowState {
            bounds,
            spacing: self.spacing,
            current_x: bounds.x,
        }
    }
}

pub struct RowState {
    bounds: Rect,
    spacing: f32,
    current_x: f32,
}

impl LayoutState for RowState {
    type Params = Vec2;

    fn layout(&mut self, layout_params: Vec2) -> Rect {
        let r = Rect::new(
            self.current_x,
            self.bounds.y,
            layout_params.x,
            layout_params.y,
        );
        self.current_x += layout_params.x + self.spacing;
        r
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

    fn begin(self, bounds: Rect) -> Self::State {
        OffsetState {
            offset: self.offset,
            inner: self.inner.begin(bounds),
        }
    }
}

pub struct OffsetState<InnerS> {
    offset: Vec2,
    inner: InnerS,
}

impl<InnerS: LayoutState> LayoutState for OffsetState<InnerS> {
    type Params = InnerS::Params;

    fn layout(&mut self, layout_params: Self::Params) -> Rect {
        let mut r = self.inner.layout(layout_params);
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
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
        let r = state.layout(Rect::new(5.0, 5.0, 20.0, 20.0));
        assert_eq!(r, Rect::new(15.0, 15.0, 20.0, 20.0));
    }

    #[test]
    fn test_column_layout() {
        let mut state = ColumnLayout { spacing: 10.0 }.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(50.0, 20.0));
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0));
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_row_layout() {
        let mut state = RowLayout { spacing: 5.0 }.begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(30.0, 20.0));
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        let r2 = state.layout(Vec2::new(20.0, 30.0));
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    fn test_offset_layout() {
        let offset = OffsetLayout {
            offset: Vec2::new(5.0, 15.0),
            inner: ColumnLayout { spacing: 10.0 },
        };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = offset.begin(bounds);

        let r1 = state.layout(Vec2::new(50.0, 20.0));
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        // Logic X is 10.0. Actual X = 10.0 - 5.0 = 5.0
        assert_eq!(r1, Rect::new(5.0, -5.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0));
        // Logic Y is 10.0 + 20.0 + 10.0 = 40.0. Actual Y = 40.0 - 15.0 = 25.0
        assert_eq!(r2, Rect::new(5.0, 25.0, 40.0, 30.0));
    }
}
