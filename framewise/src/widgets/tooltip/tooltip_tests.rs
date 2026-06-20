use super::raw::TooltipSpec;
use super::*;
use crate::{focus::FocusSystem, test_utils::TestTextBackend, DrawGlyph, PreparedGlyphToken};

#[test]
fn test_tooltip_visual_dark() {
    let mut text_backend = TestTextBackend::default();
    let spec = TooltipSpec {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        text: "Tooltip",
        variant: TooltipVariant::Dark,
        style: TooltipStyle::from_theme(&crate::theme::Theme::framewise()),
        layer: Layer::default(),
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new();
    let res = raw::post_layout_tooltip(
        spec,
        raw::TooltipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(res.bounds, Rect::new(0.0, 0.0, 72.0, 27.0));
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                color: style.dark_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: style.dark_text,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(14.0, 27.0),
                p1: Vec2::new(18.0, 31.0),
                color: style.dark_bg,
                width: style.arrow_width,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(22.0, 27.0),
                p1: Vec2::new(18.0, 31.0),
                color: style.dark_bg,
                width: style.arrow_width,
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
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 16.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 24.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 32.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 40.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 48.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 56.0, y: 16.0 }
            }
        ]
    );
}

#[test]
fn test_tooltip_visual_rust() {
    let mut text_backend = TestTextBackend::default();
    let spec = TooltipSpec {
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        text: "Tooltip",
        variant: TooltipVariant::Rust,
        style: TooltipStyle::from_theme(&crate::theme::Theme::framewise()),
        layer: Layer::default(),
    };
    let style = spec.style;
    let mut cmds = DrawCommands::new();
    let res = raw::post_layout_tooltip(
        spec,
        raw::TooltipPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(res.bounds, Rect::new(0.0, 0.0, 72.0, 27.0));
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                color: style.rust_bg,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: style.rust_text,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(14.0, 27.0),
                p1: Vec2::new(18.0, 31.0),
                color: style.rust_bg,
                width: style.arrow_width,
                z: 0,
            },
            DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(22.0, 27.0),
                p1: Vec2::new(18.0, 31.0),
                color: style.rust_bg,
                width: style.arrow_width,
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
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 16.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 24.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 32.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 40.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 48.0, y: 16.0 }
            },
            DrawGlyph {
                token: PreparedGlyphToken(112),
                top_left: Vec2 { x: 56.0, y: 16.0 }
            }
        ]
    );
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
    let result = super::tooltip(
        &mut ctx,
        TooltipSpecBuilder::new()
            .text("hi")
            .variant(TooltipVariant::Dark),
        placement,
    );
    assert_eq!(result.layout.bounds.x, placement.x);
    assert_eq!(result.layout.bounds.y, placement.y);
}

#[test]
fn test_tooltip_bounds_and_content_bounds() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
    let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
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
    let res = super::tooltip(
        &mut ctx,
        TooltipSpecBuilder::new()
            .text("hi")
            .variant(TooltipVariant::Dark),
        layout_rect,
    );

    let style = TooltipStyle::from_theme(&ctx.theme);
    let expected_w = (16.0 + style.pad_x * 2.0).min(style.max_width);
    let expected_h = 16.0 + style.pad_y_top + style.pad_y_bot;

    assert_eq!(
        res.layout.bounds,
        Rect::new(layout_rect.x, layout_rect.y, expected_w, expected_h)
    );

    let expected_content = Rect::new(
        layout_rect.x + style.pad_x,
        layout_rect.y + style.pad_y_top,
        expected_w - style.pad_x * 2.0,
        expected_h - (style.pad_y_top + style.pad_y_bot),
    );
    assert_eq!(res.layout.content_bounds, expected_content);
}
