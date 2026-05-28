use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
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
        /// 0.0 – 1.0 peak marker position (draw a rust bar at this level; None to skip).
        pub peak: Option<f32>,
        /// Number of bars to display.
        pub bars: usize,
    }

    impl Default for MeterSpec {
        fn default() -> Self {
            Self {
                rect: Rect::new(0.0, 0.0, 80.0, 14.0),
                value: 0.5,
                peak: None,
                bars: 10,
            }
        }
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
        let mut draw = DrawCommands::new();

        let ink = Color::from_srgb_f32(0.082, 0.075, 0.059, 1.0);
        let rust = Color::from_srgb_f32(0.761, 0.353, 0.173, 1.0);
        let unlit = Color::from_srgb_f32(0.082, 0.075, 0.059, 0.15);

        let n = spec.bars.max(1);
        let lit = (spec.value.clamp(0.0, 1.0) * n as f32).round() as usize;
        let peak_idx = spec
            .peak
            .map(|p| (p.clamp(0.0, 1.0) * (n - 1) as f32).round() as usize);

        for i in 0..n {
            let x = spec.rect.x + i as f32 * (BAR_W + BAR_GAP);
            let bar_rect = Rect::new(x, spec.rect.y + (spec.rect.h - BAR_H) / 2.0, BAR_W, BAR_H);

            let color = if peak_idx == Some(i) {
                rust
            } else if i < lit {
                ink
            } else {
                unlit
            };

            draw.push(DrawCmd::FillRect {
                rect: bar_rect,
                color,
            });
        }

        MeterResult { draw }
    }
}

// Bar dimensions matching the CSS spec: 6px wide, 14px tall, 2px gap.
const BAR_W: f32 = 6.0;
const BAR_H: f32 = 14.0;
const BAR_GAP: f32 = 2.0;

pub struct MeterResult {
    pub layout: LayoutInfo,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MeterSpecBuilder {
    pub rect: Option<Rect>,
    pub value: Option<f32>,
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
    pub fn defaults_from_theme(self, _theme: &crate::theme::Theme) -> Self {
        self
    }

    pub fn build(self) -> raw::MeterSpec {
        raw::MeterSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            value: self
                .value
                .expect("value not set — call .value()"),
            peak: self
                .peak
                .expect("peak not set — call .peak()"),
            bars: self
                .bars
                .expect("bars not set — call .bars()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level meter widget function using WidgetContext.
///
/// This function accepts a MeterSpec and calls the low-level raw::meter function.
pub fn meter<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: MeterSpecBuilder,
) -> MeterResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
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
            peak: None,
            bars: 10,
        };
        let res = raw::meter(spec);
        let ink = Color::from_srgb_f32(0.082, 0.075, 0.059, 1.0);
        let unlit = Color::from_srgb_f32(0.082, 0.075, 0.059, 0.15);

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
            peak: Some(0.8), // 0.8 * 9 = 7.2 -> 7
            bars: 10,
        };
        let res = raw::meter(spec);
        let ink = Color::from_srgb_f32(0.082, 0.075, 0.059, 1.0);
        let rust = Color::from_srgb_f32(0.761, 0.353, 0.173, 1.0);
        let unlit = Color::from_srgb_f32(0.082, 0.075, 0.059, 0.15);

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
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::meter(
            &mut ctx,
            layout_rect,
            MeterSpecBuilder::new().rect(custom_rect).value(0.0).peak(None).bars(10),
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
