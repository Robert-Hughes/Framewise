use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level button widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
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
            let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
            let mut draw = DrawCommands::new();
            draw.push(DrawCmd::FillRect {
                rect: spec.rect,
                color: tint(spec.style.background),
            });
            if spec.style.border_width > 0.0 {
                draw.push(DrawCmd::StrokeRect {
                    rect: spec.rect,
                    color: tint(spec.style.border),
                    width: spec.style.border_width,
                });
            }
            let text_layout =
                text_system.prepare(&spec.text, spec.style.text_size, spec.style.font);
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
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                state,
                focused: false,
            };
        }

        let focused = focus_sys.register(state.focus_id, spec.rect, spec.clip_rect);

        let is_visible = spec
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
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
                rect: spec.rect.inset(-(spec.style.border_width + 2.0)),
                color: spec.style.focus_border,
                width: 2.0,
            });
        }

        // Background fill.
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: fill,
        });

        // Border.
        if spec.style.border_width > 0.0 {
            draw.push(DrawCmd::StrokeRect {
                rect: spec.rect,
                color: spec.style.border,
                width: spec.style.border_width,
            });
        }

        // Text centered.
        let text_layout = text_system.prepare(&spec.text, spec.style.text_size, spec.style.font);
        let text_x = spec.rect.x + (spec.rect.w - text_layout.size.x) * 0.5;
        let text_y = spec.rect.y + (spec.rect.h - text_layout.size.y) * 0.5;

        draw.push(DrawCmd::Text {
            rect: Rect::new(text_x, text_y, text_layout.size.x, text_layout.size.y),
            color: spec.style.text_color,
            handle: text_layout.handle,
        });

        ButtonResult {
            draw,
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(spec.style.border_width)),
            input: InputInfo {
                hovered,
                pressed,
                clicked,
            },
            state,
            focused,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub border: Color,
    pub border_width: f32,
    pub focus_border: Color,
    pub text_size: f32,
    pub font: FontId,
    pub text_color: Color,
}

impl Default for ButtonStyle {
    fn default() -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.06),
            pressed: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.14),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            border_width: 1.0,
            focus_border: Color::from_srgb_u8(194, 90, 44, 255),
            text_size: 13.0,
            font: FontId(1),
            text_color: Color::from_srgb_u8(21, 19, 15, 255),
        }
    }
}

impl ButtonStyle {
    pub fn primary() -> Self {
        Self {
            background: Color::from_srgb_u8(21, 19, 15, 255),
            hovered: Color::BLACK,
            pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            border_width: 1.0,
            focus_border: Color::from_srgb_u8(194, 90, 44, 255),
            text_size: 13.0,
            font: FontId(1),
            text_color: Color::from_srgb_u8(244, 241, 234, 255),
        }
    }

    pub fn accent() -> Self {
        Self {
            background: Color::from_srgb_u8(194, 90, 44, 255),
            hovered: Color::from_srgb_u8(176, 79, 35, 255),
            pressed: Color::from_srgb_u8(156, 69, 32, 255),
            border: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_border: Color::from_srgb_u8(194, 90, 44, 255),
            text_size: 13.0,
            font: FontId(1),
            text_color: Color::WHITE,
        }
    }

    pub fn ghost() -> Self {
        Self {
            border: Color::TRANSPARENT,
            border_width: 0.0,
            ..Self::default()
        }
    }
}

// ── Spec ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonSpec {
    pub rect: Rect,
    pub text: String,
    pub style: ButtonStyle,
    pub clip_rect: Option<Rect>,
    pub disabled: bool,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct ButtonState {
    /// True if the mouse was pressed while hovering this button, until the mouse is released.
    pub is_active: bool,
    /// True if the spacebar was pressed while this button was focused, until space or focus is lost.
    pub space_is_active: bool,
    /// Globally unique ID for tracking keyboard focus.
    pub focus_id: crate::focus::FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct ButtonResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: ButtonState,
    pub focused: bool,
}

pub struct ButtonInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: ButtonState,
    pub focused: bool,
}

impl ButtonInfo {
    /// Shorthand for `self.input.clicked`.
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    /// Shorthand for `self.input.hovered`.
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    /// True if the widget currently has keyboard focus.
    pub fn focused(&self) -> bool {
        self.focused
    }
}

