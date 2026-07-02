use super::raw::NumberEditSpec;
use super::*;
use crate::focus::FocusId;
use crate::input::TextEvent;
use crate::test_utils::TestTextBackend;
use crate::types::Vec2;
use crate::{DrawGlyph, PreparedGlyphToken};

fn default_style() -> NumberEditStyle {
    NumberEditStyle::from_theme(&crate::theme::Theme::framewise())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LegacyNumberEditTextConverter;

impl NumberEditTextConverter for LegacyNumberEditTextConverter {
    fn display_text(&self, value: f32) -> String {
        format!("{value:.2}")
    }

    fn edit_text(&self, value: f32) -> String {
        value.to_string()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FrameDisplayLegacyEditTextConverter;

impl NumberEditTextConverter for FrameDisplayLegacyEditTextConverter {
    fn display_text(&self, value: f32) -> String {
        format!("Frame {value:.0}")
    }

    fn edit_text(&self, value: f32) -> String {
        value.to_string()
    }
}

#[derive(Debug, Clone, Copy)]
struct DisplayFnLegacyEditTextConverter {
    display_text: fn(f32) -> String,
}

impl NumberEditTextConverter for DisplayFnLegacyEditTextConverter {
    fn display_text(&self, value: f32) -> String {
        (self.display_text)(value)
    }

    fn edit_text(&self, value: f32) -> String {
        value.to_string()
    }
}

fn spec_with_text_converter<C: NumberEditTextConverter>(
    rect: Rect,
    text_converter: C,
) -> raw::NumberEditSpec<C> {
    raw::NumberEditSpec {
        layer: Layer::default(),
        rect,
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter,
        time: 0.0,
        disabled: false,
        style: default_style(),
        clip_rect: None,
    }
}

fn default_spec(rect: Rect) -> raw::NumberEditSpec<LegacyNumberEditTextConverter> {
    spec_with_text_converter(rect, LegacyNumberEditTextConverter)
}

fn run_raw<F>(
    spec: raw::NumberEditSpec<F>,
    state: &mut NumberEditState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut TestTextBackend,
    cmds: &mut DrawCommands,
) -> raw::NumberEditResult
where
    F: NumberEditTextConverter,
{
    raw::post_layout_number_edit(
        spec,
        raw::NumberEditPreLayoutResult {
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
    spec: raw::NumberEditSpec<F>,
    state: &mut NumberEditState,
    focus_system: &mut FocusSystem,
    set_key: impl FnOnce(&mut Input),
) where
    F: NumberEditTextConverter,
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

fn value_area_pos(rect: Rect) -> Vec2 {
    Vec2::new(rect.x + rect.w / 2.0, rect.y + rect.h / 2.0)
}

fn right_arrow_pos(rect: Rect) -> Vec2 {
    Vec2::new(rect.right() - 10.0, rect.y + rect.h / 2.0)
}

fn left_arrow_pos(rect: Rect) -> Vec2 {
    Vec2::new(rect.x + 10.0, rect.y + rect.h / 2.0)
}

fn value_text_pos(rect: Rect, formatted_value: &str) -> Vec2 {
    let char_width = 8.0; // default advance in TestTextBackend
    let text_w = formatted_value.len() as f32 * char_width;
    Vec2::new(
        rect.x + (rect.w - text_w) * 0.5 + text_w * 0.5,
        rect.y + rect.h * 0.5,
    )
}

fn enter_edit_state(state: &mut NumberEditState, text: &str) {
    let mut text_edit = TextEditState::new(text);
    text_edit.focus_id = state.focus_id;
    state.edit = NumberEditEditState::Editing {
        text_edit,
        error: false,
        dirty: true,
    };
}

fn assert_inactive(edit: &NumberEditEditState) {
    assert!(
        matches!(edit, NumberEditEditState::Inactive),
        "expected Inactive, got {edit:?}"
    );
}

fn assert_remembered(edit: &NumberEditEditState, expected: &str) {
    match edit {
        NumberEditEditState::Remembered { draft } => assert_eq!(draft, expected),
        other => panic!("expected Remembered {{ draft: {expected:?} }}, got {other:?}"),
    }
}

fn assert_editing(edit: &NumberEditEditState) -> (&TextEditState, bool, bool) {
    match edit {
        NumberEditEditState::Editing {
            text_edit,
            error,
            dirty,
        } => (text_edit, *error, *dirty),
        other => panic!("expected Editing, got {other:?}"),
    }
}

fn assert_editing_mut(
    edit: &mut NumberEditEditState,
) -> (&mut TextEditState, &mut bool, &mut bool) {
    match edit {
        NumberEditEditState::Editing {
            text_edit,
            error,
            dirty,
        } => (text_edit, error, dirty),
        other => panic!("expected Editing, got {other:?}"),
    }
}

fn selection_byte(text_edit: &TextEditState) -> Option<usize> {
    text_edit
        .selection_anchor
        .map(|anchor| anchor.insertion_byte_hint())
}

fn number_edit<F>(spec: NumberEditSpec<F>, value: f32) -> (raw::NumberEditResult, DrawCommands)
where
    F: NumberEditTextConverter,
{
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let res = raw::post_layout_number_edit(
        spec,
        raw::NumberEditPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut NumberEditState {
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
fn test_number_edit_measurement_and_rendering() {
    let style = NumberEditStyle::from_theme(&crate::theme::Theme::framewise());
    let default_formatter = DefaultNumberEditTextConverter;
    let integer_formatter = |v: f32| format!("{v:.0}");
    let default_spec = raw::NumberEditPreLayoutSpec {
        style,
        value: 50.0,
        step_buttons_enabled: true,
        text_converter: &default_formatter,
    };
    let integer_spec = raw::NumberEditPreLayoutSpec {
        style,
        value: 50.0,
        step_buttons_enabled: true,
        text_converter: &integer_formatter,
    };
    let no_step_buttons_spec = raw::NumberEditPreLayoutSpec {
        step_buttons_enabled: false,
        ..default_spec
    };
    let mut text_backend = TestTextBackend::default();
    let default_size = raw::pre_layout_number_edit(
        &default_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;
    let integer_size = raw::pre_layout_number_edit(
        &integer_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;
    let no_step_buttons_size = raw::pre_layout_number_edit(
        &no_step_buttons_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;

    assert!(
        integer_size.preferred.unwrap().x < default_size.preferred.unwrap().x,
        "integer formatter should request less width than default formatter"
    );
    let default_w = default_size.preferred.unwrap().x;
    let no_step_buttons_w = no_step_buttons_size.preferred.unwrap().x;
    let expected_no_step_buttons_w = "50.00".len() as f32 * 8.0 + style.text_pad_x * 2.0;
    assert!(default_w > no_step_buttons_w);
    assert_eq!(no_step_buttons_w, expected_no_step_buttons_w);

    let spec = raw::NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: integer_formatter,
        time: 0.0,
        disabled: false,
        style,
        clip_rect: None,
    };
    let (_res, cmds) = number_edit(spec, 50.0);

    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 52.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 60.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_visual_normal() {
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let (res, cmds) = number_edit(spec, 50.5);

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
                rect: Rect::new(30.0, 10.0, 30.3, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 6..7,
                color: style.step_button.glyph_color,
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
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 40.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 48.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 56.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 64.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 72.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_visual_without_step_buttons_uses_full_value_rect() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.step_buttons_enabled = false;
    spec.value_fill_enabled = true;
    spec.text_entry_mode = NumberEditTextEntryMode::OnDemand;
    let style = spec.style;

    let (_res, cmds) = number_edit(spec, 50.0);

    assert!(cmds.commands().contains(&DrawCmd::FillRect {
        rect,
        color: style.background,
        z: 0,
    }));
    assert!(cmds.commands().contains(&DrawCmd::FillRect {
        rect: Rect::new(10.0, 10.0, 50.0, 28.0),
        color: style.value_fill,
        z: 0,
    }));
    assert!(cmds.commands().contains(&DrawCmd::BorderRect {
        rect,
        color: style.border.unwrap().color,
        width: style.border.unwrap().width,
        placement: crate::BorderPlacement::Inside,
        z: 0,
    }));
    assert!(cmds
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, DrawCmd::GlyphRun { color, .. } if *color == style.value_text)));
    assert!(!cmds.commands().iter().any(
        |cmd| matches!(cmd, DrawCmd::GlyphRun { color, .. } if *color == style.step_button.glyph_color)
    ));
    assert!(!cmds.glyphs().iter().any(|glyph| {
        glyph.token == PreparedGlyphToken(8249) || glyph.token == PreparedGlyphToken(8250)
    }));
}

#[test]
fn test_number_edit_visual_editing() {
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut state = NumberEditState {
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
                rect: Rect::new(30.0, 10.0, 60.0, 28.0),
                color: style.text_edit_style.background,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(30.0, 10.0, 60.0, 28.0),
            },
            DrawCmd::FillRect {
                rect: Rect::new(44.0, 16.0, 32.0, 16.0),
                color: style.text_edit_style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..4,
                color: style.text_edit_style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(76.0, 16.0, 2.0, 16.0),
                color: style.text_edit_style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::GlyphRun {
                glyphs: 4..5,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
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
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 44.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 52.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 60.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 68.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_always_without_step_buttons_editor_uses_full_rect() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.step_buttons_enabled = false;
    spec.text_entry_mode = NumberEditTextEntryMode::Always;
    let mut state = NumberEditState {
        value: 12.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
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

    assert!(state.edit.is_editing());
    assert!(cmds
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, DrawCmd::PushClip { rect: clip } if *clip == rect)));
    assert!(!cmds.glyphs().iter().any(|glyph| {
        glyph.token == PreparedGlyphToken(8249) || glyph.token == PreparedGlyphToken(8250)
    }));
}

#[test]
fn test_number_edit_visual_editing_hovered() {
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut state = NumberEditState {
        value: 50.5,
        ..Default::default()
    };
    enter_edit_state(&mut state, "50.5");

    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let input = Input {
        mouse_pos: value_area_pos(spec.rect),
        ..Default::default()
    };

    // Warmup frame: claim hover for the embedded text edit.
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

    // Hover frame: the embedded TextEdit should draw using background_hovered.
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
                rect: Rect::new(30.0, 10.0, 60.0, 28.0),
                color: style.text_edit_style.background_hovered,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: Rect::new(30.0, 10.0, 60.0, 28.0),
            },
            DrawCmd::FillRect {
                rect: Rect::new(44.0, 16.0, 32.0, 16.0),
                color: style.text_edit_style.select_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..4,
                color: style.text_edit_style.text_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(76.0, 16.0, 2.0, 16.0),
                color: style.text_edit_style.caret_color,
                z: 0,
            },
            DrawCmd::PopClip,
            DrawCmd::GlyphRun {
                glyphs: 4..5,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
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
}

#[test]
fn test_number_edit_visual_hovered_value_area_draws_arrows() {
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
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
        mouse_pos: value_area_pos(spec.rect),
        ..Default::default()
    };
    let mut state = NumberEditState {
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
                rect: Rect::new(30.0, 10.0, 30.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: arrow_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 6..7,
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
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 40.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 48.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 56.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 64.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 72.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_visual_active() {
    let mut state = NumberEditState {
        value: 50.0,
        drag_start_value: 50.0,
        press_drag: crate::widgets::widget_helpers::PressDragState {
            dragging: true,
            drag_start_pos: Vec2::new(50.0, 24.0),
            ..Default::default()
        },
        ..Default::default()
    };
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let input = Input {
        mouse_down: true,
        mouse_pos: Vec2::new(50.0, 24.0),
        ..Default::default()
    };
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let res = raw::post_layout_number_edit(
        spec.clone(),
        raw::NumberEditPreLayoutResult {
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
                rect: Rect::new(30.0, 10.0, 30.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 6..7,
                color: style.step_button.glyph_color,
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
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 40.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 48.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 56.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 64.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 72.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_visual_min_value() {
    let mut text_backend = TestTextBackend::default();
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let style = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    let _res = raw::post_layout_number_edit(
        spec,
        raw::NumberEditPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut NumberEditState::default(),
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
            DrawCmd::GlyphRun {
                glyphs: 0..4,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 4..5,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
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
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 44.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 52.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 60.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 68.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8249),
                top_left: Vec2 { x: 16.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(8250),
                top_left: Vec2 { x: 96.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_click_takes_focus() {
    let state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let input = Input {
        mouse_pos: Vec2::new(50.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
    focus_system.begin_frame();
    let result = raw::post_layout_number_edit(
        spec,
        raw::NumberEditPreLayoutResult {
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
        result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Clicking number edit must request focus"
    );
}

#[test]
fn test_number_edit_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = NumberEditSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 28.0),
        min: Some(0.0),
        max: Some(100.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: NumberEditStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 28.0)),
    };

    let mut state = state;
    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let result = raw::post_layout_number_edit(
        spec,
        raw::NumberEditPreLayoutResult {
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
        "Clicking a clipped-away number edit must not take focus"
    );
}

#[test]
fn test_number_edit_focused_step_keys() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 5.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    assert_eq!(state.value, 55.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowDown);
    });
    assert_eq!(state.value, 60.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    });
    assert_eq!(state.value, 55.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowUp);
    });
    assert_eq!(state.value, 50.0);
}

#[test]
fn test_number_edit_focused_page_home_end_keys() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::PageUp);
    });
    assert_eq!(state.value, 30.0);
    assert!(focus_system.active_page_dirs(state.focus_id).up);
    assert!(focus_system.active_page_dirs(state.focus_id).down);
    assert!(focus_system.active_page_dirs(state.focus_id).left);
    assert!(focus_system.active_page_dirs(state.focus_id).right);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::PageDown);
    });
    assert_eq!(state.value, 50.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Home);
    });
    assert_eq!(state.value, 0.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::End);
    });
    assert_eq!(state.value, 100.0);
}

#[test]
fn test_number_edit_keyboard_adjustment_clamps_to_bounds() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 95.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 10.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    assert_eq!(state.value, 100.0);

    state.value = 5.0;
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    });
    assert_eq!(state.value, 0.0);
}

#[test]
fn test_number_edit_keyboard_adjustment_ignored_when_not_focused() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.step = 5.0;
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::PageDown);
    });
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::End);
    });

    assert_eq!(state.value, 50.0);
}

