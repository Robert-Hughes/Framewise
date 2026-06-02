use crate::layout::{IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken};
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

    fn layout(&mut self, layout_params: Self::Params, intrinsic: IntrinsicSize) -> Rect {
        let mut r = self.inner.layout(layout_params, intrinsic);
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
        intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        let (mut space, _) = self.inner.begin_layout(layout_params.clone(), intrinsic);
        space.x -= self.offset.x;
        space.y -= self.offset.y;
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (space, token)
    }

    fn end_layout(&mut self, layout_params: Self::Params, extent: Vec2) -> Rect {
        let mut r = self.inner.end_layout(layout_params, extent);
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }

    fn resolve_space(&self) -> Rect {
        let mut r = self.inner.resolve_space();
        r.x -= self.offset.x;
        r.y -= self.offset.y;
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layouts::column::ColumnLayout;
    use crate::layouts::CrossAlign;

    #[test]
    fn test_offset_content_extent_ignores_offset() {
        let offset = OffsetLayout {
            offset: Vec2::new(13.0, 27.0),
            inner: ColumnLayout {
                spacing: 0.0,
                align: CrossAlign::Start,
            },
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        state.layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // resolved_space shifted by offset (origin: -13.0, -27.0)
        assert_eq!(state.resolve_space(), Rect::new(-13.0, -27.0, 100.0, 100.0));
    }

    #[test]
    fn test_offset_layout() {
        let offset = OffsetLayout {
            offset: Vec2::new(5.0, 15.0),
            inner: ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            },
        };
        let bounds = Rect::new(10.0, 10.0, 100.0, 100.0);
        let mut state = offset.begin(bounds);

        let r1 = state.layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN);
        // Logic Y is 10.0. Actual Y = 10.0 - 15.0 = -5.0
        // Logic X is 10.0. Actual X = 10.0 - 5.0 = 5.0
        assert_eq!(r1, Rect::new(5.0, -5.0, 50.0, 20.0));

        let r2 = state.layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        // Logic Y is 10.0 + 20.0 + 10.0 = 40.0. Actual Y = 40.0 - 15.0 = 25.0
        assert_eq!(r2, Rect::new(5.0, 25.0, 40.0, 30.0));
    }

    #[test]
    fn test_deferred_offset_layout_lifecycle() {
        let offset = OffsetLayout {
            offset: Vec2::new(10.0, 20.0),
            inner: ColumnLayout {
                spacing: 5.0,
                align: CrossAlign::Start,
            },
        };
        let mut state = offset.begin(Rect::new(0.0, 0.0, 100.0, 100.0));

        let req = crate::layout::SizeReq {
            width: crate::layout::Extent::Fixed(50.0),
            height: crate::layout::Extent::Auto,
        };
        let (space, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);

        // Provisional space should be shifted by offset: space.x = 0.0 - 10.0 = -10.0
        assert_eq!(space.x, -10.0);
        assert_eq!(space.y, -20.0);

        let resolved_rect = token.end_layout(Vec2::new(50.0, 40.0));
        // Rect resolved in inner layout is at (0, 0, 50, 40), then shifted: (-10, -20, 50, 40)
        assert_eq!(resolved_rect, Rect::new(-10.0, -20.0, 50.0, 40.0));
    }
}
