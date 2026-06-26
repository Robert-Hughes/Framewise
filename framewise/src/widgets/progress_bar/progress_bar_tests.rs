use super::raw::ProgressBarSpec;
use super::*;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::test_utils::TestTextBackend;

#[test]
fn test_progress_bar_visual_normal() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0), // h=10
        value: 0.5,
        phase: 0.0,
        active: false,
        style,
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                color: style.track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 13.5, 50.0, 3.0),
                color: style.fill_color,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_progress_bar_visual_active() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0),
        value: 0.5,
        phase: 0.0,
        active: true,
        style,
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                color: style.track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 13.5, 50.0, 3.0),
                color: style.active_fill_color,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_progress_bar_visual_indeterminate() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0),
        value: f32::NAN,
        phase: 0.5,
        active: false,
        style,
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                color: style.track_color,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(60.0, 13.5, 30.000002, 3.0),
                color: style.fill_color,
                z: 0,
            },
        ])
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = ProgressBarSpecBuilder::new().value(0.5);
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(ProgressBarStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = ProgressBarStyle::from_theme(&theme);
    custom_style.track_height = 99.0;
    let builder = ProgressBarSpecBuilder::new().value(0.5).style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().track_height, 99.0);
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
    let result = super::progress_bar(
        &mut ctx,
        ProgressBarSpecBuilder::new().value(0.5),
        placement,
    );
    assert_eq!(result.layout.bounds, placement);
}