#[test]
fn test_number_edit_keyboard_adjustment_ignored_when_disabled() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.disabled = true;
    spec.step = 5.0;
    spec.page_step = 20.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::PageDown);
    });
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::End);
    });

    assert_eq!(state.value, 50.0);
}

#[test]
fn test_spec_new_from_theme_applies_theme_style() {
    let theme = crate::theme::Theme::framewise();
    let spec = super::NumberEditSpec::new_from_theme(&theme);
    assert_eq!(spec.style, NumberEditStyle::from_theme(&theme));
    assert_eq!(spec.min, Some(0.0));
    assert_eq!(spec.max, Some(100.0));
    assert_eq!(spec.step, 1.0);
    assert_eq!(spec.page_step, 10.0);
    assert!(spec.drag_enabled);
    assert!(spec.step_buttons_enabled);
    assert!(spec.value_fill_enabled);
    assert!(!spec.disabled);
}

#[test]
fn test_spec_theme_overwrites_style_only() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = NumberEditStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let spec = super::NumberEditSpec::default()
        .style(custom_style)
        .min(5.0)
        .max(10.0)
        .step(2.0)
        .page_step(7.0)
        .disabled(true)
        .theme(&theme);
    assert_eq!(spec.style, NumberEditStyle::from_theme(&theme));
    assert_eq!(spec.min, Some(5.0));
    assert_eq!(spec.max, Some(10.0));
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
    let mut dn_state = NumberEditState::default();
    let result = super::number_edit(
        super::NumberEditSpec::new_from_theme(&ctx.theme),
        placement,
        &mut dn_state,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_prefixed_number_edit_visual_appearance() {
    use crate::layouts::ManualLayout;

    let theme = crate::theme::Theme::framewise();
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut output = crate::Output::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let placement = Rect::new(10.0, 20.0, 128.0, 28.0);
    let mut ctx = crate::widget::WidgetContext::root(
        theme,
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };

    let result = super::prefixed_number_edit(
        "X",
        super::NumberEditSpec::new_from_theme(&theme),
        placement,
        &mut state,
        &mut ctx,
    );

    assert_eq!(result.layout.bounds, placement);
    assert!(ctx.cmds.commands().contains(&DrawCmd::FillRect {
        rect: Rect::new(10.0, 20.0, 28.0, 28.0),
        color: theme.ink,
        z: 0,
    }));
    assert!(ctx
        .cmds
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, DrawCmd::GlyphRun { color, .. } if *color == theme.paper)));
    assert!(ctx.cmds.commands().contains(&DrawCmd::BorderRect {
        rect: placement,
        color: theme.ink,
        width: theme.border,
        placement: BorderPlacement::Inside,
        z: 0,
    }));
    assert!(ctx
        .cmds
        .glyphs()
        .iter()
        .any(|glyph| glyph.token == PreparedGlyphToken(53)));
    assert!(ctx.cmds.glyphs().iter().any(|glyph| *glyph
        == DrawGlyph {
            token: PreparedGlyphToken(88),
            top_left: Vec2 { x: 20.0, y: 38.0 },
        }));
}

