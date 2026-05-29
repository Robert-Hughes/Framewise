use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::KeycapStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapResult {
        pub draw: DrawCommands,
        pub content_bounds: Rect,
    }

    /// Low-level keycap widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn keycap<'a, T: TextSystem>(spec: KeycapSpec<'a>, text_system: &mut T) -> KeycapResult {
        let mut cmds = DrawCommands::new();

        // Background + border
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.style.background,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: spec.style.border,
            width: spec.style.border_width,
        });
        // Bottom shadow line
        let shadow_rect = Rect::new(
            spec.rect.x + spec.style.shadow_offset,
            spec.rect.y + spec.rect.h,
            spec.rect.w - spec.style.shadow_offset,
            spec.style.shadow_height,
        );
        cmds.push(DrawCmd::FillRect {
            rect: shadow_rect,
            color: spec.style.shadow,
        });

        // text, centered
        if !spec.text.is_empty() {
            let layout = text_system.prepare(spec.text, spec.style.text_size, spec.style.font);
            let tx = spec.rect.x + (spec.rect.w - layout.size.x) / 2.0;
            let ty = spec.rect.y + (spec.rect.h - layout.size.y) / 2.0;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(tx, ty, layout.size.x, layout.size.y),
                color: spec.style.text_color,
                handle: layout.handle,
            });
        }

        KeycapResult {
            draw: cmds,
            content_bounds: spec.rect.inset(spec.style.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a keycap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeycapStyle {
    pub background: Color,
    pub shadow: Color,
    pub shadow_offset: f32,
    pub shadow_height: f32,
    pub border: Color,
    pub border_width: f32,
    pub text_color: Color,
    pub text_size: f32,
    pub font: FontId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeycapSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub text: Option<&'a str>,
    pub style: Option<KeycapStyle>,
}

impl<'a> KeycapSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: KeycapStyle) -> Self {
        self.style = Some(style);
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
        if self.style.is_none() {
            self.style = Some(theme.keycap_style());
        }
        self
    }

    pub fn build(self) -> raw::KeycapSpec<'a> {
        raw::KeycapSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level keycap widget function using WidgetContext.
///
/// This function accepts a KeycapSpecBuilder and calls the low-level raw::keycap function.
pub fn keycap<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: KeycapSpecBuilder<'a>,
    layout_params: S::Params,
) -> KeycapResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::keycap(spec, ctx.text_system);
    ctx.append_cmds(result.draw);
    KeycapResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::KeycapSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_keycap_visual() {
        let mut text_system = DummyTextSys;
        let custom_bg = Color::from_srgb_u8(240, 240, 240, 255);
        let custom_shadow = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_border = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_text = Color::from_srgb_u8(50, 50, 50, 255);
        let spec = KeycapSpec {
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            text: "K",
            style: KeycapStyle {
                background: custom_bg,
                border: custom_border,
                border_width: 1.0,
                shadow: custom_shadow,
                shadow_offset: 1.0,
                shadow_height: 2.0,
                text_color: custom_text,
                text_size: 14.0,
                font: FontId(0),
            },
        };
        let res = raw::keycap(spec, &mut text_system);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                    color: custom_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                    color: custom_border,
                    width: 1.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(1.0, 30.0, 29.0, 2.0),
                    color: custom_shadow,
                },
                DrawCmd::Text {
                    rect: Rect::new(11.0, 7.0, 8.0, 16.0),
                    color: custom_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = KeycapSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().font, theme.mono_font);
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let explicit_style = KeycapStyle {
            background: Color::WHITE,
            shadow: Color::BLACK,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            border: Color::WHITE,
            border_width: 1.0,
            text_color: Color::WHITE,
            text_size: 14.0,
            font: FontId(99),
        };
        let builder = KeycapSpecBuilder::new().style(explicit_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(explicit_style));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
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
        let result = super::keycap(
            &mut ctx,
            KeycapSpecBuilder::new()
                .text("X")
                .style(KeycapStyle {
                    background: Color::WHITE,
                    shadow: Color::BLACK,
                    shadow_offset: 1.0,
                    shadow_height: 2.0,
                    border: Color::WHITE,
                    border_width: 1.0,
                    text_color: Color::WHITE,
                    text_size: 14.0,
                    font: FontId(0),
                })
                .rect(custom_rect),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }

    #[test]
    fn test_keycap_bounds_and_content_bounds() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let custom_border_width = 3.5;
        let result = super::keycap(
            &mut ctx,
            KeycapSpecBuilder::new().text("X").style(KeycapStyle {
                background: Color::WHITE,
                shadow: Color::BLACK,
                shadow_offset: 1.0,
                shadow_height: 2.0,
                border: Color::WHITE,
                border_width: custom_border_width,
                text_color: Color::WHITE,
                text_size: 14.0,
                font: FontId(0),
            }),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, layout_rect);
        assert_eq!(
            result.layout.content_bounds,
            layout_rect.inset(custom_border_width)
        );
    }
}
