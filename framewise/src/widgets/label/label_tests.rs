use super::raw::LabelSpec as RawLabelSpec;
use super::*;
use crate::{
    draw::DrawCmd, test_utils::TestTextBackend, text::FontId, theme, DrawGlyph, Input,
    PreparedGlyphToken,
};

fn placement_text_backend() -> TestTextBackend {
    TestTextBackend::default()
        .with_line_height(20)
        .with_default_advance(30.0)
        .with_glyph_offset(Vec2::new(0.0, -13.0))
        .with_glyph_ink_bounds(Rect::new(-4.0, 3.0, 18.0, 10.0))
}

#[test]
fn test_label_draws_text() {
    let mut sys = TestTextBackend::default();
    let spec = RawLabelSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 50.0),
        text: "Hello",
        style: LabelStyle {
            text_style: crate::text::TextStyle::new(
                FontId(1),
                16.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::TOP_LEFT,
            text_color: Color::WHITE,
            rule: None,
        },
    };
    let mut cmds = DrawCommands::new(1.0);
    let res = raw::post_layout_label(
        spec,
        raw::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut sys,
        &mut cmds,
    );

    assert_eq!(res.content_bounds, Rect::new(0.0, 0.0, 100.0, 50.0));
    assert_eq!(
        cmds.commands(),
        vec![DrawCmd::GlyphRun {
            glyphs: 0..5,
            color: Color::WHITE,
            z: 0,
        }]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(72),
                top_left: Vec2 { x: 0.0, y: 16.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(101),
                top_left: Vec2 { x: 8.0, y: 16.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 16.0, y: 16.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 24.0, y: 16.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 32.0, y: 16.0 },
            },
        ]
    );
}

