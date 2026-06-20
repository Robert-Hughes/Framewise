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
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
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
#[path = "tooltip_tests.rs"]
mod tests;
