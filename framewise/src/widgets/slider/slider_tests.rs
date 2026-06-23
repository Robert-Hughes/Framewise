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
        value: SliderValue::Single(50.0),
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
    assert_eq!(state.value.lower(), 30.0);
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
    assert_eq!(state.value.lower(), 50.0);
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
    assert_eq!(state.value.lower(), 0.0);

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
    assert_eq!(state.value.lower(), 100.0);
}

#[test]
fn test_slider_drag() {
    let mut state = SliderState {
        ..Default::default()
    };
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        min_gap: None,
        max_gap: None,
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
    assert!(state.active_part.is_some());
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

    // Endpoints map directly to the track, so 40px of drag is 50 value units.
    assert_eq!(state.value.lower(), 50.0);
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
    assert_eq!(state.value.lower(), 20.0);
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
    assert_eq!(state.value.lower(), 20.0);

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
    assert_eq!(state.value.lower(), 40.0);
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
    assert_eq!(state.value.lower(), 60.0);

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
        value: SliderValue::Single(50.0),
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
    assert_eq!(state.value.lower(), 45.0);
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
    assert_eq!(state.value.lower(), 50.0);
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
    assert_eq!(state.value.lower(), 45.0);
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
    assert_eq!(state.value.lower(), 50.0);
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
        value: SliderValue::Single(50.0),
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
    assert_eq!(horiz_state.value.lower(), 45.0);

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
    assert_eq!(horiz_state.value.lower(), 50.0);
}

#[test]
fn test_slider_tab_moves_focus_not_arrows() {
    let mut state_a = SliderState {
        value: SliderValue::Single(50.0),
        ..Default::default()
    };
    let mut state_b = SliderState {
        value: SliderValue::Single(50.0),
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
    assert_eq!(state_a.value.lower(), 50.0, "Value must not change on Tab");
}

#[test]
fn test_slider_click_takes_focus() {
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
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
        value: SliderValue::Single(50.0),
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
        value: SliderValue::Single(50.0),
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

    assert_eq!(state.value.lower(), 50.0); // Hasn't scrolled yet, scroll_delta is 0

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
    assert_eq!(state.value.lower(), 40.0);
}

#[test]
fn test_slider_wheel_over_overhanging_thumb() {
    let mut state = SliderState {
        value: SliderValue::Single(0.0),
        ..Default::default()
    };
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        min_gap: None,
        max_gap: None,
        style,
        ..test_spec(0.0, 100.0, true) // rect is Rect::new(0.0, 0.0, 20.0, 100.0)
    };

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();
    let parent_id = FocusId::new();

    // Mouse position at Vec2::new(10.0, 5.0), which is:
    // - outside the padded track_rect (y = 10..90)
    // - inside the overhanging thumb (y = 0..20)
    input.mouse_pos = crate::types::Vec2::new(10.0, 5.0);

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
    // Simulate a parent container registration to test claim/blocking
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    assert_eq!(state.value.lower(), 0.0); // Hasn't scrolled yet

    // Frame 2: Mouse wheel spun down (negative delta, scroll_delta.y < 0) -> value should increase
    input.scroll_delta.y = -1.0;
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
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    // Assert that the slider processed the wheel while the pointer was on the overhanging thumb.
    // value = 0.0 - (-1.0) * 5.0 = 5.0
    let after_first = state.value.lower();
    assert_eq!(after_first, 5.0);

    // Assert that the parent did not win the scroll down direction
    assert!(
        !focus_system.is_active_scroll_down(parent_id),
        "parent should not win scroll-down; slider should have claimed it"
    );

    // Frame 3: Mouse wheel spun down again, mouse remains stationary at Vec2::new(10.0, 5.0)
    // Value has moved away from 0.0, meaning thumb is no longer under pointer.
    input.scroll_delta.y = -1.0;
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
    focus_system.claim_scroll_down(parent_id);
    focus_system.end_frame();

    let after_second = state.value.lower();
    assert!(after_second > after_first);
    assert_eq!(after_second, 10.0);

    // Assert that the parent did not win the scroll down direction on the second detent either
    assert!(
        !focus_system.is_active_scroll_down(parent_id),
        "parent should not win scroll-down on second detent; slider should have claimed it"
    );
}

/// Track: y=0..100, thumb main-axis length=20, value=0 → thumb at y=0..20.
/// Click empty track at y=50 → page step to 20.0, is_track_clicking.
/// Move mouse by 5px (> 4px threshold) to y=55 → snaps:
///   thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25, switches to drag.
/// Then drag to y=65 → delta=10 → val_delta=12.5 → value=68.75.
#[test]
fn test_track_click_snaps_and_drags() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        min_gap: None,
        max_gap: None,
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
    assert!(state.active_part.is_none(), "should not yet be dragging");
    assert_eq!(state.value.lower(), 20.0, "page step should fire on click");

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
        state.active_part.is_some(),
        "should switch to dragging after threshold exceeded"
    );
    assert!(!state.is_track_clicking, "track clicking should end");
    // snap: thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25
    assert!(
        (state.value.lower() - 56.25).abs() < 0.01,
        "snap to 56.25, got {}",
        state.value.lower()
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
        (state.value.lower() - 68.75).abs() < 0.01,
        "drag to 68.75, got {}",
        state.value.lower()
    );
}

