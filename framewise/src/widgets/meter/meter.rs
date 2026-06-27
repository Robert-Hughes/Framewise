use crate::{
    draw::DrawCommands,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

// ── Raw Implementation ───────────────────────────────────────────────────────

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterSpec {
        pub layer: Layer,
        pub rect: Rect,
        /// 0.0 – 1.0 fill level.
        pub value: f32,
        pub style: super::MeterStyle,
        /// 0.0 – 1.0 peak marker position (draw a rust bar at this level; None to skip).
        pub peak: Option<f32>,
        /// Number of bars to display.
        pub bars: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterPreLayoutSpec {
        pub style: super::MeterStyle,
        /// Number of bars to display.
        pub bars: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterResult {}

    /// Return the size this meter would request under `offer`.
    ///
    /// The current implementation ignores `offer` because the request is fixed
    /// by the number of bars and meter style.
    pub fn pre_layout_meter(spec: &MeterPreLayoutSpec, offer: SizeOffer) -> MeterPreLayoutResult {
        MeterPreLayoutResult {
            size_request: meter_size_request(spec, offer),
        }
    }

    fn meter_size_request(spec: &MeterPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        let w = spec.bars as f32 * spec.style.bar_w
            + (spec.bars.saturating_sub(1) as f32) * spec.style.bar_gap;
        let h = spec.style.bar_h;
        SizeRequest::preferred(Vec2::new(w, h))
    }

    /// Low‑level meter draw function.
    ///
    /// Appends draw commands to `cmds`.
    pub fn post_layout_meter(
        spec: MeterSpec,
        _pre_layout: MeterPreLayoutResult,
        cmds: &mut DrawCommands,
    ) {
        let n = spec.bars.max(1);
        let lit = (spec.value.clamp(0.0, 1.0) * n as f32).round() as usize;
        let peak_idx = spec
            .peak
            .map(|p| (p.clamp(0.0, 1.0) * (n - 1) as f32).round() as usize);

        for i in 0..n {
            let x = spec.rect.x + i as f32 * (spec.style.bar_w + spec.style.bar_gap);
            let bar_rect = Rect::new(
                x,
                spec.rect.y + (spec.rect.h - spec.style.bar_h) / 2.0,
                spec.style.bar_w,
                spec.style.bar_h,
            );
            let color = if peak_idx == Some(i) {
                spec.style.rust
            } else if i < lit {
                spec.style.ink
            } else {
                spec.style.unlit
            };
            cmds.push_crisp_fill_rect(bar_rect, color, spec.layer.get_z());
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a meter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterStyle {
    pub bar_w: f32,
    pub bar_h: f32,
    pub bar_gap: f32,
    pub ink: Color,
    pub rust: Color,
    pub unlit: Color,
}

impl MeterStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            bar_w: 6.0,
            bar_h: 14.0,
            bar_gap: 2.0,
            ink: theme.ink,
            rust: theme.rust,
            unlit: theme.muted,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MeterResult {
    pub layout: LayoutInfo,
}

impl Default for MeterStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MeterSpec {
    pub value: f32,
    pub style: MeterStyle,
    pub peak: Option<f32>,
    pub bars: usize,
}

impl MeterSpec {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            style: MeterStyle::default(),
            peak: None,
            bars: 10,
        }
    }

    pub fn new_from_theme(value: f32, theme: &crate::theme::Theme) -> Self {
        Self::new(value).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = MeterStyle::from_theme(theme);
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn style(mut self, style: MeterStyle) -> Self {
        self.style = style;
        self
    }

    pub fn peak(mut self, peak: Option<f32>) -> Self {
        self.peak = peak;
        self
    }

    pub fn bars(mut self, bars: usize) -> Self {
        self.bars = bars;
        self
    }
}

// ── High‑level widget function ───────────────────────────────────────────────────

/// High-level meter widget function using `WidgetContext`.
///
/// Consumes a complete `MeterSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn meter<T: TextBackend, S: LayoutState, CF>(
    spec: MeterSpec,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> MeterResult {
    let pre_layout_spec = raw::MeterPreLayoutSpec {
        style: spec.style,
        bars: spec.bars,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_meter(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::MeterSpec {
        layer: ctx.layer,
        rect,
        value: spec.value,
        style: spec.style,
        peak: spec.peak,
        bars: spec.bars,
    };
    raw::post_layout_meter(raw_spec, pre_layout, ctx.cmds);
    MeterResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "meter_tests.rs"]
mod tests;
