use super::raw::DragNumberSpec;
use super::*;
use crate::test_utils::TestTextBackend;
use crate::types::Vec2;
use crate::{DrawGlyph, PreparedGlyphToken};

fn drag_num<'a>(spec: DragNumberSpec<'a>, value: f32) -> (raw::DragNumberResult, DrawCommands) {
    let mut cmds = DrawCommands::new();
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
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
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
                anti_alias: false,
                rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..6,
                color: style.value_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 54.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 62.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 70.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 78.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 86.0, y: 29.0 },
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
    let mut cmds = DrawCommands::new();
    let mut text_backend = TestTextBackend::default();
    let _res = raw::post_layout_drag_number(
        spec,
        raw::DragNumberPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(9.0, 9.0, 102.0, 30.0),
                color: style.focus.unwrap().stroke.color,
                width: style.focus.unwrap().stroke.width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
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
                anti_alias: false,
                rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..6,
                color: style.value_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 54.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 62.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 70.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 78.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 86.0, y: 29.0 },
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
    let mut cmds = DrawCommands::new();
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
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
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
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(88),
                top_left: Vec2 { x: 20.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 58.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 66.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 74.0, y: 29.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 82.0, y: 29.0 },
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
    let mut cmds = DrawCommands::new();
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
    let mut cmds = DrawCommands::new();
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
    let mut cmds = DrawCommands::new();
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
    let mut cmds = DrawCommands::new();
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
    let mut dn_state = DragNumberState::default();
    let result = super::drag_number(
        &mut ctx,
        DragNumberSpecBuilder::new().text("x"),
        placement,
        &mut dn_state,
    );
    assert_eq!(result.layout.bounds, placement);
}
