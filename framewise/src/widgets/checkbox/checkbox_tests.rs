use super::raw::CheckboxSpec;
use super::*;

fn checkbox_spec(rect: Rect) -> CheckboxSpec {
    CheckboxSpec {
        rect,
        disabled: false,
        allowed_checked_states: vec![CheckedState::Unchecked, CheckedState::Checked],
        style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
        layer: Layer::default(),
    }
}

fn tri_state_checkbox_spec(rect: Rect) -> CheckboxSpec {
    CheckboxSpec {
        allowed_checked_states: vec![
            CheckedState::Unchecked,
            CheckedState::Checked,
            CheckedState::Indeterminate,
        ],
        ..checkbox_spec(rect)
    }
}

fn draw_two_checkboxes(
    focus_system: &mut FocusSystem,
    state1: &mut CheckboxState,
    state2: &mut CheckboxState,
    input: &Input,
    cmds: &mut DrawCommands,
) {
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(0.0, 0.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state1,
        input,
        focus_system,
        cmds,
    );
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(0.0, 40.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state2,
        input,
        focus_system,
        cmds,
    );
}

#[test]
fn test_checkbox_overlapping_hover() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::CheckboxPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_overlapping_click() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        true,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::CheckboxPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_tab_moves_focus_next() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_tab_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_checkboxes(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_checkbox_right_arrow_moves_focus_next() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_checkboxes(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_checkbox_down_arrow_moves_focus_next() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_checkboxes(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_checkbox_shift_tab_moves_focus_prev() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_checkboxes(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_checkbox_visual_off() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut CheckboxState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_vertically_centered() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 20.0));
    let s = spec.style;
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut CheckboxState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    // Expect Y to be 10.0 + (20.0 - 14.0) * 0.5 = 13.0
    let expected_rect = Rect::new(10.0, 13.0, 14.0, 14.0);
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: expected_rect,
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: expected_rect,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_hovered() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = CheckboxState::default();

    // Warmup frame to establish hover claim
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.hovered,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_pressed() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let mut input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = CheckboxState::default();

    // Warmup frame with mouse inside but not pressed
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.pressed,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_on() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
    let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
    let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut CheckboxState {
            checked: CheckedState::Checked,
            ..Default::default()
        },
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.selected_fill,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: true,
                p0,
                p1,
                color: s.mark.color,
                width: s.mark.width,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: true,
                p0: p1,
                p1: p2,
                color: s.mark.color,
                width: s.mark.width,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_on_hovered() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
    let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
    let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = CheckboxState {
        checked: CheckedState::Checked,
        ..Default::default()
    };

    // Warmup frame to establish hover claim
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.selected_hovered,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: true,
                p0,
                p1,
                color: s.mark.color,
                width: s.mark.width,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: true,
                p0: p1,
                p1: p2,
                color: s.mark.color,
                width: s.mark.width,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_indeterminate() {
    let spec = tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut CheckboxState {
            checked: CheckedState::Indeterminate,
            ..Default::default()
        },
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.selected_fill,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                color: s.mark.color,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_indeterminate_pressed() {
    let spec = tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let mut input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = CheckboxState {
        checked: CheckedState::Indeterminate,
        ..Default::default()
    };

    // Warmup frame with mouse inside but not pressed
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
        raw::CheckboxPreLayoutResult {
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.selected_pressed,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                color: s.mark.color,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_clamps_state_to_first_allowed_state() {
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let mut state = CheckboxState {
        checked: CheckedState::Indeterminate,
        ..Default::default()
    };

    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut FocusSystem::new(),
        &mut DrawCommands::new(),
    );

    assert_eq!(
        state.checked,
        CheckedState::Unchecked,
        "Checkbox should clamp to the first allowed state"
    );
}

#[test]
fn test_checkbox_click_cycles_allowed_states_in_order() {
    let mut state = CheckboxState::default();

    for expected in [
        CheckedState::Checked,
        CheckedState::Indeterminate,
        CheckedState::Unchecked,
    ] {
        crate::widgets::test_helpers::assert_mouse_click_on_release(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::post_layout_checkbox(
                    tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    raw::CheckboxPreLayoutResult {
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

        assert_eq!(state.checked, expected);
    }
}

#[test]
fn test_checkbox_click_honors_nonstandard_allowed_state_order() {
    let mut state = CheckboxState {
        checked: CheckedState::Checked,
        ..Default::default()
    };
    let spec = || CheckboxSpec {
        allowed_checked_states: vec![CheckedState::Checked, CheckedState::Indeterminate],
        ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
    };

    for expected in [CheckedState::Indeterminate, CheckedState::Checked] {
        crate::widgets::test_helpers::assert_mouse_click_on_release(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::post_layout_checkbox(
                    spec(),
                    raw::CheckboxPreLayoutResult {
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

        assert_eq!(state.checked, expected);
    }
}

#[test]
#[should_panic(expected = "CheckboxSpec::allowed_checked_states must not be empty")]
fn test_checkbox_panics_when_allowed_states_is_empty() {
    let mut state = CheckboxState::default();
    raw::post_layout_checkbox(
        CheckboxSpec {
            allowed_checked_states: Vec::new(),
            ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
        },
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut FocusSystem::new(),
        &mut DrawCommands::new(),
    );
}

#[test]
fn test_checkbox_visual_focused() {
    let state = CheckboxState::default();
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let mut state = state;
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
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
            DrawCmd::BorderRect {
                rect: r.inset(-s.focus.unwrap().offset),
                color: s.focus.unwrap().stroke.color,
                width: s.focus.unwrap().stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_visual_disabled() {
    let spec = CheckboxSpec {
        disabled: true,
        ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
    };
    let s = spec.style;
    let alpha = s.disabled_alpha;
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
    let r = Rect::new(10.0, 10.0, 14.0, 14.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec,
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut CheckboxState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: tint(s.background),
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: tint(s.border.unwrap().color),
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_checkbox_click_triggers_clicked_state() {
    let mut state = CheckboxState::default();

    crate::widgets::test_helpers::assert_mouse_click_on_release(
        &mut state,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_click_takes_focus() {
    let mut state = CheckboxState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_mouse_press_takes_focus(
        &mut state,
        focus_id,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_clipped_click_does_not_take_focus() {
    let mut state = CheckboxState::default();

    crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
        &mut state,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                CheckboxSpec {
                    clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
                    ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                },
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_disabled_ignores_interaction() {
    let mut state = CheckboxState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
        &mut state,
        focus_id,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                CheckboxSpec {
                    disabled: true,
                    ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                },
                raw::CheckboxPreLayoutResult {
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
    assert_eq!(state.checked, CheckedState::Unchecked);
}

#[test]
fn test_enter_toggles_raw_checkbox() {
    let mut focus_system = FocusSystem::new();
    let mut state = CheckboxState::default();
    let mut input = Input::default();

    let spec = || checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_checkbox(
        spec(),
        raw::CheckboxPreLayoutResult {
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
    let result = raw::post_layout_checkbox(
        spec(),
        raw::CheckboxPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(
        result.input.clicked,
        "Checkbox should be clicked by Enter key"
    );
    assert_eq!(
        state.checked,
        CheckedState::Checked,
        "Enter key must toggle checkbox state"
    );
}

#[test]
fn test_checkbox_hover_and_press_state() {
    let mut state = CheckboxState::default();

    crate::widgets::test_helpers::assert_hover_and_press_state(
        &mut state,
        Vec2::new(15.0, 15.0),
        Vec2::new(150.0, 150.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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
fn test_checkbox_drag_off_and_release_does_not_click_other_checkbox() {
    let mut state1 = CheckboxState::default();
    let mut state2 = CheckboxState::default();

    crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
        &mut state1,
        &mut state2,
        Vec2::new(15.0, 15.0),
        Vec2::new(15.0, 115.0),
        false,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 110.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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

    assert_eq!(
        state2.checked,
        CheckedState::Unchecked,
        "Dragging onto another checkbox must not toggle it on release"
    );
}

#[test]
fn test_checkbox_spacebar_click() {
    let mut state = CheckboxState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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

    assert_eq!(
        state.checked,
        CheckedState::Checked,
        "Spacebar release must toggle checkbox state"
    );
}

#[test]
fn test_checkbox_spacebar_loses_focus_does_not_click() {
    let mut state = CheckboxState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_checkbox(
                checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                raw::CheckboxPreLayoutResult {
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

    assert_eq!(
        state.checked,
        CheckedState::Unchecked,
        "Losing focus before Space release must not toggle checkbox state"
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = CheckboxSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(CheckboxStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = CheckboxStyle::from_theme(&theme);
    custom_style.size = 99.0;
    let builder = CheckboxSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().size, 99.0);
}

#[test]
fn test_builder_preserves_allowed_checked_states() {
    let theme = crate::theme::Theme::framewise();
    let allowed_checked_states = vec![CheckedState::Checked, CheckedState::Indeterminate];

    let spec = CheckboxSpecBuilder::new()
        .allowed_checked_states(allowed_checked_states.clone())
        .defaults_from_theme(&theme)
        .build();

    assert_eq!(spec.allowed_checked_states, allowed_checked_states);
}

#[test]
fn test_high_level_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
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
    let mut cb_state = CheckboxState::default();
    let result = super::checkbox(
        &mut ctx,
        CheckboxSpecBuilder::new(),
        placement,
        &mut cb_state,
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
    let mut cmds = crate::draw::DrawCommands::new();
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
    let custom = CheckboxStyle {
        background: Color::from_srgb_u8(1, 2, 3, 255),
        ..CheckboxStyle::from_theme(&crate::theme::Theme::default())
    };
    let mut cb_state = CheckboxState::default();
    super::checkbox(
        &mut ctx,
        CheckboxSpecBuilder::new().style(custom),
        Rect::new(100.0, 100.0, 14.0, 14.0),
        &mut cb_state,
    );

    let has_custom_fill = cmds
        .iter()
        .any(|c| matches!(c, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == custom.background));
    assert!(
        has_custom_fill,
        "high-level checkbox must honor user-set style"
    );
}

#[test]
fn test_high_level_honors_allowed_checked_states() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
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
    let mut cb_state = CheckboxState {
        checked: CheckedState::Indeterminate,
        ..Default::default()
    };

    super::checkbox(
        &mut ctx,
        CheckboxSpecBuilder::new()
            .allowed_checked_states(vec![CheckedState::Checked, CheckedState::Indeterminate]),
        Rect::new(100.0, 100.0, 14.0, 14.0),
        &mut cb_state,
    );

    assert_eq!(
        cb_state.checked,
        CheckedState::Indeterminate,
        "high-level checkbox must pass allowed states to raw checkbox"
    );
}

#[test]
fn test_size_checkbox() {
    let theme = crate::theme::Theme::default();
    let style = CheckboxStyle::from_theme(&theme);
    let spec = raw::CheckboxPreLayoutSpec { style };
    let size_request = raw::pre_layout_checkbox(&spec, SizeOffer::UNBOUNDED).size_request;
    assert_eq!(size_request, SizeRequest::preferred(Vec2::new(14.0, 14.0)));
}

#[test]
fn test_labelled_checkbox_request_size() {
    use crate::layouts::ManualLayout;
    let mut text_backend = crate::test_utils::TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new();
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

    let mut state = CheckboxState::default();
    // TestTextBackend logical size reports 8.0 per character. "vsync" is 5 chars -> 40.0.
    // Height is 16.0. Checkbox size is 14.0. Gap is 8.0.
    // Combined width: 14.0 + 8.0 + 40.0 = 62.0.
    // Combined height: max(14.0, 16.0) = 16.0.
    let result = super::labelled_checkbox(
        &mut ctx,
        CheckboxSpecBuilder::new(),
        "vsync",
        Rect::new(0.0, 0.0, 100.0, 20.0),
        &mut state,
    );

    assert_eq!(result.layout.bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
}

#[test]
fn test_labelled_checkbox_click_label_toggles_state() {
    use crate::layouts::ManualLayout;
    let mut state = CheckboxState::default();

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
            super::labelled_checkbox(
                &mut ctx,
                CheckboxSpecBuilder::new(),
                "vsync",
                Rect::new(0.0, 0.0, 100.0, 20.0),
                state,
            );
        },
    );

    assert_eq!(state.checked, CheckedState::Checked);
}

#[test]
fn test_labelled_checkbox_disabled_label_visual() {
    use crate::layouts::ManualLayout;
    let mut text_backend = crate::test_utils::TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new();
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

        let mut state = CheckboxState::default();
        super::labelled_checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new().disabled(true),
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
