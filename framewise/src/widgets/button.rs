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
    pub focus_border:  Color,
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
            focus_border: Color::rgb(0.60, 0.80, 1.00),
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

#[derive(Debug, Clone, Copy)]
pub struct ButtonState {
    /// True if the mouse was pressed while hovering this button, until the mouse is released.
    pub is_active: bool,
    /// True if the spacebar was pressed while this button was focused, until space or focus is lost.
    pub space_is_active: bool,
    /// Globally unique ID for tracking keyboard focus.
    pub focus_id: crate::focus::FocusId,
}

impl Default for ButtonState {
    fn default() -> Self {
        Self {
            is_active: false,
            space_is_active: false,
            focus_id: crate::focus::FocusId::new(),
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct ButtonResult {
    pub draw:    DrawCommands,
    pub layout:  LayoutInfo,
    pub input:   InputInfo,
    pub state:   ButtonState,
    pub focused: bool,
}

pub struct ButtonInfo {
    pub layout:  LayoutInfo,
    pub input:   InputInfo,
    pub state:   ButtonState,
    pub focused: bool,
}

impl ButtonInfo {
    /// Shorthand for `self.input.clicked`.
    pub fn clicked(&self) -> bool { self.input.clicked }
    /// Shorthand for `self.input.hovered`.
    pub fn hovered(&self) -> bool { self.input.hovered }
    /// True if the widget currently has keyboard focus.
    pub fn focused(&self) -> bool { self.focused }
}

impl WidgetResult for ButtonResult {
    type Info = ButtonInfo;

    fn into_parts(self) -> (DrawCommands, ButtonInfo) {
        (
            self.draw,
            ButtonInfo {
                layout:  self.layout,
                input:   self.input,
                state:   self.state,
                focused: self.focused,
            },
        )
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a button widget.
///
/// Hit-testing is performed immediately against `input`. The returned
/// `ButtonResult` already contains the resolved interaction state.
pub fn button<T: crate::text::TextSystem>(
    mut state: ButtonState, 
    spec: ButtonSpec, 
    input: &Input, 
    text_system: &mut T,
    focus_sys: &mut crate::focus::FocusSystem,
) -> ButtonResult {
    let focused = focus_sys.register(state.focus_id);

    let contains = spec.rect.contains(input.mouse_pos);
    
    if contains && input.mouse_pressed {
        state.is_active = true;
    }

    let hovered = contains && (!input.mouse_down || state.is_active);
    let mut clicked = state.is_active && hovered && input.mouse_clicked;

    // Trigger click on Enter (immediate) or Space release (if it was active)
    if focused && input.key_pressed_enter {
        clicked = true;
    }
    if state.space_is_active && input.key_released_space {
        clicked = true;
    }

    // Update space activation state
    if !focused || !input.key_down_space {
        state.space_is_active = false;
    }
    if focused && input.key_pressed_space {
        state.space_is_active = true;
    }

    // Update mouse activation state
    if !input.mouse_down {
        state.is_active = false;
    }

    let pressed = (state.is_active && hovered && input.mouse_down) || state.space_is_active;

    if pressed {
        focus_sys.take_focus(state.focus_id);
    }

    // Choose fill colour based on interaction state.
    let fill = if pressed {
        spec.style.pressed
    } else if hovered {
        spec.style.hovered
    } else {
        spec.style.background
    };

    let mut draw = DrawCommands::new();

    // Background fill (outer frame).
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: fill });

    // Calculate press offset
    let offset = if pressed { 2.0 } else { 0.0 };

    // Inner frame
    // We add a bit of padding inside the border for the inner frame.
    let inner_padding = spec.style.border_width + 2.0;
    let inner_rect = spec.rect.inset(inner_padding);
    let inner_shifted = Rect::new(
        inner_rect.x + offset,
        inner_rect.y + offset,
        inner_rect.w,
        inner_rect.h,
    );

    // Lighten the inner frame slightly to give a 3D bevel effect
    let inner_color = Color::rgb(
        (fill.r + 0.04).min(1.0),
        (fill.g + 0.04).min(1.0),
        (fill.b + 0.05).min(1.0),
    );
    draw.push(DrawCmd::FillRect { rect: inner_shifted, color: inner_color });

    // Border.
    if focused {
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect.inset(-2.0),
            color: spec.style.focus_border,
            width: 2.0,
        });
    }

    if spec.style.border_width > 0.0 {
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect,
            color: spec.style.border,
            width: spec.style.border_width,
        });
    }

    // Text centered in the button, shifted by the press offset.
    let text_layout = text_system.prepare(&spec.text, spec.style.text_size);
    let text_x = spec.rect.x + (spec.rect.w - text_layout.size.x) * 0.5 + offset;
    let text_y = spec.rect.y + (spec.rect.h - text_layout.size.y) * 0.5 + offset;

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
        focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextSystem, TextLayout, TextHandle};
    use crate::types::Vec2;
    use crate::focus::FocusId;

    struct DummyTextSys;
    impl TextSystem for DummyTextSys {
        fn prepare(&mut self, _text: &str, _size: f32) -> TextLayout {
            TextLayout {
                handle: TextHandle(0),
                size: Vec2::new(0.0, 0.0),
            }
        }
        fn measure_byte_x(&self, _handle: TextHandle, _byte_index: usize) -> f32 { 0.0 }
        fn hit_test_x(&self, _handle: TextHandle, _x_offset: f32) -> usize { 0 }
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
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut input = Input {
            mouse_pos: Vec2::new(50.0, 25.0),
            mouse_down: true,
            mouse_pressed: true,
            mouse_clicked: false,
            ..Default::default()
        };
        let res1 = button(state1, btn1_spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state1 = res1.state;
        assert!(res1.input.pressed);

        // Frame 2: Mouse dragged over Btn2
        input.mouse_pressed = false;
        input.mouse_pos = Vec2::new(50.0, 125.0);
        let res1 = button(state1, btn1_spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state1 = res1.state;
        let res2 = button(state2, btn2_spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state2 = res2.state;

        assert!(!res2.input.pressed, "Btn2 should not be pressed when mouse is dragged over it");
        assert!(!res2.input.hovered, "Btn2 should not be hovered while dragging another widget");

        // Frame 3: Mouse released over Btn2
        input.mouse_down = false;
        input.mouse_clicked = true;
        let res1 = button(state1, btn1_spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state1 = res1.state;
        let res2 = button(state2, btn2_spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state2 = res2.state;

        assert!(!res2.input.clicked, "Btn2 should not be clicked if mouse down was not on Btn2");
        assert!(!res1.input.clicked, "Btn1 should not be clicked since mouse was released outside");
    }

    #[test]
    fn test_click_triggers_clicked_state() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        
        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Mouse pressed
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut input = Input {
            mouse_pos: Vec2::new(50.0, 25.0),
            mouse_down: true,
            mouse_pressed: true,
            mouse_clicked: false,
            ..Default::default()
        };
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.pressed);

        // Frame 2: Mouse released
        input.mouse_down = false;
        input.mouse_pressed = false;
        input.mouse_clicked = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        
        assert!(res.input.clicked, "Button should register as clicked");
    }

    #[test]
    fn test_enter_clicks_button() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        
        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Register and take focus explicitly
        let mut input = Input::default();
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Press Enter
        input.key_pressed_enter = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        assert!(res.input.clicked, "Button should be clicked by Enter key");
    }

    #[test]
    fn test_hover_and_press_state() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        
        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Mouse outside
        let mut input = Input {
            mouse_pos: Vec2::new(150.0, 150.0),
            ..Default::default()
        };
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(!res.input.hovered);
        assert!(!res.input.pressed);

        // Frame 2: Mouse inside, not down
        input.mouse_pos = Vec2::new(50.0, 25.0);
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.hovered, "Should be hovered");
        assert!(!res.input.pressed, "Should not be pressed");

        // Frame 3: Mouse down inside
        input.mouse_down = true;
        input.mouse_pressed = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.hovered, "Should be hovered while pressed down");
        assert!(res.input.pressed, "Should be pressed");

        // Frame 4: Drag outside
        input.mouse_pos = Vec2::new(150.0, 150.0);
        input.mouse_pressed = false;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        assert!(!res.input.hovered, "Should lose hover when dragged out");
        assert!(!res.input.pressed, "Should lose pressed state when dragged out");
    }

    #[test]
    fn test_spacebar_click() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        
        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Focus
        let mut input = Input::default();
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.pressed, "Button should be visually pressed while space is down");
        assert!(!res.input.clicked, "Button should not be clicked yet");
        
        // Frame 3: Space held
        input.key_pressed_space = false;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.pressed, "Button should remain pressed");
        assert!(!res.input.clicked, "Button should not be clicked yet");

        // Frame 4: Space released
        input.key_down_space = false;
        input.key_released_space = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        assert!(!res.input.pressed, "Button should not be pressed");
        assert!(res.input.clicked, "Button should be clicked on release");
    }

    #[test]
    fn test_spacebar_loses_focus_does_not_click() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        
        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
        };

        // Frame 1: Focus
        let mut input = Input::default();
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(res.input.pressed);

        // Frame 3: Lose focus!
        input.key_pressed_space = false;
        focus_sys.take_focus(FocusId::new()); // Give focus to something else
        focus_sys.end_frame();
        
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        state = res.state;
        assert!(!res.input.pressed, "Should lose pressed state when focus lost");

        // Frame 4: Release space
        input.key_down_space = false;
        input.key_released_space = true;
        let res = button(state, spec(), &input, &mut text_system, &mut focus_sys).into_parts().1;
        assert!(!res.input.clicked, "Should not click because it lost focus");
    }
}
