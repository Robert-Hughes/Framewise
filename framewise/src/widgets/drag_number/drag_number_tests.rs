use super::raw::DragNumberSpec;
use super::*;
use crate::focus::FocusId;
use crate::input::TextEvent;
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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: default_style(),
        clip_rect: None,
    }
}

fn run_raw<F>(
    spec: raw::DragNumberSpec<'_, F>,
    state: &mut DragNumberState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut TestTextBackend,
    cmds: &mut DrawCommands,
) -> raw::DragNumberResult
where
    F: Fn(f32) -> String,
{
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

fn run_key<F>(
    spec: raw::DragNumberSpec<'_, F>,
    state: &mut DragNumberState,
    focus_system: &mut FocusSystem,
    set_key: impl FnOnce(&mut Input),
) where
    F: Fn(f32) -> String,
{
    let mut input = Input::default();
    set_key(&mut input);
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        state,
        &input,
        focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
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

fn right_arrow_pos(rect: Rect, label: &str) -> Vec2 {
    let char_width = 8.0; // default advance in TestTextBackend
    let pad_x = 10.0;
    let label_w = (label.len() as f32) * char_width + pad_x * 2.0;
    let value_x = rect.x + label_w;
    let value_w = (rect.w - label_w).max(20.0);
    Vec2::new(value_x + value_w - 10.0, rect.y + rect.h / 2.0)
}

fn value_text_pos(rect: Rect, label: &str, formatted_value: &str) -> Vec2 {
    let char_width = 8.0; // default advance in TestTextBackend
    let pad_x = 10.0;
    let label_w = (label.len() as f32) * char_width + pad_x * 2.0;
    let value_x = rect.x + label_w;
    let value_w = (rect.w - label_w).max(20.0);
    let text_w = formatted_value.len() as f32 * char_width;
    Vec2::new(
        value_x + (value_w - text_w) * 0.5 + text_w * 0.5,
        rect.y + rect.h * 0.5,
    )
}

fn enter_edit_state(state: &mut DragNumberState, text: &str) {
    let mut text_edit = TextEditState::new(text);
    text_edit.focus_id = state.focus_id;
    state.edit = DragNumberEditState::Editing {
        text_edit,
        error: false,
    };
}

fn assert_inactive(edit: &DragNumberEditState) {
    assert!(
        matches!(edit, DragNumberEditState::Inactive),
        "expected Inactive, got {edit:?}"
    );
}

fn assert_remembered(edit: &DragNumberEditState, expected: &str) {
    match edit {
        DragNumberEditState::Remembered { draft } => assert_eq!(draft, expected),
        other => panic!("expected Remembered {{ draft: {expected:?} }}, got {other:?}"),
    }
}

fn assert_editing(edit: &DragNumberEditState) -> (&TextEditState, bool) {
    match edit {
        DragNumberEditState::Editing { text_edit, error } => (text_edit, *error),
        other => panic!("expected Editing, got {other:?}"),
    }
}

fn assert_editing_mut(edit: &mut DragNumberEditState) -> (&mut TextEditState, &mut bool) {
    match edit {
        DragNumberEditState::Editing { text_edit, error } => (text_edit, error),
        other => panic!("expected Editing, got {other:?}"),
    }
}

fn drag_num<'a, F>(spec: DragNumberSpec<'a, F>, value: f32) -> (raw::DragNumberResult, DrawCommands)
where
    F: Fn(f32) -> String,
{
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
fn test_drag_number_custom_formatter_affects_measurement_and_rendering() {
    let style = DragNumberStyle::from_theme(&crate::theme::Theme::framewise());
    let default_formatter = default_drag_number_value_formatter;
    let integer_formatter = |v: f32| format!("{v:.0}");
    let default_spec = raw::DragNumberPreLayoutSpec {
        text: "X",
        style,
        min: 0.0,
        max: 100.0,
        value_formatter: &default_formatter,
    };
    let integer_spec = raw::DragNumberPreLayoutSpec {
        text: "X",
        style,
        min: 0.0,
        max: 100.0,
        value_formatter: &integer_formatter,
    };
    let mut text_backend = TestTextBackend::default();
    let default_size = raw::pre_layout_drag_number(
        &default_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;
    let integer_size = raw::pre_layout_drag_number(
        &integer_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;

    assert!(
        integer_size.preferred.unwrap().x < default_size.preferred.unwrap().x,
        "integer formatter should request less width than 2dp formatter"
    );

    let spec = raw::DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        step: 1.0,
        page_step: 10.0,
        value_formatter: integer_formatter,
        time: 0.0,
        disabled: false,
        style,
        clip_rect: None,
    };
    let (_res, cmds) = drag_num(spec, 50.0);

    assert_eq!(
        cmds.glyphs()[1..],
        [
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 66.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 74.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_drag_number_visual_normal() {
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let (res, cmds) = drag_num(spec, 50.5);

    assert_eq!(res.cursor_icon, None);

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
                rect: Rect::new(38.0, 10.0, 36.36, 28.0),
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
                token: PreparedGlyphToken(53),
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
fn test_drag_number_visual_editing() {
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut state = DragNumberState {
        value: 50.5,
        ..Default::default()
    };
    enter_edit_state(&mut state, "50.5");

    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let focus_style = style.focus.unwrap();
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::BorderRect {
                rect: Rect::new(11.0, 11.0, 98.0, 26.0),
                color: focus_style.stroke.color,
                width: focus_style.stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
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
                rect: Rect::new(38.0, 10.0, 72.0, 28.0),
                color: style.text_edit_style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(38.0, 10.0, 72.0, 28.0),
            },
            DrawCmd::FillRect {
                rect: Rect::new(58.0, 16.0, 32.0, 16.0),
                color: style.text_edit_style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..5,
                color: style.text_edit_style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(90.0, 16.0, 2.0, 16.0),
                color: style.text_edit_style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
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
                top_left: Vec2 { x: 58.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 66.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 74.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 82.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_drag_number_visual_hovered_value_area_draws_arrows() {
    let spec = DragNumberSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        text: "X",
        min: 0.0,
        max: 100.0,
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let arrow_color = Color::linear_rgba(
        style.value_text.r,
        style.value_text.g,
        style.value_text.b,
        style.value_text.a * 0.55,
    );
    let input = Input {
        mouse_pos: value_area_pos(spec.rect, spec.text),
        ..Default::default()
    };
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    // Warmup frame: claim hover for this widget so the next frame is active hover.
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

    // Hover frame: the value area should draw subtle left/right arrow glyphs.
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let res = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(res.cursor_icon, Some(crate::output::CursorIcon::EwResize));
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
            DrawCmd::GlyphRun {
                glyphs: 6..7,
                color: arrow_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 7..8,
                color: arrow_color,
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
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 44.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
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
    let res = raw::post_layout_drag_number(
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

    assert_eq!(res.cursor_icon, Some(crate::output::CursorIcon::EwResize));

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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let result = raw::post_layout_drag_number(
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

    assert_eq!(result.cursor_icon, None);
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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
        disabled: false,
        style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 28.0)),
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let result = raw::post_layout_drag_number(
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

    assert_eq!(result.cursor_icon, None);
    assert_eq!(
        focus_system.current_keyboard_focus(),
        None,
        "Clicking a clipped-away drag number must not take focus"
    );
}

#[test]
fn test_drag_number_focused_step_keys() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 5.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_right = true;
    });
    assert_eq!(state.value, 55.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_down = true;
    });
    assert_eq!(state.value, 60.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_left = true;
    });
    assert_eq!(state.value, 55.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_up = true;
    });
    assert_eq!(state.value, 50.0);
}

#[test]
fn test_drag_number_focused_page_home_end_keys() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_page_up = true;
    });
    assert_eq!(state.value, 30.0);
    assert!(focus_system.is_active_pgup_vert(state.focus_id));
    assert!(focus_system.is_active_pgdn_vert(state.focus_id));
    assert!(focus_system.is_active_pgup_horiz(state.focus_id));
    assert!(focus_system.is_active_pgdn_horiz(state.focus_id));

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_page_down = true;
    });
    assert_eq!(state.value, 50.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_home = true;
    });
    assert_eq!(state.value, 0.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_end = true;
    });
    assert_eq!(state.value, 100.0);
}

