use super::raw::MeterSpec;
use super::*;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::test_utils::TestTextBackend;

#[test]
fn test_meter_visual_normal() {
    let style = MeterStyle::from_theme(&crate::theme::Theme::default());
    let spec = MeterSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 80.0, 14.0),
        value: 0.5,
        style,
        peak: None,
        bars: 10,
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_meter(
        spec,
        raw::MeterPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    let mut expected = Vec::new();
    for i in 0..10 {
        let color = if i < 5 { style.ink } else { style.unlit };
        expected.push(DrawCmd::FillRect {
            rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
            color,
            z: 0,
        });
    }
    assert_eq!(cmds, DrawCommands::from_vec(expected));
}

#[test]
fn test_meter_visual_peak() {
    let style = MeterStyle::from_theme(&crate::theme::Theme::default());
    let spec = MeterSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 80.0, 14.0),
        value: 0.5,
        style,
        peak: Some(0.8), // 0.8 * 9 ≈ 7.2 → 7
        bars: 10,
    };
    let mut cmds = DrawCommands::new();
    raw::post_layout_meter(
        spec,
        raw::MeterPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    let mut expected = Vec::new();
    for i in 0..10 {
        let color = if i == 7 {
            style.rust
        } else if i < 5 {
            style.ink
        } else {
            style.unlit
        };
        expected.push(DrawCmd::FillRect {
            rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
            color,
            z: 0,
        });
    }
    assert_eq!(cmds, DrawCommands::from_vec(expected));
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
    let result = super::meter(
        &mut ctx,
        MeterSpecBuilder::new().value(0.0).bars(10),
        placement,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_builder_defaults() {
    let theme = crate::theme::Theme::default();
    let spec = MeterSpecBuilder::new()
        .value(0.5)
        .defaults_from_theme(&theme)
        .build();
    assert_eq!(spec.peak, None);
    assert_eq!(spec.bars, 10);
}