#[test]
fn test_prefixed_number_edit_prefix_click_focuses() {
    use crate::layouts::ManualLayout;

    let theme = crate::theme::Theme::framewise();
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let hover_input = crate::Input {
        mouse_pos: Vec2::new(24.0, 34.0),
        ..Default::default()
    };
    let press_input = crate::Input {
        mouse_pos: Vec2::new(24.0, 34.0),
        mouse_pressed: true,
        mouse_down: true,
        ..Default::default()
    };
    let mut output = crate::Output::default();
    let placement = Rect::new(10.0, 20.0, 128.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let focus_id = state.focus_id;

    {
        focus.begin_frame();
        let mut cmds = crate::draw::DrawCommands::new(1.0);
        let mut ctx = crate::widget::WidgetContext::root(
            theme,
            &mut text_backend,
            &mut focus,
            &hover_input,
            &mut output,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        super::prefixed_number_edit(
            "X",
            super::NumberEditSpec::new_from_theme(&theme),
            placement,
            &mut state,
            &mut ctx,
        );
        focus.end_frame();
    }

    focus.begin_frame();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let mut ctx = crate::widget::WidgetContext::root(
        theme,
        &mut text_backend,
        &mut focus,
        &press_input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let result = super::prefixed_number_edit(
        "X",
        super::NumberEditSpec::new_from_theme(&theme),
        placement,
        &mut state,
        &mut ctx,
    );

    assert!(result.focused);
    assert_eq!(ctx.focus_system.current_keyboard_focus(), Some(focus_id));
    ctx.focus_system.end_frame();
}

#[test]
fn test_number_edit_disabled_ignores_press_interaction() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let focus_id = state.focus_id;
    let inside_pos = value_area_pos(rect);

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
fn test_number_edit_clipped_press_does_not_take_focus_with_helper() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let inside_pos = value_area_pos(rect);

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
fn test_number_edit_mouse_press_takes_focus_with_helper() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let focus_id = state.focus_id;
    let inside_pos = value_area_pos(rect);

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
fn test_number_edit_drag_updates_value() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
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
    let res = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(
        state.press_drag.dragging,
        "Should start dragging on mouse press"
    );
    assert_eq!(res.cursor_icon, Some(crate::output::CursorIcon::EwResize));
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

    assert!(state.press_drag.dragging, "Should still be dragging");
    // dx = 68 - 50 = 18. value_w = 60 after step buttons. dx/value_w = 0.3.
    assert_eq!(
        state.value, 80.0,
        "Value should update proportionally to mouse drag"
    );
}

#[test]
fn test_number_edit_without_step_buttons_old_button_region_drags_value() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.step_buttons_enabled = false;
    spec.drag_enabled = true;
    spec.text_entry_mode = NumberEditTextEntryMode::OnDemand;
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut input = Input {
        mouse_pos: right_arrow_pos(rect),
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

    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 50.0);
    assert!(!state.is_arrow_stepping);
    assert_eq!(state.arrow_step_direction, None);
    assert!(state.press_drag.dragging);
    assert_eq!(
        result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );

    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(100.0, rect.y + rect.h * 0.5);
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

    assert!((state.value - 60.0).abs() < 0.0001);
}

#[test]
fn test_number_edit_drag_clamps_to_min_max() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
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
    let far_right_result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 100.0, "Value should clamp to max (100.0)");
    assert_eq!(
        far_right_result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );

    // Frame 3: Drag far left (e.g. x = -500.0)
    input.mouse_pos = Vec2::new(-500.0, 14.0);
    focus_system.begin_frame();
    let far_left_result = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 0.0, "Value should clamp to min (0.0)");
    assert_eq!(
        far_left_result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );
}

#[test]
fn test_number_edit_drag_releases_when_mouse_up() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
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
    assert!(state.press_drag.dragging);

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
        !state.press_drag.dragging,
        "Dragging should stop when mouse is released"
    );
}

#[test]
fn test_number_edit_arrow_hold_repeat_sequence() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut spec = default_spec(rect);
    spec.step = 5.0;
    let arrow_pos = right_arrow_pos(rect);

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
    let press_result = run_raw(
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
        Some(NumberEditStepDirection::Increment)
    );
    assert!(press_result.input.pressed);

    // Frame 2: keep holding before the repeat delay; value should not change.
    input.mouse_pressed = false;
    spec.time = 0.4;
    focus_system.begin_frame();
    let hold_result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 55.0);
    assert!(hold_result.input.pressed);

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
    let release_result = run_raw(
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
    assert!(!release_result.input.pressed);
}

