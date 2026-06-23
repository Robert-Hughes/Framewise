use super::raw::ChipSpec;
use super::*;
use crate::test_utils::TestTextBackend;
use crate::types::Vec2;
use crate::{DrawGlyph, PreparedGlyphToken};

fn chip_raw<'a>(spec: ChipSpec<'a>) -> (raw::ChipResult, DrawCommands) {
    let mut cmds = DrawCommands::new();
    let mut text_backend = TestTextBackend::default();
    let res = raw::post_layout_chip(
        spec,
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut ChipState::default(),
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );
    (res, cmds)
}

#[test]
fn test_chip_visual_normal() {
    let spec = ChipSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 50.0, 22.0),
        text: "Tag",
        disabled: false,
        style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let (_res, cmds) = chip_raw(spec);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: style.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 8.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 16.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(103),
                top_left: Vec2 { x: 24.0, y: 16.0 }
            }
        ]
    );
}

#[test]
fn test_chip_visual_active() {
    let mut text_backend = TestTextBackend::default();
    let mut state = ChipState {
        checked: true,
        ..Default::default()
    };
    let spec = ChipSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 50.0, 22.0),
        text: "Tag",
        disabled: false,
        style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        spec,
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut FocusSystem::new(),
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                color: style.active_bg,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: style.active_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 8.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 16.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(103),
                top_left: Vec2 { x: 24.0, y: 16.0 }
            }
        ]
    );
}

#[test]
fn test_chip_visual_focused() {
    let state = ChipState::default();
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let mut text_backend = TestTextBackend::default();
    let spec = ChipSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 50.0, 22.0),
        text: "Tag",
        disabled: false,
        style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut state = state;
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        spec,
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &Input::default(),
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    let r = Rect::new(0.0, 0.0, 50.0, 22.0);
    let expected_focus_rect = r.inset(-style.focus.unwrap().offset);
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::BorderRect {
                rect: expected_focus_rect,
                color: style.focus.unwrap().stroke.color,
                width: style.focus.unwrap().stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::FillRect {
                rect: r,
                color: style.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: r,
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..3,
                color: style.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(84),
                top_left: Vec2 { x: 8.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 16.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(103),
                top_left: Vec2 { x: 24.0, y: 16.0 }
            }
        ]
    );
}

#[test]
fn test_chip_click_takes_focus() {
    let mut focus_system = FocusSystem::new();
    let state = ChipState::default();
    let input = Input {
        mouse_pos: Vec2::new(10.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = ChipSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 50.0, 22.0),
        text: "Tag",
        disabled: false,
        style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        spec,
        raw::ChipPreLayoutResult {
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
        "Clicking chip must request focus"
    );
}

#[test]
fn test_chip_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let state = ChipState::default();
    let input = Input {
        mouse_pos: Vec2::new(10.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let spec = ChipSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 50.0, 22.0),
        text: "Tag",
        disabled: false,
        style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 50.0, 22.0)),
    };

    let mut state = state;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        spec,
        raw::ChipPreLayoutResult {
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
        "Clicking a clipped-away chip must not take focus"
    );
}

#[test]
fn test_chip_keyboard_toggle() {
    let mut focus_system = FocusSystem::new();
    let mut state = ChipState::default();
    let mut input = Input::default();
    let mut text_backend = TestTextBackend::default();

    // Frame 1: Focus chip
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 2: Press Space
    input.key_down_space = true;
    input.key_pressed_space = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    // Frame 3: Release Space
    input.key_down_space = false;
    input.key_pressed_space = false;
    input.key_released_space = true;
    focus_system.begin_frame();
    let mut cmds = DrawCommands::new();
    raw::post_layout_chip(
        ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::ChipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut state,
        &input,
        &mut focus_system,
        &mut text_backend,
        &mut cmds,
    );
    focus_system.end_frame();

    assert!(state.checked, "Spacebar release must toggle chip state");
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_fields() {
    let theme = crate::theme::Theme::framewise();
    let builder = ChipSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(ChipStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_fields() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = ChipStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let builder = ChipSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().text_style.size, 99.0);
}

#[test]
fn test_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
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
    let mut chip_state = ChipState::default();
    let result = super::chip(
        &mut ctx,
        ChipSpecBuilder::new().text("X"),
        placement,
        &mut chip_state,
    );
    assert_eq!(result.layout.bounds, placement);
}
