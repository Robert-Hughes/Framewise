use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

pub struct KeycapSpec<'a> {
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

impl<'a> Default for KeycapSpec<'a> {
    fn default() -> Self {
        Self {
            rect:       Rect::new(0.0, 0.0, 28.0, 22.0),
            label:      "",
            bg:         Color::rgb(0.984, 0.976, 0.957), // paper_elev
            border:     Color::rgb(0.541, 0.514, 0.471), // muted
            text_color: Color::rgb(0.082, 0.075, 0.059), // ink
            text_size:  11.0,
        }
    }
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

pub fn keycap<T: TextSystem>(spec: KeycapSpec<'_>, ts: &mut T) -> KeycapResult {
    let mut draw = DrawCommands::new();

    // Background + border
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: spec.bg });
    draw.push(DrawCmd::StrokeRect { rect: spec.rect, color: spec.border, width: 1.0 });
    // Bottom shadow line
    let shadow_rect = Rect::new(spec.rect.x + 1.0, spec.rect.y + spec.rect.h, spec.rect.w - 1.0, 2.0);
    draw.push(DrawCmd::FillRect { rect: shadow_rect, color: Color::new(0.0, 0.0, 0.0, 0.18) });

    // Label, centered
    if !spec.label.is_empty() {
        let layout = ts.prepare(spec.label, spec.text_size);
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
