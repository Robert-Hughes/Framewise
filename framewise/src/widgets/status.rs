use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    text::{layout_text, TextBackend},
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub variant: super::StatusVariant,
        pub style: super::StatusStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusCalcIntrinsicSizeSpec<'a> {
        pub text: &'a str,
        pub style: super::StatusStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusResult {}

    /// Measure a status widget's intrinsic size from its measurement spec.
    pub fn calc_status_intrinsic_size<T: TextBackend>(
        spec: &StatusCalcIntrinsicSizeSpec,
        text_backend: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let metrics = layout.metrics();
        let size = crate::types::Vec2::new(
            spec.style.dot_size + spec.style.gap + metrics.logical_size.x,
            spec.style.dot_size.max(metrics.logical_size.y),
        );
        crate::layout::IntrinsicSize::preferred(size)
    }

    /// Low-level status widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn status<T: TextBackend>(
        spec: StatusSpec<'_>,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) {
        let s = spec.style;

        let dot_size = s.dot_size;
        let gap = s.gap;

        let dot_color = match spec.variant {
            StatusVariant::Neutral => s.neutral,
            StatusVariant::Ok => s.ok,
            StatusVariant::Warn => s.warn,
            StatusVariant::Err => s.err,
            StatusVariant::Live => s.live,
        };

        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
            color: dot_color,
            z: spec.layer.get_z(),
        });

        let layout = layout_text(
            text_backend,
            spec.text,
            s.text_style,
            crate::text::TextBounds {
                max_width: Some((spec.rect.w - dot_size - gap).max(0.0)),
                max_height: Some(spec.rect.h),
            },
        );
        let metrics = layout.metrics();
        let ty = spec.rect.y + (dot_size - metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            spec.rect.x + dot_size + gap,
            ty,
            metrics.logical_size.x,
            metrics.logical_size.y,
        );
        layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            s.text,
            spec.layer.get_z(),
        );
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusVariant {
    Neutral,
    Ok,
    Warn,
    Err,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusStyle {
    pub dot_size: f32,
    pub gap: f32,
    pub text_style: crate::text::TextStyle,
    pub neutral: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub live: Color,
    pub text: Color,
}

impl StatusStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            dot_size: 6.0,
            gap: 8.0,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            neutral: theme.muted,
            ok: theme.ok,
            warn: theme.rust,
            err: theme.err,
            live: theme.rust,
            text: theme.muted,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct StatusResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct StatusSpec<'a> {
    pub text: &'a str,
    pub variant: StatusVariant,
    pub style: StatusStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct StatusSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub variant: Option<StatusVariant>,
    pub style: Option<StatusStyle>,
}

impl<'a> StatusSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: StatusStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: StatusVariant) -> Self {
        self.variant = Some(variant);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(StatusStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> StatusSpec<'a> {
        StatusSpec {
            text: self.text.expect("text not set — call .text()"),
            variant: self.variant.expect("variant not set — call .variant()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level status widget function using WidgetContext.
///
/// This function accepts a StatusSpecBuilder and calls the low-level raw::status function.
pub fn status<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: StatusSpecBuilder<'a>,
    layout_params: S::Params,
) -> StatusResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::StatusCalcIntrinsicSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let intrinsic = raw::calc_status_intrinsic_size(&calc_spec, ctx.text_backend);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::StatusSpec {
        rect,
        text: spec.text,
        variant: spec.variant,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::status(raw_spec, ctx.text_backend, ctx.cmds);
    StatusResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::StatusSpec;
    use super::*;
    use crate::types::Vec2;
    use crate::{focus::FocusSystem, test_utils::TestTextBackend, DrawGlyph, PreparedGlyphToken};

    #[test]
    fn test_status_visual_ok() {
        let mut text_backend = TestTextBackend;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Online",
            variant: StatusVariant::Ok,
            style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        raw::status(spec, &mut text_backend, &mut cmds);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.ok,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..6,
                    color: style.text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 14.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 22.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(108),
                    top_left: Vec2 { x: 30.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 38.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 46.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(101),
                    top_left: Vec2 { x: 54.0, y: 6.0 },
                },
            ]
        );
    }

    #[test]
    fn test_status_visual_warn() {
        let mut text_backend = TestTextBackend;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Warning",
            variant: StatusVariant::Warn,
            style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        raw::status(spec, &mut text_backend, &mut cmds);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.warn,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: style.text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(87),
                    top_left: Vec2 { x: 14.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 22.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(114),
                    top_left: Vec2 { x: 30.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 38.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 46.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 54.0, y: 6.0 },
                },
                DrawGlyph {
                    token: PreparedGlyphToken(103),
                    top_left: Vec2 { x: 62.0, y: 6.0 },
                },
            ]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = StatusSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(StatusStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = StatusStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = StatusSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
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
        let result = super::status(
            &mut ctx,
            StatusSpecBuilder::new()
                .text("ok")
                .variant(StatusVariant::Ok),
            placement,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