#[test]
fn test_track_click_cross_axis_drag_captures_pointer_outside_widget() {
    let mut state = SliderState::default();

    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min_gap: None,
        max_gap: None,
        style,
        ..test_spec(0.0, 100.0, true)
    };

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Horizontal slider:
    // - widget rect: x=0..100, y=0..20
    // - thumb main-axis length is 20, so usable track is x=10..90
    // - initial value 0 => thumb is at x=0..20
    //
    // Click empty track at x=50, y=19.5: horizontally in the middle of the track,
    // vertically right next to the bottom edge of the widget.
    input.mouse_pos = crate::types::Vec2::new(50.0, 19.5);

    // Warmup frame to establish hover claim.
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

    // Frame 1: press empty track. This should do the usual initial page step,
    // but it should not yet be a drag.
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
        "initial track press should enter track-clicking mode"
    );
    assert_eq!(
        state.active_part, None,
        "initial track press should not immediately start a drag"
    );
    assert_eq!(
        state.value.lower(),
        20.0,
        "initial track press should still perform the page step"
    );

    // Frame 1b: move the cursor a small distance (2.0px, less than the 4px drag threshold)
    // cross-axis so it moves off the widget (y=21.5). Advance time to 0.5 (next_repeat_time).
    // The repeated paging operation should still fire and continue.
    input.mouse_pressed = false;
    input.mouse_pos = crate::types::Vec2::new(50.0, 21.5);

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
        state.is_track_clicking,
        "should still be track-clicking since we are below the drag threshold"
    );
    assert_eq!(state.active_part, None, "should not start a drag yet");
    assert_eq!(
        state.value.lower(),
        40.0,
        "repeat paging should still fire even when cursor is slightly outside the widget"
    );

    // Frame 2: move only cross-axis, beyond the 4px drag threshold and outside
    // the widget rect. This is the desired new behaviour: the slider should
    // treat the original track press as captured, so leaving the widget must not
    // cancel track-clicking before drag promotion can happen.
    //
    // Current buggy behaviour likely cancels `is_track_clicking` here because
    // `track_rect.contains(input.mouse_pos)` becomes false, and no drag starts.
    input.mouse_pos = crate::types::Vec2::new(50.0, 25.0);

    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.1,
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

    assert_eq!(
        state.active_part,
        Some(SliderPart::LowerThumb),
        "cross-axis movement after a track press should promote to thumb drag, even outside the widget"
    );
    assert!(
        !state.is_track_clicking,
        "track-clicking should end once the interaction has promoted to a drag"
    );

    // On drag entry, the thumb should snap to the cursor's main-axis coordinate.
    // For this horizontal slider:
    //   track_start = 10
    //   track_len = 80
    //   mouse x = 50
    //   value = (50 - 10) / 80 * 100 = 50
    assert!(
        (state.value.lower() - 50.0).abs() < 0.01,
        "drag entry should snap to x=50 => value 50, got {}",
        state.value.lower()
    );

    // Frame 3: keep the cursor outside the widget, but now move parallel to the
    // track from x=50 to x=70. Because the track press has become a captured
    // drag, this should update the thumb exactly like normal thumb dragging.
    input.mouse_pos = crate::types::Vec2::new(70.0, 25.0);

    focus_system.begin_frame();
    raw::post_layout_slider(
        SliderSpec {
            time: 0.2,
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

    assert_eq!(
        state.active_part,
        Some(SliderPart::LowerThumb),
        "drag should remain active while the mouse is held, even outside the widget"
    );

    // Drag started at value 50 with drag_start_mouse_coord = 50.
    // Moving to x=70 gives delta=20.
    // val_delta = 20 / 80 * 100 = 25.
    // expected value = 50 + 25 = 75.
    assert!(
        (state.value.lower() - 75.0).abs() < 0.01,
        "parallel movement outside the widget should continue dragging to value 75, got {}",
        state.value.lower()
    );
}

// Regression: paging past the cursor causes direction-flip flicker.
// Setup: track y=0..100, thumb main-axis length=20, page_step=60.
// value=0 → thumb at y=0..20. Click at y=70 (below thumb).
// Frame 1 (initial click): page to 60 → thumb at y=48..68.
// Frame 2 (repeat at t=0.5): cursor y=70 > thumb_end=68, fires.
//   Buggy: 60+60=120 → clamped to 100 → thumb at 80..100 → cursor < thumb_start → flicker.
//   Fixed: clamp to cursor position (75.0) so thumb stops at cursor.
// Frame 3 (repeat at t=0.6): cursor inside thumb → paging stops.
#[test]
fn test_track_click_repeat_does_not_overshoot_cursor() {
    let mut state = SliderState::default();
    let mut style = SliderStyle::from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        page_step: 60.0,
        min_gap: None,
        max_gap: None,
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
    assert_eq!(state.value.lower(), 60.0, "initial page: 0 + 60 = 60");
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
    assert_eq!(state.value.lower(), 60.0);

    // Frame 3: repeat fires (t=0.5). Thumb at y=48..68, cursor at y=70 > 68 → fires.
    // Expected: value clamps to cursor position (75.0), NOT 100.
    // spec.rect is 0..100
    // thumb length is 20
    // visible/value track is padded to 10..90
    // range is 0..100
    // therefore y=70 maps to (70 - 10) / 80 * 100 = 75.0
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
        (state.value.lower() - 75.0).abs() < 0.01,
        "repeat should stop at cursor position 75, got {}",
        state.value.lower()
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
        (state.value.lower() - 75.0).abs() < 0.01,
        "value should not change after thumb reaches cursor, got {}",
        state.value.lower()
    );
    assert!(
        state.is_track_clicking,
        "is_track_clicking must stay true so drag is still possible"
    );
    assert!(state.active_part.is_none());

    // Frame 5: still holding, move mouse 5px (past 4px threshold from initial click at y=70).
    // Drag transition should fire: thumb snaps to cursor, enters drag mode.
    // snap: endpoint coordinate y=75 -> value=75.
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
        state.active_part.is_some(),
        "should enter drag mode after mouse moves past threshold"
    );
    assert!(!state.is_track_clicking);
    assert!(
        (state.value.lower() - 81.25).abs() < 0.01,
        "snap on drag entry: expected 81.25, got {}",
        state.value.lower()
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
        (state.value.lower() - 93.75).abs() < 0.01,
        "drag: expected 93.75, got {}",
        state.value.lower()
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
        min_gap: None,
        max_gap: None,
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

fn run_slider_once(spec: SliderSpec, state: &mut SliderState) {
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state,
        &Input::new(),
        &mut focus_system,
        &mut cmds,
    );
}

