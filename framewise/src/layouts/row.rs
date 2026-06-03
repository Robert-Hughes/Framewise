use crate::layout::{
    Align, AxisBound, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken, Placement,
    Placement2D, Size,
};
use crate::types::{Rect, Vec2};

// ── RowLayout ─────────────────────────────────────────────────────────────

pub struct RowLayout {
    pub spacing: f32,
}

impl Layout for RowLayout {
    type Params = Placement2D;
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
    type Params = Placement2D;

    fn layout(&mut self, layout_params: Placement2D, intrinsic: IntrinsicSize) -> Rect {
        let pref = intrinsic.preferred;
        // Main axis (width) advances the cursor; cross axis (height) fills space.
        let w = layout_params
            .width
            .resolve_size(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), self.space.height);

        let y = self.space.y + layout_params.height.align_offset(h, self.space.height);

        let r = Rect::new(self.current_x, y, w, h);
        self.content_w = (self.current_x + w) - self.space.x;
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.current_x += w + self.spacing;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Placement2D,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let width = match layout_params.width {
            Placement::Sized {
                size: Size::Fixed(w),
                ..
            } => AxisBound::Exact(w),
            Placement::Fill => match self.space.width {
                AxisBound::Exact(w) => {
                    AxisBound::Exact((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::AtMost(w) => {
                    AxisBound::AtMost((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
            Placement::Sized {
                size: Size::Auto, ..
            } => match self.space.width {
                AxisBound::Exact(w) | AxisBound::AtMost(w) => {
                    AxisBound::AtMost((w - (self.current_x - self.space.x)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let height = match layout_params.height {
            Placement::Sized {
                size: Size::Fixed(h),
                ..
            } => AxisBound::Exact(h),
            Placement::Fill => self.space.height,
            Placement::Sized {
                size: Size::Auto, ..
            } => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => AxisBound::AtMost(h),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let h = match layout_params.height {
            Placement::Sized {
                size: Size::Fixed(h),
                ..
            } => Some(h),
            Placement::Fill => match self.space.height {
                AxisBound::Exact(h) => Some(h),
                AxisBound::AtMost(_) | AxisBound::Unbounded => None,
            },
            Placement::Sized {
                size: Size::Auto,
                align,
            } => {
                if align == Align::Center || align == Align::End {
                    panic!("Layout panic: Align::{align:?} cannot align dynamic (Auto/Fill) size child in begin_layout");
                }
                None
            }
        };

        let y = self.space.y
            + layout_params
                .height
                .align_offset(h.unwrap_or(0.0), self.space.height);

        let space = LayoutSpace::new(self.current_x, y, width, height);
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: Placement2D, extent: Vec2) -> Rect {
        let pref = Some(extent);
        let w = layout_params
            .width
            .resolve_size(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), self.space.height);

        let y = self.space.y + layout_params.height.align_offset(h, self.space.height);

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_row_layout() {
        let mut state = RowLayout { spacing: 5.0 }.begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        let r2 = state.layout(Vec2::new(20.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_auto_without_intrinsic_panics() {
        let mut state = RowLayout { spacing: 0.0 }.begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let _ = state.layout(Placement2D::auto(), IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_row_auto_width_advances_cursor() {
        let mut state = RowLayout { spacing: 6.0 }.begin(Rect::new(0.0, 0.0, 400.0, 50.0));
        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(40.0),
        };
        let r1 = state.layout(req, IntrinsicSize::preferred(Vec2::new(70.0, 16.0)));
        assert_eq!(r1, Rect::new(0.0, 0.0, 70.0, 40.0));
        let r2 = state.layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 16.0)));
        assert_eq!(r2, Rect::new(76.0, 0.0, 50.0, 40.0));
    }

    #[test]
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_fill_on_unbounded_axis_panics() {
        let mut state =
            RowLayout { spacing: 0.0 }.begin(LayoutSpace::unbounded_width(0.0, 0.0, 40.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(40.0),
        };
        let _ = state.layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_row_content_extent() {
        let mut state = RowLayout { spacing: 5.0 }.begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        state.layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(20.0, 40.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(state.resolve_space(), Rect::new(0.0, 0.0, 400.0, 100.0));
    }

    #[test]
    fn test_row_cross_alignment_exact() {
        // Exact height layout space
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Exact(80.0));

        // Center alignment
        let mut center_state = RowLayout { spacing: 5.0 }.begin(space);
        let r1 = center_state.layout(
            Placement2D::fixed(40.0, 20.0).align_y(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
        // y = 10.0 + (80.0 - 20.0) * 0.5 = 40.0
        assert_eq!(r1, Rect::new(10.0, 40.0, 40.0, 20.0));

        // End alignment
        let mut end_state = RowLayout { spacing: 5.0 }.begin(space);
        let r2 = end_state.layout(
            Placement2D::fixed(40.0, 30.0).align_y(Align::End),
            IntrinsicSize::UNKNOWN,
        );
        // y = 10.0 + 80.0 - 30.0 = 60.0
        assert_eq!(r2, Rect::new(10.0, 60.0, 40.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::AtMost(80.0));
        let mut state = RowLayout { spacing: 5.0 }.begin(space);
        let _ = state.layout(
            Placement2D::fixed(40.0, 20.0).align_y(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = RowLayout { spacing: 5.0 }.begin(space);
        let _ = state.layout(
            Placement2D::fixed(40.0, 20.0).align_y(Align::End),
            IntrinsicSize::UNKNOWN,
        );
    }

    #[test]
    fn test_deferred_row_layout_lifecycle() {
        let mut state = RowLayout { spacing: 5.0 }.begin(LayoutSpace::new(
            10.0,
            10.0,
            AxisBound::Unbounded,
            AxisBound::Exact(100.0),
        ));

        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Space should start at current_x = 10.0, with unbounded width and exact height
        assert_eq!(space.x, 10.0);
        assert_eq!(space.y, 10.0);
        assert_eq!(space.width, AxisBound::Unbounded);
        assert_eq!(space.height, AxisBound::Exact(100.0));

        // Consume token with extent 60x40.
        // Width is Placement::auto -> 60.0.
        // Height is Placement::fill -> resolves to space.height (Exact 100.0).
        let resolved_rect = token.end_layout(Vec2::new(60.0, 40.0));
        assert_eq!(resolved_rect, Rect::new(10.0, 10.0, 60.0, 100.0));

        // Cursor should have advanced by width (60.0) + spacing (5.0) = 65.0, so next starts at 75.0
        let next_rect = state.layout(
            Placement2D::fixed(30.0, 20.0).align_y(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
        assert_eq!(next_rect.x, 75.0);
        // Center aligned: y = 10.0 + (100.0 - 20.0) * 0.5 = 50.0
        assert_eq!(next_rect.y, 50.0);
    }

    #[test]
    fn test_row_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(400.0), AxisBound::Exact(150.0));
        let mut state = RowLayout { spacing: 5.0 }.begin(parent_space);

        // Fixed width
        let req_fixed = Placement2D {
            width: Placement::fixed(80.0),
            height: Placement::fill(),
        };
        let (space_f, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        assert_eq!(space_f.width, AxisBound::Exact(80.0));
        assert_eq!(space_f.height, AxisBound::Exact(150.0));

        // Advance cursor by 80 + spacing(5) = 85
        state.layout(Placement2D::fixed(80.0, 100.0), IntrinsicSize::UNKNOWN);

        // Remaining parent width is 400 - 85 = 315
        // Auto width, fill height
        let req_auto = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space_auto, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        assert_eq!(space_auto.width, AxisBound::AtMost(315.0));
        assert_eq!(space_auto.height, AxisBound::Exact(150.0));
    }

    #[test]
    #[should_panic(expected = "cannot align dynamic")]
    fn test_deferred_row_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = RowLayout { spacing: 10.0 }.begin(parent_space);

        let req = Placement2D {
            width: Placement::fixed(80.0),
            height: Placement::auto().align(Align::Center),
        };
        // Auto height under Center alignment in RowLayout should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_row_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Exact(300.0));

        // Center alignment
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::Center);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box bottom edge = 20.0 + (300.0 - 100.0)/2 + 100.0 = 220.0.
            // Under resolve_space, the Exact(300.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // End alignment
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::End);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box bottom edge = 20.0 + 300.0 - 100.0 + 100.0 = 320.0.
            // Relative bottom edge = 320.0 - 20.0 = 300.0.
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // Center alignment (deferred begin/end layout)
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::Center);
            let (_, token) = state.begin_layout(req.clone(), IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(80.0, 100.0));

            // Under resolve_space, the Exact(300.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().h, 300.0);
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

        let req = Placement2D::fixed(80.0, 100.0);

        // 1. Exact(300.0) -> Expected: exact bounds (300.0) even if widgets are smaller (100.0)
        // The layout space determines the value (Exact constraint overrides any smaller child size).
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 300.0);
        }

        // 2. AtMost(300.0) -> Expected: shrink-wrapped to child's actual height (100.0)
        // Placed widgets determine the value (since the child's size is within parent bounds).
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 100.0);
        }

        // 2b. AtMost(50.0) (widgets larger than AtMost value) -> Expected: capped at AtMost value (50.0)
        // The layout space determines the value (clamped at parent limit ceiling).
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 50.0);
        }

        // 3. Unbounded -> Expected: child's actual height (100.0)
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            let mut state = RowLayout { spacing: 10.0 }.begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().h, 100.0);
        }
    }
}
