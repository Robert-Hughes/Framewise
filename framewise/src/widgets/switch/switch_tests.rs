use super::raw::SwitchSpec;
use super::*;
use crate::types::Vec2;

fn switch_spec(rect: Rect) -> SwitchSpec {
    SwitchSpec {
        rect,
        disabled: false,
        style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
        layer: Layer::default(),
    }
}

fn draw_two_switches(
    focus_system: &mut FocusSystem,
    state1: &mut SwitchState,
    state2: &mut SwitchState,
    input: &Input,
    cmds: &mut DrawCommands,
) {
    raw::post_layout_switch(
        switch_spec(Rect::new(0.0, 0.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state1,
        input,
        focus_system,
        cmds,
    );
    raw::post_layout_switch(
        switch_spec(Rect::new(0.0, 40.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state2,
        input,
        focus_system,
        cmds,
    );
}

#[test]
fn test_switch_overlapping_hover() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_switch(
                switch_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::SwitchPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_switch(
                switch_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::SwitchPreLayoutResult {
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
fn test_switch_overlapping_click() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        true,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_switch(
                switch_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::SwitchPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_switch(
                switch_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::SwitchPreLayoutResult {
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
fn test_switch_tab_moves_focus_next() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_tab_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_switches(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_switch_right_arrow_moves_focus_next() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_switches(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_switch_down_arrow_moves_focus_next() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_switches(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_switch_shift_tab_moves_focus_prev() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();
    let focus1 = state1.focus_id;
    let focus2 = state2.focus_id;

    crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
        &mut state1,
        focus1,
        &mut state2,
        focus2,
        |state1, state2, input, focus_system, cmds| {
            draw_two_switches(focus_system, state1, state2, input, cmds);
        },
    );
}

#[test]
fn test_switch_visual_off() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec,
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut SwitchState::default(),
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
                color: s.off_fill,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 13.0, 10.0, 10.0),
                color: s.off_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_hovered() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = SwitchState::default();

    // Warmup frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 13.0, 10.0, 10.0),
                color: s.off_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_pressed() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let mut input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = SwitchState::default();

    // Warmup frame with mouse inside but not pressed
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 13.0, 10.0, 10.0),
                color: s.off_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_on() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec,
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut SwitchState {
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.on_fill,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 13.0, 10.0, 10.0),
                color: s.on_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_on_hovered() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut state = SwitchState {
        checked: true,
        ..Default::default()
    };

    // Warmup frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
    raw::post_layout_switch(
        switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
        raw::SwitchPreLayoutResult {
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
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 13.0, 10.0, 10.0),
                color: s.on_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_focused() {
    let state = SwitchState::default();
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));
    let s = spec.style;
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let mut state = state;
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec,
        raw::SwitchPreLayoutResult {
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
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r.inset(-(s.focus.unwrap().offset + s.focus.unwrap().stroke.width)),
                color: s.focus.unwrap().stroke.color,
                width: s.focus.unwrap().stroke.width,
                z: 1,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: r,
                color: s.off_fill,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 13.0, 10.0, 10.0),
                color: s.off_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_visual_disabled() {
    let spec = SwitchSpec {
        disabled: true,
        ..switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0))
    };
    let s = spec.style;
    let alpha = s.disabled_alpha;
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
    let r = Rect::new(10.0, 10.0, 30.0, 16.0);
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec,
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut SwitchState::default(),
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
                color: tint(s.off_fill),
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r,
                color: tint(s.border.unwrap().color),
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 13.0, 10.0, 10.0),
                color: tint(s.off_thumb),
                z: 0,
            },
        ])
    );
}

#[test]
fn test_switch_click_triggers_clicked_state() {
    let mut state = SwitchState::default();

    crate::widgets::test_helpers::assert_mouse_click_on_release(
        &mut state,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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
fn test_switch_click_takes_focus() {
    let mut state = SwitchState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_mouse_press_takes_focus(
        &mut state,
        focus_id,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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
fn test_switch_clipped_click_does_not_take_focus() {
    let mut state = SwitchState::default();

    crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
        &mut state,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                SwitchSpec {
                    clip_rect: Some(Rect::new(500.0, 500.0, 30.0, 16.0)),
                    ..switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0))
                },
                raw::SwitchPreLayoutResult {
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
fn test_switch_disabled_ignores_interaction() {
    let mut state = SwitchState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
        &mut state,
        focus_id,
        Vec2::new(15.0, 15.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                SwitchSpec {
                    disabled: true,
                    ..switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0))
                },
                raw::SwitchPreLayoutResult {
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
fn test_enter_toggles_raw_switch() {
    let mut focus_system = FocusSystem::new();
    let mut state = SwitchState::default();
    let mut input = Input::default();

    let spec = || switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0));

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec(),
        raw::SwitchPreLayoutResult {
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
    let result = raw::post_layout_switch(
        spec(),
        raw::SwitchPreLayoutResult {
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
        "Switch should be clicked by Enter key"
    );
    assert!(state.checked, "Enter key must toggle switch state");
}

#[test]
fn test_switch_hover_and_press_state() {
    let mut state = SwitchState::default();

    crate::widgets::test_helpers::assert_hover_and_press_state(
        &mut state,
        Vec2::new(15.0, 15.0),
        Vec2::new(150.0, 150.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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
fn test_switch_drag_off_and_release_does_not_click_other_switch() {
    let mut state1 = SwitchState::default();
    let mut state2 = SwitchState::default();

    crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
        &mut state1,
        &mut state2,
        Vec2::new(15.0, 15.0),
        Vec2::new(15.0, 115.0),
        false,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 110.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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
        "Dragging onto another switch must not toggle it on release"
    );
}

#[test]
fn test_switch_spacebar_click() {
    let mut state = SwitchState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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

    assert!(state.checked, "Spacebar release must toggle switch state");
}

#[test]
fn test_switch_spacebar_loses_focus_does_not_click() {
    let mut state = SwitchState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_switch(
                switch_spec(Rect::new(10.0, 10.0, 30.0, 16.0)),
                raw::SwitchPreLayoutResult {
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
        "Losing focus before Space release must not toggle switch state"
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = SwitchSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(SwitchStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = SwitchStyle::from_theme(&theme);
    custom_style.thumb_size = 99.0;
    let builder = SwitchSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().thumb_size, 99.0);
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
    let mut sw_state = SwitchState::default();
    let result = super::switch(&mut ctx, SwitchSpecBuilder::new(), placement, &mut sw_state);
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
    let custom = SwitchStyle {
        off_fill: Color::from_srgb_u8(1, 2, 3, 255),
        ..SwitchStyle::from_theme(&crate::theme::Theme::default())
    };
    let mut sw_state = SwitchState::default();
    super::switch(
        &mut ctx,
        SwitchSpecBuilder::new().style(custom),
        Rect::new(100.0, 100.0, 30.0, 16.0),
        &mut sw_state,
    );

    let has_custom_fill = cmds
        .iter()
        .any(|c| matches!(c, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == custom.off_fill));
    assert!(
        has_custom_fill,
        "high-level switch must honor user-set style"
    );
}

#[test]
fn test_size_switch() {
    let theme = crate::theme::Theme::default();
    let style = SwitchStyle::from_theme(&theme);
    let spec = raw::SwitchPreLayoutSpec { style };
    let size_request = raw::pre_layout_switch(&spec, SizeOffer::UNBOUNDED).size_request;
    assert_eq!(size_request, SizeRequest::preferred(Vec2::new(30.0, 16.0)));
}

#[test]
fn test_switch_visual_vertically_centered() {
    let spec = switch_spec(Rect::new(10.0, 10.0, 30.0, 20.0));
    let s = spec.style;
    let mut cmds = DrawCommands::new();
    raw::post_layout_switch(
        spec,
        raw::SwitchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut SwitchState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut cmds,
    );
    // Expect Y to be 10.0 + (20.0 - 16.0) * 0.5 = 12.0
    let expected_rect = Rect::new(10.0, 12.0, 30.0, 16.0);
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: expected_rect,
                color: s.off_fill,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: expected_rect,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(12.0, 15.0, 10.0, 10.0), // 12.0 + (16.0 - 10.0) * 0.5 = 15.0
                color: s.off_thumb,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_labelled_switch_request_size() {
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

    let mut state = SwitchState::default();
    // TestTextBackend logical size reports 8.0 per character. "vsync" is 5 chars -> 40.0.
    // Height is 16.0. Switch size is 30.0 x 16.0. Gap is 8.0.
    // Combined width: 30.0 + 8.0 + 40.0 = 78.0.
    // Combined height: max(16.0, 16.0) = 16.0.
    let result = super::labelled_switch(
        &mut ctx,
        SwitchSpecBuilder::new(),
        "vsync",
        Rect::new(0.0, 0.0, 100.0, 20.0),
        &mut state,
    );

    assert_eq!(result.layout.bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
}

#[test]
fn test_labelled_switch_click_label_toggles_state() {
    use crate::layouts::ManualLayout;
    let mut state = SwitchState::default();

    crate::widgets::test_helpers::assert_labelled_widget_click_toggles(
        &mut state,
        Vec2::new(50.0, 10.0),
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
            super::labelled_switch(
                &mut ctx,
                SwitchSpecBuilder::new(),
                "vsync",
                Rect::new(0.0, 0.0, 100.0, 20.0),
                state,
            );
        },
    );

    assert!(state.checked);
}

#[test]
fn test_labelled_switch_disabled_label_visual() {
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

        let mut state = SwitchState::default();
        super::labelled_switch(
            &mut ctx,
            SwitchSpecBuilder::new().disabled(true),
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
