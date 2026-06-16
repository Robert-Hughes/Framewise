#[cfg(test)]
use crate::focus::FocusSystem;
use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::{layout_text, measure_text};

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelCalcIntrinsicSizeSpec<'a> {
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelResult {
        pub content_bounds: Rect,
    }

    /// Measure a label's intrinsic size from its measurement spec.
    pub fn calc_label_intrinsic_size<T: TextBackend>(
        spec: &LabelCalcIntrinsicSizeSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let t = measure_text(
            text_system,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        crate::layout::IntrinsicSize::preferred(t.logical_size)
    }

    /// Low-level label widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn label<T: TextBackend>(
        spec: LabelSpec,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> LabelResult {
        let layout = layout_text(
            text_system,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds {
                max_width: Some(spec.rect.w),
                max_height: Some(spec.rect.h),
            },
        );
        let text_rect = spec
            .style
            .content_placement
            .resolve_rect(spec.rect, layout.metrics().clone());
        layout.emit_glyphs(
            cmds,
            text_system,
            Vec2::new(text_rect.x, text_rect.y),
            spec.style.text_style,
            spec.style.text_color,
            0,
        );

        if spec.style.rule {
            let y = spec.rect.y + spec.rect.h;
            cmds.push(DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(spec.rect.x, y),
                p1: Vec2::new(spec.rect.x + spec.rect.w, y),
                color: spec.style.rule_color,
                width: 1.0,
                z: 0,
            });
        }

        LabelResult {
            content_bounds: spec.rect,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a label.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelStyle {
    /// How text lines flow, align, and clip internally inside the prepared text block.
    ///
    /// Note that line alignment (`text_flow.line_align`) positions each shaped line
    /// internally within the prepared text block, while layout alignment
    /// (`Placement2D::align_x`) moves the entire bounding box inside its parent cell.
    /// Content placement (`content_placement`) moves the prepared text block inside
    /// the label's own rect.
    pub text_style: crate::text::TextStyle,
    /// Placement of the prepared text block inside the label's own rect.
    pub content_placement: crate::text::TextContentPlacement,
    pub text_color: Color,
    pub rule: bool,
    pub rule_color: Color,
}

impl LabelStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::logical(
                crate::text::ContentPlacement::Fill,
                crate::text::ContentPlacement::Align(crate::Align::Start),
            ),
            text_color: theme.ink,
            rule: false,
            rule_color: theme.line,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct LabelResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct LabelSpec<'a> {
    pub text: &'a str,
    pub style: LabelStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LabelSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<LabelStyle>,
}

impl<'a> LabelSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: LabelStyle) -> Self {
        self.style = Some(style);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(LabelStyle::from_theme(theme));
        }
        self
    }
    pub fn build(self) -> LabelSpec<'a> {
        LabelSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level label widget function using WidgetContext.
