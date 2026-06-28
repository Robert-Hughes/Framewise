use super::raw::TextEditSpec;
use super::*;
use crate::draw::DrawCmd;

use crate::{
    layouts::{ColumnLayout, ColumnLayoutParams},
    test_utils::TestTextBackend,
    theme::Theme,
    DrawGlyph, PreparedGlyphToken,
};

#[test]
fn test_spec_default_from_theme_fills_style() {
    let theme = crate::theme::Theme::framewise();
    let spec = super::TextEditSpec::default_from_theme(&theme);
    assert_eq!(spec.style.font, TextEditStyle::from_theme(&theme).font);
    assert_eq!(spec.style.size, TextEditStyle::from_theme(&theme).size);
}

#[test]
fn test_spec_theme_uses_single_line_vertical_padding() {
    let theme = crate::theme::Theme::framewise();
    let spec = super::TextEditSpec::default().theme(&theme);

    assert_eq!(spec.style.padding_y, 0.0);
}

#[test]
fn test_spec_theme_uses_multiline_vertical_padding() {
    let theme = crate::theme::Theme::framewise();

    let allow_newlines = super::TextEditSpec::default()
        .newline_policy(NewlinePolicy::Preserve)
        .theme(&theme);
    assert_eq!(allow_newlines.style.padding_y, 8.0);

    let wrapped = super::TextEditSpec::default().wrap(true).theme(&theme);
    assert_eq!(wrapped.style.padding_y, 8.0);
}

#[test]
fn test_spec_style_setter_sets_style() {
    let mut custom_style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
    custom_style.size = 99.0;
    let spec = super::TextEditSpec::default().style(custom_style);
    assert_eq!(spec.style.size, 99.0);
}

#[test]
fn test_text_edit_style_scroll_area_defaults() {
    let theme = crate::theme::Theme::framewise();
    let style = TextEditStyle::from_theme(&theme);
    assert_eq!(
        style.scroll_area_style.scrollbar_width,
        TEXT_EDIT_SCROLLBAR_WIDTH
    );
    assert_eq!(style.scroll_area_style.corner_color, Some(theme.paper_elev));
    assert_eq!(style.scroll_area_style.scrollbar_style.before_stroke, None);
    assert_eq!(
        style
            .scroll_area_style
            .scrollbar_style
            .segment_style
            .unwrap()
            .cross_axis_size,
        crate::widgets::slider::CrossAxisSize::FillTrack { margin: 0.0 }
    );
}

fn spec() -> TextEditSpec {
    let mut style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
    style.padding_x = 4.0;
    style.padding_y = 4.0;

    TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 30.0),
        placeholder: None,
        clip_rect: None,
        error: false,
        disabled: false,
        time: 0.0,
        layer: Layer::default(),
        newline_policy: NewlinePolicy::ReplaceWithSpace,
        wrap: false,
        vertical_align: Align::Center,
        line_align: TextLineAlign::Start,
        style,
    }
}

fn caret_byte(state: &TextEditState) -> usize {
    insertion_byte_for_position(&state.value, state.caret)
}

fn selection_byte(state: &TextEditState) -> Option<usize> {
    state
        .selection_anchor
        .map(|position| insertion_byte_for_position(&state.value, position))
}

fn set_caret_byte(state: &mut TextEditState, byte: usize) {
    state.caret = caret_position_at_byte(&state.value, byte);
}

fn set_selection_byte(state: &mut TextEditState, byte: Option<usize>) {
    state.selection_anchor = byte.map(|byte| caret_position_at_byte(&state.value, byte));
}

fn find_caret_rect(cmds: &DrawCommands, caret_color: Color) -> Rect {
    cmds.iter()
        .find_map(|cmd| match cmd {
            DrawCmd::FillRect { rect, color, .. } if *color == caret_color => Some(*rect),
            _ => None,
        })
        .expect("caret rect should be drawn")
}

fn test_text_edit_scroll_outer_rect(edit_spec: &TextEditSpec) -> Rect {
    let mut scroll_outer_rect = edit_spec
        .rect
        .inset(edit_spec.style.border.map_or(0.0, |b| b.width));
    if edit_spec.error {
        scroll_outer_rect.x += edit_spec.style.error_stripe_width;
        scroll_outer_rect.w -= edit_spec.style.error_stripe_width;
    }
    scroll_outer_rect
}

fn focused_text_edit_state(text: &str, focus_system: &mut FocusSystem) -> TextEditState {
    let mut state = TextEditState::new(text);
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    state.had_keyboard_focus = true;
    state
}

fn run_raw_text_edit_frame(
    edit_spec: TextEditSpec,
    state: &mut TextEditState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut TestTextBackend,
    cmds: &mut DrawCommands,
) -> raw::TextEditResult {
    focus_system.begin_frame();
    let pre_layout = raw::post_layout_only_pre_layout_result(state);
    let result = raw::post_layout_text_edit(
        edit_spec,
        pre_layout,
        state,
        input,
        focus_system,
        text_backend,
        cmds,
    );
    focus_system.end_frame();
    result
}

fn has_text_edit_scrollbar(cmds: &DrawCommands, bounds: Rect) -> bool {
    cmds.iter().any(|cmd| match cmd {
        DrawCmd::FillRect { rect, .. } => {
            let inside_bounds = rect.x >= bounds.x
                && rect.y >= bounds.y
                && rect.right() <= bounds.right()
                && rect.bottom() <= bounds.bottom();
            inside_bounds
                && ((rect.w == TEXT_EDIT_SCROLLBAR_WIDTH && rect.h <= bounds.h)
                    || (rect.h == TEXT_EDIT_SCROLLBAR_WIDTH && rect.w <= bounds.w))
        }
        _ => false,
    })
}

/// Regression test for autosized text_edit input flicker.
///
/// A focused auto-sized text edit should apply same-frame text input before
/// requesting its size, so the frame that inserts a character is laid out
/// wide enough for the new value. Otherwise the widget draws the new text
/// into a rect sized for the old value and briefly shows scrollbars until
/// the next frame catches up.
#[test]
fn test_high_level_auto_sized_text_edit_sizes_same_frame_text_input() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut state = focused_text_edit_state("a", &mut focus_system);

    input.text_events.push(TextEvent::Char('b'));

    let theme = Theme::framewise();
    let mut style = TextEditStyle::from_theme(&theme);
    style.padding_x = 0.0;
    style.padding_y = 0.0;
    style.border = None;
    style.focus_border = None;
    style.min_height = 0.0;
    style.error_stripe_width = 0.0;

    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme,
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    let result = text_edit(
        super::TextEditSpec::default()
            .style(style)
            .wrap(false)
            .newline_policy(NewlinePolicy::ReplaceWithSpace),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert_eq!(state.value, "ab");
    assert_eq!(result.layout.bounds.w, 16.0);
    assert!(
        !has_text_edit_scrollbar(&cmds, result.layout.bounds),
        "same-frame text input should not render a transient scrollbar"
    );
}

/// Guards the safe-prefix path for mixed selection and insertion input.
///
/// If `SelectAll` and the following character are both processed before sizing, the
/// auto-sized layout can request the width for the replacement text in the same frame.
/// Splitting those events incorrectly would either size for stale text or fail to replace
/// the selected range.
#[test]
fn test_high_level_text_edit_pre_layout_select_all_then_char_replaces_selection() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut state = focused_text_edit_state("abc", &mut focus_system);

    input.text_events.push(TextEvent::SelectAll);
    input.text_events.push(TextEvent::Char('x'));

    let theme = Theme::framewise();
    let mut style = TextEditStyle::from_theme(&theme);
    style.padding_x = 0.0;
    style.padding_y = 0.0;
    style.border = None;
    style.focus_border = None;
    style.min_height = 0.0;
    style.error_stripe_width = 0.0;

    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme,
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    let result = text_edit(
        super::TextEditSpec::default().style(style).wrap(false),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert_eq!(state.value, "x");
    assert_eq!(result.layout.bounds.w, 8.0);
}

/// Guards event ordering when a geometry-dependent event appears before text input.
///
/// `CaretLeft` must remain post-layout because it may depend on visual caret geometry. Once
/// the safe prefix stops there, the following character must also stay in post-layout so the
/// caret move happens before insertion instead of being reordered ahead of it.
#[test]
fn test_high_level_text_edit_unsupported_event_stops_pre_layout_prefix() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut state = focused_text_edit_state("ab", &mut focus_system);

    input.text_events.push(TextEvent::CaretLeft {
        shift: false,
        ctrl: false,
    });
    input.text_events.push(TextEvent::Char('x'));

    let theme = Theme::framewise();
    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme,
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    text_edit(
        super::TextEditSpec::default()
            .style(spec().style)
            .wrap(false)
            .newline_policy(NewlinePolicy::ReplaceWithSpace),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert_eq!(state.value, "axb");
}

#[test]
fn idle_wrapped_text_edit_uses_one_prepared_layout() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrst");
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0);
    edit_spec.wrap = true;

    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );

    assert_eq!(
        text_backend.observations.shape_text_calls, 1,
        "idle wrapped rendering should not measure and then lay out again"
    );
}

fn glyphs(items: &[(char, f32, f32)]) -> Vec<DrawGlyph> {
    items
        .iter()
        .map(|(ch, x, y)| DrawGlyph {
            token: PreparedGlyphToken(*ch as u64),
            top_left: Vec2::new(*x, *y),
        })
        .collect()
}

fn insertion_byte_for_position(text: &str, position: CaretPosition) -> usize {
    position.insertion_byte_hint().min(text.len())
}

#[test]
fn test_text_edit_overlapping_hover() {
    let mut text_backend = TestTextBackend::default();
    let mut state1 = TextEditState::default();
    let mut state2 = TextEditState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let mut spec1 = spec();
            spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
            let mut spec2 = spec();
            spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

            let res1 = raw::post_layout_text_edit(
                spec1,
                raw::post_layout_only_pre_layout_result(state1),
                state1,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res2 = raw::post_layout_text_edit(
                spec2,
                raw::post_layout_only_pre_layout_result(state2),
                state2,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            (res1.input, res2.input)
        },
    );
}

#[test]
fn test_text_edit_overlapping_click() {
    let mut text_backend = TestTextBackend::default();
    let mut state1 = TextEditState::default();
    let mut state2 = TextEditState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        true,
        |state1, state2, input, focus_system, cmds| {
            let mut spec1 = spec();
            spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
            let mut spec2 = spec();
            spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

            let res1 = raw::post_layout_text_edit(
                spec1,
                raw::post_layout_only_pre_layout_result(state1),
                state1,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res2 = raw::post_layout_text_edit(
                spec2,
                raw::post_layout_only_pre_layout_result(state2),
                state2,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            (res1.input, res2.input)
        },
    );
}

#[test]
fn test_typing_and_cursor() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("");

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut input = Input::default();
    input.text_events.push(TextEvent::Char('a'));
    input.text_events.push(TextEvent::Char('b'));
    input.text_events.push(TextEvent::Char('c'));

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "abc");
    assert_eq!(caret_byte(&state), 3);

    // Move left
    input.text_events.clear();
    input.text_events.push(TextEvent::CaretLeft {
        shift: false,
        ctrl: false,
    });
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(caret_byte(&state), 2);

    // Insert at cursor
    input.text_events.clear();
    input.text_events.push(TextEvent::Char('x'));
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "abxc");
    assert_eq!(caret_byte(&state), 3);
}

#[test]
fn test_backspace_and_delete() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    set_caret_byte(&mut state, 3);
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut input = Input::default();
    input.text_events.push(TextEvent::Backspace { ctrl: false });

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "helo");
    assert_eq!(caret_byte(&state), 2);

    input.text_events.clear();
    input.text_events.push(TextEvent::Delete { ctrl: false });
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "heo");
    assert_eq!(caret_byte(&state), 2);
}

#[test]
fn test_ctrl_backspace_and_delete() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");
    set_caret_byte(&mut state, 8); // "hello wo|rld"
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut input = Input::default();
    input.text_events.push(TextEvent::Backspace { ctrl: true });

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "hello rld");
    assert_eq!(caret_byte(&state), 6); // end of "hello "

    input.text_events.clear();
    input.text_events.push(TextEvent::Delete { ctrl: true });
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "hello ");
    assert_eq!(caret_byte(&state), 6);
}

#[test]
fn test_selection_and_replacement() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    set_caret_byte(&mut state, 1);
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut input = Input::default();
    input.text_events.push(TextEvent::CaretRight {
        shift: true,
        ctrl: false,
    });
    input.text_events.push(TextEvent::CaretRight {
        shift: true,
        ctrl: false,
    });

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(selection_byte(&state), Some(1));
    assert_eq!(caret_byte(&state), 3);

    input.text_events.clear();
    input.text_events.push(TextEvent::Char('a'));
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(state.value, "halo");
    assert_eq!(caret_byte(&state), 2);
    assert_eq!(selection_byte(&state), None);
}

#[test]
fn test_text_edit_left_right_skip_same_byte_visual_side() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("ab");
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    state.caret = CaretPosition::BeforeCluster {
        cluster_byte_start: 0,
    };
    let mut input = Input::default();
    input.text_events.push(TextEvent::CaretRight {
        shift: false,
        ctrl: false,
    });

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(
        state.caret,
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1
        }
    );
    assert_eq!(caret_byte(&state), 1);

    input.text_events.clear();
    input.text_events.push(TextEvent::CaretLeft {
        shift: false,
        ctrl: false,
    });

    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert_eq!(
        state.caret,
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0
        }
    );
    assert_eq!(caret_byte(&state), 0);
}

#[test]
fn test_mouse_release_preserves_visual_side_at_shared_insertion() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("ab");
    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 18.0, 50.0),
        wrap: true,
        ..spec()
    };
    let mut input = Input {
        mouse_pos: Vec2::new(15.0, 8.0),
        ..Default::default()
    };

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(
        state.caret,
        CaretPosition::AfterCluster {
            cluster_byte_start: 0,
            cluster_byte_end: 1,
        }
    );
    assert_eq!(caret_byte(&state), 1);

    input.mouse_down = false;
    input.mouse_pressed = false;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(
        state.caret,
        CaretPosition::AfterCluster {
            cluster_byte_start: 0,
            cluster_byte_end: 1,
        }
    );
    assert_eq!(caret_byte(&state), 1);
}

