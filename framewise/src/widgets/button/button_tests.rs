use super::raw::ButtonSpec;
use super::*;

use crate::test_utils::TestTextBackend;
use crate::text::FontId;
use crate::theme;
use crate::{DrawGlyph, PreparedGlyphToken};

fn placement_text_backend() -> TestTextBackend {
    TestTextBackend::default()
        .with_line_height(20)
        .with_default_advance(30.0)
        .with_glyph_offset(Vec2::new(0.0, -13.0))
        .with_glyph_ink_bounds(Rect::new(-4.0, 3.0, 18.0, 10.0))
}

fn btn_spec(rect: Rect) -> ButtonSpec<'static> {
    ButtonSpec {
        layer: Layer::default(),
        rect,
        text: "Btn",
        style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
        clip_rect: None,
        disabled: false,
    }
}

fn draw_two_buttons(
    focus_system: &mut FocusSystem,
    s1: &mut ButtonState,
    s2: &mut ButtonState,
    input: &Input,
    text_backend: &mut TestTextBackend,
    cmds: &mut DrawCommands,
) {
    raw::post_layout_button(
        btn_spec(Rect::new(0.0, 0.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        s1,
        input,
        focus_system,
        text_backend,
        cmds,
    );
    raw::post_layout_button(
        btn_spec(Rect::new(0.0, 40.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        s2,
        input,
        focus_system,
        text_backend,
        cmds,
    );
}

#[test]
fn test_button_tab_moves_focus_next() {
    let mut s1 = ButtonState::default();
    let mut s2 = ButtonState::default();
    let focus1 = s1.focus_id;
    let focus2 = s2.focus_id;
    let mut text_backend = TestTextBackend::default();

    crate::widgets::test_helpers::assert_tab_moves_focus_next(
        &mut s1,
        focus1,
        &mut s2,
        focus2,
        |s1, s2, input, focus_system, cmds| {
            draw_two_buttons(focus_system, s1, s2, input, &mut text_backend, cmds);
        },
    );
}

#[test]
fn test_button_right_arrow_moves_focus_next() {
    let mut s1 = ButtonState::default();
    let mut s2 = ButtonState::default();
    let focus1 = s1.focus_id;
    let focus2 = s2.focus_id;
    let mut text_backend = TestTextBackend::default();

    crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
        &mut s1,
        focus1,
        &mut s2,
        focus2,
        |s1, s2, input, focus_system, cmds| {
            draw_two_buttons(focus_system, s1, s2, input, &mut text_backend, cmds);
        },
    );
}

#[test]
fn test_button_down_arrow_moves_focus_next() {
    let mut s1 = ButtonState::default();
    let mut s2 = ButtonState::default();
    let focus1 = s1.focus_id;
    let focus2 = s2.focus_id;
    let mut text_backend = TestTextBackend::default();

    crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
        &mut s1,
        focus1,
        &mut s2,
        focus2,
        |s1, s2, input, focus_system, cmds| {
            draw_two_buttons(focus_system, s1, s2, input, &mut text_backend, cmds);
        },
    );
}

#[test]
fn test_button_shift_tab_moves_focus_prev() {
    let mut s1 = ButtonState::default();
    let mut s2 = ButtonState::default();
    let focus1 = s1.focus_id;
    let focus2 = s2.focus_id;
    let mut text_backend = TestTextBackend::default();

    crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
        &mut s1,
        focus1,
        &mut s2,
        focus2,
        |s1, s2, input, focus_system, cmds| {
            draw_two_buttons(focus_system, s1, s2, input, &mut text_backend, cmds);
        },
    );
}

#[test]
fn test_drag_off_and_release_does_not_click_other_button() {
    let mut text_backend = TestTextBackend::default();
    let mut state1 = ButtonState::default();
    let mut state2 = ButtonState::default();

    crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
        &mut state1,
        &mut state2,
        Vec2::new(50.0, 25.0),
        Vec2::new(50.0, 125.0),
        false,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_button(
                ButtonSpec {
                    layer: Layer::default(),
                    text: "Click Me",
                    ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
                },
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res2 = raw::post_layout_button(
                ButtonSpec {
                    layer: Layer::default(),
                    text: "Btn2",
                    ..btn_spec(Rect::new(0.0, 100.0, 100.0, 50.0))
                },
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
fn test_click_triggers_clicked_state() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();

    crate::widgets::test_helpers::assert_mouse_click_on_release(
        &mut state,
        Vec2::new(50.0, 25.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_button_overlapping_hover() {
    let mut text_backend = TestTextBackend::default();
    let mut state1 = ButtonState::default();
    let mut state2 = ButtonState::default();

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res2 = raw::post_layout_button(
                btn_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
fn test_button_overlapping_click() {
    let mut text_backend = TestTextBackend::default();
    let mut state1 = ButtonState::default();
    let mut state2 = ButtonState::default();

    crate::widgets::test_helpers::assert_overlapping_click(
        &mut state1,
        &mut state2,
        Vec2::new(75.0, 75.0),
        true,
        |state1, state2, input, focus_system, cmds| {
            let res1 = raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state1,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res2 = raw::post_layout_button(
                btn_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
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
fn test_button_click_takes_focus() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_mouse_press_takes_focus(
        &mut state,
        focus_id,
        Vec2::new(50.0, 25.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_button_clipped_click_does_not_take_focus() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();

    crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
        &mut state,
        Vec2::new(50.0, 25.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                ButtonSpec {
                    layer: Layer::default(),
                    rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                    text: "Btn",
                    style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
                    clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 30.0)),
                    disabled: false,
                },
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_button_disabled_ignores_interaction() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
        &mut state,
        focus_id,
        Vec2::new(50.0, 25.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                ButtonSpec {
                    layer: Layer::default(),
                    disabled: true,
                    ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
                },
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_enter_clicks_raw_button() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();
    let mut focus_system = FocusSystem::new();

    let spec = || btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0));

    // Frame 1: Register and take focus explicitly
    let mut input = Input::default();
    let mut cmds = DrawCommands::new();
    raw::post_layout_button(
        spec(),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.end_frame();

    // Frame 2: Press Enter
    input.key_pressed_enter = true;
    let res = raw::post_layout_button(
        spec(),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    assert!(res.input.clicked, "Button should be clicked by Enter key");
}

#[test]
fn test_hover_and_press_state() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();

    crate::widgets::test_helpers::assert_hover_and_press_state(
        &mut state,
        Vec2::new(50.0, 25.0),
        Vec2::new(150.0, 150.0),
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_spacebar_click() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

#[test]
fn test_spacebar_loses_focus_does_not_click() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ButtonState::default();
    let focus_id = state.focus_id;

    crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
        &mut state,
        focus_id,
        |state, input, focus_system, cmds| {
            raw::post_layout_button(
                btn_spec(Rect::new(0.0, 0.0, 100.0, 50.0)),
                raw::ButtonPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                state,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            )
            .input
        },
    );
}

// ── Visual Tests ─────────────────────────────────────────────────────────

#[test]
fn test_button_visual_normal() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let input = Input::default();
    let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let style = ButtonStyle::primary_from_theme(&theme::Theme::default());
    let background = style.background;
    let border = style.border.unwrap().color;
    let border_width = style.border.unwrap().width;
    let text_color = style.text_color;

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: border,
                width: border_width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: text_color,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 48.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 56.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 64.0, y: 30.0 },
            },
        ]
    );
}

#[test]
fn test_button_visual_hovered() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let input = Input {
        mouse_pos: Vec2::new(50.0, 25.0), // Inside bounds
        ..Default::default()
    };

    let mut state = state;
    // Warmup frame to establish hover claim
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let ButtonStyle {
        hovered,
        border,
        text_color,
        ..
    } = ButtonStyle::primary_from_theme(&theme::Theme::default());

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: hovered,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: border.unwrap().color,
                width: border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: text_color,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 48.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 56.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 64.0, y: 30.0 },
            },
        ]
    );
}

#[test]
fn test_button_visual_pressed() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let mut input = Input {
        mouse_pos: Vec2::new(50.0, 25.0),
        ..Default::default()
    };

    let mut state = state;
    // Warmup frame to establish hover claim
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Evaluation frame with mouse pressed
    input.mouse_down = true;
    input.mouse_pressed = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0)),
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let ButtonStyle {
        pressed,
        border,
        text_color,
        ..
    } = ButtonStyle::primary_from_theme(&theme::Theme::default());

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: pressed,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: border.unwrap().color,
                width: border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: text_color,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 48.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 56.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 64.0, y: 30.0 },
            },
        ]
    );
}

