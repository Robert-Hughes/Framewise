use crate::layout::{
    AxisBound, Layout, LayoutResult, LayoutSpace, LayoutState, LayoutToken, SizeOffer, SizeRequest,
};
use crate::types::{Rect, Vec2};

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

    fn peek_offer(&self, layout_params: Rect) -> LayoutResult<SizeOffer> {
        LayoutResult::Ok(SizeOffer::new(
            AxisBound::Exact(layout_params.w),
            AxisBound::Exact(layout_params.h),
        ))
    }

    fn layout(&mut self, layout_params: Rect, _request: SizeRequest) -> LayoutResult<Rect> {
        // Offset the requested rect by the layout's origin. This ensures if
        // ManualLayout is nested inside a scroll view (or any other layout), the
        // explicit rects still shift correctly relative to the parent. The
        // explicit size is independent of the available extent, so ManualLayout
        // is unaffected by an unbounded axis.
        // The requested rect is origin-relative, so its far edge *is* the content
        // extent contribution (no need to subtract the origin back out).
        self.content_extent.x = self.content_extent.x.max(layout_params.x + layout_params.w);
        self.content_extent.y = self.content_extent.y.max(layout_params.y + layout_params.h);
        LayoutResult::Ok(Rect::new(
            self.space.x + layout_params.x,
            self.space.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        ))
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Rect,
        _request: SizeRequest,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>) {
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
        (LayoutResult::Ok(space), token)
    }

    fn end_layout(&mut self, layout_params: Rect, _extent: Vec2) -> LayoutResult<Rect> {
        self.content_extent.x = self.content_extent.x.max(layout_params.x + layout_params.w);
        self.content_extent.y = self.content_extent.y.max(layout_params.y + layout_params.h);
        LayoutResult::Ok(Rect::new(
            self.space.x + layout_params.x,
            self.space.y + layout_params.y,
            layout_params.w,
            layout_params.h,
        ))
    }

    fn resolve_space(&self) -> Rect {
        self.space.resolve(self.content_extent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manual_layout() {
        let mut state = ManualLayout.begin(Rect::new(10.0, 10.0, 100.0, 100.0));
        let r = state
            .layout(Rect::new(5.0, 5.0, 20.0, 20.0), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(r, Rect::new(15.0, 15.0, 20.0, 20.0));
    }

    #[test]
    fn test_manual_peek_offer_returns_exact_rect_size() {
        let state = ManualLayout.begin(Rect::new(10.0, 20.0, 300.0, 400.0));

        let offer = state.peek_offer(Rect::new(5.0, 6.0, 70.0, 80.0)).unwrap();

        assert_eq!(
            offer,
            SizeOffer::new(AxisBound::Exact(70.0), AxisBound::Exact(80.0))
        );
    }

    #[test]
    fn test_manual_peek_offer_does_not_change_resolve_space() {
        let parent_space =
            LayoutSpace::new(100.0, 100.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let state = ManualLayout.begin(parent_space);
        let before = state.resolve_space();

        let _ = state.peek_offer(Rect::new(0.0, 0.0, 50.0, 20.0)).unwrap();

        assert_eq!(state.resolve_space(), before);
    }

    #[test]
    fn test_manual_peek_then_layout_matches_layout_alone() {
        let mut peeked = ManualLayout.begin(Rect::new(10.0, 20.0, 300.0, 400.0));
        let mut direct = ManualLayout.begin(Rect::new(10.0, 20.0, 300.0, 400.0));
        let params = Rect::new(5.0, 6.0, 70.0, 80.0);

        let _ = peeked.peek_offer(params).unwrap();
        let peeked_rect = peeked.layout(params, SizeRequest::UNKNOWN).unwrap();
        let direct_rect = direct.layout(params, SizeRequest::UNKNOWN).unwrap();

        assert_eq!(peeked_rect, direct_rect);
        assert_eq!(peeked.resolve_space(), direct.resolve_space());
    }

    #[test]
    fn test_manual_content_extent() {
        let parent_space =
            LayoutSpace::new(100.0, 100.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ManualLayout.begin(parent_space);
        let _ = state
            .layout(Rect::new(0.0, 0.0, 50.0, 20.0), SizeRequest::UNKNOWN)
            .unwrap();
        let _ = state
            .layout(Rect::new(80.0, 40.0, 30.0, 30.0), SizeRequest::UNKNOWN)
            .unwrap();
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
            let _ = state
                .layout(Rect::new(0.0, 0.0, 50.0, 40.0), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));

            // Larger: Placed child far edge (300, 260) exceeds 200x150 bounds.
            // Layout space (Exact) determines the resolved size (clamps/forces 200x150).
            let mut state = ManualLayout.begin(parent_space_exact);
            let _ = state
                .layout(Rect::new(50.0, 60.0, 250.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));
        }

        // 2. AtMost -> Expected: shrink-wrapped to child boundaries if smaller, capped if larger.
        // Both the widget sizes and the layout space limit determine the final value.
        {
            // Smaller: Placed child far edge (60, 50) is within the 200x150 limits.
            // Placed widgets determine the value (shrink-wrapped).
            let mut state = ManualLayout.begin(parent_space_at_most);
            let _ = state
                .layout(Rect::new(10.0, 10.0, 50.0, 40.0), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 60.0, 50.0));

            // Larger: Placed child far edge (300, 260) exceeds the 200x150 limits.
            // Layout space (AtMost limit) determines the value (clamps at limit ceilings).
            let mut state = ManualLayout.begin(parent_space_at_most);
            let _ = state
                .layout(Rect::new(50.0, 60.0, 250.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 200.0, 150.0));
        }

        // 3. Unbounded -> Expected: shrink-wrapped to child boundaries (max far edges).
        // Placed widgets determine the value entirely (since there is no parent constraint ceiling).
        {
            // Placed child far edge is (300, 260).
            // Placed widgets determine the value.
            let mut state = ManualLayout.begin(parent_space_unbounded);
            let _ = state
                .layout(Rect::new(50.0, 60.0, 250.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(state.resolve_space(), Rect::new(10.0, 20.0, 300.0, 260.0));
        }
    }

    #[test]
    fn test_deferred_manual_layout_lifecycle() {
        let parent_space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = ManualLayout.begin(parent_space);
        let layout_param = Rect::new(20.0, 30.0, 50.0, 40.0);
        let (space_res, token) = state.begin_layout(layout_param, SizeRequest::UNKNOWN);
        let space = space_res.unwrap();

        // ManualLayout begins at logically shifted coordinate: (10.0+20.0, 10.0+30.0) = (30.0, 40.0)
        assert_eq!(space.x, 30.0);
        assert_eq!(space.y, 40.0);
        assert_eq!(space.width, AxisBound::Exact(50.0));
        assert_eq!(space.height, AxisBound::Exact(40.0));

        let resolved_rect = token.end_layout(Vec2::new(15.0, 15.0)).unwrap();
        // Resolved rect should be exactly the requested rect shifted by origin
        assert_eq!(resolved_rect, Rect::new(30.0, 40.0, 50.0, 40.0));

        // Resolved space origin is 10.0, 10.0; extent is max far edges (70.0, 70.0)
        assert_eq!(state.resolve_space(), Rect::new(10.0, 10.0, 70.0, 70.0));
    }

    #[test]
    fn test_manual_begin_layout_propagates_exact_bounds() {
        let mut state = ManualLayout.begin(Rect::new(10.0, 20.0, 300.0, 400.0));
        let (space_res, _token) =
            state.begin_layout(Rect::new(5.0, 10.0, 100.0, 150.0), SizeRequest::UNKNOWN);
        let space = space_res.unwrap();
        assert_eq!(space.width, AxisBound::Exact(100.0));
        assert_eq!(space.height, AxisBound::Exact(150.0));
    }
}
