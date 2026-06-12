use crate::{
    draw::DrawCommands,
    focus::{FocusId, FocusSystem},
    input::Input,
    types::Vec2,
    widget::InputInfo,
};

pub fn assert_hover_and_press_state<State>(
    state: &mut State,
    inside_pos: Vec2,
    outside_pos: Vec2,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: outside_pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.hovered, "Widget should not be hovered");
    assert!(!result.pressed, "Widget should not be pressed");

    input.mouse_pos = inside_pos;
    // Warmup frame to establish the hover claim
    let _ = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    // Evaluation frame
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(result.hovered, "Widget should be hovered");
    assert!(!result.pressed, "Widget should not be pressed");

    input.mouse_down = true;
    input.mouse_pressed = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(
        result.hovered,
        "Widget should be hovered while pressed down"
    );
    assert!(result.pressed, "Widget should be pressed");

    input.mouse_pos = outside_pos;
    input.mouse_pressed = false;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.hovered, "Widget should lose hover when dragged out");
    assert!(
        !result.pressed,
        "Widget should lose pressed state when dragged out"
    );
}

pub fn assert_drag_off_and_release_does_not_click_other<StateA, StateB>(
    state_a: &mut StateA,
    state_b: &mut StateB,
    start_pos: Vec2,
    other_pos: Vec2,
    expect_other_hovered_while_dragging: bool,
    mut run: impl FnMut(
        &mut StateA,
        &mut StateB,
        &Input,
        &mut FocusSystem,
        &mut DrawCommands,
    ) -> (InputInfo, InputInfo),
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: start_pos,
        mouse_down: false,
        mouse_pressed: false,
        mouse_clicked: false,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    // Warmup frame to establish the hover claim on source widget
    let _ = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );

    // Now press down
    input.mouse_down = true;
    input.mouse_pressed = true;
    let (source, _) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(source.pressed, "Source widget should be pressed");

    input.mouse_pressed = false;
    input.mouse_pos = other_pos;
    // Warmup frame at other_pos to establish the hover claim on other widget
    let _ = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    // Evaluation frame
    let (_, other) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !other.pressed,
        "Other widget should not be pressed while dragging over it"
    );
    assert_eq!(
        other.hovered, expect_other_hovered_while_dragging,
        "Other widget hover state while dragging over it did not match expectation"
    );

    input.mouse_down = false;
    input.mouse_clicked = true;
    let (source, other) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !other.clicked,
        "Other widget should not click if mouse down started elsewhere"
    );
    assert!(
        !source.clicked,
        "Source widget should not click when mouse is released outside"
    );
}

pub fn assert_overlapping_hover<StateA, StateB>(
    state_a: &mut StateA,
    state_b: &mut StateB,
    overlap_pos: Vec2,
    mut run: impl FnMut(
        &mut StateA,
        &mut StateB,
        &Input,
        &mut FocusSystem,
        &mut DrawCommands,
    ) -> (InputInfo, InputInfo),
) {
    let mut focus_system = FocusSystem::new();
    let input = Input {
        mouse_pos: overlap_pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    // Warmup frame to establish the hover claim
    let _ = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );

    let (bottom_result, top_result) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !bottom_result.hovered,
        "Bottom widget should not be hovered when overlapped"
    );
    assert!(
        top_result.hovered,
        "Top widget should be hovered when overlapped"
    );
}

pub fn assert_overlapping_click<StateA, StateB>(
    state_a: &mut StateA,
    state_b: &mut StateB,
    overlap_pos: Vec2,
    expect_click: bool,
    mut run: impl FnMut(
        &mut StateA,
        &mut StateB,
        &Input,
        &mut FocusSystem,
        &mut DrawCommands,
    ) -> (InputInfo, InputInfo),
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: overlap_pos,
        mouse_down: false,
        mouse_pressed: false,
        mouse_clicked: false,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    // Warmup frame to establish the hover claim
    let _ = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );

    input.mouse_down = true;
    input.mouse_pressed = true;
    let (bottom_result, top_result) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !bottom_result.pressed,
        "Bottom widget should not be pressed when overlapped click starts"
    );
    assert!(
        top_result.pressed,
        "Top widget should be pressed when overlapped click starts"
    );

    input.mouse_down = false;
    input.mouse_pressed = false;
    input.mouse_clicked = true;
    let (bottom_result, top_result) = run_two_widget_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !bottom_result.clicked,
        "Bottom widget should not be clicked when overlapped"
    );
    assert_eq!(
        top_result.clicked, expect_click,
        "Top widget clicked state did not match expectation"
    );
}