#[test]
fn test_button_visual_focused() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let spec = btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0));

    focus_system.take_keyboard_focus(state.focus_id);

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let style = ButtonStyle::primary_from_theme(&theme::Theme::default());
    let background = style.background;
    let border = style.border.unwrap().color;
    let border_width = style.border.unwrap().width;
    let text_color = style.text_color;
    let focus = style.focus.unwrap().stroke.color;
    let focus_offset = style.focus.unwrap().offset;
    let focus_width = style.focus.unwrap().stroke.width;

    let expected_focus_rect = Rect::new(10.0, 10.0, 100.0, 30.0).inset(-focus_offset);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::BorderRect {
                rect: expected_focus_rect,
                color: focus,
                width: focus_width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: border,
                width: border_width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: text_color,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 48.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 56.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 64.0, y: 30.0 },
            },
        ]
    );
}

#[test]
fn test_button_visual_disabled() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let spec = ButtonSpec {
        layer: Layer::default(),
        disabled: true,
        ..btn_spec(Rect::new(10.0, 10.0, 100.0, 30.0))
    };

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let alpha = 0.32_f32;
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let primary_style = ButtonStyle::primary_from_theme(&theme::Theme::default());
    let expected_bg = tint(primary_style.background);
    let expected_border = tint(primary_style.border.unwrap().color);
    let expected_text = tint(primary_style.text_color);
    let border_width = primary_style.border.map_or(0.0, |b| b.width);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: expected_bg,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 30.0),
                color: expected_border,
                width: border_width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: expected_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 48.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 56.0, y: 30.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 64.0, y: 30.0 },
            },
        ]
    );
}

