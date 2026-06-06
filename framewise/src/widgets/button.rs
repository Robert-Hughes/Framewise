use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    types::{ClipRect, Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextSystem,
};

pub mod raw {
    use crate::TextSystem;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ButtonStyle,
        pub clip_rect: ClipRect,
        pub disabled: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Measure a button's intrinsic size from its spec.
    ///
    /// The preferred width is the label width plus horizontal padding; the
    /// preferred height is the larger of the standard control height and the
    /// padded label height.
    ///
    /// **Must not read `spec.rect`** — this runs before the rect is known, so
    /// callers pass [`Rect::PLACEHOLDER`] (NaN). Intrinsic size depends only on
    /// content and style, never on geometry. Shares text shaping with
    /// `raw::button`, which (for now) repeats it when the button is drawn.
    pub fn calc_button_intrinsic_size<T: TextSystem>(
        spec: &ButtonSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let style = &spec.style;
        let t = text_system.measure(
            spec.text,
            style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let w = t.size.x + 2.0 * style.pad_x;
        let h = (t.size.y + 2.0 * style.pad_y).max(style.min_height);
        crate::layout::IntrinsicSize::preferred(crate::types::Vec2::new(w, h))
    }

    /// Shape the label single-line, centered within `rect`, returning the draw
    /// rect (block top-left + tight size) and the prepared handle.
    fn centered_text<T: TextSystem>(
        text: &str,
        style: &super::ButtonStyle,
        rect: Rect,
        text_system: &mut T,
    ) -> (Rect, crate::text::TextHandle) {
        let m = text_system.measure(text, style.text_style, crate::text::TextBounds::UNBOUNDED);
        let tx = rect.x + (rect.w - m.size.x) * 0.5;
        let ty = rect.y + (rect.h - m.size.y) * 0.5;
        let text_rect = Rect::new(tx, ty, m.size.x, m.size.y);
        let layout = text_system.prepare(text, style.text_style, text_rect);
        (text_rect, layout.handle)
    }

    /// Low-level button widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn button<T: TextSystem>(
        spec: ButtonSpec,
        state: &mut ButtonState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> ButtonResult {
        // Disabled: register for layout but skip all interaction.
        if spec.disabled {
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            cmds.push(DrawCmd::FillRect {
                rect: spec.rect,
                color: tint(spec.style.background),
            });
            if spec.style.border_width > 0.0 {
                cmds.push(DrawCmd::StrokeRect {
                    rect: spec.rect,
                    color: tint(spec.style.border),
                    width: spec.style.border_width,
                });
            }
            let (text_rect, handle) = centered_text(spec.text, &spec.style, spec.rect, text_system);
            cmds.push(DrawCmd::Text {
                rect: text_rect,
                color: tint(spec.style.text_color),
                handle,
            });
            return ButtonResult {
                content_bounds: spec.rect.inset(spec.style.border_width),
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                focused: false,
            };
        }

        let focused = focus_system.register(state.focus_id, spec.rect, spec.clip_rect);

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
            focus_system.take_focus(state.focus_id);
        }

        focus_system.handle_traversal(focused, input, crate::focus::FocusTraversalKeys::all());

        // Choose fill colour based on interaction state.
        let fill = if pressed {
            spec.style.pressed
        } else if hovered {
            spec.style.hovered
        } else {
            spec.style.background
        };

        // Focus ring drawn first (outset — sits outside the button bounds).
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: spec
                    .rect
                    .inset(-(spec.style.border_width + spec.style.focus_offset)),
                color: spec.style.focus,
                width: spec.style.focus_width,
            });
        }

        // Background fill.
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: fill,
        });

        // Border.
        if spec.style.border_width > 0.0 {
            cmds.push(DrawCmd::StrokeRect {
                rect: spec.rect,
                color: spec.style.border,
                width: spec.style.border_width,
            });
        }

        // Text centered.
        let (text_rect, handle) = centered_text(spec.text, &spec.style, spec.rect, text_system);
        cmds.push(DrawCmd::Text {
            rect: text_rect,
            color: spec.style.text_color,
            handle,
        });

        ButtonResult {
            content_bounds: spec.rect.inset(spec.style.border_width),
            input: InputInfo {
                hovered,
                pressed,
                clicked,
            },
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
    pub focus: Color,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub text_style: crate::text::TextStyle,
    pub text_color: Color,
    pub disabled_alpha: f32,
    /// Horizontal padding each side of the label, used for intrinsic width.
    pub pad_x: f32,
    /// Vertical padding above/below the label, used for intrinsic height.
    pub pad_y: f32,
    /// Minimum intrinsic height (the standard control height); the preferred
    /// height is the larger of this and the padded text height.
    pub min_height: f32,
}

