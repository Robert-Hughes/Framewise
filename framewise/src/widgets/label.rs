use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
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
        pub size: f32,
        pub font: FontId,
        pub text_color: Color,
        /// Draw a hairline rule at the bottom of the rect.
        pub rule: bool,
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

        let layout = text_system.prepare(&spec.text, spec.size, spec.font);

        draw.push(DrawCmd::Text {
            rect: spec.rect,
            color: spec.text_color,
            handle: layout.handle,
        });

        if spec.rule {
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

// ── Result ───────────────────────────────────────────────────────────────────

pub struct LabelResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LabelSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub size: Option<f32>,
    pub font: Option<FontId>,
    pub text_color: Option<Color>,
    pub rect: Option<Rect>,
    pub rule: Option<bool>,
}

impl<'a> LabelSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }
    pub fn rule(mut self, rule: bool) -> Self {
        self.rule = Some(rule);
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
        if self.size.is_none() {
            self.size = Some(theme.text_md);
        }
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        if self.text_color.is_none() {
            self.text_color = Some(theme.ink);
        }
        self
    }
    pub fn build(self) -> raw::LabelSpec<'a> {
        raw::LabelSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            text: self.text.expect("text not set — call .text()"),
            size: self
                .size
                .expect("size not set — call .size() or defaults_from_theme()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            text_color: self
                .text_color
                .expect("text_color not set — call .text_color() or defaults_from_theme()"),
            rule: self.rule.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level label widget function using WidgetContext.
///
/// This function accepts a LabelSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::label function.
pub fn label<
    'a,
    T: TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
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
            size: 16.0,
            font: FontId(1),
            text_color: Color::WHITE,
            rule: false,
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
            size: 14.0,
            font: FontId(1),
            text_color: Color::WHITE,
            rule: true,
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
            size: 14.0,
            font: expected,
            text_color: Color::WHITE,
            rule: false,
        };

        let _ = raw::label(spec, &mut sys);

        assert_eq!(sys.font, Some(expected));
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = LabelSpecBuilder::new().text("test");
        assert!(builder.size.is_none());
        assert!(builder.font.is_none());
        assert!(builder.text_color.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.size, Some(theme.text_md));
        assert_eq!(builder.font, Some(theme.sans_font));
        assert_eq!(builder.text_color, Some(theme.ink));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = LabelSpecBuilder::new()
            .text("test")
            .size(99.0)
            .font(FontId(99))
            .text_color(Color::from_srgb_u8(1, 2, 3, 255));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.size, Some(99.0));
        assert_eq!(builder.font, Some(FontId(99)));
        assert_eq!(builder.text_color, Some(Color::from_srgb_u8(1, 2, 3, 255)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
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
