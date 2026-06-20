use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeOffer},
    text::{layout_text, TextBackend},
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub variant: super::TooltipVariant,
        pub style: super::TooltipStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::TooltipStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipResult {
        pub bounds: Rect,
        pub content_bounds: Rect,
    }

    /// Return the size this tooltip would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_tooltip<T: TextBackend>(
        spec: &TooltipPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> TooltipPreLayoutResult {
        TooltipPreLayoutResult {
            size_request: tooltip_size_request(spec, offer, text_backend),
        }
    }

    fn tooltip_size_request<T: TextBackend>(
        spec: &TooltipPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let s = spec.style;
        let layout = layout_text(
            text_backend,
            spec.text,
            s.text_style,
            crate::text::TextBounds::width((s.max_width - s.pad_x * 2.0).max(0.0)),
        );
        let metrics = layout.metrics();
        let box_w = (metrics.logical_size.x + s.pad_x * 2.0).min(s.max_width);
        let box_h = metrics.logical_size.y + s.pad_y_top + s.pad_y_bot;
        crate::layout::SizeRequest::preferred(Vec2::new(box_w, box_h))
    }

    /// Low-level tooltip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_tooltip<T: TextBackend>(
        spec: TooltipSpec<'_>,
        _pre_layout: TooltipPreLayoutResult,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> TooltipResult {
        let s = spec.style;

        let pad_x = s.pad_x;
        let pad_y_top = s.pad_y_top;
        let pad_y_bot = s.pad_y_bot;
        let arrow_h = s.arrow_h;
        let arrow_w = s.arrow_w;

        let (bg, text_color): (Color, Color) = match spec.variant {
            TooltipVariant::Dark => (s.dark_bg, s.dark_text),
            TooltipVariant::Rust => (s.rust_bg, s.rust_text),
        };

        let layout = layout_text(
            text_backend,
            spec.text,
            s.text_style,
            crate::text::TextBounds::width((s.max_width - pad_x * 2.0).max(0.0)),
        );
        let metrics = layout.metrics();
        let box_w = (metrics.logical_size.x + pad_x * 2.0).min(s.max_width);
        let box_h = metrics.logical_size.y + pad_y_top + pad_y_bot;

        let r = Rect::new(spec.rect.x, spec.rect.y, box_w, box_h);
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: r,
            color: bg,
            z: spec.layer.get_z(),
        });

        let text_rect = Rect::new(
            r.x + pad_x,
            r.y + pad_y_top,
            metrics.logical_size.x,
            metrics.logical_size.y,
        );
        layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            text_color,
            spec.layer.get_z(),
        );

        // Arrow triangle below (two lines converging to a point).
        let arrow_x = r.x + s.arrow_x;
        let arrow_y = r.y + box_h;
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(arrow_x, arrow_y),
            p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
            color: bg,
            width: s.arrow_width,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(arrow_x + arrow_w, arrow_y),
            p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
            color: bg,
            width: s.arrow_width,
            z: spec.layer.get_z(),
        });

        let content_bounds = Rect::new(
            r.x + pad_x,
            r.y + pad_y_top,
            r.w - pad_x * 2.0,
            r.h - (pad_y_top + pad_y_bot),
        );

        TooltipResult {
            bounds: r,
            content_bounds,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipVariant {
    Dark,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipStyle {
    pub text_style: crate::text::TextStyle,
    pub pad_x: f32,
    pub pad_y_top: f32,
    pub pad_y_bot: f32,
    pub arrow_h: f32,
    pub arrow_w: f32,
    pub arrow_x: f32,
    pub max_width: f32,
    pub dark_bg: Color,
    pub dark_text: Color,
    pub rust_bg: Color,
    pub rust_text: Color,
    pub arrow_width: f32,
}

impl TooltipStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            pad_x: 8.0,
            pad_y_top: 5.0,
            pad_y_bot: 6.0,
            arrow_h: 4.0,
            arrow_w: 8.0,
            arrow_x: 14.0,
            max_width: 240.0,
            dark_bg: theme.ink,
            dark_text: theme.paper,
            rust_bg: theme.rust,
            rust_text: Color::WHITE,
            arrow_width: 1.5,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipSpec<'a> {
    pub text: &'a str,
    pub variant: TooltipVariant,
    pub style: TooltipStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TooltipSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub variant: Option<TooltipVariant>,
    pub style: Option<TooltipStyle>,
}

impl<'a> TooltipSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: TooltipStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: TooltipVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(TooltipStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> TooltipSpec<'a> {
        TooltipSpec {
            text: self.text.expect("text not set — call .text()"),
            variant: self.variant.expect("variant not set — call .variant()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tooltip widget function using `WidgetContext`.
///
/// Resolves defaults, queries the layout offer, asks the raw size helper for a
/// `SizeRequest`, resolves the final rect with `layout`, then calls the raw
/// widget.
pub fn tooltip<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TooltipSpecBuilder<'a>,
    layout_params: S::Params,
) -> TooltipResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::TooltipPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_tooltip(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::TooltipSpec {
        rect,
        text: spec.text,
        variant: spec.variant,
        style: spec.style,
        layer: ctx.layer,
    };
    let result = raw::post_layout_tooltip(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);
    TooltipResult {
        layout: LayoutInfo::new(result.bounds, result.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::TooltipSpec;
    use super::*;
    use crate::{focus::FocusSystem, test_utils::TestTextBackend, DrawGlyph, PreparedGlyphToken};

    #[test]
    fn test_tooltip_visual_dark() {
        let mut text_backend = TestTextBackend;
        let spec = TooltipSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            variant: TooltipVariant::Dark,
            style: TooltipStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        let res = raw::post_layout_tooltip(
            spec,
            raw::TooltipPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut text_backend,
            &mut cmds,
        );

        assert_eq!(res.bounds, Rect::new(0.0, 0.0, 72.0, 27.0));
        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                    color: style.dark_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: style.dark_text,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(14.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.dark_bg,
                    width: style.arrow_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(22.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.dark_bg,
                    width: style.arrow_width,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 8.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 16.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 24.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(108),
                    top_left: Vec2 { x: 32.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 40.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 48.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 56.0, y: 16.0 }
                }
            ]
        );
    }

    #[test]
    fn test_tooltip_visual_rust() {
        let mut text_backend = TestTextBackend;
        let spec = TooltipSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            variant: TooltipVariant::Rust,
            style: TooltipStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        let res = raw::post_layout_tooltip(
            spec,
            raw::TooltipPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut text_backend,
            &mut cmds,
        );

        assert_eq!(res.bounds, Rect::new(0.0, 0.0, 72.0, 27.0));
        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                    color: style.rust_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: style.rust_text,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(14.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.rust_bg,
                    width: style.arrow_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(22.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.rust_bg,
                    width: style.arrow_width,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 8.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 16.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 24.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(108),
                    top_left: Vec2 { x: 32.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 40.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 48.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 56.0, y: 16.0 }
                }
            ]
        );
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let result = super::tooltip(
            &mut ctx,
            TooltipSpecBuilder::new()
                .text("hi")
                .variant(TooltipVariant::Dark),
            placement,
        );
        assert_eq!(result.layout.bounds.x, placement.x);
        assert_eq!(result.layout.bounds.y, placement.y);
    }

    #[test]
    fn test_tooltip_bounds_and_content_bounds() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let res = super::tooltip(
            &mut ctx,
            TooltipSpecBuilder::new()
                .text("hi")
                .variant(TooltipVariant::Dark),
            layout_rect,
        );

        let style = TooltipStyle::from_theme(&ctx.theme);
        let expected_w = (16.0 + style.pad_x * 2.0).min(style.max_width);
        let expected_h = 16.0 + style.pad_y_top + style.pad_y_bot;

        assert_eq!(
            res.layout.bounds,
            Rect::new(layout_rect.x, layout_rect.y, expected_w, expected_h)
        );

        let expected_content = Rect::new(
            layout_rect.x + style.pad_x,
            layout_rect.y + style.pad_y_top,
            expected_w - style.pad_x * 2.0,
            expected_h - (style.pad_y_top + style.pad_y_bot),
        );
        assert_eq!(res.layout.content_bounds, expected_content);
    }
}
