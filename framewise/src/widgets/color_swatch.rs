use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level color swatch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn color_swatch(spec: ColorSwatchSpec) -> ColorSwatchResult {
        let mut draw = DrawCommands::new();
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.color,
        });
        draw.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: spec.border,
            width: 1.0,
        });
        ColorSwatchResult {
            draw,
            layout: LayoutInfo::tight(spec.rect),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatchSpec {
    pub rect: Rect,
    pub color: Color,
    /// Border color drawn around the swatch. Transparent by default.
    pub border: Color,
}

impl Default for ColorSwatchSpec {
    fn default() -> Self {
        Self {
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            color: Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0),
            border: Color::linear_rgba(0.0, 0.0, 0.0, 0.20),
        }
    }
}

pub struct ColorSwatchResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct ColorSwatchInfo {
    pub layout: LayoutInfo,
}

impl ColorSwatchResult {
    pub fn into_parts(self) -> (DrawCommands, ColorSwatchInfo) {
        (
            self.draw,
            ColorSwatchInfo {
                layout: self.layout,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level color swatch widget function using WidgetContext.
///
/// This function accepts a ColorSwatchSpec and calls the low-level raw::color_swatch function.
pub fn color_swatch<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: ColorSwatchSpecBuilder,
) -> ColorSwatchInfo {
    let rect = ctx.layout(layout_params);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::color_swatch(spec);
    ctx.append_cmds(result.draw.0);
    ColorSwatchInfo {
        layout: result.layout,
    }
}

#[derive(Debug, Clone, PartialEq)]
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

impl ColorSwatchSpecBuilder {
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

    pub fn build(self) -> ColorSwatchSpec {
        let mut spec = ColorSwatchSpec::default();
        if let Some(r) = self.rect {
            spec.rect = r;
        }
        if let Some(c) = self.color {
            spec.color = c;
        }
        if let Some(b) = self.border {
            spec.border = b;
        }
        spec
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_swatch_visual_normal() {
        let spec = ColorSwatchSpec::default();
        let res = raw::color_swatch(spec);
        let default_color = Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0);
        let default_border = Color::linear_rgba(0.0, 0.0, 0.0, 0.20);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                    color: default_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                    color: default_border,
                    width: 1.0,
                },
            ])
        );
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
        let res = raw::color_swatch(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    color: custom_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    color: custom_border,
                    width: 1.0,
                },
            ])
        );
    }
}