#[test]
fn test_empty_mouse_click_keeps_empty_caret() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::default();
    let mut input = Input {
        mouse_pos: Vec2::new(120.0, 15.0),
        ..Input::default()
    };

    focus_system.begin_frame();
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );

    assert_eq!(state.caret, CaretPosition::EmptyText);
    assert_eq!(caret_byte(&state), 0);
}

#[test]
fn test_mouse_clicking_and_dragging() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    let mut input = Input {
        mouse_pos: crate::types::Vec2::new(
            40.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width),
            15.0,
        ),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse down / press
    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    assert_eq!(caret_byte(&state), 5);
    assert!(state.is_dragging);
    state.had_keyboard_focus = true;

    // Frame 3: Dragging
    input.mouse_pressed = false;
    input.mouse_pos.x += 24.0;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    assert_eq!(selection_byte(&state), Some(5));
    assert_eq!(caret_byte(&state), 8);

    // Frame 4: Mouse up / release
    input.mouse_down = false;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    assert!(!state.is_dragging);
    assert_eq!(selection_byte(&state), Some(5));
    assert_eq!(caret_byte(&state), 8);
}

#[test]
fn test_double_click_selection_and_drag() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello rust world");

    // Click on "rust" (byte index 8 -> pixel 64)
    let mut input = Input {
        mouse_pos: crate::types::Vec2::new(
            64.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width),
            15.0,
        ),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse down / double-press
    input.mouse_down = true;
    input.mouse_pressed = true;
    input.mouse_click_count = 2;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    // Selection should be "rust" (6 to 10)
    assert_eq!(selection_byte(&state), Some(6));
    assert_eq!(caret_byte(&state), 10);
    assert!(state.is_dragging);
    assert_eq!(state.drag_word_origin, Some((6, 10)));

    // Frame 3: Drag right to "world" (byte index 14 -> pixel 112)
    input.mouse_pressed = false;
    input.mouse_pos.x =
        112.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    // Should select "rust world", so from 6 to 16
    assert_eq!(selection_byte(&state), Some(6)); // original start
    assert_eq!(caret_byte(&state), 16); // end of "world"

    // Frame 4: Drag left to "hello" (byte index 2 -> pixel 16)
    input.mouse_pos.x =
        16.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();
    // Should select "hello rust", so from 10 to 0
    assert_eq!(selection_byte(&state), Some(10)); // original end
    assert_eq!(caret_byte(&state), 0); // start of "hello"
}

#[test]
fn test_double_click_symmetry() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    let mut run_double_click = |x_within_text: f32| -> (Option<usize>, usize) {
        let mut state = TextEditState::new("a b");
        let mut input = Input::default();
        let x_offset = spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width);
        input.mouse_pos = crate::types::Vec2::new(x_within_text + x_offset, 8.0);

        // Frame 1: Hover
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            spec(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut DrawCommands::new(1.0),
        );
        focus_system.end_frame();

        // Frame 2: Double click
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            spec(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut DrawCommands::new(1.0),
        );
        focus_system.end_frame();

        (selection_byte(&state), caret_byte(&state))
    };

    // Click at various positions in "a b"
    // 1. In 'a' [0.0, 8.0) -> should select "a" (0..1)
    // Left extreme: 1.0
    assert_eq!(run_double_click(1.0), (Some(0), 1));
    // Right extreme: 7.0
    assert_eq!(run_double_click(7.0), (Some(0), 1));

    // 2. In ' ' [8.0, 16.0) -> should select " " (1..2)
    // Left half: 9.0
    assert_eq!(run_double_click(9.0), (Some(1), 2));
    // Right half: 15.0
    assert_eq!(run_double_click(15.0), (Some(1), 2));

    // 3. In 'b' [16.0, 24.0) -> should select "b" (2..3)
    // Left extreme: 17.0
    assert_eq!(run_double_click(17.0), (Some(2), 3));
    // Right extreme: 23.0
    assert_eq!(run_double_click(23.0), (Some(2), 3));
}

#[test]
fn test_double_click_after_line_end() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    let mut run_double_click = |text: &str, y_pos: f32| -> (Option<usize>, usize) {
        let mut state = TextEditState::new(text);
        let mut input = Input::default();
        // Click way past the end of the line (e.g. x = 100.0)
        let x_offset = spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width);
        input.mouse_pos = crate::types::Vec2::new(100.0 + x_offset, y_pos);

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Preserve,
            ..spec()
        };

        // Frame 1: Hover
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            edit_spec.clone(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut DrawCommands::new(1.0),
        );
        focus_system.end_frame();

        // Frame 2: Double click
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            edit_spec,
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut DrawCommands::new(1.0),
        );
        focus_system.end_frame();

        (selection_byte(&state), caret_byte(&state))
    };

    // Case 1: Line has trailing \n. Double-clicking after line end should select just the \n character.
    // "hello\n" -> '\n' is at index 5.
    // First line is "hello\n", so we click on line 0 (y = 8.0).
    assert_eq!(run_double_click("hello\n", 8.0), (Some(5), 6));

    // Case 2: Line has trailing \n and is followed by another line.
    // "hello\nworld" -> '\n' is at index 5.
    // First line is "hello\n", click at y = 8.0.
    assert_eq!(run_double_click("hello\nworld", 8.0), (Some(5), 6));

    // Case 3: Line has no trailing \n. Double-clicking after line end should select the trailing word.
    // "hello" -> trailing word is "hello" (0..5).
    assert_eq!(run_double_click("hello", 8.0), (Some(0), 5));

    // Case 4: Line has no trailing \n but is preceded by a newline.
    // "hello\nworld" -> second line is "world" (6..11), click at y = 24.0.
    assert_eq!(run_double_click("hello\nworld", 24.0), (Some(6), 11));
}

#[test]
fn test_triple_click_selects_logical_line() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("alpha\nbravo\ncharlie");
    let mut input = Input::default();
    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 80.0),
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };

    input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    input.mouse_click_count = 3;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(6));
    assert_eq!(caret_byte(&state), 12);
    assert!(state.is_dragging);
    assert_eq!(state.drag_line_origin, Some((6, 12)));
}

#[test]
fn test_triple_click_selection_and_drag() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("alpha\nbravo\ncharlie\ndelta");
    let mut input = Input::default();
    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 90.0),
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };

    input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    input.mouse_click_count = 3;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(6));
    assert_eq!(caret_byte(&state), 12);
    assert_eq!(state.drag_line_origin, Some((6, 12)));

    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 32.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(6));
    assert_eq!(caret_byte(&state), 20);

    input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(12));
    assert_eq!(caret_byte(&state), 0);
}

#[test]
fn test_triple_click_selects_wrapped_logical_line() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrst\nzz");
    let mut input = Input::default();
    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 90.0, 80.0),
        newline_policy: NewlinePolicy::Preserve,
        wrap: true,
        vertical_align: Align::Start,
        ..spec()
    };

    input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 16.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    input.mouse_click_count = 3;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(0));
    assert_eq!(caret_byte(&state), 21);
}

#[test]
fn test_quadruple_click_selects_all() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("alpha\nbravo\ncharlie");
    let mut input = Input::default();
    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 80.0),
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };

    input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    input.mouse_click_count = 4;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(selection_byte(&state), Some(0));
    assert_eq!(caret_byte(&state), state.value.len());
}

#[test]
fn test_caret_blink_reset_on_move() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    set_caret_byte(&mut state, 5);
    state.had_keyboard_focus = true;

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut input = Input::default();

    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(has_caret, "Caret should be visible initially");

    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        TextEditSpec {
            time: 0.6,
            ..spec()
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(!has_caret, "Caret should be hidden during off phase");

    input.text_events.push(TextEvent::CaretLeft {
        shift: false,
        ctrl: false,
    });
    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        TextEditSpec {
            time: 0.6,
            ..spec()
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.last_caret_move_time, 0.6);

    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(
        has_caret,
        "Caret should be visible immediately after moving"
    );

    input.text_events.clear();
    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        TextEditSpec {
            time: 1.0,
            ..spec()
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(has_caret, "Caret should stay visible for 0.5s after moving");

    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        TextEditSpec {
            time: 1.2,
            ..spec()
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(!has_caret, "Caret should hide after 0.5s of idle");
}

#[test]
fn test_caret_blink_reset_on_focus_even_without_caret_move() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    set_caret_byte(&mut state, 5);
    state.selection_anchor = Some(caret_position_at_byte(&state.value, 0));

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        TextEditSpec {
            time: 0.6,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(state.last_caret_move_time, 0.6);
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(
        has_caret,
        "Caret should be visible immediately after gaining focus"
    );
}

#[test]
fn test_caret_blink_reset_on_mouse_focus_even_without_caret_move() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    set_caret_byte(&mut state, 5);

    let mut input = Input {
        mouse_pos: crate::types::Vec2::new(
            40.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width),
            15.0,
        ),
        ..Input::default()
    };

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        TextEditSpec {
            time: 0.6,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    input.mouse_down = false;
    input.mouse_pressed = false;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        TextEditSpec {
            time: 0.6,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(state.last_caret_move_time, 0.6);
    let has_caret = cmds.iter().any(
        |cmd| matches!(cmd, DrawCmd::FillRect {  color, .. } if *color == spec().style.caret_color),
    );
    assert!(
        has_caret,
        "Caret should be visible immediately after gaining focus from a mouse click"
    );
}

#[test]
fn test_word_boundaries() {
    let text = "hello world! 123";
    assert_eq!(word_bounds(text, 0), (0, 5));
    assert_eq!(word_bounds(text, 2), (0, 5));
    assert_eq!(word_bounds(text, 5), (5, 6));
    assert_eq!(word_bounds(text, 6), (6, 11));
    assert_eq!(word_bounds(text, 11), (11, 12));
    assert_eq!(word_bounds(text, 13), (13, 16));

    assert_eq!(find_word_boundary(text, 0, true), 5);
    assert_eq!(find_word_boundary(text, 5, true), 6);
    assert_eq!(find_word_boundary(text, 6, true), 11);

    assert_eq!(find_word_boundary(text, 16, false), 13);
    assert_eq!(find_word_boundary(text, 12, false), 11);
    assert_eq!(find_word_boundary(text, 5, false), 0);
}

#[test]
fn test_logical_line_bounds() {
    let text = "alpha\nbravo\ncharlie";
    assert_eq!(logical_line_bounds(text, 0), (0, 6));
    assert_eq!(logical_line_bounds(text, 8), (6, 12));
    assert_eq!(logical_line_bounds(text, text.len()), (12, text.len()));
    assert_eq!(logical_line_bounds("alpha\n", 6), (6, 6));
}

#[test]
fn test_focus_select_all() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    let input = Input::default();

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert!(state.had_keyboard_focus);
    assert_eq!(selection_byte(&state), Some(0));
    assert_eq!(caret_byte(&state), 11);
}

#[test]
fn test_mouse_focus_no_select_all() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    let mut input = Input {
        mouse_pos: crate::types::Vec2::new(
            40.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width),
            15.0,
        ),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse down / press
    focus_system.begin_frame();
    input.mouse_down = true;
    input.mouse_pressed = true;
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 3: Mouse release
    focus_system.begin_frame();
    input.mouse_pressed = false;
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert!(state.had_keyboard_focus);
    assert_eq!(selection_byte(&state), None);
    assert_eq!(caret_byte(&state), 5);
}

#[test]
fn test_text_edit_click_takes_focus() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");

    let mut input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 15.0),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse pressed
    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Clicking text edit must request focus"
    );
}

#[test]
fn test_text_edit_scrollbar_interaction_scrolls_text_and_takes_focus_without_selection() {
    fn first_glyph_top_left(cmds: &DrawCommands) -> Vec2 {
        cmds.iter()
            .find_map(|cmd| {
                if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                    Some(cmds.glyphs()[glyphs.start].top_left)
                } else {
                    None
                }
            })
            .expect("text edit should draw text glyphs")
    }

    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("one\ntwo\nthree\nfour\nfive");
    set_caret_byte(&mut state, 14);
    set_selection_byte(&mut state, None);

    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 40.0),
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };
    let hover_input = Input {
        mouse_pos: Vec2::new(196.0, 30.0),
        ..Default::default()
    };

    let mut initial_cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &hover_input,
        &mut focus_system,
        &mut text_backend,
        &mut initial_cmds,
    );
    focus_system.end_frame();
    let initial_glyph_pos = first_glyph_top_left(&initial_cmds);

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input {
            mouse_pos: Vec2::new(196.0, 30.0),
            mouse_pressed: true,
            mouse_down: true,
            ..Default::default()
        },
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert!(
        state.scroll.offset.y > 0.0,
        "scrollbar track press should scroll the text edit"
    );
    assert_eq!(caret_byte(&state), 14);
    assert_eq!(selection_byte(&state), None);

    let mut after_press_cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut after_press_cmds,
    );
    focus_system.end_frame();
    let after_press_glyph_pos = first_glyph_top_left(&after_press_cmds);
    assert!(
        after_press_glyph_pos.y < initial_glyph_pos.y,
        "next frame should render text at the scrolled location"
    );

    let mut drag_state = TextEditState::new("one\ntwo\nthree\nfour\nfive");
    set_caret_byte(&mut drag_state, 14);
    set_selection_byte(&mut drag_state, None);
    drag_state.scroll.vert_slider_state.active_part =
        Some(crate::widgets::slider::SliderPart::Segment);
    drag_state.scroll.vert_slider_state.drag_start_mouse_coord = 10.0;
    drag_state.scroll.vert_slider_state.drag_start_value = crate::widgets::SliderValue::Range {
        lower: 0.0,
        upper: 38.0,
    };
    let mut drag_focus_system = FocusSystem::new();

    drag_focus_system.begin_frame();
    let drag_result = raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut drag_state),
        &mut drag_state,
        &Input {
            mouse_pos: Vec2::new(196.0, 25.0),
            mouse_down: true,
            ..Default::default()
        },
        &mut drag_focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    drag_focus_system.end_frame();

    assert_eq!(
        drag_result.cursor_icon,
        Some(crate::output::CursorIcon::Grabbing),
        "dragging the scrollbar thumb should set Grabbing cursor"
    );

    assert_eq!(
        drag_focus_system.current_keyboard_focus(),
        Some(drag_state.focus_id)
    );
    assert!(
        drag_state.scroll.offset.y > 0.0,
        "scrollbar drag should scroll the text edit"
    );
    assert_eq!(caret_byte(&drag_state), 14);
    assert_eq!(selection_byte(&drag_state), None);

    let mut after_drag_cmds = DrawCommands::new(1.0);
    drag_focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut drag_state),
        &mut drag_state,
        &Input::default(),
        &mut drag_focus_system,
        &mut text_backend,
        &mut after_drag_cmds,
    );
    drag_focus_system.end_frame();
    let after_drag_glyph_pos = first_glyph_top_left(&after_drag_cmds);
    assert!(
        after_drag_glyph_pos.y < initial_glyph_pos.y,
        "next frame should render dragged text at the scrolled location"
    );
}