#[test]
fn test_drag_number_keyboard_adjustment_clamps_to_bounds() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 95.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 10.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_right = true;
    });
    assert_eq!(state.value, 100.0);

    state.value = 5.0;
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_left = true;
    });
    assert_eq!(state.value, 0.0);
}

#[test]
fn test_drag_number_keyboard_adjustment_ignored_when_not_focused() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 5.0;
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_right = true;
    });
    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_page_down = true;
    });
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_end = true;
    });

    assert_eq!(state.value, 50.0);
}

#[test]
fn test_drag_number_keyboard_adjustment_ignored_when_disabled() {
    let mut focus_system = FocusSystem::new();
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.disabled = true;
    spec.step = 5.0;
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_right = true;
    });
    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.key_pressed_page_down = true;
    });
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_end = true;
    });

    assert_eq!(state.value, 50.0);
}

#[test]
fn test_spec_default_from_theme_applies_theme_style() {
    let theme = crate::theme::Theme::framewise();
    let spec = super::DragNumberSpec::default_from_theme(&theme);
    assert_eq!(spec.style, DragNumberStyle::from_theme(&theme));
    assert_eq!(spec.min, 0.0);
    assert_eq!(spec.max, 100.0);
    assert_eq!(spec.step, 1.0);
    assert_eq!(spec.page_step, 10.0);
    assert!(!spec.disabled);
}

