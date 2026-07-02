use super::raw::TabsSpec;
use super::*;
use crate::test_utils::TestTextBackend;
use crate::{DrawGlyph, PreparedGlyphToken};

fn make_spec<'a>(items: &'a [&'a str]) -> TabsSpec<'a> {
    TabsSpec {
        rect: Rect::new(0.0, 0.0, 300.0, 36.0),
        items,
        disabled: false,
        style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
        layer: Layer::default(),
    }
}

fn tabs_dummy<'a>(spec: TabsSpec<'a>, active_index: usize) -> (DrawCommands, raw::TabsResult) {
    let mut cmds = DrawCommands::new(1.0);
    let mut text_backend = TestTextBackend::default();
    let result = raw::post_layout_tabs(
        spec,
        raw::TabsPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut TabsState {
            active_index,
            ..Default::default()
        },
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );
    (cmds, result)
}

#[test]
fn test_tabs_visual_normal() {
    let items = ["Tab1", "Tab2"];
    let spec = make_spec(&items);
    let style = spec.style;
    let (cmds, _res) = tabs_dummy(spec, 0);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(
                    0.0,
                    36.0 - style.border.unwrap().width,
                    300.0,
                    style.border.unwrap().width
                ),
                color: style.border.unwrap().color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..4,
                color: style.text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 33.0, 68.0, 3.0),
                color: style.accent,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 27.0, 3.0, 9.0),
                color: style.accent,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(65.0, 27.0, 3.0, 9.0),
                color: style.accent,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 4..8,
                color: style.inactive_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 18.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 26.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(98),
                top_left: Vec2 { x: 34.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(49),
                top_left: Vec2 { x: 42.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 86.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 94.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(98),
                top_left: Vec2 { x: 102.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(50),
                top_left: Vec2 { x: 110.0, y: 23.0 }
            }
        ]
    );
}

#[test]
fn test_tabs_visual_focused() {
    let mut state = TabsState {
        active_index: 1,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let mut text_backend = TestTextBackend::default();
    let items = ["Tab1", "Tab2"];
    let spec = make_spec(&items);
    let style = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    let _res = raw::post_layout_tabs(
        spec,
        raw::TabsPreLayoutResult {
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
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(
                    0.0,
                    36.0 - style.border.unwrap().width,
                    300.0,
                    style.border.unwrap().width
                ),
                color: style.border.unwrap().color,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..4,
                color: style.inactive_text,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(66.0, -2.0, 72.0, 40.0),
                color: style.focus.unwrap().stroke.color,
                width: style.focus.unwrap().stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::GlyphRun {
                glyphs: 4..8,
                color: style.text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(68.0, 33.0, 68.0, 3.0),
                color: style.accent,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(68.0, 27.0, 3.0, 9.0),
                color: style.accent,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(133.0, 27.0, 3.0, 9.0),
                color: style.accent,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 18.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 26.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(98),
                top_left: Vec2 { x: 34.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(49),
                top_left: Vec2 { x: 42.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 86.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 94.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(98),
                top_left: Vec2 { x: 102.0, y: 23.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(50),
                top_left: Vec2 { x: 110.0, y: 23.0 }
            }
        ]
    );
}

#[test]
fn test_tabs_click_takes_focus() {
    let mut state = TabsState::default();
    let input = Input {
        mouse_pos: Vec2::new(20.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = ["Tab1", "Tab2"];
    let spec = make_spec(&items);

    let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    let result = raw::post_layout_tabs(
        spec,
        raw::TabsPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
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
        "Clicking tabs must request focus"
    );
}

#[test]
fn test_tabs_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let mut state = TabsState::default();
    let input = Input {
        mouse_pos: Vec2::new(20.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = ["Tab1", "Tab2"];
    let spec = TabsSpec {
        rect: Rect::new(0.0, 0.0, 300.0, 36.0),
        items: &items,
        disabled: false,
        style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 300.0, 36.0)),
        layer: Layer::default(),
    };

    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    let result = raw::post_layout_tabs(
        spec,
        raw::TabsPreLayoutResult {
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
        "Clicking a clipped-away tabs widget must not take focus"
    );
}

#[test]
fn test_tabs_keyboard_navigation() {
    let mut focus_system = FocusSystem::new();
    let mut state = TabsState::default();
    let mut input = Input::default();
    let mut text_backend = TestTextBackend::default();
    let items = ["Tab1", "Tab2"];

    // Focus the widget
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: Press Arrow Right -> changes active index to 1
    input.keys_pressed.insert(crate::input::Key::ArrowRight);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_tabs(
        make_spec(&items),
        raw::TabsPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();
    input.keys_pressed.remove(crate::input::Key::ArrowRight);

    assert_eq!(state.active_index, 1);

    // Frame 2: Press Arrow Left -> changes active index back to 0
    input.keys_pressed.insert(crate::input::Key::ArrowLeft);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_tabs(
        make_spec(&items),
        raw::TabsPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert_eq!(state.active_index, 0);
}

#[test]
fn test_tabs_spec_theme_overwrites_style() {
    let theme = crate::theme::Theme::framewise();
    let items = ["A"];
    let spec = super::TabsSpec::new(&items).theme(&theme);
    assert_eq!(spec.style, TabsStyle::from_theme(&theme));
}

#[test]
fn test_tabs_spec_theme_preserves_semantic_fields() {
    let theme = crate::theme::Theme::framewise();
    let items = ["A"];
    let spec = super::TabsSpec::new(&items).disabled(true).theme(&theme);
    assert_eq!(spec.items, &items);
    assert!(spec.disabled);
    assert_eq!(spec.style, TabsStyle::from_theme(&theme));
}

#[test]
fn test_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let placement = Rect::new(10.0, 20.0, 200.0, 36.0);
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
    let mut tabs_state = TabsState::default();
    let result = super::tabs(
        super::TabsSpec::new_from_theme(&[], &ctx.theme),
        placement,
        &mut tabs_state,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_size_tabs() {
    let mut ts = TestTextBackend::default();
    let spec = raw::TabsPreLayoutSpec {
        items: &["Tab1", "Tab2"],
        style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
    };
    // Tab1 = 4 chars * 8px = 32px + 2*18 pad = 68px; Tab2 = same = 68px; total = 136px
    let size_request = raw::pre_layout_tabs(&spec, SizeOffer::UNBOUNDED, &mut ts).size_request;
    assert_eq!(size_request.preferred, Some(Vec2::new(136.0, 36.0)));
}