#[test]
fn test_text_edit_clipped_click_does_not_take_focus() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");

    // Mouse is inside the widget rect but outside the clip_rect.
    let clipped_spec = TextEditSpec {
        clip_rect: Some(Rect::new(500.0, 500.0, 200.0, 30.0)),
        ..spec()
    };

    let input = Input {
        mouse_pos: crate::types::Vec2::new(10.0, 15.0),
        mouse_pressed: true,
        mouse_down: true,
        ..Default::default()
    };

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        clipped_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(
        focus_system.current_keyboard_focus(),
        None,
        "Clicking a clipped-away text edit must not take focus"
    );
}

#[test]
fn test_clipboard_actions() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    set_selection_byte(&mut state, Some(6));
    set_caret_byte(&mut state, 11);
    state.had_keyboard_focus = true;

    let mut input = Input::default();
    input.text_events.push(TextEvent::Copy);
    let res = run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Copy(s)) if s == "world"));
    assert_eq!(state.value, "hello world");

    input.text_events.clear();
    input.text_events.push(TextEvent::Cut);
    let res = run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Cut(s)) if s == "world"));
    assert_eq!(state.value, "hello ");
    assert_eq!(selection_byte(&state), None);
    assert_eq!(caret_byte(&state), 6);

    input.text_events.clear();
    input.text_events.push(TextEvent::Paste("rust".to_string()));
    let res = run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    assert!(res.clipboard_action.is_none());
    assert_eq!(state.value, "hello rust");
    assert_eq!(caret_byte(&state), 10);
}

// ── Visual Tests ───────────────────────────────────────────────────────────

#[test]
fn test_text_edit_visual_normal() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(1.0, 1.0, 198.0, 28.0),
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.border.unwrap().color,
                width: spec().style.border.map_or(0.0, |b| b.width),
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('h', 5.0, 19.0),
            ('e', 13.0, 19.0),
            ('l', 21.0, 19.0),
            ('l', 29.0, 19.0),
            ('o', 37.0, 19.0),
        ])
    );
}

#[test]
fn test_text_edit_visual_hover_background() {
    let mut text_backend = TestTextBackend::default();

    {
        let mut state = TextEditState::new("hello");
        let input = Input {
            mouse_pos: Vec2::new(100.0, 15.0),
            ..Input::default()
        };
        let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
        let mut cmds = DrawCommands::new(1.0);

        raw::post_layout_text_edit(
            spec(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        assert!(matches!(
            cmds.iter().next(),
            Some(DrawCmd::FillRect { color, .. }) if *color == spec().style.background_hovered
        ));
    }

    {
        let mut state = TextEditState::new("hello");
        let input = Input {
            // Inside the outer text edit rect, but inside the 1px border,
            // not inside text_edit_scroll_outer_rect().
            mouse_pos: Vec2::new(0.5, 15.0),
            ..Input::default()
        };
        let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
        let mut cmds = DrawCommands::new(1.0);

        raw::post_layout_text_edit(
            spec(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        assert!(matches!(
            cmds.iter().next(),
            Some(DrawCmd::FillRect { color, .. }) if *color == spec().style.background
        ));
    }
}

#[test]
fn test_text_edit_visual_placeholder() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::default();
    let mut cmds = DrawCommands::new(1.0);

    raw::post_layout_text_edit(
        TextEditSpec {
            placeholder: Some("frame_buffer".to_string()),
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert!(cmds.iter().any(|cmd| matches!(
        cmd,
        DrawCmd::GlyphRun { color, .. } if *color == spec().style.placeholder_color
    )));

    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();
    state.had_keyboard_focus = true;
    let mut focused_cmds = DrawCommands::new(1.0);

    raw::post_layout_text_edit(
        TextEditSpec {
            placeholder: Some("frame_buffer".to_string()),
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut focused_cmds,
    );

    assert!(!focused_cmds.iter().any(|cmd| matches!(
        cmd,
        DrawCmd::GlyphRun { color, .. } if *color == spec().style.placeholder_color
    )));
}

#[test]
fn test_text_edit_visual_focused_caret() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();

    state.had_keyboard_focus = true; // ensure state knows

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(1.0, 1.0, 198.0, 28.0),
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(45.0, 7.0, spec().style.caret_width, 16.0),
                color: spec().style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.focus_border.unwrap().color,
                width: spec().style.focus_border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: spec().layer.get_focus_z(),
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('h', 5.0, 19.0),
            ('e', 13.0, 19.0),
            ('l', 21.0, 19.0),
            ('l', 29.0, 19.0),
            ('o', 37.0, 19.0),
        ])
    );
}

#[test]
fn test_text_edit_visual_focused_selection() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();

    state.had_keyboard_focus = true;
    set_selection_byte(&mut state, Some(0));
    set_caret_byte(&mut state, 5);

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(1.0, 1.0, 198.0, 28.0),
            },
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 7.0, 40.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(45.0, 7.0, spec().style.caret_width, 16.0),
                color: spec().style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: spec().style.focus_border.unwrap().color,
                width: spec().style.focus_border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: spec().layer.get_focus_z(),
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('h', 5.0, 19.0),
            ('e', 13.0, 19.0),
            ('l', 21.0, 19.0),
            ('l', 29.0, 19.0),
            ('o', 37.0, 19.0),
        ])
    );
}

#[test]
fn test_text_edit_selection_highlight_respects_horizontal_line_alignment() {
    for (line_align, expected_x) in [(TextLineAlign::Center, 80.0), (TextLineAlign::End, 155.0)] {
        let mut text_backend = TestTextBackend::default();
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(0));
        set_caret_byte(&mut state, 5);

        let input = Input::default();
        let mut cmds = DrawCommands::new(1.0);
        raw::post_layout_text_edit(
            TextEditSpec {
                line_align,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        let has_aligned_selection = cmds.iter().any(|cmd| {
            matches!(
                cmd,
                DrawCmd::FillRect {
                    rect,
                    color,
                    ..
                } if *color == spec().style.select_color
                    && *rect == Rect::new(expected_x, 7.0, 40.0, 16.0)
            )
        });
        assert!(
            has_aligned_selection,
            "{line_align:?} selection highlight should cover the horizontally aligned text"
        );
    }
}

#[test]
fn test_text_edit_center_aligns_non_wrapped_hard_lines_independently() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcd\nx");
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);

    raw::post_layout_text_edit(
        TextEditSpec {
            line_align: TextLineAlign::Center,
            newline_policy: NewlinePolicy::Preserve,
            rect: Rect::new(0.0, 0.0, 200.0, 60.0),
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('a', 84.0, 26.0),
            ('b', 92.0, 26.0),
            ('c', 100.0, 26.0),
            ('d', 108.0, 26.0),
            ('x', 96.0, 42.0),
        ])
    );
}

#[test]
fn test_text_edit_center_aligned_overflow_typing_draws_caret_with_same_frame_scroll() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    let mut state = focused_text_edit_state("abcdefghijklmnopqrstuvwx", &mut focus_system);
    focus_system.begin_frame();
    let end = state.value.len();
    set_caret_byte(&mut state, end);
    state.scroll.offset.x = 0.0;

    let input = Input {
        text_events: vec![TextEvent::Char('y')],
        ..Input::default()
    };

    let edit_spec = TextEditSpec {
        line_align: TextLineAlign::Center,
        rect: Rect::new(0.0, 0.0, 120.0, 30.0),
        ..spec()
    };

    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(state.value, "abcdefghijklmnopqrstuvwxy");
    assert_eq!(caret_byte(&state), state.value.len());
    assert!(state.scroll.offset.x > 0.0);

    let caret_rect = find_caret_rect(&cmds, edit_spec.style.caret_color);
    let scroll_outer_rect = test_text_edit_scroll_outer_rect(&edit_spec);
    assert!(
        caret_rect.x >= scroll_outer_rect.x
            && caret_rect.x + caret_rect.w <= scroll_outer_rect.x + scroll_outer_rect.w,
        "center-aligned typed-character caret reveal should affect same-frame drawing"
    );
}

#[test]
fn test_text_edit_visual_error() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello");
    let mut sp = spec();
    sp.error = true;

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        sp.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: sp.style.error_background,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 4.0, 30.0),
                color: sp.style.error_border.unwrap().color,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(5.0, 1.0, 194.0, 28.0),
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                color: sp.style.error_border.unwrap().color,
                width: spec().style.border.map_or(0.0, |b| b.width),
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('h', 9.0, 19.0),
            ('e', 17.0, 19.0),
            ('l', 25.0, 19.0),
            ('l', 33.0, 19.0),
            ('o', 41.0, 19.0),
        ])
    );

    let mut focus_system = FocusSystem::new();
    let mut focused_state = focused_text_edit_state("hello", &mut focus_system);
    focus_system.begin_frame();
    let mut focused_cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        sp.clone(),
        raw::post_layout_only_pre_layout_result(&mut focused_state),
        &mut focused_state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut focused_cmds,
    );

    let focused_error_border = focused_cmds.iter().find(|cmd| {
        matches!(
            cmd,
            DrawCmd::BorderRect {
                color,
                width,
                placement: crate::BorderPlacement::Inside,
                ..
            } if *color == sp.style.error_border.unwrap().color
                && *width == spec().style.border.map_or(0.0, |b| b.width)
        )
    });
    assert!(
        matches!(focused_error_border, Some(DrawCmd::BorderRect { z, .. }) if *z == sp.layer.get_z()),
        "focused error border should keep the error border on the normal layer"
    );
}

#[test]
fn test_user_rect_not_overridden() {
    use crate::layouts::ManualLayout;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
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
    let mut te_state = TextEditState::default();
    let result = super::text_edit(
        super::TextEditSpec::default_from_theme(&ctx.theme),
        custom_rect,
        &mut te_state,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, custom_rect);
}

#[test]
fn test_text_edit_caret_auto_scrolling() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    // 36 characters. Width = 36 * 8 = 288. Inner scroll width = 288 + 8 = 296.
    // Viewport width = 200 - 2 = 198.
    // Max scroll = 296 - 198 = 98.
    let mut state = TextEditState::new("hello world how are you today doing");
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);

    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);

    // 1. Caret at start (0): scroll should be 0.0
    set_caret_byte(&mut state, 0);
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.scroll.offset.x, 0.0);

    // 2. Caret moves from 23 to 24 (x = 192): exceeds right threshold (198 - 16 = 182)
    // Expected scroll = (192 + 4.0) - 198 + 16 = 14.0
    set_caret_byte(&mut state, 23);
    input.text_events = vec![TextEvent::CaretRight {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 24);
    assert_eq!(state.scroll.offset.x, 14.0);

    // 3. Caret moves from 34 to 35 (x = 280): exceeds right threshold
    // Expected scroll = 280 - 198 + 16 = 98.0, clamped to max_scroll (90.0)
    set_caret_byte(&mut state, 34);
    input.text_events = vec![TextEvent::CaretRight {
        shift: false,
        ctrl: false,
    }];
    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 35);
    assert_eq!(state.scroll.offset.x, 90.0);
    let caret_rect = find_caret_rect(&cmds, spec().style.caret_color);
    assert_eq!(caret_rect.x, 195.0);

    // 4. Move caret left from 3 to 2 (x = 16): below left threshold (98.0 + 16 = 114)
    // Expected scroll = (16 + 4.0) - 16 = 4.0
    set_caret_byte(&mut state, 3);
    input.text_events = vec![TextEvent::CaretLeft {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 2);
    assert_eq!(state.scroll.offset.x, 4.0);

    // 5. Typing at the end of an overflowing line should update scroll and draw the
    // caret with that updated scroll offset in the same frame.
    let mut state =
        focused_text_edit_state("hello world how are you today doing", &mut focus_system);
    let end = state.value.len();
    set_caret_byte(&mut state, end);
    state.scroll.offset.x = 0.0;

    let input = Input {
        text_events: vec![TextEvent::Char('!')],
        ..Input::default()
    };

    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        spec(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(state.value, "hello world how are you today doing!");
    assert_eq!(caret_byte(&state), state.value.len());
    assert!(state.scroll.offset.x > 0.0);

    let caret_rect = find_caret_rect(&cmds, spec().style.caret_color);
    let scroll_outer_rect = test_text_edit_scroll_outer_rect(&spec());
    assert!(
        caret_rect.x >= scroll_outer_rect.x
            && caret_rect.x + caret_rect.w <= scroll_outer_rect.x + scroll_outer_rect.w,
        "typed-character caret reveal should affect same-frame drawing"
    );

    // 6. Caret reveal horizontal scrolling with custom padding_x = 10.0
    let mut edit_spec = spec();
    edit_spec.style.padding_x = 10.0;

    let mut state =
        focused_text_edit_state("hello world how are you today doing", &mut focus_system);
    let end = state.value.len();
    set_caret_byte(&mut state, end - 1);
    state.scroll.offset.x = 0.0;

    let input = Input {
        text_events: vec![TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        }],
        ..Input::default()
    };

    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(caret_byte(&state), end);
    let visible_width = test_text_edit_scroll_outer_rect(&edit_spec).w;
    let text_width = 280.0; // 35 chars * 8px
    let expected_max_scroll = text_width + 2.0 * edit_spec.style.padding_x - visible_width;
    assert_eq!(state.scroll.offset.x, expected_max_scroll);

    // 7. Caret reveal horizontal scrolling with vertical scrollbar gutter and custom padding_x = 10.0
    let mut edit_spec = spec();
    edit_spec.style.padding_x = 10.0;
    edit_spec.newline_policy = NewlinePolicy::Preserve;
    edit_spec.vertical_align = Align::Start;
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 30.0);

    let mut state =
        focused_text_edit_state("x\nhello world how are you today doing", &mut focus_system);
    let end = state.value.len();
    set_caret_byte(&mut state, end - 1);
    state.scroll.offset.x = 0.0;

    let input = Input {
        text_events: vec![TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        }],
        ..Input::default()
    };

    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(caret_byte(&state), end);
    let content_clip_w = cmds
        .iter()
        .find_map(|cmd| match cmd {
            DrawCmd::PushClip { rect } => Some(rect.w),
            _ => None,
        })
        .expect("text edit should push a content clip");

    let longest_line_width = 280.0; // "hello world how are you today doing" is 35 chars * 8px
    let expected_max_scroll_sb =
        longest_line_width + 2.0 * edit_spec.style.padding_x - content_clip_w;
    assert_eq!(state.scroll.offset.x, expected_max_scroll_sb);
}