pub fn assert_tab_moves_focus_next<StateA, StateB>(
    state_a: &mut StateA,
    focus_a: FocusId,
    state_b: &mut StateB,
    focus_b: FocusId,
    run: impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    assert_focus_moves(
        state_a,
        focus_a,
        state_b,
        focus_b,
        |input| input.key_pressed_tab = true,
        run,
    );
}

pub fn assert_right_arrow_moves_focus_next<StateA, StateB>(
    state_a: &mut StateA,
    focus_a: FocusId,
    state_b: &mut StateB,
    focus_b: FocusId,
    run: impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    assert_focus_moves(
        state_a,
        focus_a,
        state_b,
        focus_b,
        |input| input.key_pressed_right = true,
        run,
    );
}

pub fn assert_down_arrow_moves_focus_next<StateA, StateB>(
    state_a: &mut StateA,
    focus_a: FocusId,
    state_b: &mut StateB,
    focus_b: FocusId,
    run: impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    assert_focus_moves(
        state_a,
        focus_a,
        state_b,
        focus_b,
        |input| input.key_pressed_down = true,
        run,
    );
}

pub fn assert_shift_tab_moves_focus_prev<StateA, StateB>(
    state_a: &mut StateA,
    focus_a: FocusId,
    state_b: &mut StateB,
    focus_b: FocusId,
    run: impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    assert_focus_moves(
        state_a,
        focus_b,
        state_b,
        focus_a,
        |input| {
            input.key_pressed_tab = true;
            input.modifier_shift = true;
        },
        run,
    );
}

pub fn assert_mouse_click_on_release<State>(
    state: &mut State,
    inside_pos: Vec2,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: inside_pos,
        mouse_down: false,
        mouse_pressed: false,
        mouse_clicked: false,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    // Warmup frame to establish the hover claim
    let _ = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);

    // Now press down
    input.mouse_down = true;
    input.mouse_pressed = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(result.pressed, "Widget should be pressed after mouse down");
    assert!(
        !result.clicked,
        "Widget should not click until mouse release"
    );

    input.mouse_down = false;
    input.mouse_pressed = false;
    input.mouse_clicked = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(result.clicked, "Widget should click on mouse release");
}

pub fn assert_mouse_press_takes_focus<State>(
    state: &mut State,
    focus_id: FocusId,
    inside_pos: Vec2,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: inside_pos,
        mouse_down: false,
        mouse_pressed: false,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    // Warmup frame to establish the hover claim
    let _ = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);

    // Now press down
    input.mouse_down = true;
    input.mouse_pressed = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(result.pressed, "Widget should be pressed after mouse down");
    assert_eq!(
        focus_system.current_focus(),
        Some(focus_id),
        "Mouse press inside widget should request focus"
    );
}

pub fn assert_clipped_mouse_press_does_not_take_focus<State>(
    state: &mut State,
    inside_pos: Vec2,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let input = Input {
        mouse_pos: inside_pos,
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();

    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.hovered, "Clipped widget should not be hovered");
    assert!(!result.pressed, "Clipped widget should not be pressed");
    assert!(!result.clicked, "Clipped widget should not be clicked");
    assert_eq!(
        focus_system.current_focus(),
        None,
        "Mouse press on clipped-away widget should not request focus"
    );
}

