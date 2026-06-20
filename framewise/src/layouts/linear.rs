use std::marker::PhantomData;

use crate::layout::{
    Align, AxisBound, Layout, LayoutResult, LayoutSpace, LayoutState, LayoutToken, LayoutViolation,
    LayoutViolationKind, Size, SizeRequest, SpacerLayoutState,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MainAxisAlign {
    #[default]
    Append,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinearMain {
    Fill,
    Sized { size: Size, align: MainAxisAlign },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinearCross {
    Fill,
    Sized { size: Size, align: Align },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowLayoutParams {
    pub x: LinearMain,
    pub y: LinearCross,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnLayoutParams {
    pub x: LinearCross,
    pub y: LinearMain,
}

impl LinearMain {
    pub fn fixed(px: f32) -> Self {
        Self::Sized {
            size: Size::Fixed(px),
            align: MainAxisAlign::Append,
        }
    }

    pub fn auto() -> Self {
        Self::Sized {
            size: Size::Auto,
            align: MainAxisAlign::Append,
        }
    }

    pub fn fill() -> Self {
        Self::Fill
    }

    pub fn align(self, align: MainAxisAlign) -> Self {
        match self {
            Self::Fill => {
                panic!("Layout panic: cannot set alignment on LinearMain::Fill as align + fill is unrepresentable");
            }
            Self::Sized { size, .. } => Self::Sized { size, align },
        }
    }

    #[track_caller]
    pub(crate) fn resolve_size(
        self,
        intrinsic: Option<f32>,
        avail: AxisBound,
    ) -> LayoutResult<f32> {
        match self {
            LinearMain::Sized {
                size: Size::Fixed(px),
                ..
            } => LayoutResult::Ok(px),
            LinearMain::Sized {
                size: Size::Auto, ..
            } => match intrinsic {
                Some(preferred) => match avail {
                    AxisBound::Exact(_) => LayoutResult::Ok(preferred),
                    AxisBound::AtMost(w) => LayoutResult::Ok(preferred.min(w)),
                    AxisBound::Unbounded => LayoutResult::Ok(preferred),
                },
                None => LayoutResult::Fallback {
                    value: 0.0,
                    violation: LayoutViolation {
                        kind: LayoutViolationKind::MissingPreferredSize,
                        location: core::panic::Location::caller(),
                    },
                },
            },
            LinearMain::Fill => match avail {
                AxisBound::Exact(w) => LayoutResult::Ok(w),
                bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                    let val = match bound {
                        AxisBound::AtMost(w) => intrinsic.map(|i| i.min(w)).unwrap_or(0.0),
                        AxisBound::Unbounded => intrinsic.unwrap_or(0.0),
                        AxisBound::Exact(_) => unreachable!(),
                    };
                    LayoutResult::Fallback {
                        value: val,
                        violation: LayoutViolation {
                            kind: LayoutViolationKind::UnsatisfiableFill { bound },
                            location: core::panic::Location::caller(),
                        },
                    }
                }
            },
        }
    }
}

impl LinearCross {
    pub fn fixed(px: f32) -> Self {
        Self::Sized {
            size: Size::Fixed(px),
            align: Align::Start,
        }
    }

    pub fn auto() -> Self {
        Self::Sized {
            size: Size::Auto,
            align: Align::Start,
        }
    }

    pub fn fill() -> Self {
        Self::Fill
    }

    pub fn align(self, align: Align) -> Self {
        match self {
            Self::Fill => {
                panic!("Layout panic: cannot set alignment on LinearCross::Fill as align + fill is unrepresentable");
            }
            Self::Sized { size, .. } => Self::Sized { size, align },
        }
    }

    #[track_caller]
    pub(crate) fn resolve_size(
        self,
        intrinsic: Option<f32>,
        avail: AxisBound,
    ) -> LayoutResult<f32> {
        match self {
            LinearCross::Sized {
                size: Size::Fixed(px),
                ..
            } => LayoutResult::Ok(px),
            LinearCross::Sized {
                size: Size::Auto, ..
            } => match intrinsic {
                Some(preferred) => match avail {
                    AxisBound::Exact(_) => LayoutResult::Ok(preferred),
                    AxisBound::AtMost(w) => LayoutResult::Ok(preferred.min(w)),
                    AxisBound::Unbounded => LayoutResult::Ok(preferred),
                },
                None => LayoutResult::Fallback {
                    value: 0.0,
                    violation: LayoutViolation {
                        kind: LayoutViolationKind::MissingPreferredSize,
                        location: core::panic::Location::caller(),
                    },
                },
            },
            LinearCross::Fill => match avail {
                AxisBound::Exact(w) => LayoutResult::Ok(w),
                bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                    let val = match bound {
                        AxisBound::AtMost(w) => intrinsic.map(|i| i.min(w)).unwrap_or(0.0),
                        AxisBound::Unbounded => intrinsic.unwrap_or(0.0),
                        AxisBound::Exact(_) => unreachable!(),
                    };
                    LayoutResult::Fallback {
                        value: val,
                        violation: LayoutViolation {
                            kind: LayoutViolationKind::UnsatisfiableFill { bound },
                            location: core::panic::Location::caller(),
                        },
                    }
                }
            },
        }
    }

    #[track_caller]
    pub(crate) fn align_offset(self, resolved: f32, avail: AxisBound) -> LayoutResult<f32> {
        match self {
            LinearCross::Fill => LayoutResult::Ok(0.0),
            LinearCross::Sized { align, .. } => match align {
                Align::Start => LayoutResult::Ok(0.0),
                Align::Center => match avail {
                    AxisBound::Exact(w) => LayoutResult::Ok((w - resolved) * 0.5),
                    bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                        LayoutResult::Fallback {
                            value: 0.0,
                            violation: LayoutViolation {
                                kind: LayoutViolationKind::UnsatisfiableAlignment { align, bound },
                                location: core::panic::Location::caller(),
                            },
                        }
                    }
                },
                Align::End => match avail {
                    AxisBound::Exact(w) => LayoutResult::Ok(w - resolved),
                    bound @ (AxisBound::AtMost(_) | AxisBound::Unbounded) => {
                        LayoutResult::Fallback {
                            value: 0.0,
                            violation: LayoutViolation {
                                kind: LayoutViolationKind::UnsatisfiableAlignment { align, bound },
                                location: core::panic::Location::caller(),
                            },
                        }
                    }
                },
            },
        }
    }
}

impl RowLayoutParams {
    pub fn fixed(w: f32, h: f32) -> Self {
        Self {
            x: LinearMain::fixed(w),
            y: LinearCross::fixed(h),
        }
    }

    pub fn auto() -> Self {
        Self {
            x: LinearMain::auto(),
            y: LinearCross::auto(),
        }
    }

    pub fn x(mut self, x: LinearMain) -> Self {
        self.x = x;
        self
    }

    pub fn y(mut self, y: LinearCross) -> Self {
        self.y = y;
        self
    }

    pub fn fixed_x(self, px: f32) -> Self {
        self.x(LinearMain::fixed(px))
    }

    pub fn fixed_y(self, px: f32) -> Self {
        self.y(LinearCross::fixed(px))
    }

    pub fn auto_x(self) -> Self {
        self.x(LinearMain::auto())
    }

    pub fn auto_y(self) -> Self {
        self.y(LinearCross::auto())
    }

    pub fn fill_x(self) -> Self {
        self.x(LinearMain::fill())
    }

    pub fn fill_y(self) -> Self {
        self.y(LinearCross::fill())
    }

    pub fn align_x(mut self, align: MainAxisAlign) -> Self {
        self.x = self.x.align(align);
        self
    }

    pub fn align_y(mut self, align: Align) -> Self {
        self.y = self.y.align(align);
        self
    }
}

impl ColumnLayoutParams {
    pub fn fixed(w: f32, h: f32) -> Self {
        Self {
            x: LinearCross::fixed(w),
            y: LinearMain::fixed(h),
        }
    }

    pub fn auto() -> Self {
        Self {
            x: LinearCross::auto(),
            y: LinearMain::auto(),
        }
    }

    pub fn x(mut self, x: LinearCross) -> Self {
        self.x = x;
        self
    }

    pub fn y(mut self, y: LinearMain) -> Self {
        self.y = y;
        self
    }

    pub fn fixed_x(self, px: f32) -> Self {
        self.x(LinearCross::fixed(px))
    }

    pub fn fixed_y(self, px: f32) -> Self {
        self.y(LinearMain::fixed(px))
    }

    pub fn auto_x(self) -> Self {
        self.x(LinearCross::auto())
    }

    pub fn auto_y(self) -> Self {
        self.y(LinearMain::auto())
    }

    pub fn fill_x(self) -> Self {
        self.x(LinearCross::fill())
    }

    pub fn fill_y(self) -> Self {
        self.y(LinearMain::fill())
    }

    pub fn align_x(mut self, align: Align) -> Self {
        self.x = self.x.align(align);
        self
    }

    pub fn align_y(mut self, align: MainAxisAlign) -> Self {
        self.y = self.y.align(align);
        self
    }
}

impl From<Vec2> for RowLayoutParams {
    fn from(v: Vec2) -> Self {
        Self::fixed(v.x, v.y)
    }
}

impl From<Vec2> for ColumnLayoutParams {
    fn from(v: Vec2) -> Self {
        Self::fixed(v.x, v.y)
    }
}

impl<A: LinearAxis> Layout for LinearLayout<A> {
    type Params = A::Params;
    type State = LinearState<A>;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = A::orient_space(space.into());
        LinearState {
            cursor_main: space.origin_main,
            space,
            content_main: 0.0,
            content_cross: 0.0,
            pending_spacing: 0.0,
            is_closed: false,
            has_placed_child: false,
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
    type Params = RowLayoutParams;
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
    type Params = ColumnLayoutParams;
    type State = ColumnState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        LinearLayout::<Vertical>::new().begin(space)
    }
}

pub type RowState = LinearState<Horizontal>;
pub type ColumnState = LinearState<Vertical>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LinearSpacer {
    Between(f32),
    Always(f32),
}

impl LinearSpacer {
    pub fn always(size: f32) -> Self {
        Self::Always(size)
    }
}

impl From<f32> for LinearSpacer {
    fn from(size: f32) -> Self {
        LinearSpacer::Between(size)
    }
}

pub struct LinearState<A> {
    space: OrientedSpace,
    cursor_main: f32,
    /// End edge of the last child relative to the origin, excluding trailing spacing.
    content_main: f32,
    /// Largest child extent reached on the cross axis.
    content_cross: f32,
    pending_spacing: f32,
    is_closed: bool,
    has_placed_child: bool,
    _axis: PhantomData<fn() -> A>,
}

impl<A: LinearAxis> SpacerLayoutState for LinearState<A> {
    type SpacerParams = LinearSpacer;

    fn spacer(&mut self, params: Self::SpacerParams) {
        if self.is_closed {
            panic!("Layout panic: layout is closed, no more children can be placed after a MainAxisAlign::End child");
        }
        match params {
            LinearSpacer::Between(size) => {
                if self.has_placed_child {
                    self.pending_spacing += size;
                }
            }
            LinearSpacer::Always(size) => {
                self.cursor_main += self.pending_spacing;
                self.pending_spacing = 0.0;
                self.cursor_main += size;
                self.content_main = self.cursor_main - self.space.origin_main;
                self.has_placed_child = true;
            }
        }
    }
}

impl<A: LinearAxis> LayoutState for LinearState<A> {
    type Params = A::Params;

    fn layout(&mut self, layout_params: Self::Params, request: SizeRequest) -> LayoutResult<Rect> {
        let params = A::orient_params(layout_params);
        let pref = A::orient_intrinsic(request.preferred);
        self.place(params, pref)
    }

    fn begin_layout<'a>(
        &'a mut self,
        layout_params: Self::Params,
        _request: SizeRequest,
    ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>) {
        let params = A::orient_params(layout_params.clone());

        if self.is_closed {
            let main = match params.main {
                LinearMain::Fill => self.remaining_main_bound(),
                LinearMain::Sized {
                    size: Size::Fixed(size),
                    ..
                } => AxisBound::Exact(size),
                LinearMain::Sized {
                    size: Size::Auto, ..
                } => at_most_if_bounded(self.remaining_main_bound()),
            };
            let cross = match params.cross {
                LinearCross::Sized {
                    size: Size::Fixed(size),
                    ..
                } => AxisBound::Exact(size),
                LinearCross::Fill => self.space.cross,
                LinearCross::Sized {
                    size: Size::Auto, ..
                } => at_most_if_bounded(self.space.cross),
            };
            let space = OrientedSpace {
                origin_main: self.cursor_main + self.pending_spacing,
                origin_cross: self.space.origin_cross,
                main,
                cross,
            };
            let token = LayoutToken {
                state: self,
                params: layout_params,
            };
            return (
                LayoutResult::Fallback {
                    value: A::physical_space(space),
                    violation: LayoutViolation {
                        kind: LayoutViolationKind::LayoutClosed,
                        location: core::panic::Location::caller(),
                    },
                },
                token,
            );
        }

        let mut end_violation = None;
        let mut main_origin = self.cursor_main + self.pending_spacing;

        let main = match params.main {
            LinearMain::Fill => self.remaining_main_bound(),
            LinearMain::Sized { size, align } => match align {
                MainAxisAlign::Append => match size {
                    Size::Fixed(s) => AxisBound::Exact(s),
                    Size::Auto => at_most_if_bounded(self.remaining_main_bound()),
                },
                MainAxisAlign::End => match size {
                    Size::Auto => {
                        panic!(
                            "Layout panic: MainAxisAlign::End cannot be applied to an Auto-sized \
                             deferred child — its size is only known once the layout closes, and \
                             its already-emitted output cannot be shifted retroactively. Use a \
                             Fixed size."
                        );
                    }
                    Size::Fixed(s) => match self.space.main {
                        AxisBound::Exact(parent_size) => {
                            main_origin = self.space.origin_main + parent_size - s;
                            AxisBound::Exact(s)
                        }
                        bound => {
                            end_violation = Some(LayoutViolation {
                                kind: LayoutViolationKind::UnsatisfiableMainAxisEnd { bound },
                                location: core::panic::Location::caller(),
                            });
                            main_origin = self.cursor_main + self.pending_spacing;
                            AxisBound::Exact(s)
                        }
                    },
                },
            },
        };

        let cross = match params.cross {
            LinearCross::Sized {
                size: Size::Fixed(size),
                ..
            } => AxisBound::Exact(size),
            LinearCross::Fill => self.space.cross,
            LinearCross::Sized {
                size: Size::Auto, ..
            } => at_most_if_bounded(self.space.cross),
        };

        let resolved_cross = match params.cross {
            LinearCross::Sized {
                size: Size::Fixed(size),
                ..
            } => Some(size),
            LinearCross::Fill => match self.space.cross {
                AxisBound::Exact(size) => Some(size),
                AxisBound::AtMost(_) | AxisBound::Unbounded => None,
            },
            LinearCross::Sized {
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

        let (cross_offset, cross_align_violation) = params
            .cross
            .align_offset(resolved_cross.unwrap_or(0.0), self.space.cross)
            .into_parts();

        let space = OrientedSpace {
            origin_main: main_origin,
            origin_cross: self.space.origin_cross + cross_offset,
            main,
            cross,
        };
        let token = LayoutToken {
            state: self,
            params: layout_params,
        };
        (
            LayoutResult::from_parts(
                A::physical_space(space),
                cross_align_violation.or(end_violation),
            ),
            token,
        )
    }

    fn end_layout(&mut self, layout_params: Self::Params, extent: Vec2) -> LayoutResult<Rect> {
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
        if self.is_closed {
            let (main_size, _) = params
                .main
                .resolve_size(preferred.map(|p| p.main), self.remaining_main_bound())
                .into_parts();
            let (cross_size, _) = params
                .cross
                .resolve_size(preferred.map(|p| p.cross), self.space.cross)
                .into_parts();
            let (cross_offset, _) = params
                .cross
                .align_offset(cross_size, self.space.cross)
                .into_parts();
            let main_pos = self.cursor_main + self.pending_spacing;
            let cross_pos = self.space.origin_cross + cross_offset;
            let rect = A::physical_rect(OrientedRect {
                main: main_pos,
                cross: cross_pos,
                main_size,
                cross_size,
            });
            return LayoutResult::Fallback {
                value: rect,
                violation: LayoutViolation {
                    kind: LayoutViolationKind::LayoutClosed,
                    location: core::panic::Location::caller(),
                },
            };
        }

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

        let mut end_violation = None;
        let is_end = matches!(
            params.main,
            LinearMain::Sized {
                align: MainAxisAlign::End,
                ..
            }
        );

        let main_pos = if is_end {
            self.is_closed = true;
            match self.space.main {
                AxisBound::Exact(parent_size) => self.space.origin_main + parent_size - main_size,
                bound => {
                    end_violation = Some(LayoutViolation {
                        kind: LayoutViolationKind::UnsatisfiableMainAxisEnd { bound },
                        location: core::panic::Location::caller(),
                    });
                    self.cursor_main + self.pending_spacing
                }
            }
        } else {
            self.cursor_main + self.pending_spacing
        };

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
        self.has_placed_child = true;

        LayoutResult::from_parts(
            rect,
            A::first_violation(main_violation, cross_violation)
                .or(align_violation)
                .or(end_violation),
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
    main: LinearMain,
    cross: LinearCross,
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
    type Params: Clone;
    fn orient_space(space: LayoutSpace) -> OrientedSpace;
    fn physical_space(space: OrientedSpace) -> LayoutSpace;
    fn orient_params(params: Self::Params) -> OrientedPlacement;
    fn orient_intrinsic(size: Option<Vec2>) -> Option<OrientedSize>;
    fn orient_size(size: Vec2) -> OrientedSize;
    fn physical_rect(rect: OrientedRect) -> Rect;
    fn first_violation(
        main: Option<crate::layout::LayoutViolation>,
        cross: Option<crate::layout::LayoutViolation>,
    ) -> Option<crate::layout::LayoutViolation>;
}

impl LinearAxis for Horizontal {
    type Params = RowLayoutParams;

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

    fn orient_params(params: RowLayoutParams) -> OrientedPlacement {
        OrientedPlacement {
            main: params.x,
            cross: params.y,
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
    type Params = ColumnLayoutParams;

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

    fn orient_params(params: ColumnLayoutParams) -> OrientedPlacement {
        OrientedPlacement {
            main: params.y,
            cross: params.x,
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
    fn test_linear_param_fluent_helpers() {
        assert_eq!(
            RowLayoutParams::auto()
                .fixed_x(120.0)
                .fill_y()
                .align_x(MainAxisAlign::End),
            RowLayoutParams {
                x: LinearMain::fixed(120.0).align(MainAxisAlign::End),
                y: LinearCross::fill(),
            }
        );

        assert_eq!(
            ColumnLayoutParams::auto()
                .fixed_x(80.0)
                .fixed_y(32.0)
                .align_x(Align::Center)
                .align_y(MainAxisAlign::End),
            ColumnLayoutParams {
                x: LinearCross::fixed(80.0).align(Align::Center),
                y: LinearMain::fixed(32.0).align(MainAxisAlign::End),
            }
        );
    }

    #[test]
    fn test_row_layout() {
        let mut state = row().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state
            .layout(Vec2::new(30.0, 20.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        state.spacer(5.0.into());

        let r2 = state
            .layout(Vec2::new(20.0, 30.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(r2, Rect::new(45.0, 20.0, 20.0, 30.0));
    }

    #[test]
    #[should_panic(expected = "needs a preferred size request")]
    fn test_row_auto_without_intrinsic_panics() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let _ = state
            .layout(RowLayoutParams::auto(), SizeRequest::UNKNOWN)
            .unwrap();
    }

    #[test]
    fn test_row_auto_width_advances_cursor() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 50.0));
        let req = RowLayoutParams {
            x: LinearMain::auto(),
            y: LinearCross::fixed(40.0),
        };
        let r1 = state
            .layout(req, SizeRequest::preferred(Vec2::new(70.0, 16.0)))
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 70.0, 40.0));

        state.spacer(6.0.into());

        let r2 = state
            .layout(req, SizeRequest::preferred(Vec2::new(50.0, 16.0)))
            .unwrap();
        assert_eq!(r2, Rect::new(76.0, 0.0, 50.0, 40.0));
    }

    #[test]
    fn test_row_fill_width_remaining() {
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 200.0));
            let _ = state
                .layout(RowLayoutParams::fixed(30.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            let r = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fill(),
                        y: LinearCross::fixed(200.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();
            assert_eq!(r.w, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(200.0));
            let mut state = row().begin(space);
            let _ = state
                .layout(RowLayoutParams::fixed(30.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            let res = state.layout(
                RowLayoutParams {
                    x: LinearMain::fill(),
                    y: LinearCross::fixed(200.0),
                },
                SizeRequest::preferred(Vec2::new(90.0, 200.0)),
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
                .layout(RowLayoutParams::fixed(30.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            let req = RowLayoutParams {
                x: LinearMain::fill(),
                y: LinearCross::fixed(200.0),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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
                .layout(RowLayoutParams::fixed(30.0, 200.0), SizeRequest::UNKNOWN)
                .unwrap();
            let req = RowLayoutParams {
                x: LinearMain::fill(),
                y: LinearCross::fixed(200.0),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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
        let req = RowLayoutParams {
            x: LinearMain::fill(),
            y: LinearCross::fixed(40.0),
        };
        let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
    }

    #[test]
    fn test_row_content_extent() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 400.0, 100.0));
        let _ = state
            .layout(Vec2::new(30.0, 20.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        state.spacer(5.0.into());
        let _ = state
            .layout(Vec2::new(20.0, 40.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(state.resolve_space(), Rect::new(0.0, 0.0, 400.0, 100.0));
    }

    #[test]
    fn test_row_cross_alignment_exact() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Unbounded, AxisBound::Exact(80.0));

        let mut center_state = row().begin(space);
        let r1 = center_state
            .layout(
                RowLayoutParams {
                    x: LinearMain::fixed(40.0),
                    y: LinearCross::fixed(20.0).align(Align::Center),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 40.0, 40.0, 20.0));

        let mut end_state = row().begin(space);
        let r2 = end_state
            .layout(
                RowLayoutParams {
                    x: LinearMain::fixed(40.0),
                    y: LinearCross::fixed(30.0).align(Align::End),
                },
                SizeRequest::UNKNOWN,
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
                RowLayoutParams {
                    x: LinearMain::fixed(40.0),
                    y: LinearCross::fixed(20.0).align(Align::Center),
                },
                SizeRequest::UNKNOWN,
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
                RowLayoutParams {
                    x: LinearMain::fixed(40.0),
                    y: LinearCross::fixed(20.0).align(Align::End),
                },
                SizeRequest::UNKNOWN,
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

        let req = RowLayoutParams {
            x: LinearMain::auto(),
            y: LinearCross::fill(),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
        let space = space_res.unwrap();

        assert_eq!(space.x, 10.0);
        assert_eq!(space.y, 10.0);
        assert_eq!(space.width, AxisBound::Unbounded);
        assert_eq!(space.height, AxisBound::Exact(100.0));

        let resolved_rect = token.end_layout(Vec2::new(60.0, 40.0)).unwrap();
        assert_eq!(resolved_rect, Rect::new(10.0, 10.0, 60.0, 100.0));

        state.spacer(5.0.into());

        let next_rect = state
            .layout(
                RowLayoutParams {
                    x: LinearMain::fixed(30.0),
                    y: LinearCross::fixed(20.0).align(Align::Center),
                },
                SizeRequest::UNKNOWN,
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

        let req_fixed = RowLayoutParams {
            x: LinearMain::fixed(80.0),
            y: LinearCross::fill(),
        };
        let (space_f_res, _token) = state.begin_layout(req_fixed, SizeRequest::UNKNOWN);
        let space_f = space_f_res.unwrap();
        assert_eq!(space_f.width, AxisBound::Exact(80.0));
        assert_eq!(space_f.height, AxisBound::Exact(150.0));

        let _ = state
            .layout(RowLayoutParams::fixed(80.0, 100.0), SizeRequest::UNKNOWN)
            .unwrap();

        state.spacer(5.0.into());

        let req_auto = RowLayoutParams {
            x: LinearMain::auto(),
            y: LinearCross::fill(),
        };
        let (space_auto_res, _token) = state.begin_layout(req_auto, SizeRequest::UNKNOWN);
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

        let req = RowLayoutParams {
            x: LinearMain::fixed(80.0),
            y: LinearCross::auto().align(Align::Center),
        };
        let _ = state.begin_layout(req, SizeRequest::UNKNOWN);
    }

    #[test]
    fn test_row_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Unbounded, AxisBound::Exact(300.0));

        {
            let mut state = row().begin(parent_space);
            let req = RowLayoutParams {
                x: LinearMain::fixed(80.0),
                y: LinearCross::fixed(100.0).align(Align::Center),
            };
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space);
            let req = RowLayoutParams {
                x: LinearMain::fixed(80.0),
                y: LinearCross::fixed(100.0).align(Align::End),
            };
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space);
            let req = RowLayoutParams {
                x: LinearMain::fixed(80.0),
                y: LinearCross::fixed(100.0).align(Align::Center),
            };
            let (_, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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

        let req = RowLayoutParams::fixed(80.0, 100.0);

        {
            let mut state = row().begin(parent_space_exact);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 300.0);
        }

        {
            let mut state = row().begin(parent_space_at_most);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 100.0);
        }

        {
            let mut state = row().begin(parent_space_at_most_overflow);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 50.0);
        }

        {
            let mut state = row().begin(parent_space_unbounded);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().h, 100.0);
        }
    }
    #[test]
    fn test_column_layout() {
        let mut state = column().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let r1 = state
            .layout(Vec2::new(50.0, 20.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 50.0, 20.0));

        state.spacer(10.0.into());

        let r2 = state
            .layout(Vec2::new(40.0, 30.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(r2, Rect::new(0.0, 30.0, 40.0, 30.0));
    }

    #[test]
    fn test_column_auto_uses_intrinsic_preferred() {
        let mut state = column().begin(Rect::new(0.0, 0.0, 200.0, 500.0));
        let req = ColumnLayoutParams {
            x: LinearCross::fixed(120.0),
            y: LinearMain::auto(),
        };
        let intrinsic = SizeRequest::preferred(Vec2::new(80.0, 24.0));
        let r = state.layout(req, intrinsic).unwrap();
        assert_eq!(r, Rect::new(0.0, 0.0, 120.0, 24.0));
    }

    #[test]
    fn test_column_fill_cross_axis_uses_bounds_width() {
        let mut state = column().begin(Rect::new(5.0, 0.0, 200.0, 500.0));
        let req = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::fixed(30.0),
        };
        let r = state.layout(req, SizeRequest::UNKNOWN).unwrap();
        assert_eq!(r, Rect::new(5.0, 0.0, 200.0, 30.0));
    }

    #[test]
    fn test_column_fill_height_against_exact() {
        let mut state = column().begin(Rect::new(0.0, 10.0, 200.0, 500.0));
        let req = ColumnLayoutParams {
            x: LinearCross::fixed(120.0),
            y: LinearMain::fill(),
        };
        let r = state.layout(req, SizeRequest::UNKNOWN).unwrap();
        assert_eq!(r, Rect::new(0.0, 10.0, 120.0, 500.0));
    }

    #[test]
    fn test_column_fill_height_remaining() {
        {
            let mut state = column().begin(Rect::new(0.0, 0.0, 200.0, 100.0));
            let _ = state
                .layout(ColumnLayoutParams::fixed(200.0, 30.0), SizeRequest::UNKNOWN)
                .unwrap();
            let r = state
                .layout(
                    ColumnLayoutParams {
                        x: LinearCross::fixed(200.0),
                        y: LinearMain::fill(),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();
            assert_eq!(r.h, 70.0);
        }

        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::AtMost(100.0));
            let mut state = column().begin(space);
            let _ = state
                .layout(ColumnLayoutParams::fixed(200.0, 30.0), SizeRequest::UNKNOWN)
                .unwrap();
            let res = state.layout(
                ColumnLayoutParams {
                    x: LinearCross::fixed(200.0),
                    y: LinearMain::fill(),
                },
                SizeRequest::preferred(Vec2::new(200.0, 90.0)),
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
                .layout(ColumnLayoutParams::fixed(200.0, 30.0), SizeRequest::UNKNOWN)
                .unwrap();
            let req = ColumnLayoutParams {
                x: LinearCross::fixed(200.0),
                y: LinearMain::fill(),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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
                .layout(ColumnLayoutParams::fixed(200.0, 30.0), SizeRequest::UNKNOWN)
                .unwrap();
            let req = ColumnLayoutParams {
                x: LinearCross::fixed(200.0),
                y: LinearMain::fill(),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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
        let req = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::auto(),
        };
        let r1 = state
            .layout(req, SizeRequest::preferred(Vec2::new(80.0, 24.0)))
            .unwrap();
        assert_eq!(r1, Rect::new(0.0, 0.0, 200.0, 24.0));

        state.spacer(5.0.into());

        let r2 = state
            .layout(req, SizeRequest::preferred(Vec2::new(80.0, 30.0)))
            .unwrap();
        assert_eq!(r2, Rect::new(0.0, 29.0, 200.0, 30.0));
        assert!(r2.y.is_finite());
    }

    #[test]
    #[should_panic(expected = "Fill on an Unbounded axis is unsatisfiable")]
    fn test_column_fill_on_unbounded_axis_panics() {
        let mut state = column().begin(LayoutSpace::unbounded_height(0.0, 0.0, 100.0));
        let req = ColumnLayoutParams {
            x: LinearCross::fixed(50.0),
            y: LinearMain::fill(),
        };
        let _ = state
            .layout(req, SizeRequest::preferred(Vec2::new(50.0, 18.0)))
            .unwrap();
    }

    #[test]
    fn test_column_content_extent() {
        let mut state = column().begin(Rect::new(5.0, 7.0, 100.0, 500.0));
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
        let _ = state
            .layout(Vec2::new(40.0, 20.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        state.spacer(10.0.into());
        let _ = state
            .layout(Vec2::new(60.0, 30.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(state.resolve_space(), Rect::new(5.0, 7.0, 100.0, 500.0));
    }

    #[test]
    fn test_column_cross_alignment_exact() {
        let space = LayoutSpace::new(10.0, 10.0, AxisBound::Exact(100.0), AxisBound::Unbounded);

        let mut center_state = column().begin(space);
        let r1 = center_state
            .layout(
                ColumnLayoutParams {
                    x: LinearCross::fixed(40.0).align(Align::Center),
                    y: LinearMain::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(40.0, 10.0, 40.0, 20.0));

        let mut end_state = column().begin(space);
        let r2 = end_state
            .layout(
                ColumnLayoutParams {
                    x: LinearCross::fixed(30.0).align(Align::End),
                    y: LinearMain::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
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
                ColumnLayoutParams {
                    x: LinearCross::fixed(40.0).align(Align::Center),
                    y: LinearMain::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
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
                ColumnLayoutParams {
                    x: LinearCross::fixed(40.0).align(Align::End),
                    y: LinearMain::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
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

        let req = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::auto(),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
        let space = space_res.unwrap();

        assert_eq!(space.x, 0.0);
        assert_eq!(space.y, 0.0);
        assert_eq!(space.width, AxisBound::Exact(200.0));
        assert_eq!(space.height, AxisBound::Unbounded);

        let resolved_rect = token.end_layout(Vec2::new(80.0, 50.0)).unwrap();
        assert_eq!(resolved_rect, Rect::new(0.0, 0.0, 200.0, 50.0));

        state.spacer(8.0.into());

        let next_rect = state
            .layout(ColumnLayoutParams::fixed(40.0, 20.0), SizeRequest::UNKNOWN)
            .unwrap();
        assert_eq!(next_rect.y, 58.0);
    }

    #[test]
    fn test_column_begin_layout_propagates_bounds() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req_fixed = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::fixed(50.0),
        };
        let (space_f_res, _token) = state.begin_layout(req_fixed, SizeRequest::UNKNOWN);
        let space_f = space_f_res.unwrap();
        assert_eq!(space_f.width, AxisBound::Exact(200.0));
        assert_eq!(space_f.height, AxisBound::Exact(50.0));

        let _ = state
            .layout(ColumnLayoutParams::fixed(200.0, 50.0), SizeRequest::UNKNOWN)
            .unwrap();

        state.spacer(10.0.into());

        let req_fill = ColumnLayoutParams {
            x: LinearCross::auto(),
            y: LinearMain::fill(),
        };
        let (space_fill_res, _token) = state.begin_layout(req_fill, SizeRequest::UNKNOWN);
        let space_fill = space_fill_res.unwrap();
        assert_eq!(space_fill.width, AxisBound::AtMost(200.0));
        assert_eq!(space_fill.height, AxisBound::Exact(240.0));

        let req_auto = ColumnLayoutParams {
            x: LinearCross::auto(),
            y: LinearMain::auto(),
        };
        let (space_auto_res, _token) = state.begin_layout(req_auto, SizeRequest::UNKNOWN);
        let space_auto = space_auto_res.unwrap();
        assert_eq!(space_auto.width, AxisBound::AtMost(200.0));
        assert_eq!(space_auto.height, AxisBound::AtMost(240.0));
    }

    #[test]
    fn test_column_begin_layout_under_parent_at_most() {
        let parent_space =
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(150.0), AxisBound::AtMost(250.0));
        let mut state = column().begin(parent_space);

        let req1 = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::auto(),
        };
        let (space1_res, _token) = state.begin_layout(req1, SizeRequest::UNKNOWN);
        let space1 = space1_res.unwrap();
        assert_eq!(space1.width, AxisBound::AtMost(150.0));
        assert_eq!(space1.height, AxisBound::AtMost(250.0));

        let _ = state
            .layout(ColumnLayoutParams::fixed(100.0, 40.0), SizeRequest::UNKNOWN)
            .unwrap();

        state.spacer(5.0.into());

        let req2 = ColumnLayoutParams {
            x: LinearCross::auto(),
            y: LinearMain::fill(),
        };
        let (space2_res, _token) = state.begin_layout(req2, SizeRequest::UNKNOWN);
        let space2 = space2_res.unwrap();
        assert_eq!(space2.width, AxisBound::AtMost(150.0));
        assert_eq!(space2.height, AxisBound::AtMost(205.0));
    }

    #[test]
    fn test_deferred_column_center_align_fixed() {
        let parent_space =
            LayoutSpace::new(10.0, 10.0, AxisBound::Exact(200.0), AxisBound::Exact(300.0));
        let mut state = column().begin(parent_space);

        let req = ColumnLayoutParams {
            x: LinearCross::fixed(80.0).align(Align::Center),
            y: LinearMain::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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

        let req = ColumnLayoutParams {
            x: LinearCross::fill(),
            y: LinearMain::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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

        let req = ColumnLayoutParams {
            x: LinearCross::auto(),
            y: LinearMain::fixed(40.0),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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

        let req = ColumnLayoutParams {
            x: LinearCross::auto().align(Align::Center),
            y: LinearMain::fixed(40.0),
        };
        let _ = state.begin_layout(req, SizeRequest::UNKNOWN);
    }

    #[test]
    fn test_column_layout_content_extent_accounts_for_alignment_offset() {
        let parent_space =
            LayoutSpace::new(10.0, 20.0, AxisBound::Exact(400.0), AxisBound::Unbounded);

        {
            let mut state = column().begin(parent_space);
            let req = ColumnLayoutParams {
                x: LinearCross::fixed(180.0).align(Align::Center),
                y: LinearMain::fixed(32.0),
            };
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space);
            let req = ColumnLayoutParams {
                x: LinearCross::fixed(180.0).align(Align::End),
                y: LinearMain::fixed(32.0),
            };
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space);
            let req = ColumnLayoutParams {
                x: LinearCross::fixed(180.0).align(Align::Center),
                y: LinearMain::fixed(32.0),
            };
            let (_, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
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

        let req = ColumnLayoutParams::fixed(180.0, 32.0);

        {
            let mut state = column().begin(parent_space_exact);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 400.0);
        }

        {
            let mut state = column().begin(parent_space_at_most);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 180.0);
        }

        {
            let mut state = column().begin(parent_space_at_most_overflow);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 100.0);
        }

        {
            let mut state = column().begin(parent_space_unbounded);
            let _ = state.layout(req, SizeRequest::UNKNOWN).unwrap();
            assert_eq!(state.resolve_space().w, 180.0);
        }
    }

    #[test]
    fn test_spacers_accumulation() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let _ = state
            .layout(Vec2::new(10.0, 10.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        state.spacer(5.0.into());
        state.spacer(10.0.into());
        let r = state
            .layout(Vec2::new(10.0, 10.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        // Should be placed at: 10 + 5 + 10 = 25
        assert_eq!(r.x, 25.0);
    }

    #[test]
    fn test_trailing_spacer_ignored() {
        let space = LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Unbounded);
        let mut state = row().begin(space);
        let _ = state
            .layout(Vec2::new(10.0, 10.0).into(), SizeRequest::UNKNOWN)
            .unwrap();
        state.spacer(10.0.into());
        assert_eq!(state.resolve_space().w, 10.0); // Not 20!
    }

    #[test]
    fn test_row_end_placement() {
        let mut state = row().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state
            .layout(
                RowLayoutParams {
                    x: LinearMain::fixed(30.0),
                    y: LinearCross::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 20.0, 30.0, 20.0));

        let r2 = state
            .layout(
                RowLayoutParams {
                    x: LinearMain::fixed(25.0).align(MainAxisAlign::End),
                    y: LinearCross::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        // Flush to the right edge: 10 + 100 - 25 = 85
        assert_eq!(r2, Rect::new(85.0, 20.0, 25.0, 20.0));
    }

    #[test]
    fn test_column_end_placement() {
        let mut state = column().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let r1 = state
            .layout(
                ColumnLayoutParams {
                    x: LinearCross::fixed(20.0),
                    y: LinearMain::fixed(30.0),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        assert_eq!(r1, Rect::new(10.0, 20.0, 20.0, 30.0));

        let r2 = state
            .layout(
                ColumnLayoutParams {
                    x: LinearCross::fixed(20.0),
                    y: LinearMain::fixed(25.0).align(MainAxisAlign::End),
                },
                SizeRequest::UNKNOWN,
            )
            .unwrap();
        // Flush to the bottom edge: 20 + 100 - 25 = 95
        assert_eq!(r2, Rect::new(10.0, 95.0, 20.0, 25.0));
    }

    #[test]
    fn test_row_end_deferred() {
        let mut state = row().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let req = RowLayoutParams {
            x: LinearMain::fixed(25.0).align(MainAxisAlign::End),
            y: LinearCross::fixed(20.0),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
        let space = space_res.unwrap();
        // Provisional space starts at: 10 + 100 - 25 = 85
        assert_eq!(space.x, 85.0);
        assert_eq!(space.width, AxisBound::Exact(25.0));

        let r = token.end_layout(Vec2::new(25.0, 20.0)).unwrap();
        assert_eq!(r, Rect::new(85.0, 20.0, 25.0, 20.0));
    }

    #[test]
    fn test_column_end_deferred() {
        let mut state = column().begin(Rect::new(10.0, 20.0, 100.0, 100.0));
        let req = ColumnLayoutParams {
            x: LinearCross::fixed(20.0),
            y: LinearMain::fixed(25.0).align(MainAxisAlign::End),
        };
        let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
        let space = space_res.unwrap();
        // Provisional space starts at: 20 + 100 - 25 = 95
        assert_eq!(space.y, 95.0);
        assert_eq!(space.height, AxisBound::Exact(25.0));

        let r = token.end_layout(Vec2::new(20.0, 25.0)).unwrap();
        assert_eq!(r, Rect::new(10.0, 95.0, 20.0, 25.0));
    }

    #[test]
    fn test_spacer_ignored_before_end() {
        // Immediate path
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            state.spacer(15.0.into());
            let r = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                        y: LinearCross::fixed(20.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();
            // Lands exactly at 80.0, ignoring spacer
            assert_eq!(r.x, 80.0);
        }

        // Deferred path
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            state.spacer(15.0.into());
            let req = RowLayoutParams {
                x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                y: LinearCross::fixed(20.0),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
            let space = space_res.unwrap();
            assert_eq!(space.x, 80.0);
            let r = token.end_layout(Vec2::new(20.0, 20.0)).unwrap();
            assert_eq!(r.x, 80.0);
        }
    }

    #[test]
    fn test_spacer_before_first_child() {
        // Between spacer before first child is ignored
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            state.spacer(15.0.into());
            let r = state
                .layout(Vec2::new(10.0, 10.0).into(), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(r.x, 0.0);
        }

        // Always spacer before first child is not ignored
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            state.spacer(LinearSpacer::Always(15.0));
            let r = state
                .layout(Vec2::new(10.0, 10.0).into(), SizeRequest::UNKNOWN)
                .unwrap();
            assert_eq!(r.x, 15.0);
        }
    }

    #[test]
    fn test_closed_linear_layout_errors() {
        // Immediate path: child after End reports LayoutClosed
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            let _ = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                        y: LinearCross::fixed(20.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();

            let res = state.layout(RowLayoutParams::fixed(10.0, 10.0), SizeRequest::UNKNOWN);
            assert!(res.violation().is_some());
            assert_eq!(
                res.violation().unwrap().kind,
                LayoutViolationKind::LayoutClosed
            );
        }

        // Deferred path: child after End reports LayoutClosed
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            let _ = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                        y: LinearCross::fixed(20.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();

            let (space_res, _token) =
                state.begin_layout(RowLayoutParams::fixed(10.0, 10.0), SizeRequest::UNKNOWN);
            assert!(space_res.violation().is_some());
            assert_eq!(
                space_res.violation().unwrap().kind,
                LayoutViolationKind::LayoutClosed
            );
        }

        // spacer called after closed panics
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            let _ = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                        y: LinearCross::fixed(20.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();

            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                state.spacer(10.0.into());
            }));
            assert!(res.is_err());
        }

        // second End reports LayoutClosed
        {
            let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
            let _ = state
                .layout(
                    RowLayoutParams {
                        x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                        y: LinearCross::fixed(20.0),
                    },
                    SizeRequest::UNKNOWN,
                )
                .unwrap();

            let res = state.layout(
                RowLayoutParams {
                    x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                    y: LinearCross::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
            );
            assert!(res.violation().is_some());
            assert_eq!(
                res.violation().unwrap().kind,
                LayoutViolationKind::LayoutClosed
            );
        }
    }

    #[test]
    fn test_unsatisfiable_end_fallbacks() {
        // End under AtMost (Immediate)
        {
            let space =
                LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(100.0));
            let mut state = row().begin(space);
            let res = state.layout(
                RowLayoutParams {
                    x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                    y: LinearCross::fixed(20.0),
                },
                SizeRequest::UNKNOWN,
            );
            let (r, violation) = res.into_parts();
            // Falls back to append: placed at x = 0.0
            assert_eq!(r.x, 0.0);
            assert!(violation.is_some());
            assert!(matches!(
                violation.unwrap().kind,
                LayoutViolationKind::UnsatisfiableMainAxisEnd {
                    bound: AxisBound::AtMost(100.0)
                }
            ));
        }

        // End under Unbounded (Deferred)
        {
            let space = LayoutSpace::new(0.0, 0.0, AxisBound::Unbounded, AxisBound::Exact(100.0));
            let mut state = row().begin(space);
            let req = RowLayoutParams {
                x: LinearMain::fixed(20.0).align(MainAxisAlign::End),
                y: LinearCross::fixed(20.0),
            };
            let (space_res, token) = state.begin_layout(req, SizeRequest::UNKNOWN);
            let (prov_space, violation) = space_res.into_parts();
            // Falls back to append: origin_main starts at cursor (0.0)
            assert_eq!(prov_space.x, 0.0);
            assert!(violation.is_some());
            assert!(matches!(
                violation.unwrap().kind,
                LayoutViolationKind::UnsatisfiableMainAxisEnd {
                    bound: AxisBound::Unbounded
                }
            ));

            let r = token.end_layout(Vec2::new(20.0, 20.0)).value();
            assert_eq!(r.x, 0.0);
        }
    }

    #[test]
    #[should_panic(expected = "MainAxisAlign::End cannot be applied to an Auto-sized")]
    fn test_deferred_end_auto_panic() {
        let mut state = row().begin(Rect::new(0.0, 0.0, 100.0, 100.0));
        let req = RowLayoutParams {
            x: LinearMain::auto().align(MainAxisAlign::End),
            y: LinearCross::fixed(20.0),
        };
        let _ = state.begin_layout(req, SizeRequest::UNKNOWN);
    }
}