#[test]
fn test_selection_aware_auto_scrolling() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    // String: "leftwordoverlappingedge middle rightwordoverlappingedge"
    // Character counts:
    // leftwordoverlappingedge: 23 chars (0..23)
    // space: 1 char (23)
    // middle: 6 chars (24..30)
    // space: 1 char (30)
    // rightwordoverlappingedge: 24 chars (31..55)
    //
    // Widths (at 8px per char):
    // leftwordoverlappingedge: 184px (0.0..184.0)
    // middle: 48px (192.0..240.0)
    // rightwordoverlappingedge: 192px (248.0..440.0)
    //
    // Total text width: 440px
    // Inner scroll size: width = 440 + 2 * padding(16) = 472px
    // Viewport width: 200px (from spec())
    // Max scroll: 472 - 200 = 272px
    let mut state = TextEditState::new("leftwordoverlappingedge middle rightwordoverlappingedge");
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);

    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);

    // Warmup frame to establish hover
    input.mouse_pos = Vec2::new(10.0, 15.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Test case 1: Ctrl-A (Select All) should not change the scroll state.
    state.scroll.offset.x = 120.0;
    input.text_events = vec![TextEvent::SelectAll];
    input.mouse_pressed = false;
    input.mouse_click_count = 0;
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.scroll.offset.x, 120.0);

    // Test case 2: Double-clicking the long word on the left should scroll the viewport left
    // just far enough to move the end of that word to the right of the viewport.
    state.scroll.offset.x = 120.0;
    input.text_events = vec![];
    input.mouse_pressed = true;
    input.mouse_click_count = 2;
    input.mouse_pos = Vec2::new(
        136.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width) - 120.0,
        15.0,
    );
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(selection_byte(&state), Some(0));
    assert_eq!(caret_byte(&state), 23);
    assert_eq!(state.scroll.offset.x, 6.0);

    // Test case 3: Double-clicking the long word on the right should scroll the viewport right
    // just far enough to align the start of the word with the left edge of the viewport.
    state.scroll.offset.x = 120.0;
    input.text_events = vec![];
    input.mouse_pressed = true;
    input.mouse_click_count = 2;
    input.mouse_pos = Vec2::new(
        256.0 + spec().style.padding_x + spec().style.border.map_or(0.0, |b| b.width) - 120.0,
        15.0,
    );
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(selection_byte(&state), Some(31));
    assert_eq!(caret_byte(&state), 55);
    assert_eq!(state.scroll.offset.x, 236.0);
}

#[test]
fn test_text_edit_scroll_coordinate_translation() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world how are you today doing");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();
    state.had_keyboard_focus = true;

    // Manually inject a scroll offset of 50.0
    state.scroll.offset.x = 50.0;
    // Selection from index 0 to 5
    set_selection_byte(&mut state, Some(0));
    set_caret_byte(&mut state, 5);

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Find and check the coordinates of Text, Caret FillRect, and Selection FillRect
    let mut found_text = false;
    let mut found_selection = false;

    for cmd in cmds.iter() {
        match cmd {
            DrawCmd::GlyphRun { glyphs, .. } => {
                // Originally, text_x was: outer_rect.x + padding = 1.0 + 4.0 = 5.0
                // Scrolled left by 50.0 -> 5.0 - 50.0 = -45.0
                assert_eq!(cmds.glyphs()[glyphs.start].top_left.x, -45.0);
                found_text = true;
            }
            DrawCmd::FillRect { rect, color, .. } => {
                if *color == spec().style.select_color {
                    // Selection starts at 0 (x = 0) and ends at 5 (x = 40)
                    // Selection rect.x: text_rect.x + start = -45.0 + 0 = -45.0
                    assert_eq!(rect.x, -45.0);
                    assert_eq!(rect.w, 40.0);
                    found_selection = true;
                }
            }
            _ => {}
        }
    }

    assert!(found_text);
    assert!(found_selection);
}

#[test]
fn test_text_edit_click_with_scroll_offset() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world how are you today doing");

    // Manually inject a scroll offset of 50.0
    state.scroll.offset.x = 50.0;

    let mut input = Input {
        mouse_pos: Vec2::new(45.0, 15.0),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse pressed
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;

    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        spec(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Caret should have jumped to 11
    assert_eq!(caret_byte(&state), 11);
}

#[test]
fn test_text_edit_vertical_scroll_coordinate_translation() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("line1\nline2\nline3\nline4");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    state.had_keyboard_focus = true;

    let edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // ── Case 1: Scrolled vertically by 20.0 ────────────────────────────────────
    // Since text (64px) is taller than the viewport (28px), we expect top-alignment.
    // Expected text_y = outer_rect.y + padding - offset.y = 1.0 + 4.0 - 20.0 = -15.0
    state.scroll.offset.y = 20.0;
    set_selection_byte(&mut state, Some(0));
    set_caret_byte(&mut state, 5);

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let mut found_text = false;
    let mut found_selection = false;
    for cmd in cmds.iter() {
        match cmd {
            DrawCmd::GlyphRun { glyphs, .. } => {
                assert_eq!(cmds.glyphs()[glyphs.start].top_left.y, -3.0);
                found_text = true;
            }
            DrawCmd::FillRect { rect, color, .. } => {
                if *color == spec().style.select_color {
                    assert_eq!(rect.y, -15.0);
                    assert_eq!(rect.h, 16.0);
                    found_selection = true;
                }
            }
            _ => {}
        }
    }
    assert!(found_text);
    assert!(found_selection);

    // ── Case 2: Not scrolled (offset = 0.0) ────────────────────────────────────
    // Since text (64px) is taller than the viewport (28px), we expect top-alignment.
    // Expected text_y = outer_rect.y + padding - offset.y = 1.0 + 4.0 - 0.0 = 5.0
    state.scroll.offset.y = 0.0;
    let mut cmds = DrawCommands::new(1.0);
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let mut found_text = false;
    let mut found_selection = false;
    for cmd in cmds.iter() {
        match cmd {
            DrawCmd::GlyphRun { glyphs, .. } => {
                assert_eq!(cmds.glyphs()[glyphs.start].top_left.y, 17.0);
                found_text = true;
            }
            DrawCmd::FillRect { rect, color, .. } => {
                if *color == spec().style.select_color {
                    assert_eq!(rect.y, 5.0);
                    assert_eq!(rect.h, 16.0);
                    found_selection = true;
                }
            }
            _ => {}
        }
    }
    assert!(found_text);
    assert!(found_selection);
}

#[test]
fn test_text_edit_vertical_click_with_scroll_offset() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("line1\nline2\nline3\nline4\nline5\nline6");

    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 50.0),
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // Manually inject a vertical scroll offset of 20.0
    state.scroll.offset.y = 20.0;

    // border = 1.0, padding = 4.0, offset.x = 0.0 => text_x = 5.0.
    // Clicking at x = 5.0, y = 38.0.
    // scroll_outer_rect.h = 48.0, metrics.logical_size.y = 96.0.
    // Since text is taller than the viewport, text_y = 1.0 + 4.0 - 20.0 = -15.0.
    // relative_pos.y = 38.0 - (-15.0) = 53.0, which lands on Line 3 ("line4\n", starts at 18)
    let mut input = Input {
        mouse_pos: Vec2::new(5.0, 38.0),
        ..Default::default()
    };

    // Frame 1: Warmup to establish hover claim
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    // Frame 2: Mouse pressed
    focus_system.begin_frame();
    input.mouse_pressed = true;
    input.mouse_down = true;

    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Caret should have jumped to 18 (start of "line4\n")
    assert_eq!(caret_byte(&state), 18);
}

