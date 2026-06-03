use crate::layout::{
    Align, AxisBound, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken, Placement, Size,
};
use crate::types::{Rect, Vec2};

// ── SplitRow ──────────────────────────────────────────────────────────────

/// A declared-structure row that divides its width into `count` **equal** cells.
/// Unlike [`RowLayout`], which advances a cursor by each child's own
/// width, `SplitRow` knows the child count up front, so every slot's width is a
/// constant `(width - spacing*(count-1)) / count` — the classic "three buttons
/// sharing a row in equal thirds" case.
///
/// Declaring `count` is what makes this one-pass: an equal split needs to know
/// how many ways to divide (a *future-sibling* fact for a plain row), and the
/// declaration converts that into a known constant resolved from available space
/// alone. No measure-all or emit-reorder is required.
///
/// Each child's **width is imposed** (the slot), so children only declare their
/// cross-axis ([`Placement`] height) sizing via [`Params`](LayoutState::Params).
///
/// Dividing space requires a committed far edge, so `SplitRow` requires the
/// available width to be [`AxisBound::Exact`] and panics otherwise — the same
/// "position & distribution policies require `Exact`" rule that governs `Fill`
/// and alignment.
pub struct SplitRow {
    /// Number of equal cells to divide the width into. The caller is expected to
    /// emit exactly this many children.
    pub count: usize,
    /// Gap between adjacent cells.
    pub spacing: f32,
}

impl Layout for SplitRow {
    type Params = Placement;
    type State = SplitRowState;

    fn begin(self, space: impl Into<LayoutSpace>) -> Self::State {
        let space = space.into();
        let avail = match space.width {
            AxisBound::Exact(w) => w,
            AxisBound::AtMost(_) => panic!(
                "Layout panic: SplitRow requires AxisBound::Exact width (a committed frame to \
                 divide into equal cells), but width is AtMost"
            ),
            AxisBound::Unbounded => panic!(
                "Layout panic: SplitRow requires AxisBound::Exact width (a committed frame to \
                 divide into equal cells), but width is Unbounded"
            ),
        };
        let slot_w = if self.count == 0 {
            0.0
        } else {
            let gaps = self.spacing * (self.count.saturating_sub(1)) as f32;
            ((avail - gaps) / self.count as f32).max(0.0)
        };
        SplitRowState {
            space,
            slot_w,
            spacing: self.spacing,
            count: self.count,
            index: 0,
            content_h: 0.0,
        }
    }
}

pub struct SplitRowState {
    space: LayoutSpace,
    /// Width of each equal cell, computed once in `begin`.
    slot_w: f32,
    spacing: f32,
    count: usize,
    /// Index of the next cell to fill.
    index: usize,
    /// Tallest child placed so far (cross axis).
    content_h: f32,
}

impl SplitRowState {
    /// X origin of cell `i`.
    fn slot_x(&self, i: usize) -> f32 {
        self.space.x + i as f32 * (self.slot_w + self.spacing)
    }
}

impl LayoutState for SplitRowState {
    type Params = Placement;

    fn layout(&mut self, height: Placement, intrinsic: IntrinsicSize) -> Rect {
        debug_assert!(
            self.index < self.count,
            "SplitRow: emitted child #{} but only {} cell(s) were declared",
            self.index + 1,
            self.count
        );
        let pref = intrinsic.preferred;
        let w = self.slot_w;
        let h = height.resolve_size(pref.map(|p| p.y), self.space.height);

        let x = self.slot_x(self.index);
        let y_offset = height.align_offset(h, self.space.height);
        let y = self.space.y + y_offset;

        let r = Rect::new(x, y, w, h);
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.index += 1;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        height: Placement,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        debug_assert!(
            self.index < self.count,
            "SplitRow: emitted child #{} but only {} cell(s) were declared",
            self.index + 1,
            self.count
        );
        let width = AxisBound::Exact(self.slot_w);
        let bound_height = match height {
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

        let h = match height {
            Placement::Sized {
                size: Size::Fixed(h),
                ..
            } => Some(h),
            Placement::Fill => match self.space.height {
                AxisBound::Exact(h) => Some(h),
                AxisBound::AtMost(_) | AxisBound::Unbounded => {
                    let _ = height.resolve_size(None, self.space.height);
                    None
                }
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

        let y = self.space.y + height.align_offset(h.unwrap_or(0.0), self.space.height);

        let space = LayoutSpace::new(self.slot_x(self.index), y, width, bound_height);
        let token = LayoutToken {
            state: self,
            params: height,
        };
        (space, token)
    }

    fn end_layout(&mut self, height: Placement, extent: Vec2) -> Rect {
        let w = self.slot_w;
        let h = height.resolve_size(Some(extent.y), self.space.height);

        let x = self.slot_x(self.index);
        let y_offset = height.align_offset(h, self.space.height);
        let y = self.space.y + y_offset;

        let r = Rect::new(x, y, w, h);
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.index += 1;
        r
    }

    fn resolve_space(&self) -> Rect {
        self.space.resolve(Vec2::new(0.0, self.content_h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_row_equal_thirds() {
        let mut state = SplitRow {
            count: 3,
            spacing: 5.0,
        }
        .begin(Rect::new(10.0, 20.0, 100.0, 40.0));
        let a = state.layout(Placement::fill(), IntrinsicSize::UNKNOWN);
        let b = state.layout(Placement::fill(), IntrinsicSize::UNKNOWN);
        let c = state.layout(Placement::fill(), IntrinsicSize::UNKNOWN);
        assert_eq!(a, Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(b, Rect::new(45.0, 20.0, 30.0, 40.0));
        assert_eq!(c, Rect::new(80.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn test_split_row_fixed_height_aligns_center() {
        let mut state = SplitRow {
            count: 2,
            spacing: 0.0,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 50.0));
        let r = state.layout(
            Placement::fixed(20.0).align(Align::Center),
            IntrinsicSize::UNKNOWN,
        );
        assert_eq!(r, Rect::new(0.0, 15.0, 50.0, 20.0));
    }

    #[test]
    fn test_split_row_deferred_slot_gets_exact_width() {
        let mut state = SplitRow {
            count: 4,
            spacing: 0.0,
        }
        .begin(Rect::new(0.0, 0.0, 80.0, 30.0));
        let (space, token) = state.begin_layout(Placement::fill(), IntrinsicSize::UNKNOWN);
        assert_eq!(space.width, AxisBound::Exact(20.0)); // 80 / 4
        assert_eq!(space.x, 0.0);
        let r = token.end_layout(Vec2::new(999.0, 30.0));
        assert_eq!(r, Rect::new(0.0, 0.0, 20.0, 30.0));
        let next = state.layout(Placement::fill(), IntrinsicSize::UNKNOWN);
        assert_eq!(next.x, 20.0);
    }

    #[test]
    #[should_panic(expected = "SplitRow requires AxisBound::Exact width")]
    fn test_split_row_panics_on_non_exact_width() {
        let space = LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(40.0));
        let _ = SplitRow {
            count: 3,
            spacing: 0.0,
        }
        .begin(space);
    }
}