impl ButtonStyle {
    pub fn secondary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.hover,
            pressed: theme.press,
            border: theme.ink,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn primary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.ink,
            hovered: Color::BLACK,
            pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: theme.ink,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            text_color: theme.paper,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn accent_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.rust,
            hovered: Color::from_srgb_u8(176, 79, 35, 255),
            pressed: Color::from_srgb_u8(156, 69, 32, 255),
            border: theme.rust,
            border_width: theme.border,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            text_color: Color::WHITE,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn ghost_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.hover,
            pressed: theme.press,
            border: Color::TRANSPARENT,
            border_width: 0.0,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ButtonState {
    /// True if the mouse was pressed while hovering this button, until the mouse is released.
    pub is_active: bool,
    /// True if the spacebar was pressed while this button was focused, until space or focus is lost.
    pub space_is_active: bool,
    /// Globally unique ID for tracking keyboard focus.
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ButtonSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub text: Option<&'a str>,
    pub style: Option<ButtonStyle>,
    pub clip_rect: Option<ClipRect>,
    pub disabled: Option<bool>,
}

impl<'a> ButtonSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
            self.style = Some(ButtonStyle::secondary_from_theme(theme));
        }
        self
    }
    /// Sets the clip rectangle. High-level context functions supply this automatically — only needed when using the raw API directly.
    pub fn clip_rect(mut self, clip_rect: ClipRect) -> Self {
        self.clip_rect = Some(clip_rect);
        self
    }
    pub fn build(self) -> raw::ButtonSpec<'a> {
        raw::ButtonSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level button widget function using WidgetContext.