#[test]
fn test_text_edit_vertical_caret_auto_scrolling() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    // 10 lines of 16px: total height = 160px. Padding = 4px. Inner scroll height = 160 + 8 = 168px.
    // Viewport height = 60 - 2 = 58px.
    // Max scroll = 168 - 58 = 110px.
    let mut state = TextEditState::new("l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9");
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);

    let mut input = Input::default();
    let mut cmds = DrawCommands::new(1.0);

    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 60.0),
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // 1. Caret at start (Line 0, index 0): scroll should be 0.0
    set_caret_byte(&mut state, 0);
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.scroll.offset.y, 0.0);

    // 2. Caret moves down from Line 2 to Line 3 (index 9, y_top = 48.0, height = 16.0): exceeds bottom threshold (58 - 16 = 42)
    // Expected scroll = (64 + 4.0) - 58 + 16 = 26.0
    set_caret_byte(&mut state, 6);
    input.text_events = vec![TextEvent::CaretDown { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.scroll.offset.y, 26.0);

    // 3. Caret moves down to Line 9 (index 27, y_top = 144.0, height = 16.0): exceeds bottom threshold
    // Expected scroll = 160 - 58 + 16 = 118.0, clamped to max_scroll (110.0)
    set_caret_byte(&mut state, 27);
    input.text_events = vec![TextEvent::CaretDown { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.scroll.offset.y, 110.0);

    // 4. Caret moves up from Line 2 to Line 1 (index 3, y_top = 16.0, height = 16.0): below top threshold
    // Expected scroll = (16 + 4.0) - 16 = 4.0
    set_caret_byte(&mut state, 6);
    input.text_events = vec![TextEvent::CaretUp { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(state.scroll.offset.y, 4.0);
}

#[test]
fn test_text_edit_vertical_selection_aware_auto_scrolling() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();

    // 10 lines of 16px: total height = 160px.
    // Viewport height = 58px. Max scroll = 110px.
    let mut state = TextEditState::new("l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9");
    state.had_keyboard_focus = true;
    focus_system.take_keyboard_focus(state.focus_id);

    let mut input = Input {
        mouse_pos: Vec2::new(5.0, 9.0),
        ..Input::default()
    };
    let mut cmds = DrawCommands::new(1.0);

    let edit_spec = TextEditSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 60.0),
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // Warmup frame
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Test case 1: Ctrl-A (Select All) should not change the vertical scroll state.
    state.scroll.offset.y = 50.0;
    input.text_events = vec![TextEvent::SelectAll];
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.scroll.offset.y, 50.0);

    // Test case 2: Double-clicking word on Line 1 (starts at byte index 3, y_top = 16.0)
    // when scroll.y = 20.0.
    // text_y = 5.0 - 20.0 = -15.0.
    // relative_pos.y = 24.0, mouse_pos.y = 24.0 - 15.0 = 9.0 (inside viewport).
    state.scroll.offset.y = 20.0;
    input.text_events = vec![];
    input.mouse_pressed = true;
    input.mouse_click_count = 2;
    input.mouse_pos = Vec2::new(5.0, 9.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(selection_byte(&state), Some(3));
    assert_eq!(caret_byte(&state), 5);
    assert_eq!(state.scroll.offset.y, 4.0);

    // Test case 3: Double-clicking word on Line 9 (starts at byte index 27, y_top = 144.0)
    // when scroll.y = 100.0.
    // text_y = 5.0 - 100.0 = -95.0.
    // relative_pos.y = 152.0, mouse_pos.y = 152.0 - 95.0 = 57.0 (inside viewport).
    state.scroll.offset.y = 100.0;
    input.text_events = vec![];
    input.mouse_pressed = true;
    input.mouse_click_count = 2;
    input.mouse_pos = Vec2::new(5.0, 57.0);
    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(selection_byte(&state), Some(27));
    assert_eq!(caret_byte(&state), 29);
    assert_eq!(state.scroll.offset.y, 110.0);
}

#[test]
fn test_text_edit_caret_movement_with_selection() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    // --- GROUP 1: Horizontal Movement (no Shift) - Collapsing Selection ---

    // 1. Left without Shift collapses a selection to the left/min edge (starts at 4, selection at 2 -> collapses to 2)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 4);
        set_selection_byte(&mut state, Some(2));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 2);
        assert_eq!(selection_byte(&state), None);
    }

    // 2. Left without Shift collapses selection to left edge (starts at 11, selection at 6 -> collapses to 6)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 6);
        assert_eq!(selection_byte(&state), None);
    }

    // 3. Right without Shift collapses a selection to the right/max edge (starts at 2, selection at 4 -> collapses to 4)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 2);
        set_selection_byte(&mut state, Some(4));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 4);
        assert_eq!(selection_byte(&state), None);
    }

    // 4. Right without Shift collapses selection to right edge (starts at 11, selection at 6 -> collapses to 11)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 11);
        assert_eq!(selection_byte(&state), None);
    }

    // --- GROUP 2: Ctrl + Horizontal Movement (no Shift) - Collapsing and Word Movement ---

    // 5. Ctrl+Left collapses selection and moves one word left (starts at 11, selection at 6 -> starts from 6 and moves to 5)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: true,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 5);
        assert_eq!(selection_byte(&state), None);
    }

    // 6. Ctrl+Right collapses selection and moves one word right (starts at 11, selection at 6 -> starts from 11 and moves to 12)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: false,
            ctrl: true,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 12);
        assert_eq!(selection_byte(&state), None);
    }

    // --- GROUP 3: Horizontal Movement (with Shift) - Creating / Extending Selection ---

    // 7. Shift+Left creates selection (starts at 3 -> moves to 2, selection anchor is 3)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 2);
        assert_eq!(selection_byte(&state), Some(3));
    }

    // 8. Shift+Left extends selection (starts at 11, selection at 6 -> moves to 10, selection anchor remains 6)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 10);
        assert_eq!(selection_byte(&state), Some(6));
    }

    // 9. Shift+Right creates selection (starts at 3 -> moves to 4, selection anchor is 3)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 4);
        assert_eq!(selection_byte(&state), Some(3));
    }

    // 10. Shift+Right extends selection (starts at 11, selection at 6 -> moves to 12, selection anchor remains 6)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 12);
        assert_eq!(selection_byte(&state), Some(6));
    }

    // --- GROUP 4: Ctrl + Horizontal Movement (with Shift) - Word Selection ---

    // 11. Ctrl+Shift+Left extends selection by word (starts at 11, selection at 6 -> moves to 6, selection anchor remains 6)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretLeft {
            shift: true,
            ctrl: true,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 6);
        assert_eq!(selection_byte(&state), Some(6));
    }

    // 12. Ctrl+Shift+Right extends selection by word (starts at 11, selection at 6 -> moves to 12, selection anchor remains 6)
    {
        let mut state = TextEditState::new("hello world how are you");
        set_caret_byte(&mut state, 11);
        set_selection_byte(&mut state, Some(6));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: true,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 12);
        assert_eq!(selection_byte(&state), Some(6));
    }

    // --- GROUP 5: Home / End Movement (Shift / no Shift) ---

    // 13. Shift+Home extends selection to line start (starts at 3 -> moves to 0, selection anchor is 3)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretHome {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 0);
        assert_eq!(selection_byte(&state), Some(3));
    }

    // 14. Shift+End extends selection to line end (starts at 3 -> moves to 5, selection anchor is 3)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretEnd {
            shift: true,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 5);
        assert_eq!(selection_byte(&state), Some(3));
    }

    // --- GROUP 6: Vertical Movement (no Shift) - Collapsing selection and Line Movement ---

    // 15. CaretUp (no shift) collapses selection and moves one line up from start boundary.
    // Start: caret 7, selection 1 (min=1, max=7) in "l0\nl1\nl2".
    // Start boundary 1 is on Line 0, so moving up goes to 0 (start of text).
    {
        let mut state = TextEditState::new("l0\nl1\nl2");
        set_caret_byte(&mut state, 7);
        set_selection_byte(&mut state, Some(1));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretUp { shift: false });
        run_raw_text_edit_frame(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 0);
        assert_eq!(selection_byte(&state), None);
    }

    // 16. CaretUp (no shift) collapses selection and moves one line up from start boundary.
    // Start: caret 7, selection 4 (min=4, max=7) in "l0\nl1\nl2".
    // Start boundary 4 is on Line 1, so moving up goes to Line 0 (byte 1).
    {
        let mut state = TextEditState::new("l0\nl1\nl2");
        set_caret_byte(&mut state, 7);
        set_selection_byte(&mut state, Some(4));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretUp { shift: false });
        run_raw_text_edit_frame(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 1);
        assert_eq!(selection_byte(&state), None);
    }

    // 17. CaretDown (no shift) collapses selection and moves one line down from end boundary.
    // Start: caret 1, selection 4 (min=1, max=4) in "l0\nl1\nl2".
    // End boundary 4 is on Line 1, so moving down goes to Line 2 (byte 7).
    {
        let mut state = TextEditState::new("l0\nl1\nl2");
        set_caret_byte(&mut state, 1);
        set_selection_byte(&mut state, Some(4));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input
            .text_events
            .push(TextEvent::CaretDown { shift: false });
        run_raw_text_edit_frame(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 7);
        assert_eq!(selection_byte(&state), None);
    }

    // 18. CaretDown (no shift) collapses selection and moves one line down from end boundary.
    // Start: caret 4, selection 7 (min=4, max=7) in "l0\nl1\nl2".
    // End boundary 7 is on Line 2 (last visual line), so moving down goes to 8 (end of text).
    {
        let mut state = TextEditState::new("l0\nl1\nl2");
        set_caret_byte(&mut state, 4);
        set_selection_byte(&mut state, Some(7));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input
            .text_events
            .push(TextEvent::CaretDown { shift: false });
        run_raw_text_edit_frame(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 8);
        assert_eq!(selection_byte(&state), None);
    }

    // --- GROUP 7: Vertical Movement (with Shift) - Extending Selection ---

    // 19. Shift+Up/Down extends selection and keeps original anchor.
    // Start: caret 5 (at 'e') in "abc\ndef\nghi" (anchor 5).
    // Shift+Up -> caret 1 ('b'), selection Some(5).
    // Shift+Down -> caret 5 ('e'), selection Some(5).
    // Shift+Down again -> caret 9 ('h'), selection Some(5).
    {
        let mut state = TextEditState::new("abc\ndef\nghi");
        set_caret_byte(&mut state, 5);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Preserve,
            ..spec()
        };

        // Shift+Up
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretUp { shift: true });
        run_raw_text_edit_frame(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 1);
        assert_eq!(selection_byte(&state), Some(5));

        // Shift+Down
        input.text_events.clear();
        input.text_events.push(TextEvent::CaretDown { shift: true });
        run_raw_text_edit_frame(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 5);
        assert_eq!(selection_byte(&state), Some(5));

        // Shift+Down again
        input.text_events.clear();
        input.text_events.push(TextEvent::CaretDown { shift: true });
        run_raw_text_edit_frame(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 9);
        assert_eq!(selection_byte(&state), Some(5));
    }

    // --- GROUP 8: Non-Movement / Other Movements clearing selection ---

    // 20. Non-Shift movement after a selection clears the selection (e.g. CaretUp with no shift)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 4);
        set_selection_byte(&mut state, Some(2));
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretUp { shift: false });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), None);
    }

    // --- GROUP 9: Layout / Text Sync and Movement ---

    // 21. Movement after text/layout sync still updates both visual caret and caret_byte consistently (insert character + immediate CaretLeft)
    {
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Char('x'));
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        run_raw_text_edit_frame(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        assert_eq!(state.value, "helxlo");
        assert_eq!(caret_byte(&state), 3);
        assert_eq!(selection_byte(&state), None);
    }
}

#[test]
fn test_newline_policies() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    // A. NewlinePolicy::process unit tests
    {
        let p_preserve = NewlinePolicy::Preserve;
        let p_space = NewlinePolicy::ReplaceWithSpace;
        let p_trim = NewlinePolicy::TrimAfterFirstNewline;

        // no newline
        assert_eq!(p_preserve.process("abc"), "abc");
        assert_eq!(p_space.process("abc"), "abc");
        assert_eq!(p_trim.process("abc"), "abc");

        // LF
        assert_eq!(p_preserve.process("a\nb"), "a\nb");
        assert_eq!(p_space.process("a\nb"), "a b");
        assert_eq!(p_trim.process("a\nb"), "a");

        // CRLF
        assert_eq!(p_preserve.process("a\r\nb"), "a\nb");
        assert_eq!(p_space.process("a\r\nb"), "a b");
        assert_eq!(p_trim.process("a\r\nb"), "a");

        // CR
        assert_eq!(p_preserve.process("a\rb"), "a\nb");
        assert_eq!(p_space.process("a\rb"), "a b");
        assert_eq!(p_trim.process("a\rb"), "a");

        // multiple newlines
        assert_eq!(p_preserve.process("a\nb\nc"), "a\nb\nc");
        assert_eq!(p_space.process("a\nb\nc"), "a b c");
        assert_eq!(p_trim.process("a\nb\nc"), "a");

        // leading newline
        assert_eq!(p_preserve.process("\nabc"), "\nabc");
        assert_eq!(p_space.process("\nabc"), " abc");
        assert_eq!(p_trim.process("\nabc"), "");
    }

    // B. Initial/programmatic state value sanitization
    {
        // Preserve
        let mut state = TextEditState::new("hello\nworld");
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "hello\nworld");

        // ReplaceWithSpace
        let mut state = TextEditState::new("hello\nworld");
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::ReplaceWithSpace,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "hello world");

        // TrimAfterFirstNewline
        let mut state = TextEditState::new("hello\nworld");
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "hello");

        // Normalization check under Preserve
        let mut state = TextEditState::new("hello\r\nworld\rfoo");
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "hello\nworld\nfoo");
    }

    // C. Paste
    {
        // Paste a\nb under Preserve
        let mut state = TextEditState::new("x");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            text_events: vec![TextEvent::Paste("a\nb".to_string())],
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "xa\nb");

        // Paste a\nb under ReplaceWithSpace
        let mut state = TextEditState::new("x");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            text_events: vec![TextEvent::Paste("a\nb".to_string())],
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::ReplaceWithSpace,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "xa b");

        // Paste a\nb under TrimAfterFirstNewline
        let mut state = TextEditState::new("x");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            text_events: vec![TextEvent::Paste("a\nb".to_string())],
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "xa");

        // Paste empty processed under TrimAfterFirstNewline with selection
        let mut state = TextEditState::new("xy");
        set_caret_byte(&mut state, 0);
        set_selection_byte(&mut state, Some(2));
        state.had_keyboard_focus = true;
        let input = Input {
            text_events: vec![TextEvent::Paste("\nab".to_string())], // processes to empty string
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        // Selection remains untouched
        assert_eq!(state.value, "xy");
        assert_eq!(selection_byte(&state), Some(2));
    }

    // D. Enter key
    {
        // Enter under Preserve
        let mut state = TextEditState::new("abc");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            key_pressed_enter: true,
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::Preserve,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "a\nbc");

        // Enter under ReplaceWithSpace
        let mut state = TextEditState::new("abc");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            key_pressed_enter: true,
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::ReplaceWithSpace,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "a bc");

        // Enter under TrimAfterFirstNewline
        let mut state = TextEditState::new("abc");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
        let input = Input {
            key_pressed_enter: true,
            ..Input::default()
        };
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);
        raw::post_layout_text_edit(
            TextEditSpec {
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
                ..spec()
            },
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        assert_eq!(state.value, "abc"); // unchanged
    }
}

#[test]
fn test_caret_up_down_navigation() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    // Initial text: three lines, each 5 characters (excluding newline) -> 8px * 5 = 40px wide per line
    // "line1\nline2\nline3"
    // Line 0: "line1\n" -> byte_start=0, byte_end=6
    // Line 1: "line2\n" -> byte_start=6, byte_end=12
    // Line 2: "line3"   -> byte_start=12, byte_end=17
    let mut state = TextEditState::new("line1\nline2\nline3");
    let mut input = Input::default();
    focus_system.take_keyboard_focus(state.focus_id);

    let edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // Initialize/prepare the layout once
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // 1. Arrow Down from Line 0 to Line 1
    set_caret_byte(&mut state, 5);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretDown { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 11);

    // 2. Arrow Up from Line 1 to Line 0
    input.text_events = vec![TextEvent::CaretUp { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 5);

    // 3. Boundary Condition: Arrow Up on first line
    set_caret_byte(&mut state, 2);
    input.text_events = vec![TextEvent::CaretUp { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 0);

    // 4. Boundary Condition: Arrow Down on last line
    set_caret_byte(&mut state, 14);
    input.text_events = vec![TextEvent::CaretDown { shift: false }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 17);

    // 5. Shift + Arrow Down from Line 0 to Line 1 (extending selection)
    set_caret_byte(&mut state, 2);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretDown { shift: true }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(selection_byte(&state), Some(2));
    assert_eq!(caret_byte(&state), 8);
}

#[test]
fn test_page_up_down_moves_by_outer_scroll_height_whole_lines() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    let mut edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };
    // 1px border on each side leaves a 48px scroll outer height.
    // TestTextBackend lines are 16px tall, so PgUp/PgDown moves three lines.
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

    let mut state = TextEditState::new("line0\nline1\nline2\nline3\nline4\nline5");
    let mut input = Input::default();
    focus_system.take_keyboard_focus(state.focus_id);

    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    set_caret_byte(&mut state, 2);
    set_selection_byte(&mut state, None);
    input.key_pressed_page_down = true;
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 20);
    assert_eq!(selection_byte(&state), None);

    input.key_pressed_page_down = false;
    input.key_pressed_page_up = true;
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 2);

    set_caret_byte(&mut state, 29);
    input.key_pressed_page_up = false;
    input.key_pressed_page_down = true;
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), state.value.len());

    set_caret_byte(&mut state, 9);
    input.key_pressed_page_down = false;
    input.key_pressed_page_up = true;
    run_raw_text_edit_frame(
        edit_spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 0);
}

#[test]
fn test_page_up_down_preserves_caret_x_with_short_target_lines() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    let mut edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

    let mut state = TextEditState::new("000000\n1\n222222\n333\n444444\n5");
    let mut input = Input::default();
    focus_system.take_keyboard_focus(state.focus_id);

    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Line 0 column 5 pages down to line 3. Line 3 has only three
    // characters, so the closest valid x-position is its end.
    set_caret_byte(&mut state, 5);
    set_selection_byte(&mut state, None);
    input.key_pressed_page_down = true;
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 19);

    // Line 4 column 5 pages up to line 1. Line 1 has one character, so
    // preserving x clamps to that line's end.
    set_caret_byte(&mut state, 25);
    input.key_pressed_page_down = false;
    input.key_pressed_page_up = true;
    run_raw_text_edit_frame(
        edit_spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(caret_byte(&state), 8);
}

