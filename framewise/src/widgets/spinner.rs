use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerSpec {
        /// Top-left. Size is either 16 or 24 (use `large` flag).
        pub rect: Rect,
        pub large: bool,
        pub style: super::SpinnerStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerResult {
        pub draw: DrawCommands,
    }

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
            color: spec.style.color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x, y),
            p1: Vec2::new(x + arm, y),
            color: spec.style.color,
            width: w,
        });
        // Top-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size - arm, y),
            p1: Vec2::new(x + size, y),
            color: spec.style.color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y),
            p1: Vec2::new(x + size, y + arm),
            color: spec.style.color,
            width: w,
        });
        // Bottom-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y + size - arm),
            p1: Vec2::new(x + size, y + size),
            color: spec.style.color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + size, y + size),
            p1: Vec2::new(x + size - arm, y + size),
            color: spec.style.color,
            width: w,
        });
        // Bottom-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x + arm, y + size),
            p1: Vec2::new(x, y + size),
            color: spec.style.color,
            width: w,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(x, y + size),
            p1: Vec2::new(x, y + size - arm),
            color: spec.style.color,
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

// ── Style ─────────────────────────────────────────────────────────────────────

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

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpinnerSpecBuilder {
    pub rect: Option<Rect>,
    pub large: Option<bool>,
    pub style: Option<SpinnerStyle>,
}

impl SpinnerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = Some(large);
        self
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = Some(style);
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
            self.style = Some(theme.spinner_style());
        }
        self
    }

    pub fn build(self) -> raw::SpinnerSpec {
        raw::SpinnerSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            large: self.large.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level spinner widget function using WidgetContext.
///
/// This function accepts a SpinnerSpec and calls the low-level raw::spinner function.
pub fn spinner<T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SpinnerSpecBuilder,
    layout_params: S::Params,
) -> SpinnerResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::spinner(spec);
    ctx.append_cmds(result.draw);
    SpinnerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SpinnerSpec;
    use super::*;

    #[test]
    fn test_spinner_visual_normal() {
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            large: false,
            style: crate::theme::Theme::framewise().spinner_style(),
        };
        let style = spec.style;
        let res = raw::spinner(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                // Top-left
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 5.0),
                    p1: Vec2::new(0.0, 0.0),
                    color: style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 0.0),
                    p1: Vec2::new(5.0, 0.0),
                    color: style.color,
                    width: style.width
                },
                // Top-right
                DrawCmd::StrokeLine {
                    p0: Vec2::new(11.0, 0.0),
                    p1: Vec2::new(16.0, 0.0),
                    color: style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(16.0, 0.0),
                    p1: Vec2::new(16.0, 5.0),
                    color: style.color,
                    width: style.width
                },
                // Bottom-right
                DrawCmd::StrokeLine {
                    p0: Vec2::new(16.0, 11.0),
                    p1: Vec2::new(16.0, 16.0),
                    color: style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(16.0, 16.0),
                    p1: Vec2::new(11.0, 16.0),
                    color: style.color,
                    width: style.width
                },
                // Bottom-left
                DrawCmd::StrokeLine {
                    p0: Vec2::new(5.0, 16.0),
                    p1: Vec2::new(0.0, 16.0),
                    color: style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 16.0),
                    p1: Vec2::new(0.0, 11.0),
                    color: style.color,
                    width: style.width
                },
                // Highlight
                DrawCmd::StrokeLine {
                    p0: Vec2::new(1.6, 0.0),
                    p1: Vec2::new(8.0, 0.0),
                    color: style.highlight,
                    width: style.width
                },
            ])
        );
    }

    #[test]
    fn test_spinner_visual_large() {
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 24.0, 24.0),
            large: true,
            style: crate::theme::Theme::framewise().spinner_style(),
        };
        let style = spec.style;
        let res = raw::spinner(spec.clone());

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                // Top-left
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 7.0),
                    p1: Vec2::new(0.0, 0.0),
                    color: spec.style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 0.0),
                    p1: Vec2::new(7.0, 0.0),
                    color: spec.style.color,
                    width: style.width
                },
                // Top-right
                DrawCmd::StrokeLine {
                    p0: Vec2::new(17.0, 0.0),
                    p1: Vec2::new(24.0, 0.0),
                    color: spec.style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(24.0, 0.0),
                    p1: Vec2::new(24.0, 7.0),
                    color: spec.style.color,
                    width: style.width
                },
                // Bottom-right
                DrawCmd::StrokeLine {
                    p0: Vec2::new(24.0, 17.0),
                    p1: Vec2::new(24.0, 24.0),
                    color: spec.style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(24.0, 24.0),
                    p1: Vec2::new(17.0, 24.0),
                    color: spec.style.color,
                    width: style.width
                },
                // Bottom-left
                DrawCmd::StrokeLine {
                    p0: Vec2::new(7.0, 24.0),
                    p1: Vec2::new(0.0, 24.0),
                    color: spec.style.color,
                    width: style.width
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 24.0),
                    p1: Vec2::new(0.0, 17.0),
                    color: spec.style.color,
                    width: style.width
                },
                // Highlight
                DrawCmd::StrokeLine {
                    p0: Vec2::new(2.4, 0.0),
                    p1: Vec2::new(12.0, 0.0),
                    color: style.highlight,
                    width: style.width
                },
            ])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = SpinnerSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.spinner_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.spinner_style();
        custom_style.width = 99.0;
        let builder = SpinnerSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().width, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
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
        super::spinner(
            &mut ctx,
            SpinnerSpecBuilder::new().rect(custom_rect),
            layout_rect,
        );
        // First draw command is StrokeLine with p0 at (x, y+arm) and p1 at (x, y)
        // where x = custom_rect.x, y = custom_rect.y
        match &cmds[0] {
            crate::draw::DrawCmd::StrokeLine { p1, .. } => {
                assert_eq!(p1.x, custom_rect.x);
                assert_eq!(p1.y, custom_rect.y);
            }
            other => panic!("Expected StrokeLine, got {:?}", other),
        }
    }
}