#[test]
fn test_number_edit_step_hold_pauses_outside_and_resumes_on_return_when_drag_disabled() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut spec = default_spec(rect);
    spec.drag_enabled = false;
    spec.step = 5.0;
    let arrow_pos = right_arrow_pos(rect);

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

    input.mouse_down = true;
    input.mouse_pressed = true;
    spec.time = 0.0;
    focus_system.begin_frame();
    let press_result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    assert_eq!(state.value, 55.0);
    assert_eq!(
        press_result.cursor_icon,
        Some(crate::output::CursorIcon::Pointer)
    );
    assert!(state.is_arrow_stepping);
    assert_eq!(
        state.arrow_step_direction,
        Some(NumberEditStepDirection::Increment)
    );

    input.mouse_pressed = false;
    input.mouse_pos = Vec2::new(rect.right() + 20.0, rect.y + rect.h / 2.0);
    spec.time = 0.6;
    focus_system.begin_frame();
    let outside_result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 55.0);
    assert_eq!(outside_result.cursor_icon, None);
    assert!(state.is_arrow_stepping);
    assert_eq!(
        state.arrow_step_direction,
        Some(NumberEditStepDirection::Increment)
    );
    assert!(!state.press_drag.dragging);

    input.mouse_pos = arrow_pos;
    spec.time = 0.7;
    focus_system.begin_frame();
    let return_result = run_raw(
        spec.clone(),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.value, 60.0);
    assert_eq!(
        return_result.cursor_icon,
        Some(crate::output::CursorIcon::Pointer)
    );
    assert!(state.is_arrow_stepping);
    assert_eq!(
        state.arrow_step_direction,
        Some(NumberEditStepDirection::Increment)
    );
    assert!(!state.press_drag.dragging);

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

    assert_eq!(state.value, 60.0);
    assert!(!state.is_arrow_stepping);
    assert_eq!(state.arrow_step_direction, None);
}

#[test]
fn test_number_edit_arrow_step_promotes_to_drag_after_motion_threshold() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut spec = default_spec(rect);
    spec.step = 5.0;
    let arrow_pos = right_arrow_pos(rect);

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
    assert!(!state.press_drag.dragging);

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
    assert!(!state.press_drag.dragging);

    // Frame 3: move beyond the threshold; promote to normal drag from current value.
    input.mouse_pos = Vec2::new(arrow_pos.x + 6.0, arrow_pos.y);
    focus_system.begin_frame();
    let result = run_raw(
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
    assert!(state.press_drag.dragging);
    assert_eq!(state.press_drag.drag_start_pos.x, input.mouse_pos.x);
    assert_eq!(state.drag_start_value, 55.0);
    assert_eq!(
        result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );

    // Frame 4: keep dragging; value follows the existing number-edit scrub formula.
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
    let expected = 55.0 + (7.0 / 60.0) * 100.0;
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
    assert!(!state.press_drag.dragging);
}

#[test]
fn test_number_edit_min_greater_than_max_keyboard_does_not_panic() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let spec = raw::NumberEditSpec {
        layer: Layer::default(),
        rect,
        min: Some(100.0),
        max: Some(0.0),
        step: 1.0,
        page_step: 10.0,
        text_entry_mode: NumberEditTextEntryMode::OnDemand,
        drag_enabled: true,
        step_buttons_enabled: true,
        value_fill_enabled: true,
        text_converter: LegacyNumberEditTextConverter,
        time: 0.0,
        disabled: false,
        style: default_style(),
        clip_rect: None,
    };

    let run_res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut state = NumberEditState {
            value: 50.0,
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);

        let input = Input {
            keys_pressed: crate::input::KeySet::from_key(crate::input::Key::ArrowRight),
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
fn test_number_edit_overlapping_hover_uses_top_widget() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state_bottom = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut state_top = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let overlap_pos = value_area_pos(rect);

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
fn test_number_edit_overlapping_press_uses_top_widget() {
    let rect = Rect::new(10.0, 10.0, 100.0, 28.0);
    let mut state_bottom = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut state_top = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let overlap_pos = value_area_pos(rect);
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    // Warm up hover over two overlapping number-edits
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
        !state_bottom.press_drag.dragging,
        "Bottom widget should not enter press-drag dragging"
    );
    assert!(
        state_top.press_drag.dragging,
        "Top widget should enter press-drag dragging"
    );
    assert!(
        !res_b.input.pressed,
        "Bottom widget result should not be pressed"
    );
    assert!(res_t.input.pressed, "Top widget result should be pressed");
}

#[test]
fn test_number_edit_arrow_keys_no_traversal_but_tab_traverses() {
    let mut focus_system = FocusSystem::new();
    let mut state1 = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut state2 = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    // Initial frame to register widgets and focus the first one
    focus_system.take_keyboard_focus(state1.focus_id);

    // Frame 1: Press ArrowRight. First value should increase, focus should remain on first
    let input_right = Input {
        keys_pressed: crate::input::KeySet::from_key(crate::input::Key::ArrowRight),
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
        keys_pressed: crate::input::KeySet::from_key(crate::input::Key::Tab),
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
fn test_number_edit_double_click_value_text_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 72.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let formatter: fn(f32) -> String = |v| format!("{v:.0} px");
    let spec = spec_with_text_converter(
        rect,
        DisplayFnLegacyEditTextConverter {
            display_text: formatter,
        },
    );
    let pos = value_text_pos(rect, "72 px");

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

    let (text_edit, error, _) = assert_editing(&state.edit);
    assert_eq!(text_edit.focus_id, state.focus_id);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert_eq!(text_edit.value, "72");
    assert!(!error);
}

#[test]
fn test_number_edit_focused_enter_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 72.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    let formatter: fn(f32) -> String = |v| format!("{v:.0} px");
    let spec = spec_with_text_converter(
        rect,
        DisplayFnLegacyEditTextConverter {
            display_text: formatter,
        },
    );

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });

    let (text_edit, error, _) = assert_editing(&state.edit);
    assert_eq!(text_edit.focus_id, state.focus_id);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert_eq!(text_edit.value, "72");
    assert!(!error);
}

#[test]
fn test_number_edit_double_click_value_drag_region_enters_text_edit() {
    let rect = Rect::new(0.0, 0.0, 300.0, 28.0);
    let mut state = NumberEditState {
        value: 5.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let formatter: fn(f32) -> String = |v| format!("{v:.0}");
    let spec = spec_with_text_converter(
        rect,
        DisplayFnLegacyEditTextConverter {
            display_text: formatter,
        },
    );
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
fn test_number_edit_double_click_arrow_does_not_enter_text_edit() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let spec = default_spec(rect);
    let pos = right_arrow_pos(rect);
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
fn test_number_edit_text_edit_enter_commits_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });

    assert_eq!(state.value, 42.5);
    assert_inactive(&state.edit);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_number_edit_text_edit_enter_clamps_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "150");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });
    assert_eq!(state.value, 100.0);
    assert_inactive(&state.edit);

    enter_edit_state(&mut state, "-20");
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });
    assert_eq!(state.value, 0.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_number_edit_text_edit_enter_invalid_keeps_editing_and_sets_error() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "NaN");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });

    assert_eq!(state.value, 10.0);
    let (_, error, _) = assert_editing(&state.edit);
    assert!(error);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_number_edit_text_edit_escape_cancels() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Escape);
    });

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
}

