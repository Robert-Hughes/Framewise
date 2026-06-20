#[cfg(test)]
use crate::focus::FocusSystem;
use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub color: Color,
        pub width: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DividerResult {}

    /// Return the size this divider would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a divider has no
    /// inherent preferred size. This returns [`SizeRequest::UNKNOWN`].
    ///
    pub fn pre_layout_divider(
        spec: &DividerPreLayoutSpec,
        offer: SizeOffer,
    ) -> DividerPreLayoutResult {
        DividerPreLayoutResult {
            size_request: divider_size_request(spec, offer),
        }
    }

    fn divider_size_request(
        spec: &DividerPreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// Low-level divider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_divider(
        spec: DividerSpec,
        _pre_layout: DividerPreLayoutResult,
        cmds: &mut DrawCommands,
    ) -> DividerResult {
        let mid_y = spec.rect.y + spec.rect.h * 0.5;
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(spec.rect.x, mid_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, mid_y),
            color: spec.color,
            width: spec.width,
            z: 0,
        });
        DividerResult {}
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DividerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DividerSpec {
    pub color: Color,
    pub width: f32,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DividerSpecBuilder {
    pub color: Option<Color>,
    pub width: Option<f32>,
}

impl DividerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn color(mut self, color: Color) -> Self {
        self.color = Some(color);
        self
    }
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.color.is_none() {
            self.color = Some(theme.line);
        }
        self
    }
    pub fn build(self) -> DividerSpec {
        DividerSpec {
            color: self
                .color
                .expect("color not set — call .color() or defaults_from_theme()"),
            width: self.width.unwrap_or(1.0),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level divider widget function using `WidgetContext`.
///
/// Resolves defaults, queries the layout offer, asks the raw size helper for a
/// `SizeRequest`, resolves the final rect with `layout`, then calls the raw
/// widget.
pub fn divider<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: DividerSpecBuilder,
    layout_params: S::Params,
) -> DividerResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::DividerPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_divider(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::DividerSpec {
        layer: ctx.layer,
        rect,
        color: spec.color,
        width: spec.width,
    };
    let _result = raw::post_layout_divider(raw_spec, pre_layout, ctx.cmds);

    DividerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::DividerSpec;
    use super::*;
    use crate::test_utils::TestTextBackend;

    #[test]
    fn test_divider_visual() {
        let spec = DividerSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 100.0, 10.0),
            color: Color::WHITE,
            width: 1.0,
        };
        let mut cmds = DrawCommands::new();
        let _res = raw::post_layout_divider(
            spec,
            raw::DividerPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![DrawCmd::StrokeLine {
                anti_alias: false,
                p0: Vec2::new(0.0, 5.0),
                p1: Vec2::new(100.0, 5.0),
                color: Color::WHITE,
                width: 1.0,
                z: 0,
            }])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_color() {
        let theme = crate::theme::Theme::framewise();
        let builder = DividerSpecBuilder::new();
        assert!(builder.color.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.color, Some(theme.line));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_color() {
        let theme = crate::theme::Theme::framewise();
        let sentinel = Color::from_srgb_u8(1, 2, 3, 255);
        let builder = DividerSpecBuilder::new().color(sentinel);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.color, Some(sentinel));
    }

    #[test]
    fn test_size_divider_ignores_offer() {
        use crate::layout::AxisBound;

        let spec = raw::DividerPreLayoutSpec {};
        let offers = [
            SizeOffer::UNBOUNDED,
            SizeOffer::new(AxisBound::Exact(50.0), AxisBound::Exact(20.0)),
            SizeOffer::new(AxisBound::AtMost(100.0), AxisBound::AtMost(40.0)),
        ];

        let expected = raw::pre_layout_divider(&spec, offers[0]).size_request;
        for offer in offers {
            assert_eq!(raw::pre_layout_divider(&spec, offer).size_request, expected);
        }
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
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
        let result = super::divider(&mut ctx, DividerSpecBuilder::new(), placement);
        assert_eq!(result.layout.bounds, placement);
    }
}
