use crate::{
    draw::DrawCommands,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{Layer, Rect, Stroke, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerSpec {
        /// Top-left. Size is either 16 or 24 (use `large` flag).
        pub rect: Rect,
        pub large: bool,
        pub style: super::SpinnerStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerResult {}

    /// Return the size this spinner would request under `offer`.
    ///
    /// The current implementation ignores `offer` because the spinner's extent
    /// is caller-driven. This returns [`SizeRequest::UNKNOWN`].
    pub fn pre_layout_spinner(
        spec: &SpinnerPreLayoutSpec,
        offer: SizeOffer,
    ) -> SpinnerPreLayoutResult {
        SpinnerPreLayoutResult {
            size_request: spinner_size_request(spec, offer),
        }
    }

    fn spinner_size_request(_spec: &SpinnerPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        SizeRequest::UNKNOWN
    }

    /// Low-level spinner widget function.
    ///
    /// Appends draw commands to `cmds`.
    /// Square reticle spinner — four corner brackets with a single animated segment.
    /// Since we can't animate, we draw it at a fixed phase (segment at top).
    pub fn post_layout_spinner(
        spec: SpinnerSpec,
        _pre_layout: SpinnerPreLayoutResult,
        cmds: &mut DrawCommands,
    ) {
        let size = if spec.large {
            spec.style.large_size
        } else {
            spec.style.small_size
        };

        let x = spec.rect.x;
        let y = spec.rect.y;

        // Corner bracket size: 5px at 16, 7px at 24.
        let arm = if spec.large {
            spec.style.large_arm
        } else {
            spec.style.small_arm
        };

        // Top-left bracket.
        cmds.push_stroke_line(
            Vec2::new(x, y + arm),
            Vec2::new(x, y),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        cmds.push_stroke_line(
            Vec2::new(x, y),
            Vec2::new(x + arm, y),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        // Top-right bracket.
        cmds.push_stroke_line(
            Vec2::new(x + size - arm, y),
            Vec2::new(x + size, y),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        cmds.push_stroke_line(
            Vec2::new(x + size, y),
            Vec2::new(x + size, y + arm),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        // Bottom-right bracket.
        cmds.push_stroke_line(
            Vec2::new(x + size, y + size - arm),
            Vec2::new(x + size, y + size),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        cmds.push_stroke_line(
            Vec2::new(x + size, y + size),
            Vec2::new(x + size - arm, y + size),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        // Bottom-left bracket.
        cmds.push_stroke_line(
            Vec2::new(x + arm, y + size),
            Vec2::new(x, y + size),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );
        cmds.push_stroke_line(
            Vec2::new(x, y + size),
            Vec2::new(x, y + size - arm),
            Some(spec.style.stroke),
            spec.layer.get_z(),
            false,
        );

        // Animated segment on the top edge — drawn as a highlight.
        let seg_w = size * spec.style.highlight_fraction;
        cmds.push_stroke_line(
            Vec2::new(x + size * 0.1, y),
            Vec2::new(x + size * 0.1 + seg_w, y),
            Some(spec.style.highlight_stroke),
            spec.layer.get_z(),
            false,
        );
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerStyle {
    pub stroke: Stroke,
    pub highlight_stroke: Stroke,
    pub small_size: f32,
    pub large_size: f32,
    pub small_arm: f32,
    pub large_arm: f32,
    pub highlight_fraction: f32,
}

impl SpinnerStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            stroke: Stroke::new(theme.ink, 1.5),
            highlight_stroke: Stroke::new(theme.rust, 1.5),
            small_size: 16.0,
            large_size: 24.0,
            small_arm: 5.0,
            large_arm: 7.0,
            highlight_fraction: 0.4,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerSpec {
    pub large: bool,
    pub style: SpinnerStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpinnerSpecBuilder {
    pub large: Option<bool>,
    pub style: Option<SpinnerStyle>,
}

impl SpinnerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = Some(large);
        self
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SpinnerStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SpinnerSpec {
        SpinnerSpec {
            large: self.large.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level spinner widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn spinner<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SpinnerSpecBuilder,
    layout_params: S::Params,
) -> SpinnerResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::SpinnerPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_spinner(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SpinnerSpec {
        rect,
        large: spec.large,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::post_layout_spinner(raw_spec, pre_layout, ctx.cmds);
    SpinnerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
