use crate::layout::{
    Align, AxisBound, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken, Placement,
    Placement2D, Size,
};
use crate::types::{Rect, Vec2};

// ── ColumnLayout ──────────────────────────────────────────────────────────

pub struct ColumnLayout {
    pub spacing: f32,
}

impl Layout for ColumnLayout {
    type Params = Placement2D;
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
    type Params = Placement2D;

    fn layout(&mut self, layout_params: Placement2D, intrinsic: IntrinsicSize) -> Rect {
        let pref = intrinsic.preferred;
        // Cross axis (width) fills the column space; main axis (height) stacks.
        let w = layout_params
            .width
            .resolve_size(pref.map(|p| p.x), self.space.width)
            .unwrap();
        let h = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), self.space.height)
            .unwrap();

        let x = self.space.x
            + layout_params
                .width
                .align_offset(w, self.space.width)
                .unwrap();

        let r = Rect::new(x, self.current_y, w, h);
        self.content_w = self.content_w.max((x + w) - self.space.x);
        self.content_h = (self.current_y + h) - self.space.y;
        self.current_y += h + self.spacing;
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
            Placement::Fill => self.space.width,
            Placement::Sized {
                size: Size::Auto, ..
            } => match self.space.width {
                AxisBound::Exact(w) | AxisBound::AtMost(w) => AxisBound::AtMost(w),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let height = match layout_params.height {
            Placement::Sized {
                size: Size::Fixed(h),
                ..
            } => AxisBound::Exact(h),
            Placement::Fill => match self.space.height {
                AxisBound::Exact(h) => {
                    AxisBound::Exact((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::AtMost(h) => {
                    AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
            Placement::Sized {
                size: Size::Auto, ..
            } => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => {
                    AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
                }
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        let w = match layout_params.width {
            Placement::Sized {
                size: Size::Fixed(w),
                ..
            } => Some(w),
            Placement::Fill => match self.space.width {
                AxisBound::Exact(w) => Some(w),
                AxisBound::AtMost(_) | AxisBound::Unbounded => None,
            },
            Placement::Sized {
                size: Size::Auto,
                align,
            } => {
                if align == Align::Center || align == Align::End {
                    panic!(
                        "Layout panic: Align::{align:?} cannot be applied to an Auto-sized \
                         deferred child — its size is only known once the layout closes, and \
                         its already-emitted output cannot be shifted retroactively. Use a \
                         Fixed size, or Align::Start."
                    );
                }
                None
            }
        };

        let x = self.space.x
            + layout_params
                .width
                .align_offset(w.unwrap_or(0.0), self.space.width)
                .unwrap();

        let space = LayoutSpace::new(x, self.current_y, width, height);
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
            .resolve_size(pref.map(|p| p.x), self.space.width)
            .unwrap();
        let h = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), self.space.height)
            .unwrap();

        let x = self.space.x
            + layout_params
                .width
                .align_offset(w, self.space.width)
                .unwrap();

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_layout() {
        let mut state = ColumnLayout { spacing: 10.0 }.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state.layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_column_auto_uses_intrinsic_preferred() {
        let mut state = ColumnLayout { spacing: 0.0 }.begin(Rect::new(0.0, 0.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fixed(120.0),
            height: Placement::auto(),
        };
        let intrinsic = IntrinsicSize::preferred(Vec2::new(80.0, 24.0));
        let r = state.layout(req, intrinsic);
        // Auto height reads intrinsic.preferred.y; width stays fixed.
        assert_eq!(r, Rect::new(0.0, 0.0, 120.0, 24.0));
    }

    #[test]
    fn test_column_fill_cross_axis_uses_bounds_width() {
        let mut state = ColumnLayout { spacing: 0.0 }.begin(Rect::new(5.0, 0.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(30.0),
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN);
        // Fill width spans the column bounds width.
        assert_eq!(r, Rect::new(5.0, 0.0, 200.0, 30.0));
    }

    #[test]
    fn test_column_fill_height_against_exact() {
        let mut state = ColumnLayout { spacing: 0.0 }.begin(Rect::new(0.0, 10.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fixed(120.0),
            height: Placement::fill(),
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN);
        assert_eq!(r, Rect::new(0.0, 10.0, 120.0, 500.0));
    }

    #[test]
    fn test_column_unbounded_height_resolves_concrete() {
        let mut state =
            ColumnLayout { spacing: 5.0 }.begin(LayoutSpace::unbounded_height(0.0, 0.0, 200.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
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
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_fill_on_unbounded_axis_panics() {
        let mut state =
            ColumnLayout { spacing: 0.0 }.begin(LayoutSpace::unbounded_height(0.0, 0.0, 100.0));
        let req = Placement2D {
            width: Placement::fixed(50.0),
            height: Placement::fill(),
        };
        let _ = state.layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 18.0)));
    }

    #[test]
    fn test_column_content_extent() {
        let mut state = ColumnLayout { spacing: 10.0 }.begin(Rect::new(5.0, 7.0, 100.0, 500.0));
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(60.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
    }

    #[test]
    fn test_column_cross_alignment_exact() {
        // Exact width layout space
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Exact(100.0), AxisBound::Unbounded);

        // Center alignment
        let mut center_state = ColumnLayout { spacing: 5.0 }.begin(space);
        let r1 = center_state.layout(
            Placement2D::fixed(40.0, 20.0).align_x(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
        // x = 10.0 + (100.0 - 40.0) * 0.5 = 40.0
        assert_eq!(r1, Rect::new(40.0, 10.0, 40.0, 20.0));

        // End alignment
        let mut end_state = ColumnLayout { spacing: 5.0 }.begin(space);
        let r2 = end_state.layout(
            Placement2D::fixed(30.0, 20.0).align_x(Align::End),
            IntrinsicSize::UNKNOWN,
        );
        // x = 10.0 + 100.0 - 30.0 = 80.0
        assert_eq!(r2, Rect::new(80.0, 10.0, 30.0, 20.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let mut state = ColumnLayout { spacing: 5.0 }.begin(space);
        let _ = state.layout(
            Placement2D::fixed(40.0, 20.0).align_x(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ColumnLayout { spacing: 5.0 }.begin(space);
        let _ = state.layout(
            Placement2D::fixed(40.0, 20.0).align_x(Align::End),
            IntrinsicSize::UNKNOWN,
        );
    }

    #[test]
    fn test_deferred_layout_token_lifecycle() {
        let mut state = ColumnLayout { spacing: 8.0 }.begin(LayoutSpace::new(
            0.0,
            0.0,
            AxisBound::Exact(200.0),
            AxisBound::Unbounded,
        ));

        // 1. Begin layout for a fit container
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Space should start at current_y = 0.0, with unbounded height and exact width
        assert_eq!(space.x, 0.0);
        assert_eq!(space.y, 0.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));
        assert_eq!(space.height, AxisBound::Unbounded);

        // 2. End layout with the child's computed extent (say 80x50)
        let resolved_rect = token.end_layout(Vec2::new(80.0, 50.0));

        // Fill width resolves to 200.0. Start aligned.
        assert_eq!(resolved_rect, Rect::new(0.0, 0.0, 200.0, 50.0));

        // Cursor should have advanced by height (50.0) + spacing (8.0) = 58.0
        let next_rect = state.layout(Placement2D::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(next_rect.y, 58.0);
    }

    #[test]
    fn test_column_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);

        // 1. Fixed child height
        let req_fixed = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(50.0),
        };
        let (space_f, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        assert_eq!(space_f.width, AxisBound::Exact(200.0));
        assert_eq!(space_f.height, AxisBound::Exact(50.0));

        // Place a child to advance cursor by 50 + spacing(10) = 60
        state.layout(Placement2D::fixed(200.0, 50.0), IntrinsicSize::UNKNOWN);

        // Remaining parent height is 300 - 60 = 240.
        // 2. Fill child height
        let req_fill = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space_fill, _token) = state.begin_layout(req_fill, IntrinsicSize::UNKNOWN);
        assert_eq!(space_fill.width, AxisBound::AtMost(200.0));
        assert_eq!(space_fill.height, AxisBound::Exact(240.0));

        // 3. Auto child height
        let req_auto = Placement2D {
            width: Placement::auto(),
            height: Placement::auto(),
        };
        let (space_auto, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        assert_eq!(space_auto.width, AxisBound::AtMost(200.0));
        assert_eq!(space_auto.height, AxisBound::AtMost(240.0));
    }

    #[test]
    fn test_column_begin_layout_under_parent_at_most() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(150.0), AxisBound::AtMost(250.0));
        let mut state = ColumnLayout { spacing: 5.0 }.begin(parent_space);

        // 1. Fill child width, Auto child height
        let req1 = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
        };
        let (space1, _token) = state.begin_layout(req1, IntrinsicSize::UNKNOWN);
        // Fill width under parent AtMost(150) should propagate AtMost(150)
        assert_eq!(space1.width, AxisBound::AtMost(150.0));
        // Auto height under parent AtMost(250) should propagate AtMost(250)
        assert_eq!(space1.height, AxisBound::AtMost(250.0));

        // Advance cursor by 40 + spacing(5) = 45
        state.layout(Placement2D::fixed(100.0, 40.0), IntrinsicSize::UNKNOWN);

        // 2. Fill child height (remaining = 250 - 45 = 205)
        let req2 = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space2, _token) = state.begin_layout(req2, IntrinsicSize::UNKNOWN);
        // Auto width under parent AtMost(150) should yield AtMost(150)
        assert_eq!(space2.width, AxisBound::AtMost(150.0));
        // Fill height under parent AtMost should yield AtMost(205)
        assert_eq!(space2.height, AxisBound::AtMost(205.0));
    }

    #[test]
    fn test_deferred_column_center_align_fixed() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);

        let req = Placement2D {
            width: Placement::fixed(80.0).align(Align::Center),
            height: Placement::fixed(40.0),
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
        let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);

        // Fill has alignment impossible, but let's test End alignment for fixed or auto.
        // If Fill is used, it cannot be aligned (it spans the full width), so x is space.x = 10.0.
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(40.0),
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
        let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);

        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(40.0),
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
    #[should_panic(expected = "cannot be applied to an Auto-sized")]
    fn test_deferred_column_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);

        let req = Placement2D {
            width: Placement::auto().align(Align::Center),
            height: Placement::fixed(40.0),
        };
        // Auto width under Center alignment should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_column_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(400.0), AxisBound::Unbounded);

        // Center alignment
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::Center);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box right edge = 10.0 + (400.0 - 180.0)/2 + 180.0 = 300.0.
            // Under resolve_space, the Exact(400.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // End alignment
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::End);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);

            // Bounding box right edge = 10.0 + 400.0 - 180.0 + 180.0 = 410.0.
            // Relative right edge = 410.0 - 10.0 = 400.0.
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // Center alignment (deferred begin/end layout)
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::Center);
            let (_, token) = state.begin_layout(req.clone(), IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(180.0, 32.0));

            // Under resolve_space, the Exact(400.0) parent constraint is preserved.
            assert_eq!(state.resolve_space().w, 400.0);
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

        let req = Placement2D::fixed(180.0, 32.0);

        // 1. Exact(400.0) -> Expected: exact bounds (400.0) even if widgets are smaller (180.0)
        // The layout space determines the value (Exact constraint overrides any smaller child size).
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 400.0);
        }

        // 2. AtMost(400.0) -> Expected: shrink-wrapped to child's actual width (180.0)
        // Placed widgets determine the value (since the child's size is within parent bounds).
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 180.0);
        }

        // 2b. AtMost(100.0) (widgets larger than AtMost value) -> Expected: capped at AtMost value (100.0)
        // The layout space determines the value (clamped at parent limit ceiling).
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 100.0);
        }

        // 3. Unbounded -> Expected: child's actual width (180.0)
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            let mut state = ColumnLayout { spacing: 10.0 }.begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN);
            assert_eq!(state.resolve_space().w, 180.0);
        }
    }
}
