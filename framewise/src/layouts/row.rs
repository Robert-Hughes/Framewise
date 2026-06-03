use crate::layout::{
    AxisBound, CrossAlign, Extent, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken,
    SizeReq,
};
use crate::types::{Rect, Vec2};

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
        // `Fill` on an unbounded axis is unsatisfiable and panics in Extent::resolve.
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height);

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
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height);

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

#[cfg(test)]
mod tests {
    use super::*;

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
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_auto_without_intrinsic_panics() {
        let mut state = RowLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        // Auto sizing with no intrinsic reported is an unsatisfiable request → panic.
        let _ = state.layout(SizeReq::auto(), IntrinsicSize::UNKNOWN);
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
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_fill_on_unbounded_axis_panics() {
        let mut state = RowLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(LayoutSpace::unbounded_width(0.0, 0.0, 40.0));
        let req = SizeReq {
            width: Extent::Fill,
            height: Extent::Fixed(40.0),
        };
        // Fill width on the unbounded axis → no edge to fill into → unsatisfiable → panic.
        let _ = state.layout(req, IntrinsicSize::UNKNOWN);
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
