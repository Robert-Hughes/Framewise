use super::raw::SliderSpec;
use super::*;

#[test]
fn test_slider_overlapping_hover() {
    let mut state1 = SliderState::default();
    let mut state2 = SliderState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let mut spec1 = test_spec(0.0, 100.0, false);
            spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
            let mut spec2 = test_spec(0.0, 100.0, false);
            spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

            let res1 = raw::post_layout_slider(
                spec1,
                raw::SliderPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_slider(
                spec2,
                raw::SliderPreLayoutResult {
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
fn test_slider_overlapping_click() {
    let mut state1 = SliderState::default();
    let mut state2 = SliderState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        false,
        |state1, state2, input, focus_system, cmds| {
            let mut spec1 = test_spec(0.0, 100.0, false);
            spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
            let mut spec2 = test_spec(0.0, 100.0, false);
            spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

            let res1 = raw::post_layout_slider(
                spec1,
                raw::SliderPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                cmds,
            );
            let res2 = raw::post_layout_slider(
                spec2,
                raw::SliderPreLayoutResult {
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
fn test_slider_page_up_down_keyboard() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let spec = test_spec(0.0, 100.0, true);

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Must be focused to receive keyboard events
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: register_keyboard claims
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Page Up
    focus_system.begin_frame();
    input.key_pressed_page_up = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 30.0);
    focus_system.end_frame();

    // Frame 3: Page Down
    focus_system.begin_frame();
    input.key_pressed_page_up = false;
    input.key_pressed_page_down = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 50.0);
    focus_system.end_frame();

    input.key_pressed_page_down = false;
    input.key_pressed_home = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 0.0);

    input.key_pressed_home = false;
    input.key_pressed_end = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 100.0);
}

#[test]
fn test_slider_drag() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.thumb.cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        thumb_len: ThumbLen::Fixed(20.0),
        style,
        ..test_spec(0.0, 100.0, true)
    };
    // Thumb is 20px high. Usable track = 100 - 20 = 80px.
    // So moving 40px down should increase value by 50.

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Warmup frame to establish hover claim
    input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // 1. Click on thumb (thumb is at y=0 to y=20)
    input.mouse_pressed = true;
    input.mouse_down = true;

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(state.is_dragging);
    assert_eq!(state.drag_start_mouse_coord, 10.0);

    // 2. Drag down by 40px (mouse y = 50)
    input.mouse_pressed = false;
    input.mouse_pos.y = 50.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // 40 / 80 usable track = 0.5 ratio = 50 value
    assert_eq!(state.value, 50.0);
}

#[test]
fn test_slider_track_click_hold() {
    let mut state = SliderState::default();
    let spec = test_spec(0.0, 100.0, true);
    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Warmup frame to establish hover claim
    input.mouse_pos = crate::types::Vec2::new(10.0, 80.0);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // 1. Initial click at bottom of track (y=80)
    input.mouse_pressed = true;
    input.mouse_down = true;

    // Frame 1: time=0.0. Should page down by 20.0
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 20.0);
    assert!(state.is_track_clicking);
    assert_eq!(state.next_repeat_time, 0.5); // wait 500ms

    // Frame 2: time=0.4 (before repeat). No change.
    input.mouse_pressed = false;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.4,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 20.0);

    // Frame 3: time=0.5 (trigger repeat). Should page down to 40.0
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.5,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 40.0);
    assert_eq!(state.next_repeat_time, 0.55); // next in 50ms

    // Frame 4: time=0.6 (trigger repeat again). Should page down to 60.0
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.6,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 60.0);

    // Release mouse -> track clicking ends
    input.mouse_down = false;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.7,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(!state.is_track_clicking);
}

