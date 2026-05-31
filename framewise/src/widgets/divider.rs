use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerSpec {
        pub rect: Rect,
        pub color: Color,
        pub width: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerResult {
        pub draw: DrawCommands,
    }

    /// Low-level divider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn divider(spec: DividerSpec) -> DividerResult {
        let mut cmds = DrawCommands::new();
        let mid_y = spec.rect.y + spec.rect.h * 0.5;
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(spec.rect.x, mid_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, mid_y),
            color: spec.color,
            width: spec.width,
        });
        DividerResult { draw: cmds }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DividerResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DividerSpecBuilder {
    pub rect: Option<Rect>,
    pub color: Option<Color>,
    pub width: Option<f32>,
}

impl DividerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.color.is_none() {
            self.color = Some(theme.line);
        }
        self
    }
    pub fn build(self) -> raw::DividerSpec {
        raw::DividerSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            color: self
                .color
                .expect("color not set — call .color() or defaults_from_theme()"),
            width: self.width.unwrap_or(1.0),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level divider widget function using WidgetContext.
///
/// This function accepts a DividerSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::divider function.
pub fn divider<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: DividerSpecBuilder,
    layout_params: S::Params,
) -> DividerResult {
    let layout_rect = ctx.layout_state.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::divider(spec);

    ctx.append_cmds(result.draw);

    DividerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::DividerSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

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

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_color() {
        let theme = crate::theme::Theme::framewise();
        let builder = DividerSpecBuilder::new();
        assert!(builder.color.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.color, Some(theme.line));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_color() {
        let theme = crate::theme::Theme::framewise();
        let sentinel = Color::from_srgb_u8(1, 2, 3, 255);
        let builder = DividerSpecBuilder::new().color(sentinel);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.color, Some(sentinel));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::divider(
            &mut ctx,
            DividerSpecBuilder::new().rect(custom_rect),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