#[test]
fn test_label_rule() {
    let mut sys = TestTextBackend::default();
    let spec = RawLabelSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        text: "Section",
        style: LabelStyle {
            text_style: crate::text::TextStyle::new(
                FontId(1),
                14.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::TOP_LEFT,
            text_color: Color::WHITE,
            rule: Some(Stroke::new(Color::WHITE, 1.0)),
        },
    };
    let mut cmds = DrawCommands::new(1.0);
    let res = raw::post_layout_label(
        spec,
        raw::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut sys,
        &mut cmds,
    );
    assert_eq!(res.content_bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
    assert_eq!(
        cmds.commands(),
        vec![
            DrawCmd::GlyphRun {
                glyphs: 0..7,
                color: Color::WHITE,
                z: 0,
            },
            DrawCmd::FillRect {
                rect: Rect::new(0.0, 19.0, 100.0, 1.0),
                color: Color::WHITE,
                z: 0,
            }
        ]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(83),
                top_left: Vec2 { x: 0.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(101),
                top_left: Vec2 { x: 8.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(99),
                top_left: Vec2 { x: 16.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(116),
                top_left: Vec2 { x: 24.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(105),
                top_left: Vec2 { x: 32.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 40.0, y: 14.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(110),
                top_left: Vec2 { x: 48.0, y: 14.0 },
            },
        ]
    );
}

#[test]
fn test_label_logical_content_placement_bottom_right() {
    let mut sys = TestTextBackend::default();
    let spec = RawLabelSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 20.0, 100.0, 50.0),
        text: "Hello",
        style: LabelStyle {
            content_placement: crate::text::TextContentPlacement::logical(
                crate::text::ContentPlacement::Align(crate::Align::End),
                crate::text::ContentPlacement::Align(crate::Align::End),
            ),
            ..LabelStyle::from_theme(&theme::Theme::default())
        },
    };
    let mut cmds = DrawCommands::new(1.0);
    let _ = raw::post_layout_label(
        spec,
        raw::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut sys,
        &mut cmds,
    );

    assert_eq!(
        cmds.commands(),
        vec![DrawCmd::GlyphRun {
            glyphs: 0..5,
            color: LabelStyle::from_theme(&theme::Theme::default()).text_color,
            z: 0,
        }]
    );
    assert_eq!(
        cmds.glyphs(),
        vec![
            DrawGlyph {
                token: PreparedGlyphToken(72),
                top_left: Vec2 { x: 70.0, y: 67.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(101),
                top_left: Vec2 { x: 78.0, y: 67.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 86.0, y: 67.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(108),
                top_left: Vec2 { x: 94.0, y: 67.0 },
            },
            DrawGlyph {
                token: PreparedGlyphToken(111),
                top_left: Vec2 { x: 102.0, y: 67.0 },
            },
        ]
    );
}

#[test]
fn test_label_ink_content_placement_uses_ink_bounds() {
    let mut sys = placement_text_backend();
    let spec = RawLabelSpec {
        layer: Layer::default(),
        rect: Rect::new(10.0, 20.0, 100.0, 50.0),
        text: "◎",
        style: LabelStyle {
            content_placement: crate::text::TextContentPlacement::INK_CENTER,
            ..LabelStyle::from_theme(&theme::Theme::default())
        },
    };
    let mut cmds = DrawCommands::new(1.0);
    let _ = raw::post_layout_label(
        spec,
        raw::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut sys,
        &mut cmds,
    );

    assert_eq!(
        sys.observations.prepared_glyph_rects.first().copied(),
        Some(Rect::new(55.0, 37.0, 30.0, 20.0))
    );
}

#[test]
fn test_label_passes_spec_font_to_text_backend() {
    let mut sys = TestTextBackend::default().with_default_advance(1.0);
    let expected = FontId(42);
    let spec = RawLabelSpec {
        layer: Layer::default(),
        rect: Rect::new(0.0, 0.0, 100.0, 20.0),
        text: "font",
        style: LabelStyle {
            text_style: crate::text::TextStyle::new(
                expected,
                14.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::TOP_LEFT,
            text_color: Color::WHITE,
            rule: None,
        },
    };

    let mut cmds = DrawCommands::new(1.0);
    let _ = raw::post_layout_label(
        spec,
        raw::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut sys,
        &mut cmds,
    );

    assert_eq!(
        sys.observations
            .shaped_styles
            .first()
            .map(|style| style.font),
        Some(expected)
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
    let result = super::label(
        LabelSpec::new_from_theme("X", &ctx.theme),
        placement,
        &mut ctx,
    );
    assert_eq!(result.layout.bounds, placement);
}

#[test]
fn test_high_level_honors_user_style() {
    use crate::layouts::ManualLayout;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
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
    let theme = crate::theme::Theme::framewise();
    let custom = LabelStyle {
        text_color: Color::from_srgb_u8(1, 2, 3, 255),
        ..LabelStyle::from_theme(&theme)
    };
    super::label(
        LabelSpec::new("X").style(custom),
        Rect::new(100.0, 100.0, 40.0, 28.0),
        &mut ctx,
    );
    let has_custom_color = cmds
        .commands()
        .iter()
        .any(|c| matches!(c, DrawCmd::GlyphRun { color, .. } if *color == custom.text_color));
    assert!(
        has_custom_color,
        "high-level label must honor user-set style"
    );
}

#[test]
fn test_size_label() {
    let mut ts = TestTextBackend::default();
    let theme = crate::theme::Theme::default();
    let spec = raw::LabelPreLayoutSpec {
        text: "Hello",
        style: LabelStyle::from_theme(&theme),
    };
    let i = raw::pre_layout_label(&spec, SizeOffer::UNBOUNDED, &mut ts).size_request;
    assert_eq!(i.preferred, Some(Vec2::new(40.0, 16.0)));
}

#[test]
fn test_size_label_ignores_offer() {
    use crate::layout::AxisBound;

    let theme = crate::theme::Theme::default();
    let spec = raw::LabelPreLayoutSpec {
        text: "Hello",
        style: LabelStyle::from_theme(&theme),
    };
    let offers = [
        SizeOffer::UNBOUNDED,
        SizeOffer::new(AxisBound::Exact(50.0), AxisBound::Exact(20.0)),
        SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::AtMost(40.0)),
    ];

    let mut ts = TestTextBackend::default();
    let expected = raw::pre_layout_label(&spec, offers[0], &mut ts).size_request;
    for offer in offers {
        let mut ts = TestTextBackend::default();
        assert_eq!(
            raw::pre_layout_label(&spec, offer, &mut ts).size_request,
            expected
        );
    }
}

#[test]
fn test_label_peeks_offer_before_layout() {
    use crate::layout::{AxisBound, Layout, LayoutResult, LayoutSpace, LayoutToken, SizeRequest};
    use std::{cell::RefCell, rc::Rc};

    #[derive(Clone)]
    struct RecordingLayout {
        calls: Rc<RefCell<Vec<&'static str>>>,
    }

    struct RecordingLayoutState {
        calls: Rc<RefCell<Vec<&'static str>>>,
    }

    impl Layout for RecordingLayout {
        type Params = ();
        type State = RecordingLayoutState;

        fn begin(self, _space: impl Into<LayoutSpace>) -> Self::State {
            RecordingLayoutState { calls: self.calls }
        }
    }

    impl crate::layout::LayoutState for RecordingLayoutState {
        type Params = ();

        fn peek_offer(&self, _layout_params: Self::Params) -> LayoutResult<SizeOffer> {
            self.calls.borrow_mut().push("peek_offer");
            LayoutResult::Ok(SizeOffer::new(
                AxisBound::Exact(123.0),
                AxisBound::AtMost(45.0),
            ))
        }

        fn layout(
            &mut self,
            _layout_params: Self::Params,
            _request: SizeRequest,
        ) -> LayoutResult<Rect> {
            self.calls.borrow_mut().push("layout");
            LayoutResult::Ok(Rect::new(10.0, 20.0, 30.0, 40.0))
        }

        fn begin_deferred_layout<'a>(
            &'a mut self,
            layout_params: Self::Params,
        ) -> (LayoutResult<LayoutSpace>, LayoutToken<'a, Self>) {
            (
                LayoutResult::Ok(LayoutSpace::new(
                    0.0,
                    0.0,
                    AxisBound::Exact(0.0),
                    AxisBound::Exact(0.0),
                )),
                LayoutToken {
                    state: self,
                    params: layout_params,
                },
            )
        }

        fn end_deferred_layout(
            &mut self,
            _layout_params: Self::Params,
            _extent: Vec2,
        ) -> LayoutResult<Rect> {
            LayoutResult::Ok(Rect::new(0.0, 0.0, 0.0, 0.0))
        }

        fn resolve_space(&self) -> Rect {
            Rect::new(0.0, 0.0, 0.0, 0.0)
        }
    }

    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let mut output = crate::Output::default();
    let mut ctx = crate::widget::WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        RecordingLayout {
            calls: calls.clone(),
        },
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );

    let result = super::label(LabelSpec::new_from_theme("Hello", &ctx.theme), (), &mut ctx);

    assert_eq!(&*calls.borrow(), &["peek_offer", "layout"]);
    assert_eq!(result.layout.bounds, Rect::new(10.0, 20.0, 30.0, 40.0));
}

#[test]
fn test_label_auto_layout_uses_size_request() {
    use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = Input::default();
    let mut cmds = DrawCommands::new(1.0);
    let mut output = crate::Output::default();
    let mut ctx = WidgetContext::root(
        theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 300.0, 400.0), ColumnLayout);
    let r = super::label(
        LabelSpec::new_from_theme("Hello", &col.theme),
        ColumnLayoutParams::auto(),
        &mut col,
    );
    assert_eq!(r.layout.bounds, Rect::new(10.0, 10.0, 40.0, 16.0));
}

#[test]
fn test_size_label_with_custom_flow() {
    let mut ts = TestTextBackend::default();
    let flow = crate::text::TextFlow::wrapped();
    let theme = crate::theme::Theme::default();
    let mut style = LabelStyle::from_theme(&theme);
    style.text_style.flow = flow;
    let spec = raw::LabelPreLayoutSpec {
        text: "Hello World",
        style,
    };
    let i = raw::pre_layout_label(&spec, SizeOffer::UNBOUNDED, &mut ts).size_request;
    assert_eq!(i.preferred, Some(Vec2::new(88.0, 16.0)));
}

#[test]
fn test_label_with_custom_flow() {
    use crate::layouts::ManualLayout;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
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

    let flow = crate::text::TextFlow {
        overflow_x: crate::text::OverflowX::WrapWord {
            fallback: crate::text::WrapWordFallback::WrapCluster {
                fallback: crate::text::WrapClusterFallback::Drop,
            },
        },
        overflow_y: crate::text::OverflowY::Ellipsis {
            fallback: crate::text::EllipsisFallback::Drop,
        },
        line_align: crate::text::TextLineAlign::Center,
    };

    let mut style = LabelStyle::from_theme(&crate::theme::Theme::framewise());
    style.text_style.flow = flow;
    let result = super::label(
        LabelSpec::new("Hello").style(style),
        Rect::new(10.0, 20.0, 200.0, 50.0),
        &mut ctx,
    );

    assert_eq!(result.layout.bounds, Rect::new(10.0, 20.0, 200.0, 50.0));
}
