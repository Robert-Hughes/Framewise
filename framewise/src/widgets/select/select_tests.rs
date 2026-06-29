use super::raw::{SelectResult, SelectSpec};
use super::*;
use crate::test_utils::TestTextBackend;
use crate::types::Vec2;
use crate::{DrawGlyph, PreparedGlyphToken};

fn post_layout_select_for_test<'a, T: crate::text::TextBackend>(
    spec: SelectSpec<'a>,
    state: &mut SelectState,
    input: &crate::Input,
    focus_system: &mut crate::focus::FocusSystem,
    text_backend: &mut T,
    cmds: &mut crate::draw::DrawCommands,
) -> SelectResult {
    raw::post_layout_select(
        spec,
        raw::SelectPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        state,
        input,
        focus_system,
        text_backend,
        cmds,
    )
}

fn select_dummy<'a>(spec: SelectSpec<'a>) -> (SelectResult, DrawCommands) {
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let result = post_layout_select_for_test(
        spec,
        &mut SelectState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );
    (result, cmds)
}

#[test]
fn test_select_visual_normal() {
    let items = vec!["Option 1", "Option 2", "Option 3"];
    let spec = SelectSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 180.0, 28.0),
        value: "Option 1",
        items: &items,
        disabled: false,
        style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let s = spec.style;
    let (_, cmds) = select_dummy(spec);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: s.text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 7..8,
                color: s.muted,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 10.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 18.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 26.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 34.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 42.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 50.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(49),
                top_left: Vec2 { x: 66.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(118),
                top_left: Vec2 { x: 162.0, y: 17.0 }
            }
        ]
    );
}

#[test]
fn test_select_visual_open() {
    let mut text_backend = TestTextBackend::default();
    let items = vec!["Option 1", "Option 2", "Option 3"];
    let spec = SelectSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 180.0, 28.0),
        value: "Option 1",
        items: &items,
        disabled: false,
        style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let s = spec.style;

    // Pass SelectState { open: true, ... } to simulate open state
    let state = SelectState {
        selected_index: 0,
        open: true,
        hovered: Some(1),
        space_is_active: false,
        focus_id: FocusId::new(),
    };

    let mut state = state;
    let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
    let mut cmds = DrawCommands::new(1.0);
    let input = Input {
        mouse_pos: Vec2::new(10.0, 70.0),
        ..Default::default()
    };
    let result = post_layout_select_for_test(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );

    let r = Rect::new(0.0, 0.0, 180.0, 28.0);
    let popup = Rect::new(0.0, 30.0, 180.0, 86.0);

    assert_eq!(result.cursor_icon, Some(crate::output::CursorIcon::Pointer));

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::BorderRect {
                rect: r.inset(-s.focus.unwrap().offset),
                color: s.focus.unwrap().stroke.color,
                width: s.focus.unwrap().stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::FillRect {
                rect: r,
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: s.text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 7..8,
                color: s.accent,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: popup,
                color: s.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: popup,
                color: s.border.unwrap().color,
                width: s.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 34.0, 180.0, 26.0),
                color: s.selected_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 8..15,
                color: s.selected_text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 60.0, 180.0, 26.0),
                color: s.hover,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 15..22,
                color: s.text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 22..29,
                color: s.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 10.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 18.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 26.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 34.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 42.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 50.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(49),
                top_left: Vec2 { x: 66.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(118),
                top_left: Vec2 { x: 162.0, y: 17.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 12.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 20.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 28.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 36.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 44.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 52.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(49),
                top_left: Vec2 { x: 68.0, y: 52.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 12.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 20.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 28.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 36.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 44.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 52.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(50),
                top_left: Vec2 { x: 68.0, y: 78.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 12.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 20.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 28.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 36.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 44.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 52.0, y: 104.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(51),
                top_left: Vec2 { x: 68.0, y: 104.0 }
            }
        ]
    );
}

#[test]
fn test_select_click_takes_focus_and_opens() {
    let mut focus_system = FocusSystem::new();
    let state = SelectState::default();
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = vec!["Option 1", "Option 2"];
    let spec = SelectSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 180.0, 28.0),
        value: "Option 1",
        items: &items,
        disabled: false,
        style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut state = state;
    focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    let result = post_layout_select_for_test(
        spec,
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(result.cursor_icon, Some(crate::output::CursorIcon::Pointer));
    assert_eq!(
        focus_system.current_keyboard_focus(),
        Some(state.focus_id),
        "Clicking select must request focus"
    );
    assert!(state.open, "Clicking select must open the popup dropdown");
}

#[test]
fn test_select_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let state = SelectState::default();
    let input = Input {
        mouse_pos: Vec2::new(15.0, 15.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = vec!["Option 1", "Option 2"];
    let spec = SelectSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 180.0, 28.0),
        value: "Option 1",
        items: &items,
        disabled: false,
        style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 180.0, 28.0)),
    };

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    let result = post_layout_select_for_test(
        spec,
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
        "Clicking a clipped-away select must not take focus"
    );
}

#[test]
fn test_select_keyboard_navigation() {
    let mut focus_system = FocusSystem::new();
    let mut state = SelectState::default();
    let mut input = Input::default();
    let mut text_backend = TestTextBackend::default();
    let items = vec!["Option 1", "Option 2", "Option 3"];

    // Focus the widget first
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: Press Arrow Down while closed -> selected index changes to 1
    input.key_pressed_down = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    post_layout_select_for_test(
        SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    input.key_pressed_down = false;

    assert_eq!(state.selected_index, 1);
    assert!(!state.open);

    // Frame 2: Press Space -> opens dropdown
    input.key_down_space = true;
    input.key_pressed_space = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    post_layout_select_for_test(
        SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 2",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    input.key_down_space = false;
    input.key_pressed_space = false;
    input.key_released_space = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    post_layout_select_for_test(
        SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 2",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    input.key_released_space = false;

    assert!(state.open);
    assert_eq!(state.hovered, Some(1));

    // Frame 3: Press Arrow Down while open -> hovers index 2
    input.key_pressed_down = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    post_layout_select_for_test(
        SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 2",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    input.key_pressed_down = false;

    assert_eq!(state.hovered, Some(2));

    // Frame 4: Press Enter while open -> selects hovered (index 2) and closes dropdown
    input.key_pressed_enter = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    post_layout_select_for_test(
        SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 2",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(!state.open);
    assert_eq!(state.selected_index, 2);
}

#[test]
fn test_select_spec_theme_overwrites_style() {
    let theme = crate::theme::Theme::framewise();
    let items = ["A"];
    let spec = super::SelectSpec::new("A", &items).theme(&theme);
    assert_eq!(spec.style, SelectStyle::from_theme(&theme));
}

#[test]
fn test_select_spec_theme_preserves_semantic_fields() {
    let theme = crate::theme::Theme::framewise();
    let items = ["A"];
    let spec = super::SelectSpec::new("A", &items)
        .disabled(true)
        .theme(&theme);
    assert_eq!(spec.value, "A");
    assert_eq!(spec.items, &items);
    assert!(spec.disabled);
    assert_eq!(spec.style, SelectStyle::from_theme(&theme));
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
    let mut sel_state = SelectState::default();
    let result = super::select(
        super::SelectSpec::new_from_theme("", &[], &ctx.theme),
        custom_rect,
        &mut sel_state,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, custom_rect);
}