#[test]
fn test_number_edit_text_edit_click_away_valid_commits_without_stealing_focus() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);

    for (case, click_pos) in [
        ("step button", left_arrow_pos(rect)),
        (
            "outside widget",
            Vec2::new(rect.right() + 20.0, rect.y + rect.h / 2.0),
        ),
    ] {
        let mut state = NumberEditState {
            value: 10.0,
            ..Default::default()
        };
        enter_edit_state(&mut state, "42.5");
        let mut focus_system = FocusSystem::new();
        let other_focus = FocusId::new();
        focus_system.take_keyboard_focus(other_focus);

        run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
            input.mouse_pos = click_pos;
            input.mouse_pressed = true;
            input.mouse_down = true;
        });

        assert_eq!(state.value, 42.5, "{case}");
        assert_inactive(&state.edit);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(other_focus),
            "{case}"
        );
    }
}

#[test]
fn test_number_edit_text_edit_click_away_invalid_remembers_and_reenter_restores_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);

    for (case, click_pos) in [
        ("step button", left_arrow_pos(rect)),
        (
            "outside widget",
            Vec2::new(rect.right() + 20.0, rect.y + rect.h / 2.0),
        ),
    ] {
        let mut state = NumberEditState {
            value: 10.0,
            ..Default::default()
        };
        enter_edit_state(&mut state, "abc");
        let mut focus_system = FocusSystem::new();
        let other_focus = FocusId::new();
        focus_system.take_keyboard_focus(other_focus);

        run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
            input.mouse_pos = click_pos;
            input.mouse_pressed = true;
            input.mouse_down = true;
        });

        assert_eq!(state.value, 10.0, "{case}");
        match &state.edit {
            NumberEditEditState::Remembered { draft } => assert_eq!(draft, "abc", "{case}"),
            other => panic!("expected Remembered {{ draft: \"abc\" }}, got {other:?}: {case}"),
        }
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(other_focus),
            "{case}"
        );

        let pos = value_text_pos(rect, "10.00");
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

        let (text_edit, error, _) = assert_editing(&state.edit);
        assert_eq!(text_edit.value, "abc", "{case}");
        assert!(!error, "{case}");
        assert_eq!(text_edit.focus_id, state.focus_id, "{case}");

        let (text_edit, _, _) = assert_editing_mut(&mut state.edit);
        text_edit.value = "42".to_string();
        run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
            input.keys_pressed.insert(crate::input::Key::Enter);
        });

        assert_eq!(state.value, 42.0, "{case}");
        assert_inactive(&state.edit);
    }
}

#[test]
fn test_number_edit_text_edit_focus_lost_commits_valid_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
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
fn test_number_edit_text_edit_focus_lost_invalid_remembers_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
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
fn test_number_edit_always_focus_lost_commits_valid_value_and_stays_editing() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");

    let mut focus_system = FocusSystem::new();
    let other_focus = FocusId::new();
    focus_system.take_keyboard_focus(other_focus);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );

    assert_eq!(state.value, 42.5);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "42.5");
    assert!(!error);
    assert!(!dirty);
    assert_eq!(focus_system.current_keyboard_focus(), Some(other_focus));
}

#[test]
fn test_number_edit_always_focus_lost_invalid_keeps_draft_and_sets_error() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "abc");

    let mut focus_system = FocusSystem::new();
    let other_focus = FocusId::new();
    focus_system.take_keyboard_focus(other_focus);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );

    assert_eq!(state.value, 10.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");
    assert!(error);
    assert!(dirty);
    assert_eq!(focus_system.current_keyboard_focus(), Some(other_focus));
}

#[test]
fn test_number_edit_text_edit_arrow_keys_move_caret_not_value() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "10");
    let (text_edit, _, _) = assert_editing(&state.edit);
    let original_caret = text_edit.caret;
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
    });

    assert_eq!(state.value, 10.0);
    let (text_edit, _, _) = assert_editing(&state.edit);
    assert_ne!(text_edit.caret, original_caret);
}

#[test]
fn test_number_edit_text_edit_disabled_exits_edit_mode() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        is_arrow_stepping: true,
        arrow_step_direction: Some(NumberEditStepDirection::Increment),
        press_drag: crate::widgets::widget_helpers::PressDragState {
            dragging: true,
            drag_start_pos: value_area_pos(rect),
            ..Default::default()
        },
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    let mut spec = default_spec(rect);
    spec.disabled = true;

    run_key(spec, &mut state, &mut focus_system, |_| {});

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
    assert!(!state.press_drag.dragging);
    assert!(!state.is_arrow_stepping);
}

#[test]
fn test_number_edit_text_edit_disabled_clears_remembered_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        edit: NumberEditEditState::Remembered {
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
fn test_number_edit_activation_frame_does_not_text_edit_double_click_select_word() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 12.34,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let spec = default_spec(rect);
    let pos = value_text_pos(rect, "12.34");

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

    let (text_edit, _, _) = assert_editing(&state.edit);
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
fn test_number_edit_text_edit_escape_clears_restored_draft() {
    let rect = Rect::new(0.0, 0.0, 140.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        edit: NumberEditEditState::Remembered {
            draft: "abc".into(),
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let pos = value_text_pos(rect, "10.00");
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

    let (text_edit, _, _) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");

    run_key(default_spec(rect), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Escape);
    });

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
}

fn always_text_entry_spec(rect: Rect) -> raw::NumberEditSpec<LegacyNumberEditTextConverter> {
    raw::NumberEditSpec {
        text_entry_mode: NumberEditTextEntryMode::Always,
        ..default_spec(rect)
    }
}

fn disabled_text_entry_spec(rect: Rect) -> raw::NumberEditSpec<LegacyNumberEditTextConverter> {
    raw::NumberEditSpec {
        text_entry_mode: NumberEditTextEntryMode::Disabled,
        ..default_spec(rect)
    }
}

fn frame_display_always_spec(
    rect: Rect,
) -> raw::NumberEditSpec<FrameDisplayLegacyEditTextConverter> {
    raw::NumberEditSpec {
        text_entry_mode: NumberEditTextEntryMode::Always,
        ..spec_with_text_converter(rect, FrameDisplayLegacyEditTextConverter)
    }
}

#[test]
fn test_number_edit_always_renders_text_edit_first_frame_with_raw_text() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 248.0,
        ..Default::default()
    };
    let spec = frame_display_always_spec(rect);
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    let result = run_raw(
        spec,
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "248");
    assert!(!error);
    assert!(!dirty);
    assert!(!result.focused);
}

#[test]
fn test_number_edit_always_keeps_focused_dirty_draft_across_frames() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 12.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "123");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    state.value = 99.0;
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );

    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "123");
    assert!(dirty);
}

