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
