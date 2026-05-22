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

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
pub struct ButtonState {
    pub is_active: bool,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct ButtonResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
    pub input:  InputInfo,
    pub state:  ButtonState,
}

pub struct ButtonInfo {
    pub layout: LayoutInfo,
    pub input:  InputInfo,
    pub state:  ButtonState,
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
                state:  self.state,
            },
        )
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a button widget.
///
/// Hit-testing is performed immediately against `input`. The returned
/// `ButtonResult` already contains the resolved interaction state.
pub fn button<T: TextSystem>(mut state: ButtonState, spec: ButtonSpec, input: &Input, text_system: &mut T) -> ButtonResult {
    let contains = spec.rect.contains(input.mouse_pos);
    
    if contains && input.mouse_pressed {
        state.is_active = true;
    }
    if !input.mouse_down {
        state.is_active = false;
    }

    let hovered = contains && (!input.mouse_down || state.is_active);
    let pressed  = state.is_active && hovered;
    let clicked  = state.is_active && hovered && input.mouse_clicked;

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
    let text_layout = text_system.prepare(&spec.text, spec.style.text_size);
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
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextSystem, TextLayout, TextHandle};
    use crate::types::Vec2;

    struct DummyTextSys;
    impl TextSystem for DummyTextSys {
        fn prepare(&mut self, _text: &str, _size: f32) -> TextLayout {
            TextLayout { size: Vec2::new(0.0, 0.0), handle: TextHandle(0) }
        }
    }

    #[test]
    fn test_drag_off_and_release_does_not_click_other_button() {
        let mut text_system = DummyTextSys;
        
        let mut state1 = ButtonState::default();
        let mut state2 = ButtonState::default();

        let btn1_spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn1".to_string(),
            style: ButtonStyle::default(),
        };
        let btn2_spec = || ButtonSpec {
            rect: Rect::new(0.0, 100.0, 100.0, 50.0),
            text: "Btn2".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Mouse down on Btn1
        let mut input = Input {
            mouse_pos: Vec2::new(50.0, 25.0),
            mouse_down: true,
            mouse_pressed: true,
            mouse_clicked: false,
        };
        let res1 = button(state1, btn1_spec(), &input, &mut text_system).into_parts().1;
        state1 = res1.state;
        assert!(res1.input.pressed);

        // Frame 2: Mouse dragged over Btn2
        input.mouse_pressed = false;
        input.mouse_pos = Vec2::new(50.0, 125.0);
        let res1 = button(state1, btn1_spec(), &input, &mut text_system).into_parts().1;
        state1 = res1.state;
        let res2 = button(state2, btn2_spec(), &input, &mut text_system).into_parts().1;
        state2 = res2.state;

        assert!(!res2.input.pressed, "Btn2 should not be pressed when mouse is dragged over it");
        assert!(!res2.input.hovered, "Btn2 should not be hovered while dragging another widget");

        // Frame 3: Mouse released over Btn2
        input.mouse_down = false;
        input.mouse_clicked = true;
        let res1 = button(state1, btn1_spec(), &input, &mut text_system).into_parts().1;
        state1 = res1.state;
        let res2 = button(state2, btn2_spec(), &input, &mut text_system).into_parts().1;
        state2 = res2.state;

        assert!(!res2.input.clicked, "Btn2 should not be clicked if mouse down was not on Btn2");
        assert!(!res1.input.clicked, "Btn1 should not be clicked since mouse was released outside");
    }
}
