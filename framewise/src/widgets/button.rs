use crate::{
    draw::{DrawCmd, DrawCommands},
    input::Input,
    text::TextSystem,
    theme::Theme,
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
    /// Secondary button: transparent background, ink border.
    fn default() -> Self {
        let t = Theme::framewise();
        Self {
            background:   Color::TRANSPARENT,
            hovered:      t.hover,
            pressed:      t.press,
            border:       t.ink,
            border_width: t.border,
            focus_border: t.rust,
            text_size:    t.text_md,
            text_color:   t.ink,
        }
    }
}

impl ButtonStyle {
    /// Primary: ink fill, paper text.
    pub fn primary() -> Self {
        let t = Theme::framewise();
        Self {
            background:   t.ink,
            hovered:      Color::rgb(0.0, 0.0, 0.0),
            pressed:      Color::from_u8(42, 37, 32, 255),
            border:       t.ink,
            border_width: t.border,
            focus_border: t.rust,
            text_size:    t.text_md,
            text_color:   t.paper,
        }
    }

    /// Accent: rust fill, white text.
    pub fn accent() -> Self {
        let t = Theme::framewise();
        Self {
            background:   t.rust,
            hovered:      Color::from_u8(176, 79, 35, 255),
            pressed:      Color::from_u8(156, 69, 32, 255),
            border:       t.rust,
            border_width: t.border,
            focus_border: t.rust,
            text_size:    t.text_md,
            text_color:   Color::WHITE,
        }
    }

    /// Ghost: no border, transparent background.
    pub fn ghost() -> Self {
        let t = Theme::framewise();
        Self {
            background:   Color::TRANSPARENT,
            hovered:      t.hover,
            pressed:      t.press,
            border:       Color::TRANSPARENT,
            border_width: 0.0,
            focus_border: t.rust,
            text_size:    t.text_md,
            text_color:   t.ink,
        }
    }
}

// ── Spec ──────────────────────────────────────────────────────────────────────

