use super::raw::DividerSpec;
use super::*;
use crate::draw::DrawCmd;
use crate::test_utils::TestTextBackend;

#[test]
fn test_divider_visual() {
    let spec = DividerSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 10.0),
        stroke: Stroke::new(Color::WHITE, 1.0),
    };
    let mut cmds = DrawCommands::new();
    let _res = raw::post_layout_divider(
        spec,
        raw::DividerPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut cmds,
    );

    assert_eq!(
        cmds,
        DrawCommands::from_vec(vec![DrawCmd::StrokeLine {
            p0: Vec2::new(0.0, 5.0),
            p1: Vec2::new(100.0, 5.0),
            color: Color::WHITE,
            width: 1.0,
            z: 0,
        }])
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_color() {
    let theme = crate::theme::Theme::framewise();
    let spec = DividerSpecBuilder::new()
        .defaults_from_theme(&theme)
        .build();
    assert_eq!(spec.stroke.color, theme.line_on_paper);
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_color() {
    let theme = crate::theme::Theme::framewise();
    let sentinel = Color::from_srgb_u8(1, 2, 3, 255);
    let spec = DividerSpecBuilder::new()
        .color(sentinel)
        .defaults_from_theme(&theme)
        .build();
    assert_eq!(spec.stroke.color, sentinel);
}

#[test]
fn test_size_divider_ignores_offer() {
    use crate::layout::AxisBound;

    let spec = raw::DividerPreLayoutSpec {};
    let offers = [
        SizeOffer::UNBOUNDED,
        SizeOffer::new(AxisBound::Exact(50.0), AxisBound::Exact(20.0)),
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::AtMost(40.0)),
    ];

    let expected = raw::pre_layout_divider(&spec, offers[0]).size_request;
    for offer in offers {
        assert_eq!(raw::pre_layout_divider(&spec, offer).size_request, expected);
    }
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
    let result = super::divider(&mut ctx, DividerSpecBuilder::new(), placement);
    assert_eq!(result.layout.bounds, placement);
}
