use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelResult {
        pub draw: DrawCommands,
    }

    /// Low-level label widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn label<T: TextSystem>(spec: LabelSpec, text_system: &mut T) -> LabelResult {
        let mut draw = DrawCommands::new();

        let layout = text_system.prepare(spec.text, spec.style.size, spec.style.font);

        draw.push(DrawCmd::Text {
            rect: spec.rect,
            color: spec.style.text_color,
            handle: layout.handle,
        });

        if spec.style.rule {
            let y = spec.rect.y + spec.rect.h;
            draw.push(DrawCmd::StrokeLine {
                p0: Vec2::new(spec.rect.x, y),
                p1: Vec2::new(spec.rect.x + spec.rect.w, y),
                color: Color::linear_rgba(0.0, 0.0, 0.0, 0.12),
                width: 1.0,
            });
        }

        LabelResult { draw }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelStyle {
    pub size: f32,
    pub font: FontId,
    pub text_color: Color,
    pub rule: bool,
    pub rule_color: Color,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct LabelResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LabelSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub text: Option<&'a str>,
    pub style: Option<LabelStyle>,
}

impl<'a> LabelSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: LabelStyle) -> Self {
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
            self.style = Some(theme.label_style());
        }
        self
    }
    pub fn build(self) -> raw::LabelSpec<'a> {
        raw::LabelSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level label widget function using WidgetContext.
///
/// This function accepts a LabelSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::label function.
pub fn label<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: LabelSpecBuilder<'a>,
    layout_params: S::Params,
) -> LabelResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::label(spec, ctx.text_system);

    ctx.append_cmds(result.draw);

    LabelResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::LabelSpec;
    use super::*;
    use crate::{test_utils::DummyTextSys, text::TextHandle};

    struct RecordingTextSys {
        font: Option<FontId>,
    }

    impl TextSystem for RecordingTextSys {
        fn prepare(&mut self, _text: &str, _size: f32, font: FontId) -> crate::text::TextLayout {
            self.font = Some(font);
            crate::text::TextLayout {
                handle: TextHandle(0),
                size: Vec2::new(0.0, 0.0),
            }
        }

        fn measure_byte_x(&self, _handle: TextHandle, _byte_index: usize) -> f32 {
            0.0
        }

        fn hit_test_x(&self, _handle: TextHandle, _x_offset: f32) -> usize {
            0
        }
    }

    #[test]
    fn test_label_draws_text() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Hello",
            style: LabelStyle {
                size: 16.0,
                font: FontId(1),
                text_color: Color::WHITE,
                rule: false,
                rule_color: Color::WHITE,
            },
        };
        let res = raw::label(spec, &mut sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![DrawCmd::Text {
                rect: Rect::new(0.0, 0.0, 100.0, 50.0),
                color: Color::WHITE,
                handle: TextHandle(0),
            }])
        );
    }

    #[test]
    fn test_label_rule() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Section",
            style: LabelStyle {
                size: 14.0,
                font: FontId(1),
                text_color: Color::WHITE,
                rule: true,
                rule_color: Color::WHITE,
            },
        };
        let res = raw::label(spec, &mut sys);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::Text {
                    rect: Rect::new(0.0, 0.0, 100.0, 20.0),
                    color: Color::WHITE,
                    handle: TextHandle(0),
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 20.0),
                    p1: Vec2::new(100.0, 20.0),
                    color: Color::linear_rgba(0.0, 0.0, 0.0, 0.12),
                    width: 1.0,
                }
            ])
        );
    }

    #[test]
    fn test_label_passes_spec_font_to_text_system() {
        let mut sys = RecordingTextSys { font: None };
        let expected = FontId(42);
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "font",
            style: LabelStyle {
                size: 14.0,
                font: expected,
                text_color: Color::WHITE,
                rule: false,
                rule_color: Color::WHITE,
            },
        };

        let _ = raw::label(spec, &mut sys);

        assert_eq!(sys.font, Some(expected));
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = LabelSpecBuilder::new().text("test");
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.label_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let custom_style = LabelStyle {
            size: 99.0,
            font: FontId(99),
            text_color: Color::from_srgb_u8(1, 2, 3, 255),
            rule: true,
            rule_color: Color::from_srgb_u8(4, 5, 6, 255),
        };
        let builder = LabelSpecBuilder::new().text("test").style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(custom_style));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::label(
            &mut ctx,
            LabelSpecBuilder::new().text("X".into()).rect(custom_rect),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
