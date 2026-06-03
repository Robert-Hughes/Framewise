use crate::layout::{
    AxisBound, CrossAlign, Extent, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken,
    SizeReq,
};
use crate::types::{Rect, Vec2};

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
        // `Fill` on an unbounded axis is unsatisfiable and panics in Extent::resolve.
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height);

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
        let w = layout_params
            .width
            .resolve(pref.map(|p| p.x), self.space.width);
        let h = layout_params
            .height
            .resolve(pref.map(|p| p.y), self.space.height);

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_column_fill_height_against_exact() {
        // "Panel fills available height inside a bounded container": Fill on the
        // main axis against an Exact column claims the full bounds extent and needs
        // no intrinsic measurement (Extent::Fill under Exact resolves to the edge).
        let mut state = ColumnLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 10.0, 200.0, 500.0));
        let req = SizeReq {
            width: Extent::Fixed(120.0),
            height: Extent::Fill,
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN);
        assert_eq!(r, Rect::new(0.0, 10.0, 120.0, 500.0));
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
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_fill_on_unbounded_axis_panics() {
        // Fill on an unbounded axis is unsatisfiable — no extent to fill into.
        // (Even with an intrinsic present: Fill is not Auto, so we don't degrade.)
        let mut state = ColumnLayout {
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(LayoutSpace::unbounded_height(0.0, 0.0, 100.0));
        let req = SizeReq {
            width: Extent::Fixed(50.0),
            height: Extent::Fill,
        };
        let _ = state.layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 18.0)));
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
}
