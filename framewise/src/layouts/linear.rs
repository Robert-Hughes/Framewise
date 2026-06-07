use std::marker::PhantomData;

use crate::layout::{
    Align, AxisBound, IntrinsicSize, Layout, LayoutResult, LayoutSpace, LayoutState, LayoutToken,
    Placement, Placement2D, Size, SpacerLayoutState,
};
use crate::types::{Rect, Vec2};

// -- LinearLayout -----------------------------------------------------------

pub enum Horizontal {}
pub enum Vertical {}

struct LinearLayout<A> {
    _axis: PhantomData<fn() -> A>,
}

impl<A> LinearLayout<A> {
    pub fn new() -> Self {
        Self { _axis: PhantomData }
    }
}

impl<A: LinearAxis> Layout for LinearLayout<A> {
    type Params = Placement2D;
    type State = LinearState<A>;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = A::orient_space(space.into());
        LinearState {
            cursor_main: space.origin_main,
            space,
            content_main: 0.0,
            content_cross: 0.0,
            pending_spacing: 0.0,
            _axis: PhantomData,
        }
    }
}

#[derive(Default)]
pub struct RowLayout;

impl RowLayout {
    pub fn new() -> Self {
        RowLayout
    }
}

impl Layout for RowLayout {
    type Params = Placement2D;
    type State = RowState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        LinearLayout::<Horizontal>::new().begin(space)
    }
}

#[derive(Default)]
pub struct ColumnLayout;

impl ColumnLayout {
    pub fn new() -> Self {
        ColumnLayout
    }
}

impl Layout for ColumnLayout {
    type Params = Placement2D;
    type State = ColumnState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        LinearLayout::<Vertical>::new().begin(space)
    }
}

pub type RowState = LinearState<Horizontal>;
pub type ColumnState = LinearState<Vertical>;

pub struct LinearState<A> {
    space: OrientedSpace,
    cursor_main: f32,
    /// End edge of the last child relative to the origin, excluding trailing spacing.
    content_main: f32,
    /// Largest child extent reached on the cross axis.
    content_cross: f32,
    pending_spacing: f32,
    _axis: PhantomData<fn() -> A>,
}

impl<A: LinearAxis> SpacerLayoutState for LinearState<A> {
    type SpacerParams = f32;

    fn spacer(&mut self, size: Self::SpacerParams) {
        self.pending_spacing += size;
    }
}

impl<A: LinearAxis> LayoutState for LinearState<A> {
    type Params = Placement2D;

