use crate::{
    draw::{DrawCmd, DrawCommands},
    input::Input,
    text::TextSystem,
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetResult},
};

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a button.
#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {
    pub background:    Color,
    pub hovered:       Color,
    pub pressed:       Color,
    pub border:        Color,
    pub border_width:  f32,
    pub text_size:     f32,
    pub text_color:    Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background:   Color::rgb(0.25, 0.25, 0.30),
            hovered:      Color::rgb(0.35, 0.35, 0.42),
            pressed:      Color::rgb(0.18, 0.18, 0.22),
            border:       Color::rgb(0.50, 0.50, 0.58),
            border_width: 1.5,
            text_size:    16.0,
            text_color:   Color::rgb(0.90, 0.90, 0.95),
        }
    }
}

// ── Spec ──────────────────────────────────────────────────────────────────────

pub struct ButtonSpec {
    pub rect:  Rect,
    pub text:  String,
    pub style: ButtonStyle,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct ButtonResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
    pub input:  InputInfo,
}

pub struct ButtonInfo {
    pub layout: LayoutInfo,
    pub input:  InputInfo,
}

impl ButtonInfo {
    /// Shorthand for `self.input.clicked`.
    pub fn clicked(&self) -> bool { self.input.clicked }
    /// Shorthand for `self.input.hovered`.
    pub fn hovered(&self) -> bool { self.input.hovered }
}

impl WidgetResult for ButtonResult {
    type Info = ButtonInfo;

    fn into_parts(self) -> (DrawCommands, ButtonInfo) {
        (
            self.draw,
            ButtonInfo {
                layout: self.layout,
                input:  self.input,
            },
        )
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a button widget.
///
/// Hit-testing is performed immediately against `input`. The returned
/// `ButtonResult` already contains the resolved interaction state.
pub fn button<T: TextSystem>(spec: ButtonSpec, input: &Input, text_sys: &mut T) -> ButtonResult {
    let hovered = spec.rect.contains(input.mouse_pos);
    let pressed  = hovered && input.mouse_down;
    let clicked  = hovered && input.mouse_clicked;

    // Choose fill colour based on interaction state.
    let fill = if pressed {
        spec.style.pressed
    } else if hovered {
        spec.style.hovered
    } else {
        spec.style.background
    };

    let mut draw = DrawCommands::new();

    // Background fill.
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: fill });

    // Border.
    if spec.style.border_width > 0.0 {
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect,
            color: spec.style.border,
            width: spec.style.border_width,
        });
    }

    // Text centered in the button.
    let text_layout = text_sys.prepare(&spec.text, spec.style.text_size);
    let text_x = spec.rect.x + (spec.rect.w - text_layout.size.x) * 0.5;
    let text_y = spec.rect.y + (spec.rect.h - text_layout.size.y) * 0.5;

    draw.push(DrawCmd::Text {
        rect:  Rect::new(text_x, text_y, text_layout.size.x, text_layout.size.y),
        color: spec.style.text_color,
        handle: text_layout.handle,
    });

    ButtonResult {
        draw,
        layout: LayoutInfo::new(spec.rect, spec.rect.inset(spec.style.border_width)),
        input:  InputInfo { hovered, pressed, clicked },
    }
}
