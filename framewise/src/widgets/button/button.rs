use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::layout_text;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonSpec<'a> {
        pub layer: Layer,
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ButtonStyle,
        pub clip_rect: ClipRect,
        pub disabled: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::ButtonStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ButtonResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Return the size this button would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    ///
    /// The preferred width is the label width plus horizontal padding; the
    /// preferred height is the larger of the standard control height and the
    /// padded label height.
    pub fn pre_layout_button<T: TextBackend>(
        spec: &ButtonPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> ButtonPreLayoutResult {
        ButtonPreLayoutResult {
            size_request: button_size_request(spec, offer, text_backend),
        }
    }

    fn button_size_request<T: TextBackend>(
        spec: &ButtonPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let style = &spec.style;
        let layout = layout_text(
            text_backend,
            spec.text,
            style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let t = layout.metrics();
        let w = t.logical_size.x + 2.0 * style.pad_x;
        let h = (t.logical_size.y + 2.0 * style.pad_y).max(style.min_height);
        crate::layout::SizeRequest::preferred(crate::types::Vec2::new(w, h))
    }

    /// Shape the label inside the button content rect and emit it.
    fn emit_placed_text<T: TextBackend>(
        text: &str,
        style: &super::ButtonStyle,
        rect: Rect,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
        color: Color,
        z: u32,
    ) -> Rect {
        let content_rect = Rect::new(
            rect.x + style.pad_x,
            rect.y + style.pad_y,
            (rect.w - 2.0 * style.pad_x).max(0.0),
            (rect.h - 2.0 * style.pad_y).max(0.0),
        );
        let layout = layout_text(
            text_backend,
            text,
            style.text_style,
            crate::text::TextBounds {
                max_width: Some(content_rect.w),
                max_height: Some(content_rect.h),
            },
        );
        let text_rect = style
            .content_placement
            .resolve_rect(content_rect, layout.metrics().clone());
        layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            color,
            z,
        );
        text_rect
    }

    /// Low-level button widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_button<T: TextBackend>(
        spec: ButtonSpec,
        _pre_layout: ButtonPreLayoutResult,
        state: &mut ButtonState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> ButtonResult {
        // Disabled: register_keyboard for layout but skip all interaction.
        if spec.disabled {
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            cmds.push(DrawCmd::FillRect {
                rect: spec.rect,
                color: tint(spec.style.background),
                z: spec.layer.get_z(),
            });
            let tint_stroke = |s: Stroke| Stroke::new(tint(s.color), s.width);
            cmds.push_border_rect(
                spec.rect,
                spec.style.border.map(tint_stroke),
                BorderPlacement::Inside,
                spec.layer.get_z(),
            );
            emit_placed_text(
                spec.text,
                &spec.style,
                spec.rect,
                text_backend,
                cmds,
                tint(spec.style.text_color),
                spec.layer.get_z(),
            );
            return ButtonResult {
                content_bounds: spec.rect.inset(spec.style.border.map_or(0.0, |s| s.width)),
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                focused: false,
            };
        }

        let interaction = crate::widgets::widget_helpers::handle_press_interaction(
            crate::widgets::widget_helpers::PressInteractionSpec {
                focus_id: state.focus_id,
                rect: spec.rect,
                clip_rect: spec.clip_rect,
                disabled: false,
                traversal_keys: crate::focus::FocusTraversalKeys::all(),
            },
            input,
            focus_system,
            &mut state.is_active,
            &mut state.space_is_active,
        );
        let focused = interaction.focused;
        let input_info = interaction.input;

        // Choose fill colour based on interaction state.
        let fill = crate::widgets::widget_helpers::interaction_color(
            spec.style.background,
            spec.style.hovered,
            spec.style.pressed,
            input_info.hovered,
            input_info.pressed,
        );

        // CSS outline sits outside the border box. BorderRect draws outside the
        // rect (using BorderPlacement::Outside), so expand by the desired gap.
        if focused {
            if let Some(outline) = spec.style.focus {
                cmds.push_border_rect(
                    spec.rect.inset(-outline.offset),
                    Some(outline.stroke),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        // Background fill.
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: fill,
            z: spec.layer.get_z(),
        });

        // Border.
        cmds.push_border_rect(
            spec.rect,
            spec.style.border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        // Text centered.
        emit_placed_text(
            spec.text,
            &spec.style,
            spec.rect,
            text_backend,
            cmds,
            spec.style.text_color,
            spec.layer.get_z(),
        );

        ButtonResult {
            content_bounds: spec.rect.inset(spec.style.border.map_or(0.0, |s| s.width)),
            input: input_info,
            focused,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a button.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonStyle {
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub border: Option<Stroke>,
    pub focus: Option<Outline>,
    pub text_style: crate::text::TextStyle,
    /// Placement of the prepared text block inside the padded button content rect.
    pub content_placement: crate::text::TextContentPlacement,
    pub text_color: Color,
    pub disabled_alpha: f32,
    /// Horizontal padding each side of the label, used for the preferred width request.
    pub pad_x: f32,
    /// Vertical padding above/below the label, used for the preferred height request.
    pub pad_y: f32,
    /// Minimum requested height (the standard control height); the preferred
    /// height is the larger of this and the padded text height.
    pub min_height: f32,
}

impl ButtonStyle {
    pub fn secondary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.paper_hover,
            pressed: theme.paper_press,
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn primary_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.ink,
            hovered: Color::BLACK,
            pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.paper,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn accent_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.rust,
            hovered: Color::from_srgb_u8(176, 79, 35, 255),
            pressed: Color::from_srgb_u8(156, 69, 32, 255),
            border: Some(Stroke::new(theme.rust, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: Color::WHITE,
            disabled_alpha: 0.32f32,
            pad_x: 14.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }

    pub fn ghost_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: Color::TRANSPARENT,
            hovered: theme.paper_hover,
            pressed: theme.paper_press,
            border: None,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                500,
                crate::text::TextFlow::single_line(),
            ),
            content_placement: crate::text::TextContentPlacement::CENTER,
            text_color: theme.ink,
            disabled_alpha: 0.32f32,
            pad_x: 10.0,
            pad_y: 6.0,
            min_height: theme.h_md,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ButtonState {
    /// True if the mouse was pressed while hovering this button, until the mouse is released.
    pub is_active: bool,
    /// True if the spacebar was pressed while this button was focused, until space or focus is lost.
    pub space_is_active: bool,
    /// Globally unique ID for tracking keyboard focus.
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ButtonSpec<'a> {
    pub text: &'a str,
    pub style: ButtonStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ButtonSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<ButtonStyle>,
    pub disabled: Option<bool>,
}

impl<'a> ButtonSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(ButtonStyle::secondary_from_theme(theme));
        }
        self
    }
    pub fn build(self) -> ButtonSpec<'a> {
        ButtonSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level button widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn button<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::ButtonPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_button(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ButtonSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        disabled: spec.disabled,
    };

    let r = raw::post_layout_button(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}

#[cfg(test)]
#[path = "button_tests.rs"]
mod tests;