#[test]
fn test_single_slider_ignores_gap_constraints() {
    let mut state = SliderState {
        value: SliderValue::Single(150.0),
        ..Default::default()
    };
    run_slider_once(
        SliderSpec {
            min_gap: Some(40.0),
            max_gap: Some(40.0),
            ..test_spec(0.0, 100.0, true)
        },
        &mut state,
    );

    assert_eq!(state.value.lower(), 100.0);
    assert_eq!(state.value.upper(), None);
}

#[test]
fn test_range_slider_preserves_ordered_interval() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 20.0,
            upper: 80.0,
        },
        ..Default::default()
    };
    run_slider_once(test_spec(0.0, 100.0, true), &mut state);

    assert_eq!(state.value.lower(), 20.0);
    assert_eq!(state.value.upper(), Some(80.0));
}

#[test]
fn test_gap_repair_clamping_rules() {
    let spec = SliderSpec {
        min_gap: Some(20.0),
        max_gap: Some(40.0),
        ..test_spec(0.0, 100.0, true)
    };

    let mut reversed = SliderState {
        value: SliderValue::Range {
            lower: 80.0,
            upper: 20.0,
        },
        ..Default::default()
    };
    run_slider_once(spec.clone(), &mut reversed);
    assert_eq!(
        (reversed.value.lower(), reversed.value.upper()),
        (20.0, Some(60.0))
    );

    let mut too_small = SliderState {
        value: SliderValue::Range {
            lower: 40.0,
            upper: 45.0,
        },
        ..Default::default()
    };
    run_slider_once(spec.clone(), &mut too_small);
    assert_eq!(
        (too_small.value.lower(), too_small.value.upper()),
        (40.0, Some(60.0))
    );

    let mut too_large = SliderState {
        value: SliderValue::Range {
            lower: 10.0,
            upper: 90.0,
        },
        ..Default::default()
    };
    run_slider_once(spec, &mut too_large);
    assert_eq!(
        (too_large.value.lower(), too_large.value.upper()),
        (10.0, Some(50.0))
    );
}

#[test]
fn test_fixed_span_slider_clamps_at_domain_end() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 90.0,
            upper: 150.0,
        },
        ..Default::default()
    };
    run_slider_once(
        SliderSpec {
            min_gap: Some(30.0),
            max_gap: Some(30.0),
            ..test_spec(0.0, 100.0, true)
        },
        &mut state,
    );

    assert_eq!(
        (state.value.lower(), state.value.upper()),
        (70.0, Some(100.0))
    );
}

#[test]
fn test_segment_only_slider_drag_moves_fixed_span() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 20.0,
            upper: 40.0,
        },
        active_part: Some(SliderPart::Segment),
        drag_start_mouse_coord: 30.0,
        drag_start_value: SliderValue::Range {
            lower: 20.0,
            upper: 40.0,
        },
        ..Default::default()
    };
    let mut style = SliderStyle::scrollbar_from_theme(&crate::theme::Theme::framewise());
    style.lower_thumb_style = None;
    style.upper_thumb_style = None;
    let input = Input {
        mouse_down: true,
        mouse_pos: Vec2::new(50.0, 10.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();
    raw::post_layout_slider(
        SliderSpec {
            orientation: Orientation::Horizontal,
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            min_gap: Some(20.0),
            max_gap: Some(20.0),
            style,
            ..test_spec(0.0, 100.0, true)
        },
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut cmds,
    );

    assert_eq!(
        (state.value.lower(), state.value.upper()),
        (40.0, Some(60.0))
    );
}

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
    assert_eq!(state.value.lower(), 0.0);
}