#[test]
fn test_spec_theme_overwrites_style_only() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = DragNumberStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let spec = super::DragNumberSpec::default()
        .text("x")
        .style(custom_style)
        .min(5.0)
        .max(10.0)
        .step(2.0)
        .page_step(7.0)
        .disabled(true)
        .theme(&theme);
    assert_eq!(spec.style, DragNumberStyle::from_theme(&theme));
    assert_eq!(spec.text, "x");
    assert_eq!(spec.min, 5.0);
    assert_eq!(spec.max, 10.0);
    assert_eq!(spec.step, 2.0);
    assert_eq!(spec.page_step, 7.0);
    assert!(spec.disabled);
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
        super::DragNumberSpec::new_from_theme("x", &ctx.theme),
        placement,
        &mut dn_state,
        &mut ctx,
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
fn test_drag_number_arrow_hold_repeat_sequence() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut spec = default_spec(rect);
    spec.step = 5.0;
    let arrow_pos = right_arrow_pos(rect, "X");

    // Warmup frame: hover the right arrow so the focus system grants active hover.
    let mut input = Input {
        mouse_pos: arrow_pos,
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

    // Frame 1: press the right arrow; it steps immediately and schedules repeat.
    input.mouse_down = true;
    input.mouse_pressed = true;
    spec.time = 0.0;
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
    assert_eq!(state.value, 55.0);
    assert!(state.is_arrow_stepping);
    assert_eq!(
        state.arrow_step_direction,
        Some(DragNumberStepDirection::Increment)
    );
    assert_eq!(state.next_repeat_time, 0.5);

    // Frame 2: keep holding before the repeat delay; value should not change.
    input.mouse_pressed = false;
    spec.time = 0.4;
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
    assert_eq!(state.value, 55.0);

    // Frame 3: reach the initial repeat time; one repeat step should fire.
    spec.time = 0.5;
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
    assert_eq!(state.value, 60.0);
    assert_eq!(state.next_repeat_time, 0.55);

    // Frame 4: continue holding past the fast repeat interval; another step fires.
    spec.time = 0.6;
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
    assert_eq!(state.value, 65.0);

    // Frame 5: release the mouse; arrow-step state is cleared.
    input.mouse_down = false;
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
    assert!(!state.is_arrow_stepping);
    assert_eq!(state.arrow_step_direction, None);
}

