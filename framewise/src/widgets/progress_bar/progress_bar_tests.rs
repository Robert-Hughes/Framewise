use super::*;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::test_utils::TestTextBackend;

#[test]
fn test_progress_bar_visual_normal() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = raw::ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0), // h=10
        value: 0.5,
        phase: 0.0,
        active: false,
        style,
    };
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 14.0, 100.0, 3.0),
                    color: style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 14.0, 50.0, 3.0),
                    color: style.fill_color,
                    z: 0,
                },
            ],
            1.0
        )
    );
}

#[test]
fn test_progress_bar_visual_active() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = raw::ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0),
        value: 0.5,
        phase: 0.0,
        active: true,
        style,
    };
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 14.0, 100.0, 3.0),
                    color: style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 14.0, 50.0, 3.0),
                    color: style.active_fill_color,
                    z: 0,
                },
            ],
            1.0
        )
    );
}

#[test]
fn test_progress_bar_visual_indeterminate() {
    let style = ProgressBarStyle::from_theme(&crate::theme::Theme::framewise());
    let spec = raw::ProgressBarSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 10.0, 100.0, 10.0),
        value: f32::NAN,
        phase: 0.5,
        active: false,
        style,
    };
    let mut cmds = DrawCommands::new(1.0);
    raw::post_layout_progress_bar(
        spec,
        raw::ProgressBarPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 14.0, 100.0, 3.0),
                    color: style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(60.0, 14.0, 30.0, 3.0),
                    color: style.fill_color,
                    z: 0,
                },
            ],
            1.0
        )
    );
}

#[test]
fn test_spec_defaults_from_theme() {
    let theme = crate::theme::Theme::framewise();
    let spec = ProgressBarSpec::new_from_theme(0.5, &theme);
    assert_eq!(spec.style, ProgressBarStyle::from_theme(&theme));
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
    let result = super::progress_bar(
        ProgressBarSpec::new_from_theme(0.5, &ctx.theme),
        placement,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, placement);
}
