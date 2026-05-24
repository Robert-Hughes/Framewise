use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

pub struct KeycapSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
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

impl WidgetResult for KeycapResult {
    type Info = KeycapInfo;
    fn into_parts(self) -> (DrawCommands, KeycapInfo) {
        (
            self.draw,
            KeycapInfo {
                layout: self.layout,
            },
        )
    }
}

pub fn keycap<'a, T: crate::text::TextSystem>(spec: KeycapSpec<'a, T>) -> KeycapResult {
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
        let layout = spec.ts.prepare(spec.label, spec.text_size, spec.font);
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

pub struct KeycapSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub bg: Option<Color>,
    pub border: Option<Color>,
    pub text_color: Option<Color>,
    pub text_size: Option<f32>,
    pub font: Option<FontId>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> KeycapSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            bg: None,
            border: None,
            text_color: None,
            text_size: None,
            font: None,
            rect: None,
            ts: None,
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

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for KeycapSpecBuilder<'a, T>
{
    type Spec = KeycapSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        KeycapSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            bg: self.bg.unwrap(),
            border: self.border.unwrap(),
            text_color: self.text_color.unwrap(),
            text_size: self.text_size.unwrap(),
            font: self.font.unwrap_or(FontId::MONO),
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
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            label: "K",
            bg: custom_bg,
            border: custom_border,
            text_color: custom_text,
            text_size: 14.0,
            font: FontId::MONO,
        };
        let res = keycap(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4);

        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == custom_bg));
        assert!(
            matches!(&cmds[1], DrawCmd::StrokeRect { color, width, .. } if *color == custom_border && *width == 1.0)
        );
        assert!(
            matches!(&cmds[2], DrawCmd::FillRect { color, .. } if *color == Color::linear_rgba(0.0, 0.0, 0.0, 0.18))
        ); // shadow
        assert!(matches!(&cmds[3], DrawCmd::Text { color, .. } if *color == custom_text));
    }
}

