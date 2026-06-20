use crate::layout::{
    Layout, LayoutResult, LayoutSpace, LayoutState, LayoutToken, SizeOffer, SizeRequest,
    SpacerLayoutState,
};
use crate::types::{Rect, Vec2};

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

    fn peek_offer(&self, layout_params: Self::Params) -> LayoutResult<SizeOffer> {
        self.inner.peek_offer(layout_params)
    }

    fn layout(&mut self, layout_params: Self::Params, request: SizeRequest) -> LayoutResult<Rect> {
        self.inner.layout(layout_params, request).map(|mut r| {
            r.x -= self.offset.x;
            r.y -= self.offset.y;
            r
        })
    }

    fn begin_deferred_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>) {
        let (space_res, _) = self.inner.begin_deferred_layout(layout_params.clone());
        let space_res = space_res.map(|mut space| {
            space.x -= self.offset.x;
            space.y -= self.offset.y;
            space
        });
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space_res, token)
    }

    fn end_deferred_layout(
        &mut self,
        layout_params: Self::Params,
        extent: Vec2,
    ) -> LayoutResult<Rect> {
        self.inner
            .end_deferred_layout(layout_params, extent)
            .map(|mut r| {
                r.x -= self.offset.x;
                r.y -= self.offset.y;
                r
            })
    }

    fn resolve_space(&self) -> Rect {
        let mut r = self.inner.resolve_space();
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }
}

impl<InnerS: SpacerLayoutState> SpacerLayoutState for OffsetState<InnerS> {
    type SpacerParams = InnerS::SpacerParams;

    fn spacer(&mut self, params: Self::SpacerParams) {
        self.inner.spacer(params);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::linear::{ColumnLayout, ColumnLayoutParams};

    #[test]
    fn test_offset_content_extent_ignores_offset() {
        let offset = OffsetLayout {
            offset: Vec2::new(13.0, 27.0),
            inner: ColumnLayout,
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let _ = state
            .layout(ColumnLayoutParams::fixed(40.0, 20.0), SizeRequest::UNKNOWN)
            .unwrap();
        let _ = state
            .layout(ColumnLayoutParams::fixed(40.0, 30.0), SizeRequest::UNKNOWN)
            .unwrap();
        // resolved_space shifted by offset (origin: -13.0, -27.0)
        assert_eq!(state.resolve_space(), Rect::new(-13.0, -27.0, 100.0, 100.0));
    }

    #[test]
    fn test_offset_peek_offer_delegates_without_offset() {
        let offset = OffsetLayout {
            offset: Vec2::new(13.0, 27.0),
            inner: ColumnLayout,
        };
        let state = offset.begin(Rect::new(0.0, 0.0, 100.0, 200.0));

        let offer = state
            .peek_offer(ColumnLayoutParams::auto().fixed_y(30.0))
            .unwrap();

        assert_eq!(offer.width, crate::layout::AxisBound::AtMost(100.0));
        assert_eq!(offer.height, crate::layout::AxisBound::Exact(30.0));
    }

    #[test]
    fn test_offset_layout() {
        let offset = OffsetLayout {
            offset: Vec2::new(5.0, 15.0),
            inner: ColumnLayout,
        };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = offset.begin(bounds);

        let r1 = state
            .layout(ColumnLayoutParams::fixed(50.0, 20.0), SizeRequest::UNKNOWN)
            .unwrap();
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        // Logic X is 10.0. Actual X = 10.0 - 5.0 = 5.0
        assert_eq!(r1, Rect::new(5.0, -5.0, 50.0, 20.0));

        state.spacer(10.0.into());

        let r2 = state
            .layout(ColumnLayoutParams::fixed(40.0, 30.0), SizeRequest::UNKNOWN)
            .unwrap();
        // Logic Y is 10.0 + 20.0 + 10.0 = 40.0. Actual Y = 40.0 - 15.0 = 25.0
        assert_eq!(r2, Rect::new(5.0, 25.0, 40.0, 30.0));
    }

    #[test]
    fn test_deferred_offset_layout_lifecycle() {
        let offset = OffsetLayout {
            offset: Vec2::new(10.0, 20.0),
            inner: ColumnLayout,
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));

        let req = crate::layouts::linear::ColumnLayoutParams {
            x: crate::layouts::linear::LinearCross::fixed(50.0),
            y: crate::layouts::linear::LinearMain::auto(),
        };
        let (space_res, token) = state.begin_deferred_layout(req);
        let space = space_res.unwrap();

        // Provisional space should be shifted by offset: space.x = 0.0 - 10.0 = -10.0
        assert_eq!(space.x, -10.0);
        assert_eq!(space.y, -20.0);

        let resolved_rect = token.end_deferred_layout(Vec2::new(50.0, 40.0)).unwrap();
        // Rect resolved in inner layout is at (0, 0, 50, 40), then shifted: (-10, -20, 50, 40)
        assert_eq!(resolved_rect, Rect::new(-10.0, -20.0, 50.0, 40.0));
    }
}
