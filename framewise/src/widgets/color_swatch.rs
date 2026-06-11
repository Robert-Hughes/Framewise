use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Layer, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub color: Color,
        /// Border color drawn around the swatch. Transparent by default.
        pub border: Color,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchCalcIntrinsicSizeSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchResult {
        pub content_bounds: Rect,
    }

    /// Measure a color swatch's intrinsic size from its spec.
    ///
    /// A color swatch has no inherent preferred size. This returns
    /// [`IntrinsicSize::UNKNOWN`].
    ///
    pub fn calc_color_swatch_intrinsic_size(
        spec: &ColorSwatchCalcIntrinsicSizeSpec,
    ) -> crate::layout::IntrinsicSize {
        let _ = spec;
        crate::layout::IntrinsicSize::UNKNOWN
    }

    /// Low-level color swatch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn color_swatch(spec: ColorSwatchSpec, cmds: &mut DrawCommands) -> ColorSwatchResult {
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: spec.rect,
            color: spec.color,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: spec.rect,
            color: spec.border,
            width: 1.0,
            z: spec.layer.get_z(),
        });
        ColorSwatchResult {
            content_bounds: spec.rect.inset(1.0),
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatchResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ColorSwatchSpec {
    pub color: Color,
    pub border: Color,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorSwatchSpecBuilder {
    pub color: Option<Color>,
    pub border: Option<Color>,
}

impl ColorSwatchSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }

    pub fn border(mut self, border: Color) -> Self {
        self.border = Some(border);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.border.is_none() {
            self.border = Some(theme.ink);
        }
        // Note color doesn't come from theme - this is the colour being indicated by the swatch!
        self
    }

    pub fn build(self) -> ColorSwatchSpec {
        ColorSwatchSpec {
            color: self
                .color
                .expect("color not set — call .color() or defaults_from_theme()"),
            border: self
                .border
                .expect("border not set — call .border() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level color swatch widget function using WidgetContext.
///
/// This function accepts a ColorSwatchSpecBuilder and calls the low-level raw::color_swatch function.
pub fn color_swatch<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ColorSwatchSpecBuilder,
    layout_params: S::Params,
) -> ColorSwatchResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::ColorSwatchCalcIntrinsicSizeSpec {};
    let intrinsic = raw::calc_color_swatch_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::ColorSwatchSpec {
        layer: ctx.layer,
        rect,
        color: spec.color,
        border: spec.border,
    };
    let result = raw::color_swatch(raw_spec, ctx.cmds);
    ColorSwatchResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::ColorSwatchSpec;
    use super::*;
    use crate::focus::FocusSystem;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_color_swatch_visual_normal() {
        let spec = ColorSwatchSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 16.0, 16.0),
            color: Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0),
            border: Color::linear_rgba(0.0, 0.0, 0.0, 0.20),
        };
        let mut cmds = DrawCommands::new();
        let res = raw::color_swatch(spec, &mut cmds);
        let default_color = Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0);
        let default_border = Color::linear_rgba(0.0, 0.0, 0.0, 0.20);

        assert_eq!(
            res.content_bounds,
            Rect::new(0.0, 0.0, 16.0, 16.0).inset(1.0)
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                    color: default_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 16.0, 16.0),
                    color: default_border,
                    width: 1.0,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_color_swatch_visual_custom() {
        let custom_color = Color::from_srgb_f32(1.0, 0.0, 0.0, 1.0);
        let custom_border = Color::from_srgb_f32(0.0, 1.0, 0.0, 1.0);
        let spec = ColorSwatchSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
            color: custom_color,
            border: custom_border,
        };
        let mut cmds = DrawCommands::new();
        let res = raw::color_swatch(spec, &mut cmds);

        assert_eq!(
            res.content_bounds,
            Rect::new(0.0, 0.0, 20.0, 20.0).inset(1.0)
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    color: custom_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 20.0, 20.0),
                    color: custom_border,
                    width: 1.0,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let result = super::color_swatch(
            &mut ctx,
            ColorSwatchSpecBuilder::new().color(Color::from_srgb_u8(0, 0, 0, 0)),
            placement,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_color_swatch_bounds_and_content_bounds() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let result = super::color_swatch(
            &mut ctx,
            ColorSwatchSpecBuilder::new().color(Color::from_srgb_u8(0, 0, 0, 0)),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, layout_rect);
        assert_eq!(result.layout.content_bounds, layout_rect.inset(1.0));
    }
}
