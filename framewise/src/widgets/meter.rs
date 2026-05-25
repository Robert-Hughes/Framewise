use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level meter widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn meter(spec: MeterSpec) -> MeterResult {
        let mut draw = DrawCommands::new();

        let ink    = Color::from_srgb_f32(0.082, 0.075, 0.059, 1.0);
        let rust   = Color::from_srgb_f32(0.761, 0.353, 0.173, 1.0);
        let unlit  = Color::from_srgb_f32(0.082, 0.075, 0.059, 0.15);

        let n = spec.bars.max(1);
        let lit = (spec.value.clamp(0.0, 1.0) * n as f32).round() as usize;
        let peak_idx = spec.peak.map(|p| (p.clamp(0.0, 1.0) * (n - 1) as f32).round() as usize);

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

            draw.push(DrawCmd::FillRect { rect: bar_rect, color });
        }

        MeterResult {
            draw,
            layout: LayoutInfo::tight(spec.rect),
        }
    }
}

// Bar dimensions matching the CSS spec: 6px wide, 14px tall, 2px gap.
const BAR_W: f32 = 6.0;
const BAR_H: f32 = 14.0;
const BAR_GAP: f32 = 2.0;

pub struct MeterSpec {
    pub rect:  Rect,
    /// 0.0 – 1.0 fill level.
    pub value: f32,
    /// 0.0 – 1.0 peak marker position (draw a rust bar at this level; None to skip).
    pub peak:  Option<f32>,
    /// Number of bars to display.
    pub bars:  usize,
}

impl Default for MeterSpec {
    fn default() -> Self {
        Self { rect: Rect::new(0.0, 0.0, 80.0, 14.0), value: 0.5, peak: None, bars: 10 }
    }
}

pub struct MeterResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct MeterInfo {
    pub layout: LayoutInfo,
}

impl MeterResult {
    pub fn into_parts(self) -> (DrawCommands, MeterInfo) {
        (self.draw, MeterInfo { layout: self.layout })
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level meter widget function using WidgetContext.
///
/// This function accepts a MeterSpec and calls the low-level raw::meter function.
pub fn meter<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    spec: MeterSpec,
) -> MeterInfo {
    let result = raw::meter(spec);
    ctx.append_cmds(result.draw.0);
    MeterInfo { layout: result.layout }
}

// ── Re-export raw function for direct use ───────────────────────────────────────────
pub use raw::meter as meter_raw;


pub struct MeterSpecBuilder {
    pub rect: Option<Rect>,
    pub value: Option<f32>,
    pub peak: Option<Option<f32>>,
    pub bars: Option<usize>,
}

impl MeterSpecBuilder {
    pub fn new() -> Self {
        Self {
            rect: None,
            value: None,
            peak: None,
            bars: None,
        }
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
}

impl MeterSpecBuilder {
    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn build(self) -> MeterSpec {
        let mut spec = MeterSpec::default();
        if let Some(r) = self.rect { spec.rect = r; }
        if let Some(v) = self.value { spec.value = v; }
        if let Some(p) = self.peak { spec.peak = p; }
        if let Some(b) = self.bars { spec.bars = b; }
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meter_visual_normal() {
        let spec = MeterSpec {
            rect: Rect::new(0.0, 0.0, 80.0, 14.0),
            value: 0.5,
            peak: None,
            bars: 10,
        };
        let res = meter(spec);
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
        let res = meter(spec);
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
}
