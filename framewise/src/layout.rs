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

    /// Optional: the clip rectangle for this layout, used for input hit-testing.
    fn clip_rect(&self) -> Option<Rect> {
        None
    }
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

// ── ScrollLayout ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub offset_y: f32,
    pub content_height: f32,
}

pub struct ScrollLayout<'a> {
    pub state: &'a mut ScrollState,
    pub spacing: f32,
    pub input: Option<&'a crate::input::Input>,
}

impl<'a> ScrollLayout<'a> {
    pub fn new(state: &'a mut ScrollState) -> Self {
        Self { state, spacing: 0.0, input: None }
    }

    pub fn with_spacing(state: &'a mut ScrollState, spacing: f32) -> Self {
        Self { state, spacing, input: None }
    }

    pub fn with_input(mut self, input: &'a crate::input::Input) -> Self {
        self.input = Some(input);
        self
    }
}

impl<'a> Layout for ScrollLayout<'a> {
    type Params = Vec2;
    type State = ScrollStateImpl<'a>;

    fn begin(self, bounds: Rect) -> Self::State {
        // Apply scroll delta if hovered
        if let Some(input) = self.input {
            if bounds.contains(input.mouse_pos) && input.scroll_delta.y != 0.0 {
                self.state.offset_y -= input.scroll_delta.y * 30.0;
                let max_scroll = (self.state.content_height - bounds.h).max(0.0);
                self.state.offset_y = self.state.offset_y.max(0.0).min(max_scroll);
            }
        }

        ScrollStateImpl {
            bounds,
            state: self.state,
            spacing: self.spacing,
            current_y: bounds.y,
        }
    }
}

pub struct ScrollStateImpl<'a> {
    bounds: Rect,
    state: &'a mut ScrollState,
    spacing: f32,
    current_y: f32, // represents logical Y ignoring scroll offset
}

impl<'a> LayoutState for ScrollStateImpl<'a> {
    type Params = Vec2;

    fn layout(&mut self, layout_params: Vec2) -> Rect {
        let r = Rect::new(
            self.bounds.x,
            self.current_y - self.state.offset_y,
            layout_params.x,
            layout_params.y,
        );
        self.current_y += layout_params.y + self.spacing;
        // Update content height tracking
        self.state.content_height = self.state.content_height.max(self.current_y - self.bounds.y);
        r
    }

    fn clip_rect(&self) -> Option<Rect> {
        Some(self.bounds)
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
    fn test_scroll_layout() {
        let mut scroll = ScrollState { offset_y: 15.0, content_height: 0.0 };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = ScrollLayout::new(&mut scroll).begin(bounds);

        let r1 = state.layout(Vec2::new(50.0, 20.0));
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        assert_eq!(r1, Rect::new(10.0, -5.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0));
        // Logic Y is 30.0. Actual Y = 30.0 - 15.0 = 15.0
        assert_eq!(r2, Rect::new(10.0, 15.0, 40.0, 30.0));

        assert_eq!(state.clip_rect(), Some(bounds));
    }
}
