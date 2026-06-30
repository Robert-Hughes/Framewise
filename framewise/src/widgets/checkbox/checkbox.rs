#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer, SizeRequest},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxSpec {
        /// Top-left of the 14x14 box.
        pub rect: Rect,
        pub disabled: bool,
        pub allowed_checked_states: Vec<CheckedState>,
        pub style: super::CheckboxStyle,
        pub clip_rect: ClipRect,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxPreLayoutSpec {
        pub style: super::CheckboxStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this checkbox would request under `offer`.
    ///
    /// The current implementation ignores `offer` because this widget's request
    /// is fixed by its style.
    pub fn pre_layout_checkbox(
        spec: &CheckboxPreLayoutSpec,
        offer: SizeOffer,
    ) -> CheckboxPreLayoutResult {
        CheckboxPreLayoutResult {
            size_request: checkbox_size_request(spec, offer),
        }
    }

    fn checkbox_size_request(spec: &CheckboxPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        SizeRequest::preferred(Vec2::new(spec.style.size, spec.style.size))
    }

    fn next_allowed_checked_state(
        current: CheckedState,
        allowed_checked_states: &[CheckedState],
        advance: bool,
    ) -> CheckedState {
        assert!(
            !allowed_checked_states.is_empty(),
            "CheckboxSpec::allowed_checked_states must not be empty"
        );

        let Some(index) = allowed_checked_states
            .iter()
            .position(|state| *state == current)
        else {
            return allowed_checked_states[0];
        };

        if advance {
            allowed_checked_states[(index + 1) % allowed_checked_states.len()]
        } else {
            current
        }
    }

    /// Low-level checkbox widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_checkbox(
        spec: CheckboxSpec,
        _pre_layout: CheckboxPreLayoutResult,
        state: &mut CheckboxState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> CheckboxResult {
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

        state.checked = next_allowed_checked_state(
            state.checked,
            &spec.allowed_checked_states,
            input_info.clicked,
        );

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(
            spec.rect.x,
            spec.rect.y + (spec.rect.h - s.size) * 0.5,
            s.size,
            s.size,
        );

        // Focus ring (outset 2px).
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

        // Box fill.
        let fill = match state.checked {
            CheckedState::Unchecked => crate::widgets::widget_helpers::interaction_color(
                s.background,
                s.hovered,
                s.pressed,
                input_info.hovered,
                input_info.pressed,
            ),
            _ => crate::widgets::widget_helpers::interaction_color(
                s.selected_fill,
                s.selected_hovered,
                s.selected_pressed,
                input_info.hovered,
                input_info.pressed,
            ),
        };
        cmds.push_crisp_fill_rect(r, tint(fill), spec.layer.get_z());

        // Box border.
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_crisp_border_rect(
            r,
            s.border.map(tint_stroke),
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        // Inner mark.
        match state.checked {
            CheckedState::Checked => {
                // Checkmark: two lines forming a tick (v).
                let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
                let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
                let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
                let mark = tint_stroke(s.mark);
                cmds.push_stroke_line(p0, p1, Some(mark), spec.layer.get_z());
                cmds.push_stroke_line(p1, p2, Some(mark), spec.layer.get_z());
            }
            CheckedState::Indeterminate => {
                // Horizontal dash.
                cmds.push_crisp_fill_rect(
                    Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    tint(s.mark.color),
                    spec.layer.get_z(),
                );
            }
            CheckedState::Unchecked => {}
        }

        CheckboxResult {
            input: input_info,
            focused,
            content_bounds: r.inset(s.border.map_or(0.0, |st| st.width)),
            cursor_icon: interaction.cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckboxStyle {
    pub size: f32,
    pub background: Color,
    pub hovered: Color,
    pub pressed: Color,
    pub selected_fill: Color,
    pub selected_hovered: Color,
    pub selected_pressed: Color,
    pub border: Option<Stroke>,
    pub mark: Stroke,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

impl CheckboxStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            size: 14.0,
            background: theme.paper_elev,
            hovered: theme.paper_elev_hover,
            pressed: theme.paper_elev_press,
            selected_fill: theme.ink,
            selected_hovered: Color::BLACK,
            selected_pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: Some(Stroke::new(theme.ink, 1.0)),
            mark: Stroke::new(theme.paper, 1.5),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.35,
        }
    }
}

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CheckedState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckboxState {
    pub checked: CheckedState,
    /// True if the mouse was pressed while hovering this checkbox, until the mouse is released.
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxSpec {
    pub disabled: bool,
    pub allowed_checked_states: Vec<CheckedState>,
    pub style: CheckboxStyle,
}

impl Default for CheckboxSpec {
    fn default() -> Self {
        Self {
            disabled: false,
            allowed_checked_states: vec![CheckedState::Unchecked, CheckedState::Checked],
            style: CheckboxStyle::default(),
        }
    }
}

impl CheckboxSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = CheckboxStyle::from_theme(theme);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn allowed_checked_states(mut self, allowed_checked_states: Vec<CheckedState>) -> Self {
        self.allowed_checked_states = allowed_checked_states;
        self
    }

    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level checkbox widget function using `WidgetContext`.
///
/// Consumes a complete `CheckboxSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn checkbox<T: TextBackend, S: LayoutState, CF>(
    spec: CheckboxSpec,
    layout_params: S::Params,
    state: &mut CheckboxState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> CheckboxResult {
    let pre_layout_spec = raw::CheckboxPreLayoutSpec { style: spec.style };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_checkbox(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::CheckboxSpec {
        rect,
        disabled: spec.disabled,
        allowed_checked_states: spec.allowed_checked_states,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::post_layout_checkbox(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    CheckboxResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

/// High-level labelled checkbox widget function using WidgetContext.
///
/// This draws a checkbox along with a label by its side. Clicking the label
/// behaves identically to clicking the checkbox, and all mouse interactions
/// (hover, pressed, click-and-drag) span the combined bounds.
///
/// Consumes a complete `CheckboxSpec`.
pub fn labelled_checkbox<T: TextBackend, S: LayoutState, CF>(
    spec: CheckboxSpec,
    label_text: &str,
    layout_params: S::Params,
    state: &mut CheckboxState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> CheckboxResult {
    // Resolve label style and measure text size
    let mut label_style = crate::widgets::label::LabelStyle::from_theme(&ctx.theme);
    label_style.content_placement = crate::text::TextContentPlacement::logical(
        crate::text::ContentPlacement::Align(crate::Align::Start),
        crate::text::ContentPlacement::Align(crate::Align::Center),
    );
    if spec.disabled {
        let alpha = spec.style.disabled_alpha;
        label_style.text_color = Color::linear_rgba(
            label_style.text_color.r,
            label_style.text_color.g,
            label_style.text_color.b,
            label_style.text_color.a * alpha,
        );
    }

    // Query size requests using the official functions of both widgets.
    let offer = ctx.peek_offer(layout_params.clone());
    let checkbox_pre_layout_spec = raw::CheckboxPreLayoutSpec { style: spec.style };
    let checkbox_pre_layout = raw::pre_layout_checkbox(&checkbox_pre_layout_spec, offer);
    let checkbox_size = checkbox_pre_layout.size_request.preferred.unwrap();

    let label_pre_layout_spec = crate::widgets::label::raw::LabelPreLayoutSpec {
        text: label_text,
        style: label_style,
    };
    let label_pre_layout = crate::widgets::label::raw::pre_layout_label(
        &label_pre_layout_spec,
        offer,
        ctx.text_backend,
    );
    let label_size = label_pre_layout.size_request.preferred.unwrap();

    let gap = 8.0;
    let combined_width = checkbox_size.x + gap + label_size.x;
    let combined_height = f32::max(checkbox_size.y, label_size.y);
    let size_request = SizeRequest::preferred(Vec2::new(combined_width, combined_height));

    // Resolve combined bounds
    let rect = ctx.layout(layout_params, size_request);

    // Run underlying checkbox interaction and draw control box
    let raw_spec = raw::CheckboxSpec {
        rect, // Pass the combined bounds for unified interaction handling
        disabled: spec.disabled,
        allowed_checked_states: spec.allowed_checked_states,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::post_layout_checkbox(
        raw_spec,
        checkbox_pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    // Draw the label text to the right of the control box
    let label_rect = Rect::new(
        rect.x + checkbox_size.x + gap,
        rect.y,
        rect.w - checkbox_size.x - gap,
        rect.h,
    );
    let raw_label_spec = crate::widgets::label::raw::LabelSpec {
        layer: ctx.layer,
        rect: label_rect,
        text: label_text,
        style: label_style,
    };
    crate::widgets::label::raw::post_layout_label(
        raw_label_spec,
        label_pre_layout,
        ctx.text_backend,
        ctx.cmds,
    );

    CheckboxResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "checkbox_tests.rs"]
mod tests;
