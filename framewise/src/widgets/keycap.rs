use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::LayoutState,
    text::{layout_text, TextBackend},
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::KeycapStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapCalcSizeRequestSpec<'a> {
        pub text: &'a str,
        pub style: super::KeycapStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct KeycapResult {
        pub content_bounds: Rect,
    }

    /// Calculate a keycap's size request from its size-request spec.
    pub fn calc_keycap_intrinsic_size<T: TextBackend>(
        spec: &KeycapCalcSizeRequestSpec,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        crate::layout::SizeRequest::preferred(layout.metrics().logical_size)
    }

    /// Low-level keycap widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn keycap<T: TextBackend>(
        spec: KeycapSpec<'_>,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> KeycapResult {
        // Background + border
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: spec.rect,
            color: spec.style.background,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: spec.rect,
            color: spec.style.border,
            width: spec.style.border_width,
            z: spec.layer.get_z(),
        });
        // Bottom shadow line
        let shadow_rect = Rect::new(
            spec.rect.x + spec.style.shadow_offset,
            spec.rect.y + spec.rect.h,
            spec.rect.w - spec.style.shadow_offset,
            spec.style.shadow_height,
        );
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: shadow_rect,
            color: spec.style.shadow,
            z: spec.layer.get_z(),
        });

        // text, centered
        if !spec.text.is_empty() {
            let layout = layout_text(
                text_backend,
                spec.text,
                spec.style.text_style,
                crate::text::TextBounds {
                    max_width: Some(spec.rect.w),
                    max_height: Some(spec.rect.h),
                },
            );
            let text_rect = spec
                .style
                .content_placement
                .resolve_rect(spec.rect, layout.metrics().clone());
            layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(text_rect.x, text_rect.y),
                spec.style.text_color,
                spec.layer.get_z(),
            );
        }

        KeycapResult {
            content_bounds: spec.rect.inset(spec.style.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a keycap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KeycapStyle {
    pub background: Color,
    pub shadow: Color,
    pub shadow_offset: f32,
    pub shadow_height: f32,
    pub border: Color,
    pub border_width: f32,
    pub text_color: Color,
    pub text_style: crate::text::TextStyle,
    pub content_placement: crate::text::TextContentPlacement,
}

impl KeycapStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            shadow: theme.line,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            border: theme.line,
            border_width: theme.border,
            text_color: theme.ink,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct KeycapSpec<'a> {
    pub text: &'a str,
    pub style: KeycapStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeycapSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<KeycapStyle>,
}

impl<'a> KeycapSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: KeycapStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(KeycapStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> KeycapSpec<'a> {
        KeycapSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level keycap widget function using WidgetContext.
///
/// This function accepts a KeycapSpecBuilder and calls the low-level raw::keycap function.
pub fn keycap<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: KeycapSpecBuilder<'a>,
    layout_params: S::Params,
) -> KeycapResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::KeycapCalcSizeRequestSpec {
        text: spec.text,
        style: spec.style,
    };
    let size_request = raw::calc_keycap_intrinsic_size(&calc_spec, ctx.text_backend);
    let rect = ctx.layout(layout_params, size_request);
    let raw_spec = raw::KeycapSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
    };
    let result = raw::keycap(raw_spec, ctx.text_backend, ctx.cmds);
    KeycapResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::KeycapSpec;
    use super::*;
    use crate::{
        focus::FocusSystem, test_utils::TestTextBackend, text::FontId, DrawGlyph,
        PreparedGlyphToken, Vec2,
    };

    #[test]
    fn test_keycap_visual() {
        let mut text_backend = TestTextBackend;
        let custom_bg = Color::from_srgb_u8(240, 240, 240, 255);
        let custom_shadow = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_border = Color::from_srgb_u8(10, 10, 10, 255);
        let custom_text = Color::from_srgb_u8(50, 50, 50, 255);
        let spec = KeycapSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 30.0, 30.0),
            text: "K",
            style: KeycapStyle {
                background: custom_bg,
                border: custom_border,
                border_width: 1.0,
                shadow: custom_shadow,
                shadow_offset: 1.0,
                shadow_height: 2.0,
                text_color: custom_text,
                text_style: crate::text::TextStyle::new(
                    FontId(0),
                    14.0,
                    400,
                    crate::text::TextFlow::single_line(),
                ),
                content_placement: crate::text::TextContentPlacement::CENTER,
            },
        };
        let mut cmds = DrawCommands::new();
        let res = raw::keycap(spec, &mut text_backend, &mut cmds);

        assert_eq!(
            res.content_bounds,
            Rect::new(0.0, 0.0, 30.0, 30.0).inset(1.0)
        );
        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                    color: custom_bg,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 30.0, 30.0),
                    color: custom_border,
                    width: 1.0,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(1.0, 30.0, 29.0, 2.0),
                    color: custom_shadow,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..1,
                    color: custom_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![DrawGlyph {
                token: PreparedGlyphToken(75),
                top_left: Vec2 { x: 11.0, y: 21.0 }
            }]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = KeycapSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.font, theme.mono_font);
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let explicit_style = KeycapStyle {
            background: Color::WHITE,
            shadow: Color::BLACK,
            shadow_offset: 1.0,
            shadow_height: 2.0,
            border: Color::WHITE,
            border_width: 1.0,
            text_color: Color::WHITE,
            text_style: crate::text::TextStyle::new(
                FontId(99),
                14.0,
                400,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
        };
        let builder = KeycapSpecBuilder::new().style(explicit_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(explicit_style));
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
        let result = super::keycap(&mut ctx, KeycapSpecBuilder::new().text("X"), placement);
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_keycap_bounds_and_content_bounds() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let custom_border_width = 3.5;
        let result = super::keycap(
            &mut ctx,
            KeycapSpecBuilder::new().text("X").style(KeycapStyle {
                background: Color::WHITE,
                shadow: Color::BLACK,
                shadow_offset: 1.0,
                shadow_height: 2.0,
                border: Color::WHITE,
                border_width: custom_border_width,
                text_color: Color::WHITE,
                text_style: crate::text::TextStyle::new(
                    FontId(0),
                    14.0,
                    400,
                    crate::text::TextFlow::single_line(),
                ),
                content_placement: crate::text::TextContentPlacement::CENTER,
            }),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, layout_rect);
        assert_eq!(
            result.layout.content_bounds,
            layout_rect.inset(custom_border_width)
        );
    }
}