#[test]
fn test_shift_page_down_extends_selection() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    let mut edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

    let mut state = TextEditState::new("line0\nline1\nline2\nline3\nline4\nline5");
    let mut input = Input::default();
    focus_system.take_keyboard_focus(state.focus_id);

    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    set_caret_byte(&mut state, 2);
    set_selection_byte(&mut state, None);
    input.key_pressed_page_down = true;
    input.modifier_shift = true;
    run_raw_text_edit_frame(
        edit_spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(selection_byte(&state), Some(2));
    assert_eq!(caret_byte(&state), 20);
}

#[test]
fn test_text_edit_claims_page_keys_inside_outer_scroll_area() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);
    let mut outer_scroll = crate::widgets::scroll_area::ScrollState::default();
    let mut state = TextEditState::new("line0\nline1\nline2\nline3\nline4\nline5");
    set_caret_byte(&mut state, 2);
    focus_system.take_keyboard_focus(state.focus_id);

    let mut edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        vertical_align: Align::Start,
        ..spec()
    };
    edit_spec.rect = Rect::new(0.0, 0.0, 190.0, 50.0);

    let outer_spec = crate::widgets::scroll_area::raw::ScrollAreaSpec {
        rect: Rect::new(0.0, 0.0, 200.0, 80.0),
        horizontal: crate::widgets::scroll_area::ScrollAxis {
            extent: crate::widgets::scroll_area::ScrollExtent::FIT,
            vis: crate::widgets::ScrollbarVisibility::Auto,
        },
        vertical: crate::widgets::scroll_area::ScrollAxis {
            extent: crate::widgets::scroll_area::ScrollExtent::SCROLL,
            vis: crate::widgets::ScrollbarVisibility::Always,
        },
        clip_rect: None,
        time: 0.0,
        style: crate::widgets::scroll_area::ScrollAreaStyle {
            scrollbar_width: 10.0,
            scrollbar_style: crate::widgets::SliderStyle::scrollbar_from_theme(
                &crate::theme::Theme::default(),
            ),
            corner_color: Some(crate::theme::Theme::default().paper_elev),
        },
        layer: Layer::default(),
        keyboard_focusable: true,
    };

    for _ in 0..2 {
        let input = Input {
            key_pressed_page_down: true,
            ..Default::default()
        };

        focus_system.begin_frame();
        let outer_token = crate::widgets::scroll_area::raw::begin_scroll_area(
            outer_spec.clone(),
            crate::widgets::scroll_area::raw::ScrollAreaPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut outer_scroll,
            &input,
            &mut focus_system,
            &mut cmds,
        )
        .token;

        raw::post_layout_text_edit(
            edit_spec.clone(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        crate::widgets::scroll_area::raw::end_scroll_area(
            outer_token,
            Vec2::new(200.0, 1000.0),
            &mut outer_scroll,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
    }

    assert_eq!(outer_scroll.offset.y, 0.0);
    assert_ne!(caret_byte(&state), 2);
}

// ── Home / End navigation ───────────────────────────────────────────────────
//
// Text used throughout: "line1\nline2\nline3"
//   Line 0: bytes  0 ..  6  ("line1\n")
//   Line 1: bytes  6 .. 12  ("line2\n")
//   Line 2: bytes 12 .. 17  ("line3")
//
// Expected behaviour
// ------------------
// Home (ctrl=false): move caret to the first byte of the *current* line.
// End  (ctrl=false): move caret to the last byte of the *current* line
//                    (i.e. just before '\n', or to value.len() on the last line).
// Home (ctrl=true) : move caret to byte 0 (start of the whole string).
// End  (ctrl=true) : move caret to value.len() (end of the whole string).
// Adding Shift extends the selection from the *old* caret position.
//
// NOTE: these tests are expected to FAIL with the current implementation.
// CaretHome / CaretEnd today always jump to 0 / value.len() irrespective of
// `ctrl`, and they have no line-awareness at all.
#[test]
fn test_home_end_multiline() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut cmds = DrawCommands::new(1.0);

    let mut state = TextEditState::new("line1\nline2\nline3");
    state.had_keyboard_focus = true;
    focus_system.begin_frame();
    focus_system.take_keyboard_focus(state.focus_id);

    let edit_spec = TextEditSpec {
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };

    // Warm-up frame so the widget knows the layout.
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let mut input = Input::default();

    // ── 1. Home (ctrl=false) from middle of Line 1 → start of Line 1 ──────
    set_caret_byte(&mut state, 9); // "line2|2\n" → inside Line 1
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretHome {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        6,
        "Home (no ctrl) from line 1 mid should move to start of line 1 (byte 6)"
    );
    assert_eq!(selection_byte(&state), None);

    // ── 2. End (ctrl=false) from middle of Line 1 → end of Line 1 ──────────
    set_caret_byte(&mut state, 9); // restore to mid-line-1
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretEnd {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        11,
        "End (no ctrl) from line 1 mid should move to end of line 1 (byte 11, before \\n)"
    );
    assert_eq!(selection_byte(&state), None);

    // ── 3. Home (ctrl=false) on Line 0 already at start → stays at 0 ───────
    set_caret_byte(&mut state, 0);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretHome {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        0,
        "Home (no ctrl) at byte 0 should stay at 0"
    );

    // ── 4. End (ctrl=false) on last line → value.len() ─────────────────────
    set_caret_byte(&mut state, 14); // inside "line3"
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretEnd {
        shift: false,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        17,
        "End (no ctrl) on last line should move to value.len() (byte 17)"
    );

    // ── 5. Shift+Home (ctrl=false) extends selection to start of line ──────
    set_caret_byte(&mut state, 9); // mid-line-1
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretHome {
        shift: true,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        selection_byte(&state),
        Some(9),
        "Shift+Home (no ctrl) should anchor selection at old caret (9)"
    );
    assert_eq!(
        caret_byte(&state),
        6,
        "Shift+Home (no ctrl) should move caret to start of current line (6)"
    );

    // ── 6. Shift+End (ctrl=false) extends selection to end of line ──────────
    set_caret_byte(&mut state, 9); // mid-line-1
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretEnd {
        shift: true,
        ctrl: false,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        selection_byte(&state),
        Some(9),
        "Shift+End (no ctrl) should anchor selection at old caret (9)"
    );
    assert_eq!(
        caret_byte(&state),
        11,
        "Shift+End (no ctrl) should move caret to end of current line (11)"
    );

    // ── 7. Home (ctrl=true) from mid-string → byte 0 ───────────────────────
    set_caret_byte(&mut state, 9);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretHome {
        shift: false,
        ctrl: true,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        0,
        "Home (ctrl=true) should always move to byte 0"
    );
    assert_eq!(selection_byte(&state), None);

    // ── 8. End (ctrl=true) from mid-string → value.len() ───────────────────
    set_caret_byte(&mut state, 9);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretEnd {
        shift: false,
        ctrl: true,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        caret_byte(&state),
        17,
        "End (ctrl=true) should always move to value.len()"
    );
    assert_eq!(selection_byte(&state), None);

    // ── 9. Shift+Ctrl+Home extends selection to byte 0 ──────────────────────
    set_caret_byte(&mut state, 9);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretHome {
        shift: true,
        ctrl: true,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        selection_byte(&state),
        Some(9),
        "Shift+Ctrl+Home should anchor selection at old caret (9)"
    );
    assert_eq!(
        caret_byte(&state),
        0,
        "Shift+Ctrl+Home should move caret to byte 0"
    );

    // ── 10. Shift+Ctrl+End extends selection to value.len() ─────────────────
    set_caret_byte(&mut state, 9);
    set_selection_byte(&mut state, None);
    input.text_events = vec![TextEvent::CaretEnd {
        shift: true,
        ctrl: true,
    }];
    run_raw_text_edit_frame(
        edit_spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert_eq!(
        selection_byte(&state),
        Some(9),
        "Shift+Ctrl+End should anchor selection at old caret (9)"
    );
    assert_eq!(
        caret_byte(&state),
        17,
        "Shift+Ctrl+End should move caret to value.len()"
    );
}

#[test]
fn test_text_edit_visual_multiline_selection() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello\nworld");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();

    state.had_keyboard_focus = true;
    set_selection_byte(&mut state, Some(3)); // 'l' in "hello"
    set_caret_byte(&mut state, 9); // 'r' in "world"

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 100.0),
            newline_policy: NewlinePolicy::Preserve,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                color: spec().style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(1.0, 1.0, 198.0, 98.0),
            },
            // Selection Rect for Line 0: "lo\n"
            DrawCmd::FillRect {
                rect: Rect::new(29.0, 34.0, 24.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            // Selection Rect for Line 1: "wo"
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 50.0, 24.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..10,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(29.0, 50.0, spec().style.caret_width, 16.0),
                color: spec().style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                color: spec().style.focus_border.unwrap().color,
                width: spec().style.focus_border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: spec().layer.get_focus_z(),
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('h', 5.0, 46.0),
            ('e', 13.0, 46.0),
            ('l', 21.0, 46.0),
            ('l', 29.0, 46.0),
            ('o', 37.0, 46.0),
            ('w', 5.0, 62.0),
            ('o', 13.0, 62.0),
            ('r', 21.0, 62.0),
            ('l', 29.0, 62.0),
            ('d', 37.0, 62.0),
        ])
    );
}

#[test]
fn test_text_edit_selection_highlights_collapsed_trailing_space_affordance() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("a b");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();

    state.had_keyboard_focus = true;
    set_selection_byte(&mut state, Some(0));
    set_caret_byte(&mut state, 2);

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 100.0),
            newline_policy: NewlinePolicy::Preserve,
            wrap: true,
            vertical_align: Align::Start,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let has_collapsed_space_affordance = cmds.iter().any(|cmd| {
        matches!(
            cmd,
            DrawCmd::FillRect {
                rect,
                color,
                ..
            } if *color == spec().style.select_color
                && *rect == Rect::new(5.0, 5.0, 16.0, 16.0)
        )
    });

    assert!(
        has_collapsed_space_affordance,
        "selection highlight should extend past line.logical_width for the collapsed trailing space"
    );
}

#[test]
fn test_text_edit_visual_multiline_selection_three_lines() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("one\ntwo\nthree");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    focus_system.begin_frame();

    state.had_keyboard_focus = true;
    set_selection_byte(&mut state, Some(2)); // 'e' in "one"
    set_caret_byte(&mut state, 10); // 'r' in "three"

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 100.0),
            newline_policy: NewlinePolicy::Preserve,
            ..spec()
        },
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                color: spec().style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(1.0, 1.0, 198.0, 98.0),
            },
            // Selection Rect for Line 0: "e\n"
            DrawCmd::FillRect {
                rect: Rect::new(21.0, 26.0, 16.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            // Selection Rect for Line 1: "two\n"
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 42.0, 32.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            // Selection Rect for Line 2: "th"
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 58.0, 16.0, 16.0),
                color: spec().style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..11,
                color: spec().style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(21.0, 58.0, spec().style.caret_width, 16.0),
                color: spec().style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                color: spec().style.focus_border.unwrap().color,
                width: spec().style.focus_border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: spec().layer.get_focus_z(),
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        glyphs(&[
            ('o', 5.0, 38.0),
            ('n', 13.0, 38.0),
            ('e', 21.0, 38.0),
            ('t', 5.0, 54.0),
            ('w', 13.0, 54.0),
            ('o', 21.0, 54.0),
            ('t', 5.0, 70.0),
            ('h', 13.0, 70.0),
            ('r', 21.0, 70.0),
            ('e', 29.0, 70.0),
            ('e', 37.0, 70.0),
        ])
    );
}

#[test]
fn test_text_edit_caret_up_down_width_mismatch() {
    // This test verifies that CaretUp and CaretDown navigation use the correct layout width.
    // Under the layout width mismatch bug, CaretUp and CaretDown events prepare their text layout
    // using the widget's physical border width (spec.rect.w - 2.0 * spec.style.border_width),
    // ignoring any error stripe subtraction or dynamic maximum logical size boundaries used
    // during the draw/render phase.
    // This mismatch leads to incorrect wrapping or line calculations during navigation, causing
    // the caret to jump unexpectedly or land on wrong characters compared to what is rendered.

    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghij");
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    // Configure style to enable wrapping
    let mut spec_error = spec();
    spec_error.rect = Rect::new(0.0, 0.0, 52.0, 100.0); // 50px width content boundary + 2px borders
    spec_error.style.border = Some(Stroke::new(Color::BLACK, 1.0));
    spec_error.style.padding_x = 0.0;
    spec_error.style.padding_y = 0.0;
    spec_error.wrap = true;
    spec_error.error = true;
    spec_error.style.error_stripe_width = 4.0;

    // With spec.error = true and spec.rect.w = 52.0:
    // - Correct layout width is metrics.logical_size.x.max(scroll_outer_rect.w)
    //   where logical_size.x = 40.0, scroll_outer_rect.w = 46.0 (since it wraps at 46.0px max_width).
    //   So correct width is 46.0.
    // - Line 0: "abcde" (bytes 0..5), Line 1: "fghij" (bytes 5..10).
    // - Start caret at index 8 ('i', Line 1). CaretUp should move to index 3 ('d', Line 0).
    // - Buggy event handler layout width is 50.0 (ignoring error stripe). Fits 6 characters per line.
    //   Visual lines under buggy handler: Line 0: "abcdef" (bytes 0..6), Line 1: "ghij" (bytes 6..10).
    //   Under the buggy handler, CaretUp thinks index 8 is column 2 on Line 1, moving it up to
    //   index 2 on Line 0.

    // --- Test CaretUp Mismatch ---
    // Start caret at index 8 ('i'). Since the correct layout has 46.0 width, it wraps as
    // "abcde" and "fghij", so CaretUp should move it to index 3.
    // Due to the bug (layout width 50.0), CaretUp thinks index 8 is on Line 1 of the 50.0 layout,
    // and moves it up to index 2.
    set_caret_byte(&mut state, 8);
    state.had_keyboard_focus = true;
    focus_system.begin_frame();

    let mut input = Input::default();
    input.text_events.push(TextEvent::CaretUp { shift: false });

    raw::post_layout_text_edit(
        spec_error,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );

    assert_eq!(
        caret_byte(&state),
        3,
        "CaretUp should move caret to index 3 under correct wrapped layout width"
    );
}

