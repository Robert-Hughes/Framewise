use super::raw::ColorSwatchSpec;
use super::*;
use crate::focus::FocusSystem;
use crate::test_utils::TestTextBackend;

#[test]
fn test_color_swatch_visual_normal() {
    let spec = ColorSwatchSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        color: Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0),
        border: Some(Stroke::new(Color::linear_rgba(0.0, 0.0, 0.0, 0.20), 1.0)),
    };
    let mut cmds = DrawCommands::new();
    let res = raw::post_layout_color_swatch(
        spec,
        raw::ColorSwatchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );
    let default_color = Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0);
    let default_border = Color::linear_rgba(0.0, 0.0, 0.0, 0.20);

    assert_eq!(
        res.content_bounds,
        Rect::new(0.0, 0.0, 16.0, 16.0).inset(1.0)
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                color: default_color,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                color: default_border,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_color_swatch_visual_custom() {
    let custom_color = Color::from_srgb_f32(1.0, 0.0, 0.0, 1.0);
    let custom_border = Color::from_srgb_f32(0.0, 1.0, 0.0, 1.0);
    let spec = ColorSwatchSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        color: custom_color,
        border: Some(Stroke::new(custom_border, 1.0)),
    };
    let mut cmds = DrawCommands::new();
    let res = raw::post_layout_color_swatch(
        spec,
        raw::ColorSwatchPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        res.content_bounds,
        Rect::new(0.0, 0.0, 20.0, 20.0).inset(1.0)
    );
    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                color: custom_color,
                z: 0,
            },
            DrawCmd::BorderRect {
                rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                color: custom_border,
                width: 1.0,
                placement: crate::BorderPlacement::Inside,
                z: 0,
            },
        ])
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
    let result = super::color_swatch(
        &mut ctx,
        ColorSwatchSpecBuilder::new().color(Color::from_srgb_u8(0, 0, 0, 0)),
        placement,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_color_swatch_bounds_and_content_bounds() {
    use crate::layouts::ManualLayout;
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
    let result = super::color_swatch(
        &mut ctx,
        ColorSwatchSpecBuilder::new().color(Color::from_srgb_u8(0, 0, 0, 0)),
        layout_rect,
    );
    assert_eq!(result.layout.bounds, layout_rect);
    assert_eq!(result.layout.content_bounds, layout_rect.inset(1.0));
}
