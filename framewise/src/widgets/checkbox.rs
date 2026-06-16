use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{IntrinsicSize, LayoutState},
    types::{ClipRect, Color, Layer, Rect, Vec2},
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
    pub struct CheckboxCalcIntrinsicSizeSpec {
        pub style: super::CheckboxStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Compute intrinsic size for Checkbox.
    pub fn calc_checkbox_intrinsic_size(spec: &CheckboxCalcIntrinsicSizeSpec) -> IntrinsicSize {
        IntrinsicSize::preferred(Vec2::new(spec.style.size, spec.style.size))
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
    pub fn checkbox(
        spec: CheckboxSpec,
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
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r.inset(-(s.focus_offset + s.focus_width)),
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_focus_z(),
            });
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
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: r,
            color: tint(fill),
            z: spec.layer.get_z(),
        });

        // Box border.
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: r,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        // Inner mark.
        match state.checked {
            CheckedState::Checked => {
                // Checkmark: two lines forming a tick (v).
                let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
                let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
                let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
                let mark = tint(s.mark);
                cmds.push(DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0,
                    p1,
                    color: mark,
                    width: s.mark_width,
                    z: spec.layer.get_z(),
                });
                cmds.push(DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0: p1,
                    p1: p2,
                    color: mark,
                    width: s.mark_width,
                    z: spec.layer.get_z(),
                });
            }
            CheckedState::Indeterminate => {
                // Horizontal dash.
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: tint(s.mark),
                    z: spec.layer.get_z(),
                });
            }
            CheckedState::Unchecked => {}
        }

        CheckboxResult {
            input: input_info,
            focused,
            content_bounds: r.inset(s.border_width),
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
    pub border: Color,
    pub mark: Color,
    pub focus: Color,
    pub border_width: f32,
    pub mark_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl CheckboxStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            size: 14.0,
            background: theme.paper_elev,
            hovered: theme.hover,
            pressed: theme.press,
            selected_fill: theme.ink,
            selected_hovered: Color::BLACK,
            selected_pressed: Color::from_srgb_u8(42, 37, 32, 255),
            border: theme.ink,
            mark: theme.paper,
            focus: theme.rust,
            border_width: 1.0,
            mark_width: 1.5,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
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

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckboxSpecBuilder {
    pub disabled: Option<bool>,
    pub allowed_checked_states: Option<Vec<CheckedState>>,
    pub style: Option<CheckboxStyle>,
}
impl CheckboxSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn allowed_checked_states(mut self, allowed_checked_states: Vec<CheckedState>) -> Self {
        self.allowed_checked_states = Some(allowed_checked_states);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(CheckboxStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> CheckboxSpec {
        CheckboxSpec {
            disabled: self.disabled.unwrap_or(false),
            allowed_checked_states: self
                .allowed_checked_states
                .unwrap_or_else(|| vec![CheckedState::Unchecked, CheckedState::Checked]),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level checkbox widget function using WidgetContext.
///
/// This function accepts a CheckboxSpecBuilder and calls the low-level raw::checkbox function.
pub fn checkbox<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: CheckboxSpecBuilder,
    layout_params: S::Params,
    state: &mut CheckboxState,
) -> CheckboxResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::CheckboxCalcIntrinsicSizeSpec { style: spec.style };
    let intrinsic = raw::calc_checkbox_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::CheckboxSpec {
        rect,
        disabled: spec.disabled,
        allowed_checked_states: spec.allowed_checked_states,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::checkbox(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);

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
pub fn labelled_checkbox<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: CheckboxSpecBuilder,
    label_text: &str,
    layout_params: S::Params,
    state: &mut CheckboxState,
) -> CheckboxResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();

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

    // Calculate intrinsic size using the official functions of both widgets
    let checkbox_calc_spec = raw::CheckboxCalcIntrinsicSizeSpec { style: spec.style };
    let checkbox_intrinsic = raw::calc_checkbox_intrinsic_size(&checkbox_calc_spec);
    let checkbox_size = checkbox_intrinsic.preferred.unwrap();

    let label_calc_spec = crate::widgets::label::raw::LabelCalcIntrinsicSizeSpec {
        text: label_text,
        style: label_style,
    };
    let label_intrinsic =
        crate::widgets::label::raw::calc_label_intrinsic_size(&label_calc_spec, ctx.text_system);
    let label_size = label_intrinsic.preferred.unwrap();

    let gap = 8.0;
    let combined_width = checkbox_size.x + gap + label_size.x;
    let combined_height = f32::max(checkbox_size.y, label_size.y);
    let intrinsic = IntrinsicSize::preferred(Vec2::new(combined_width, combined_height));

    // Resolve combined bounds
    let rect = ctx.layout(layout_params, intrinsic);

    // Run underlying checkbox interaction and draw control box
    let raw_spec = raw::CheckboxSpec {
        rect, // Pass the combined bounds for unified interaction handling
        disabled: spec.disabled,
        allowed_checked_states: spec.allowed_checked_states,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::checkbox(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);

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
    crate::widgets::label::raw::label(raw_label_spec, ctx.text_system, ctx.cmds);

    CheckboxResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::CheckboxSpec;
    use super::*;

    fn checkbox_spec(rect: Rect) -> CheckboxSpec {
        CheckboxSpec {
            rect,
            disabled: false,
            allowed_checked_states: vec![CheckedState::Unchecked, CheckedState::Checked],
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        }
    }

    fn tri_state_checkbox_spec(rect: Rect) -> CheckboxSpec {
        CheckboxSpec {
            allowed_checked_states: vec![
                CheckedState::Unchecked,
                CheckedState::Checked,
                CheckedState::Indeterminate,
            ],
            ..checkbox_spec(rect)
        }
    }

    fn draw_two_checkboxes(
        focus_system: &mut FocusSystem,
        state1: &mut CheckboxState,
        state2: &mut CheckboxState,
        input: &Input,
        cmds: &mut DrawCommands,
    ) {
        raw::checkbox(
            checkbox_spec(Rect::new(0.0, 0.0, 14.0, 14.0)),
            state1,
            input,
            focus_system,
            cmds,
        );
        raw::checkbox(
            checkbox_spec(Rect::new(0.0, 40.0, 14.0, 14.0)),
            state2,
            input,
            focus_system,
            cmds,
        );
    }

    #[test]
    fn test_checkbox_overlapping_hover() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();

        crate::widgets::test_helpers::assert_overlapping_hover(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::checkbox(
                    checkbox_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    state1,
                    input,
                    focus_system,
                    cmds,
                );
                let res2 = raw::checkbox(
                    checkbox_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                    state2,
                    input,
                    focus_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_checkbox_overlapping_click() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();

        crate::widgets::test_helpers::assert_overlapping_click(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            true,
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::checkbox(
                    checkbox_spec(Rect::new(0.0, 0.0, 100.0, 100.0)),
                    state1,
                    input,
                    focus_system,
                    cmds,
                );
                let res2 = raw::checkbox(
                    checkbox_spec(Rect::new(50.0, 50.0, 100.0, 100.0)),
                    state2,
                    input,
                    focus_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_checkbox_tab_moves_focus_next() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_tab_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_checkboxes(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_checkbox_right_arrow_moves_focus_next() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_right_arrow_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_checkboxes(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_checkbox_down_arrow_moves_focus_next() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_down_arrow_moves_focus_next(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_checkboxes(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_checkbox_shift_tab_moves_focus_prev() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();
        let focus1 = state1.focus_id;
        let focus2 = state2.focus_id;

        crate::widgets::test_helpers::assert_shift_tab_moves_focus_prev(
            &mut state1,
            focus1,
            &mut state2,
            focus2,
            |state1, state2, input, focus_system, cmds| {
                draw_two_checkboxes(focus_system, state1, state2, input, cmds);
            },
        );
    }

    #[test]
    fn test_checkbox_visual_off() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_vertically_centered() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 20.0));
        let s = spec.style;
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        // Expect Y to be 10.0 + (20.0 - 14.0) * 0.5 = 13.0
        let expected_rect = Rect::new(10.0, 13.0, 14.0, 14.0);
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: expected_rect,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: expected_rect,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_hovered() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState::default();

        // Warmup frame to establish hover claim
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.hovered,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_pressed() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState::default();

        // Warmup frame with mouse inside but not pressed
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame with mouse pressed down
        input.mouse_down = true;
        input.mouse_pressed = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.pressed,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_on() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
        let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
        let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState {
                checked: CheckedState::Checked,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.selected_fill,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0,
                    p1,
                    color: s.mark,
                    width: s.mark_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0: p1,
                    p1: p2,
                    color: s.mark,
                    width: s.mark_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_on_hovered() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
        let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
        let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState {
            checked: CheckedState::Checked,
            ..Default::default()
        };

        // Warmup frame to establish hover claim
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.selected_hovered,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0,
                    p1,
                    color: s.mark,
                    width: s.mark_width,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: true,
                    p0: p1,
                    p1: p2,
                    color: s.mark,
                    width: s.mark_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_indeterminate() {
        let spec = tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState {
                checked: CheckedState::Indeterminate,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.selected_fill,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: s.mark,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_indeterminate_pressed() {
        let spec = tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState {
            checked: CheckedState::Indeterminate,
            ..Default::default()
        };

        // Warmup frame with mouse inside but not pressed
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame with mouse pressed down
        input.mouse_down = true;
        input.mouse_pressed = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.selected_pressed,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: s.mark,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_clamps_state_to_first_allowed_state() {
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let mut state = CheckboxState {
            checked: CheckedState::Indeterminate,
            ..Default::default()
        };

        raw::checkbox(
            spec,
            &mut state,
            &Input::default(),
            &mut FocusSystem::new(),
            &mut DrawCommands::new(),
        );

        assert_eq!(
            state.checked,
            CheckedState::Unchecked,
            "Checkbox should clamp to the first allowed state"
        );
    }

    #[test]
    fn test_checkbox_click_cycles_allowed_states_in_order() {
        let mut state = CheckboxState::default();

        for expected in [
            CheckedState::Checked,
            CheckedState::Indeterminate,
            CheckedState::Unchecked,
        ] {
            crate::widgets::test_helpers::assert_mouse_click_on_release(
                &mut state,
                Vec2::new(15.0, 15.0),
                |state, input, focus_system, cmds| {
                    raw::checkbox(
                        tri_state_checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                        state,
                        input,
                        focus_system,
                        cmds,
                    )
                    .input
                },
            );

            assert_eq!(state.checked, expected);
        }
    }

    #[test]
    fn test_checkbox_click_honors_nonstandard_allowed_state_order() {
        let mut state = CheckboxState {
            checked: CheckedState::Checked,
            ..Default::default()
        };
        let spec = || CheckboxSpec {
            allowed_checked_states: vec![CheckedState::Checked, CheckedState::Indeterminate],
            ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
        };

        for expected in [CheckedState::Indeterminate, CheckedState::Checked] {
            crate::widgets::test_helpers::assert_mouse_click_on_release(
                &mut state,
                Vec2::new(15.0, 15.0),
                |state, input, focus_system, cmds| {
                    raw::checkbox(spec(), state, input, focus_system, cmds).input
                },
            );

            assert_eq!(state.checked, expected);
        }
    }

    #[test]
    #[should_panic(expected = "CheckboxSpec::allowed_checked_states must not be empty")]
    fn test_checkbox_panics_when_allowed_states_is_empty() {
        let mut state = CheckboxState::default();
        raw::checkbox(
            CheckboxSpec {
                allowed_checked_states: Vec::new(),
                ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
            },
            &mut state,
            &Input::default(),
            &mut FocusSystem::new(),
            &mut DrawCommands::new(),
        );
    }

    #[test]
    fn test_checkbox_visual_focused() {
        let state = CheckboxState::default();
        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.begin_frame();
        let spec = checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut state = state;
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r.inset(-(s.focus_offset + s.focus_width)),
                    color: s.focus,
                    width: s.focus_width,
                    z: 1,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_disabled() {
        let spec = CheckboxSpec {
            disabled: true,
            ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
        };
        let s = spec.style;
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: tint(s.background),
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: tint(s.border),
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_click_triggers_clicked_state() {
        let mut state = CheckboxState::default();

        crate::widgets::test_helpers::assert_mouse_click_on_release(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_checkbox_click_takes_focus() {
        let mut state = CheckboxState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_mouse_press_takes_focus(
            &mut state,
            focus_id,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_checkbox_clipped_click_does_not_take_focus() {
        let mut state = CheckboxState::default();

        crate::widgets::test_helpers::assert_clipped_mouse_press_does_not_take_focus(
            &mut state,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    CheckboxSpec {
                        clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
                        ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                    },
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_checkbox_disabled_ignores_interaction() {
        let mut state = CheckboxState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_disabled_ignores_press_interaction(
            &mut state,
            focus_id,
            Vec2::new(15.0, 15.0),
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    CheckboxSpec {
                        disabled: true,
                        ..checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0))
                    },
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
        assert_eq!(state.checked, CheckedState::Unchecked);
    }

    #[test]
    fn test_enter_toggles_raw_checkbox() {
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState::default();
        let mut input = Input::default();

        let spec = || checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0));

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        input.key_pressed_enter = true;
        focus_system.begin_frame();
        let result = raw::checkbox(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert!(
            result.input.clicked,
            "Checkbox should be clicked by Enter key"
        );
        assert_eq!(
            state.checked,
            CheckedState::Checked,
            "Enter key must toggle checkbox state"
        );
    }

    #[test]
    fn test_checkbox_hover_and_press_state() {
        let mut state = CheckboxState::default();

        crate::widgets::test_helpers::assert_hover_and_press_state(
            &mut state,
            Vec2::new(15.0, 15.0),
            Vec2::new(150.0, 150.0),
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );
    }

    #[test]
    fn test_checkbox_drag_off_and_release_does_not_click_other_checkbox() {
        let mut state1 = CheckboxState::default();
        let mut state2 = CheckboxState::default();

        crate::widgets::test_helpers::assert_drag_off_and_release_does_not_click_other(
            &mut state1,
            &mut state2,
            Vec2::new(15.0, 15.0),
            Vec2::new(15.0, 115.0),
            false,
            |state1, state2, input, focus_system, cmds| {
                let res1 = raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state1,
                    input,
                    focus_system,
                    cmds,
                );
                let res2 = raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 110.0, 14.0, 14.0)),
                    state2,
                    input,
                    focus_system,
                    cmds,
                );
                (res1.input, res2.input)
            },
        );

        assert_eq!(
            state2.checked,
            CheckedState::Unchecked,
            "Dragging onto another checkbox must not toggle it on release"
        );
    }

    #[test]
    fn test_checkbox_spacebar_click() {
        let mut state = CheckboxState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );

        assert_eq!(
            state.checked,
            CheckedState::Checked,
            "Spacebar release must toggle checkbox state"
        );
    }

    #[test]
    fn test_checkbox_spacebar_loses_focus_does_not_click() {
        let mut state = CheckboxState::default();
        let focus_id = state.focus_id;

        crate::widgets::test_helpers::assert_spacebar_loses_focus_does_not_click(
            &mut state,
            focus_id,
            |state, input, focus_system, cmds| {
                raw::checkbox(
                    checkbox_spec(Rect::new(10.0, 10.0, 14.0, 14.0)),
                    state,
                    input,
                    focus_system,
                    cmds,
                )
                .input
            },
        );

        assert_eq!(
            state.checked,
            CheckedState::Unchecked,
            "Losing focus before Space release must not toggle checkbox state"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = CheckboxSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(CheckboxStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = CheckboxStyle::from_theme(&theme);
        custom_style.size = 99.0;
        let builder = CheckboxSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().size, 99.0);
    }

    #[test]
    fn test_builder_preserves_allowed_checked_states() {
        let theme = crate::theme::Theme::framewise();
        let allowed_checked_states = vec![CheckedState::Checked, CheckedState::Indeterminate];

        let spec = CheckboxSpecBuilder::new()
            .allowed_checked_states(allowed_checked_states.clone())
            .defaults_from_theme(&theme)
            .build();

        assert_eq!(spec.allowed_checked_states, allowed_checked_states);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
        let mut text_system = TestTextBackend;
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
        let mut cb_state = CheckboxState::default();
        let result = super::checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new(),
            placement,
            &mut cb_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_high_level_honors_user_style() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let custom = CheckboxStyle {
            background: Color::from_srgb_u8(1, 2, 3, 255),
            ..CheckboxStyle::from_theme(&crate::theme::Theme::default())
        };
        let mut cb_state = CheckboxState::default();
        super::checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new().style(custom),
            Rect::new(100.0, 100.0, 14.0, 14.0),
            &mut cb_state,
        );

        let has_custom_fill = cmds
            .iter()
            .any(|c| matches!(c, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == custom.background));
        assert!(
            has_custom_fill,
            "high-level checkbox must honor user-set style"
        );
    }

    #[test]
    fn test_high_level_honors_allowed_checked_states() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::TestTextBackend;
        let mut text_system = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut cb_state = CheckboxState {
            checked: CheckedState::Indeterminate,
            ..Default::default()
        };

        super::checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new()
                .allowed_checked_states(vec![CheckedState::Checked, CheckedState::Indeterminate]),
            Rect::new(100.0, 100.0, 14.0, 14.0),
            &mut cb_state,
        );

        assert_eq!(
            cb_state.checked,
            CheckedState::Indeterminate,
            "high-level checkbox must pass allowed states to raw checkbox"
        );
    }

    #[test]
    fn test_calc_checkbox_intrinsic_size() {
        let theme = crate::theme::Theme::default();
        let style = CheckboxStyle::from_theme(&theme);
        let spec = raw::CheckboxCalcIntrinsicSizeSpec { style };
        let intrinsic = raw::calc_checkbox_intrinsic_size(&spec);
        assert_eq!(intrinsic, IntrinsicSize::preferred(Vec2::new(14.0, 14.0)));
    }

    #[test]
    fn test_labelled_checkbox_intrinsic_size() {
        use crate::layouts::ManualLayout;
        let mut text_system = crate::test_utils::TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut ctx = WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );

        let mut state = CheckboxState::default();
        // TestTextBackend logical size reports 8.0 per character. "vsync" is 5 chars -> 40.0.
        // Height is 16.0. Checkbox size is 14.0. Gap is 8.0.
        // Combined width: 14.0 + 8.0 + 40.0 = 62.0.
        // Combined height: max(14.0, 16.0) = 16.0.
        let result = super::labelled_checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new(),
            "vsync",
            Rect::new(0.0, 0.0, 100.0, 20.0),
            &mut state,
        );

        assert_eq!(result.layout.bounds, Rect::new(0.0, 0.0, 100.0, 20.0));
    }

    #[test]
    fn test_labelled_checkbox_click_label_toggles_state() {
        use crate::layouts::ManualLayout;
        let mut state = CheckboxState::default();

        crate::widgets::test_helpers::assert_labelled_widget_click_toggles(
            &mut state,
            Vec2::new(40.0, 10.0),
            |state, input, focus, cmds| {
                let mut text_system = crate::test_utils::TestTextBackend;
                let mut ctx = WidgetContext::root(
                    crate::theme::Theme::framewise(),
                    &mut text_system,
                    focus,
                    input,
                    ManualLayout,
                    Rect::new(0.0, 0.0, 800.0, 600.0),
                    cmds,
                );
                super::labelled_checkbox(
                    &mut ctx,
                    CheckboxSpecBuilder::new(),
                    "vsync",
                    Rect::new(0.0, 0.0, 100.0, 20.0),
                    state,
                );
            },
        );

        assert_eq!(state.checked, CheckedState::Checked);
    }

    #[test]
    fn test_labelled_checkbox_disabled_label_visual() {
        use crate::layouts::ManualLayout;
        let mut text_system = crate::test_utils::TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let theme = crate::theme::Theme::framewise();
        {
            let mut ctx = WidgetContext::root(
                theme.clone(),
                &mut text_system,
                &mut focus,
                &input,
                ManualLayout,
                Rect::new(0.0, 0.0, 800.0, 600.0),
                &mut cmds,
            );

            let mut state = CheckboxState::default();
            super::labelled_checkbox(
                &mut ctx,
                CheckboxSpecBuilder::new().disabled(true),
                "vsync",
                Rect::new(0.0, 0.0, 100.0, 20.0),
                &mut state,
            );
        }

        // Find the text draw command to check its color.
        let text_cmd = cmds.iter().find_map(|cmd| {
            if let DrawCmd::GlyphRun { color, .. } = cmd {
                Some(*color)
            } else {
                None
            }
        });

        assert!(text_cmd.is_some());
        let color = text_cmd.unwrap();
        // The default ink color from theme should have disabled_alpha (0.35) applied to its alpha channel.
        let default_label_style = crate::widgets::label::LabelStyle::from_theme(&theme);
        let expected_alpha = default_label_style.text_color.a * 0.35;
        assert!((color.a - expected_alpha).abs() < 1e-4);
    }
}
