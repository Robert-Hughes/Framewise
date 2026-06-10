use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Layer, Rect},
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
    pub fn calc_status_intrinsic_size<T: TextSystem>(
        spec: &StatusCalcIntrinsicSizeSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let metrics = text_system.measure(
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
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
    pub fn status<T: TextSystem>(
        spec: StatusSpec<'_>,
        text_system: &mut T,
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
            rect: Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
            color: dot_color,
        });

        let metrics = text_system.measure(
            spec.text,
            s.text_style,
            crate::text::TextBounds {
                max_width: Some((spec.rect.w - dot_size - gap).max(0.0)),
                max_height: Some(spec.rect.h),
            },
        );
        let ty = spec.rect.y + (dot_size - metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            spec.rect.x + dot_size + gap,
            ty,
            metrics.logical_size.x,
            metrics.logical_size.y,
        );
        let layout = text_system.prepare(spec.text, s.text_style, text_rect);
        cmds.push(DrawCmd::Text {
            rect: text_rect,
            color: s.text,
            handle: layout.handle,
        });
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
pub fn status<'a, T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: StatusSpecBuilder<'a>,
    layout_params: S::Params,
) -> StatusResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::StatusCalcIntrinsicSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let intrinsic = raw::calc_status_intrinsic_size(&calc_spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::StatusSpec {
        rect,
        text: spec.text,
        variant: spec.variant,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::status(raw_spec, ctx.text_system, ctx.cmds);
    StatusResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::StatusSpec;
    use super::*;
    use crate::{focus::FocusSystem, test_utils::DummyTextSys};

    #[test]
    fn test_status_visual_ok() {
        let mut text_system = DummyTextSys;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Online",
            variant: StatusVariant::Ok,
            style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        raw::status(spec, &mut text_system, &mut cmds);

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.ok,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, -5.0, 48.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_status_visual_warn() {
        let mut text_system = DummyTextSys;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Warning",
            variant: StatusVariant::Warn,
            style: StatusStyle::from_theme(&crate::theme::Theme::framewise()),
            layer: Layer::default(),
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        raw::status(spec, &mut text_system, &mut cmds);

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.warn,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, -5.0, 56.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
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
