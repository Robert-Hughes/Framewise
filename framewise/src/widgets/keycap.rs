use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    text::FontId,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level keycap widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn keycap<'a, T: crate::text::TextSystem>(
        spec: KeycapSpec<'a>,
        text_system: &mut T,
    ) -> KeycapResult {
        let mut draw = DrawCommands::new();

        // Background + border
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.bg,
        });
        draw.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: spec.border,
            width: 1.0,
        });
        // Bottom shadow line
        let shadow_rect = Rect::new(
            spec.rect.x + 1.0,
            spec.rect.y + spec.rect.h,
            spec.rect.w - 1.0,
            2.0,
        );
        draw.push(DrawCmd::FillRect {
            rect: shadow_rect,
            color: Color::linear_rgba(0.0, 0.0, 0.0, 0.18),
        });

        // Label, centered
        if !spec.label.is_empty() {
            let layout = text_system.prepare(spec.label, spec.text_size, spec.font);
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
            layout: LayoutInfo::tight(spec.rect),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapSpec<'a> {
    pub rect: Rect,
    pub label: &'a str,
    /// Background fill (default: paper_elev).
    pub bg: Color,
    /// Border color.
    pub border: Color,
    /// Label text color.
    pub text_color: Color,
    pub text_size: f32,
    pub font: FontId,
}

pub struct KeycapResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct KeycapInfo {
    pub layout: LayoutInfo,
}

impl KeycapResult {
    pub fn into_parts(self) -> (DrawCommands, KeycapInfo) {
        (
            self.draw,
            KeycapInfo {
                layout: self.layout,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level keycap widget function using WidgetContext.
///
/// This function accepts a KeycapSpec and calls the low-level raw::keycap function.
pub fn keycap<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: KeycapSpecBuilder<'a>,
) -> KeycapInfo {
    let rect = ctx.layout(layout_params);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::keycap(spec, ctx.text_system);
    ctx.append_cmds(result.draw.0);
    KeycapInfo {
        layout: result.layout,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapSpecBuilder<'a> {
    pub label: Option<&'a str>,
    pub bg: Option<Color>,
    pub border: Option<Color>,
    pub text_color: Option<Color>,
    pub text_size: Option<f32>,
    pub font: Option<FontId>,
    pub rect: Option<Rect>,
}

impl<'a> KeycapSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            bg: None,
            border: None,
            text_color: None,
            text_size: None,
            font: None,
            rect: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn bg(mut self, bg: Color) -> Self {
        self.bg = Some(bg);
        self
    }
    pub fn border(mut self, border: Color) -> Self {
        self.border = Some(border);
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
}

impl<'a> KeycapSpecBuilder<'a> {
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

    pub fn build(self) -> KeycapSpec<'a> {
        KeycapSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            label: self.label.expect("label not set — call .label()"),
            bg: self.bg.expect("bg not set — call .bg()"),
            border: self.border.expect("border not set — call .border()"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_keycap_visual() {
        let mut text_sys = DummyTextSys;
        let custom_bg = Color::from_srgb_u8(240, 240, 240, 255);
        let custom_border = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_text = Color::from_srgb_u8(50, 50, 50, 255);
        let spec = KeycapSpec {
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            label: "K",
            bg: custom_bg,
            border: custom_border,
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
                    color: Color::linear_rgba(0.0, 0.0, 0.0, 0.18),
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
}
