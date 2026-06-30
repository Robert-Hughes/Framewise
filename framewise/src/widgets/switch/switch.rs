#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
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
    pub struct SwitchSpec {
        /// Top-left of the 30x16 bounding area.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::SwitchStyle,
        pub clip_rect: ClipRect,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchPreLayoutSpec {
        pub style: super::SwitchStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this switch would request under `offer`.
    ///
    /// The current implementation ignores `offer` because this widget's request
    /// is fixed by its style.
    pub fn pre_layout_switch(
        spec: &SwitchPreLayoutSpec,
        offer: SizeOffer,
    ) -> SwitchPreLayoutResult {
        SwitchPreLayoutResult {
            size_request: switch_size_request(spec, offer),
        }
    }

    fn switch_size_request(spec: &SwitchPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        SizeRequest::preferred(spec.style.size)
    }

    /// Low-level switch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_switch(
        spec: SwitchSpec,
        _pre_layout: SwitchPreLayoutResult,
        state: &mut SwitchState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> SwitchResult {
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
            state.checked = !state.checked;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(
            spec.rect.x,
            spec.rect.y + (spec.rect.h - s.size.y) * 0.5,
            s.size.x,
            s.size.y,
        );

        // Focus ring.
        if focused {
            if let Some(outline) = s.focus {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_crisp_border_rect(
                    r.inset(-outline.offset),
                    Some(tint_stroke(outline.stroke)),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        // Track fill.
        let track_fill = if state.checked {
            crate::widgets::widget_helpers::interaction_color(
                s.on_fill,
                s.selected_hovered,
                s.selected_pressed,
                input_info.hovered,
                input_info.pressed,
            )
        } else {
            crate::widgets::widget_helpers::interaction_color(
                s.off_fill,
                s.hovered,
                s.pressed,
                input_info.hovered,
                input_info.pressed,
            )
        };
        cmds.push_crisp_fill_rect(r, tint(track_fill), spec.layer.get_z());

        // Track border.
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_crisp_border_rect(
            r,
            s.border.map(tint_stroke),
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let border_width = s.border.map_or(0.0, |st| st.width);
        // Thumb dot (10x10, vertically centered, left/right positioned).
        let dot_y = r.y + (r.h - s.thumb_size) * 0.5;
        let dot_x = if state.checked {
            r.x + r.w - border_width - s.thumb_offset - s.thumb_size
        } else {
            r.x + border_width + s.thumb_offset
        };
        let dot_color = if state.checked {
            s.on_thumb
        } else {
            s.off_thumb
        };
        cmds.push_crisp_fill_rect(
            Rect::new(dot_x, dot_y, s.thumb_size, s.thumb_size),
            tint(dot_color),
            spec.layer.get_z(),
        );

        SwitchResult {
            input: input_info,
            focused,
            content_bounds: r.inset(border_width),
            cursor_icon: interaction.cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchStyle {
    pub size: Vec2,
    pub thumb_size: f32,
    pub off_fill: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub on_fill: Color,
    pub selected_hovered: Color,
    pub selected_pressed: Color,
    pub border: Option<Stroke>,
    pub off_thumb: Color,
    pub on_thumb: Color,
    pub focus: Option<Outline>,
    pub thumb_offset: f32,
    pub disabled_alpha: f32,
}

impl SwitchStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            size: Vec2::new(30.0, 16.0),
            thumb_size: 10.0,
            off_fill: theme.paper_elev,
            hovered: theme.paper_elev_hover,
            pressed: theme.paper_elev_press,
            on_fill: theme.ink,
            selected_hovered: Color::BLACK,
            selected_pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: Some(Stroke::new(theme.ink, 1.0)),
            off_thumb: theme.ink,
            on_thumb: theme.paper,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            thumb_offset: 1.0,
            disabled_alpha: 0.35,
        }
    }
}

impl Default for SwitchStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchState {
    pub checked: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchSpec {
    pub disabled: bool,
    pub style: SwitchStyle,
}

impl SwitchSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = SwitchStyle::from_theme(theme);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: SwitchStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level switch widget function using `WidgetContext`.
///
/// Consumes a complete `SwitchSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn switch<T: TextBackend, S: LayoutState, CF>(
    spec: SwitchSpec,
    layout_params: S::Params,
    state: &mut SwitchState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SwitchResult {
    let pre_layout_spec = raw::SwitchPreLayoutSpec { style: spec.style };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_switch(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SwitchSpec {
        rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::post_layout_switch(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    SwitchResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

/// High-level labelled switch widget function using WidgetContext.
///
/// This draws a switch along with a label by its side. Clicking the label
/// behaves identically to clicking the switch, and all mouse interactions
/// (hover, pressed, click-and-drag) span the combined bounds.
///
/// Consumes a complete `SwitchSpec`.
pub fn labelled_switch<T: TextBackend, S: LayoutState, CF>(
    spec: SwitchSpec,
    label_text: &str,
    layout_params: S::Params,
    state: &mut SwitchState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SwitchResult {
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
    let switch_pre_layout_spec = raw::SwitchPreLayoutSpec { style: spec.style };
    let switch_pre_layout = raw::pre_layout_switch(&switch_pre_layout_spec, offer);
    let switch_size = switch_pre_layout.size_request.preferred.unwrap();

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
            switch_size,
            label_size,
            trailing_style.gap,
        ),
    );
    let layout = crate::widgets::widget_helpers::layout_trailing_label(
        rect,
        switch_size,
        label_size,
        trailing_style.gap,
    );

    let raw_spec = raw::SwitchSpec {
        rect: layout.outer_rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::post_layout_switch(
        raw_spec,
        switch_pre_layout,
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

    SwitchResult {
        layout: LayoutInfo::new(layout.outer_rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "switch_tests.rs"]
mod tests;
