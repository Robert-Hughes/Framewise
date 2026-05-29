use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct MeterSpec {
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
    pub struct MeterResult {
        pub draw: DrawCommands,
    }

    /// Low-level meter widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn meter(spec: MeterSpec) -> MeterResult {
        let mut cmds = DrawCommands::new();

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
                rect: bar_rect,
                color,
            });
        }

        MeterResult { draw: cmds }
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

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeterSpecBuilder {
    pub rect: Option<Rect>,
    pub value: Option<f32>,
    pub style: Option<MeterStyle>,
    pub peak: Option<Option<f32>>,
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

    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(MeterStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> raw::MeterSpec {
        raw::MeterSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            value: self.value.expect("value not set — call .value()"),
            style: self
                .style
                .expect("style not set — call .style() or .defaults_from_theme()"),
            peak: self.peak.unwrap_or(None),
            bars: self.bars.unwrap_or(10),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level meter widget function using WidgetContext.
///
/// This function accepts a MeterSpecBuilder and calls the low-level raw::meter function.
pub fn meter<T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: MeterSpecBuilder,
    layout_params: S::Params,
) -> MeterResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::meter(spec);
    ctx.append_cmds(result.draw);
    MeterResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::MeterSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_meter_visual_normal() {
        let spec = MeterSpec {
            rect: Rect::new(0.0, 0.0, 80.0, 14.0),
            value: 0.5,
            style: MeterStyle::from_theme(&crate::theme::Theme::default()),
            peak: None,
            bars: 10,
        };
        let ink = spec.style.ink;
        let unlit = spec.style.unlit;
        let res = raw::meter(spec);

        let mut expected = Vec::new();
        for i in 0..10 {
            let color = if i < 5 { ink } else { unlit };
            expected.push(DrawCmd::FillRect {
                rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
                color,
            });
        }

        assert_eq!(res.draw, DrawCommands(expected));
    }

    #[test]
    fn test_meter_visual_peak() {
        let spec = MeterSpec {
            rect: Rect::new(0.0, 0.0, 80.0, 14.0),
            value: 0.5,
            style: MeterStyle::from_theme(&crate::theme::Theme::default()),
            peak: Some(0.8), // 0.8 * 9 = 7.2 -> 7
            bars: 10,
        };
        let ink = spec.style.ink;
        let rust = spec.style.rust;
        let unlit = spec.style.unlit;
        let res = raw::meter(spec);

        let mut expected = Vec::new();
        for i in 0..10 {
            let color = if i == 7 {
                rust
            } else if i < 5 {
                ink
            } else {
                unlit
            };
            expected.push(DrawCmd::FillRect {
                rect: Rect::new(i as f32 * 8.0, 0.0, 6.0, 14.0),
                color,
            });
        }

        assert_eq!(res.draw, DrawCommands(expected));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::meter(
            &mut ctx,
            MeterSpecBuilder::new()
                .rect(custom_rect)
                .value(0.0)
                .peak(None)
                .bars(10),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }

    #[test]
    fn test_builder_defaults() {
        let theme = crate::theme::Theme::default();
        let spec = MeterSpecBuilder::new()
            .rect(Rect::new(0.0, 0.0, 100.0, 20.0))
            .value(0.5)
            .defaults_from_theme(&theme)
            .build();
        assert_eq!(spec.peak, None);
        assert_eq!(spec.bars, 10);
    }
}