#[test]
fn test_slider_arrow_keys() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let spec = test_spec(0.0, 100.0, true);

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    focus_system.take_keyboard_focus(state.focus_id);

    // Up decrements
    input.key_pressed_up = true;
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 45.0);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Up arrow must not move focus away from slider"
    );

    // Down increments
    input.key_pressed_up = false;
    input.key_pressed_down = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 50.0);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Down arrow must not move focus away from slider"
    );

    // Left decrements (same as Up)
    input.key_pressed_down = false;
    input.key_pressed_left = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 45.0);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Left arrow must not move focus away from slider"
    );

    // Right increments (same as Down)
    input.key_pressed_left = false;
    input.key_pressed_right = true;
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(state.value, 50.0);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Right arrow must not move focus away from slider"
    );

    // Left/Right also work on a horizontal slider
    input.key_pressed_right = false;
    let horiz_spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        ..spec.clone()
    };
    let mut horiz_state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(horiz_state.focus_id);

    input.key_pressed_left = true;
    raw::post_layout_slider(
        horiz_spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut horiz_state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(horiz_state.value, 45.0);

    input.key_pressed_left = false;
    input.key_pressed_right = true;
    raw::post_layout_slider(
        horiz_spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut horiz_state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(horiz_state.value, 50.0);
}

#[test]
fn test_slider_tab_moves_focus_not_arrows() {
    let mut state_a = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut state_b = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    focus_system.take_keyboard_focus(state_a.focus_id);

    // Frame 1: Tab on focused slider_a — should shift focus to slider_b
    focus_system.begin_frame();
    let input = crate::input::Input {
        key_pressed_tab: true,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state_a,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state_b,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: confirm focus moved to slider_b
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state_a,
        &crate::input::Input::new(),
        &mut focus_system,
        &mut cmds,
    );
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state_b,
        &crate::input::Input::new(),
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state_b.focus_id),
        "Tab should move focus from slider_a to slider_b"
    );
    assert_eq!(state_a.value, 50.0, "Value must not change on Tab");
}

#[test]
fn test_slider_click_takes_focus() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    // Click on the track
    let mut input = crate::input::Input::new();
    input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);

    // Warmup frame to establish hover claim
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame with mouse pressed
    input.mouse_pressed = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Clicking slider must request focus"
    );
}

#[test]
fn test_slider_clipped_click_does_not_take_focus() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();

    // Mouse is inside the widget rect but outside the clip_rect.
    let mut spec = test_spec(0.0, 100.0, true);
    spec.clip_rect = Some(Rect::new(500.0, 500.0, 20.0, 100.0));

    let mut input = crate::input::Input::new();
    input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
    input.mouse_pressed = true;

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        focus_system.current_keyboard_focus(),
        None,
        "Clicking a clipped-away slider must not take focus"
    );
}

#[test]
fn test_slider_mouse_wheel() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let spec = test_spec(0.0, 100.0, true);

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Hover over the slider track
    input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);

    // Frame 1: Register hover
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 50.0); // Hasn't scrolled yet, scroll_delta is 0

    // Frame 2: Mouse wheel spun up (positive delta) -> value should decrease
    input.scroll_delta.y = 2.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // value = 50.0 - 2.0 * 5.0 = 40.0
    assert_eq!(state.value, 40.0);
}

/// Track: y=0..100, thumb_len=20, usable=80, value=0 → thumb at y=0..20.
/// Click empty track at y=50 → page step to 20.0, is_track_clicking.
/// Move mouse by 5px (> 4px threshold) to y=55 → snaps:
///   thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25, switches to drag.
/// Then drag to y=65 → delta=10 → val_delta=12.5 → value=68.75.
#[test]
fn test_track_click_snaps_and_drags() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.thumb.cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        thumb_len: ThumbLen::Fixed(20.0),
        style,
        ..test_spec(0.0, 100.0, true)
    };
    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Warmup frame to establish hover claim
    input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: click empty track at y=50 (thumb is at y=0..20) → page step
    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        state.is_track_clicking,
        "should be track-clicking after initial track click"
    );
    assert!(!state.is_dragging, "should not yet be dragging");
    assert_eq!(state.value, 20.0, "page step should fire on click");

    // Frame 2: move mouse 5px (> 4px threshold) while holding → transitions to drag+snap
    input.mouse_pressed = false;
    input.mouse_pos.y = 55.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        state.is_dragging,
        "should switch to dragging after threshold exceeded"
    );
    assert!(!state.is_track_clicking, "track clicking should end");
    // snap: thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25
    assert!(
        (state.value - 56.25).abs() < 0.01,
        "snap to 56.25, got {}",
        state.value
    );

    // Frame 3: drag to y=65 → delta=10 → val_delta=12.5 → value=68.75
    input.mouse_pos.y = 65.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        (state.value - 68.75).abs() < 0.01,
        "drag to 68.75, got {}",
        state.value
    );
}

