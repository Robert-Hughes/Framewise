use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

pub struct KeycapSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect:  Rect,
    pub label: &'a str,
    /// Background fill (default: paper_elev).
    pub bg:    Color,
    /// Border color.
    pub border: Color,
    /// Label text color.
    pub text_color: Color,
    pub text_size: f32,
}



pub struct KeycapResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct KeycapInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for KeycapResult {
    type Info = KeycapInfo;
    fn into_parts(self) -> (DrawCommands, KeycapInfo) {
        (self.draw, KeycapInfo { layout: self.layout })
    }
}

pub fn keycap<'a, T: crate::text::TextSystem>(spec: KeycapSpec<'a, T>) -> KeycapResult {
    let mut draw = DrawCommands::new();

    // Background + border
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: spec.bg });
    draw.push(DrawCmd::StrokeRect { rect: spec.rect, color: spec.border, width: 1.0 });
    // Bottom shadow line
    let shadow_rect = Rect::new(spec.rect.x + 1.0, spec.rect.y + spec.rect.h, spec.rect.w - 1.0, 2.0);
    draw.push(DrawCmd::FillRect { rect: shadow_rect, color: Color::linear_rgba(0.0, 0.0, 0.0, 0.18) });

    // Label, centered
    if !spec.label.is_empty() {
        let layout = spec.ts.prepare(spec.label, spec.text_size);
        let tx = spec.rect.x + (spec.rect.w - layout.size.x) / 2.0;
        let ty = spec.rect.y + (spec.rect.h - layout.size.y) / 2.0;
        draw.push(DrawCmd::Text {
            rect:  Rect::new(tx, ty, layout.size.x, layout.size.y),
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
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for KeycapSpecBuilder<'a, T> {
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
        }
    }
}