///
/// This function accepts a ButtonSpecBuilder and layout parameters, resolves geometry and styles internally,
/// and calls the low-level raw::button function.
pub fn button<'a, T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    // Build the spec up front with a placeholder rect so we can measure the
    // intrinsic size; the real rect is then determined by the layout system and
    // assigned below. Any `rect` set on the builder is ignored by the high-level
    // path — placement is the layout's job (use `ManualLayout`, or the raw fn,
    // for explicit rects).
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let mut spec = builder
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .rect(Rect::PLACEHOLDER)
        .build();
    let intrinsic = raw::calc_button_intrinsic_size(&spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    spec.rect = rect;

    let r = raw::button(
        spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_system,
        ctx.cmds,
    );

    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::ButtonSpec;
    use super::*;

    use crate::test_utils::DummyTextSys;
    use crate::text::FontId;
    use crate::text::TextHandle;
    use crate::theme;
    use crate::types::Vec2;
    use FocusId;
    fn btn_spec(rect: Rect) -> ButtonSpec<'static> {
        ButtonSpec {
            rect,
            text: "Btn",
            style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
            clip_rect: None,
            disabled: false,
        }
    }

    /// Run one frame with two buttons.
    fn two_btn_frame(
        focus_system: &mut FocusSystem,
        s1: &mut ButtonState,
        s2: &mut ButtonState,
        input: &Input,
    ) {
        let mut ts = DummyTextSys;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::button(
            btn_spec(Rect::new(0.0, 0.0, 100.0, 30.0)),
            s1,
            input,
            focus_system,
            &mut ts,
            &mut cmds,
        );
        raw::button(
            btn_spec(Rect::new(0.0, 40.0, 100.0, 30.0)),
            s2,
            input,
            focus_system,
            &mut ts,
            &mut cmds,
        );
        focus_system.end_frame();
    }

    #[test]
    fn test_button_tab_moves_focus_next() {
        let mut focus_system = FocusSystem::new();
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        focus_system.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_tab = true;
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &input);
        // Focus shift resolves at end_frame; confirm in next frame
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &Input::default());
        assert_eq!(
            focus_system.current_focus(),
            Some(s2.focus_id),
            "Tab should move focus to btn2"
        );
    }

    #[test]
    fn test_button_right_arrow_moves_focus_next() {
        let mut focus_system = FocusSystem::new();
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        focus_system.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_right = true;
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &input);
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &Input::default());
        assert_eq!(
            focus_system.current_focus(),
            Some(s2.focus_id),
            "Right arrow should move focus to btn2"
        );
    }

    #[test]
    fn test_button_down_arrow_moves_focus_next() {
        let mut focus_system = FocusSystem::new();
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        focus_system.take_focus(s1.focus_id);

        let mut input = Input::default();
        input.key_pressed_down = true;
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &input);
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &Input::default());
        assert_eq!(
            focus_system.current_focus(),
            Some(s2.focus_id),
            "Down arrow should move focus to btn2"
        );
    }

    #[test]
    fn test_button_shift_tab_moves_focus_prev() {
        let mut focus_system = FocusSystem::new();
        let mut s1 = ButtonState::default();
        let mut s2 = ButtonState::default();
        // Start with focus on s2
        focus_system.take_focus(s2.focus_id);

        let mut input = Input::default();
        input.key_pressed_tab = true;
        input.modifier_shift = true;
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &input);
        two_btn_frame(&mut focus_system, &mut s1, &mut s2, &Input::default());
        assert_eq!(
            focus_system.current_focus(),
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
            text: "Click Me",
            ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
        };
        let btn2_spec = || ButtonSpec {
            text: "Btn2",
            ..btn_spec(Rect::new(0.0, 100.0, 100.0, 50.0))
        };

        // Frame 1: Mouse down on Btn1
        let mut focus_system = FocusSystem::new();
        let mut input = Input {
            mouse_pos: Vec2::new(50.0, 25.0),
            mouse_down: true,
            mouse_pressed: true,
            mouse_clicked: false,
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        let res1 = raw::button(
            btn1_spec(),
            &mut state1,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res1.input.pressed);

        // Frame 2: Mouse dragged over Btn2
        input.mouse_pressed = false;
        input.mouse_pos = Vec2::new(50.0, 125.0);
        raw::button(
            btn1_spec(),
            &mut state1,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        let res2 = raw::button(
            btn2_spec(),
            &mut state2,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

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
            btn1_spec(),
            &mut state1,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        let res2 = raw::button(
            btn2_spec(),
            &mut state2,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

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

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Mouse pressed
        let mut focus_system = FocusSystem::new();
        let mut input = Input {
            mouse_pos: Vec2::new(50.0, 25.0),
            mouse_down: true,
            mouse_pressed: true,
            mouse_clicked: false,
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.pressed);

        // Frame 2: Mouse released
        input.mouse_down = false;
        input.mouse_pressed = false;
        input.mouse_clicked = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert!(res.input.clicked, "Button should register as clicked");
    }

    #[test]
    fn test_button_click_takes_focus() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_system = FocusSystem::new();

        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking button must request focus"
        );
    }

    #[test]
    fn test_button_clipped_click_does_not_take_focus() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        // Mouse is inside the widget rect but outside the clip_rect.
        let spec = ButtonSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 30.0),
            text: "Btn".into(),
            style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
            clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 30.0)),
            disabled: false,
        };
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        let mut state = ButtonState::default();
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away button must not take focus"
        );
    }

    #[test]
    fn test_enter_clicks_raw_button() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_system = FocusSystem::new();

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Register and take focus explicitly
        let mut input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.take_focus(state.focus_id);
        focus_system.end_frame();

        // Frame 2: Press Enter
        input.key_pressed_enter = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.clicked, "Button should be clicked by Enter key");
    }

    #[test]
    fn test_hover_and_press_state() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_system = FocusSystem::new();

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Mouse outside
        let mut input = Input {
            mouse_pos: Vec2::new(150.0, 150.0),
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(!res.input.hovered);
        assert!(!res.input.pressed);

        // Frame 2: Mouse inside, not down
        input.mouse_pos = Vec2::new(50.0, 25.0);
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.hovered, "Should be hovered");
        assert!(!res.input.pressed, "Should not be pressed");

        // Frame 3: Mouse down inside
        input.mouse_down = true;
        input.mouse_pressed = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.hovered, "Should be hovered while pressed down");
        assert!(res.input.pressed, "Should be pressed");

        // Frame 4: Drag outside
        input.mouse_pos = Vec2::new(150.0, 150.0);
        input.mouse_pressed = false;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
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
        let mut focus_system = FocusSystem::new();

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Focus
        let mut input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.take_focus(state.focus_id);
        focus_system.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(
            res.input.pressed,
            "Button should be visually pressed while space is down"
        );
        assert!(!res.input.clicked, "Button should not be clicked yet");

        // Frame 3: Space held
        input.key_pressed_space = false;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.pressed, "Button should remain pressed");
        assert!(!res.input.clicked, "Button should not be clicked yet");

        // Frame 4: Space released
        input.key_down_space = false;
        input.key_released_space = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(!res.input.pressed, "Button should not be pressed");
        assert!(res.input.clicked, "Button should be clicked on release");
    }

    #[test]
    fn test_spacebar_loses_focus_does_not_click() {
        let mut text_system = DummyTextSys;
        let mut state = ButtonState::default();
        let mut focus_system = FocusSystem::new();

        let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

        // Frame 1: Focus
        let mut input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.take_focus(state.focus_id);
        focus_system.end_frame();

        // Frame 2: Space down
        input.key_down_space = true;
        input.key_pressed_space = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(res.input.pressed);

        // Frame 3: Lose focus!
        input.key_pressed_space = false;
        focus_system.take_focus(FocusId::new()); // Give focus to something else
        focus_system.end_frame();

        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(
            !res.input.pressed,
            "Should lose pressed state when focus lost"
        );

        // Frame 4: Release space
        input.key_down_space = false;
        input.key_released_space = true;
        let res = raw::button(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert!(!res.input.clicked, "Should not click because it lost focus");
    }

    // ── Visual Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_button_visual_normal() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();
        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            &cmds[..],
            &[
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
            ]
        );
    }

    #[test]
    fn test_button_visual_hovered() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0); // Inside bounds
        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            hovered,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            &cmds[..],
            &[
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
            ]
        );
    }

    #[test]
    fn test_button_visual_pressed() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(50.0, 25.0);
        input.mouse_down = true;
        input.mouse_pressed = true;

        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            pressed,
            border,
            border_width,
            text_color,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        assert_eq!(
            &cmds[..],
            &[
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
            ]
        );
    }

    #[test]
    fn test_button_visual_focused() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

        focus_system.take_focus(state.focus_id);

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let ButtonStyle {
            background,
            border,
            border_width,
            text_color,
            focus,
            focus_offset,
            focus_width,
            ..
        } = ButtonStyle::primary_from_theme(&theme::Theme::default());

        let expected_focus_rect =
            Rect::new(10.0, 10.0, 100.0, 30.0).inset(-(border_width + focus_offset));

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::StrokeRect {
                    rect: expected_focus_rect,
                    color: focus,
                    width: focus_width,
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
            ]
        );
    }

    #[test]
    fn test_button_visual_disabled() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let spec = ButtonSpec {
            disabled: true,
            ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let alpha = 0.32_f32;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let primary_style = ButtonStyle::primary_from_theme(&theme::Theme::default());
        let expected_bg = tint(primary_style.background);
        let expected_border = tint(primary_style.border);
        let expected_text = tint(primary_style.text_color);
        let border_width = primary_style.border_width;

        assert_eq!(
            &cmds[..],
            &[
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
            ]
        );
    }

    #[test]
    fn test_regression_custom_style_no_theme_lookup() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let state = ButtonState::default();
        let input = Input::default();

        let custom_style = ButtonStyle {
            background: Color::from_srgb_u8(100, 150, 200, 255),
            hovered: Color::from_srgb_u8(110, 160, 210, 255),
            pressed: Color::from_srgb_u8(120, 170, 220, 255),
            border: Color::from_srgb_u8(220, 230, 240, 255),
            border_width: 4.5,
            focus: Color::from_srgb_u8(255, 0, 0, 255),
            focus_width: 2.0,
            focus_offset: 2.0,
            text_style: crate::text::TextStyle::new(
                FontId(0),
                19.5,
                400,
                crate::text::TextFlow::single_line(),
            ),
            text_color: Color::from_srgb_u8(50, 60, 70, 255),
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: 28.0,
        };

        let spec = ButtonSpec {
            rect: Rect::new(5.0, 15.0, 120.0, 45.0),
            text: "Explicit Spec",
            style: custom_style,
            clip_rect: None,
            disabled: false,
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _res = raw::button(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            &cmds[..],
            &[
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
            ]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = ButtonSpecBuilder::new().text("test");
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        let expected = ButtonStyle::secondary_from_theme(&theme);
        assert_eq!(
            builder.style.unwrap().text_style.font,
            expected.text_style.font
        );
        assert_eq!(
            builder.style.unwrap().text_style.size,
            expected.text_style.size
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let default_primary = ButtonStyle::primary_from_theme(&theme::Theme::default());
        let custom_style = ButtonStyle {
            text_style: crate::text::TextStyle {
                size: 99.0,
                ..default_primary.text_style
            },
            ..default_primary
        };
        let builder = ButtonSpecBuilder::new().text("test").style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut btn_state = ButtonState::default();
        // Under ManualLayout the layout param *is* the rect — the sanctioned way
        // to place a high-level widget explicitly.
        let result = super::button(
            &mut ctx,
            ButtonSpecBuilder::new().text("X"),
            placement,
            &mut btn_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_high_level_honors_user_style() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        // A user-set builder field (style) must be honored, not overwritten by
        // theme defaults.
        let custom = ButtonStyle {
            background: Color::from_srgb_u8(1, 2, 3, 255),
            ..ButtonStyle::accent_from_theme(&theme::Theme::default())
        };
        let mut btn_state = ButtonState::default();
        // Placed away from the default mouse position (0,0) so it isn't hovered.
        super::button(
            &mut ctx,
            ButtonSpecBuilder::new().text("X").style(custom),
            Rect::new(100.0, 100.0, 40.0, 28.0),
            &mut btn_state,
        );
        let has_custom_fill = cmds
            .iter()
            .any(|c| matches!(c, DrawCmd::FillRect { color, .. } if *color == custom.background));
        assert!(
            has_custom_fill,
            "high-level button must honor user-set style"
        );
    }

    #[test]
    fn test_calc_button_intrinsic_size() {
        let mut ts = DummyTextSys;
        // Measured from a spec with a placeholder rect — calc must not read it.
        let spec = btn_spec(Rect::PLACEHOLDER);
        // "Btn" = 3 chars * 8px = 24 wide, 16 tall (DummyTextSys).
        // width = 24 + 2*pad_x(14) = 52; height = max(16 + 2*pad_y(6), min_height 28) = 28.
        let i = raw::calc_button_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(52.0, 28.0)));
    }

    #[test]
    fn test_button_auto_layout_uses_intrinsic_size() {
        use crate::layout::Placement2D;
        use crate::layouts::{ColumnLayout, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut ctx = WidgetContext::root(
            theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut col = ctx.child_with_layout(
            Rect::new(10.0, 10.0, 300.0, 400.0),
            ColumnLayout { spacing: 0.0 },
        );
        let mut st = ButtonState::default();
        // Auto on both axes → the button sizes to its label intrinsic.
        // "Save" = 4*8 = 32 wide; width = 32 + 28 = 60; height = 28.
        let r = super::button(
            &mut col,
            ButtonSpecBuilder::new().text("Save"),
            Placement2D::auto(),
            &mut st,
        );
        assert_eq!(r.layout.bounds, Rect::new(10.0, 10.0, 60.0, 28.0));
    }
}