#[test]
fn test_number_edit_always_click_focuses_text_edit() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 12.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut input = Input {
        mouse_pos: value_area_pos(rect),
        ..Default::default()
    };
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    focus_system.begin_frame();
    let _ = run_raw(
        always_text_entry_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let result = run_raw(
        always_text_entry_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(focus_system.current_keyboard_focus(), Some(state.focus_id));
    assert!(result.focused);
}

#[test]
fn test_number_edit_always_valid_click_away_commits_and_stays_editing() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42.5");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = Vec2::new(rect.right() + 20.0, rect.y + rect.h * 0.5);
            input.mouse_pressed = true;
            input.mouse_down = true;
        },
    );

    assert_eq!(state.value, 42.5);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "42.5");
    assert!(!error);
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_invalid_click_away_keeps_draft_and_error() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "abc");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = Vec2::new(rect.right() + 20.0, rect.y + rect.h * 0.5);
            input.mouse_pressed = true;
            input.mouse_down = true;
        },
    );

    assert_eq!(state.value, 10.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");
    assert!(error);
    assert!(dirty);
}

#[test]
fn test_number_edit_always_enter_commits_or_marks_error_without_exiting() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "21");
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::Enter);
        },
    );
    assert_eq!(state.value, 21.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    // Confirm that pressing Enter also selects the whole text
    assert_eq!(text_edit.caret.insertion_byte_hint(), text_edit.value.len());
    assert_eq!(selection_byte(text_edit), Some(0));
    assert!(!error);
    assert!(!dirty);

    enter_edit_state(&mut state, "bad");
    focus_system.take_keyboard_focus(state.focus_id);
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::Enter);
        },
    );
    assert_eq!(state.value, 21.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "bad");
    assert!(error);
    assert!(dirty);
}

#[test]
fn test_number_edit_always_enter_clamped_commit_updates_visible_text() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut spec = always_text_entry_spec(rect);
    spec.min = Some(0.0);
    spec.max = Some(100.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "150");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Enter);
    });

    assert_eq!(state.value, 100.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "100");
    assert_eq!(text_edit.caret.insertion_byte_hint(), text_edit.value.len());
    assert_eq!(selection_byte(text_edit), Some(0));
    assert!(!error);
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_unfocused_frame_validates_focus_loss_not_enter_escape() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let other_focus = FocusId::new();
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42");
    focus_system.take_keyboard_focus(other_focus);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::Enter);
        },
    );
    assert_eq!(state.value, 42.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "42");
    assert!(!error);
    assert!(!dirty);

    enter_edit_state(&mut state, "bad");
    focus_system.take_keyboard_focus(other_focus);
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::Escape);
        },
    );
    assert_eq!(state.value, 42.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "bad");
    assert!(error);
    assert!(dirty);
    assert_eq!(focus_system.current_keyboard_focus(), Some(other_focus));
}

#[test]
fn test_number_edit_always_escape_restores_committed_value_and_clears_dirty() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 33.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "bad");
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::Escape);
        },
    );

    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "33");
    assert!(!error);
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_ignores_drag_and_value_fill() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let spec = always_text_entry_spec(rect);
    let input = Input {
        mouse_pos: value_area_pos(rect),
        mouse_pressed: true,
        mouse_down: true,
        ..Default::default()
    };

    focus_system.begin_frame();
    let result = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(!state.press_drag.dragging);
    assert_ne!(
        result.cursor_icon,
        Some(crate::output::CursorIcon::EwResize)
    );
    assert!(!cmds.commands().iter().any(|cmd| matches!(
        cmd,
        DrawCmd::FillRect { rect, .. }
            if *rect == Rect::from_ltrb(20.0, 0.0, 80.0, 28.0)
    )));
}

fn click_always_increment_step(
    rect: Rect,
    state: &mut NumberEditState,
    focus_system: &mut FocusSystem,
) {
    run_key(always_text_entry_spec(rect), state, focus_system, |input| {
        input.mouse_pos = right_arrow_pos(rect);
    });
    run_key(always_text_entry_spec(rect), state, focus_system, |input| {
        input.mouse_pos = right_arrow_pos(rect);
        input.mouse_pressed = true;
        input.mouse_down = true;
    });
}

#[test]
fn test_number_edit_always_step_button_commits_dirty_valid_draft_before_stepping() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42");
    let mut focus_system = FocusSystem::new();

    click_always_increment_step(rect, &mut state, &mut focus_system);

    assert_eq!(state.value, 43.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "43");
    assert_eq!(text_edit.caret.insertion_byte_hint(), text_edit.value.len());
    assert_eq!(selection_byte(text_edit), Some(0));
    assert!(!error);
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_step_button_updates_clean_visible_text_after_stepping() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "50");
    if let NumberEditEditState::Editing { dirty, .. } = &mut state.edit {
        *dirty = false;
    }
    let mut focus_system = FocusSystem::new();

    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let input = Input {
        mouse_pos: right_arrow_pos(rect),
        ..Default::default()
    };

    // Frame 1: Warm up the hover claim
    focus_system.begin_frame();
    let _ = run_raw(
        always_text_entry_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Hover active
    focus_system.begin_frame();
    let hover_result = run_raw(
        always_text_entry_spec(rect),
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        hover_result.cursor_icon,
        Some(crate::output::CursorIcon::Pointer)
    );

    click_always_increment_step(rect, &mut state, &mut focus_system);

    assert_eq!(state.value, 51.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "51");
    assert_eq!(text_edit.caret.insertion_byte_hint(), text_edit.value.len());
    assert_eq!(selection_byte(text_edit), Some(0));
    assert!(!error);
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_step_button_invalid_draft_blocks_step() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "bad");
    let mut focus_system = FocusSystem::new();

    click_always_increment_step(rect, &mut state, &mut focus_system);

    assert_eq!(state.value, 10.0);
    let (text_edit, error, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "bad");
    assert!(error);
    assert!(dirty);
    assert!(!state.is_arrow_stepping);
}

#[test]
fn test_number_edit_always_dirty_aware_external_sync() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    state.value = 20.0;
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "20");
    assert!(!dirty);

    enter_edit_state(&mut state, "25");
    focus_system.take_keyboard_focus(state.focus_id);
    state.value = 30.0;
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "25");
    assert!(dirty);

    enter_edit_state(&mut state, "30");
    if let NumberEditEditState::Editing { dirty, .. } = &mut state.edit {
        *dirty = false;
    }
    focus_system.take_keyboard_focus(state.focus_id);
    state.value = 40.0;
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "30");
    assert!(!dirty);
}

#[test]
fn test_number_edit_always_user_input_sets_dirty() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.text_events.push(TextEvent::Char('5'));
        },
    );

    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_ne!(text_edit.value, "10");
    assert!(dirty);
}

#[test]
fn test_number_edit_disabled_prevents_text_entry_and_discards_remembered() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 10.0,
        edit: NumberEditEditState::Remembered {
            draft: "abc".into(),
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);

    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = value_area_pos(rect);
            input.mouse_pressed = true;
            input.mouse_down = true;
            input.mouse_click_count = 2;
            input.keys_pressed.insert(crate::input::Key::Enter);
        },
    );

    assert_eq!(state.value, 10.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_number_edit_disabled_commits_valid_edit_and_discards_invalid_edit() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut state, "42");
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(state.value, 42.0);
    assert_inactive(&state.edit);

    enter_edit_state(&mut state, "abc");
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(state.value, 42.0);
    assert_inactive(&state.edit);
}

