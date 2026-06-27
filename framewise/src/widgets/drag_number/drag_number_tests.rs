use super::raw::DragNumberSpec;
use super::*;
use crate::test_utils::TestTextBackend;
use crate::types::Vec2;
use crate::{DrawGlyph, PreparedGlyphToken};

fn default_style() -> DragNumberStyle {
    DragNumberStyle::from_theme(&crate::theme::Theme::framewise())
}

fn default_spec(rect: Rect) -> raw::DragNumberSpec<'static> {
    raw::DragNumberSpec {
        layer: Layer::default(),
        rect,
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: default_style(),
        clip_rect: None,
    }
}

fn run_raw(
    spec: raw::DragNumberSpec<'_>,
    state: &mut DragNumberState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut TestTextBackend,
    cmds: &mut DrawCommands,
) -> raw::DragNumberResult {
    raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state,
        input,
        focus_system,
        text_backend,
        cmds,
    )
}

fn label_area_pos(rect: Rect, label: &str) -> Vec2 {
    let char_width = 8.0; // default advance in TestTextBackend
    let pad_x = 10.0;
    let label_w = (label.len() as f32) * char_width + pad_x * 2.0;
    Vec2::new(rect.x + label_w / 2.0, rect.y + rect.h / 2.0)
}

fn value_area_pos(rect: Rect, label: &str) -> Vec2 {
    let char_width = 8.0; // default advance in TestTextBackend
    let pad_x = 10.0;
    let label_w = (label.len() as f32) * char_width + pad_x * 2.0;
    let value_x = rect.x + label_w;
    let value_w = (rect.w - label_w).max(20.0);
    Vec2::new(value_x + value_w / 2.0, rect.y + rect.h / 2.0)
}

fn drag_num<'a>(spec: DragNumberSpec<'a>, value: f32) -> (raw::DragNumberResult, DrawCommands) {
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let res = raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut DragNumberState {
            value,
            ..Default::default()
        },
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );
    (res, cmds)
}

#[test]
fn test_drag_number_visual_normal() {
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let (_res, cmds) = drag_num(spec, 50.0);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                color: style.text_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: style.text_text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..6,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 54.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 62.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 70.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 78.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 86.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_drag_number_visual_active() {
    let mut state = DragNumberState {
        value: 50.0,
        is_dragging: true,
        drag_start_value: 50.0,
        ..Default::default()
    };
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let input = Input {
        mouse_down: true,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let _res = raw::post_layout_drag_number(
        spec.clone(),
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );

    // Extract focus ring command and assert its properties.
    let focus_style = style.focus.unwrap();
    let focus_color = focus_style.stroke.color;

    let focus_cmd_idx = cmds
        .commands()
        .iter()
        .position(|cmd| {
            if let DrawCmd::BorderRect { color, width, .. } = cmd {
                *color == focus_color && *width == focus_style.stroke.width
            } else {
                false
            }
        })
        .expect("Focus ring border command should be present");

    let focus_cmd = &cmds.commands()[focus_cmd_idx];
    if let DrawCmd::BorderRect {
        rect, placement, z, ..
    } = focus_cmd
    {
        assert_eq!(*rect, spec.rect.inset(-focus_style.offset));
        assert_eq!(*placement, crate::BorderPlacement::Outside);
        assert_eq!(
            *z,
            spec.layer.get_focus_z(),
            "Focus ring z must equal spec.layer.get_focus_z()"
        );
    } else {
        panic!("Found command is not a BorderRect");
    }

    let mut other_cmds = cmds.commands().to_vec();
    other_cmds.remove(focus_cmd_idx);

    assert_eq!(
        other_cmds,
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                color: style.active_text_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: style.text_text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..6,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 54.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 62.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 70.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 78.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 86.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_drag_number_visual_min_value() {
    let mut text_backend = TestTextBackend::default();
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    let _res = raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut DragNumberState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                color: style.text_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: style.text_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..5,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 58.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 66.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 74.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 82.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_drag_number_click_takes_focus() {
    let mut focus_system = FocusSystem::new();
    let state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
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
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Clicking drag number must request focus"
    );
}

#[test]
fn test_drag_number_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 28.0)),
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
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
        focus_system.current_keyboard_focus(),
        None,
        "Clicking a clipped-away drag number must not take focus"
    );
}

