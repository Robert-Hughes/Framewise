#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{Color, Layer, Rect, Stroke},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub color: Color,
        /// Border stroke drawn around the swatch.
        pub border: Option<Stroke>,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchResult {
        pub content_bounds: Rect,
    }

    /// Return the size this color swatch would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a color swatch has no
    /// inherent preferred size. This returns [`SizeRequest::UNKNOWN`].
    ///
    pub fn pre_layout_color_swatch(
        spec: &ColorSwatchPreLayoutSpec,
        offer: SizeOffer,
    ) -> ColorSwatchPreLayoutResult {
        ColorSwatchPreLayoutResult {
            size_request: color_swatch_size_request(spec, offer),
        }
    }

    fn color_swatch_size_request(
        spec: &ColorSwatchPreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// Low-level color swatch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_color_swatch(
        spec: ColorSwatchSpec,
        _pre_layout: ColorSwatchPreLayoutResult,
        cmds: &mut DrawCommands,
    ) -> ColorSwatchResult {
        cmds.push_crisp_fill_rect(spec.rect, spec.color, spec.layer.get_z());
        cmds.push_crisp_border_rect(
            spec.rect,
            spec.border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );
        ColorSwatchResult {
            content_bounds: spec.rect.inset(spec.border.map_or(0.0, |b| b.width)),
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatchResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatchSpec {
    pub color: Color,
    /// Border stroke drawn around the swatch.
    pub border: Option<Stroke>,
}

impl ColorSwatchSpec {
    pub fn new(color: Color) -> Self {
        Self {
            color,
            border: Some(Stroke::new(crate::theme::Theme::minimal().ink, 1.0)),
        }
    }

    pub fn new_from_theme(color: Color, theme: &crate::theme::Theme) -> Self {
        Self::new(color).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.border = Some(Stroke::new(theme.ink, 1.0));
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn border(mut self, border: Option<Stroke>) -> Self {
        self.border = border;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level color swatch widget function using `WidgetContext`.
///
/// Consumes a complete `ColorSwatchSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn color_swatch<T: TextBackend, S: LayoutState, CF>(
    spec: ColorSwatchSpec,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ColorSwatchResult {
    let pre_layout_spec = raw::ColorSwatchPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_color_swatch(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ColorSwatchSpec {
        layer: ctx.layer,
        rect,
        color: spec.color,
        border: spec.border,
    };
    let result = raw::post_layout_color_swatch(raw_spec, pre_layout, ctx.cmds);
    ColorSwatchResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
#[path = "color_swatch_tests.rs"]
mod tests;
