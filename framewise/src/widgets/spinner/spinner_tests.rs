use super::raw::SpinnerSpec;
use super::*;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;

#[test]
fn test_spinner_visual_normal() {
    let style = SpinnerStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = SpinnerSpec {
        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        large: false,
        style,
        layer: Layer::default(),
    };
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_spinner(
        spec,
        raw::SpinnerPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(
            vec![
                // Top-left
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 2.0, 5.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 5.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Top-right
                DrawCmd::FillRect {
                    rect: Rect::new(11.0, 0.0, 5.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(15.0, 0.0, 2.0, 5.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Bottom-right
                DrawCmd::FillRect {
                    rect: Rect::new(15.0, 11.0, 2.0, 5.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.0, 15.0, 5.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Bottom-left
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 15.0, 5.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 11.0, 2.0, 5.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Highlight
                DrawCmd::FillRect {
                    rect: Rect::new(2.0, 0.0, 6.0, 2.0),
                    color: style.highlight_stroke.color,
                    z: 0,
                },
            ],
            1.0
        )
    );
}

#[test]
fn test_spinner_visual_large() {
    let style = SpinnerStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = SpinnerSpec {
        rect: Rect::new(0.0, 0.0, 24.0, 24.0),
        large: true,
        style,
        layer: Layer::default(),
    };
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_spinner(
        spec,
        raw::SpinnerPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(
            vec![
                // Top-left
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 2.0, 7.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 7.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Top-right
                DrawCmd::FillRect {
                    rect: Rect::new(17.0, 0.0, 7.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(23.0, 0.0, 2.0, 7.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Bottom-right
                DrawCmd::FillRect {
                    rect: Rect::new(23.0, 17.0, 2.0, 7.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(17.0, 23.0, 7.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Bottom-left
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 23.0, 7.0, 2.0),
                    color: style.stroke.color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 17.0, 2.0, 7.0),
                    color: style.stroke.color,
                    z: 0,
                },
                // Highlight
                DrawCmd::FillRect {
                    rect: Rect::new(2.0, 0.0, 10.0, 2.0),
                    color: style.highlight_stroke.color,
                    z: 0,
                },
            ],
            1.0
        )
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = SpinnerSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(SpinnerStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = SpinnerStyle::from_theme(&theme);
    custom_style.stroke.width = 99.0;
    let builder = SpinnerSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().stroke.width, 99.0);
}

#[test]
fn test_high_level_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
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
    let result = super::spinner(&mut ctx, SpinnerSpecBuilder::new(), placement);
    assert_eq!(result.layout.bounds, placement);
}