#[test]
fn test_text_edit_alignment_combinations() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let input = Input::default();

    // 1. Top-Left (Start, Start)
    {
        let mut state = TextEditState::new("hello");
        let mut cmds = DrawCommands::new(1.0);
        let edit_spec = TextEditSpec {
            vertical_align: Align::Start,
            line_align: TextLineAlign::Start,
            ..spec()
        };
        raw::post_layout_text_edit(
            edit_spec,
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        // Inset by border (1.0) and padding (4.0).
        // scroll_outer_rect = (1.0, 1.0, 198.0, 28.0).
        // Align::Start text_y = scroll_outer_rect.y + padding = 1.0 + 4.0 = 5.0.
        let has_text = cmds.iter().any(|cmd| {
            if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                cmds.glyphs()[glyphs.start].top_left.y == 17.0
            } else {
                false
            }
        });
        assert!(has_text, "Align::Start text Y should be 5.0");
    }

    // 2. Center-Center (Center, Center)
    {
        let mut state = TextEditState::new("hello");
        let mut cmds = DrawCommands::new(1.0);
        let edit_spec = TextEditSpec {
            vertical_align: Align::Center,
            line_align: TextLineAlign::Center,
            ..spec()
        };
        raw::post_layout_text_edit(
            edit_spec,
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        // Align::Center text_y = scroll_outer_rect.y + (28.0 - 16.0)/2.0 = 1.0 + 6.0 = 7.0.
        let has_text = cmds.iter().any(|cmd| {
            if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                cmds.glyphs()[glyphs.start].top_left.y == 19.0
            } else {
                false
            }
        });
        assert!(has_text, "Align::Center text Y should be 7.0");
    }

    // 3. Bottom-Right (End, End)
    {
        let mut state = TextEditState::new("hello");
        let mut cmds = DrawCommands::new(1.0);
        let edit_spec = TextEditSpec {
            vertical_align: Align::End,
            line_align: TextLineAlign::End,
            ..spec()
        };
        raw::post_layout_text_edit(
            edit_spec,
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );

        // Align::End text_y = scroll_outer_rect.y + scroll_outer_rect.h - padding - logical_size.y
        // = 1.0 + 28.0 - 4.0 - 16.0 = 9.0.
        let has_text = cmds.iter().any(|cmd| {
            if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                cmds.glyphs()[glyphs.start].top_left.y == 21.0
            } else {
                false
            }
        });
        assert!(has_text, "Align::End text Y should be 9.0");
    }

    // 4. Hit-testing: verify that clicking on aligned text maps to correct caret index
    // Let's test bottom-aligned text (vertical_align = Align::End).
    // Since Y is 9.0, clicking at Y = 17.0 (middle of the line) should hit-test correctly.
    {
        let mut state = TextEditState::new("hello");
        let mut cmds = DrawCommands::new(1.0);
        let edit_spec = TextEditSpec {
            vertical_align: Align::End,
            line_align: TextLineAlign::Start,
            ..spec()
        };

        // Text is placed at x = 5.0 (scroll_outer_rect.x + padding = 1.0 + 4.0).
        // Character width in TestTextBackend is 8.0px.
        // Click on 'l' (index 3). x offset should be around 5.0 + 3 * 8.0 = 29.0.
        // Let's click at x = 31.0 (between 29.0 and 37.0).
        // Click Y should be in the line: text Y is 9.0, height is 16.0, so middle is 17.0.
        let mut click_input = Input {
            mouse_pos: Vec2::new(31.0, 17.0),
            ..Default::default()
        };

        // Frame 1: Warmup to claim hover
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            edit_spec.clone(),
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &click_input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 2: Mouse click
        click_input.mouse_pressed = true;
        click_input.mouse_down = true;
        focus_system.begin_frame();
        raw::post_layout_text_edit(
            edit_spec,
            raw::post_layout_only_pre_layout_result(&mut state),
            &mut state,
            &click_input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            caret_byte(&state),
            3,
            "Hit testing should resolve correctly to index 3"
        );
    }
}

#[test]
fn test_prepare_text_edit_layout_applies_wrapped_gutter_to_layout_width() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0);
    edit_spec.wrap = true;
    edit_spec.style.border = Some(Stroke::new(Color::BLACK, 1.0));
    edit_spec.style.padding_x = 4.0;
    edit_spec.style.padding_y = 4.0;

    // scroll_outer_rect: x=1.0, y=1.0, w=98.0, h=28.0.
    // Wrapped editors reserve the vertical scrollbar gutter before layout.
    // available text width = 98.0 - 2 * 4.0 - 5.0 = 85.0.
    // In characters: 85.0 / 8.0 = 10.625 -> 10 characters.

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);

    let prepared = super::raw::prepare_text_edit_layout(
        "abcdefghijk",
        &edit_spec,
        text_style,
        &mut text_backend,
    );
    let metrics = prepared.layout.metrics();
    assert_eq!(metrics.line_count, 2);
    assert_eq!(metrics.lines[0].byte_end - metrics.lines[0].byte_start, 10);
    assert_eq!(metrics.lines[1].byte_end - metrics.lines[1].byte_start, 1);
    assert!(prepared.reserved_vertical_scrollbar);
    assert_eq!(prepared.layout_width, 85.0);
    assert_eq!(prepared.layout_height, 32.0);
    assert_eq!(prepared.inner_scroll_size, Vec2::new(88.0, 40.0));
}

#[test]
fn test_should_reserve_vertical_scrollbar_gutter_counts_non_wrapped_lines() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    edit_spec.wrap = false;
    edit_spec.style.border = Some(Stroke::new(Color::BLACK, 1.0));
    edit_spec.style.padding_x = 4.0;
    edit_spec.style.padding_y = 4.0;
    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);

    let scroll_outer_rect = edit_spec
        .rect
        .inset(edit_spec.style.border.map_or(0.0, |b| b.width));
    assert!(
        !super::raw::should_reserve_vertical_scrollbar_gutter(
            "one",
            &edit_spec,
            text_style,
            &mut text_backend,
            scroll_outer_rect,
        ),
        "single-line text should not reserve when one line fits"
    );
    assert!(
        !super::raw::should_reserve_vertical_scrollbar_gutter(
            "one\ntwo",
            &edit_spec,
            text_style,
            &mut text_backend,
            scroll_outer_rect,
        ),
        "non-wrapped multiline text should not reserve when all hard lines fit"
    );
    assert!(
        super::raw::should_reserve_vertical_scrollbar_gutter(
            "one\ntwo\nthree",
            &edit_spec,
            text_style,
            &mut text_backend,
            scroll_outer_rect,
        ),
        "non-wrapped multiline text should reserve when hard lines overflow"
    );

    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 20.0);
    let short_scroll_outer_rect = edit_spec
        .rect
        .inset(edit_spec.style.border.map_or(0.0, |b| b.width));
    assert!(
        super::raw::should_reserve_vertical_scrollbar_gutter(
            "one",
            &edit_spec,
            text_style,
            &mut text_backend,
            short_scroll_outer_rect,
        ),
        "single-line text should reserve when even one line does not fit"
    );

    edit_spec.wrap = true;
    let wrapped_text_style =
        super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    assert!(
        super::raw::should_reserve_vertical_scrollbar_gutter(
            "one",
            &edit_spec,
            wrapped_text_style,
            &mut text_backend,
            scroll_outer_rect,
        ),
        "wrapped text reserves conservatively before layout"
    );
}

#[test]
fn test_prepare_text_edit_layout_non_wrapped_gutter_rules() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0);
    edit_spec.wrap = false;
    edit_spec.style.border = Some(Stroke::new(Color::BLACK, 1.0));
    edit_spec.style.padding_x = 4.0;
    edit_spec.style.padding_y = 4.0;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);

    let prepared = super::raw::prepare_text_edit_layout(
        "abcdefghijk",
        &edit_spec,
        text_style,
        &mut text_backend,
    );
    let metrics = prepared.layout.metrics();
    assert_eq!(metrics.line_count, 1);
    assert!(!prepared.reserved_vertical_scrollbar);
    assert_eq!(prepared.layout_width, 98.0);
    assert_eq!(prepared.inner_scroll_size, Vec2::new(96.0, 24.0));

    let prepared =
        super::raw::prepare_text_edit_layout("abc\ndef", &edit_spec, text_style, &mut text_backend);
    let metrics = prepared.layout.metrics();
    assert_eq!(metrics.line_count, 2);
    assert!(prepared.reserved_vertical_scrollbar);
    assert_eq!(prepared.layout_width, 98.0);

    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 50.0);
    let prepared =
        super::raw::prepare_text_edit_layout("abc\ndef", &edit_spec, text_style, &mut text_backend);
    assert_eq!(prepared.layout.metrics().line_count, 2);
    assert!(!prepared.reserved_vertical_scrollbar);
    assert_eq!(prepared.layout_width, 98.0);

    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 20.0);
    let prepared = super::raw::prepare_text_edit_layout(
        "abcdefghijk",
        &edit_spec,
        text_style,
        &mut text_backend,
    );
    assert_eq!(prepared.layout.metrics().line_count, 1);
    assert!(prepared.reserved_vertical_scrollbar);
    assert_eq!(prepared.layout_width, 98.0);
}

#[test]
fn test_prepare_text_edit_layout_offsets_short_unwrapped_center_block() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = TextEditSpec {
        line_align: TextLineAlign::Center,
        ..spec()
    };
    edit_spec.wrap = false;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared =
        super::raw::prepare_text_edit_layout("hello", &edit_spec, text_style, &mut text_backend);

    assert_eq!(prepared.layout.metrics().logical_size.x, 40.0);
    assert_eq!(prepared.block_align_offset_x, 75.0);
    assert_eq!(prepared.layout.metrics().lines[0].logical_x, 0.0);
}

#[test]
fn test_prepare_text_edit_layout_offsets_short_unwrapped_end_block() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = TextEditSpec {
        line_align: TextLineAlign::End,
        ..spec()
    };
    edit_spec.wrap = false;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared =
        super::raw::prepare_text_edit_layout("hello", &edit_spec, text_style, &mut text_backend);

    assert_eq!(prepared.layout.metrics().logical_size.x, 40.0);
    assert_eq!(prepared.block_align_offset_x, 150.0);
    assert_eq!(prepared.layout.metrics().lines[0].logical_x, 0.0);
}

#[test]
fn test_prepare_text_edit_layout_does_not_offset_wide_unwrapped_block() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = TextEditSpec {
        line_align: TextLineAlign::End,
        ..spec()
    };
    edit_spec.wrap = false;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared = super::raw::prepare_text_edit_layout(
        "abcdefghijklmnopqrstuvwxyz0123",
        &edit_spec,
        text_style,
        &mut text_backend,
    );

    assert_eq!(prepared.layout.metrics().logical_size.x, 240.0);
    assert_eq!(prepared.block_align_offset_x, 0.0);
    assert_eq!(prepared.inner_scroll_size.x, 248.0);
}

#[test]
fn test_prepare_text_edit_layout_aligns_unwrapped_lines_then_offsets_block() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = TextEditSpec {
        line_align: TextLineAlign::Center,
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };
    edit_spec.wrap = false;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared =
        super::raw::prepare_text_edit_layout("abcd\nx", &edit_spec, text_style, &mut text_backend);
    let metrics = prepared.layout.metrics();

    assert_eq!(metrics.logical_size.x, 32.0);
    assert_eq!(metrics.lines[0].logical_x, 0.0);
    assert_eq!(metrics.lines[1].logical_x, 12.0);
    assert_eq!(prepared.block_align_offset_x, 76.5);
}

#[test]
fn test_prepare_text_edit_layout_wrap_uses_bounded_line_alignment_without_block_offset() {
    let mut text_backend = TestTextBackend::default();
    let mut edit_spec = TextEditSpec {
        line_align: TextLineAlign::Center,
        newline_policy: NewlinePolicy::Preserve,
        ..spec()
    };
    edit_spec.wrap = true;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared =
        super::raw::prepare_text_edit_layout("a\nabcd", &edit_spec, text_style, &mut text_backend);
    let metrics = prepared.layout.metrics();

    assert_eq!(prepared.block_align_offset_x, 0.0);
    assert_eq!(metrics.lines[0].logical_x, 88.5);
    assert_eq!(metrics.lines[1].logical_x, 76.5);
}

#[test]
fn test_text_edit_error_vertical_scrollbar_layout_and_hit_test() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("line1\nline2\nline3\nline4\nline5");
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0);
    edit_spec.error = true;
    edit_spec.newline_policy = NewlinePolicy::Preserve;

    state.scroll.offset.y = 16.0;

    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    assert!(
        cmds.iter().any(|cmd| matches!(
            cmd,
            DrawCmd::PushClip {
                rect: Rect {
                    x: 5.0,
                    y: 1.0,
                    w: 189.0,
                    h: 38.0
                }
            }
        )),
        "content clip should account for border, error stripe, and vertical scrollbar"
    );

    assert!(
        cmds.iter().any(|cmd| {
            if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                cmds.glyphs()[glyphs.start].top_left == Vec2::new(9.0, 1.0)
            } else {
                false
            }
        }),
        "text origin should be offset by the error stripe and scroll amount"
    );

    assert!(
        cmds.iter().any(|cmd| {
            if let DrawCmd::FillRect { rect, .. } = cmd {
                rect.x == 194.0 && rect.w == 5.0 && rect.y >= 1.0 && rect.bottom() <= 39.0
            } else {
                false
            }
        }),
        "vertical scrollbar should stay tucked against the right edge"
    );

    let mut click_input = Input {
        mouse_pos: Vec2::new(9.0, 37.0),
        ..Default::default()
    };

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &click_input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    click_input.mouse_pressed = true;
    click_input.mouse_down = true;

    focus_system.begin_frame();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &click_input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(
        caret_byte(&state),
        18,
        "hit testing should use the error stripe-adjusted, scrolled text rect"
    );
}