pub struct ButtonSpec {
    pub rect:      Rect,
    pub text:      String,
    pub style:     ButtonStyle,
    pub clip_rect: Option<Rect>,
    pub disabled:  bool,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
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
    // Disabled: register for layout but skip all interaction.
    if spec.disabled {
        let alpha = 0.32_f32;
        let tint = |c: Color| Color::new(c.r, c.g, c.b, c.a * alpha);
        let mut draw = DrawCommands::new();
        draw.push(DrawCmd::FillRect { rect: spec.rect, color: tint(spec.style.background) });
        if spec.style.border_width > 0.0 {
            draw.push(DrawCmd::StrokeRect { rect: spec.rect, color: tint(spec.style.border), width: spec.style.border_width });
        }
        let text_layout = text_system.prepare(&spec.text, spec.style.text_size);
        let tx = spec.rect.x + (spec.rect.w - text_layout.size.x) * 0.5;
        let ty = spec.rect.y + (spec.rect.h - text_layout.size.y) * 0.5;
        draw.push(DrawCmd::Text {
            rect: Rect::new(tx, ty, text_layout.size.x, text_layout.size.y),
            color: tint(spec.style.text_color),
            handle: text_layout.handle,
        });
        return ButtonResult {
            draw,
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(spec.style.border_width)),
            input:  InputInfo { hovered: false, pressed: false, clicked: false },
            state,
            focused: false,
        };
    }

    let focused = focus_sys.register(state.focus_id, spec.rect, spec.clip_rect);

    let is_visible = spec.clip_rect.map_or(true, |clip| clip.contains(input.mouse_pos));
    let contains = spec.rect.contains(input.mouse_pos) && is_visible;

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

    focus_sys.handle_traversal(focused, input, crate::focus::FocusTraversalKeys::all());

    // Choose fill colour based on interaction state.
    let fill = if pressed {
        spec.style.pressed
    } else if hovered {
        spec.style.hovered
    } else {
        spec.style.background
    };

    let mut draw = DrawCommands::new();

    // Focus ring drawn first (outset — sits outside the button bounds).
    if focused {
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect.inset(-(spec.style.border_width + 2.0)),
            color: spec.style.focus_border,
            width: 2.0,
        });
    }

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

    // Text centered.
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
    fn btn_spec(y: f32) -> ButtonSpec {
        ButtonSpec { rect: Rect::new(0.0, y, 100.0, 30.0), text: "B".into(), style: Default::default(), clip_rect: None, disabled: false }
    }

    /// Run one frame with two buttons and return their states.
    fn two_btn_frame(
        focus_sys: &mut crate::focus::FocusSystem,
        s1: ButtonState, s2: ButtonState,
        input: &Input,
    ) -> (ButtonState, ButtonState) {
        let mut ts = DummyTextSys;
        focus_sys.begin_frame();
        let r1 = button(s1, btn_spec(0.0),  input, &mut ts, focus_sys).into_parts().1;
        let r2 = button(s2, btn_spec(40.0), input, &mut ts, focus_sys).into_parts().1;
        focus_sys.end_frame();
        (r1.state, r2.state)
    }

    #[test]
    fn test_button_tab_moves_focus_next() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let s1 = ButtonState::default();
        let s2 = ButtonState::default();
        focus_sys.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_tab = true;
        let (s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &input);
        // Focus shift resolves at end_frame; confirm in next frame
        let (_s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &Input::default());
        assert_eq!(focus_sys.current_focus(), Some(s2.focus_id), "Tab should move focus to btn2");
    }

    #[test]
    fn test_button_right_arrow_moves_focus_next() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let s1 = ButtonState::default();
        let s2 = ButtonState::default();
        focus_sys.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_right = true;
        let (s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &input);
        let (_s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &Input::default());
        assert_eq!(focus_sys.current_focus(), Some(s2.focus_id), "Right arrow should move focus to btn2");
    }

    #[test]
    fn test_button_down_arrow_moves_focus_next() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let s1 = ButtonState::default();
        let s2 = ButtonState::default();
        focus_sys.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_down = true;
        let (s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &input);
        let (_s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &Input::default());
        assert_eq!(focus_sys.current_focus(), Some(s2.focus_id), "Down arrow should move focus to btn2");
    }

    #[test]
    fn test_button_shift_tab_moves_focus_prev() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let s1 = ButtonState::default();
        let s2 = ButtonState::default();
        // Start with focus on s2
        focus_sys.take_focus(s2.focus_id);

        let mut input = Input::default();
        input.key_pressed_tab = true;
        input.modifier_shift = true;
        let (s1, s2) = two_btn_frame(&mut focus_sys, s1, s2, &input);
        let (s1, _s2) = two_btn_frame(&mut focus_sys, s1, s2, &Input::default());
        assert_eq!(focus_sys.current_focus(), Some(s1.focus_id), "Shift+Tab should move focus back to btn1");
    }

    #[test]
    fn test_drag_off_and_release_does_not_click_other_button() {
        let mut text_system = DummyTextSys;
        
        let mut state1 = ButtonState::default();
        let mut state2 = ButtonState::default();

        let btn1_spec = || ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Click Me".to_string(),
            style: ButtonStyle::default(),
            clip_rect: None,
            disabled:  false,
        };
        let btn2_spec = || ButtonSpec {
            rect: Rect::new(0.0, 100.0, 100.0, 50.0),
            text: "Btn2".to_string(),
            style: ButtonStyle::default(),
            clip_rect: None,
            disabled:  false,
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
            clip_rect: None,
            disabled:  false,
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
            clip_rect: None,
            disabled:  false,
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
            clip_rect: None,
            disabled:  false,
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
            clip_rect: None,
            disabled:  false,
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
            clip_rect: None,
            disabled:  false,
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
