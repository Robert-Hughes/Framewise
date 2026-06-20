use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeRequest},
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
    pub struct MeterCalcSizeRequestSpec {
        pub style: super::MeterStyle,
        /// Number of bars to display.
        pub bars: usize,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterResult {}

    /// Compute the intrinsic size of a meter widget.
    ///
    /// Width = total bar width + gaps, Height = bar height.
    pub fn calc_meter_intrinsic_size(spec: &MeterCalcSizeRequestSpec) -> SizeRequest {
        let w = spec.bars as f32 * spec.style.bar_w
            + (spec.bars.saturating_sub(1) as f32) * spec.style.bar_gap;
        let h = spec.style.bar_h;
        SizeRequest::preferred(Vec2::new(w, h))
    }

    /// Low‑level meter draw function.
    ///
    /// Appends draw commands to `cmds`.
    pub fn meter(spec: MeterSpec, cmds: &mut DrawCommands) {
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
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: bar_rect,
                color,
                z: spec.layer.get_z(),
            });
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

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MeterSpec {
    pub value: f32,
    pub style: MeterStyle,
    pub peak: Option<f32>,
    pub bars: usize,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeterSpecBuilder {
    pub value: Option<f32>,
    pub style: Option<MeterStyle>,
    pub peak: Option<Option<f32>>, // matches original API
    pub bars: Option<usize>,
}

impl MeterSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn style(mut self, style: MeterStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn peak(mut self, peak: Option<f32>) -> Self {
        self.peak = Some(peak);
        self
    }

    pub fn bars(mut self, bars: usize) -> Self {
        self.bars = Some(bars);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(MeterStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> MeterSpec {
        MeterSpec {
            value: self.value.expect("value not set — call .value()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            peak: self.peak.unwrap_or(None),
            bars: self.bars.unwrap_or(10),
        }
    }
}

// ── High‑level widget function ───────────────────────────────────────────────────

/// High‑level meter widget function using `WidgetContext`.
pub fn meter<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: MeterSpecBuilder,
    layout_params: S::Params,
) -> MeterResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::MeterCalcSizeRequestSpec {
        style: spec.style,
        bars: spec.bars,
    };
    let intrinsic = raw::calc_meter_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::MeterSpec {
        layer: ctx.layer,
        rect,
        value: spec.value,
        style: spec.style,
        peak: spec.peak,
        bars: spec.bars,
    };
    raw::meter(raw_spec, ctx.cmds);
    MeterResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::MeterSpec;
    use super::*;
    use crate::focus::FocusSystem;
    use crate::test_utils::TestTextBackend;

    #[test]
    fn test_meter_visual_normal() {
        let style = MeterStyle::from_theme(&crate::theme::Theme::default());
        let spec = MeterSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 80.0, 14.0),
            value: 0.5,
            style,
            peak: None,
            bars: 10,
        };
        let mut cmds = DrawCommands::new();
        raw::meter(spec, &mut cmds);

        let mut expected = Vec::new();
        for i in 0..10 {
            let color = if i < 5 { style.ink } else { style.unlit };
            expected.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
                color,
                z: 0,
            });
        }
        assert_eq!(cmds, DrawCommands::from_vec(expected));
    }

    #[test]
    fn test_meter_visual_peak() {
        let style = MeterStyle::from_theme(&crate::theme::Theme::default());
        let spec = MeterSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 80.0, 14.0),
            value: 0.5,
            style,
            peak: Some(0.8), // 0.8 * 9 ≈ 7.2 → 7
            bars: 10,
        };
        let mut cmds = DrawCommands::new();
        raw::meter(spec, &mut cmds);

        let mut expected = Vec::new();
        for i in 0..10 {
            let color = if i == 7 {
                style.rust
            } else if i < 5 {
                style.ink
            } else {
                style.unlit
            };
            expected.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
                color,
                z: 0,
            });
        }
        assert_eq!(cmds, DrawCommands::from_vec(expected));
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let result = super::meter(
            &mut ctx,
            MeterSpecBuilder::new().value(0.0).bars(10),
            placement,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_builder_defaults() {
        let theme = crate::theme::Theme::default();
        let spec = MeterSpecBuilder::new()
            .value(0.5)
            .defaults_from_theme(&theme)
            .build();
        assert_eq!(spec.peak, None);
        assert_eq!(spec.bars, 10);
    }
}