#[test]
fn test_standalone_slider_wheel_at_max_blocks_propagation() {
    let mut state = SliderState {
        value: SliderValue::Single(100.0), // already at max
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
    assert_eq!(state.value.lower(), 100.0);
}

#[test]
fn test_vertical_standalone_slider_blocks_horizontal_scroll() {
    // Regression: vertical standalone slider inside a horizontal scroll area was
    // letting horizontal scroll events propagate because claim_scroll_at_ends only
    // claimed up/down, not left/right.
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
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
    assert_eq!(state.value.lower(), 0.0, "inner value unchanged");
}

#[test]
fn test_propagating_slider_at_max_yields_scroll_down_to_parent() {
    let mut state = SliderState {
        value: SliderValue::Single(100.0), // at max — can't scroll down
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
    assert_eq!(state.value.lower(), 100.0, "inner value unchanged");
}

#[test]
fn test_propagating_slider_mid_range_wins_both_directions() {
    // When not at an end, the inner propagating slider claims both directions
    // and the parent gets neither.
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
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
        value: SliderValue::Single(50.0),
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

    assert_eq!(
        state.value.lower(),
        50.0,
        "disabled slider must not change value"
    );
    assert!(
        state.active_part.is_none(),
        "disabled slider must not start a drag"
    );
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
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.0,
            upper: 50.0,
        },
        ..Default::default()
    };
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
    let track_color = theme.scrollbar_track_on_paper;
    let border_color = theme.line_soft_on_paper;
    let thumb_color = theme.ink;

    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        DrawCmd::FillRect { color, .. } if *color == tint(track_color)
    )));
    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        DrawCmd::StrokeLine { color, .. } if *color == tint(border_color)
    )));
    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        DrawCmd::FillRect { color, .. } if *color == tint(thumb_color)
    )));
}

fn input_none() -> Input {
    Input::new()
}

// ── Visual Tests ───────────────────────────────────────────────────────────

#[test]
fn test_slider_visual_normal() {
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
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
                rect: Rect::new(9.25, 6.0, 1.5, 44.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 50.0, 1.5, 44.0),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_hovered() {
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let spec = test_spec(0.0, 100.0, true);

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 50.0),
        ..Default::default()
    };

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

    // Second frame: hover is active, should resolve to theme.hover
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
                rect: Rect::new(9.25, 6.0, 1.5, 44.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 50.0, 1.5, 44.0),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev_hover,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_drag() {
    let mut state = SliderState {
        active_part: Some(SliderPart::LowerThumb),
        value: SliderValue::Single(50.0),
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
                rect: Rect::new(9.25, 6.0, 1.5, 44.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 50.0, 1.5, 44.0),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.rust,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_slider_visual_focused() {
    let mut state = SliderState {
        value: SliderValue::Single(50.0),
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
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 6.0, 1.5, 44.0),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(9.25, 50.0, 1.5, 44.0),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(-2.0, -2.0, 24.0, 104.0),
                color: theme.rust,
                width: 2.0,
                placement: crate::BorderPlacement::Outside,
                z: 1,
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
    // whose x starts at the resolved track rect's x = placement.x + padding.
    match &cmds[0] {
        crate::draw::DrawCmd::FillRect {
            anti_alias: false,
            rect,
            ..
        } => {
            assert_eq!(rect.x, placement.x + 6.0);
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
    style.lower_thumb_style.as_mut().unwrap().cross_axis = ThumbCrossAxis::FixedCentered(20.0);
    let spec = SliderSpec {
        page_step: 60.0,
        min_gap: None,
        max_gap: None,
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
    assert_eq!(state.value.lower(), 60.0);
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
    assert_eq!(state.value.lower(), 60.0);

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
    assert_eq!(state.value.lower(), 60.0, "should not jump back on itself");
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
fn test_segment_only_slider_visual_normal() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 38.0,
            upper: 62.0,
        },
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
        min_gap: None,
        max_gap: None,
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

    let track_color = theme.scrollbar_track_on_paper;
    let border_color = theme.line_soft_on_paper;
    let thumb_color = theme.ink;

    // Scrollbar-style: background_fill, segment, and separator line. No visible track strokes.
    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 100.0),
                color: track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 38.0, 18.0, 24.0),
                color: thumb_color,
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
        ]
    );
}

#[test]
fn test_segment_only_slider_visual_hover() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 38.0,
            upper: 62.0,
        },
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
        min_gap: None,
        max_gap: None,
        style,
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::YieldSameAxisAtEnds,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_pos: Vec2::new(10.0, 50.0), // over segment rect [1..19, 38..62]
        ..Default::default()
    };

    // Warmup frame
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

    // Second frame
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

    let track_color = theme.scrollbar_track_on_paper;
    let border_color = theme.line_soft_on_paper;
    // Scrollbar fill uses Color::BLACK for hovered segment
    let thumb_color = Color::BLACK;

    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 100.0),
                color: track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 38.0, 18.0, 24.0),
                color: thumb_color,
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
        ]
    );
}

#[test]
fn test_segment_only_slider_visual_drag() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 38.0,
            upper: 62.0,
        },
        active_part: Some(SliderPart::Segment),
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
        min_gap: None,
        max_gap: None,
        style,
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::YieldSameAxisAtEnds,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_down: true,
        ..Default::default()
    };

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

    let track_color = theme.scrollbar_track_on_paper;
    let border_color = theme.line_soft_on_paper;
    // Scrollbar fill uses theme.rust for dragged segment
    let thumb_color = theme.rust;

    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 100.0),
                color: track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 38.0, 18.0, 24.0),
                color: thumb_color,
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
        ]
    );
}