pub fn assert_disabled_ignores_press_interaction<State>(
    state: &mut State,
    focus_id: FocusId,
    inside_pos: Vec2,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    let mouse_press = Input {
        mouse_pos: inside_pos,
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    let result = run_frame(state, &mouse_press, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.hovered, "Disabled widget should not be hovered");
    assert!(!result.pressed, "Disabled widget should not be pressed");
    assert!(
        !result.clicked,
        "Disabled widget should not click on mouse press"
    );
    assert_eq!(
        focus_system.current_focus(),
        None,
        "Disabled widget should not take focus on mouse press"
    );

    let mouse_release = Input {
        mouse_pos: inside_pos,
        mouse_clicked: true,
        ..Default::default()
    };
    let result = run_frame(
        state,
        &mouse_release,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    assert!(
        !result.clicked,
        "Disabled widget should not click on mouse release"
    );

    focus_system.take_focus(focus_id);
    let enter = Input {
        key_pressed_enter: true,
        ..Default::default()
    };
    let result = run_frame(state, &enter, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.clicked, "Disabled widget should ignore Enter");

    let space_down = Input {
        key_down_space: true,
        key_pressed_space: true,
        ..Default::default()
    };
    let result = run_frame(state, &space_down, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.pressed, "Disabled widget should ignore Space press");
    assert!(!result.clicked, "Disabled widget should ignore Space press");

    let space_up = Input {
        key_released_space: true,
        ..Default::default()
    };
    let result = run_frame(state, &space_up, &mut focus_system, &mut cmds, &mut run);
    assert!(
        !result.clicked,
        "Disabled widget should ignore Space release"
    );
}

pub fn assert_spacebar_click<State>(
    state: &mut State,
    focus_id: FocusId,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    let mut cmds = DrawCommands::new();

    focus_system.take_focus(focus_id);
    let _ = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);

    input.key_down_space = true;
    input.key_pressed_space = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(
        result.pressed,
        "Widget should be visually pressed while space is down"
    );
    assert!(!result.clicked, "Widget should not be clicked yet");

    input.key_pressed_space = false;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(result.pressed, "Widget should remain pressed");
    assert!(!result.clicked, "Widget should not be clicked yet");

    input.key_down_space = false;
    input.key_released_space = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(!result.pressed, "Widget should not be pressed");
    assert!(result.clicked, "Widget should be clicked on release");
}

pub fn assert_spacebar_loses_focus_does_not_click<State>(
    state: &mut State,
    focus_id: FocusId,
    mut run: impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    let mut cmds = DrawCommands::new();

    focus_system.take_focus(focus_id);
    let _ = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);

    input.key_down_space = true;
    input.key_pressed_space = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(
        result.pressed,
        "Widget should be visually pressed while space is down"
    );

    input.key_pressed_space = false;
    focus_system.take_focus(FocusId::new());
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(
        !result.pressed,
        "Widget should lose pressed state when focus is lost"
    );

    input.key_down_space = false;
    input.key_released_space = true;
    let result = run_frame(state, &input, &mut focus_system, &mut cmds, &mut run);
    assert!(
        !result.clicked,
        "Widget should not click after losing focus"
    );
}

fn run_frame<State>(
    state: &mut State,
    input: &Input,
    focus_system: &mut FocusSystem,
    cmds: &mut DrawCommands,
    run: &mut impl FnMut(&mut State, &Input, &mut FocusSystem, &mut DrawCommands) -> InputInfo,
) -> InputInfo {
    focus_system.begin_frame();
    let result = run(state, input, focus_system, cmds);
    focus_system.end_frame();
    result
}

fn assert_focus_moves<StateA, StateB>(
    state_a: &mut StateA,
    initial_focus: FocusId,
    state_b: &mut StateB,
    expected_focus: FocusId,
    configure_input: impl FnOnce(&mut Input),
    mut run: impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    configure_input(&mut input);
    let mut cmds = DrawCommands::new();
    focus_system.take_focus(initial_focus);

    run_focus_frame(
        state_a,
        state_b,
        &input,
        &mut focus_system,
        &mut cmds,
        &mut run,
    );
    run_focus_frame(
        state_a,
        state_b,
        &Input::default(),
        &mut focus_system,
        &mut cmds,
        &mut run,
    );

    assert_eq!(focus_system.current_focus(), Some(expected_focus));
}

fn run_focus_frame<StateA, StateB>(
    state_a: &mut StateA,
    state_b: &mut StateB,
    input: &Input,
    focus_system: &mut FocusSystem,
    cmds: &mut DrawCommands,
    run: &mut impl FnMut(&mut StateA, &mut StateB, &Input, &mut FocusSystem, &mut DrawCommands),
) {
    focus_system.begin_frame();
    run(state_a, state_b, input, focus_system, cmds);
    focus_system.end_frame();
}

fn run_two_widget_frame<StateA, StateB>(
    state_a: &mut StateA,
    state_b: &mut StateB,
    input: &Input,
    focus_system: &mut FocusSystem,
    cmds: &mut DrawCommands,
    run: &mut impl FnMut(
        &mut StateA,
        &mut StateB,
        &Input,
        &mut FocusSystem,
        &mut DrawCommands,
    ) -> (InputInfo, InputInfo),
) -> (InputInfo, InputInfo) {
    focus_system.begin_frame();
    let result = run(state_a, state_b, input, focus_system, cmds);
    focus_system.end_frame();
    result
}
