#[cfg(test)]
use crate::focus::FocusSystem;
use crate::{
    draw::DrawCommands,
    layout::{LayoutState, SizeOffer},
    types::{Color, Layer, Rect, Stroke, Vec2},
    widget::{LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::layout_text;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::LabelStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct LabelResult {
        pub content_bounds: Rect,
    }

    /// Return the size this label would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_label<T: TextBackend>(
        spec: &LabelPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> LabelPreLayoutResult {
        LabelPreLayoutResult {
            size_request: label_size_request(spec, offer, text_backend),
        }
    }

    fn label_size_request<T: TextBackend>(
        spec: &LabelPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        crate::layout::SizeRequest::preferred(layout.metrics().logical_size)
    }

    /// Low-level label widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_label<T: TextBackend>(
        spec: LabelSpec,
        _pre_layout: LabelPreLayoutResult,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> LabelResult {
        let layout = layout_text(
            text_backend,
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
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            spec.style.text_color,
            spec.layer.get_z(),
        );

        if let Some(rule_stroke) = spec.style.rule {
            let y = spec.rect.y + spec.rect.h;
            cmds.push_stroke_line(
                Vec2::new(spec.rect.x, y),
                Vec2::new(spec.rect.x + spec.rect.w, y),
                Some(rule_stroke),
                spec.layer.get_z(),
            );
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
    /// Note that line alignment (`text_style.flow.line_align`) positions each
    /// shaped line internally within the prepared text block, while layout alignment
    /// (`Placement2D::align_x`) moves the entire bounding box inside its parent cell.
    /// Content placement (`content_placement`) moves the prepared text block inside
    /// the label's own rect.
    pub text_style: crate::text::TextStyle,
    /// Placement of the prepared text block inside the label's own rect.
    pub content_placement: crate::text::TextContentPlacement,
    pub text_color: Color,
    pub rule: Option<Stroke>,
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
            rule: None,
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

/// High-level label widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn label<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: LabelSpecBuilder<'a>,
    layout_params: S::Params,
) -> LabelResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::LabelPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_label(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::LabelSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
    };

    let r = raw::post_layout_label(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);

    LabelResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
    }
}

#[cfg(test)]
#[path = "label_tests.rs"]
mod tests;
