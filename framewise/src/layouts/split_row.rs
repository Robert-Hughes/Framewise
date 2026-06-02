use crate::layout::{
    AxisBound, CrossAlign, Extent, IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken,
};
use crate::types::{Rect, Vec2};

// ── SplitRow ──────────────────────────────────────────────────────────────

/// A declared-structure row that divides its width into `count` **equal** cells
/// (Phase 4). Unlike [`RowLayout`], which advances a cursor by each child's own
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
/// cross-axis ([`Extent`] height) sizing via [`Params`](LayoutState::Params).
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
    /// Cross-axis (height) alignment of each child within the row, like
    /// [`RowLayout::align`].
    pub align: CrossAlign,
}

impl Layout for SplitRow {
    type Params = Extent;
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
            align: self.align,
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
    align: CrossAlign,
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
    type Params = Extent;

    fn layout(&mut self, height: Extent, intrinsic: IntrinsicSize) -> Rect {
        debug_assert!(
            self.index < self.count,
            "SplitRow: emitted child #{} but only {} cell(s) were declared",
            self.index + 1,
            self.count
        );
        let pref = intrinsic.preferred;
        let w = self.slot_w;
        let h = height.resolve(pref.map(|p| p.y), self.space.height);

        let x = self.slot_x(self.index);
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

        let r = Rect::new(x, y, w, h);
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.index += 1;
        r
    }

    fn begin_layout<'a>(
        &'a mut self,
        height: Extent,
        _intrinsic: IntrinsicSize,
    ) -> (LayoutSpace, LayoutToken<'a, Self>) {
        debug_assert!(
            self.index < self.count,
            "SplitRow: emitted child #{} but only {} cell(s) were declared",
            self.index + 1,
            self.count
        );
        // Width is always the imposed cell (Exact); only the cross axis is the
        // child's to choose.
        let width = AxisBound::Exact(self.slot_w);
        let bound_height = match height {
            Extent::Fixed(h) => AxisBound::Exact(h),
            Extent::Fill => self.space.height,
            Extent::Auto => match self.space.height {
                AxisBound::Exact(h) | AxisBound::AtMost(h) => AxisBound::AtMost(h),
                AxisBound::Unbounded => AxisBound::Unbounded,
            },
        };

        // Concrete height (if known) for alignment; mirrors RowState.
        let h = match height {
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
                    _ => panic!("Layout panic: CrossAlign::Center requires AxisBound::Exact available space on the cross axis"),
                },
                None => panic!("Layout panic: CrossAlign::Center cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
            CrossAlign::End => match h {
                Some(val) => match self.space.height {
                    AxisBound::Exact(height) => self.space.y + height - val,
                    _ => panic!("Layout panic: CrossAlign::End requires AxisBound::Exact available space on the cross axis"),
                },
                None => panic!("Layout panic: CrossAlign::End cannot align dynamic (Auto/Fill) size child in begin_layout"),
            },
        };

        let space = LayoutSpace::new(self.slot_x(self.index), y, width, bound_height);
        let token = LayoutToken {
            state: self,
            params: height,
        };
        (space, token)
    }

    fn end_layout(&mut self, height: Extent, extent: Vec2) -> Rect {
        let w = self.slot_w;
        let h = height.resolve(Some(extent.y), self.space.height);

        let x = self.slot_x(self.index);
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

        let r = Rect::new(x, y, w, h);
        self.content_h = self.content_h.max((y + h) - self.space.y);
        self.index += 1;
        r
    }

    fn resolve_space(&self) -> Rect {
        // Width is the committed (Exact) frame, so the measured width is ignored;
        // height is the tallest cell.
        self.space.resolve(Vec2::new(0.0, self.content_h))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_row_equal_thirds() {
        // Width 100, 3 cells, spacing 5 → gaps 10, usable 90, slot 30 each.
        let mut state = SplitRow {
            count: 3,
            spacing: 5.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(10.0, 20.0, 100.0, 40.0));
        let a = state.layout(Extent::Fill, IntrinsicSize::UNKNOWN);
        let b = state.layout(Extent::Fill, IntrinsicSize::UNKNOWN);
        let c = state.layout(Extent::Fill, IntrinsicSize::UNKNOWN);
        // Equal 30-wide cells, 35px apart (slot + spacing); Fill height = row height.
        assert_eq!(a, Rect::new(10.0, 20.0, 30.0, 40.0));
        assert_eq!(b, Rect::new(45.0, 20.0, 30.0, 40.0));
        assert_eq!(c, Rect::new(80.0, 20.0, 30.0, 40.0));
    }

    #[test]
    fn test_split_row_fixed_height_aligns_center() {
        let mut state = SplitRow {
            count: 2,
            spacing: 0.0,
            align: CrossAlign::Center,
        }
        .begin(Rect::new(0.0, 0.0, 100.0, 50.0));
        // Fixed 20-tall child centered in the 50-tall row → y = 15.
        let r = state.layout(Extent::Fixed(20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(r, Rect::new(0.0, 15.0, 50.0, 20.0));
    }

    #[test]
    fn test_split_row_deferred_slot_gets_exact_width() {
        // A nested (deferred) child in a SplitRow cell is handed the imposed slot
        // width as Exact, regardless of its own content; end_layout returns the cell.
        let mut state = SplitRow {
            count: 4,
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(Rect::new(0.0, 0.0, 80.0, 30.0));
        let (space, token) = state.begin_layout(Extent::Fill, IntrinsicSize::UNKNOWN);
        assert_eq!(space.width, AxisBound::Exact(20.0)); // 80 / 4
        assert_eq!(space.x, 0.0);
        let r = token.end_layout(Vec2::new(999.0, 30.0)); // oversized content ignored on main axis
        assert_eq!(r, Rect::new(0.0, 0.0, 20.0, 30.0));
        // Next cell advances by the slot width.
        let next = state.layout(Extent::Fill, IntrinsicSize::UNKNOWN);
        assert_eq!(next.x, 20.0);
    }

    #[test]
    #[should_panic(expected = "SplitRow requires AxisBound::Exact width")]
    fn test_split_row_panics_on_non_exact_width() {
        let space = LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(40.0));
        let _ = SplitRow {
            count: 3,
            spacing: 0.0,
            align: CrossAlign::Start,
        }
        .begin(space);
    }
}