#[test]
fn test_segment_only_slider_visual_focused() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 38.0,
            upper: 62.0,
        },
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
        min_gap: None,
        max_gap: None,
        style,
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::YieldSameAxisAtEnds,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    focus_system.take_keyboard_focus(state.focus_id);

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

    let track_color = theme.scrollbar_track_on_paper;
    let border_color = theme.line_soft_on_paper;
    let thumb_color = theme.ink;

    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 100.0),
                color: track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 38.0, 18.0, 24.0),
                color: thumb_color,
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
            DrawCmd::BorderRect {
                rect: Rect::new(-2.0, -2.0, 24.0, 104.0),
                color: theme.rust,
                width: 2.0,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
        ]
    );
}

#[test]
fn test_slider_track_line_invisible_stroke() {
    let mut state = SliderState::default();
    let mut spec = test_spec(0.0, 100.0, false);

    // Set track stroke to an invisible stroke
    spec.style.before_stroke = None;
    spec.style.after_stroke = None;

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

    // Since track stroke is None, it is not visible.
    // Let's assert that there is no FillRect with width 1.5 (which is the default thickness).
    for cmd in cmds.commands() {
        if let DrawCmd::FillRect { rect, .. } = cmd {
            if rect.w == 1.5 || rect.h == 1.5 {
                panic!("found track line or fill bar DrawCmd::FillRect when track stroke is invisible: {:?}", rect);
            }
        }
    }
}

#[test]
fn test_overhanging_thumb_hover_and_click() {
    let mut state = SliderState {
        value: SliderValue::Single(0.0),
        ..Default::default()
    };
    let spec = test_spec(0.0, 100.0, false); // vertical track from y=0 to y=100
                                             // thumb is centered at lower_coord = 6.0, with len=12.0, so y bounds are 0.0 to 12.0.

    // 1. Hover test: mouse at x=10.0, y=3.0 (outside track, but inside thumb)
    let input_hover = Input {
        mouse_pos: Vec2::new(10.0, 3.0),
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    // Frame 1: Warmup frame
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_hover,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Hover resolution frame
    focus_system.begin_frame();
    let result = raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_hover,
        &mut focus_system,
        &mut cmds,
    );
    // Since hit_part is Some(LowerThumb), it claims hover even outside track_rect.
    assert!(
        result.input.hovered,
        "should be hovered because pointer is inside the overhanging thumb"
    );
    focus_system.end_frame();

    // 2. Click/Drag test: click at x=10.0, y=3.0 (reusing the same focus_system where hover is active)
    let input_click = Input {
        mouse_pos: Vec2::new(10.0, 3.0),
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    let _result = raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_click,
        &mut focus_system,
        &mut cmds,
    );
    assert_eq!(
        state.active_part,
        Some(SliderPart::LowerThumb),
        "should claim active part on click"
    );
}

#[test]
fn test_lower_drag_respects_min_gap() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 30.0,
            upper: 80.0,
        },
        active_part: Some(SliderPart::LowerThumb),
        drag_start_mouse_coord: 30.0,
        drag_start_value: SliderValue::Range {
            lower: 30.0,
            upper: 80.0,
        },
        ..Default::default()
    };
    // Let's use a horizontal slider from x=0 to x=100.
    // min_gap = 20.0.
    let mut spec = test_spec(0.0, 100.0, false);
    spec.orientation = Orientation::Horizontal;
    spec.rect = Rect::new(0.0, 0.0, 100.0, 20.0);
    spec.min_gap = Some(20.0);

    // Drag lower thumb to coord 70.0 (which would map to value 70.0).
    let input = Input {
        mouse_down: true,
        mouse_pos: Vec2::new(70.0, 10.0),
        ..Default::default()
    };

    let mut focus_system = FocusSystem::new();
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

    // Clamped lower: upper - min_gap = 80.0 - 20.0 = 60.0. Upper preserved: 80.0.
    assert_eq!(state.value.lower(), 60.0);
    assert_eq!(state.value.upper(), Some(80.0));
}

#[test]
fn test_upper_drag_respects_min_gap() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 30.0,
            upper: 80.0,
        },
        active_part: Some(SliderPart::UpperThumb),
        drag_start_mouse_coord: 80.0,
        drag_start_value: SliderValue::Range {
            lower: 30.0,
            upper: 80.0,
        },
        ..Default::default()
    };
    // Let's use a horizontal slider from x=0 to x=100.
    // min_gap = 20.0.
    let mut spec = test_spec(0.0, 100.0, false);
    spec.orientation = Orientation::Horizontal;
    spec.rect = Rect::new(0.0, 0.0, 100.0, 20.0);
    spec.min_gap = Some(20.0);

    // Drag upper thumb to coord 40.0 (which would map to value 40.0).
    let input = Input {
        mouse_down: true,
        mouse_pos: Vec2::new(40.0, 10.0),
        ..Default::default()
    };

    let mut focus_system = FocusSystem::new();
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

    // Clamped upper: lower + min_gap = 30.0 + 20.0 = 50.0. Lower preserved: 30.0.
    assert_eq!(state.value.lower(), 30.0);
    assert_eq!(state.value.upper(), Some(50.0));
}

