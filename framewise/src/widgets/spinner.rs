use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeRequest},
    text::TextBackend,
    types::{Color, Layer, Rect, Vec2},
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
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerCalcSizeRequestSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerResult {}

    /// Compute intrinsic size for Spinner. Currently returns UNKNOWN.
    pub fn calc_spinner_intrinsic_size(_spec: &SpinnerCalcSizeRequestSpec) -> SizeRequest {
        SizeRequest::UNKNOWN
    }

    /// Low-level spinner widget function.
    ///
    /// Appends draw commands to `cmds`.
    /// Square reticle spinner — four corner brackets with a single animated segment.
    /// Since we can't animate, we draw it at a fixed phase (segment at top).
    pub fn spinner(spec: SpinnerSpec, cmds: &mut DrawCommands) {
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
            anti_alias: false,
            p0: Vec2::new(x, y + arm),
            p1: Vec2::new(x, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x, y),
            p1: Vec2::new(x + arm, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Top-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size - arm, y),
            p1: Vec2::new(x + size, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y),
            p1: Vec2::new(x + size, y + arm),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Bottom-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y + size - arm),
            p1: Vec2::new(x + size, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y + size),
            p1: Vec2::new(x + size - arm, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Bottom-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + arm, y + size),
            p1: Vec2::new(x, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x, y + size),
            p1: Vec2::new(x, y + size - arm),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });

        // Animated segment on the top edge — drawn as a rust highlight.
        let seg_w = size * spec.style.highlight_fraction;
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size * 0.1, y),
            p1: Vec2::new(x + size * 0.1 + seg_w, y),
            color: spec.style.highlight,
            width: w,
            z: spec.layer.get_z(),
        });
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

impl SpinnerStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            color: theme.ink,
            highlight: theme.rust,
            small_size: 16.0,
            large_size: 24.0,
            small_arm: 5.0,
            large_arm: 7.0,
            width: 1.5,
            highlight_fraction: 0.4,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerSpec {
    pub large: bool,
    pub style: SpinnerStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpinnerSpecBuilder {
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SpinnerStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SpinnerSpec {
        SpinnerSpec {
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
/// This function accepts a SpinnerSpecBuilder and calls the low-level raw::spinner function.
pub fn spinner<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SpinnerSpecBuilder,
    layout_params: S::Params,
) -> SpinnerResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::SpinnerCalcSizeRequestSpec {};
    let intrinsic = raw::calc_spinner_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::SpinnerSpec {
        rect,
        large: spec.large,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::spinner(raw_spec, ctx.cmds);
    SpinnerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SpinnerSpec;
    use super::*;
    use crate::focus::FocusSystem;

    #[test]
    fn test_spinner_visual_normal() {
        let style = SpinnerStyle::from_theme(&crate::theme::Theme::framewise());
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            large: false,
            style,
            layer: Layer::default(),
        };
        let mut cmds = DrawCommands::new();
        raw::spinner(spec, &mut cmds);

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                // Top-left
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 5.0),
                    p1: Vec2::new(0.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 0.0),
                    p1: Vec2::new(5.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Top-right
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(11.0, 0.0),
                    p1: Vec2::new(16.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(16.0, 0.0),
                    p1: Vec2::new(16.0, 5.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Bottom-right
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(16.0, 11.0),
                    p1: Vec2::new(16.0, 16.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(16.0, 16.0),
                    p1: Vec2::new(11.0, 16.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Bottom-left
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(5.0, 16.0),
                    p1: Vec2::new(0.0, 16.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 16.0),
                    p1: Vec2::new(0.0, 11.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Highlight
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(1.6, 0.0),
                    p1: Vec2::new(8.0, 0.0),
                    color: style.highlight,
                    width: style.width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_spinner_visual_large() {
        let style = SpinnerStyle::from_theme(&crate::theme::Theme::framewise());
        let spec = SpinnerSpec {
            rect: Rect::new(0.0, 0.0, 24.0, 24.0),
            large: true,
            style,
            layer: Layer::default(),
        };
        let mut cmds = DrawCommands::new();
        raw::spinner(spec, &mut cmds);

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                // Top-left
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 7.0),
                    p1: Vec2::new(0.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 0.0),
                    p1: Vec2::new(7.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Top-right
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(17.0, 0.0),
                    p1: Vec2::new(24.0, 0.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(24.0, 0.0),
                    p1: Vec2::new(24.0, 7.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Bottom-right
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(24.0, 17.0),
                    p1: Vec2::new(24.0, 24.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(24.0, 24.0),
                    p1: Vec2::new(17.0, 24.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Bottom-left
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(7.0, 24.0),
                    p1: Vec2::new(0.0, 24.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 24.0),
                    p1: Vec2::new(0.0, 17.0),
                    color: style.color,
                    width: style.width,
                    z: 0,
                },
                // Highlight
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(2.4, 0.0),
                    p1: Vec2::new(12.0, 0.0),
                    color: style.highlight,
                    width: style.width,
                    z: 0,
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
        assert_eq!(builder.style, Some(SpinnerStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = SpinnerStyle::from_theme(&theme);
        custom_style.width = 99.0;
        let builder = SpinnerSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().width, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
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
        let result = super::spinner(&mut ctx, SpinnerSpecBuilder::new(), placement);
        assert_eq!(result.layout.bounds, placement);
    }
}