///
/// This function accepts a LabelSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::label function.
pub fn label<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: LabelSpecBuilder<'a>,
    layout_params: S::Params,
) -> LabelResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::LabelCalcIntrinsicSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let intrinsic = raw::calc_label_intrinsic_size(&calc_spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::LabelSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
    };

    let r = raw::label(raw_spec, ctx.text_system, ctx.cmds);

    LabelResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::LabelSpec;
    use super::*;
    use crate::{
        test_utils::TestTextBackend,
        text::FontId,
        text::{PrepareGlyphRequest, ShapedCluster, ShapedGlyph, ShapedText},
        theme, DrawGlyph, Input, PreparedGlyphHandle,
    };

    struct RecordingTextSys {
        font: Option<FontId>,
    }

    struct PlacementTextSys {
        metrics: crate::text::TextMetrics,
        prepared_rect: Option<Rect>,
    }

    impl TextBackend for PlacementTextSys {
        type ShapedGlyphId = u32;

        fn line_height(&mut self, _style: crate::text::TextStyle) -> f32 {
            self.metrics.logical_size.y.max(1.0)
        }

        fn shape_text(
            &mut self,
            text: &str,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            if text.is_empty() {
                return ShapedText {
                    clusters: Vec::new(),
                };
            }
            ShapedText {
                clusters: vec![ShapedCluster {
                    byte_start: 0,
                    byte_end: text.len(),
                    advance: self.metrics.logical_size.x,
                    is_whitespace: false,
                    glyphs: vec![ShapedGlyph {
                        id: 1,
                        x: 0.0,
                        y: -style.size.round(),
                        advance: self.metrics.logical_size.x,
                    }],
                }],
            }
        }

        fn shape_ellipsis(
            &mut self,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            self.shape_text(".", style)
        }

        fn prepare_glyph(
            &mut self,
            request: PrepareGlyphRequest<Self::ShapedGlyphId>,
        ) -> Option<DrawGlyph> {
            self.prepared_rect = Some(Rect::new(
                request.glyph_origin.x,
                request.glyph_origin.y,
                self.metrics.logical_size.x,
                self.metrics.logical_size.y,
            ));
            Some(DrawGlyph {
                handle: PreparedGlyphHandle(request.glyph),
                top_left: request.glyph_origin,
            })
        }
    }

    impl TextBackend for RecordingTextSys {
        type ShapedGlyphId = u32;

        fn line_height(&mut self, _style: crate::text::TextStyle) -> f32 {
            16.0
        }

        fn shape_text(
            &mut self,
            text: &str,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            self.font = Some(style.font);
            if text.is_empty() {
                return ShapedText {
                    clusters: Vec::new(),
                };
            }
            ShapedText {
                clusters: vec![ShapedCluster {
                    byte_start: 0,
                    byte_end: text.len(),
                    advance: 1.0,
                    is_whitespace: false,
                    glyphs: vec![ShapedGlyph {
                        id: 1,
                        x: 0.0,
                        y: -style.size.round(),
                        advance: 1.0,
                    }],
                }],
            }
        }

        fn shape_ellipsis(
            &mut self,
            style: crate::text::TextStyle,
        ) -> ShapedText<Self::ShapedGlyphId> {
            self.shape_text(".", style)
        }

        fn prepare_glyph(
            &mut self,
            request: PrepareGlyphRequest<Self::ShapedGlyphId>,
        ) -> Option<DrawGlyph> {
            self.font = Some(request.style.font);
            Some(DrawGlyph {
                handle: PreparedGlyphHandle(request.glyph),
                top_left: request.glyph_origin,
            })
        }
    }

    #[test]
    fn test_label_draws_text() {
        let mut sys = TestTextBackend;
        let spec = LabelSpec {
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
                rule: false,
                rule_color: Color::WHITE,
            },
        };
        let mut cmds = DrawCommands::new();
        let res = raw::label(spec, &mut sys, &mut cmds);

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
                    handle: PreparedGlyphHandle(72),
                    top_left: Vec2 { x: 0.0, y: 16.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(101),
                    top_left: Vec2 { x: 8.0, y: 16.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(108),
                    top_left: Vec2 { x: 16.0, y: 16.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(108),
                    top_left: Vec2 { x: 24.0, y: 16.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(111),
                    top_left: Vec2 { x: 32.0, y: 16.0 },
                },
            ]
        );
    }

    #[test]
    fn test_label_rule() {
        let mut sys = TestTextBackend;
        let spec = LabelSpec {
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
                rule: true,
                rule_color: Color::WHITE,
            },
        };
        let mut cmds = DrawCommands::new();
        let res = raw::label(spec, &mut sys, &mut cmds);
        assert_eq!(res.content_bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: Color::WHITE,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 20.0),
                    p1: Vec2::new(100.0, 20.0),
                    color: Color::WHITE,
                    width: 1.0,
                    z: 0,
                }
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(83),
                    top_left: Vec2 { x: 0.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(101),
                    top_left: Vec2 { x: 8.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(99),
                    top_left: Vec2 { x: 16.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(116),
                    top_left: Vec2 { x: 24.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(105),
                    top_left: Vec2 { x: 32.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(111),
                    top_left: Vec2 { x: 40.0, y: 14.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(110),
                    top_left: Vec2 { x: 48.0, y: 14.0 },
                },
            ]
        );
    }

    #[test]
    fn test_label_logical_content_placement_bottom_right() {
        let mut sys = TestTextBackend;
        let spec = LabelSpec {
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
        let mut cmds = DrawCommands::new();
        let _ = raw::label(spec, &mut sys, &mut cmds);

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
                    handle: PreparedGlyphHandle(72),
                    top_left: Vec2 { x: 70.0, y: 67.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(101),
                    top_left: Vec2 { x: 78.0, y: 67.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(108),
                    top_left: Vec2 { x: 86.0, y: 67.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(108),
                    top_left: Vec2 { x: 94.0, y: 67.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(111),
                    top_left: Vec2 { x: 102.0, y: 67.0 },
                },
            ]
        );
    }

    #[test]
    fn test_label_ink_content_placement_uses_ink_bounds() {
        let metrics = crate::text::TextMetrics {
            logical_size: Vec2::new(30.0, 20.0),
            ink_bounds: Rect::new(-4.0, 3.0, 18.0, 10.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
            lines: Vec::new(),
        };
        let mut sys = PlacementTextSys {
            metrics,
            prepared_rect: None,
        };
        let spec = LabelSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            text: "◎",
            style: LabelStyle {
                content_placement: crate::text::TextContentPlacement::INK_CENTER,
                ..LabelStyle::from_theme(&theme::Theme::default())
            },
        };
        let mut cmds = DrawCommands::new();
        let _ = raw::label(spec, &mut sys, &mut cmds);

        assert_eq!(sys.prepared_rect, Some(Rect::new(45.0, 35.0, 30.0, 20.0)));
    }

    #[test]
    fn test_label_passes_spec_font_to_text_system() {
        let mut sys = RecordingTextSys { font: None };
        let expected = FontId(42);
        let spec = LabelSpec {
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
                rule: false,
                rule_color: Color::WHITE,
            },
        };

        let mut cmds = DrawCommands::new();
        let _ = raw::label(spec, &mut sys, &mut cmds);

        assert_eq!(sys.font, Some(expected));
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = LabelSpecBuilder::new().text("test");
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(LabelStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let custom_style = LabelStyle {
            text_style: crate::text::TextStyle::new(
                FontId(99),
                99.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::TOP_LEFT,
            text_color: Color::from_srgb_u8(1, 2, 3, 255),
            rule: true,
            rule_color: Color::from_srgb_u8(4, 5, 6, 255),
        };
        let builder = LabelSpecBuilder::new().text("test").style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(custom_style));
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let result = super::label(&mut ctx, LabelSpecBuilder::new().text("X"), placement);
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_high_level_honors_user_style() {
        use crate::layouts::ManualLayout;
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
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
            &mut ctx,
            LabelSpecBuilder::new().text("X").style(custom),
            Rect::new(100.0, 100.0, 40.0, 28.0),
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
    fn test_calc_label_intrinsic_size() {
        let mut ts = TestTextBackend;
        let theme = crate::theme::Theme::default();
        let spec = raw::LabelCalcIntrinsicSizeSpec {
            text: "Hello",
            style: LabelStyle::from_theme(&theme),
        };
        let i = raw::calc_label_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(40.0, 16.0)));
    }

    #[test]
    fn test_label_auto_layout_uses_intrinsic_size() {
        use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut ctx = WidgetContext::root(
            theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 300.0, 400.0), ColumnLayout);
        let r = super::label(
            &mut col,
            LabelSpecBuilder::new().text("Hello"),
            ColumnLayoutParams::auto(),
        );
        assert_eq!(r.layout.bounds, Rect::new(10.0, 10.0, 40.0, 16.0));
    }

    #[test]
    fn test_calc_label_intrinsic_size_with_custom_flow() {
        let mut ts = TestTextBackend;
        let flow = crate::text::TextFlow::wrapped();
        let theme = crate::theme::Theme::default();
        let mut style = LabelStyle::from_theme(&theme);
        style.text_style.flow = flow;
        let spec = raw::LabelCalcIntrinsicSizeSpec {
            text: "Hello World",
            style,
        };
        let i = raw::calc_label_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(88.0, 16.0)));
    }

    #[test]
    fn test_label_with_custom_flow() {
        use crate::layouts::ManualLayout;
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
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
            &mut ctx,
            LabelSpecBuilder::new().text("Hello").style(style),
            Rect::new(10.0, 20.0, 200.0, 50.0),
        );

        assert_eq!(result.layout.bounds, Rect::new(10.0, 20.0, 200.0, 50.0));
    }
}
