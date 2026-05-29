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
        /// Background fill (default: paper_elev).
        pub background: Color,
        pub shadow: Color,
        pub shadow_offset: f32,
        pub shadow_height: f32,
        /// Border color.
        pub border: Color,
        pub border_width: f32,
        /// Label text color.
        pub text_color: Color,
        pub text_size: f32,
        pub font: FontId,
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
        let mut draw = DrawCommands::new();

        // Background + border
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.background,
        });
        draw.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: spec.border,
            width: spec.border_width,
        });
        // Bottom shadow line
        let shadow_rect = Rect::new(
            spec.rect.x + spec.shadow_offset,
            spec.rect.y + spec.rect.h,
            spec.rect.w - spec.shadow_offset,
            spec.shadow_height,
        );
        draw.push(DrawCmd::FillRect {
            rect: shadow_rect,
            color: spec.shadow,
        });

        // text, centered
        if !spec.text.is_empty() {
            let layout = text_system.prepare(spec.text, spec.text_size, spec.font);
            let tx = spec.rect.x + (spec.rect.w - layout.size.x) / 2.0;
            let ty = spec.rect.y + (spec.rect.h - layout.size.y) / 2.0;
            draw.push(DrawCmd::Text {
                rect: Rect::new(tx, ty, layout.size.x, layout.size.y),
                color: spec.text_color,
                handle: layout.handle,
            });
        }

        KeycapResult {
            draw,
            content_bounds: spec.rect.inset(1.0),
        }
    }
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
    pub background: Option<Color>,
    pub shadow: Option<Color>,
    pub shadow_offset: Option<f32>,
    pub shadow_height: Option<f32>,
    pub border: Option<Color>,
    pub border_width: Option<f32>,
    pub text_color: Option<Color>,
    pub text_size: Option<f32>,
    pub font: Option<FontId>,
}

impl<'a> KeycapSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }
    pub fn shadow(mut self, shadow: Color) -> Self {
        self.shadow = Some(shadow);
        self
    }
    pub fn shadow_offset(mut self, shadow_offset: f32) -> Self {
        self.shadow_offset = Some(shadow_offset);
        self
    }
    pub fn shadow_height(mut self, shadow_height: f32) -> Self {
        self.shadow_height = Some(shadow_height);
        self
    }
    pub fn border(mut self, border: Color) -> Self {
        self.border = Some(border);
        self
    }
    pub fn border_width(mut self, border_width: f32) -> Self {
        self.border_width = Some(border_width);
        self
    }
    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
    pub fn text_size(mut self, text_size: f32) -> Self {
        self.text_size = Some(text_size);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
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
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::KeycapSpec<'a> {
        raw::KeycapSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            background: self
                .background
                .expect("background not set — call .background()"),
            border: self.border.expect("border not set — call .border()"),
            border_width: self
                .border_width
                .expect("border_width not set — call .border_width()"),
            shadow: self.shadow.expect("shadow not set — call .shadow()"),
            shadow_offset: self
                .shadow_offset
                .expect("shadow_offset not set — call .shadow_offset()"),
            shadow_height: self
                .shadow_height
                .expect("shadow_height not set — call .shadow_height()"),
            text_color: self
                .text_color
                .expect("text_color not set — call .text_color()"),
            text_size: self
                .text_size
                .expect("text_size not set — call .text_size()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
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
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
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
        let mut text_sys = DummyTextSys;
        let custom_bg = Color::from_srgb_u8(240, 240, 240, 255);
        let custom_shadow = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_border = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_text = Color::from_srgb_u8(50, 50, 50, 255);
        let spec = KeycapSpec {
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            text: "K",
            background: custom_bg,
            border: custom_border,
            border_width: 1.0,
            shadow: custom_shadow,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            text_color: custom_text,
            text_size: 14.0,
            font: FontId(0),
        };
        let res = raw::keycap(spec, &mut text_sys);

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
    fn test_builder_defaults_from_theme_fills_unset_font() {
        let theme = crate::theme::Theme::framewise();
        let builder = KeycapSpecBuilder::new();
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_font() {
        let theme = crate::theme::Theme::framewise();
        let builder = KeycapSpecBuilder::new().font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.font, Some(FontId(99)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
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
        let result = super::keycap(
            &mut ctx,
            KeycapSpecBuilder::new()
                .text("X")
                .background(Color::WHITE)
                .shadow(Color::BLACK)
                .shadow_offset(1.0)
                .shadow_height(2.0)
                .border(Color::WHITE)
                .border_width(1.0)
                .text_color(Color::WHITE)
                .text_size(14.0)
                .rect(custom_rect),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