impl ButtonResult {
    pub fn into_parts(self) -> (DrawCommands, ButtonInfo) {
        (
            self.draw,
            ButtonInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonSpecBuilder {
    pub text: String,
    pub style: Option<ButtonStyle>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
    pub disabled: bool,
}

impl ButtonSpecBuilder {
    pub fn new(text: String) -> Self {
        Self {
            text,
            style: None,
            rect: None,
            clip_rect: None,
            disabled: false,
        }
    }
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
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
        if self.style.is_none() {
            self.style = Some(theme.button_secondary_style());
        }
        self
    }
    /// Overrides the clip rectangle. High-level context functions supply this from
    /// the surrounding clip region — only needed when using the raw API directly, or
    /// to clip tighter than the context default.
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
    pub fn build(self) -> ButtonSpec {
        ButtonSpec {
            rect: self.rect.unwrap_or_default(),
            text: self.text,
            style: self.style.unwrap_or_default(),
            clip_rect: self.clip_rect,
            disabled: self.disabled,
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level button widget function using WidgetContext.
///
/// This function accepts a ButtonSpecBuilder and layout parameters, resolves geometry and styles internally,
/// and calls the low-level raw::button function.
pub fn button<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    state: ButtonState,
    layout_params: S::Params,
    builder: ButtonSpecBuilder,
) -> ButtonInfo {
    let rect = ctx.layout(layout_params);
    let clip_rect = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip_rect)
        .build();

    let result = raw::button(state, spec, ctx.input, ctx.text_system, ctx.focus_sys);

    ctx.append_cmds(result.draw.0);

    ButtonInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::focus::FocusId;
    use crate::test_utils::DummyTextSys;
    use crate::text::TextHandle;
    use crate::types::Vec2;
    fn btn_spec(y: f32) -> ButtonSpec {
        ButtonSpec {
            rect: Rect::new(0.0, y, 100.0, 30.0),
            text: "B".into(),
            style: Default::default(),
            clip_rect: None,
            disabled: false,
        }
    }

    /// Run one frame with two buttons and return their states.
    fn two_btn_frame(
        focus_sys: &mut crate::focus::FocusSystem,
        s1: ButtonState,
        s2: ButtonState,
        input: &Input,
    ) -> (ButtonState, ButtonState) {
        let mut ts = DummyTextSys;
        focus_sys.begin_frame();
        let r1 = raw::button(s1, btn_spec(0.0), input, &mut ts, focus_sys)
            .into_parts()
            .1;
        let r2 = raw::button(s2, btn_spec(40.0), input, &mut ts, focus_sys)
            .into_parts()
            .1;
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
        assert_eq!(
            focus_sys.current_focus(),
            Some(s2.focus_id),
            "Tab should move focus to btn2"
        );
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
        assert_eq!(
            focus_sys.current_focus(),
            Some(s2.focus_id),
            "Right arrow should move focus to btn2"
        );
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
        assert_eq!(
            focus_sys.current_focus(),
            Some(s2.focus_id),
            "Down arrow should move focus to btn2"
        );
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
        assert_eq!(
            focus_sys.current_focus(),
            Some(s1.focus_id),
            "Shift+Tab should move focus back to btn1"
        );
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
            disabled: false,
        };
        let btn2_spec = || ButtonSpec {
            rect: Rect::new(0.0, 100.0, 100.0, 50.0),
            text: "Btn2".to_string(),
            style: ButtonStyle::default(),
            clip_rect: None,
            disabled: false,
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
        let res1 = raw::button(
            state1,
            btn1_spec(),
            &input,
            &mut text_system,
            &mut focus_sys,
        )
        .into_parts()
        .1;
        state1 = res1.state;
        assert!(res1.input.pressed);

        // Frame 2: Mouse dragged over Btn2
        input.mouse_pressed = false;
        input.mouse_pos = Vec2::new(50.0, 125.0);
        let res1 = raw::button(
            state1,
            btn1_spec(),
            &input,
            &mut text_system,
            &mut focus_sys,
        )
        .into_parts()
        .1;
        state1 = res1.state;
        let res2 = raw::button(
            state2,
            btn2_spec(),
            &input,
            &mut text_system,
            &mut focus_sys,
        )
        .into_parts()
        .1;
        state2 = res2.state;

        assert!(
            !res2.input.pressed,
            "Btn2 should not be pressed when mouse is dragged over it"
        );
        assert!(
            !res2.input.hovered,
            "Btn2 should not be hovered while dragging another widget"
        );

        // Frame 3: Mouse released over Btn2
        input.mouse_down = false;
        input.mouse_clicked = true;
        let res1 = raw::button(
            state1,
            btn1_spec(),
            &input,
            &mut text_system,
            &mut focus_sys,
        )
        .into_parts()
        .1;

        let res2 = raw::button(
            state2,
            btn2_spec(),
            &input,
            &mut text_system,
            &mut focus_sys,
        )
        .into_parts()
        .1;

        assert!(
            !res2.input.clicked,
            "Btn2 should not be clicked if mouse down was not on Btn2"
        );
        assert!(
            !res1.input.clicked,
            "Btn1 should not be clicked since mouse was released outside"
        );
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
            disabled: false,
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
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(res.input.pressed);

        // Frame 2: Mouse released
        input.mouse_down = false;
        input.mouse_pressed = false;
        input.mouse_clicked = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;

        assert!(res.input.clicked, "Button should register as clicked");
    }

    #[test]
    fn test_button_click_takes_focus() {
        let mut text_system = DummyTextSys;
        let state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
            clip_rect: None,
            disabled: false,
        };

        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        focus_sys.begin_frame();
        let res = raw::button(state, spec, &input, &mut text_system, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking button must request focus"
        );
    }

    #[test]
    fn test_enter_clicks_raw_button() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        let spec = || ButtonSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Btn".to_string(),
            style: ButtonStyle::default(),
            clip_rect: None,
            disabled: false,
        };

        // Frame 1: Register and take focus explicitly
        let mut input = Input::default();
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Press Enter
        input.key_pressed_enter = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
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
            disabled: false,
        };

