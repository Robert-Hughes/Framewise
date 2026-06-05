use crate::layout::{
    AxisBound, IntrinsicSize, Layout, LayoutResult, LayoutSpace, LayoutState, LayoutToken,
    Placement, Placement2D, Size,
};
use crate::types::{Rect, Vec2};

// ── WrapLayout ─────────────────────────────────────────────────────────────

/// A flow layout: places children left-to-right, wrapping to the next line when
/// the next child would overflow the bounds width. Intrinsic-aware — children
/// are sized from their [`Placement2D`] and reported intrinsic size, exactly like
/// row/column.
pub struct WrapLayout {
    /// Horizontal gap between items on a line.
    pub spacing: f32,
    /// Vertical gap between lines.
    pub line_spacing: f32,
}

impl Layout for WrapLayout {
    type Params = Placement2D;
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
    type Params = Placement2D;

    fn layout(
        &mut self,
        layout_params: Placement2D,
        intrinsic: IntrinsicSize,
    ) -> LayoutResult<Rect> {
        let pref = intrinsic.preferred;
        let (w, v1) = layout_params
            .width
            .resolve_size(pref.map(|p| p.x), self.space.width)
            .into_parts();

        // Wrap check happens *before* item is positioned, but we must resolve the size
        // before we can perform the wrap check (as wrap check depends on width `w`).
        // Therefore, when resolving the height, we must first predict if it wraps,
        // so we can resolve height against the remaining height of the correct line.
        let at_line_start = self.current_x == self.space.x;
        let overflows = match self.space.width {
            AxisBound::Exact(width) | AxisBound::AtMost(width) => {
                self.current_x + w > self.space.x + width
            }
            AxisBound::Unbounded => false,
        };
        let predicted_y = if !at_line_start && overflows {
            self.current_y + self.line_height + self.line_spacing
        } else {
            self.current_y
        };

        let remaining_h = match self.space.height {
            AxisBound::Exact(h) => AxisBound::Exact((h - (predicted_y - self.space.y)).max(0.0)),
            AxisBound::AtMost(h) => AxisBound::AtMost((h - (predicted_y - self.space.y)).max(0.0)),
            AxisBound::Unbounded => AxisBound::Unbounded,
        };
        let (h, v2) = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), remaining_h)
            .into_parts();

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
        LayoutResult::from_parts(r, v1.or(v2))
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Placement2D,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>)
    where
        Self: Sized,
    {
        let w = match layout_params.width {
            Placement::Sized { size: Size::Fixed(w), .. } => w,
            Placement::Fill => match self.space.width {
                AxisBound::Exact(w) => (w - (self.current_x - self.space.x)).max(0.0),
                AxisBound::AtMost(_) | AxisBound::Unbounded => panic!("Layout panic: WrapLayout cannot resolve Placement::Fill under non-Exact bounds in begin_layout"),
            },
            Placement::Sized { size: Size::Auto, .. } => panic!("Layout panic: WrapLayout does not support Auto-sized deferred containers because wrapping must be resolved in begin_layout"),
        };

        let width = AxisBound::Exact(w);

        let height = match layout_params.height {
            Placement::Sized {
                size: Size::Fixed(h),
                ..
            } => AxisBound::Exact(h),
            Placement::Fill
            | Placement::Sized {
                size: Size::Auto, ..
            } => match self.space.height {
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
        (LayoutResult::Ok(space), token)
    }

    fn end_layout(&mut self, layout_params: Placement2D, extent: Vec2) -> LayoutResult<Rect> {
        let pref = Some(extent);
        let (w, v1) = layout_params
            .width
            .resolve_size(pref.map(|p| p.x), self.space.width)
            .into_parts();

        let remaining_h = match self.space.height {
            AxisBound::Exact(h) => AxisBound::Exact((h - (self.current_y - self.space.y)).max(0.0)),
            AxisBound::AtMost(h) => {
                AxisBound::AtMost((h - (self.current_y - self.space.y)).max(0.0))
            }
            AxisBound::Unbounded => AxisBound::Unbounded,
        };
        let (h, v2) = layout_params
            .height
            .resolve_size(pref.map(|p| p.y), remaining_h)
            .into_parts();

        let r = Rect::new(self.current_x, self.current_y, w, h);
        self.content_w = self.content_w.max((self.current_x + w) - self.space.x);
        self.current_x += w + self.spacing;
        self.line_height = self.line_height.max(h);
        LayoutResult::from_parts(r, v1.or(v2))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wrap_layout_wraps_on_overflow() {
        // 100px-wide bounds, 40px items, no spacing: two per line, then wrap.
        let mut state = WrapLayout {
            spacing: 0.0,
            line_spacing: 5.0,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 500.0));
        let item = Placement2D {
            width: Placement::fixed(40.0),
            height: Placement::fixed(20.0),
        };
        let r1 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
        let r2 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
        let r3 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
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
        let r = state
            .layout(
                Placement2D::auto(),
                IntrinsicSize::preferred(Vec2::new(80.0, 16.0)),
            )
            .unwrap();
        assert_eq!(r, Rect::new(0.0, 0.0, 80.0, 16.0));
    }

    #[test]
    fn test_wrap_unbounded_width_never_wraps() {
        // An unbounded width has no edge to overflow: every item stays on line 0.
        let mut state = WrapLayout {
            spacing: 0.0,
            line_spacing: 5.0,
        }
        .begin(LayoutSpace::unbounded_width(0.0, 0.0, 500.0));
        let item = Placement2D {
            width: Placement::fixed(40.0),
            height: Placement::fixed(20.0),
        };
        let r1 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
        let r2 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
        let r3 = state.layout(item, IntrinsicSize::UNKNOWN).unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 40.0, 20.0));
        assert_eq!(r2, Rect::new(40.0, 0.0, 40.0, 20.0));
        assert_eq!(r3, Rect::new(80.0, 0.0, 40.0, 20.0));
    }

    #[test]
    fn test_deferred_wrap_layout_lifecycle() {
        let mut state = WrapLayout {
            spacing: 10.0,
            line_spacing: 5.0,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 200.0));

        // Place first item normally
        let r1 = state
            .layout(Placement2D::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 40.0, 20.0)); // cursor x is now 40 + 10 = 50

        // Place a deferred item of width 40.0.
        let req1 = Placement2D {
            width: Placement::fixed(40.0),
            height: Placement::auto(),
        };
        let (space_res, token) = state.begin_layout(req1, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();
        // provisional space starts at current_x (50) and current_y (0)
        assert_eq!(space.x, 50.0);
        assert_eq!(space.y, 0.0);

        // child measures 40x30.
        // Item is placed at current_x = 50. x + w = 50 + 40 = 90 <= 100, no overflow.
        let resolved_rect = token.end_layout(Vec2::new(40.0, 30.0)).unwrap();
        assert_eq!(resolved_rect, Rect::new(50.0, 0.0, 40.0, 30.0)); // cursor x now 50 + 40 + 10 = 100. line_height = max(20, 30) = 30.

        // Place next item of width 20.0. Under 100px width limit, cursor x = 100. 100 + 20 = 120 > 100, so it wraps.
        let req2 = Placement2D {
            width: Placement::fixed(20.0),
            height: Placement::auto(),
        };
        let (space2_res, token2) = state.begin_layout(req2, IntrinsicSize::UNKNOWN);
        let space2 = space2_res.unwrap();
        // Under WrapLayout's upfront wrap resolution, space2 wraps to start of next line: (0.0, 35.0)
        assert_eq!(space2.x, 0.0);
        assert_eq!(space2.y, 35.0);

        // This item is width 20.
        let resolved_rect2 = token2.end_layout(Vec2::new(20.0, 15.0)).unwrap();
        assert_eq!(resolved_rect2, Rect::new(0.0, 35.0, 20.0, 15.0));
    }

    #[test]
    fn test_wrap_fill_height_remaining() {
        // 1. Exact(100.0) height
        {
            let mut state = WrapLayout {
                spacing: 0.0,
                line_spacing: 0.0,
            }
            .begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            // First item takes 80px width, 30px height
            let _ = state
                .layout(
                    Placement2D {
                        width: Placement::fixed(80.0),
                        height: Placement::fixed(30.0),
                    },
                    IntrinsicSize::UNKNOWN,
                )
                .unwrap();
            // Second item has width 80px (wraps to next line, y=30) and fills remaining height
            let r = state
                .layout(
                    Placement2D {
                        width: Placement::fixed(80.0),
                        height: Placement::fill(),
                    },
                    IntrinsicSize::UNKNOWN,
                )
                .unwrap();
            assert_eq!(r.h, 70.0);
        }

        // 2. AtMost(100.0) height
        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::Exact(100.0), AxisBound::AtMost(100.0));
            let mut state = WrapLayout {
                spacing: 0.0,
                line_spacing: 0.0,
            }
            .begin(space);
            // First item takes 80px width, 30px height
            let _ = state
                .layout(
                    Placement2D {
                        width: Placement::fixed(80.0),
                        height: Placement::fixed(30.0),
                    },
                    IntrinsicSize::UNKNOWN,
                )
                .unwrap();
            // Second item has width 80px (wraps to next line, y=30) and fills remaining height with large intrinsic
            let res = state.layout(
                Placement2D {
                    width: Placement::fixed(80.0),
                    height: Placement::fill(),
                },
                IntrinsicSize::preferred(Vec2::new(80.0, 90.0)),
            );
            let (r, violation) = res.into_parts();
            assert_eq!(r.h, 70.0);
            assert!(violation.is_some());
        }
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
        state
            .layout(Placement2D::fixed(100.0, 40.0), IntrinsicSize::UNKNOWN)
            .unwrap();

        // Remaining width on this line is 250 - 110 = 140.
        // Fixed width, auto height child container.
        let req = Placement2D {
            width: Placement::fixed(80.0),
            height: Placement::auto(),
        };
        let (space_res, _token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();
        assert_eq!(space.width, AxisBound::Exact(80.0));
        assert_eq!(space.height, AxisBound::AtMost(200.0));
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

        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(40.0),
        };
        // Auto width under WrapLayout should panic during begin_layout
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }
}
