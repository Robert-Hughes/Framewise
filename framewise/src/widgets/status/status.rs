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
    pub struct StatusSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub variant: super::StatusVariant,
        pub style: super::StatusStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::StatusStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusResult {}

    /// Return the size this status widget would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_status<T: TextBackend>(
        spec: &StatusPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> StatusPreLayoutResult {
        StatusPreLayoutResult {
            size_request: status_size_request(spec, offer, text_backend),
        }
    }

    fn status_size_request<T: TextBackend>(
        spec: &StatusPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
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
        crate::layout::SizeRequest::preferred(size)
    }

    /// Low-level status widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_status<T: TextBackend>(
        spec: StatusSpec<'_>,
        _pre_layout: StatusPreLayoutResult,
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

impl Default for StatusStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
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

impl<'a> StatusSpec<'a> {
    pub fn new(text: &'a str, variant: StatusVariant) -> Self {
        Self {
            text,
            variant,
            style: StatusStyle::default(),
        }
    }

    pub fn new_from_theme(
        text: &'a str,
        variant: StatusVariant,
        theme: &crate::theme::Theme,
    ) -> Self {
        Self::new(text, variant).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = StatusStyle::from_theme(theme);
        self
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }

    pub fn variant(mut self, variant: StatusVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn style(mut self, style: StatusStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level status widget function using `WidgetContext`.
///
/// Consumes a complete `StatusSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn status<'a, T: TextBackend, S: LayoutState, CF>(
    spec: StatusSpec<'a>,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> StatusResult {
    let pre_layout_spec = raw::StatusPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_status(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::StatusSpec {
        rect,
        text: spec.text,
        variant: spec.variant,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::post_layout_status(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);
    StatusResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "status_tests.rs"]
mod tests;