#[test]
fn test_drag_number_keyboard_navigation() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut input = Input::default();
    let mut text_backend = TestTextBackend::default();

    // Focus the widget
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: Press Arrow Right -> value increases by 1.0 (step = 100 * 0.01)
    input.key_pressed_right = true;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_drag_number(
        DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    input.key_pressed_right = false;

    assert_eq!(state.value, 51.0);

    // Frame 2: Press Arrow Left -> value decreases back to 50.0
    input.key_pressed_left = true;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    raw::post_layout_drag_number(
        DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 50.0);
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_fields() {
    let theme = crate::theme::Theme::framewise();
    let builder = DragNumberSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(DragNumberStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_fields() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = DragNumberStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let builder = DragNumberSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().text_style.size, 99.0);
}

#[test]
fn test_high_level_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
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
    let mut dn_state = DragNumberState::default();
    let result = super::drag_number(
        &mut ctx,
        DragNumberSpecBuilder::new().text("x"),
        placement,
        &mut dn_state,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_drag_number_disabled_ignores_press_interaction() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let focus_id = state.focus_id;
    let inside_pos = value_area_pos(rect, "X");

    crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
        &mut state,
        focus_id,
        inside_pos,
        |state, input, focus_system, cmds| {
            let mut spec = default_spec(rect);
            spec.disabled = true;
            let mut text_backend = TestTextBackend::default();
            let res = run_raw(spec, state, input, focus_system, &mut text_backend, cmds);
            res.input
        },
    );
}

#[test]
fn test_drag_number_clipped_press_does_not_take_focus_with_helper() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let inside_pos = value_area_pos(rect, "X");

    crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
        &mut state,
        inside_pos,
        |state, input, focus_system, cmds| {
            let mut spec = default_spec(rect);
            // clip_rect excludes inside_pos
            spec.clip_rect = Some(Rect::new(500.0, 500.0, 10.0, 10.0));
            let mut text_backend = TestTextBackend::default();
            let res = run_raw(spec, state, input, focus_system, &mut text_backend, cmds);
            res.input
        },
    );
}

#[test]
fn test_drag_number_mouse_press_takes_focus_with_helper() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let focus_id = state.focus_id;
    let inside_pos = value_area_pos(rect, "X");

    crate::widgets::test_helpers::assert_mouse_press_takes_focus(
        &mut state,
        focus_id,
        inside_pos,
        |state, input, focus_system, cmds| {
            let spec = default_spec(rect);
            let mut text_backend = TestTextBackend::default();
            let res = run_raw(spec, state, input, focus_system, &mut text_backend, cmds);
            res.input
        },
    );
}

#[test]
fn test_drag_number_drag_updates_value() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let spec = default_spec(rect);

    // Warmup frame: Mouse inside value area
    let mut input = Input {
        mouse_pos: Vec2::new(50.0, 14.0),
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: Mouse press down in the value area at x = 50.0, y = 14.0
    input.mouse_down = true;
    input.mouse_pressed = true;

    focus_system.begin_frame();
    let _res = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(state.is_dragging, "Should start dragging on mouse press");
    assert_eq!(
        state.value, 50.0,
        "Value shouldn't change on the initial click frame"
    );

    // Frame 2: Mouse moved right to x = 68.0, with mouse_down still true
    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(68.0, 14.0);

    focus_system.begin_frame();
    let _res2 = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(state.is_dragging, "Should still be dragging");
    // dx = 68 - 50 = 18. value_w = 100 - (8 + 20) = 72. dx/value_w = 0.25. delta = 0.25 * 100 = 25.
    assert_eq!(
        state.value, 75.0,
        "Value should update proportionally to mouse drag"
    );
}

#[test]
fn test_drag_number_drag_clamps_to_min_max() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let spec = default_spec(rect);

    // Warmup frame: Mouse inside value area
    let mut input = Input {
        mouse_pos: Vec2::new(50.0, 14.0),
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: Mouse press down in the value area
    input.mouse_down = true;
    input.mouse_pressed = true;

    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Drag far right (e.g. x = 500.0)
    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(500.0, 14.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 100.0, "Value should clamp to max (100.0)");

    // Frame 3: Drag far left (e.g. x = -500.0)
    input.mouse_pos = Vec2::new(-500.0, 14.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 0.0, "Value should clamp to min (0.0)");
}

#[test]
fn test_drag_number_drag_releases_when_mouse_up() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let spec = default_spec(rect);

    // Warmup frame: Mouse inside value area
    let mut input = Input {
        mouse_pos: Vec2::new(50.0, 14.0),
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 1: Drag starts
    input.mouse_down = true;
    input.mouse_pressed = true;

    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(state.is_dragging);

    // Frame 2: Mouse released
    input.mouse_down = false;
    input.mouse_pressed = false;
    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert!(
        !state.is_dragging,
        "Dragging should stop when mouse is released"
    );
}

