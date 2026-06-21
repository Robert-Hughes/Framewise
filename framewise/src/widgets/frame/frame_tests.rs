use super::*;
use crate::layouts::ColumnLayout;
use crate::test_utils::TestTextBackend;

#[test]
fn test_frame_layout_and_draw() {
    let mut cmds = DrawCommands::new();
    let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
    let style = FrameStyle {
        background: Color::WHITE,
        border: Some(Stroke::new(Color::linear_rgb(0.5, 0.5, 0.5), 2.0)),
        padding: 3.0,
    };

    let spec = raw::FrameSpec {
        layer: Layer::default(),
        rect,
        style,
    };
    let pre_layout =
        raw::pre_layout_frame(&raw::FramePreLayoutSpec { style }, SizeOffer::UNBOUNDED);
    let raw::FrameResult {
        token,
        content_bounds: content,
    } = raw::begin_frame(spec, pre_layout, &mut cmds);

    // Content rect should be inset by border_width + padding = 5.0
    assert_eq!(content.x, 15.0);
    assert_eq!(content.y, 15.0);
    assert_eq!(content.w, 90.0);
    assert_eq!(content.h, 40.0);

    // FillRect and PushClip placeholders are pushed before children
    assert_eq!(cmds.len(), 2);
    assert!(matches!(
        cmds[0],
        DrawCmd::FillRect {
            anti_alias: false,
            ..
        }
    ));
    assert!(matches!(cmds[1], DrawCmd::PushClip { .. }));

    // end_frame patches both placeholders, then appends PopClip and StrokeRect
    let final_rect = Rect::new(10.0, 10.0, 120.0, 60.0);
    let final_content = final_rect.inset(5.0); // border_width(2) + padding(3)
    raw::end_frame(
        token,
        raw::FrameSpec {
            layer: Layer::default(),
            rect: final_rect,
            ..spec
        },
        &mut cmds,
    );

    assert_eq!(
        &cmds[..],
        &[
            DrawCmd::FillRect {
                anti_alias: false,
                rect: final_rect,
                color: Color::WHITE,
                z: 0,
            },
            DrawCmd::PushClip {
                rect: final_content
            },
            DrawCmd::PopClip,
            DrawCmd::StrokeRect {
                anti_alias: false,
                rect: final_rect,
                color: Color::linear_rgb(0.5, 0.5, 0.5),
                width: 2.0,
                z: 0,
            },
        ]
    );
}

#[test]
fn test_size_frame() {
    let style = FrameStyle {
        background: Color::WHITE,
        border: Some(Stroke::new(Color::BLACK, 2.0)),
        padding: 4.0,
    };
    let spec = raw::FramePreLayoutSpec { style };
    assert_eq!(
        raw::pre_layout_frame(&spec, SizeOffer::UNBOUNDED).size_request,
        crate::layout::SizeRequest::UNKNOWN
    );
}

#[test]
fn test_builder_defaults_from_theme_fills_unset_style() {
    let theme = crate::theme::Theme::framewise();
    let builder = FrameSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert!(builder.style.is_some());
    let expected = FrameStyle::from_theme(&theme);
    assert_eq!(
        builder.style.unwrap().border.unwrap().width,
        expected.border.unwrap().width
    );
    assert_eq!(builder.style.unwrap().padding, expected.padding);
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_style() {
    let theme = crate::theme::Theme::framewise();
    let custom_style = FrameStyle {
        background: Color::TRANSPARENT,
        border: Some(Stroke::new(Color::BLACK, 99.0)),
        padding: 0.0,
    };
    let builder = FrameSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().border.unwrap().width, 99.0);
}

#[test]
fn test_high_level_container_fit_to_children() {
    use crate::layouts::ColumnLayoutParams;
    let mut ts = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = DrawCommands::new();

    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut ts,
        &mut focus,
        &input,
        &mut output,
        ColumnLayout,
        Rect::new(0.0, 0.0, 400.0, 600.0),
        &mut cmds,
    );

    // 1. Begin an auto-sizing frame inside the column
    let style = FrameStyle {
        background: Color::WHITE,
        border: Some(Stroke::new(Color::BLACK, 2.0)),
        padding: 8.0,
    };
    let FrameResult { ctx: mut f_ctx } = begin_frame(
        &mut ctx,
        FrameSpecBuilder::new().style(style),
        ColumnLayoutParams::auto().fill_x(),
        ColumnLayout,
    );

    // 2. Place some children inside the frame context
    // Inner layout starts at (10, 10) due to insets. Fill width spans outer space (400 - 20) = 380.
    let r1 = f_ctx.layout(
        ColumnLayoutParams::auto().fill_x().fixed_y(20.0),
        crate::layout::SizeRequest::UNKNOWN,
    );
    assert_eq!(r1, Rect::new(10.0, 10.0, 380.0, 20.0));

    f_ctx.spacer(5.0);

    let r2 = f_ctx.layout(
        ColumnLayoutParams::auto().fill_x().fixed_y(30.0),
        crate::layout::SizeRequest::UNKNOWN,
    );
    // stack height: 20 + spacing(5) = 25
    assert_eq!(r2, Rect::new(10.0, 35.0, 380.0, 30.0));

    // 3. Finish the frame!
    f_ctx.finish();

    // 4. Verify outer column layout advanced correctly.
    // Child content extent is: width 380, height (35 + 30 - 10) = 55.
    // Total outer size is: height = 55 + inset * 2 = 75.
    // Next sibling y should be: height(75) + spacing(10) = 85.
    ctx.spacer(10.0);
    let sibling = ctx.layout(
        ColumnLayoutParams::fixed(50.0, 30.0),
        crate::layout::SizeRequest::UNKNOWN,
    );
    assert_eq!(sibling.y, 85.0);
}