// Regression: paging past the cursor causes direction-flip flicker.
// Setup: track y=0..100, thumb_len=20, usable=80, page_step=60.
// value=0 → thumb at y=0..20. Click at y=70 (below thumb).
// Frame 1 (initial click): page to 60 → thumb at y=48..68.
// Frame 2 (repeat at t=0.5): cursor y=70 > thumb_end=68, fires.
//   Buggy: 60+60=120 → clamped to 100 → thumb at 80..100 → cursor < thumb_start → flicker.
//   Fixed: clamp to cursor position (87.5) so thumb stops at cursor.
// Frame 3 (repeat at t=0.6): cursor inside thumb → paging stops.
#[test]
fn test_track_click_repeat_does_not_overshoot_cursor() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.thumb.cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        page_step: 60.0,
        thumb_len: ThumbLen::Fixed(20.0),
        style,
        ..test_spec(0.0, 100.0, true)
    };
    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Warmup frame to establish hover claim
    input.mouse_pos = crate::types::Vec2::new(10.0, 70.0);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: initial click at y=70 (well below thumb at y=0..20).
    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 60.0, "initial page: 0 + 60 = 60");
    assert!(state.is_track_clicking);
    assert_eq!(state.next_repeat_time, 0.5);

    // Frame 2: hold, before repeat fires.
    input.mouse_pressed = false;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.4,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 60.0);

    // Frame 3: repeat fires (t=0.5). Thumb at y=48..68, cursor at y=70 > 68 → fires.
    // Expected: value clamps to cursor position (87.5), NOT 100.
    // cursor_val = (70/80) * 100 = 87.5
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.5,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        (state.value - 87.5).abs() < 0.01,
        "repeat should stop at cursor position 87.5, got {}",
        state.value
    );

    // Frame 4: repeat fires again (t=0.6). Thumb now at y=70..90, cursor=70 inside → stop paging.
    // is_track_clicking must remain true so the drag transition can still fire.
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.6,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        (state.value - 87.5).abs() < 0.01,
        "value should not change after thumb reaches cursor, got {}",
        state.value
    );
    assert!(
        state.is_track_clicking,
        "is_track_clicking must stay true so drag is still possible"
    );
    assert!(!state.is_dragging);

    // Frame 5: still holding, move mouse 5px (past 4px threshold from initial click at y=70).
    // Drag transition should fire: thumb snaps to cursor, enters drag mode.
    // snap: mouse_coord=75, track_start=0, thumb_len=20 → snapped=75-10=65 → value=65/80*100=81.25
    input.mouse_pos.y = 75.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.65,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        state.is_dragging,
        "should enter drag mode after mouse moves past threshold"
    );
    assert!(!state.is_track_clicking);
    assert!(
        (state.value - 81.25).abs() < 0.01,
        "snap on drag entry: expected 81.25, got {}",
        state.value
    );

    // Frame 6: drag to y=85 → delta=10 → val_delta=12.5 → value=93.75
    input.mouse_pressed = false;
    input.mouse_pos.y = 85.0;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.7,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        (state.value - 93.75).abs() < 0.01,
        "drag: expected 93.75, got {}",
        state.value
    );
}

// Helper to build a standard test spec.
fn test_spec(min: f32, max: f32, claim_at_ends: bool) -> SliderSpec {
    SliderSpec {
        orientation: Orientation::Vertical,
        rect: Rect::new(0.0, 0.0, 20.0, 100.0),
        min,
        max,
        page_step: 20.0,
        step: 5.0,
        thumb_len: ThumbLen::Fixed(12.0),
        style: SliderStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
        scroll_claim: if claim_at_ends {
            ScrollClaimPolicy::ClaimAllDirections
        } else {
            ScrollClaimPolicy::YieldSameAxisAtEnds
        },
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    }
}

// ── Standalone slider ──────────────────────────────────────────────────────

#[test]
fn test_standalone_slider_wheel_at_min_blocks_propagation() {
    // Even when at minimum, a standalone slider claims both directions,
    // so a hypothetical parent scroll area would never see the event.
    let mut state = SliderState::default();
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let mut input = Input::new();
    input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
    input.scroll_delta.y = 1.0; // scroll up

    // Frame 1: slider registers first (inner), parent second (outer)
    focus_system.begin_frame();
    // Standalone slider registers first (inner)
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, true),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent registers after (outer, simulating parent's end())
    focus_system.claim_scroll_up(parent_id);
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    // Frame 2: parent checks — it should NOT have won either direction
    assert!(
        !focus_system.is_active_scroll_up(parent_id),
        "parent should not win scroll-up; standalone slider blocked it"
    );
    // Value stays at 0.0 (clamped, can't go below min)
    assert_eq!(state.value, 0.0);
}