    fn layout(
        &mut self,
        layout_params: Placement2D,
        intrinsic: IntrinsicSize,
    ) -> LayoutResult<Rect> {
        let params = A::orient_params(layout_params);
        let pref = A::orient_intrinsic(intrinsic.preferred);
        self.place(params, pref)
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Placement2D,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>) {
        let params = A::orient_params(layout_params);

        let main = match params.main {
            Placement::Sized {
                size: Size::Fixed(size),
                ..
            } => AxisBound::Exact(size),
            Placement::Fill => self.remaining_main_bound(),
            Placement::Sized {
                size: Size::Auto, ..
            } => at_most_if_bounded(self.remaining_main_bound()),
        };

        let cross = match params.cross {
            Placement::Sized {
                size: Size::Fixed(size),
                ..
            } => AxisBound::Exact(size),
            Placement::Fill => self.space.cross,
            Placement::Sized {
                size: Size::Auto, ..
            } => at_most_if_bounded(self.space.cross),
        };

        let resolved_cross = match params.cross {
            Placement::Sized {
                size: Size::Fixed(size),
                ..
            } => Some(size),
            Placement::Fill => match self.space.cross {
                AxisBound::Exact(size) => Some(size),
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

        let (cross_offset, violation) = params
            .cross
            .align_offset(resolved_cross.unwrap_or(0.0), self.space.cross)
            .into_parts();

        let space = OrientedSpace {
            origin_main: self.cursor_main + self.pending_spacing,
            origin_cross: self.space.origin_cross + cross_offset,
            main,
            cross,
        };
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (
            LayoutResult::from_parts(A::physical_space(space), violation),
            token,
        )
    }

    fn end_layout(&mut self, layout_params: Placement2D, extent: Vec2) -> LayoutResult<Rect> {
        let params = A::orient_params(layout_params);
        let pref = Some(A::orient_size(extent));
        self.place(params, pref)
    }

    fn resolve_space(&self) -> Rect {
        let measured = OrientedSize {
            main: self.content_main,
            cross: self.content_cross,
        };
        A::physical_rect(self.space.resolve(measured))
    }
}

impl<A: LinearAxis> LinearState<A> {
    fn place(
        &mut self,
        params: OrientedPlacement,
        preferred: Option<OrientedSize>,
    ) -> LayoutResult<Rect> {
        let (main_size, main_violation) = params
            .main
            .resolve_size(preferred.map(|p| p.main), self.remaining_main_bound())
            .into_parts();
        let (cross_size, cross_violation) = params
            .cross
            .resolve_size(preferred.map(|p| p.cross), self.space.cross)
            .into_parts();

        let (cross_offset, align_violation) = params
            .cross
            .align_offset(cross_size, self.space.cross)
            .into_parts();
        let cross_pos = self.space.origin_cross + cross_offset;

        let main_pos = self.cursor_main + self.pending_spacing;

        let rect = A::physical_rect(OrientedRect {
            main: main_pos,
            cross: cross_pos,
            main_size,
            cross_size,
        });

        self.content_main = (main_pos + main_size) - self.space.origin_main;
        self.content_cross = self
            .content_cross
            .max((cross_pos + cross_size) - self.space.origin_cross);
        self.cursor_main = main_pos + main_size;
        self.pending_spacing = 0.0;

        LayoutResult::from_parts(
            rect,
            A::first_violation(main_violation, cross_violation).or(align_violation),
        )
    }

    fn remaining_main_bound(&self) -> AxisBound {
        let consumed = (self.cursor_main + self.pending_spacing) - self.space.origin_main;
        match self.space.main {
            AxisBound::Exact(size) => AxisBound::Exact((size - consumed).max(0.0)),
            AxisBound::AtMost(size) => AxisBound::AtMost((size - consumed).max(0.0)),
            AxisBound::Unbounded => AxisBound::Unbounded,
        }
    }
}

fn at_most_if_bounded(bound: AxisBound) -> AxisBound {
    match bound {
        AxisBound::Exact(size) | AxisBound::AtMost(size) => AxisBound::AtMost(size),
        AxisBound::Unbounded => AxisBound::Unbounded,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedSpace {
    origin_main: f32,
    origin_cross: f32,
    main: AxisBound,
    cross: AxisBound,
}

impl OrientedSpace {
    fn resolve(self, measured: OrientedSize) -> OrientedRect {
        OrientedRect {
            main: self.origin_main,
            cross: self.origin_cross,
            main_size: self.main.resolve(measured.main),
            cross_size: self.cross.resolve(measured.cross),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedPlacement {
    main: Placement,
    cross: Placement,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedSize {
    main: f32,
    cross: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrientedRect {
    main: f32,
    cross: f32,
    main_size: f32,
    cross_size: f32,
}

pub trait LinearAxis {
    fn orient_space(space: LayoutSpace) -> OrientedSpace;
    fn physical_space(space: OrientedSpace) -> LayoutSpace;
    fn orient_params(params: Placement2D) -> OrientedPlacement;
    fn orient_intrinsic(size: Option<Vec2>) -> Option<OrientedSize>;
    fn orient_size(size: Vec2) -> OrientedSize;
    fn physical_rect(rect: OrientedRect) -> Rect;
    fn first_violation(
        main: Option<crate::layout::LayoutViolation>,
        cross: Option<crate::layout::LayoutViolation>,
    ) -> Option<crate::layout::LayoutViolation>;
}

impl LinearAxis for Horizontal {
    fn orient_space(space: LayoutSpace) -> OrientedSpace {
        OrientedSpace {
            origin_main: space.x,
            origin_cross: space.y,
            main: space.width,
            cross: space.height,
        }
    }

    fn physical_space(space: OrientedSpace) -> LayoutSpace {
        LayoutSpace::new(
            space.origin_main,
            space.origin_cross,
            space.main,
            space.cross,
        )
    }

    fn orient_params(params: Placement2D) -> OrientedPlacement {
        OrientedPlacement {
            main: params.width,
            cross: params.height,
        }
    }

    fn orient_intrinsic(size: Option<Vec2>) -> Option<OrientedSize> {
        size.map(Self::orient_size)
    }

    fn orient_size(size: Vec2) -> OrientedSize {
        OrientedSize {
            main: size.x,
            cross: size.y,
        }
    }

    fn physical_rect(rect: OrientedRect) -> Rect {
        Rect::new(rect.main, rect.cross, rect.main_size, rect.cross_size)
    }

    fn first_violation(
        main: Option<crate::layout::LayoutViolation>,
        cross: Option<crate::layout::LayoutViolation>,
    ) -> Option<crate::layout::LayoutViolation> {
        main.or(cross)
    }
}

impl LinearAxis for Vertical {
    fn orient_space(space: LayoutSpace) -> OrientedSpace {
        OrientedSpace {
            origin_main: space.y,
            origin_cross: space.x,
            main: space.height,
            cross: space.width,
        }
    }

    fn physical_space(space: OrientedSpace) -> LayoutSpace {
        LayoutSpace::new(
            space.origin_cross,
            space.origin_main,
            space.cross,
            space.main,
        )
    }

    fn orient_params(params: Placement2D) -> OrientedPlacement {
        OrientedPlacement {
            main: params.height,
            cross: params.width,
        }
    }

    fn orient_intrinsic(size: Option<Vec2>) -> Option<OrientedSize> {
        size.map(Self::orient_size)
    }

    fn orient_size(size: Vec2) -> OrientedSize {
        OrientedSize {
            main: size.y,
            cross: size.x,
        }
    }

    fn physical_rect(rect: OrientedRect) -> Rect {
        Rect::new(rect.cross, rect.main, rect.cross_size, rect.main_size)
    }

    fn first_violation(
        main: Option<crate::layout::LayoutViolation>,
        cross: Option<crate::layout::LayoutViolation>,
    ) -> Option<crate::layout::LayoutViolation> {
        cross.or(main)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row() -> RowLayout {
        RowLayout::new()
    }

    fn column() -> ColumnLayout {
        ColumnLayout::new()
    }

    #[test]
    fn test_row_layout() {
        let mut state = row().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state
            .layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        state.spacer(5.0);

        let r2 = state
            .layout(Vec2::new(20.0, 30.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "needs an intrinsic measurement")]
    fn test_row_auto_without_intrinsic_panics() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let _ = state
            .layout(Placement2D::auto(), IntrinsicSize::UNKNOWN)
            .unwrap();
    }

    #[test]
    fn test_row_auto_width_advances_cursor() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 50.0));
        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(40.0),
        };
        let r1 = state
            .layout(req, IntrinsicSize::preferred(Vec2::new(70.0, 16.0)))
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 70.0, 40.0));

        state.spacer(6.0);

        let r2 = state
            .layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 16.0)))
            .unwrap();
        assert_eq!(r2, Rect::new(76.0, 0.0, 50.0, 40.0));
    }

    #[test]
    fn test_row_fill_width_remaining() {
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 200.0));
            let _ = state
                .layout(Placement2D::fixed(30.0, 200.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let r = state
                .layout(
                    Placement2D {
                        width: Placement::fill(),
                        height: Placement::fixed(200.0),
                    },
                    IntrinsicSize::UNKNOWN,
                )
                .unwrap();
            assert_eq!(r.w, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(200.0));
            let mut state = row().begin(space);
            let _ = state
                .layout(Placement2D::fixed(30.0, 200.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let res = state.layout(
                Placement2D {
                    width: Placement::fill(),
                    height: Placement::fixed(200.0),
                },
                IntrinsicSize::preferred(Vec2::new(90.0, 200.0)),
            );
            let (r, violation) = res.into_parts();
            assert_eq!(r.w, 70.0);
            assert!(violation.is_some());
        }
    }

    #[test]
    fn test_row_fill_width_remaining_deferred() {
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 200.0));
            let _ = state
                .layout(Placement2D::fixed(30.0, 200.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let req = Placement2D {
                width: Placement::fill(),
                height: Placement::fixed(200.0),
            };
            let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let space = space_res.unwrap();
            assert_eq!(space.width, AxisBound::Exact(70.0));

            let r = token.end_layout(Vec2::new(50.0, 200.0)).unwrap();
            assert_eq!(r.w, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(200.0));
            let mut state = row().begin(space);
            let _ = state
                .layout(Placement2D::fixed(30.0, 200.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let req = Placement2D {
                width: Placement::fill(),
                height: Placement::fixed(200.0),
            };
            let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let space = space_res.unwrap();
            assert_eq!(space.width, AxisBound::AtMost(70.0));

            let res = token.end_layout(Vec2::new(90.0, 200.0));
            let (r, violation) = res.into_parts();
            assert_eq!(r.w, 70.0);
            assert!(violation.is_some());
        }
    }

    #[test]
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_row_fill_on_unbounded_axis_panics() {
        let mut state = row().begin(LayoutSpace::unbounded_width(0.0, 0.0, 40.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(40.0),
        };
        let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
    }

    #[test]
    fn test_row_content_extent() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let _ = state
            .layout(Vec2::new(30.0, 20.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        state.spacer(5.0);
        let _ = state
            .layout(Vec2::new(20.0, 40.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(state.resolve_space(), Rect::new(0.0, 0.0, 400.0, 100.0));
    }

    #[test]
    fn test_row_cross_alignment_exact() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Exact(80.0));

        let mut center_state = row().begin(space);
        let r1 = center_state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_y(Align::Center),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 40.0, 40.0, 20.0));

        let mut end_state = row().begin(space);
        let r2 = end_state
            .layout(
                Placement2D::fixed(40.0, 30.0).align_y(Align::End),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r2, Rect::new(10.0, 60.0, 40.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::AtMost(80.0));
        let mut state = row().begin(space);
        let _ = state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_y(Align::Center),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_row_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = row().begin(space);
        let _ = state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_y(Align::End),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
    }

    #[test]
    fn test_deferred_row_layout_lifecycle() {
        let mut state = row().begin(LayoutSpace::new(
            10.0,
            10.0,
            AxisBound::Unbounded,
            AxisBound::Exact(100.0),
        ));

        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();

        assert_eq!(space.x, 10.0);
        assert_eq!(space.y, 10.0);
        assert_eq!(space.width, AxisBound::Unbounded);
        assert_eq!(space.height, AxisBound::Exact(100.0));

        let resolved_rect = token.end_layout(Vec2::new(60.0, 40.0)).unwrap();
        assert_eq!(resolved_rect, Rect::new(10.0, 10.0, 60.0, 100.0));

        state.spacer(5.0);

        let next_rect = state
            .layout(
                Placement2D::fixed(30.0, 20.0).align_y(Align::Center),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
        assert_eq!(next_rect.x, 75.0);
        assert_eq!(next_rect.y, 50.0);
    }

    #[test]
    fn test_row_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(400.0), AxisBound::Exact(150.0));
        let mut state = row().begin(parent_space);

        let req_fixed = Placement2D {
            width: Placement::fixed(80.0),
            height: Placement::fill(),
        };
        let (space_f_res, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        let space_f = space_f_res.unwrap();
        assert_eq!(space_f.width, AxisBound::Exact(80.0));
        assert_eq!(space_f.height, AxisBound::Exact(150.0));

        let _ = state
            .layout(Placement2D::fixed(80.0, 100.0), IntrinsicSize::UNKNOWN)
            .unwrap();

        state.spacer(5.0);

        let req_auto = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space_auto_res, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        let space_auto = space_auto_res.unwrap();
        assert_eq!(space_auto.width, AxisBound::AtMost(315.0));
        assert_eq!(space_auto.height, AxisBound::Exact(150.0));
    }

    #[test]
    #[should_panic(expected = "cannot be applied to an Auto-sized")]
    fn test_deferred_row_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = row().begin(parent_space);

        let req = Placement2D {
            width: Placement::fixed(80.0),
            height: Placement::auto().align(Align::Center),
        };
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_row_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Exact(300.0));

        {
            let mut state = row().begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::Center);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::End);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space);
            let req = Placement2D::fixed(80.0, 100.0).align_y(Align::Center);
            let (_, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(80.0, 100.0)).unwrap();
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

        {
            let mut state = row().begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 100.0);
        }