#[test]
fn test_away_drags_respect_max_gap() {
    let mut spec = test_spec(0.0, 100.0, false);
    spec.orientation = Orientation::Horizontal;
    spec.rect = Rect::new(0.0, 0.0, 100.0, 20.0);
    spec.max_gap = Some(30.0);

    // Case 1: Lower thumb drag away (downwards) from upper thumb
    {
        let mut state = SliderState {
            value: SliderValue::Range {
                lower: 30.0,
                upper: 50.0,
            },
            active_part: Some(SliderPart::LowerThumb),
            drag_start_mouse_coord: 30.0,
            drag_start_value: SliderValue::Range {
                lower: 30.0,
                upper: 50.0,
            },
            ..Default::default()
        };
        // Drag lower to 10.0
        let input = Input {
            mouse_down: true,
            mouse_pos: Vec2::new(10.0, 10.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
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
        // Clamped lower: upper - max_gap = 50.0 - 30.0 = 20.0. Upper preserved: 50.0.
        assert_eq!(state.value.lower(), 20.0);
        assert_eq!(state.value.upper(), Some(50.0));
    }

    // Case 2: Upper thumb drag away (upwards) from lower thumb
    {
        let mut state = SliderState {
            value: SliderValue::Range {
                lower: 30.0,
                upper: 50.0,
            },
            active_part: Some(SliderPart::UpperThumb),
            drag_start_mouse_coord: 50.0,
            drag_start_value: SliderValue::Range {
                lower: 30.0,
                upper: 50.0,
            },
            ..Default::default()
        };
        // Drag upper to 90.0
        let input = Input {
            mouse_down: true,
            mouse_pos: Vec2::new(90.0, 10.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
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
        // Clamped upper: lower + max_gap = 30.0 + 30.0 = 60.0. Lower preserved: 30.0.
        assert_eq!(state.value.lower(), 30.0);
        assert_eq!(state.value.upper(), Some(60.0));
    }
}

#[test]
fn test_range_slider_visual_normal() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input::new();
    let mut focus_system = FocusSystem::new();
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
        &cmds[..],
        &[
            // Before track
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            // After track
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            // Segment
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            // Lower thumb fill
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            // Lower thumb border
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            // Upper thumb fill
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0 + 60.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            // Upper thumb border
            DrawCmd::BorderRect {
                rect: Rect::new(6.0 + 60.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_hover_lower_thumb() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_pos: Vec2::new(25.0, 10.0), // over lower thumb rect [22..34, 4..16]
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();

    // Warmup frame
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

    // Second frame
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            // Lower thumb fill is now hovered
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev_hover,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_hover_upper_thumb() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_pos: Vec2::new(75.0, 10.0), // over upper thumb rect [66..78, 4..16]
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();

    // Warmup frame
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

    // Second frame
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            // Upper thumb fill is hovered
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev_hover,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_hover_segment() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_pos: Vec2::new(50.0, 10.0), // over segment rect [28..72, 9.25..10.75]
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();

    // Warmup frame
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

    // Second frame
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            // Segment is hovered
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: Color::BLACK,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_drag_lower_thumb() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        active_part: Some(SliderPart::LowerThumb),
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_down: true,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            // Lower thumb fill is now dragged, and border is active/dragged
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.rust,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_drag_upper_thumb() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        active_part: Some(SliderPart::UpperThumb),
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_down: true,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            // Upper thumb fill is now dragged, and border is active/dragged
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.rust,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_drag_segment() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        active_part: Some(SliderPart::Segment),
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let input = Input {
        mouse_down: true,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
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
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            // Segment is dragged
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.rust,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_range_slider_visual_focused() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 25.0,
            upper: 75.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

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

    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(6.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(72.0, 9.25, 22.0, 1.5),
                color: theme.line_on_paper,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(28.0, 9.25, 44.0, 1.5),
                color: theme.ink,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(22.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.paper_elev,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, 4.0, 12.0, 12.0),
                color: theme.ink,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            // Focus outline around spec.rect
            DrawCmd::BorderRect {
                rect: Rect::new(-2.0, -2.0, 104.0, 24.0),
                color: theme.rust,
                width: 2.0,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
        ]
    );
}

#[test]
fn test_range_slider_segment_drag() {
    let theme = crate::theme::Theme::framewise();
    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    // Subcase A: Lower thumb wins over segment
    {
        let mut state = SliderState {
            value: SliderValue::Range {
                lower: 20.0,
                upper: 60.0,
            },
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // Put mouse inside lower thumb rect (lower thumb center x = 23.6, thumb is 12px wide, so x: 17.6..29.6)
        // Click at x = 25.0 (which overlaps with segment [23.6..58.8])
        let input_hover = Input {
            mouse_pos: Vec2::new(25.0, 10.0),
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_hover,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let input_click = Input {
            mouse_pos: Vec2::new(25.0, 10.0),
            mouse_down: true,
            mouse_pressed: true,
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_click,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.active_part, Some(SliderPart::LowerThumb));
    }

    // Subcase B: Upper thumb wins over segment
    {
        let mut state = SliderState {
            value: SliderValue::Range {
                lower: 20.0,
                upper: 60.0,
            },
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // Put mouse inside upper thumb rect (upper thumb center x = 58.8, thumb is 12px wide, so x: 52.8..64.8)
        // Click at x = 55.0 (which overlaps with segment [23.6..58.8])
        let input_hover = Input {
            mouse_pos: Vec2::new(55.0, 10.0),
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_hover,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let input_click = Input {
            mouse_pos: Vec2::new(55.0, 10.0),
            mouse_down: true,
            mouse_pressed: true,
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_click,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.active_part, Some(SliderPart::UpperThumb));
    }

    // Subcase C: Segment area away from thumbs starts segment drag
    {
        let mut state = SliderState {
            value: SliderValue::Range {
                lower: 20.0,
                upper: 60.0,
            },
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // Put mouse clearly inside segment but away from either thumb, e.g. at x = 40.0
        let input_hover = Input {
            mouse_pos: Vec2::new(40.0, 10.0),
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_hover,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let input_click = Input {
            mouse_pos: Vec2::new(40.0, 10.0),
            mouse_down: true,
            mouse_pressed: true,
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_click,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.active_part, Some(SliderPart::Segment));

        // Move the mouse right by 10 value units.
        // track_len is 88px for range 100.
        // So 10 value units is 8.8 pixels.
        // Let's move the mouse by 8.8 pixels: x = 48.8.
        let input_drag = Input {
            mouse_pos: Vec2::new(48.8, 10.0),
            mouse_down: true,
            mouse_pressed: false,
            ..Default::default()
        };
        focus_system.begin_frame();
        raw::post_layout_slider(
            spec.clone(),
            raw::SliderPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state,
            &input_drag,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // 20..60 -> drag right by 10 value units -> 30..70
        assert!((state.value.lower() - 30.0).abs() < 1e-3);
        assert!((state.value.upper().unwrap() - 70.0).abs() < 1e-3);
    }
}

#[test]
fn test_range_slider_track_click_pages_whole_range() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 20.0,
            upper: 40.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    // Click at x=80.0, which is after the range (20..40 maps to coords 23.6..41.2)
    let input_hover = Input {
        mouse_pos: Vec2::new(80.0, 10.0),
        ..Default::default()
    };
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_hover,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let input_click = Input {
        mouse_pos: Vec2::new(80.0, 10.0),
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_click,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value.lower(), 40.0);
    assert_eq!(state.value.upper(), Some(60.0));
    assert!(state.is_track_clicking);
}

#[test]
fn test_segment_only_slider_track_click_pages_segment() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 20.0,
            upper: 40.0,
        },
        ..Default::default()
    };

    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: Some(20.0),
        max_gap: Some(20.0),
        style: SliderStyle::scrollbar_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();

    // Click at x=80.0, which is after the segment (20..40 maps to coords 20.0..40.0)
    let input_hover = Input {
        mouse_pos: Vec2::new(80.0, 10.0),
        ..Default::default()
    };
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec.clone(),
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_hover,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    let input_click = Input {
        mouse_pos: Vec2::new(80.0, 10.0),
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    raw::post_layout_slider(
        spec,
        raw::SliderPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input_click,
        &mut focus_system,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value.lower(), 40.0);
    assert_eq!(state.value.upper(), Some(60.0));
    assert!(state.is_track_clicking);
}

#[test]
fn test_range_slider_keyboard_preserves_span() {
    let theme = crate::theme::Theme::framewise();
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 30.0,
            upper: 50.0,
        },
        ..Default::default()
    };
    let spec = SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        min: 0.0,
        max: 100.0,
        page_step: 20.0,
        step: 5.0,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    };

    let mut input = Input::new();
    let mut focus_system = FocusSystem::new();

    // Must be focused to receive keyboard events
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: register keyboard focus
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

    // Subcase: Right or Down should move both endpoints by step, preserving span
    // 30..50 -> 35..55
    focus_system.begin_frame();
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
    assert_eq!(state.value.lower(), 35.0);
    assert_eq!(state.value.upper(), Some(55.0));
    focus_system.end_frame();

    // Left or Up should move both endpoints back
    // 35..55 -> 30..50
    focus_system.begin_frame();
    input.key_pressed_right = false;
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
    assert_eq!(state.value.lower(), 30.0);
    assert_eq!(state.value.upper(), Some(50.0));
    focus_system.end_frame();

    // PageDown should move both endpoints by page_step
    // 30..50 -> 50..70
    focus_system.begin_frame();
    input.key_pressed_left = false;
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
    assert_eq!(state.value.lower(), 50.0);
    assert_eq!(state.value.upper(), Some(70.0));
    focus_system.end_frame();

    // PageUp should move both endpoints back
    // 50..70 -> 30..50
    focus_system.begin_frame();
    input.key_pressed_page_down = false;
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
    assert_eq!(state.value.lower(), 30.0);
    assert_eq!(state.value.upper(), Some(50.0));
    focus_system.end_frame();

    // Home should move the range to the minimum while preserving span
    // 30..50 -> 0..20
    focus_system.begin_frame();
    input.key_pressed_page_up = false;
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
    assert_eq!(state.value.lower(), 0.0);
    assert_eq!(state.value.upper(), Some(20.0));
    focus_system.end_frame();

    // Reset value for end test
    state.value = SliderValue::Range {
        lower: 30.0,
        upper: 50.0,
    };

    // End should move the range to the maximum while preserving span
    // 30..50 -> 80..100
    focus_system.begin_frame();
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
    assert_eq!(state.value.lower(), 80.0);
    assert_eq!(state.value.upper(), Some(100.0));
    focus_system.end_frame();
}

fn test_range_spec_horizontal(min: f32, max: f32) -> SliderSpec {
    let theme = crate::theme::Theme::framewise();
    SliderSpec {
        orientation: Orientation::Horizontal,
        rect: Rect::new(0.0, 0.0, 112.0, 20.0),
        min,
        max,
        page_step: 0.1,
        step: 0.01,
        min_gap: None,
        max_gap: None,
        style: SliderStyle::range_from_theme(&theme),
        clip_rect: None,
        scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
        time: 0.0,
        disabled: false,
        keyboard_focusable: true,
        layer: Layer::default(),
    }
}

#[test]
fn test_range_slider_overlap_partial() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.4,
            upper: 0.5,
        },
        ..Default::default()
    };
    let spec = test_range_spec_horizontal(0.0, 1.0);
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();
    let input = Input::new();

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

    // Verify drawing commands
    let mut fill_rects = Vec::new();
    let mut stroke_rects = Vec::new();
    let mut stroke_lines = Vec::new();

    for cmd in cmds.commands() {
        match cmd {
            DrawCmd::FillRect { rect, .. } => fill_rects.push(*rect),
            DrawCmd::BorderRect { rect, .. } => stroke_rects.push(*rect),
            DrawCmd::StrokeLine { p0, p1, .. } => stroke_lines.push((*p0, *p1)),
            _ => {}
        }
    }

    // Filter to only find lower and upper thumb fills (their heights are 12.0)
    let thumb_fills: Vec<Rect> = fill_rects.into_iter().filter(|r| r.h == 12.0).collect();
    assert_eq!(thumb_fills.len(), 2);
    assert_eq!(thumb_fills[0], Rect::new(40.0, 4.0, 11.5, 12.0));
    assert_eq!(thumb_fills[1], Rect::new(50.5, 4.0, 11.5, 12.0));

    assert_eq!(stroke_rects.len(), 1);
    assert_eq!(stroke_rects[0], Rect::new(40.0, 4.0, 22.0, 12.0));

    assert_eq!(stroke_lines.len(), 2);
    assert_eq!(stroke_lines[0].0.x, 46.0);
    assert_eq!(stroke_lines[1].0.x, 56.0);

    // Verify hit-testing: click left of midpoint (51.0)
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.4,
            upper: 0.5,
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut input = Input::new();
    input.mouse_pos = Vec2::new(50.0, 10.0);

    // Frame 1: Hover claim
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

    // Frame 2: Click
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.active_part, Some(SliderPart::LowerThumb));
    focus_system.end_frame();

    // Verify hit-testing: click right of midpoint (51.0)
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.4,
            upper: 0.5,
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut input = Input::new();
    input.mouse_pos = Vec2::new(52.0, 10.0);

    // Frame 1: Hover claim
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

    // Frame 2: Click
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.active_part, Some(SliderPart::UpperThumb));
    focus_system.end_frame();
}

#[test]
fn test_range_slider_overlap_full() {
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.5,
            upper: 0.5,
        },
        ..Default::default()
    };
    let spec = test_range_spec_horizontal(0.0, 1.0);
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new();
    let input = Input::new();

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

    // Verify drawing commands
    let mut fill_rects = Vec::new();
    let mut stroke_rects = Vec::new();
    let mut stroke_lines = Vec::new();

    for cmd in cmds.commands() {
        match cmd {
            DrawCmd::FillRect { rect, .. } => fill_rects.push(*rect),
            DrawCmd::BorderRect { rect, .. } => stroke_rects.push(*rect),
            DrawCmd::StrokeLine { p0, p1, .. } => stroke_lines.push((*p0, *p1)),
            _ => {}
        }
    }

    let thumb_fills: Vec<Rect> = fill_rects.into_iter().filter(|r| r.h == 12.0).collect();
    assert_eq!(thumb_fills.len(), 2);
    assert_eq!(thumb_fills[0], Rect::new(50.0, 4.0, 6.5, 12.0));
    assert_eq!(thumb_fills[1], Rect::new(55.5, 4.0, 6.5, 12.0));

    assert_eq!(stroke_rects.len(), 1);
    assert_eq!(stroke_rects[0], Rect::new(50.0, 4.0, 12.0, 12.0));

    assert_eq!(stroke_lines.len(), 2);
    assert_eq!(stroke_lines[0].0.x, 56.0);
    assert_eq!(stroke_lines[1].0.x, 56.0);

    // Verify hit-testing: click left of midpoint (56.0)
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.5,
            upper: 0.5,
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut input = Input::new();
    input.mouse_pos = Vec2::new(55.0, 10.0);

    // Frame 1: Hover claim
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

    // Frame 2: Click
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.active_part, Some(SliderPart::LowerThumb));
    focus_system.end_frame();

    // Verify hit-testing: click right of midpoint (56.0)
    let mut state = SliderState {
        value: SliderValue::Range {
            lower: 0.5,
            upper: 0.5,
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut input = Input::new();
    input.mouse_pos = Vec2::new(57.0, 10.0);

    // Frame 1: Hover claim
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

    // Frame 2: Click
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.active_part, Some(SliderPart::UpperThumb));
    focus_system.end_frame();
}