#[test]
fn test_drag_number_arrow_step_promotes_to_drag_after_motion_threshold() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut spec = default_spec(rect);
    spec.step = 5.0;
    let arrow_pos = right_arrow_pos(rect, "X");

    // Warmup frame: hover the right arrow so the press goes to this widget.
    let mut input = Input {
        mouse_pos: arrow_pos,
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

    // Frame 1: press the right arrow; it steps once and remains in arrow mode.
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
    assert_eq!(state.value, 55.0);
    assert!(state.is_arrow_stepping);
    assert!(!state.is_dragging);

    // Frame 2: move less than the 4px threshold; stay in arrow-step mode.
    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(arrow_pos.x + 3.0, arrow_pos.y);
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
    assert!(state.is_arrow_stepping);
    assert!(!state.is_dragging);

    // Frame 3: move beyond the threshold; promote to normal drag from current value.
    input.mouse_pos = Vec2::new(arrow_pos.x + 6.0, arrow_pos.y);
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
    assert!(!state.is_arrow_stepping);
    assert_eq!(state.arrow_step_direction, None);
    assert!(state.is_dragging);
    assert_eq!(state.drag_start_x, input.mouse_pos.x);
    assert_eq!(state.drag_start_value, 55.0);

    // Frame 4: keep dragging; value follows the existing drag-number scrub formula.
    input.mouse_pos = Vec2::new(arrow_pos.x + 13.0, arrow_pos.y);
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
    let expected = 55.0 + (7.0 / 72.0) * 100.0;
    assert!((state.value - expected).abs() < 0.0001);

    // Release: normal drag cleanup should run.
    input.mouse_down = false;
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
    assert!(!state.is_dragging);
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
        step: 1.0,
        page_step: 10.0,
        value_formatter: default_drag_number_value_formatter,
        time: 0.0,
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
fn test_drag_number_label_area_takes_focus_without_dragging() {
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
        !state.is_dragging,
        "Pressing inside label area should not start dragging"
    );
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Pressing inside label area should still take focus"
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

#[test]
fn test_drag_number_double_click_value_text_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 72.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let formatter: fn(f32) -> String = |v| format!("{v:.0} px");
    let spec = raw::DragNumberSpec {
        value_formatter: formatter,
        ..default_spec(rect)
    };
    let pos = value_text_pos(rect, "X", "72 px");

    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
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

    let (text_edit, error) = assert_editing(&state.edit);
    assert_eq!(text_edit.focus_id, state.focus_id);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert_eq!(text_edit.value, "72");
    assert!(!error);
}

#[test]
fn test_drag_number_focused_enter_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 72.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    let formatter: fn(f32) -> String = |v| format!("{v:.0} px");
    let spec = raw::DragNumberSpec {
        value_formatter: formatter,
        ..default_spec(rect)
    };

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });

    let (text_edit, error) = assert_editing(&state.edit);
    assert_eq!(text_edit.focus_id, state.focus_id);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert_eq!(text_edit.value, "72");
    assert!(!error);
}

#[test]
fn test_drag_number_double_click_value_drag_region_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 300.0, 28.0);
    let mut state = DragNumberState {
        value: 5.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let formatter: fn(f32) -> String = |v| format!("{v:.0}");
    let spec = raw::DragNumberSpec {
        value_formatter: formatter,
        ..default_spec(rect)
    };
    let pos = Vec2::new(60.0, rect.y + rect.h * 0.5);

    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
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

    let _ = assert_editing(&state.edit);
}

#[test]
fn test_drag_number_double_click_arrow_does_not_enter_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let spec = default_spec(rect);
    let pos = right_arrow_pos(rect, "X");
    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
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

    assert_inactive(&state.edit);
}

#[test]
fn test_drag_number_text_edit_enter_commits_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });

    assert_eq!(state.value, 42.5);
    assert_inactive(&state.edit);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_drag_number_text_edit_enter_clamps_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "150");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });
    assert_eq!(state.value, 100.0);
    assert_inactive(&state.edit);

    enter_edit_state(&mut state, "-20");
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });
    assert_eq!(state.value, 0.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_drag_number_text_edit_enter_invalid_keeps_editing_and_sets_error() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "NaN");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });

    assert_eq!(state.value, 10.0);
    let (_, error) = assert_editing(&state.edit);
    assert!(error);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_drag_number_text_edit_escape_cancels() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_escape = true;
    });

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_drag_number_text_edit_click_outside_valid_commits_without_stealing_focus() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    let other_focus = FocusId::new();
    focus_system.take_keyboard_focus(other_focus);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.mouse_pos = Vec2::new(5.0, 14.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
    });

    assert_eq!(state.value, 42.5);
    assert_inactive(&state.edit);
    assert_eq!(focus_system.current_keyboard_focus(), Some(other_focus));
}