#[test]
fn test_standalone_slider_wheel_at_max_blocks_propagation() {
    let mut state = SliderState {
        value: 100.0, // already at max
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(0.0, -1.0), // scroll down
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, true),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (simulating parent's end())
    focus_system.claim_scroll_up(parent_id);
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    assert!(
        !focus_system.is_active_scroll_down(parent_id),
        "parent should not win scroll-down; standalone slider blocked it"
    );
    assert_eq!(state.value, 100.0);
}

#[test]
fn test_vertical_standalone_slider_blocks_horizontal_scroll() {
    // Regression: vertical standalone slider inside a horizontal scroll area was
    // letting horizontal scroll events propagate because claim_scroll_at_ends only
    // claimed up/down, not left/right.
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(3.0, 0.0), // horizontal scroll only
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, true),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (simulating parent's end())
    focus_system.claim_scroll_left(parent_id);
    focus_system.claim_scroll_right(parent_id);
    focus_system.end_frame();

    assert!(
        !focus_system.is_active_scroll_left(parent_id),
        "parent should not win scroll-left; vertical standalone slider should block it"
    );
    assert!(
        !focus_system.is_active_scroll_right(parent_id),
        "parent should not win scroll-right; vertical standalone slider should block it"
    );
}

// ── Propagating slider (scrollbar-within-scroll-area mode) ─────────────────

#[test]
fn test_propagating_slider_at_min_yields_scroll_up_to_parent() {
    let mut state = SliderState::default();
    // value = 0.0 — at min, can't scroll up
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(0.0, 1.0), // scroll up
        ..Default::default()
    };

    // Frame 1: inner propagating slider first, then parent claims simulating parent's end()
    focus_system.begin_frame();
    // Inner propagating slider at min: skips claim_scroll_up
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, false),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (simulating parent's end())
    focus_system.claim_scroll_up(parent_id); // parent can scroll up
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    // Parent should have retained the scroll-up claim
    assert!(
        focus_system.is_active_scroll_up(parent_id),
        "parent should win scroll-up when inner is at its minimum"
    );
    assert_eq!(state.value, 0.0, "inner value unchanged");
}

#[test]
fn test_propagating_slider_at_max_yields_scroll_down_to_parent() {
    let mut state = SliderState {
        value: 100.0, // at max — can't scroll down
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(0.0, -1.0), // scroll down
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, false),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (simulating parent's end())
    focus_system.claim_scroll_up(parent_id);
    focus_system.claim_scroll_down(parent_id); // parent can scroll down
    focus_system.end_frame();

    assert!(
        focus_system.is_active_scroll_down(parent_id),
        "parent should win scroll-down when inner is at its maximum"
    );
    assert_eq!(state.value, 100.0, "inner value unchanged");
}

#[test]
fn test_propagating_slider_mid_range_wins_both_directions() {
    // When not at an end, the inner propagating slider claims both directions
    // and the parent gets neither.
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(0.0, 1.0),
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        test_spec(0.0, 100.0, false),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (simulating parent's end())
    focus_system.claim_scroll_up(parent_id);
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    assert!(
        !focus_system.is_active_scroll_up(parent_id),
        "parent should not win"
    );
    assert!(
        !focus_system.is_active_scroll_down(parent_id),
        "parent should not win"
    );
}

// ── Disabled ─────────────────────────────────────────────────────────────

fn disabled_spec(scrollbar_mode: bool) -> SliderSpec {
    let theme = crate::theme::Theme::framewise();
    let style = if scrollbar_mode {
        SliderStyle::scrollbar_from_theme(&theme)
    } else {
        SliderStyle::from_theme(&theme)
    };
    SliderSpec {
        disabled: true,
        style,
        ..test_spec(0.0, 100.0, true)
    }
}

/// A disabled slider ignores mouse press, drag, wheel, and keyboard, and
/// never takes focus (it isn't registered in the focus order).
#[test]
fn test_disabled_slider_ignores_all_input() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let spec = disabled_spec(false);
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    // Press on the thumb (thumb is centered around value=50).
    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        mouse_pressed: true,
        mouse_down: true,
        scroll_delta: Vec2::new(0.0, 5.0),
        key_pressed_page_down: true,
        key_pressed_end: true,
        ..Default::default()
    };

    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 50.0, "disabled slider must not change value");
    assert!(!state.is_dragging, "disabled slider must not start a drag");
    assert!(!state.is_track_clicking);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        None,
        "disabled slider must not take focus"
    );
}

