use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

pub struct ColorSwatchSpec {
    pub rect:   Rect,
    pub color:  Color,
    /// Border color drawn around the swatch. Transparent by default.
    pub border: Color,
}

impl Default for ColorSwatchSpec {
    fn default() -> Self {
        Self {
            rect:   Rect::new(0.0, 0.0, 16.0, 16.0),
            color:  Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0),
            border: Color::linear_rgba(0.0, 0.0, 0.0, 0.20),
        }
    }
}

pub struct ColorSwatchResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct ColorSwatchInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for ColorSwatchResult {
    type Info = ColorSwatchInfo;
    fn into_parts(self) -> (DrawCommands, ColorSwatchInfo) {
        (self.draw, ColorSwatchInfo { layout: self.layout })
    }
}

pub fn color_swatch(spec: ColorSwatchSpec) -> ColorSwatchResult {
    let mut draw = DrawCommands::new();
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: spec.color });
    draw.push(DrawCmd::StrokeRect { rect: spec.rect, color: spec.border, width: 1.0 });
    ColorSwatchResult {
        draw,
        layout: LayoutInfo::tight(spec.rect),
    }
}
