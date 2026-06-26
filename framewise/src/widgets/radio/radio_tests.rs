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
    raw::post_layout_radio(
        radio_spec(Rect::new(0.0, 0.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state1,
        input,
        focus_system,
        cmds,
    );
    raw::post_layout_radio(
        radio_spec(Rect::new(0.0, 40.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state2,
        input,
        focus_system,
        cmds,
    );
}

#[test]
fn test_radio_overlapping_hover() {
    let mut state1 = RadioState::default();
    let mut state2 = RadioState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_radio(
                radio_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_radio(
                radio_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state2,
                input,
                focus_system,
                cmds,
            );
            (res1.input, res2.input)
        },
    );
}

#[test]
fn test_radio_overlapping_click() {
    let mut state1 = RadioState::default();
    let mut state2 = RadioState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        true,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_radio(
                radio_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_radio(
                radio_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state2,
                input,
                focus_system,
                cmds,
            );
            (res1.input, res2.input)
        },
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
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec,
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    let mut focus_system = FocusSystem::new();
    let mut state = RadioState {
        checked: false,
        ..Default::default()
    };

    // Warmup frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    let mut input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = RadioState {
        checked: false,
        ..Default::default()
    };

    // Warmup frame with mouse inside but not pressed
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame with mouse pressed down
    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec,
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    let mut focus_system = FocusSystem::new();
    let mut state = RadioState {
        checked: true,
        ..Default::default()
    };

    // Warmup frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let mut state = state;
    let center = Vec2::new(17.0, 17.0);
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec,
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
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
                radius: s.radius + s.focus.unwrap().offset + s.focus.unwrap().stroke.width * 0.5,
                color: s.focus.unwrap().stroke.color,
                width: s.focus.unwrap().stroke.width,
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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
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
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec,
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
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
                color: tint(s.border.unwrap().color),
                width: s.border.unwrap().width,
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
            raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
            raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
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
fn test_radio_clipped_click_does_not_take_focus() {
    let mut state = RadioState::default();

    crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
        &mut state,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_radio(
                RadioSpec {
                    clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
                    ..radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                },
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
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
            raw::post_layout_radio(
                RadioSpec {
                    disabled: true,
                    ..radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                },
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
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
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec(),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    input.key_pressed_enter = true;
    focus_system.begin_frame();
    let result = raw::post_layout_radio(
        spec(),
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
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
            raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
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
            let res1 = raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 110.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
            raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
            raw::post_layout_radio(
                radio_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::RadioPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
    let mut output = crate::Output::default();
    let mut ctx = crate::widget::WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
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
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let mut output = crate::Output::default();
    let mut ctx = crate::widget::WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
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
        .any(|c| matches!(c, DrawCmd::FillCircle {  color, .. } if *color == custom.background));
    assert!(
        has_custom_fill,
        "high-level radio must honor user-set style"
    );
}

#[test]
fn test_size_radio() {
    let theme = crate::theme::Theme::default();
    let style = RadioStyle::from_theme(&theme);
    let spec = raw::RadioPreLayoutSpec { style };
    let size_request = raw::pre_layout_radio(&spec, SizeOffer::UNBOUNDED).size_request;
    assert_eq!(size_request, SizeRequest::preferred(Vec2::new(14.0, 14.0)));
}

#[test]
fn test_radio_visual_vertically_centered() {
    let spec = radio_spec(Rect::new(10.0, 10.0, 14.0, 20.0));
    let s = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_radio(
        spec,
        raw::RadioPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut RadioState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    // Expect Y to be 10.0 + (20.0 - 14.0) * 0.5 = 13.0
    // Expect center to be Vec2::new(17.0, 20.0) -> cx = 10.0 + 7.0 = 17.0, cy = 13.0 + 7.0 = 20.0
    let center = Vec2::new(17.0, 20.0);
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
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_labelled_radio_request_size() {
    use crate::layouts::ManualLayout;
    let mut text_backend = crate::test_utils::TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );

    let mut state = RadioState::default();
    // TestTextBackend logical size reports 8.0 per character. "vsync" is 5 chars -> 40.0.
    // Height is 16.0. Radio size is 14.0 x 14.0. Gap is 8.0.
    // Combined width: 14.0 + 8.0 + 40.0 = 62.0.
    // Combined height: max(14.0, 16.0) = 16.0.
    let result = super::labelled_radio(
        &mut ctx,
        RadioSpecBuilder::new(),
        "vsync",
        Rect::new(0.0, 0.0, 100.0, 20.0),
        &mut state,
    );

    assert_eq!(result.layout.bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
}

#[test]
fn test_labelled_radio_click_label_toggles_state() {
    use crate::layouts::ManualLayout;
    let mut state = RadioState::default();

    crate::widgets::test_helpers::assert_labelled_widget_click_toggles(
        &mut state,
        Vec2::new(40.0, 10.0),
        |state, input, focus, cmds| {
            let mut text_backend = crate::test_utils::TestTextBackend::default();
            let mut output = crate::Output::default();
            let mut ctx = WidgetContext::root(
                crate::theme::Theme::framewise(),
                &mut text_backend,
                focus,
                input,
                &mut output,
                ManualLayout,
                Rect::new(0.0, 0.0, 800.0, 600.0),
                cmds,
            );
            super::labelled_radio(
                &mut ctx,
                RadioSpecBuilder::new(),
                "vsync",
                Rect::new(0.0, 0.0, 100.0, 20.0),
                state,
            );
        },
    );

    assert!(state.checked);
}

#[test]
fn test_labelled_radio_disabled_label_visual() {
    use crate::layouts::ManualLayout;
    let mut text_backend = crate::test_utils::TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let theme = crate::theme::Theme::framewise();
    {
        let mut output = crate::Output::default();
        let mut ctx = WidgetContext::root(
            theme,
            &mut text_backend,
            &mut focus,
            &input,
            &mut output,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );

        let mut state = RadioState::default();
        super::labelled_radio(
            &mut ctx,
            RadioSpecBuilder::new().disabled(true),
            "vsync",
            Rect::new(0.0, 0.0, 100.0, 20.0),
            &mut state,
        );
    }

    // Find the text draw command to check its color.
    let text_cmd = cmds.iter().find_map(|cmd| {
        if let DrawCmd::GlyphRun { color, .. } = cmd {
            Some(*color)
        } else {
            None
        }
    });

    assert!(text_cmd.is_some());
    let color = text_cmd.unwrap();
    // The default ink color from theme should have disabled_alpha (0.35) applied to its alpha channel.
    let default_label_style = crate::widgets::label::LabelStyle::from_theme(&theme);
    let expected_alpha = default_label_style.text_color.a * 0.35;
    assert!((color.a - expected_alpha).abs() < 1e-4);
}
