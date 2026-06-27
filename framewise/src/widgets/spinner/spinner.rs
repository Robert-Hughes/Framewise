use crate::{
    draw::DrawCommands,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{Layer, Rect, Stroke},
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

        let sw = spec.style.stroke.width;

        // Top-left bracket.
        cmds.push_crisp_v_rule(x, y, arm, Some(spec.style.stroke), spec.layer.get_z());
        cmds.push_crisp_h_rule(x, y, arm, Some(spec.style.stroke), spec.layer.get_z());
        // Top-right bracket.
        cmds.push_crisp_h_rule(
            x + size - arm,
            y,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );
        cmds.push_crisp_v_rule(
            x + size - sw,
            y,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );
        // Bottom-right bracket.
        cmds.push_crisp_v_rule(
            x + size - sw,
            y + size - arm,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );
        cmds.push_crisp_h_rule(
            x + size - arm,
            y + size - sw,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );
        // Bottom-left bracket.
        cmds.push_crisp_h_rule(
            x,
            y + size - sw,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );
        cmds.push_crisp_v_rule(
            x,
            y + size - arm,
            arm,
            Some(spec.style.stroke),
            spec.layer.get_z(),
        );

        // Animated segment on the top edge — drawn as a highlight.
        let seg_w = size * spec.style.highlight_fraction;
        cmds.push_crisp_h_rule(
            x + size * 0.1,
            y,
            seg_w,
            Some(spec.style.highlight_stroke),
            spec.layer.get_z(),
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

impl Default for SpinnerStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SpinnerSpec {
    pub large: bool,
    pub style: SpinnerStyle,
}

impl SpinnerSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = SpinnerStyle::from_theme(theme);
        self
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = large;
        self
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level spinner widget function using `WidgetContext`.
///
/// Consumes a complete `SpinnerSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase (which appends draw commands even though the raw result is empty).
pub fn spinner<T: TextBackend, S: LayoutState, CF>(
    spec: SpinnerSpec,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SpinnerResult {
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
