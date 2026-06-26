#[cfg(test)]
use crate::focus::FocusSystem;
use crate::{
    draw::DrawCommands,
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{Color, Layer, Rect, Stroke},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub stroke: Stroke,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerResult {}

    /// Return the size this divider would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a divider has no
    /// inherent preferred size. This returns [`SizeRequest::UNKNOWN`].
    ///
    pub fn pre_layout_divider(
        spec: &DividerPreLayoutSpec,
        offer: SizeOffer,
    ) -> DividerPreLayoutResult {
        DividerPreLayoutResult {
            size_request: divider_size_request(spec, offer),
        }
    }

    fn divider_size_request(
        spec: &DividerPreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// Low-level divider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_divider(
        spec: DividerSpec,
        _pre_layout: DividerPreLayoutResult,
        cmds: &mut DrawCommands,
    ) -> DividerResult {
        if spec.stroke.is_visible() {
            let y =
                cmds.snap_to_physical_pixel(spec.rect.y + (spec.rect.h - spec.stroke.width) * 0.5);
            cmds.push_crisp_fill_rect(
                Rect::new(spec.rect.x, y, spec.rect.w, spec.stroke.width),
                spec.stroke.color,
                spec.layer.get_z(),
            );
        }
        DividerResult {}
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DividerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DividerSpec {
    pub stroke: Stroke,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DividerSpecBuilder {
    pub stroke: Option<Stroke>,
    pub color: Option<Color>,
    pub width: Option<f32>,
}

impl DividerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn stroke(mut self, stroke: Stroke) -> Self {
        self.stroke = Some(stroke);
        self
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.stroke.is_none() {
            let color = self.color.unwrap_or(theme.line_on_paper);
            let width = self.width.unwrap_or(1.0);
            self.stroke = Some(Stroke::new(color, width));
        } else {
            let mut s = self.stroke.unwrap();
            if let Some(c) = self.color {
                s.color = c;
            }
            if let Some(w) = self.width {
                s.width = w;
            }
            self.stroke = Some(s);
        }
        self
    }
    pub fn build(self) -> DividerSpec {
        DividerSpec {
            stroke: self
                .stroke
                .expect("stroke not set — call defaults_from_theme() or stroke()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level divider widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn divider<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: DividerSpecBuilder,
    layout_params: S::Params,
) -> DividerResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::DividerPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_divider(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::DividerSpec {
        layer: ctx.layer,
        rect,
        stroke: spec.stroke,
    };
    let _result = raw::post_layout_divider(raw_spec, pre_layout, ctx.cmds);

    DividerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "divider_tests.rs"]
mod tests;
