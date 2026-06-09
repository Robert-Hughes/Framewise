use crate::{
    draw::DrawCommands, focus::FocusId, focus::FocusSystem, input::Input, widget::InputInfo,
};

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