/// A disabled slider does not claim scroll, so a parent scroll area still
/// wins the wheel even when the cursor is over the (degenerate) bar.
#[test]
fn test_disabled_slider_does_not_block_parent_scroll() {
    let mut state = SliderState::default();
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        scroll_delta: Vec2::new(0.0, 1.0),
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        disabled_spec(true),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    // Parent claims after (inner-first ordering).
    focus_system.claim_scroll_up(parent_id);
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    assert!(
        focus_system.is_active_scroll_up(parent_id),
        "disabled slider must let the parent win the wheel"
    );
}

/// A disabled slider still draws (track + thumb), tinted by disabled_alpha,
/// so it occupies its reserved track.
#[test]
fn test_disabled_slider_draws_tinted() {
    let mut state = SliderState::default();
    let spec = disabled_spec(true); // scrollbar mode: track fill + thumb fill
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_none(),
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let a = spec.style.disabled_alpha;
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * a);
    let theme = crate::theme::Theme::framewise();
    let track_color = Color::linear_rgba(theme.ink.r, theme.ink.g, theme.ink.b, 0.04);
    let border_color = theme.line_soft;
    let thumb_color = theme.ink;

    match (&cmds[0], &cmds[1], &cmds[2]) {
        (
            DrawCmd::FillRect {
                anti_alias: false,
                rect: tr,
                color: tc,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                color: bc,
                ..
            },
            DrawCmd::FillRect {
                anti_alias: false,
                color: hc,
                ..
            },
        ) => {
            assert_eq!(*tr, spec.rect, "track fill spans the full reserved rect");
            assert_eq!(*tc, tint(track_color));
            assert_eq!(*bc, tint(border_color));
            assert_eq!(*hc, tint(thumb_color));
        }
        other => panic!("unexpected draw commands: {:?}", other),
    }
    assert_eq!(
        cmds.len(),
        3,
        "scrollbar-mode disabled draws track + border + thumb"
    );
}

fn input_none() -> Input {
    Input::new()
}

// ── Visual Tests ───────────────────────────────────────────────────────────

#[test]
fn test_slider_visual_normal() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    let input = Input::new();
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _result = raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let theme = crate::theme::Theme::framewise();
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.5,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_hovered() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        ..Default::default()
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _result = raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let theme = crate::theme::Theme::framewise();
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.5,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_drag() {
    let mut state = SliderState {
        is_dragging: true,
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    let input = Input {
        mouse_down: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _result = raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let theme = crate::theme::Theme::framewise();
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.rust,
                width: 1.5,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_focused() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    focus_system.take_keyboard_focus(state.focus_id);

    let input = Input::new();
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _result = raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let theme = crate::theme::Theme::framewise();
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(-4.0, -4.0, 28.0, 108.0),
                color: theme.rust,
                width: 2.0,
                z: 1,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.5,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = SliderSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(SliderStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = SliderStyle::from_theme(&theme);
    custom_style.disabled_alpha = 0.99;
    let builder = SliderSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().disabled_alpha, 0.99);
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
    let mut state = SliderState::default();
    // Under ManualLayout the layout param *is* the rect — the sanctioned way
    // to place a high-level widget explicitly.
    super::slider(&mut ctx, SliderSpecBuilder::new(), placement, &mut state);
    // First draw command for a horizontal slider is the track-line FillRect,
    // whose x starts at the resolved track rect's x = placement.x.
    match &cmds[0] {
        crate::draw::DrawCmd::FillRect {
            anti_alias: false,
            rect,
            ..
        } => {
            assert_eq!(rect.x, placement.x);
        }
        other => panic!("Expected FillRect, got {:?}", other),
    }
}

#[test]
fn test_size_slider() {
    // A slider's size is caller-driven; it reports no size request.
    let spec = raw::SliderPreLayoutSpec {};
    assert_eq!(
        raw::pre_layout_slider(&spec, SizeOffer::UNBOUNDED).size_request,
        crate::layout::SizeRequest::UNKNOWN
    );
}

#[test]
fn test_track_click_overshoot_first_page_no_jump_back() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.thumb.cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        page_step: 60.0,
        thumb_len: ThumbLen::Fixed(20.0),
        style,
        ..test_spec(0.0, 100.0, true) // track y=0..100, usable_track=80
    };
    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Warmup frame
    input.mouse_pos = crate::types::Vec2::new(10.0, 25.0);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: Click at y=25 (right next to the initial thumb at y=0..20)
    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Moving one page allows overshoot (value goes to 60.0, thumb at y=48..68)
    assert_eq!(state.value, 60.0);
    assert!(state.is_track_clicking);

    // Frame 2: Hold, before repeat fires (t=0.4)
    input.mouse_pressed = false;
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.4,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 60.0);

    // Frame 3: Repeat fires (t=0.5).
    // Since we overshot, the cursor y=25 is now behind the thumb.
    // It must NOT jump back or trigger overshoot protection. Value must stay 60.0.
    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.5,
            ..spec.clone()
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 60.0, "should not jump back on itself");
    assert!(state.is_track_clicking);
}

