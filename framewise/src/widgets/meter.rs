use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

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

impl WidgetResult for MeterResult {
    type Info = MeterInfo;
    fn into_parts(self) -> (DrawCommands, MeterInfo) {
        (self.draw, MeterInfo { layout: self.layout })
    }
}

pub fn meter(spec: MeterSpec) -> MeterResult {
    let mut draw = DrawCommands::new();

    let ink    = Color::rgb(0.082, 0.075, 0.059);
    let rust   = Color::rgb(0.761, 0.353, 0.173);
    let unlit  = Color::new(0.082, 0.075, 0.059, 0.15);

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
