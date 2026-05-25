use crate::{
    draw::{DrawCmd, DrawCommands},
    types::Color,
    types::{Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level divider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn divider(spec: DividerSpec) -> DividerResult {
        let mut draw = DrawCommands::new();
        let mid_y = spec.rect.y + spec.rect.h * 0.5;
        draw.push(DrawCmd::StrokeLine {
            p0: Vec2::new(spec.rect.x, mid_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, mid_y),
            color: spec.color,
            width: spec.width,
        });
        DividerResult {
            draw,
            layout: LayoutInfo::new(spec.rect, spec.rect),
        }
    }
}

pub struct DividerSpec {
    pub rect: Rect,
    pub color: Color,
    pub width: f32,
}

pub struct DividerResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct DividerInfo {
    pub layout: LayoutInfo,
}

impl DividerResult {
    pub fn into_parts(self) -> (DrawCommands, DividerInfo) {
        (
            self.draw,
            DividerInfo {
                layout: self.layout,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level divider widget function using WidgetContext.
///
/// This function accepts a DividerSpec and calls the low-level raw::divider function.
pub fn divider<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    spec: DividerSpec,
) -> DividerInfo {
    let result = raw::divider(spec);
    
    ctx.append_cmds(result.draw.0);
    
    DividerInfo {
        layout: result.layout,
    }
}

// ── Re-export raw function for direct use ───────────────────────────────────────────
pub use raw::divider as divider_raw;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_visual() {
        let spec = DividerSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 10.0),
            color: Color::WHITE,
            width: 1.0,
        };
        let res = divider(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![DrawCmd::StrokeLine {
                p0: Vec2::new(0.0, 5.0),
                p1: Vec2::new(100.0, 5.0),
                color: Color::WHITE,
                width: 1.0,
            }])
        );
    }
}
