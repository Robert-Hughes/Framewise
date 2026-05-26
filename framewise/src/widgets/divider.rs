use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    types::{Color, Rect, Vec2},
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

// ── Spec Builder ───────────────────────────────────────────────────────────────

pub struct DividerSpecBuilder {
    pub color: Option<Color>,
    pub width: Option<f32>,
    pub rect: Option<Rect>,
}

impl Default for DividerSpecBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl DividerSpecBuilder {
    pub fn new() -> Self {
        Self {
            color: None,
            width: None,
            rect: None,
        }
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }
    pub fn apply_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.color.is_none() {
            self.color = Some(theme.line);
        }
        self
    }
    pub fn build(self) -> DividerSpec {
        DividerSpec {
            rect: self.rect.unwrap_or_default(),
            color: self.color.unwrap_or(Color::WHITE),
            width: self.width.unwrap_or(1.0),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level divider widget function using WidgetContext.
///
/// This function accepts a DividerSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::divider function.
pub fn divider<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: DividerSpecBuilder,
) -> DividerInfo {
    let rect = ctx.layout(layout_params);
    let spec = builder.rect(rect).apply_theme(&ctx.theme).build();
    let result = raw::divider(spec);

    ctx.append_cmds(result.draw.0);

    DividerInfo {
        layout: result.layout,
    }
}

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
        let res = raw::divider(spec);

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
