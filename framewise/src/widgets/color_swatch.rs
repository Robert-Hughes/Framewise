use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchSpec {
        pub rect: Rect,
        pub color: Color,
        /// Border color drawn around the swatch. Transparent by default.
        pub border: Color,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ColorSwatchResult {
        pub content_bounds: Rect,
    }

    /// Measure a color swatch's intrinsic size from its spec.
    ///
    /// A color swatch has no inherent preferred size. This returns
    /// [`IntrinsicSize::UNKNOWN`].
    ///
    /// **Must not read `spec.rect`** — this runs before the rect is known, so
    /// callers pass [`Rect::PLACEHOLDER`] (NaN).
    pub fn calc_color_swatch_intrinsic_size(
        spec: &ColorSwatchSpec,
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
            rect: spec.rect,
            color: spec.color,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: spec.border,
            width: 1.0,
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

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ColorSwatchSpecBuilder {
    pub rect: Option<Rect>,
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

    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.border.is_none() {
            self.border = Some(theme.ink);
        }
        // Note color doesn't come from theme - this is the colour being indicated by the swatch!
        self
    }

    pub fn build(self) -> raw::ColorSwatchSpec {
        raw::ColorSwatchSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
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
    let mut spec = builder
        .defaults_from_theme(&ctx.theme)
        .rect(Rect::PLACEHOLDER)
        .build();
    let intrinsic = raw::calc_color_swatch_intrinsic_size(&spec);
    let rect = ctx.layout(layout_params, intrinsic);
    spec.rect = rect;
    let result = raw::color_swatch(spec, ctx.cmds);
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
        let spec = ColorSwatchSpecBuilder::new()
            .rect(Rect::new(0.0, 0.0, 16.0, 16.0))
            .color(Color::from_srgb_f32(0.5, 0.5, 0.5, 1.0))
            .border(Color::linear_rgba(0.0, 0.0, 0.0, 0.20))
            .build();
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
