#[cfg(test)]
use crate::focus::FocusSystem;
use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
    TextSystem,
};

pub mod raw {
    use crate::TextSystem;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelResult {
        pub content_bounds: Rect,
    }

    /// Measure a label's intrinsic size from its spec.
    ///
    /// **Must not read `spec.rect`** — this runs before the rect is known, so
    /// callers pass [`Rect::PLACEHOLDER`] (NaN). Intrinsic size depends only on
    /// content and style, never on geometry.
    pub fn calc_label_intrinsic_size<T: TextSystem>(
        spec: &LabelSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let t = text_system.measure(
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
    pub fn label<T: TextSystem>(
        spec: LabelSpec,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> LabelResult {
        let metrics = text_system.measure(
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
            .resolve_rect(spec.rect, metrics);
        let layout = text_system.prepare(spec.text, spec.style.text_style, text_rect);
        cmds.push(DrawCmd::Text {
            rect: text_rect,
            color: spec.style.text_color,
            handle: layout.handle,
        });

        if spec.style.rule {
            let y = spec.rect.y + spec.rect.h;
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(spec.rect.x, y),
                p1: Vec2::new(spec.rect.x + spec.rect.w, y),
                color: spec.style.rule_color,
                width: 1.0,
            });
        }

        LabelResult {
            content_bounds: spec.rect,
        }
    }
}

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
            content_placement: crate::text::TextContentPlacement::TOP_LEFT,
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

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LabelSpecBuilder<'a> {
    pub rect: Option<Rect>,
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
    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(LabelStyle::from_theme(theme));
        }
        self
    }
    pub fn build(self) -> raw::LabelSpec<'a> {
        raw::LabelSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
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
pub fn label<'a, T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: LabelSpecBuilder<'a>,
    layout_params: S::Params,
) -> LabelResult {
    // Build the spec up front with a placeholder rect so we can measure the
    // intrinsic size; the real rect is then determined by the layout system and
    // assigned below. Any `rect` set on the builder is ignored by the high-level
    // path — placement is the layout's job (use `ManualLayout`, or the raw fn,
    // for explicit rects).
    let mut spec = builder
        .defaults_from_theme(&ctx.theme)
        .rect(Rect::PLACEHOLDER)
        .build();
    let intrinsic = raw::calc_label_intrinsic_size(&spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    spec.rect = rect;

    let r = raw::label(spec, ctx.text_system, ctx.cmds);

    LabelResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::LabelSpec;
    use super::*;
    use crate::{
        test_utils::DummyTextSys, text::FontId, text::TextHandle, theme, types::Vec2, Input,
    };

    struct RecordingTextSys {
        font: Option<FontId>,
    }

    struct PlacementTextSys {
        metrics: crate::text::TextMetrics,
        prepared_rect: Option<Rect>,
    }

    impl TextSystem for PlacementTextSys {
        fn measure(
            &mut self,
            _text: &str,
            _style: crate::text::TextStyle,
            _bounds: crate::text::TextBounds,
        ) -> crate::text::TextMetrics {
            self.metrics
        }

        fn prepare(
            &mut self,
            _text: &str,
            _style: crate::text::TextStyle,
            rect: Rect,
        ) -> crate::text::TextLayout {
            self.prepared_rect = Some(rect);
            crate::text::TextLayout {
                handle: TextHandle(7),
                metrics: self.metrics,
            }
        }

        fn caret_geom(&self, _handle: TextHandle, _byte_index: usize) -> crate::text::CaretGeom {
            crate::text::CaretGeom {
                x: 0.0,
                y_top: 0.0,
                height: 0.0,
            }
        }

        fn hit_test(&self, _handle: TextHandle, _pos: Vec2) -> usize {
            0
        }
    }

    impl TextSystem for RecordingTextSys {
        fn measure(
            &mut self,
            _text: &str,
            _style: crate::text::TextStyle,
            _bounds: crate::text::TextBounds,
        ) -> crate::text::TextMetrics {
            crate::text::TextMetrics {
                logical_size: Vec2::new(0.0, 0.0),
                ink_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
                line_count: 1,
                truncated_horizontal: false,
                truncated_vertical: false,
            }
        }

        fn prepare(
            &mut self,
            _text: &str,
            style: crate::text::TextStyle,
            _rect: Rect,
        ) -> crate::text::TextLayout {
            self.font = Some(style.font);
            crate::text::TextLayout {
                handle: TextHandle(0),
                metrics: crate::text::TextMetrics {
                    logical_size: Vec2::new(0.0, 0.0),
                    ink_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
                    line_count: 1,
                    truncated_horizontal: false,
                    truncated_vertical: false,
                },
            }
        }

        fn caret_geom(&self, _handle: TextHandle, _byte_index: usize) -> crate::text::CaretGeom {
            crate::text::CaretGeom {
                x: 0.0,
                y_top: 0.0,
                height: 0.0,
            }
        }

        fn hit_test(&self, _handle: TextHandle, _pos: Vec2) -> usize {
            0
        }
    }

    #[test]
    fn test_label_draws_text() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
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
            &cmds[..],
            &[DrawCmd::Text {
                rect: Rect::new(0.0, 0.0, 40.0, 16.0),
                color: Color::WHITE,
                handle: TextHandle(0),
            }]
        );
    }

    #[test]
    fn test_label_rule() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
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
            &cmds[..],
            &[
                DrawCmd::Text {
                    rect: Rect::new(0.0, 0.0, 56.0, 16.0),
                    color: Color::WHITE,
                    handle: TextHandle(0),
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 20.0),
                    p1: Vec2::new(100.0, 20.0),
                    color: Color::WHITE,
                    width: 1.0,
                }
            ]
        );
    }

    #[test]
    fn test_label_logical_content_placement_bottom_right() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            text: "Hello",
            style: LabelStyle {
                content_placement: crate::text::TextContentPlacement::logical(
                    crate::Align::End,
                    crate::Align::End,
                ),
                ..LabelStyle::from_theme(&theme::Theme::default())
            },
        };
        let mut cmds = DrawCommands::new();
        let _ = raw::label(spec, &mut sys, &mut cmds);

        assert_eq!(
            &cmds[..],
            &[DrawCmd::Text {
                rect: Rect::new(70.0, 54.0, 40.0, 16.0),
                color: LabelStyle::from_theme(&theme::Theme::default()).text_color,
                handle: TextHandle(0),
            }]
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
        };
        let mut sys = PlacementTextSys {
            metrics,
            prepared_rect: None,
        };
        let spec = LabelSpec {
            rect: Rect::new(10.0, 20.0, 100.0, 50.0),
            text: "◎",
            style: LabelStyle {
                content_placement: crate::text::TextContentPlacement::INK_CENTER,
                ..LabelStyle::from_theme(&theme::Theme::default())
            },
        };
        let mut cmds = DrawCommands::new();
        let _ = raw::label(spec, &mut sys, &mut cmds);

        assert_eq!(sys.prepared_rect, Some(Rect::new(55.0, 37.0, 30.0, 20.0)));
    }

    #[test]
    fn test_label_passes_spec_font_to_text_system() {
        let mut sys = RecordingTextSys { font: None };
        let expected = FontId(42);
        let spec = LabelSpec {
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
        let mut text_system = DummyTextSys;
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
        let mut text_system = DummyTextSys;
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
            .iter()
            .any(|c| matches!(c, DrawCmd::Text { color, .. } if *color == custom.text_color));
        assert!(
            has_custom_color,
            "high-level label must honor user-set style"
        );
    }

    #[test]
    fn test_calc_label_intrinsic_size() {
        let mut ts = DummyTextSys;
        let theme = crate::theme::Theme::default();
        let spec = LabelSpec {
            rect: Rect::PLACEHOLDER,
            text: "Hello",
            style: LabelStyle::from_theme(&theme),
        };
        let i = raw::calc_label_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(40.0, 16.0)));
    }

    #[test]
    fn test_label_auto_layout_uses_intrinsic_size() {
        use crate::layouts::{ColumnLayout, ColumnLayoutParams, ManualLayout};
        let mut text_system = DummyTextSys;
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
        let mut ts = DummyTextSys;
        let flow = crate::text::TextFlow::wrapped();
        let theme = crate::theme::Theme::default();
        let mut style = LabelStyle::from_theme(&theme);
        style.text_style.flow = flow;
        let spec = LabelSpec {
            rect: Rect::PLACEHOLDER,
            text: "Hello World",
            style,
        };
        let i = raw::calc_label_intrinsic_size(&spec, &mut ts);
        assert_eq!(i.preferred, Some(Vec2::new(88.0, 16.0)));
    }

    #[test]
    fn test_label_with_custom_flow() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
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
                fallback: crate::text::WrapWordFallback::WrapGlyph {
                    fallback: crate::text::WrapGlyphFallback::Drop,
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