#[test]
fn test_button_logical_content_placement_respects_padding() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let mut state = ButtonState::default();
    let spec = ButtonSpec {
        layer: Layer::default(),
        style: ButtonStyle {
            content_placement: crate::text::TextContentPlacement::logical(
                crate::text::ContentPlacement::Align(crate::Align::End),
                crate::text::ContentPlacement::Align(crate::Align::End),
            ),
            ..ButtonStyle::primary_from_theme(&theme::Theme::default())
        },
        ..btn_spec(Rect::new(10.0, 20.0, 100.0, 50.0))
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _ = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(
        cmds.glyphs()
            .first()
            .is_some_and(|glyph| glyph.top_left == Vec2::new(72.0, 61.0)),
        "button text should be bottom-right aligned inside the padded content rect"
    );
}

#[test]
fn test_button_ink_content_placement_uses_ink_bounds_when_disabled() {
    let mut text_backend = placement_text_backend();
    let mut focus_system = FocusSystem::new();
    let mut state = ButtonState::default();
    let spec = ButtonSpec {
        layer: Layer::default(),
        disabled: true,
        style: ButtonStyle {
            content_placement: crate::text::TextContentPlacement::INK_CENTER,
            ..ButtonStyle::primary_from_theme(&theme::Theme::default())
        },
        text: "B",
        ..btn_spec(Rect::new(10.0, 20.0, 100.0, 50.0))
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _ = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        text_backend
            .observations
            .prepared_glyph_rects
            .first()
            .copied(),
        Some(Rect::new(55.0, 37.0, 30.0, 20.0))
    );
}