#[test]
fn test_text_edit_visual_vertical_scrollbar() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("one\ntwo\nthree\nfour\nfive"); // 5 lines, height 80px
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0); // height 40px -> viewport h = 38px
    edit_spec.newline_policy = NewlinePolicy::Preserve;

    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Find if a vertical scrollbar track/thumb was drawn.
    // Slider/scrollbar drawing uses FillRect for track/thumb, and since the viewport height
    // is 38px, and text height is 80px + padding = 88px, it overflows.
    // Specifically, let's assert that content bounds width in PushClip is shrunk to 193.0 (200 - 2 border - 5 scrollbar).
    let has_shrunk_clip = cmds.iter().any(|cmd| {
        if let DrawCmd::PushClip { rect } = cmd {
            rect.w == 193.0
        } else {
            false
        }
    });
    assert!(
        has_shrunk_clip,
        "The clip rect width should be shrunk to 193.0 to accommodate the vertical scrollbar"
    );

    // The vertical track has width 5.0 and is placed at x = 194.0
    let has_vertical_track = cmds.iter().any(|cmd| {
        if let DrawCmd::FillRect { rect, .. } = cmd {
            rect.x == 194.0 && rect.w == 5.0
        } else {
            false
        }
    });
    assert!(has_vertical_track, "Should render vertical scrollbar track");
}

#[test]
fn test_text_edit_visual_horizontal_scrollbar() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz0123"); // 30 chars = 240px wide
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0); // width 200px -> viewport w = 198px

    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Since horizontal scrollbar is triggered, the content height is shrunk by 5px (from 38px to 33px)
    let has_shrunk_clip = cmds.iter().any(|cmd| {
        if let DrawCmd::PushClip { rect } = cmd {
            rect.h == 33.0
        } else {
            false
        }
    });
    assert!(
        has_shrunk_clip,
        "The clip rect height should be shrunk to 33.0 to accommodate the horizontal scrollbar"
    );

    // The horizontal track has height 5.0 and is placed at y = 34.0
    let has_horizontal_track = cmds.iter().any(|cmd| {
        if let DrawCmd::FillRect { rect, .. } = cmd {
            rect.y == 34.0 && rect.h == 5.0
        } else {
            false
        }
    });
    assert!(
        has_horizontal_track,
        "Should render horizontal scrollbar track"
    );
}

#[test]
fn test_text_edit_wrapping() {
    let mut text_backend = TestTextBackend::default();
    let state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // available width = 90px without scrollbar (11 chars)
    edit_spec.wrap = true;

    let text_style = super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);
    let prepared = super::raw::prepare_text_edit_layout(
        &state.value,
        &edit_spec,
        text_style,
        &mut text_backend,
    );
    let metrics = prepared.layout.metrics();

    // Verify that the text is laid out with the narrower max_width (85px)
    // so that it fits 10 characters per line.
    assert_eq!(prepared.layout_width, 85.0);
    assert_eq!(metrics.line_count, 2);
    assert_eq!(metrics.lines[0].byte_end - metrics.lines[0].byte_start, 10);
    assert_eq!(metrics.lines[1].byte_end - metrics.lines[1].byte_start, 10);
}

#[test]
fn test_text_edit_wrapping_home_end() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();
    state.had_keyboard_focus = true;

    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
    edit_spec.wrap = true;

    // Visual Line 0: "abcdefghij" (index 0..10)
    // Visual Line 1: "klmnopqrst" (index 10..20)

    // Test Home
    set_caret_byte(&mut state, 15); // caret is on 'p' (Line 1)
    focus_system.begin_frame();
    let mut input = Input::default();
    input.text_events.push(TextEvent::CaretHome {
        shift: false,
        ctrl: false,
    });
    raw::post_layout_text_edit(
        edit_spec.clone(),
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(caret_byte(&state), 10);

    // Test End
    set_caret_byte(&mut state, 3); // caret is on 'd' (Line 0)
    focus_system.begin_frame();
    let mut input = Input::default();
    input.text_events.push(TextEvent::CaretEnd {
        shift: false,
        ctrl: false,
    });
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );
    focus_system.end_frame();

    assert_eq!(caret_byte(&state), 10);
}

#[test]
fn test_text_edit_wrapping_selection_visual() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
    state.had_keyboard_focus = true;
    set_selection_byte(&mut state, Some(5));
    set_caret_byte(&mut state, 15);
    focus_system.take_keyboard_focus(state.focus_id);

    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
    edit_spec.wrap = true;

    let mut cmds = DrawCommands::new(1.0);
    let input = Input::default();
    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    // Assert the selection rectangle for Line 0 (indices 5..10).
    // In TestTextBackend:
    // - start_x = 5 chars * 8px = 40.0px.
    // - end_x = 10 chars * 8px = 80.0px.
    // So the selection rect should start at x = 45.0 (40.0 + 5.0 padding) and have width = 40.0.
    let has_correct_selection = cmds.iter().any(|cmd| {
        if let DrawCmd::FillRect { rect, color, .. } = cmd {
            *color == spec().style.select_color && rect.x == 45.0 && rect.w == 40.0
        } else {
            false
        }
    });
    assert!(
        has_correct_selection,
        "Selection highlight should cover the selected range [5..10] on Line 0"
    );
}

#[test]
fn test_size_text_edit_auto_wrap_with_offer() {
    let mut text_backend = TestTextBackend::default();
    let theme = crate::theme::Theme::framewise();
    let spec = super::TextEditSpec::default().wrap(true).theme(&theme);
    let size_spec = raw::TextEditPreLayoutSpec {
        style: spec.style,
        wrap: spec.wrap,
        line_align: spec.line_align,
        error: spec.error,
        disabled: spec.disabled,
        newline_policy: spec.newline_policy,
    };

    let mut state = TextEditState::new("abcdefghijklmnopqrst");
    let input = Input::default();
    let focus_system = FocusSystem::new();

    let size_unbounded = raw::pre_layout_text_edit(
        &size_spec,
        SizeOffer::UNBOUNDED,
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;

    let limit_width = 100.0;
    let size_limited = raw::pre_layout_text_edit(
        &size_spec,
        SizeOffer::new(AxisBound::AtMost(limit_width), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;

    assert!(
        size_limited.preferred.unwrap().y > size_unbounded.preferred.unwrap().y,
        "Auto-height wrapping should increase the preferred height when constrained by offer width"
    );
}

#[test]
fn test_narrow_text_edit_caret_reset() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("");
    focus_system.take_keyboard_focus(state.focus_id);

    let mut edit_spec = spec();
    edit_spec.rect = Rect::new(0.0, 0.0, 10.0, 30.0);
    edit_spec.wrap = true;

    let mut input = Input::default();
    input.text_events.push(TextEvent::Char('a'));

    raw::post_layout_text_edit(
        edit_spec,
        raw::post_layout_only_pre_layout_result(&mut state),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut DrawCommands::new(1.0),
    );

    assert_eq!(caret_byte(&state), 1);
}

#[test]
fn test_size_text_edit_geometry_deductions() {
    let mut text_backend = TestTextBackend::default();
    let mut style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
    style.border = Some(Stroke::new(Color::BLACK, 2.0));
    style.padding_x = 4.0;
    style.error_stripe_width = 8.0;

    let mut state = TextEditState::new("abcdefghij"); // 10 chars * 8px = 80px
    let input = Input::default();
    let focus_system = FocusSystem::new();

    // A. Wrapped bounded offer accounts for padding and border
    let size_spec_a = raw::TextEditPreLayoutSpec {
        style,
        wrap: true,
        line_align: TextLineAlign::Start,
        error: false,
        disabled: false,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
    };
    let size_a = raw::pre_layout_text_edit(
        &size_spec_a,
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert_eq!(size_a.preferred.unwrap().x, 97.0);

    // B. Wrapped bounded offer accounts for vertical scrollbar gutter
    let size_b = raw::pre_layout_text_edit(
        &size_spec_a,
        SizeOffer::new(AxisBound::AtMost(88.0), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert!(size_b.preferred.unwrap().y > size_a.preferred.unwrap().y);

    // C. Wrapped narrow bounded offer honours the gutter threshold
    let size_c = raw::pre_layout_text_edit(
        &size_spec_a,
        SizeOffer::new(AxisBound::AtMost(20.0), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert_eq!(size_c.preferred.unwrap().x, 20.0);

    // D. Error state affects sizing
    let size_spec_d = raw::TextEditPreLayoutSpec {
        style,
        wrap: true,
        line_align: TextLineAlign::Start,
        error: true,
        disabled: false,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
    };
    let size_d = raw::pre_layout_text_edit(
        &size_spec_d,
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert!(size_d.preferred.unwrap().y > size_a.preferred.unwrap().y);
    assert_eq!(size_d.preferred.unwrap().x, 97.0);

    // E. Non-error state does not include error stripe
    let mut state_short = TextEditState::new("abc"); // 3 chars = 24px
    let size_spec_non_error = raw::TextEditPreLayoutSpec {
        style,
        wrap: true,
        line_align: TextLineAlign::Start,
        error: false,
        disabled: false,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
    };
    let size_spec_error = raw::TextEditPreLayoutSpec {
        style,
        wrap: true,
        line_align: TextLineAlign::Start,
        error: true,
        disabled: false,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
    };
    let size_non_error = raw::pre_layout_text_edit(
        &size_spec_non_error,
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::Unbounded),
        &mut state_short,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    let size_error = raw::pre_layout_text_edit(
        &size_spec_error,
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::Unbounded),
        &mut state_short,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert_eq!(
        size_error.preferred.unwrap().x - size_non_error.preferred.unwrap().x,
        style.error_stripe_width
    );

    // F. Unwrapped sizing is not accidentally changed
    let size_spec_f = raw::TextEditPreLayoutSpec {
        style,
        wrap: false,
        line_align: TextLineAlign::Start,
        error: false,
        disabled: false,
        newline_policy: NewlinePolicy::ReplaceWithSpace,
    };
    let size_f = raw::pre_layout_text_edit(
        &size_spec_f,
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::Unbounded),
        &mut state,
        &input,
        &focus_system,
        &mut text_backend,
    )
    .size_request;
    assert_eq!(size_f.preferred.unwrap().x, 92.0);

    // G. Shared-helper equivalence test
    for wrap in [true, false] {
        let mut spec_g = spec();
        spec_g.wrap = wrap;
        let scroll_outer_width =
            raw::text_edit_scroll_outer_width(spec_g.rect.w, spec_g.style, spec_g.error);
        let available_text_width =
            raw::text_edit_available_text_width(scroll_outer_width, spec_g.style);
        let reserved_scrollbar = raw::should_reserve_vertical_scrollbar_gutter(
            "abcdef",
            &spec_g,
            TextStyle::new(
                spec_g.style.font,
                14.0,
                crate::theme::Theme::framewise().sans_weight_regular,
                TextFlow::single_line(),
            ),
            &mut text_backend,
            raw::text_edit_scroll_outer_rect(&spec_g),
        );
        let reserved_vertical_width =
            raw::text_edit_reserved_vertical_width(reserved_scrollbar, available_text_width);
        let content_width = raw::text_edit_content_width(available_text_width, reserved_scrollbar);

        let prepared_g = raw::prepare_text_edit_layout(
            "abcdef",
            &spec_g,
            TextStyle::new(
                spec_g.style.font,
                14.0,
                crate::theme::Theme::framewise().sans_weight_regular,
                TextFlow::single_line(),
            ),
            &mut text_backend,
        );

        assert_eq!(
            available_text_width,
            (prepared_g.scroll_outer_rect.w - 2.0 * spec_g.style.padding_x).max(0.0)
        );
        assert_eq!(
            reserved_vertical_width,
            if prepared_g.reserved_vertical_scrollbar {
                5.0
            } else {
                0.0
            }
        );
        if wrap {
            assert_eq!(content_width, prepared_g.layout_width);
        }
    }
}

#[test]
fn test_high_level_text_edit_copy() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    set_selection_byte(&mut state, Some(6));
    set_caret_byte(&mut state, 11);
    state.had_keyboard_focus = true;

    let mut input = Input::default();
    input.text_events.push(TextEvent::Copy);

    let mut output = crate::Output::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut ctx = WidgetContext::root(
        Theme::framewise(),
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    let _res = text_edit(
        super::TextEditSpec::default_from_theme(&ctx.theme),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert_eq!(output.new_clipboard_contents.as_deref(), Some("world"));
    assert_eq!(state.value, "hello world");
}

#[test]
fn test_high_level_text_edit_cut() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = TextEditState::new("hello world");

    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    set_selection_byte(&mut state, Some(6));
    set_caret_byte(&mut state, 11);
    state.had_keyboard_focus = true;

    let mut input = Input::default();
    input.text_events.push(TextEvent::Cut);

    let mut output = crate::Output::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut ctx = WidgetContext::root(
        Theme::framewise(),
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    let _res = text_edit(
        super::TextEditSpec::default_from_theme(&ctx.theme),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert_eq!(output.new_clipboard_contents.as_deref(), Some("world"));
    assert_eq!(state.value, "hello ");
}

#[test]
fn test_text_edit_high_level_sets_output_cursor() {
    let mut text_backend = TestTextBackend::default();
    let mut state = TextEditState::new("");
    let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
    let input = Input {
        mouse_pos: Vec2::new(5.0, 15.0),
        ..Input::default()
    };
    let mut output = crate::Output::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut ctx = WidgetContext::root(
        Theme::framewise(),
        &mut text_backend,
        &mut focus_system,
        &input,
        &mut output,
        ColumnLayout::new(),
        Rect::new(0.0, 0.0, 500.0, 100.0),
        &mut cmds,
    );

    let result = text_edit(
        super::TextEditSpec::default_from_theme(&ctx.theme),
        ColumnLayoutParams::auto(),
        &mut state,
        &mut ctx,
    );
    ctx.finish();

    assert!(result.input.hovered);
    assert_eq!(output.cursor_icon, Some(CursorIcon::Text));
}
