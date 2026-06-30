use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioSpec {
        pub layer: Layer,
        /// Top-left of the 14x14 bounding area.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::RadioStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioPreLayoutSpec {
        pub style: super::RadioStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this radio button would request under `offer`.
    ///
    /// The current implementation ignores `offer` because this widget's request
    /// is fixed by its style.
    pub fn pre_layout_radio(spec: &RadioPreLayoutSpec, offer: SizeOffer) -> RadioPreLayoutResult {
        RadioPreLayoutResult {
            size_request: radio_size_request(spec, offer),
        }
    }

    fn radio_size_request(spec: &RadioPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        SizeRequest::preferred(Vec2::new(spec.style.radius * 2.0, spec.style.radius * 2.0))
    }

    /// Low-level radio widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_radio(
        spec: RadioSpec,
        _pre_layout: RadioPreLayoutResult,
        state: &mut RadioState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> RadioResult {
        let interaction = crate::widgets::widget_helpers::handle_press_interaction(
            crate::widgets::widget_helpers::PressInteractionSpec {
                focus_id: state.focus_id,
                rect: spec.rect,
                clip_rect: spec.clip_rect,
                disabled: spec.disabled,
                traversal_keys: crate::focus::FocusTraversalKeys::all(),
                hover_cursor_icon: Some(crate::output::CursorIcon::Pointer),
            },
            input,
            focus_system,
            &mut state.is_active,
            &mut state.space_is_active,
        );
        let focused = interaction.focused;
        let input_info = interaction.input;

        if input_info.clicked {
            state.checked = true;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(
            spec.rect.x,
            spec.rect.y + (spec.rect.h - s.radius * 2.0) * 0.5,
            s.radius * 2.0,
            s.radius * 2.0,
        );

        let cx = r.x + s.radius;
        let cy = r.y + s.radius;
        let center = Vec2::new(cx, cy);

        // Focus ring (outset 2px).
        if focused {
            if let Some(outline) = s.focus {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_stroke_circle(
                    center,
                    s.radius + outline.offset + outline.stroke.width * 0.5,
                    Some(tint_stroke(outline.stroke)),
                    spec.layer.get_focus_z(),
                );
            }
        }

        // Background fill.
        let fill = if state.checked {
            crate::widgets::widget_helpers::interaction_color(
                s.background,
                s.selected_hovered,
                s.selected_pressed,
                input_info.hovered,
                input_info.pressed,
            )
        } else {
            crate::widgets::widget_helpers::interaction_color(
                s.background,
                s.hovered,
                s.pressed,
                input_info.hovered,
                input_info.pressed,
            )
        };
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: s.radius,
            color: tint(fill),
            z: spec.layer.get_z(),
        });

        // Outer ring.
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_stroke_circle(
            center,
            s.radius,
            s.border.map(tint_stroke),
            spec.layer.get_z(),
        );

        // Inner dot when selected.
        if state.checked {
            cmds.push(DrawCmd::FillCircle {
                center,
                radius: s.dot_radius,
                color: tint(s.dot),
                z: spec.layer.get_z(),
            });
        }

        RadioResult {
            input: input_info,
            focused,
            content_bounds: r.inset(s.border.map_or(0.0, |st| st.width)),
            cursor_icon: interaction.cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioStyle {
    pub radius: f32,
    pub dot_radius: f32,
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub selected_hovered: Color,
    pub selected_pressed: Color,
    pub border: Option<Stroke>,
    pub dot: Color,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

impl RadioStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            radius: 7.0,
            dot_radius: 3.0,
            background: theme.paper_elev,
            hovered: theme.paper_elev_hover,
            pressed: theme.paper_elev_press,
            selected_hovered: theme.paper_elev_hover,
            selected_pressed: theme.paper_elev_press,
            border: Some(Stroke::new(theme.ink, 1.5)),
            dot: theme.ink,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.35,
        }
    }
}

impl Default for RadioStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioState {
    pub checked: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RadioResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioSpec {
    pub disabled: bool,
    pub style: RadioStyle,
}

impl RadioSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = RadioStyle::from_theme(theme);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level radio widget function using `WidgetContext`.
///
/// Consumes a complete `RadioSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn radio<T: TextBackend, S: LayoutState, CF>(
    spec: RadioSpec,
    layout_params: S::Params,
    state: &mut RadioState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> RadioResult {
    let pre_layout_spec = raw::RadioPreLayoutSpec { style: spec.style };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_radio(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::RadioSpec {
        layer: ctx.layer,
        rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_radio(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    RadioResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

/// High-level labelled radio widget function using WidgetContext.
///
/// This draws a radio along with a label by its side. Clicking the label
/// behaves identically to clicking the radio, and all mouse interactions
/// (hover, pressed, click-and-drag) span the combined bounds.
///
/// Consumes a complete `RadioSpec`.
pub fn labelled_radio<T: TextBackend, S: LayoutState, CF>(
    spec: RadioSpec,
    label_text: &str,
    layout_params: S::Params,
    state: &mut RadioState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> RadioResult {
    let label_style = crate::widgets::widget_helpers::trailing_label_style_from_theme(
        &ctx.theme,
        spec.disabled,
        spec.style.disabled_alpha,
    );
    let trailing_style = crate::widgets::widget_helpers::TrailingLabelStyle {
        label_style,
        gap: 8.0,
    };

    let offer = ctx.peek_offer(layout_params.clone());
    let radio_pre_layout_spec = raw::RadioPreLayoutSpec { style: spec.style };
    let radio_pre_layout = raw::pre_layout_radio(&radio_pre_layout_spec, offer);
    let radio_size = radio_pre_layout.size_request.preferred.unwrap();

    let label_pre_layout_spec = crate::widgets::label::raw::LabelPreLayoutSpec {
        text: label_text,
        style: trailing_style.label_style,
    };
    let label_pre_layout = crate::widgets::label::raw::pre_layout_label(
        &label_pre_layout_spec,
        offer,
        ctx.text_backend,
    );
    let label_size = label_pre_layout.size_request.preferred.unwrap();

    let rect = ctx.layout(
        layout_params,
        crate::widgets::widget_helpers::trailing_label_size_request(
            radio_size,
            label_size,
            trailing_style.gap,
        ),
    );
    let layout = crate::widgets::widget_helpers::layout_trailing_label(
        rect,
        radio_size,
        label_size,
        trailing_style.gap,
    );

    let raw_spec = raw::RadioSpec {
        layer: ctx.layer,
        // Pass the combined bounds for unified interaction handling:
        // the label area should hover, press, focus, and click like the control itself.
        rect: layout.outer_rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_radio(
        raw_spec,
        radio_pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    crate::widgets::widget_helpers::draw_trailing_label(
        layout.label_rect,
        label_text,
        trailing_style.label_style,
        label_pre_layout,
        ctx.layer,
        ctx.text_backend,
        ctx.cmds,
    );

    RadioResult {
        layout: LayoutInfo::new(layout.outer_rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "radio_tests.rs"]
mod tests;