#[test]
fn test_regression_custom_style_no_theme_lookup() {
    let mut text_backend = TestTextBackend::default();
    let mut focus_system = FocusSystem::new();
    let state = ButtonState::default();
    let input = Input::default();

    let custom_style = ButtonStyle {
        background: Color::from_srgb_u8(100, 150, 200, 255),
        hovered: Color::from_srgb_u8(110, 160, 210, 255),
        pressed: Color::from_srgb_u8(120, 170, 220, 255),
        border: Some(Stroke::new(Color::from_srgb_u8(220, 230, 240, 255), 4.5)),
        focus: Some(Outline::new(Color::from_srgb_u8(255, 0, 0, 255), 2.0, 2.0)),
        text_style: crate::text::TextStyle::new(
            FontId(0),
            19.5,
            400,
            crate::text::TextFlow::single_line(),
        ),
        content_placement: crate::text::TextContentPlacement::CENTER,
        text_color: Color::from_srgb_u8(50, 60, 70, 255),
        disabled_alpha: 0.32f32,
        pad_x: 14.0,
        pad_y: 6.0,
        min_height: 28.0,
    };

    let spec = ButtonSpec {
        layer: Layer::default(),
        rect: Rect::new(5.0, 15.0, 120.0, 45.0),
        text: "Explicit Spec",
        style: custom_style,
        clip_rect: None,
        disabled: false,
    };

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_button(
        spec,
        raw::ButtonPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                color: custom_style.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(5.0, 15.0, 120.0, 45.0),
                color: custom_style.border.unwrap().color,
                width: custom_style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..10,
                color: custom_style.text_color,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(69),
                top_left: Vec2 { x: 21.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(120),
                top_left: Vec2 { x: 29.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 37.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 45.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 53.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(99),
                top_left: Vec2 { x: 61.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 69.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 77.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(83),
                top_left: Vec2 { x: 93.0, y: 49.5 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 101.0, y: 49.5 },
            },
        ]
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = ButtonSpecBuilder::new().text("test");
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert!(builder.style.is_some());
    let expected = ButtonStyle::secondary_from_theme(&theme);
    assert_eq!(
        builder.style.unwrap().text_style.font,
        expected.text_style.font
    );
    assert_eq!(
        builder.style.unwrap().text_style.size,
        expected.text_style.size
    );
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let default_primary = ButtonStyle::primary_from_theme(&theme::Theme::default());
    let custom_style = ButtonStyle {
        text_style: crate::text::TextStyle {
            size: 99.0,
            ..default_primary.text_style
        },
        ..default_primary
    };
    let builder = ButtonSpecBuilder::new().text("test").style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().text_style.size, 99.0);
}

#[test]
fn test_high_level_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
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
    let mut btn_state = ButtonState::default();
    // Under ManualLayout the layout param *is* the rect — the sanctioned way
    // to place a high-level widget explicitly.
    let result = super::button(
        &mut ctx,
        ButtonSpecBuilder::new().text("X"),
        placement,
        &mut btn_state,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_high_level_honors_user_style() {
    use crate::layouts::ManualLayout;
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
    // A user-set builder field (style) must be honored, not overwritten by
    // theme defaults.
    let custom = ButtonStyle {
        background: Color::from_srgb_u8(1, 2, 3, 255),
        ..ButtonStyle::accent_from_theme(&theme::Theme::default())
    };
    let mut btn_state = ButtonState::default();
    // Placed away from the default mouse position (0,0) so it isn't hovered.
    super::button(
        &mut ctx,
        ButtonSpecBuilder::new().text("X").style(custom),
        Rect::new(100.0, 100.0, 40.0, 28.0),
        &mut btn_state,
    );
    let has_custom_fill = cmds
        .iter()
        .any(|c| matches!(c, DrawCmd::FillRect {  color, .. } if *color == custom.background));
    assert!(
        has_custom_fill,
        "high-level button must honor user-set style"
    );
}

#[test]
fn test_size_button() {
    let mut ts = TestTextBackend::default();
    let spec = raw::ButtonPreLayoutSpec {
        text: "Btn",
        style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
    };
    // "Btn" = 3 chars * 8px = 24 wide, 16 tall (TestTextBackend).
    // width = 24 + 2*pad_x(14) = 52; height = max(16 + 2*pad_y(6), min_height 28) = 28.
    let i = raw::pre_layout_button(&spec, SizeOffer::UNBOUNDED, &mut ts).size_request;
    assert_eq!(i.preferred, Some(Vec2::new(52.0, 28.0)));
}

#[test]
fn test_size_button_ignores_offer() {
    use crate::layout::AxisBound;

    let spec = raw::ButtonPreLayoutSpec {
        text: "Btn",
        style: ButtonStyle::primary_from_theme(&theme::Theme::default()),
    };
    let offers = [
        SizeOffer::UNBOUNDED,
        SizeOffer::new(AxisBound::Exact(50.0), AxisBound::Exact(20.0)),
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::AtMost(40.0)),
    ];

    let mut ts = TestTextBackend::default();
    let expected = raw::pre_layout_button(&spec, offers[0], &mut ts).size_request;
    for offer in offers {
        let mut ts = TestTextBackend::default();
        assert_eq!(
            raw::pre_layout_button(&spec, offer, &mut ts).size_request,
            expected
        );
    }
}

#[test]
fn test_button_auto_layout_uses_size_request() {
    use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new();
    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 300.0, 400.0), ColumnLayout);
    let mut st = ButtonState::default();
    // Auto on both axes → the button sizes to its label request.
    // "Save" = 4*8 = 32 wide; width = 32 + 28 = 60; height = 28.
    let r = super::button(
        &mut col,
        ButtonSpecBuilder::new().text("Save"),
        ColumnLayoutParams::auto(),
        &mut st,
    );
    assert_eq!(r.layout.bounds, Rect::new(10.0, 10.0, 60.0, 28.0));
}

#[test]
fn test_button_peek_offer_flow_does_not_move_sibling() {
    use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new();
    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 300.0, 400.0), ColumnLayout);
    let mut st = ButtonState::default();

    let button = super::button(
        &mut col,
        ButtonSpecBuilder::new().text("Save"),
        ColumnLayoutParams::auto(),
        &mut st,
    );
    let sibling = col.layout(
        ColumnLayoutParams::fixed(20.0, 10.0),
        crate::layout::SizeRequest::UNKNOWN,
    );

    assert_eq!(button.layout.bounds, Rect::new(10.0, 10.0, 60.0, 28.0));
    assert_eq!(sibling, Rect::new(10.0, 38.0, 20.0, 10.0));
}
