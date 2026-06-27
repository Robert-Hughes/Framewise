use super::raw::StatusSpec as RawStatusSpec;
use super::*;
use crate::types::Vec2;
use crate::{focus::FocusSystem, test_utils::TestTextBackend, DrawGlyph, PreparedGlyphToken};

#[test]
fn test_status_visual_ok() {
    let mut text_backend = TestTextBackend::default();
    let spec = RawStatusSpec {
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        text: "Online",
        variant: StatusVariant::Ok,
        style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
        layer: Layer::default(),
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_status(
        spec,
        raw::StatusPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                color: style.ok,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..6,
                color: style.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(79),
                top_left: Vec2 { x: 14.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 22.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 30.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 38.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 46.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(101),
                top_left: Vec2 { x: 54.0, y: 6.0 },
            },
        ]
    );
}

#[test]
fn test_status_visual_warn() {
    let mut text_backend = TestTextBackend::default();
    let spec = RawStatusSpec {
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        text: "Warning",
        variant: StatusVariant::Warn,
        style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
        layer: Layer::default(),
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_status(
        spec,
        raw::StatusPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                color: style.warn,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: style.text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(87),
                top_left: Vec2 { x: 14.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(97),
                top_left: Vec2 { x: 22.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(114),
                top_left: Vec2 { x: 30.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 38.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 46.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 54.0, y: 6.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(103),
                top_left: Vec2 { x: 62.0, y: 6.0 },
            },
        ]
    );
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
    let result = super::status(
        StatusSpec::new_from_theme("ok", StatusVariant::Ok, &ctx.theme),
        placement,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, placement);
}
