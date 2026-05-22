use crate::types::{Rect, Vec2};

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
        let r = Rect::new(self.bounds.x, self.current_y, layout_params.x, layout_params.y);
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
        let r = Rect::new(self.current_x, self.bounds.y, layout_params.x, layout_params.y);
        self.current_x += layout_params.x + self.spacing;
        r
    }
}

// ── OffsetLayout ──────────────────────────────────────────────────────────

/// A pure decorator layout that shifts coordinates on the Y axis.
pub struct OffsetLayout<L> {
    pub offset_y: f32,
    pub inner: L,
}

impl<L: Layout> Layout for OffsetLayout<L> {
    type Params = L::Params;
    type State = OffsetState<L::State>;

    fn begin(self, bounds: Rect) -> Self::State {
        OffsetState {
            offset_y: self.offset_y,
            inner: self.inner.begin(bounds),
        }
    }
}

pub struct OffsetState<InnerS> {
    offset_y: f32,
    inner: InnerS,
}

impl<InnerS: LayoutState> LayoutState for OffsetState<InnerS> {
    type Params = InnerS::Params;

    fn layout(&mut self, layout_params: Self::Params) -> Rect {
        let mut r = self.inner.layout(layout_params);
        r.y -= self.offset_y;
        r
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
            offset_y: 15.0,
            inner: ColumnLayout { spacing: 10.0 },
        };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = offset.begin(bounds);

        let r1 = state.layout(Vec2::new(50.0, 20.0));
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        assert_eq!(r1, Rect::new(10.0, -5.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0));
        // Logic Y is 10.0 + 20.0 + 10.0 = 40.0. Actual Y = 40.0 - 15.0 = 25.0
        assert_eq!(r2, Rect::new(10.0, 25.0, 40.0, 30.0));
    }
}

