use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
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
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.color,
            z: spec.layer.get_z(),
        });
        cmds.push_border_rect(
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
    pub border: Option<Stroke>,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorSwatchSpecBuilder {
    pub color: Option<Color>,
    pub border: Option<Option<Stroke>>,
}

impl ColorSwatchSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn border(mut self, border: Option<Stroke>) -> Self {
        self.border = Some(border);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.border.is_none() {
            self.border = Some(Some(Stroke::new(theme.ink, 1.0)));
        }
        // Note color doesn't come from theme - this is the colour being indicated by the swatch!
        self
    }

    pub fn build(self) -> ColorSwatchSpec {
        ColorSwatchSpec {
            color: self.color.expect("color not set — call .color()"),
            border: self
                .border
                .expect("border not set — call .border() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level color swatch widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn color_swatch<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ColorSwatchSpecBuilder,
    layout_params: S::Params,
) -> ColorSwatchResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
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
