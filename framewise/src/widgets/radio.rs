use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{IntrinsicSize, LayoutState},
    text::TextSystem,
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioSpec {
        pub layer: Layer,
        /// Top-left of the 14x14 bounding area.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::RadioStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioCalcIntrinsicSizeSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Compute intrinsic size for Radio. Currently returns UNKNOWN.
    pub fn calc_radio_intrinsic_size(_spec: &RadioCalcIntrinsicSizeSpec) -> IntrinsicSize {
        IntrinsicSize::UNKNOWN
    }

    /// Low-level radio widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn radio(
        spec: RadioSpec,
        state: &mut RadioState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> RadioResult {
        let interaction = crate::widgets::widget_helpers::handle_press_interaction(
            crate::widgets::widget_helpers::PressInteractionSpec {
                focus_id: state.focus_id,
                rect: spec.rect,
                clip_rect: spec.clip_rect,
                disabled: spec.disabled,
                traversal_keys: crate::focus::FocusTraversalKeys::all(),
            },
            input,
            focus_system,
            &mut state.is_active,
            &mut state.space_is_active,
        );
        let focused = interaction.focused;
        let input_info = interaction.input;

        if input_info.clicked {
            state.checked = true;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let cx = spec.rect.x + s.radius;
        let cy = spec.rect.y + s.radius;
        let center = Vec2::new(cx, cy);

        // Focus ring (outset 2px).
        if focused {
            cmds.push(DrawCmd::StrokeCircle {
                center,
                radius: s.radius + s.focus_offset + s.focus_width * 0.5,
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_focus_z(),
            });
        }

        // Background fill.
        let fill = if state.checked {
            crate::widgets::widget_helpers::interaction_color(
                s.background,
                s.selected_hovered,
                s.selected_pressed,
                input_info.hovered,
                input_info.pressed,
            )
        } else {
            crate::widgets::widget_helpers::interaction_color(
                s.background,
                s.hovered,
                s.pressed,
                input_info.hovered,
                input_info.pressed,
            )
        };
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: s.radius,
            color: tint(fill),
            z: spec.layer.get_z(),
        });

        // Outer ring.
        cmds.push(DrawCmd::StrokeCircle {
            center,
            radius: s.radius,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        // Inner dot when selected.
        if state.checked {
            cmds.push(DrawCmd::FillCircle {
                center,
                radius: s.dot_radius,
                color: tint(s.dot),
                z: spec.layer.get_z(),
            });
        }

        RadioResult {
            input: input_info,
            focused,
            content_bounds: spec.rect.inset(s.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioStyle {
    pub radius: f32,
    pub dot_radius: f32,
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub selected_hovered: Color,
    pub selected_pressed: Color,
    pub border: Color,
    pub dot: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl RadioStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            radius: 7.0,
            dot_radius: 3.0,
            background: theme.paper_elev,
            hovered: theme.hover,
            pressed: theme.press,
            selected_hovered: theme.hover,
            selected_pressed: theme.press,
            border: theme.ink,
            dot: theme.ink,
            focus: theme.rust,
            border_width: 1.5,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioState {
    pub checked: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RadioResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RadioSpec {
    pub disabled: bool,
    pub style: RadioStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RadioSpecBuilder {
    pub disabled: Option<bool>,
    pub style: Option<RadioStyle>,
}

impl RadioSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(RadioStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> RadioSpec {
        RadioSpec {
            disabled: self.disabled.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level radio widget function using WidgetContext.
///
/// This function accepts a RadioSpecBuilder and calls the low-level raw::radio function.
pub fn radio<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: RadioSpecBuilder,
    layout_params: S::Params,
    state: &mut RadioState,
) -> RadioResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::RadioCalcIntrinsicSizeSpec {};
    let intrinsic = raw::calc_radio_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::RadioSpec {
        layer: ctx.layer,
        rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::radio(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);

    RadioResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::RadioSpec;
    use super::*;

    fn radio_spec(rect: Rect) -> RadioSpec {
        RadioSpec {
            layer: Layer::default(),
            rect,
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        }
    }

    fn draw_two_radios(
        focus_system: &mut FocusSystem,
        state1: &mut RadioState,
        state2: &mut RadioState,
        input: &Input,
        cmds: &mut DrawCommands,
    ) {
        raw::radio(
            radio_spec(Rect::new(0.0, 0.0, 14.0, 14.0)),
            state1,
            input,
            focus_system,
            cmds,
        );
        raw::radio(
            radio_spec(Rect::new(0.0, 40.0, 14.0, 14.0)),
            state2,
            input,
            focus_system,
            cmds,
        );
    }

    #[test]
    fn test_radio_tab_moves_focus_next() {
        let mut state1 = RadioState::default();
        let mut state2 = RadioState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_tab_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_radios(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_radio_right_arrow_moves_focus_next() {
        let mut state1 = RadioState::default();
        let mut state2 = RadioState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_radios(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_radio_down_arrow_moves_focus_next() {
        let mut state1 = RadioState::default();
        let mut state2 = RadioState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_radios(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_radio_shift_tab_moves_focus_prev() {
        let mut state1 = RadioState::default();
        let mut state2 = RadioState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_radios(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_radio_visual_unselected() {
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
                checked: false,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_hovered() {
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
                checked: false,
                ..Default::default()
            },
            &input,
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.hovered,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_pressed() {
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            mouse_down: true,
            mouse_pressed: true,
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
                checked: false,
                ..Default::default()
            },
            &input,
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.pressed,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_selected() {
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
                checked: true,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.dot_radius,
                    color: s.dot,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_selected_hovered() {
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
                checked: true,
                ..Default::default()
            },
            &input,
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.selected_hovered,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.dot_radius,
                    color: s.dot,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_focused() {
        let state = RadioState::default();
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let mut state = state;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius + s.focus_offset + s.focus_width * 0.5,
                    color: s.focus,
                    width: s.focus_width,
                    z: 1,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_disabled() {
        let spec = RadioSpec {
            disabled: true,
            ..radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
        };
        let s = spec.style;
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.background),
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.border),
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_click_triggers_clicked_state() {
        let mut state = RadioState::default();

        crate::widgets::test_helpers::assert_mouse_click_on_release(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
        assert!(state.checked);
    }

    #[test]
    fn test_radio_click_takes_focus() {
        let mut state = RadioState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_mouse_press_takes_focus(
            &mut state,
            focus_id,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_radio_clipped_click_does_not_take_focus() {
        let mut state = RadioState::default();

        crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::radio(
                    RadioSpec {
                        clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
                        ..radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                    },
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_radio_disabled_ignores_interaction() {
        let mut state = RadioState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
            &mut state,
            focus_id,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::radio(
                    RadioSpec {
                        disabled: true,
                        ..radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                    },
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
        assert!(!state.checked);
    }

    #[test]
    fn test_enter_selects_raw_radio() {
        let mut focus_system = FocusSystem::new();
        let mut state = RadioState::default();
        let mut input = Input::default();

        let spec = || radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::radio(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.take_focus(state.focus_id);
        focus_system.end_frame();

        input.key_pressed_enter = true;
        focus_system.begin_frame();
        let result = raw::radio(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert!(result.input.clicked, "Radio should be clicked by Enter key");
        assert!(state.checked, "Enter key must select radio state");
    }

    #[test]
    fn test_radio_hover_and_press_state() {
        let mut state = RadioState::default();

        crate::widgets::test_helpers::assert_hover_and_press_state(
            &mut state,
            Vec2::new(15.0, 15.0),
            Vec2::new(150.0, 150.0),
            |state, input, focus_system, cmds| {
                raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_radio_drag_off_and_release_does_not_select_other_radio() {
        let mut state1 = RadioState::default();
        let mut state2 = RadioState::default();

        crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
            &mut state1,
            &mut state2,
            Vec2::new(15.0, 15.0),
            Vec2::new(15.0, 115.0),
            false,
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state1,
                    input,
                    focus_system,
                    cmds,
                );
                let res2 = raw::radio(
                    radio_spec(Rect::new(10.0, 110.0, 14.0, 14.0)),
                    state2,
                    input,
                    focus_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );

        assert!(
            !state2.checked,
            "Dragging onto another radio must not select it on release"
        );
    }

    #[test]
    fn test_radio_spacebar_click() {
        let mut state = RadioState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );

        assert!(state.checked, "Spacebar release must select radio state");
    }

    #[test]
    fn test_radio_spacebar_loses_focus_does_not_click() {
        let mut state = RadioState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::radio(
                    radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );

        assert!(
            !state.checked,
            "Losing focus before Space release must not select radio state"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = RadioSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(RadioStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = RadioStyle::from_theme(&theme);
        custom_style.radius = 99.0;
        let builder = RadioSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().radius, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::DummyTextSys;
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
        let mut radio_state = RadioState::default();
        let result = super::radio(
            &mut ctx,
            RadioSpecBuilder::new(),
            placement,
            &mut radio_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_high_level_honors_user_style() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::DummyTextSys;
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
        let custom = RadioStyle {
            background: Color::from_srgb_u8(1, 2, 3, 255),
            ..RadioStyle::from_theme(&crate::theme::Theme::default())
        };
        let mut radio_state = RadioState::default();
        super::radio(
            &mut ctx,
            RadioSpecBuilder::new().style(custom),
            Rect::new(100.0, 100.0, 14.0, 14.0),
            &mut radio_state,
        );

        let has_custom_fill = cmds
            .iter()
            .any(|c| matches!(c, DrawCmd::FillCircle { color, .. } if *color == custom.background));
        assert!(
            has_custom_fill,
            "high-level radio must honor user-set style"
        );
    }

    #[test]
    fn test_calc_radio_intrinsic_size() {
        let spec = raw::RadioCalcIntrinsicSizeSpec {};
        let intrinsic = raw::calc_radio_intrinsic_size(&spec);
        assert_eq!(intrinsic, IntrinsicSize::UNKNOWN);
    }
}
