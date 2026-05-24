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


pub struct ColorSwatchSpecBuilder {
    pub rect: Option<Rect>,
    pub color: Option<Color>,
    pub border: Option<Color>,
}

impl ColorSwatchSpecBuilder {
    pub fn new() -> Self {
        Self {
            rect: None,
            color: None,
            border: None,
        }
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    
    pub fn border(mut self, border: Color) -> Self {
        self.border = Some(border);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for ColorSwatchSpecBuilder {
    type Spec = ColorSwatchSpec;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn build(self) -> Self::Spec {
        let mut spec = ColorSwatchSpec::default();
        if let Some(r) = self.rect { spec.rect = r; }
        if let Some(c) = self.color { spec.color = c; }
        if let Some(b) = self.border { spec.border = b; }
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_swatch_visual_normal() {
        let spec = ColorSwatchSpec::default();
        let res = color_swatch(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);
        let default_color = Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0);
        let default_border = Color::linear_rgba(0.0, 0.0, 0.0, 0.20);
        
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == default_color));
        assert!(matches!(&cmds[1], DrawCmd::StrokeRect { color, width, .. } if *color == default_border && *width == 1.0));
    }

    #[test]
    fn test_color_swatch_visual_custom() {
        let custom_color = Color::from_srgb_f32(1.0, 0.0, 0.0, 1.0);
        let custom_border = Color::from_srgb_f32(0.0, 1.0, 0.0, 1.0);
        let spec = ColorSwatchSpec {
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            color: custom_color,
            border: custom_border,
        };
        let res = color_swatch(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == custom_color));
        assert!(matches!(&cmds[1], DrawCmd::StrokeRect { color, width, .. } if *color == custom_border && *width == 1.0));
    }
}
