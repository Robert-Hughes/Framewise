use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    layout::{LayoutState, SizeOffer},
    text::{layout_text, TextBackend},
    types::{Color, Layer, Rect, Stroke, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::KeycapStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::KeycapStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapResult {
        pub content_bounds: Rect,
    }

    /// Return the size this keycap would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_keycap<T: TextBackend>(
        spec: &KeycapPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> KeycapPreLayoutResult {
        KeycapPreLayoutResult {
            size_request: keycap_size_request(spec, offer, text_backend),
        }
    }

    fn keycap_size_request<T: TextBackend>(
        spec: &KeycapPreLayoutSpec,
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

    /// Low-level keycap widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_keycap<T: TextBackend>(
        spec: KeycapSpec<'_>,
        _pre_layout: KeycapPreLayoutResult,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> KeycapResult {
        // Background + border
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.style.background,
            z: spec.layer.get_z(),
        });
        cmds.push_border_rect(
            spec.rect,
            spec.style.border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );
        // Bottom shadow line
        let shadow_rect = Rect::new(
            spec.rect.x + spec.style.shadow_offset,
            spec.rect.y + spec.rect.h,
            spec.rect.w - spec.style.shadow_offset,
            spec.style.shadow_height,
        );
        cmds.push(DrawCmd::FillRect {
            rect: shadow_rect,
            color: spec.style.shadow,
            z: spec.layer.get_z(),
        });

        // text, centered
        if !spec.text.is_empty() {
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
        }

        KeycapResult {
            content_bounds: spec.rect.inset(spec.style.border.map_or(0.0, |s| s.width)),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a keycap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeycapStyle {
    pub background: Color,
    pub shadow: Color,
    pub shadow_offset: f32,
    pub shadow_height: f32,
    pub border: Option<Stroke>,
    pub text_color: Color,
    pub text_style: crate::text::TextStyle,
    pub content_placement: crate::text::TextContentPlacement,
}

impl KeycapStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            shadow: theme.line_on_paper,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            border: Some(Stroke::new(theme.line_on_paper, theme.border)),
            text_color: theme.ink,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapSpec<'a> {
    pub text: &'a str,
    pub style: KeycapStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeycapSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<KeycapStyle>,
}

impl<'a> KeycapSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: KeycapStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(KeycapStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> KeycapSpec<'a> {
        KeycapSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level keycap widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn keycap<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: KeycapSpecBuilder<'a>,
    layout_params: S::Params,
) -> KeycapResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::KeycapPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_keycap(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::KeycapSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
    };
    let result = raw::post_layout_keycap(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);
    KeycapResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
#[path = "keycap_tests.rs"]
mod tests;