        // Frame 1: Mouse outside
        let mut input = Input {
            mouse_pos: Vec2::new(150.0, 150.0),
            ..Default::default()
        };
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(!res.input.hovered);
        assert!(!res.input.pressed);

        // Frame 2: Mouse inside, not down
        input.mouse_pos = Vec2::new(50.0, 25.0);
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(res.input.hovered, "Should be hovered");
        assert!(!res.input.pressed, "Should not be pressed");

        // Frame 3: Mouse down inside
        input.mouse_down = true;
        input.mouse_pressed = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(res.input.hovered, "Should be hovered while pressed down");
        assert!(res.input.pressed, "Should be pressed");

        // Frame 4: Drag outside
        input.mouse_pos = Vec2::new(150.0, 150.0);
        input.mouse_pressed = false;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        assert!(!res.input.hovered, "Should lose hover when dragged out");
        assert!(
            !res.input.pressed,
            "Should lose pressed state when dragged out"
        );
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
            disabled: false,
        };

        // Frame 1: Focus
        let mut input = Input::default();
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(
            res.input.pressed,
            "Button should be visually pressed while space is down"
        );
        assert!(!res.input.clicked, "Button should not be clicked yet");

        // Frame 3: Space held
        input.key_pressed_space = false;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(res.input.pressed, "Button should remain pressed");
        assert!(!res.input.clicked, "Button should not be clicked yet");

        // Frame 4: Space released
        input.key_down_space = false;
        input.key_released_space = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
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
            disabled: false,
        };

        // Frame 1: Focus
        let mut input = Input::default();
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(res.input.pressed);

        // Frame 3: Lose focus!
        input.key_pressed_space = false;
        focus_sys.take_focus(FocusId::new()); // Give focus to something else
        focus_sys.end_frame();

        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        state = res.state;
        assert!(
            !res.input.pressed,
            "Should lose pressed state when focus lost"
        );

        // Frame 4: Release space
        input.key_down_space = false;
        input.key_released_space = true;
        let res = raw::button(state, spec(), &input, &mut text_system, &mut focus_sys)
            .into_parts()
            .1;
        assert!(!res.input.clicked, "Should not click because it lost focus");
    }

    // ── Visual Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_button_visual_normal() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();
        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::primary(),
            clip_rect: None,
            disabled: false,
        };

        focus_sys.begin_frame();
        let res = raw::button(state, spec, &input, &mut text_sys, &mut focus_sys);
        focus_sys.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(48.0, 17.0, 24.0, 16.0),
                    color: text_color,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_button_visual_hovered() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0); // Inside bounds
        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::primary(),
            clip_rect: None,
            disabled: false,
        };

        focus_sys.begin_frame();
        let res = raw::button(state, spec, &input, &mut text_sys, &mut focus_sys);
        focus_sys.end_frame();

        let ButtonStyle {
            hovered,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: hovered,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(48.0, 17.0, 24.0, 16.0),
                    color: text_color,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_button_visual_pressed() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);
        input.mouse_down = true;
        input.mouse_pressed = true;

        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::primary(),
            clip_rect: None,
            disabled: false,
        };

        focus_sys.begin_frame();
        let res = raw::button(state, spec, &input, &mut text_sys, &mut focus_sys);
        focus_sys.end_frame();

        let ButtonStyle {
            pressed,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: pressed,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(48.0, 17.0, 24.0, 16.0),
                    color: text_color,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_button_visual_focused() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::primary(),
            clip_rect: None,
            disabled: false,
        };

        focus_sys.take_focus(state.focus_id);

        focus_sys.begin_frame();
        let res = raw::button(
            state,
            spec,
            &Input::default(),
            &mut text_sys,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            focus_border,
            ..
        } = ButtonStyle::primary();

        let expected_focus_rect = Rect::new(10.0, 10.0, 100.0, 30.0).inset(-(border_width + 2.0));

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: expected_focus_rect,
                    color: focus_border,
                    width: 2.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: border,
                    width: border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(48.0, 17.0, 24.0, 16.0),
                    color: text_color,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_button_visual_disabled() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".to_string(),
            style: ButtonStyle::primary(),
            clip_rect: None,
            disabled: true,
        };

        focus_sys.begin_frame();
        let res = raw::button(
            state,
            spec,
            &Input::default(),
            &mut text_sys,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        let alpha = 0.32_f32;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let expected_bg = tint(ButtonStyle::primary().background);
        let expected_border = tint(ButtonStyle::primary().border);
        let expected_text = tint(ButtonStyle::primary().text_color);
        let border_width = ButtonStyle::primary().border_width;

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: expected_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    color: expected_border,
                    width: border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(48.0, 17.0, 24.0, 16.0),
                    color: expected_text,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_regression_custom_style_no_theme_lookup() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();

        let custom_style = ButtonStyle {
            background: Color::from_srgb_u8(100, 150, 200, 255),
            hovered: Color::from_srgb_u8(110, 160, 210, 255),
            pressed: Color::from_srgb_u8(120, 170, 220, 255),
            border: Color::from_srgb_u8(220, 230, 240, 255),
            border_width: 4.5,
            focus_border: Color::from_srgb_u8(255, 0, 0, 255),
            text_size: 19.5,
            font: FontId(0),
            text_color: Color::from_srgb_u8(50, 60, 70, 255),
        };

        let spec = ButtonSpec {
            rect: Rect::new(5.0, 15.0, 120.0, 45.0),
            text: "Explicit Spec".to_string(),
            style: custom_style,
            clip_rect: None,
            disabled: false,
        };

        focus_sys.begin_frame();
        let res = raw::button(state, spec, &input, &mut text_sys, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                    color: custom_style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                    color: custom_style.border,
                    width: custom_style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(13.0, 29.5, 104.0, 16.0),
                    color: custom_style.text_color,
                    handle: TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = ButtonSpecBuilder::new("test".to_string());
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        let expected = theme.button_secondary_style();
        assert_eq!(builder.style.unwrap().font, expected.font);
        assert_eq!(builder.style.unwrap().text_size, expected.text_size);
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let custom_style = ButtonStyle { text_size: 99.0, ..ButtonStyle::default() };
        let builder = ButtonSpecBuilder::new("test".to_string()).style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
    }
}