#[test]
fn test_drag_number_min_greater_than_max_keyboard_does_not_panic() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let spec = raw::DragNumberSpec {
        layer: Layer::default(),
        rect,
        text: "X",
        min: 100.0,
        max: 0.0,
        disabled: false,
        style: default_style(),
        clip_rect: None,
    };

    let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut state = DragNumberState {
            value: 50.0,
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);

        let input = Input {
            key_pressed_right: true,
            ..Default::default()
        };
        let mut text_backend = TestTextBackend::default();
        let mut cmds = DrawCommands::new(1.0);

        focus_system.begin_frame();
        let _ = run_raw(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();
    }));

    assert!(
        run_res.is_ok(),
        "Keyboard adjustment when min > max should not panic"
    );
}

#[test]
fn test_drag_number_label_area_starts_dragging() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let label = "X";
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let spec = default_spec(rect);

    let press_pos = label_area_pos(rect, label);

    // Warmup frame: Mouse inside label area
    let mut input = Input {
        mouse_pos: press_pos,
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Mouse press inside the label area
    input.mouse_down = true;
    input.mouse_pressed = true;

    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(
        state.is_dragging,
        "Pressing inside label area should start dragging"
    );
}

#[test]
fn test_drag_number_overlapping_hover_uses_top_widget() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state_bottom = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut state_top = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let overlap_pos = value_area_pos(rect, "X");

    crate::widgets::test_helpers::assert_overlapping_hover(
        &mut state_bottom,
        &mut state_top,
        overlap_pos,
        |state_b, state_t, input, focus_system, cmds| {
            let mut text_backend = TestTextBackend::default();
            let spec_b = default_spec(rect);
            let spec_t = default_spec(rect);
            let res_b = run_raw(
                spec_b,
                state_b,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            let res_t = run_raw(
                spec_t,
                state_t,
                input,
                focus_system,
                &mut text_backend,
                cmds,
            );
            (res_b.input, res_t.input)
        },
    );
}

#[test]
fn test_drag_number_overlapping_press_uses_top_widget() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state_bottom = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut state_top = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let overlap_pos = value_area_pos(rect, "X");
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    // Warm up hover over two overlapping drag-numbers
    let warmup_input = Input {
        mouse_pos: overlap_pos,
        ..Default::default()
    };
    focus_system.begin_frame();
    let spec_b = default_spec(rect);
    let spec_t = default_spec(rect);
    let _ = run_raw(
        spec_b.clone(),
        &mut state_bottom,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let _ = run_raw(
        spec_t.clone(),
        &mut state_top,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Mouse press/down in the overlapping control area
    let press_input = Input {
        mouse_pos: overlap_pos,
        mouse_down: true,
        mouse_pressed: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    let res_b = run_raw(
        spec_b,
        &mut state_bottom,
        &press_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let res_t = run_raw(
        spec_t,
        &mut state_top,
        &press_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(
        !state_bottom.is_dragging,
        "Bottom widget should not enter is_dragging"
    );
    assert!(state_top.is_dragging, "Top widget should enter is_dragging");
    assert!(
        !res_b.input.pressed,
        "Bottom widget result should not be pressed"
    );
    assert!(res_t.input.pressed, "Top widget result should be pressed");
}

#[test]
fn test_drag_number_arrow_keys_no_traversal_but_tab_traverses() {
    let mut focus_system = FocusSystem::new();
    let mut state1 = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut state2 = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    // Initial frame to register widgets and focus the first one
    focus_system.take_keyboard_focus(state1.focus_id);

    // Frame 1: Press ArrowRight. First value should increase, focus should remain on first
    let input_right = Input {
        key_pressed_right: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    let spec1 = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    let spec2 = default_spec(Rect::new(0.0, 40.0, 100.0, 28.0));

    // Run first widget
    let _ = run_raw(
        spec1.clone(),
        &mut state1,
        &input_right,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    // Run second widget
    let _ = run_raw(
        spec2.clone(),
        &mut state2,
        &input_right,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        state1.value, 51.0,
        "First value should adjust with ArrowRight"
    );
    assert_eq!(state2.value, 50.0, "Second value should not change");
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state1.focus_id),
        "Focus should remain on first widget after ArrowRight"
    );

    // Frame 2: Press Tab. Focus should traverse to the second widget
    let input_tab = Input {
        key_pressed_tab: true,
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec1,
        &mut state1,
        &input_tab,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    let _ = run_raw(
        spec2,
        &mut state2,
        &input_tab,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state2.focus_id),
        "Focus should traverse to the second widget after Tab"
    );
}
