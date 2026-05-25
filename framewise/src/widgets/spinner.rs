use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect, Vec2},
    widget::{WidgetContext, WidgetScope},
};

pub mod raw {
    use super::*;

    /// Low-level spinner widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    /// Square reticle spinner — four corner brackets with a single animated segment.
    /// Since we can't animate, we draw it at a fixed phase (segment at top).
    pub fn spinner(spec: SpinnerSpec) -> SpinnerResult {
        let mut cmds = DrawCommands::new();

        let size = if spec.large {
            spec.style.large_size
        } else {
            spec.style.small_size
        };
        let color = spec.color.unwrap_or(spec.style.color);

        let x = spec.rect.x;
        let y = spec.rect.y;

        // Corner bracket size: 5px at 16, 7px at 24.
        let arm = if spec.large {
            spec.style.large_arm
        } else {
            spec.style.small_arm
        };
        let w = spec.style.width;

        // Top-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x, y + arm),
            p1: Vec2::new(x, y),
            color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x, y),
            p1: Vec2::new(x + arm, y),
            color,
            width: w,
        });
        // Top-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size - arm, y),
            p1: Vec2::new(x + size, y),
            color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y),
            p1: Vec2::new(x + size, y + arm),
            color,
            width: w,
        });
        // Bottom-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y + size - arm),
            p1: Vec2::new(x + size, y + size),
            color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y + size),
            p1: Vec2::new(x + size - arm, y + size),
            color,
            width: w,
        });
        // Bottom-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + arm, y + size),
            p1: Vec2::new(x, y + size),
            color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x, y + size),
            p1: Vec2::new(x, y + size - arm),
            color,
            width: w,
        });

        // Animated segment on the top edge — drawn as a rust highlight.
        let seg_w = size * spec.style.highlight_fraction;
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size * 0.1, y),
            p1: Vec2::new(x + size * 0.1 + seg_w, y),
            color: spec.style.highlight,
            width: w,
        });

        SpinnerResult { draw: cmds }
    }
}

pub struct SpinnerSpec {
    /// Top-left. Size is either 16 or 24 (use `large` flag).
    pub rect: Rect,
    pub large: bool,
    pub color: Option<Color>,
    pub style: SpinnerStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerStyle {
    pub color: Color,
    pub highlight: Color,
    pub small_size: f32,
    pub large_size: f32,
    pub small_arm: f32,
    pub large_arm: f32,
    pub width: f32,
    pub highlight_fraction: f32,
}

impl Default for SpinnerStyle {
    fn default() -> Self {
        Self {
            color: Color::from_srgb_u8(21, 19, 15, 255),
            highlight: Color::from_srgb_u8(194, 90, 44, 255),
            small_size: 16.0,
            large_size: 24.0,
            small_arm: 5.0,
            large_arm: 7.0,
            width: 1.5,
            highlight_fraction: 0.4,
        }
    }
}


pub struct SpinnerSpecBuilder {
    spec: SpinnerSpec,
}

impl SpinnerSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: SpinnerSpec {
                rect: Rect::ZERO,
                large: false,
                color: None,
                style: SpinnerStyle {
                    color: Color::BLACK,
                    highlight: Color::BLACK,
                    small_size: 16.0,
                    large_size: 24.0,
                    small_arm: 5.0,
                    large_arm: 7.0,
                    width: 1.5,
                    highlight_fraction: 0.4,
                },
            },
        }
    }

    pub fn large(mut self, large: bool) -> Self {
        self.spec.large = large;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.spec.color = Some(color);
        self
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.spec.style = style;
        self
    }

    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.spec.style = theme.spinner_style();
        self
    }

    pub fn build(self) -> SpinnerSpec {
        self.spec
    }
}

pub struct SpinnerResult {
    pub draw: DrawCommands,
}

impl SpinnerResult {
    pub fn into_parts(self) -> (DrawCommands, ()) {
        (self.draw, ())
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level spinner widget function using WidgetContext.
///
/// This function accepts a SpinnerSpec and calls the low-level raw::spinner function.
pub fn spinner<T: crate::text::TextSystem, S: crate::layout::LayoutState, Scope: WidgetScope>(
    ctx: &mut WidgetContext<T, S, Scope>,
    layout_params: S::Params,
    builder: SpinnerSpecBuilder,
) {
    let rect = ctx.layout(layout_params);
    let builder = builder
        .with_rect(rect)
        .with_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::spinner(spec);
    ctx.append_cmds(result.draw.0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_visual_normal() {
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            large: false,
            color: None,
            style: Default::default(),
        };
        let style = spec.style;
        let res = raw::spinner(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                // Top-left
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 5.0), p1: Vec2::new(0.0, 0.0), color: style.color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 0.0), p1: Vec2::new(5.0, 0.0), color: style.color, width: style.width },
                // Top-right
                DrawCmd::StrokeLine { p0: Vec2::new(11.0, 0.0), p1: Vec2::new(16.0, 0.0), color: style.color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(16.0, 0.0), p1: Vec2::new(16.0, 5.0), color: style.color, width: style.width },
                // Bottom-right
                DrawCmd::StrokeLine { p0: Vec2::new(16.0, 11.0), p1: Vec2::new(16.0, 16.0), color: style.color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(16.0, 16.0), p1: Vec2::new(11.0, 16.0), color: style.color, width: style.width },
                // Bottom-left
                DrawCmd::StrokeLine { p0: Vec2::new(5.0, 16.0), p1: Vec2::new(0.0, 16.0), color: style.color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 16.0), p1: Vec2::new(0.0, 11.0), color: style.color, width: style.width },
                // Highlight
                DrawCmd::StrokeLine { p0: Vec2::new(1.6, 0.0), p1: Vec2::new(8.0, 0.0), color: style.highlight, width: style.width },
            ])
        );
    }

    #[test]
    fn test_spinner_visual_large_custom_color() {
        let custom_color = Color::from_srgb_f32(0.1, 0.2, 0.3, 1.0);
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 24.0, 24.0),
            large: true,
            color: Some(custom_color),
            style: Default::default(),
        };
        let style = spec.style;
        let res = raw::spinner(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                // Top-left
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 7.0), p1: Vec2::new(0.0, 0.0), color: custom_color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 0.0), p1: Vec2::new(7.0, 0.0), color: custom_color, width: style.width },
                // Top-right
                DrawCmd::StrokeLine { p0: Vec2::new(17.0, 0.0), p1: Vec2::new(24.0, 0.0), color: custom_color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(24.0, 0.0), p1: Vec2::new(24.0, 7.0), color: custom_color, width: style.width },
                // Bottom-right
                DrawCmd::StrokeLine { p0: Vec2::new(24.0, 17.0), p1: Vec2::new(24.0, 24.0), color: custom_color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(24.0, 24.0), p1: Vec2::new(17.0, 24.0), color: custom_color, width: style.width },
                // Bottom-left
                DrawCmd::StrokeLine { p0: Vec2::new(7.0, 24.0), p1: Vec2::new(0.0, 24.0), color: custom_color, width: style.width },
                DrawCmd::StrokeLine { p0: Vec2::new(0.0, 24.0), p1: Vec2::new(0.0, 17.0), color: custom_color, width: style.width },
                // Highlight
                DrawCmd::StrokeLine { p0: Vec2::new(2.4, 0.0), p1: Vec2::new(12.0, 0.0), color: style.highlight, width: style.width },
            ])
        );
    }
}