#[test]
fn test_number_edit_disabled_keeps_drag_step_and_keyboard_step() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();

    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = value_area_pos(rect);
        },
    );
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = value_area_pos(rect);
            input.mouse_pressed = true;
            input.mouse_down = true;
        },
    );
    assert!(state.press_drag.dragging);

    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.mouse_pos = right_arrow_pos(rect);
            input.mouse_pressed = true;
            input.mouse_down = true;
        },
    );
    assert!(state.value > 50.0);

    focus_system.take_keyboard_focus(state.focus_id);
    let before = state.value;
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |input| {
            input.keys_pressed.insert(crate::input::Key::ArrowRight);
        },
    );
    assert!(state.value > before);
}

#[test]
fn test_number_edit_mode_transitions() {
    let rect = Rect::new(0.0, 0.0, 160.0, 28.0);
    let mut focus_system = FocusSystem::new();

    // OnDemand remembered invalid draft -> Always restores the draft and marks it dirty.
    let mut state = NumberEditState {
        value: 10.0,
        edit: NumberEditEditState::Remembered {
            draft: "abc".into(),
        },
        ..Default::default()
    };
    run_key(
        always_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    let (text_edit, _, dirty) = assert_editing(&state.edit);
    assert_eq!(text_edit.value, "abc");
    assert!(dirty);

    // OnDemand editing valid draft -> Disabled commits the draft and exits editing.
    enter_edit_state(&mut state, "44");
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(state.value, 44.0);
    assert_inactive(&state.edit);

    // OnDemand editing invalid draft -> Disabled discards the draft and exits editing.
    enter_edit_state(&mut state, "bad");
    run_key(
        disabled_text_entry_spec(rect),
        &mut state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(state.value, 44.0);
    assert_inactive(&state.edit);

    // Existing valid editor -> OnDemand commits on unfocused frame and exits editing.
    enter_edit_state(&mut state, "55");
    focus_system.take_keyboard_focus(FocusId::new());
    run_key(default_spec(rect), &mut state, &mut focus_system, |_| {});
    assert_eq!(state.value, 55.0);
    assert_inactive(&state.edit);

    // Existing invalid editor -> OnDemand remembers the invalid draft on unfocused frame.
    enter_edit_state(&mut state, "bad");
    focus_system.take_keyboard_focus(FocusId::new());
    run_key(default_spec(rect), &mut state, &mut focus_system, |_| {});
    assert_remembered(&state.edit, "bad");

    // Always valid draft -> OnDemand commits and exits after a real Always frame.
    let mut valid_state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut valid_state, "55");
    let mut focus_system = FocusSystem::new();
    run_key(
        always_text_entry_spec(rect),
        &mut valid_state,
        &mut focus_system,
        |_| {},
    );
    focus_system.take_keyboard_focus(FocusId::new());
    run_key(
        default_spec(rect),
        &mut valid_state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(valid_state.value, 55.0);
    assert_inactive(&valid_state.edit);

    // Always invalid draft -> OnDemand remembers after a real Always frame.
    let mut invalid_state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut invalid_state, "bad");
    let mut focus_system = FocusSystem::new();
    run_key(
        always_text_entry_spec(rect),
        &mut invalid_state,
        &mut focus_system,
        |_| {},
    );
    focus_system.take_keyboard_focus(FocusId::new());
    run_key(
        default_spec(rect),
        &mut invalid_state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(invalid_state.value, 10.0);
    assert_remembered(&invalid_state.edit, "bad");

    // Always valid draft -> Disabled commits and exits after a real Always frame.
    let mut valid_state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut valid_state, "66");
    let mut focus_system = FocusSystem::new();
    run_key(
        always_text_entry_spec(rect),
        &mut valid_state,
        &mut focus_system,
        |_| {},
    );
    run_key(
        disabled_text_entry_spec(rect),
        &mut valid_state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(valid_state.value, 66.0);
    assert_inactive(&valid_state.edit);

    // Always invalid draft -> Disabled discards and exits after a real Always frame.
    let mut invalid_state = NumberEditState {
        value: 10.0,
        ..Default::default()
    };
    enter_edit_state(&mut invalid_state, "bad");
    let mut focus_system = FocusSystem::new();
    run_key(
        always_text_entry_spec(rect),
        &mut invalid_state,
        &mut focus_system,
        |_| {},
    );
    run_key(
        disabled_text_entry_spec(rect),
        &mut invalid_state,
        &mut focus_system,
        |_| {},
    );
    assert_eq!(invalid_state.value, 10.0);
    assert_inactive(&invalid_state.edit);
}

#[test]
fn test_number_edit_optional_min_clamps_lower_only() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 2.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.min = Some(0.0);
    spec.max = None;
    spec.step = 5.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    });
    assert_eq!(state.value, 0.0);

    state.value = 98.0;
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    assert_eq!(state.value, 103.0);
}

#[test]
fn test_number_edit_optional_max_clamps_upper_only() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 98.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.min = None;
    spec.max = Some(100.0);
    spec.step = 5.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    assert_eq!(state.value, 100.0);

    state.value = 2.0;
    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    });
    assert_eq!(state.value, -3.0);
}

#[test]
fn test_number_edit_unbounded_does_not_clamp() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 0.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.min = None;
    spec.max = None;
    spec.step = 150.0;

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    });
    assert_eq!(state.value, -150.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::ArrowRight);
    });
    assert_eq!(state.value, 0.0);
}

#[test]
fn test_number_edit_home_end_respect_optional_bounds() {
    let mut focus_system = FocusSystem::new();
    let mut state = NumberEditState {
        value: 42.0,
        ..Default::default()
    };
    focus_system.take_keyboard_focus(state.focus_id);

    let mut spec = default_spec(Rect::new(0.0, 0.0, 100.0, 28.0));
    spec.min = None;
    spec.max = Some(100.0);

    run_key(spec.clone(), &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::Home);
    });
    assert_eq!(state.value, 42.0);

    run_key(spec, &mut state, &mut focus_system, |input| {
        input.keys_pressed.insert(crate::input::Key::End);
    });
    assert_eq!(state.value, 100.0);
}

#[test]
fn test_number_edit_drag_disabled_does_not_start_drag_or_set_ew_resize() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.drag_enabled = false;
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut input = Input {
        mouse_pos: value_area_pos(rect),
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

    input.mouse_pressed = true;
    input.mouse_down = true;
    focus_system.begin_frame();
    let result = run_raw(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(!state.press_drag.dragging);
    assert_eq!(result.cursor_icon, None);
}

#[test]
fn test_number_edit_drag_disabled_still_allows_double_click_edit() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.drag_enabled = false;
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);
    let hover_input = Input {
        mouse_pos: value_area_pos(rect),
        ..Default::default()
    };
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &hover_input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let input = Input {
        mouse_pos: value_area_pos(rect),
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

    assert!(state.edit.is_editing());
}

#[test]
fn test_number_edit_value_fill_disabled_draws_no_value_fill() {
    let mut spec = default_spec(Rect::new(10.0, 10.0, 100.0, 28.0));
    spec.value_fill_enabled = false;
    let style = spec.style;
    let (_res, cmds) = number_edit(spec, 50.0);

    assert!(!cmds
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == style.value_fill)));
}

