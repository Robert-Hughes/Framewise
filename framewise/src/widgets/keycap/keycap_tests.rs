use super::raw::KeycapSpec;
use super::*;
use crate::{
    focus::FocusSystem, test_utils::TestTextBackend, text::FontId, DrawGlyph, PreparedGlyphToken,
    Vec2,
};

#[test]
fn test_keycap_visual() {
    let mut text_backend = TestTextBackend::default();
    let custom_bg = Color::from_srgb_u8(240, 240, 240, 255);
    let custom_shadow = Color::from_srgb_u8(10, 10, 10, 255);
    let custom_border = Color::from_srgb_u8(10, 10, 10, 255);
    let custom_text = Color::from_srgb_u8(50, 50, 50, 255);
    let spec = KeycapSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 30.0, 30.0),
        text: "K",
        style: KeycapStyle {
            background: custom_bg,
            border: custom_border,
            border_width: 1.0,
            shadow: custom_shadow,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            text_color: custom_text,
            text_style: crate::text::TextStyle::new(
                FontId(0),
                14.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
        },
    };
    let mut cmds = DrawCommands::new();
    let res = raw::post_layout_keycap(
        spec,
        raw::KeycapPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut text_backend,
        &mut cmds,
    );

    assert_eq!(
        res.content_bounds,
        Rect::new(0.0, 0.0, 30.0, 30.0).inset(1.0)
    );
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                color: custom_bg,
                z: 0,
            },
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                color: custom_border,
                width: 1.0,
                z: 0,
            },
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(1.0, 30.0, 29.0, 2.0),
                color: custom_shadow,
                z: 0,
            },
            DrawCmd::GlyphRun {
                glyphs: 0..1,
                color: custom_text,
                z: 0,
            },
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![DrawGlyph {
            token: PreparedGlyphToken(75),
            top_left: Vec2 { x: 11.0, y: 21.0 }
        }]
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = KeycapSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().text_style.font, theme.mono_font);
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let explicit_style = KeycapStyle {
        background: Color::WHITE,
        shadow: Color::BLACK,
        shadow_offset: 1.0,
        shadow_height: 2.0,
        border: Color::WHITE,
        border_width: 1.0,
        text_color: Color::WHITE,
        text_style: crate::text::TextStyle::new(
            FontId(99),
            14.0,
            400,
            crate::text::TextFlow::single_line(),
        ),
        content_placement: crate::text::TextContentPlacement::CENTER,
    };
    let builder = KeycapSpecBuilder::new().style(explicit_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(explicit_style));
}

#[test]
fn test_high_level_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
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
    let result = super::keycap(&mut ctx, KeycapSpecBuilder::new().text("X"), placement);
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_keycap_bounds_and_content_bounds() {
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
    let custom_border_width = 3.5;
    let result = super::keycap(
        &mut ctx,
        KeycapSpecBuilder::new().text("X").style(KeycapStyle {
            background: Color::WHITE,
            shadow: Color::BLACK,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            border: Color::WHITE,
            border_width: custom_border_width,
            text_color: Color::WHITE,
            text_style: crate::text::TextStyle::new(
                FontId(0),
                14.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
        }),
        layout_rect,
    );
    assert_eq!(result.layout.bounds, layout_rect);
    assert_eq!(
        result.layout.content_bounds,
        layout_rect.inset(custom_border_width)
    );
}