        {
            let mut state = row().begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 50.0);
        }

        {
            let mut state = row().begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 100.0);
        }
    }
    #[test]
    fn test_column_layout() {
        let mut state = column().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state
            .layout(Vec2::new(50.0, 20.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        state.spacer(10.0);

        let r2 = state
            .layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_column_auto_uses_intrinsic_preferred() {
        let mut state = column().begin(Rect::new(0.0, 0.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fixed(120.0),
            height: Placement::auto(),
        };
        let intrinsic = IntrinsicSize::preferred(Vec2::new(80.0, 24.0));
        let r = state.layout(req, intrinsic).unwrap();
        assert_eq!(r, Rect::new(0.0, 0.0, 120.0, 24.0));
    }

    #[test]
    fn test_column_fill_cross_axis_uses_bounds_width() {
        let mut state = column().begin(Rect::new(5.0, 0.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(30.0),
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
        assert_eq!(r, Rect::new(5.0, 0.0, 200.0, 30.0));
    }

    #[test]
    fn test_column_fill_height_against_exact() {
        let mut state = column().begin(Rect::new(0.0, 10.0, 200.0, 500.0));
        let req = Placement2D {
            width: Placement::fixed(120.0),
            height: Placement::fill(),
        };
        let r = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
        assert_eq!(r, Rect::new(0.0, 10.0, 120.0, 500.0));
    }

    #[test]
    fn test_column_fill_height_remaining() {
        {
            let mut state = column().begin(Rect::new(0.0, 0.0, 200.0, 100.0));
            let _ = state
                .layout(Placement2D::fixed(200.0, 30.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let r = state
                .layout(
                    Placement2D {
                        width: Placement::fixed(200.0),
                        height: Placement::fill(),
                    },
                    IntrinsicSize::UNKNOWN,
                )
                .unwrap();
            assert_eq!(r.h, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::AtMost(100.0));
            let mut state = column().begin(space);
            let _ = state
                .layout(Placement2D::fixed(200.0, 30.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let res = state.layout(
                Placement2D {
                    width: Placement::fixed(200.0),
                    height: Placement::fill(),
                },
                IntrinsicSize::preferred(Vec2::new(200.0, 90.0)),
            );
            let (r, violation) = res.into_parts();
            assert_eq!(r.h, 70.0);
            assert!(violation.is_some());
        }
    }

    #[test]
    fn test_column_fill_height_remaining_deferred() {
        {
            let mut state = column().begin(Rect::new(0.0, 0.0, 200.0, 100.0));
            let _ = state
                .layout(Placement2D::fixed(200.0, 30.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let req = Placement2D {
                width: Placement::fixed(200.0),
                height: Placement::fill(),
            };
            let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let space = space_res.unwrap();
            assert_eq!(space.height, AxisBound::Exact(70.0));

            let r = token.end_layout(Vec2::new(200.0, 50.0)).unwrap();
            assert_eq!(r.h, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::AtMost(100.0));
            let mut state = column().begin(space);
            let _ = state
                .layout(Placement2D::fixed(200.0, 30.0), IntrinsicSize::UNKNOWN)
                .unwrap();
            let req = Placement2D {
                width: Placement::fixed(200.0),
                height: Placement::fill(),
            };
            let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let space = space_res.unwrap();
            assert_eq!(space.height, AxisBound::AtMost(70.0));

            let res = token.end_layout(Vec2::new(200.0, 90.0));
            let (r, violation) = res.into_parts();
            assert_eq!(r.h, 70.0);
            assert!(violation.is_some());
        }
    }

    #[test]
    fn test_column_unbounded_height_resolves_concrete() {
        let mut state = column().begin(LayoutSpace::unbounded_height(0.0, 0.0, 200.0));
        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
        };
        let r1 = state
            .layout(req, IntrinsicSize::preferred(Vec2::new(80.0, 24.0)))
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 200.0, 24.0));

        state.spacer(5.0);

        let r2 = state
            .layout(req, IntrinsicSize::preferred(Vec2::new(80.0, 30.0)))
            .unwrap();
        assert_eq!(r2, Rect::new(0.0, 29.0, 200.0, 30.0));
        assert!(r2.y.is_finite());
    }

    #[test]
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_column_fill_on_unbounded_axis_panics() {
        let mut state = column().begin(LayoutSpace::unbounded_height(0.0, 0.0, 100.0));
        let req = Placement2D {
            width: Placement::fixed(50.0),
            height: Placement::fill(),
        };
        let _ = state
            .layout(req, IntrinsicSize::preferred(Vec2::new(50.0, 18.0)))
            .unwrap();
    }

    #[test]
    fn test_column_content_extent() {
        let mut state = column().begin(Rect::new(5.0, 7.0, 100.0, 500.0));
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
        let _ = state
            .layout(Vec2::new(40.0, 20.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        state.spacer(10.0);
        let _ = state
            .layout(Vec2::new(60.0, 30.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
    }

    #[test]
    fn test_column_cross_alignment_exact() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Exact(100.0), AxisBound::Unbounded);

        let mut center_state = column().begin(space);
        let r1 = center_state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_x(Align::Center),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(40.0, 10.0, 40.0, 20.0));

        let mut end_state = column().begin(space);
        let r2 = end_state
            .layout(
                Placement2D::fixed(30.0, 20.0).align_x(Align::End),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r2, Rect::new(80.0, 10.0, 30.0, 20.0));
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_at_most() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let mut state = column().begin(space);
        let _ = state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_x(Align::Center),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
    }

    #[test]
    #[should_panic(expected = "requires AxisBound::Exact")]
    fn test_column_cross_alignment_panic_unbounded() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Unbounded);
        let mut state = column().begin(space);
        let _ = state
            .layout(
                Placement2D::fixed(40.0, 20.0).align_x(Align::End),
                IntrinsicSize::UNKNOWN,
            )
            .unwrap();
    }

    #[test]
    fn test_deferred_column_layout_lifecycle() {
        let mut state = column().begin(LayoutSpace::new(
            0.0,
            0.0,
            AxisBound::Exact(200.0),
            AxisBound::Unbounded,
        ));

        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
        };
        let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();

        assert_eq!(space.x, 0.0);
        assert_eq!(space.y, 0.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));
        assert_eq!(space.height, AxisBound::Unbounded);

        let resolved_rect = token.end_layout(Vec2::new(80.0, 50.0)).unwrap();
        assert_eq!(resolved_rect, Rect::new(0.0, 0.0, 200.0, 50.0));

        state.spacer(8.0);

        let next_rect = state
            .layout(Placement2D::fixed(40.0, 20.0), IntrinsicSize::UNKNOWN)
            .unwrap();
        assert_eq!(next_rect.y, 58.0);
    }

    #[test]
    fn test_column_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req_fixed = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(50.0),
        };
        let (space_f_res, _token) = state.begin_layout(req_fixed, IntrinsicSize::UNKNOWN);
        let space_f = space_f_res.unwrap();
        assert_eq!(space_f.width, AxisBound::Exact(200.0));
        assert_eq!(space_f.height, AxisBound::Exact(50.0));

        let _ = state
            .layout(Placement2D::fixed(200.0, 50.0), IntrinsicSize::UNKNOWN)
            .unwrap();

        state.spacer(10.0);

        let req_fill = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space_fill_res, _token) = state.begin_layout(req_fill, IntrinsicSize::UNKNOWN);
        let space_fill = space_fill_res.unwrap();
        assert_eq!(space_fill.width, AxisBound::AtMost(200.0));
        assert_eq!(space_fill.height, AxisBound::Exact(240.0));

        let req_auto = Placement2D {
            width: Placement::auto(),
            height: Placement::auto(),
        };
        let (space_auto_res, _token) = state.begin_layout(req_auto, IntrinsicSize::UNKNOWN);
        let space_auto = space_auto_res.unwrap();
        assert_eq!(space_auto.width, AxisBound::AtMost(200.0));
        assert_eq!(space_auto.height, AxisBound::AtMost(240.0));
    }

    #[test]
    fn test_column_begin_layout_under_parent_at_most() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(150.0), AxisBound::AtMost(250.0));
        let mut state = column().begin(parent_space);

        let req1 = Placement2D {
            width: Placement::fill(),
            height: Placement::auto(),
        };
        let (space1_res, _token) = state.begin_layout(req1, IntrinsicSize::UNKNOWN);
        let space1 = space1_res.unwrap();
        assert_eq!(space1.width, AxisBound::AtMost(150.0));
        assert_eq!(space1.height, AxisBound::AtMost(250.0));

        let _ = state
            .layout(Placement2D::fixed(100.0, 40.0), IntrinsicSize::UNKNOWN)
            .unwrap();

        state.spacer(5.0);

        let req2 = Placement2D {
            width: Placement::auto(),
            height: Placement::fill(),
        };
        let (space2_res, _token) = state.begin_layout(req2, IntrinsicSize::UNKNOWN);
        let space2 = space2_res.unwrap();
        assert_eq!(space2.width, AxisBound::AtMost(150.0));
        assert_eq!(space2.height, AxisBound::AtMost(205.0));
    }

    #[test]
    fn test_deferred_column_center_align_fixed() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req = Placement2D {
            width: Placement::fixed(80.0).align(Align::Center),
            height: Placement::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();
        assert_eq!(space.x, 70.0);
        assert_eq!(space.width, AxisBound::Exact(80.0));

        let rect = token.end_layout(Vec2::new(80.0, 40.0)).unwrap();
        assert_eq!(rect, Rect::new(70.0, 10.0, 80.0, 40.0));
    }

    #[test]
    fn test_deferred_column_end_align_fill() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req = Placement2D {
            width: Placement::fill(),
            height: Placement::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();
        assert_eq!(space.x, 10.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));

        let rect = token.end_layout(Vec2::new(200.0, 40.0)).unwrap();
        assert_eq!(rect, Rect::new(10.0, 10.0, 200.0, 40.0));
    }

    #[test]
    fn test_deferred_column_start_align_auto() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req = Placement2D {
            width: Placement::auto(),
            height: Placement::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
        let space = space_res.unwrap();
        assert_eq!(space.x, 10.0);
        assert_eq!(space.width, AxisBound::AtMost(200.0));

        let rect = token.end_layout(Vec2::new(80.0, 40.0)).unwrap();
        assert_eq!(rect, Rect::new(10.0, 10.0, 80.0, 40.0));
    }

    #[test]
    #[should_panic(expected = "cannot be applied to an Auto-sized")]
    fn test_deferred_column_center_align_auto_panic() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req = Placement2D {
            width: Placement::auto().align(Align::Center),
            height: Placement::fixed(40.0),
        };
        let _ = state.begin_layout(req, IntrinsicSize::UNKNOWN);
    }

    #[test]
    fn test_column_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(400.0), AxisBound::Unbounded);

        {
            let mut state = column().begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::Center);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::End);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space);
            let req = Placement2D::fixed(180.0, 32.0).align_x(Align::Center);
            let (_, token) = state.begin_layout(req, IntrinsicSize::UNKNOWN);
            let _ = token.end_layout(Vec2::new(180.0, 32.0)).unwrap();
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

        {
            let mut state = column().begin(parent_space_exact);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space_at_most);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 180.0);
        }

        {
            let mut state = column().begin(parent_space_at_most_overflow);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 100.0);
        }

        {
            let mut state = column().begin(parent_space_unbounded);
            let _ = state.layout(req, IntrinsicSize::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 180.0);
        }
    }

    #[test]
    fn test_spacers_accumulation() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let _ = state
            .layout(Vec2::new(10.0, 10.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        state.spacer(5.0);
        state.spacer(10.0);
        let r = state
            .layout(Vec2::new(10.0, 10.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        // Should be placed at: 10 + 5 + 10 = 25
        assert_eq!(r.x, 25.0);
    }

    #[test]
    fn test_trailing_spacer_ignored() {
        let space = LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let mut state = row().begin(space);
        let _ = state
            .layout(Vec2::new(10.0, 10.0).into(), IntrinsicSize::UNKNOWN)
            .unwrap();
        state.spacer(10.0);
        assert_eq!(state.resolve_space().w, 10.0); // Not 20!
    }
}
