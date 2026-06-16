use super::TextMetrics;
use crate::{layout::Align, types::Rect};

/// Which measured text geometry a widget should align inside its content rect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextContentBasis {
    /// Align the logical text block, based on shaped advances and line height.
    Logical,
    /// Align the visible ink bounds for optical/icon-like placement.
    Ink,
}

/// Horizontal or vertical placement policy for a prepared text block inside a containing box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ContentPlacement {
    /// Span the layout's available extent in that direction, allowing line alignment to take effect.
    #[default]
    Fill,
    /// Shrink-wrap the content in that direction and align it.
    Align(Align),
}

/// Placement of a prepared text block inside a widget's own content rect.
///
/// This is widget-local content placement. It is distinct from
/// [`TextFlow::line_align`], which positions individual lines inside a text
/// layout block, and from layout [`Align`], which positions a whole widget
/// inside its parent layout space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextContentPlacement {
    pub x: ContentPlacement,
    pub y: ContentPlacement,
    pub basis: TextContentBasis,
}

impl TextContentPlacement {
    pub const TOP_LEFT: Self = Self::logical(
        ContentPlacement::Align(Align::Start),
        ContentPlacement::Align(Align::Start),
    );
    pub const CENTER: Self = Self::logical(
        ContentPlacement::Align(Align::Center),
        ContentPlacement::Align(Align::Center),
    );
    pub const INK_CENTER: Self = Self::ink(
        ContentPlacement::Align(Align::Center),
        ContentPlacement::Align(Align::Center),
    );

    pub const fn logical(x: ContentPlacement, y: ContentPlacement) -> Self {
        Self {
            x,
            y,
            basis: TextContentBasis::Logical,
        }
    }

    pub const fn ink(x: ContentPlacement, y: ContentPlacement) -> Self {
        Self {
            x,
            y,
            basis: TextContentBasis::Ink,
        }
    }

    /// Resolve the logical text block rect used for owned text layout.
    pub fn resolve_rect(self, content_rect: Rect, metrics: TextMetrics) -> Rect {
        let logical = metrics.logical_size;
        let ink = metrics.ink_bounds;

        let (basis_x, basis_w) = match self.basis {
            TextContentBasis::Logical => (0.0, logical.x),
            TextContentBasis::Ink if ink.w > 0.0 => (ink.x, ink.w),
            TextContentBasis::Ink => (0.0, logical.x),
        };
        let (basis_y, basis_h) = match self.basis {
            TextContentBasis::Logical => (0.0, logical.y),
            TextContentBasis::Ink if ink.h > 0.0 => (ink.y, ink.h),
            TextContentBasis::Ink => (0.0, logical.y),
        };

        let (x, w) = match self.x {
            ContentPlacement::Fill => (content_rect.x, content_rect.w),
            ContentPlacement::Align(align) => {
                let x = content_rect.x + align_offset(content_rect.w, basis_w, align) - basis_x;
                let w = logical.x.min(content_rect.w);
                (x, w)
            }
        };

        let (y, h) = match self.y {
            ContentPlacement::Fill => (content_rect.y, content_rect.h),
            ContentPlacement::Align(align) => {
                let y = content_rect.y + align_offset(content_rect.h, basis_h, align) - basis_y;
                let h = logical.y.min(content_rect.h);
                (y, h)
            }
        };

        Rect::new(x, y, w, h)
    }
}

fn align_offset(available: f32, content: f32, align: Align) -> f32 {
    match align {
        Align::Start => 0.0,
        Align::Center => (available - content) * 0.5,
        Align::End => available - content,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Vec2;

    #[test]
    fn text_content_placement_keeps_prepare_rect_inside_content_size() {
        let content_rect = Rect::new(10.0, 20.0, 4.0, 12.0);
        let overflow_metrics = TextMetrics {
            logical_size: Vec2::new(9.0, 18.0),
            ink_bounds: Rect::new(0.0, 0.0, 8.0, 16.0),
            line_count: 2,
            truncated_horizontal: true,
            truncated_vertical: true,
            lines: Vec::new(),
        };

        assert_eq!(
            TextContentPlacement::TOP_LEFT.resolve_rect(content_rect, overflow_metrics),
            Rect::new(10.0, 20.0, 4.0, 12.0)
        );
    }

    #[test]
    fn text_content_placement_fill_uses_full_content_size() {
        let content_rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let metrics = TextMetrics {
            logical_size: Vec2::new(40.0, 16.0),
            ink_bounds: Rect::new(0.0, 0.0, 40.0, 16.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };

        let placement = TextContentPlacement {
            x: ContentPlacement::Fill,
            y: ContentPlacement::Fill,
            basis: TextContentBasis::Logical,
        };

        assert_eq!(
            placement.resolve_rect(content_rect, metrics),
            Rect::new(10.0, 20.0, 100.0, 50.0)
        );
    }

    #[test]
    fn text_content_placement_fill_x_align_y() {
        let content_rect = Rect::new(10.0, 20.0, 100.0, 50.0);
        let metrics = TextMetrics {
            logical_size: Vec2::new(40.0, 16.0),
            ink_bounds: Rect::new(0.0, 0.0, 40.0, 16.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };

        let placement = TextContentPlacement {
            x: ContentPlacement::Fill,
            y: ContentPlacement::Align(Align::Center),
            basis: TextContentBasis::Logical,
        };

        assert_eq!(
            placement.resolve_rect(content_rect, metrics),
            Rect::new(10.0, 37.0, 100.0, 16.0)
        );
    }
}