#[test]
fn test_drag_number_text_edit_click_outside_invalid_remembers_and_reenter_restores_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "abc");
    let mut focus_system = FocusSystem::new();
    let other_focus = FocusId::new();
    focus_system.take_keyboard_focus(other_focus);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.mouse_pos = Vec2::new(5.0, 14.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
    });

    assert_eq!(state.value, 10.0);
    assert_remembered(&state.edit, "abc");
    assert_eq!(focus_system.current_keyboard_focus(), Some(other_focus));

    let pos = value_text_pos(rect, "X", "10.00");
    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    focus_system.begin_frame();
    let _ = run_raw(
        default_spec(rect),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        default_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let (text_edit, error) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");
    assert!(!error);
    assert_eq!(text_edit.focus_id, state.focus_id);

    let (text_edit, _) = assert_editing_mut(&mut state.edit);
    text_edit.value = "42".to_string();
    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_enter = true;
    });

    assert_eq!(state.value, 42.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_drag_number_text_edit_focus_lost_commits_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(FocusId::new());

    run_key(default_spec(rect), &mut state, &mut focus_system, |_| {});

    assert_eq!(state.value, 42.5);
    assert_inactive(&state.edit);
}

#[test]
fn test_drag_number_text_edit_focus_lost_invalid_remembers_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "abc");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(FocusId::new());

    run_key(default_spec(rect), &mut state, &mut focus_system, |_| {});

    assert_eq!(state.value, 10.0);
    assert_remembered(&state.edit, "abc");
}

#[test]
fn test_drag_number_text_edit_arrow_keys_do_not_step_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "10");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_right = true;
        input.text_events.push(TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        });
    });

    assert_eq!(state.value, 10.0);
    let _ = assert_editing(&state.edit);
}

#[test]
fn test_drag_number_text_edit_disabled_exits_edit_mode() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        is_dragging: true,
        is_arrow_stepping: true,
        arrow_step_direction: Some(DragNumberStepDirection::Increment),
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    let mut spec = default_spec(rect);
    spec.disabled = true;

    run_key(spec, &mut state, &mut focus_system, |_| {});

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
    assert!(!state.is_dragging);
    assert!(!state.is_arrow_stepping);
}

#[test]
fn test_drag_number_text_edit_disabled_clears_remembered_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        edit: DragNumberEditState::Remembered {
            draft: "abc".into(),
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut spec = default_spec(rect);
    spec.disabled = true;

    run_key(spec, &mut state, &mut focus_system, |_| {});

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_drag_number_activation_frame_does_not_text_edit_double_click_select_word() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = DragNumberState {
        value: 12.34,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let spec = default_spec(rect);
    let pos = value_text_pos(rect, "X", "12.34");

    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
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

    let (text_edit, _) = assert_editing(&state.edit);
    let caret = text_edit.caret.insertion_byte_hint();
    let selection = text_edit
        .selection_anchor
        .map(|anchor| anchor.insertion_byte_hint());
    assert!(
        selection.is_none() || selection == Some(0) && caret == text_edit.value.len(),
        "activation double-click should not create a partial word selection"
    );
}

#[test]
fn test_drag_number_text_edit_escape_clears_restored_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = DragNumberState {
        value: 10.0,
        edit: DragNumberEditState::Remembered {
            draft: "abc".into(),
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let pos = value_text_pos(rect, "X", "10.00");
    let warmup_input = Input {
        mouse_pos: pos,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        default_spec(rect),
        &mut state,
        &warmup_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: pos,
        mouse_down: true,
        mouse_pressed: true,
        mouse_click_count: 2,
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        default_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let (text_edit, _) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.key_pressed_escape = true;
    });

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
}