#[test]
fn test_number_edit_value_fill_suppressed_when_unbounded() {
    let mut spec = default_spec(Rect::new(10.0, 10.0, 100.0, 28.0));
    spec.min = Some(0.0);
    spec.max = None;
    let style = spec.style;
    let (_res, cmds) = number_edit(spec, 50.0);

    assert!(!cmds
        .commands()
        .iter()
        .any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == style.value_fill)));
}

#[test]
fn test_number_edit_step_button_visual_appearance() {
    let mut style = default_style();
    style.step_button.padding_x = 4.0;
    style.step_button.background = Color::linear_rgba(0.1, 0.2, 0.3, 1.0);
    style.step_button.background_hovered = Color::linear_rgba(0.2, 0.3, 0.4, 1.0);
    style.step_button.background_pressed = Color::linear_rgba(0.3, 0.4, 0.5, 1.0);
    style.step_button.border = Some(Stroke::new(Color::linear_rgba(0.4, 0.5, 0.6, 1.0), 1.0));
    style.step_button.glyph_color = Color::linear_rgba(0.7, 0.8, 0.9, 1.0);
    style.step_button.decrement_glyph = "-";
    style.step_button.increment_glyph = "+";
    let text_converter = LegacyNumberEditTextConverter;
    let pre_layout_spec = raw::NumberEditPreLayoutSpec {
        style,
        value: 50.0,
        step_buttons_enabled: true,
        text_converter: &text_converter,
    };
    let mut text_backend = TestTextBackend::default()
        .with_char_advance('-', 5.0)
        .with_char_advance('+', 13.0);

    let size = raw::pre_layout_number_edit(
        &pre_layout_spec,
        crate::layout::SizeOffer::UNBOUNDED,
        &mut text_backend,
    )
    .size_request;

    assert_eq!(
        size.preferred.unwrap().x,
        "50.00".len() as f32 * 8.0 + style.text_pad_x * 2.0 + (13.0 + 8.0) * 2.0
    );

    let spec = raw::NumberEditSpec {
        style,
        ..default_spec(Rect::new(10.0, 10.0, 100.0, 28.0))
    };
    let mut state = NumberEditState {
        value: 50.0,
        is_arrow_stepping: true,
        arrow_step_direction: Some(NumberEditStepDirection::Increment),
        press_drag: crate::widgets::widget_helpers::PressDragState {
            held: true,
            press_start_pos: Vec2::new(99.0, 24.0),
            drag_start_pos: Vec2::new(99.0, 24.0),
            ..Default::default()
        },
        repeat_timer: {
            let mut timer = RepeatTimer::default();
            timer.start(0.5, RepeatTiming::PRESS);
            timer
        },
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default()
        .with_char_advance('-', 5.0)
        .with_char_advance('+', 13.0);
    let mut cmds = DrawCommands::new(1.0);
    let input = Input {
        mouse_pos: Vec2::new(99.0, 24.0),
        mouse_down: true,
        ..Default::default()
    };

    let _ = run_raw(
        spec,
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
                rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(31.0, 10.0, 29.0, 28.0),
                color: style.value_fill,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 21.0, 28.0),
                color: style.step_button.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(10.0, 10.0, 21.0, 28.0),
                color: style.step_button.border.unwrap().color,
                width: style.step_button.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(89.0, 10.0, 21.0, 28.0),
                color: style.step_button.background_pressed,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(89.0, 10.0, 21.0, 28.0),
                color: style.step_button.border.unwrap().color,
                width: style.step_button.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..5,
                color: style.value_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 5..6,
                color: style.step_button.glyph_color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 6..7,
                color: style.step_button.glyph_color,
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
                token: PreparedGlyphToken(53),
                top_left: Vec2 { x: 40.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 48.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(46),
                top_left: Vec2 { x: 56.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 64.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(48),
                top_left: Vec2 { x: 72.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken('-' as u64),
                top_left: Vec2 { x: 18.0, y: 28.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken('+' as u64),
                top_left: Vec2 { x: 93.0, y: 28.0 },
            },
        ]
    );
}

#[test]
fn test_number_edit_arrow_key_draws_stepper_button_pressed() {
    let mut style = default_style();
    style.step_button.background_pressed = Color::linear_rgba(0.73, 0.12, 0.31, 1.0);
    let spec = raw::NumberEditSpec {
        style,
        ..default_spec(Rect::new(10.0, 10.0, 100.0, 28.0))
    };
    let mut state = NumberEditState {
        value: 50.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    let mut text_backend = TestTextBackend::default();

    let mut cmds = DrawCommands::new(1.0);
    focus_system.begin_frame();
    let _ = run_raw(
        spec.clone(),
        &mut state,
        &Input {
            keys_down: crate::input::KeySet::from_key(crate::input::Key::ArrowLeft),
            ..Default::default()
        },
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        state.value, 50.0,
        "held-only ArrowLeft should draw pressed without stepping the value"
    );
    assert!(cmds.commands().iter().any(|cmd| {
        matches!(
            cmd,
            DrawCmd::FillRect { rect, color, .. }
                if *rect == Rect::new(10.0, 10.0, 20.0, 28.0)
                    && *color == style.step_button.background_pressed
        )
    }));
    assert!(!cmds.commands().iter().any(|cmd| {
        matches!(
            cmd,
            DrawCmd::FillRect { rect, color, .. }
                if *rect == Rect::new(90.0, 10.0, 20.0, 28.0)
                    && *color == style.step_button.background_pressed
        )
    }));

    let mut cmds = DrawCommands::new(1.0);
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let _ = run_raw(
        spec,
        &mut state,
        &Input {
            keys_down: crate::input::KeySet::from_key(crate::input::Key::ArrowRight),
            ..Default::default()
        },
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(
        state.value, 50.0,
        "held-only ArrowRight should draw pressed without stepping the value"
    );
    assert!(cmds.commands().iter().any(|cmd| {
        matches!(
            cmd,
            DrawCmd::FillRect { rect, color, .. }
                if *rect == Rect::new(90.0, 10.0, 20.0, 28.0)
                    && *color == style.step_button.background_pressed
        )
    }));
    assert!(!cmds.commands().iter().any(|cmd| {
        matches!(
            cmd,
            DrawCmd::FillRect { rect, color, .. }
                if *rect == Rect::new(10.0, 10.0, 20.0, 28.0)
                    && *color == style.step_button.background_pressed
        )
    }));
}

#[test]
fn test_number_edit_step_buttons_increment_and_decrement_with_optional_bounds() {
    let rect = Rect::new(0.0, 0.0, 100.0, 28.0);
    let mut spec = default_spec(rect);
    spec.min = Some(0.0);
    spec.max = Some(2.0);
    spec.step = 2.0;
    let mut state = NumberEditState {
        value: 1.0,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    let mut text_backend = TestTextBackend::default();
    let mut cmds = DrawCommands::new(1.0);

    let mut input = Input {
        mouse_pos: Vec2::new(10.0, 14.0),
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
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.value, 0.0);

    input.mouse_pressed = false;
    input.mouse_down = false;
    input.mouse_pos = Vec2::new(90.0, 14.0);
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
    input.mouse_pressed = true;
    input.mouse_down = true;
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
    assert_eq!(state.value, 2.0);
}