#[test]
fn test_non_keyboard_focusable_slider() {
    let mut state = SliderState::default();
    let mut spec = test_spec(0.0, 100.0, true);
    spec.keyboard_focusable = false;

    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();
    let mut input = Input::new();

    // 1. Hovering & Scroll Wheel Claim
    input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);

    // Frame 1: Register hovers/scrolls
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Check active hovers/scrolls (they are resolved on end_frame/begin_frame transition)
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );

    // Assert: not registered in keyboard focus order
    assert_eq!(focus_system.current_keyboard_focus(), None);

    // Assert: claims hover and scroll up/down
    assert!(focus_system.is_hover_active(state.focus_id));
    assert!(focus_system.is_active_scroll_up(state.focus_id));
    assert!(focus_system.is_active_scroll_down(state.focus_id));
    focus_system.end_frame();

    // 2. Click does NOT take keyboard focus
    input.mouse_pressed = true;
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(focus_system.current_keyboard_focus(), None);
}

#[test]
fn test_scrollbar_visual_normal() {
    let mut state = SliderState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let theme = crate::theme::Theme::framewise();
    let style = SliderStyle::scrollbar_from_theme(&theme);
    let spec = SliderSpec {
        orientation: Orientation::Vertical,
        rect: Rect::new(0.0, 0.0, 20.0, 100.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        thumb_len: ThumbLen::Fixed(24.0),
        style,
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::YieldSameAxisAtEnds,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input::new();
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let track_color = Color::linear_rgba(theme.ink.r, theme.ink.g, theme.ink.b, 0.04);
    let border_color = theme.line_soft;
    let thumb_color = theme.ink;

    // Scrollbar-style: track rect fill, separator line, fill-track thumb with margin, no thumb border
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 100.0),
                color: track_color,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(0.0, 0.0),
                p1: Vec2::new(0.0, 100.0),
                color: border_color,
                width: 1.0,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 38.0, 18.0, 24.0), // Margin=1.0 on x (cross axis), y = 50% of 76 usable = 38.0
                color: thumb_color,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_track_line_invisible_stroke() {
    let mut state = SliderState::default();
    let mut spec = test_spec(0.0, 100.0, false);

    // Set track stroke to an invisible stroke (e.g. width = 0.0)
    if let TrackStyle::Line { stroke, .. } = &mut spec.style.track {
        stroke.width = 0.0;
    } else {
        panic!("expected default TrackStyle::Line");
    }

    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_none(),
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Since stroke width is 0.0, it is not visible.
    // The match in slider.rs for TrackStyle::Line should skip track and fill_before_thumb rects.
    // Let's assert that there is no FillRect with width 1.5 (which is the default thickness).
    for cmd in cmds.commands() {
        if let DrawCmd::FillRect { rect, .. } = cmd {
            if rect.w == 1.5 || rect.h == 1.5 {
                panic!("found track line or fill bar DrawCmd::FillRect when track stroke is invisible: {:?}", rect);
            }
        }
    }
}
