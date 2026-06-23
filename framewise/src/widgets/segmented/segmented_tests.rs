use super::raw::SegmentedSpec;
use super::*;
use crate::test_utils::TestTextBackend;
use crate::{DrawGlyph, PreparedGlyphToken};

fn segmented_dummy<'a>(
    spec: SegmentedSpec<'a>,
    active_index: usize,
) -> (raw::SegmentedResult, DrawCommands) {
    let mut cmds = DrawCommands::new();
    let mut text_backend = TestTextBackend::default();
    let res = raw::post_layout_segmented(
        spec,
        raw::SegmentedPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut SegmentedState {
            active_index,
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
fn test_segmented_visual_normal() {
    let items = ["A", "B"];
    let spec = SegmentedSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 200.0, 28.0),
        items: &items,
        disabled: false,
        style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let (_res, cmds) = segmented_dummy(spec, 0);

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 36.0, 28.0),
                color: style.active_bg,
                z: 0,
            },
            DrawCmd::StrokeLine {
                p0: Vec2::new(36.0, 0.0),
                p1: Vec2::new(36.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: style.active_text,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..2,
                color: style.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(65),
                top_left: Vec2 { x: 14.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 50.0, y: 19.0 }
            }
        ]
    );
}

#[test]
fn test_segmented_visual_focused() {
    let mut state = SegmentedState {
        active_index: 1,
        ..Default::default()
    };
    let mut focus_system = FocusSystem::new();
    focus_system.take_keyboard_focus(state.focus_id);
    focus_system.begin_frame();
    let mut text_backend = TestTextBackend::default();
    let items = ["A", "B"];
    let spec = SegmentedSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 200.0, 28.0),
        items: &items,
        disabled: false,
        style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_segmented(
        spec,
        raw::SegmentedPreLayoutResult {
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
                rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                color: style.background,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
            DrawCmd::StrokeLine {
                p0: Vec2::new(36.0, 0.0),
                p1: Vec2::new(36.0, 28.0),
                color: style.border.unwrap().color,
                width: style.border.unwrap().width,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: style.text,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(36.0, 0.0, 36.0, 28.0),
                color: style.active_bg,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(40.0, 4.0, 28.0, 20.0),
                color: style.focus.unwrap().stroke.color,
                width: style.focus.unwrap().stroke.width,
                placement: crate::BorderPlacement::Outside,
                z: 1,
            },
            DrawCmd::GlyphRun {
                glyphs: 1..2,
                color: style.active_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(65),
                top_left: Vec2 { x: 14.0, y: 19.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(66),
                top_left: Vec2 { x: 50.0, y: 19.0 }
            }
        ]
    );
}

#[test]
fn test_segmented_click_takes_focus() {
    let mut focus_system = FocusSystem::new();
    let mut state = SegmentedState::default();
    let input = Input {
        mouse_pos: Vec2::new(20.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = ["A", "B"];
    let spec = SegmentedSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 200.0, 28.0),
        items: &items,
        disabled: false,
        style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: None,
    };

    let mut cmds = DrawCommands::new();
    focus_system.begin_frame();
    raw::post_layout_segmented(
        spec,
        raw::SegmentedPreLayoutResult {
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
        "Clicking segmented must request focus"
    );
}

#[test]
fn test_segmented_clipped_click_does_not_take_focus() {
    let mut focus_system = FocusSystem::new();
    let mut state = SegmentedState::default();
    let input = Input {
        mouse_pos: Vec2::new(20.0, 10.0),
        mouse_pressed: true,
        ..Default::default()
    };

    let mut text_backend = TestTextBackend::default();
    let items = ["A", "B"];
    let spec = SegmentedSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 200.0, 28.0),
        items: &items,
        disabled: false,
        style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
        clip_rect: Some(Rect::new(500.0, 500.0, 200.0, 28.0)),
    };

    let mut cmds = DrawCommands::new();
    focus_system.begin_frame();
    raw::post_layout_segmented(
        spec,
        raw::SegmentedPreLayoutResult {
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
        "Clicking a clipped-away segmented control must not take focus"
    );
}

#[test]
fn test_segmented_keyboard_navigation() {
    let mut focus_system = FocusSystem::new();
    let mut state = SegmentedState::default();
    let mut input = Input::default();
    let mut text_backend = TestTextBackend::default();
    let items = ["A", "B"];

    // Focus the widget
    focus_system.take_keyboard_focus(state.focus_id);

    // Frame 1: Press Arrow Right -> changes active index to 1
    input.key_pressed_right = true;
    let mut cmds = DrawCommands::new();
    focus_system.begin_frame();
    raw::post_layout_segmented(
        SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::SegmentedPreLayoutResult {
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

    assert_eq!(state.active_index, 1);

    // Frame 2: Press Arrow Left -> changes active index back to 0
    input.key_pressed_left = true;
    let mut cmds = DrawCommands::new();
    focus_system.begin_frame();
    raw::post_layout_segmented(
        SegmentedSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            disabled: false,
            style: SegmentedStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        },
        raw::SegmentedPreLayoutResult {
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
fn test_builder_defaults_from_theme_fills_unset_fields() {
    let theme = crate::theme::Theme::framewise();
    let builder = SegmentedSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(SegmentedStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_fields() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = SegmentedStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let builder = SegmentedSpecBuilder::new().style(custom_style);
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
    let mut seg_state = SegmentedState::default();
    let result = super::segmented(
        &mut ctx,
        SegmentedSpecBuilder::new().items(&[]),
        placement,
        &mut seg_state,
    );
    assert_eq!(result.layout.bounds, placement);
}
