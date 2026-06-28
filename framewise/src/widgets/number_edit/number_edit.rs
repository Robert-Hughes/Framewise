use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{Align, AxisBound, LayoutState, SizeOffer},
    text::{layout_text, TextBackend, TextLineAlign},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    widgets::{
        text_edit::{self, NewlinePolicy, TextEditState, TextEditStyle},
        widget_helpers::{
            draw_prefixed_control_base, draw_prefixed_control_chrome, layout_prefixed_control,
            prefixed_control_child_offer, prefixed_control_prefix_width,
            prefixed_control_size_request, PrefixedControlStyle,
        },
    },
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct NumberEditSpec<F = super::DefaultNumberEditValueFormatter>
    where
        F: Fn(f32) -> String,
    {
        pub layer: Layer,
        /// Bounding rect for the editable numeric value area.
        pub rect: Rect,
        pub style: super::NumberEditStyle,
        pub min: f32,
        pub max: f32,
        pub step: f32,
        pub page_step: f32,
        pub value_formatter: F,
        pub time: f64,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct NumberEditPreLayoutSpec<'f, F>
    where
        F: Fn(f32) -> String,
    {
        pub style: super::NumberEditStyle,
        pub min: f32,
        pub max: f32,
        pub value_formatter: &'f F,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct NumberEditPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct NumberEditResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this number edit would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_number_edit<T: TextBackend, F>(
        spec: &NumberEditPreLayoutSpec<'_, F>,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> NumberEditPreLayoutResult
    where
        F: Fn(f32) -> String,
    {
        NumberEditPreLayoutResult {
            size_request: number_edit_size_request(spec, offer, text_backend),
        }
    }

    fn number_edit_size_request<T: TextBackend, F>(
        spec: &NumberEditPreLayoutSpec<'_, F>,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest
    where
        F: Fn(f32) -> String,
    {
        let s = spec.style;
        let min_text = (spec.value_formatter)(spec.min);
        let max_text = (spec.value_formatter)(spec.max);
        let min_layout = layout_text(
            text_backend,
            &min_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let min_metrics = min_layout.metrics();
        let max_layout = layout_text(
            text_backend,
            &max_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let max_metrics = max_layout.metrics();
        let value_w =
            min_metrics.logical_size.x.max(max_metrics.logical_size.x) + s.text_pad_x * 2.0;
        crate::layout::SizeRequest::preferred(Vec2::new(value_w, s.height))
    }

    /// Low-level number edit widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_number_edit<T: TextBackend, F>(
        spec: NumberEditSpec<F>,
        _pre_layout: NumberEditPreLayoutResult,
        state: &mut NumberEditState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> NumberEditResult
    where
        F: Fn(f32) -> String,
    {
        if spec.disabled {
            state.is_dragging = false;
            state.is_arrow_stepping = false;
            state.arrow_step_direction = None;
            state.edit = NumberEditEditState::Inactive;
        }

        let s = spec.style;

        // All interaction and drawing below use the final value rect, especially
        // the embedded edit field.
        let value_x = spec.rect.x;
        let value_w = spec.rect.w.max(20.0);
        let value_rect = Rect::new(value_x, spec.rect.y, value_w, spec.rect.h);
        let arrow_w = value_w.min(20.0).min(value_w * 0.5);
        let left_arrow_rect = Rect::new(value_x, spec.rect.y, arrow_w, spec.rect.h);
        let right_arrow_rect = Rect::new(
            value_x + value_w - arrow_w,
            spec.rect.y,
            arrow_w,
            spec.rect.h,
        );

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        let contains = spec.rect.contains(input.mouse_pos) && is_visible;
        let contains_value = value_rect.contains(input.mouse_pos) && is_visible;

        if contains && !spec.disabled {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = focus_system.is_hover_active(state.focus_id);

        let clamp_min = spec.min.min(spec.max);
        let clamp_max = spec.min.max(spec.max);

        // Measure the displayed value text once here; drawing happens later, but
        // the layout also informs the overall value-lane visual balance.
        let value_text = (spec.value_formatter)(state.value);
        let value_layout = layout_text(
            text_backend,
            &value_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let value_metrics = value_layout.metrics();
        let vtx = value_x + (value_w - value_metrics.logical_size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - value_metrics.logical_size.y) * 0.5;
        let value_text_rect = Rect::new(
            vtx,
            vty,
            value_metrics.logical_size.x,
            value_metrics.logical_size.y,
        );

        let mut started_editing_this_frame = false;
        let hovered_left_arrow = contains_value && left_arrow_rect.contains(input.mouse_pos);
        let hovered_right_arrow = contains_value && right_arrow_rect.contains(input.mouse_pos);
        let hovered_arrow_direction = if hovered_left_arrow {
            Some(NumberEditStepDirection::Decrement)
        } else if hovered_right_arrow {
            Some(NumberEditStepDirection::Increment)
        } else {
            None
        };
        let hovered_drag_region =
            contains_value && is_hover_active && hovered_arrow_direction.is_none();
        // The embedded TextEdit reuses the NumberEdit focus id. Entering edit mode
        // happens before normal focus registration so only one widget registers it.
        if !state.edit.is_editing()
            && !spec.disabled
            && input.mouse_pressed
            && input.mouse_click_count == 2
            && hovered_drag_region
        {
            enter_number_edit_mode(state);
            focus_system.take_keyboard_focus(state.focus_id);
            started_editing_this_frame = true;
        }

        let keyboard_enter_starts_editing = !state.edit.is_editing()
            && !spec.disabled
            && input.key_pressed_enter
            && focus_system.current_keyboard_focus() == Some(state.focus_id);
        if keyboard_enter_starts_editing {
            enter_number_edit_mode(state);
            focus_system.take_keyboard_focus(state.focus_id);
            started_editing_this_frame = true;
        }

        let mut text_edit_result = None;
        // In display mode the NumberEdit owns focus registration; in edit mode raw
        // TextEdit registers the same focus id instead.
        let focused = if state.edit.is_editing() || spec.disabled {
            false
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::tab_only(),
                spec.disabled,
            )
            .0
        };

        // Display-mode mouse interaction: arrow stepping, repeat, and scrub drag.
        // Edit mode bypasses this so typed values do not also trigger value changes.
        if !spec.disabled && !state.edit.is_editing() {
            let hovered_value_area = contains_value && is_hover_active;

            if state.is_arrow_stepping && !input.mouse_down {
                state.is_arrow_stepping = false;
                state.arrow_step_direction = None;
            }

            let dx = input.mouse_pos.x - state.arrow_step_start_mouse_pos.x;
            let dy = input.mouse_pos.y - state.arrow_step_start_mouse_pos.y;
            let drag_dist = dx.hypot(dy);
            const ARROW_DRAG_THRESHOLD: f32 = 4.0;
            if state.is_arrow_stepping && input.mouse_down && drag_dist > ARROW_DRAG_THRESHOLD {
                state.is_arrow_stepping = false;
                state.arrow_step_direction = None;
                state.is_dragging = true;
                state.drag_start_x = input.mouse_pos.x;
                state.drag_start_value = state.value;
            }

            if input.mouse_pressed && contains && is_hover_active {
                focus_system.take_keyboard_focus(state.focus_id);
            }

            if input.mouse_pressed && hovered_value_area {
                if let Some(direction) = hovered_arrow_direction {
                    step_value(state, direction, spec.step, clamp_min, clamp_max);
                    state.is_arrow_stepping = true;
                    state.arrow_step_start_mouse_pos = input.mouse_pos;
                    state.arrow_step_direction = Some(direction);
                    state.next_repeat_time = spec.time + 0.5;
                } else {
                    state.is_dragging = true;
                    state.drag_start_x = input.mouse_pos.x;
                    state.drag_start_value = state.value;
                }
            }

            if state.is_arrow_stepping && input.mouse_down && spec.time >= state.next_repeat_time {
                if let Some(direction) = state.arrow_step_direction {
                    step_value(state, direction, spec.step, clamp_min, clamp_max);
                    state.next_repeat_time = spec.time + 0.05;
                }
            }

            if state.is_dragging {
                if !input.mouse_down {
                    state.is_dragging = false;
                } else {
                    let dx = input.mouse_pos.x - state.drag_start_x;
                    let value_range = spec.max - spec.min;
                    let delta_val = (dx / value_w) * value_range;
                    state.value = (state.drag_start_value + delta_val).clamp(clamp_min, clamp_max);
                }
            }
        }

        // Display-mode keyboard stepping. TextEdit consumes caret movement while editing.
        if focused && !spec.disabled && !state.edit.is_editing() {
            focus_system.claim_pgup_vert(state.focus_id);
            focus_system.claim_pgdn_vert(state.focus_id);
            focus_system.claim_pgup_horiz(state.focus_id);
            focus_system.claim_pgdn_horiz(state.focus_id);

            if input.key_pressed_left || input.key_pressed_up {
                state.value = (state.value - spec.step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_right || input.key_pressed_down {
                state.value = (state.value + spec.step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_page_up {
                state.value = (state.value - spec.page_step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_page_down {
                state.value = (state.value + spec.page_step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_home {
                state.value = clamp_min;
            }
            if input.key_pressed_end {
                state.value = clamp_max;
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_active = focused
            || state.is_dragging
            || focus_system.current_keyboard_focus() == Some(state.focus_id);
        let draw_outer = cmds.snap_rect_edges_to_physical_pixel(spec.rect);
        let draw_value_x = cmds.snap_to_physical_pixel(value_x);

        // Focus / active ring.
        if visually_active && !spec.disabled {
            if let Some(outline) = s.focus {
                let focus_stroke = Stroke::new(tint(outline.stroke.color), outline.stroke.width);
                cmds.push_crisp_border_rect(
                    spec.rect.inset(-outline.offset),
                    Some(focus_stroke),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        cmds.push_crisp_fill_rect(draw_outer, tint(s.background), spec.layer.get_z());

        // Value area: rust_soft fill proportional to value fraction.
        let frac = if spec.max > spec.min {
            ((state.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0)
        } else if spec.max < spec.min {
            ((state.value - spec.max) / (spec.min - spec.max)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if frac > 0.0 && !state.edit.is_editing() {
            // Keep the leading, top, and bottom edges crisp, but leave the moving
            // right edge unsnapped so the fill can animate smoothly between pixels.
            let draw_value_r = value_x + value_w * frac;
            cmds.push(DrawCmd::FillRect {
                rect: Rect::from_ltrb(
                    draw_value_x,
                    draw_outer.y,
                    draw_value_r,
                    draw_outer.bottom(),
                ),
                color: tint(s.value_fill),
                z: spec.layer.get_z(),
            });
        }

        if let NumberEditEditState::Editing { text_edit, error } = &mut state.edit {
            // Suppress the activation click for the inner TextEdit. The shared focus id
            // is intentional, but the initial double-click must not become word selection.
            let mut edit_input;
            let input_for_text_edit = if started_editing_this_frame {
                edit_input = input.clone();
                edit_input.mouse_pressed = false;
                edit_input.mouse_down = false;
                edit_input.mouse_clicked = false;
                edit_input.mouse_click_count = 0;
                edit_input.key_pressed_enter = false;
                &edit_input
            } else {
                input
            };
            let old_edit_value = text_edit.value.clone();
            // Run raw TextEdit after layout because value_rect only exists in
            // NumberEdit's post-layout phase.
            let pre_layout_spec = text_edit::raw::TextEditPreLayoutSpec {
                style: s.text_edit_style,
                wrap: false,
                line_align: TextLineAlign::Center,
                error: *error,
                disabled: spec.disabled,
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
            };
            let offer = SizeOffer::new(
                AxisBound::Exact(value_rect.w),
                AxisBound::Exact(value_rect.h),
            );
            let pre_layout = text_edit::raw::pre_layout_text_edit(
                &pre_layout_spec,
                offer,
                text_edit,
                input_for_text_edit,
                focus_system,
                text_backend,
            );
            if text_edit.value != old_edit_value {
                *error = false;
            }
            let text_edit_spec = text_edit::raw::TextEditSpec {
                rect: value_rect,
                style: s.text_edit_style,
                placeholder: None,
                clip_rect: spec.clip_rect,
                error: *error,
                disabled: spec.disabled,
                time: spec.time,
                layer: spec.layer,
                newline_policy: NewlinePolicy::TrimAfterFirstNewline,
                wrap: false,
                vertical_align: Align::Center,
                line_align: TextLineAlign::Center,
            };
            text_edit_result = Some(text_edit::raw::post_layout_text_edit(
                text_edit_spec,
                pre_layout,
                text_edit,
                input_for_text_edit,
                focus_system,
                text_backend,
                cmds,
            ));
        } else {
            value_layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(value_text_rect.x, value_text_rect.y),
                tint(s.value_text),
                spec.layer.get_z(),
            );
        }

        if !state.edit.is_editing()
            && !spec.disabled
            && (contains_value && is_hover_active || state.is_arrow_stepping)
        {
            let arrow_color = tint(Color::linear_rgba(
                s.value_text.r,
                s.value_text.g,
                s.value_text.b,
                s.value_text.a * 0.55,
            ));
            let left_arrow = "\u{2039}";
            let left_layout = layout_text(
                text_backend,
                left_arrow,
                s.text_style,
                crate::text::TextBounds::UNBOUNDED,
            );
            let left_metrics = left_layout.metrics();
            let left_x =
                left_arrow_rect.x + (left_arrow_rect.w - left_metrics.logical_size.x) * 0.5;
            let left_y = spec.rect.y + (spec.rect.h - left_metrics.logical_size.y) * 0.5;
            left_layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(left_x, left_y),
                arrow_color,
                spec.layer.get_z(),
            );

            let right_arrow = "\u{203A}";
            let right_layout = layout_text(
                text_backend,
                right_arrow,
                s.text_style,
                crate::text::TextBounds::UNBOUNDED,
            );
            let right_metrics = right_layout.metrics();
            let right_x =
                right_arrow_rect.x + (right_arrow_rect.w - right_metrics.logical_size.x) * 0.5;
            let right_y = spec.rect.y + (spec.rect.h - right_metrics.logical_size.y) * 0.5;
            right_layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(right_x, right_y),
                arrow_color,
                spec.layer.get_z(),
            );
        }

        // Border pushed at the very end to draw on top of the value fill.
        let tinted_border = s.border.map(|b| Stroke::new(tint(b.color), b.width));
        cmds.push_crisp_border_rect(
            spec.rect,
            tinted_border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let mut edit_focused = false;
        let mut edit_input_info = None;
        let mut edit_cursor_icon = None;
        // Commit/cancel after raw TextEdit runs so this frame's text events are included.
        if let Some(result) = text_edit_result {
            edit_focused = result.focused;
            edit_input_info = Some(result.input);
            edit_cursor_icon = result.cursor_icon;
        }

        if state.edit.is_editing() && !spec.disabled {
            let clicked_outside_text_edit =
                input.mouse_pressed && !value_rect.contains(input.mouse_pos);
            if input.key_pressed_escape {
                state.edit = NumberEditEditState::Inactive;
                focus_system.take_keyboard_focus(state.focus_id);
                edit_focused = true;
            } else if input.key_pressed_enter && !started_editing_this_frame {
                if try_commit_number_edit(state, clamp_min, clamp_max) {
                    focus_system.take_keyboard_focus(state.focus_id);
                    edit_focused = true;
                }
            } else if clicked_outside_text_edit {
                commit_or_remember_number_edit_on_focus_loss(state, clamp_min, clamp_max);
                edit_focused = false;
            } else if !edit_focused {
                commit_or_remember_number_edit_on_focus_loss(state, clamp_min, clamp_max);
            }
        }

        let hovered = contains
            && is_hover_active
            && !spec.disabled
            && (!input.mouse_down || state.is_dragging || state.edit.is_editing());

        let active_arrow = state.is_arrow_stepping && !spec.disabled;
        let hovered_arrow = !spec.disabled
            && contains_value
            && is_hover_active
            && (left_arrow_rect.contains(input.mouse_pos)
                || right_arrow_rect.contains(input.mouse_pos));
        let cursor_icon = if state.edit.is_editing() {
            edit_cursor_icon
        } else if active_arrow || hovered_arrow {
            Some(crate::output::CursorIcon::Pointer)
        } else if !spec.disabled && (contains_value || state.is_dragging) {
            Some(crate::output::CursorIcon::EwResize)
        } else {
            None
        };

        NumberEditResult {
            input: edit_input_info.unwrap_or(InputInfo {
                hovered,
                pressed: state.is_dragging && !spec.disabled,
                clicked: false,
            }),
            focused: focused
                || edit_focused
                || focus_system.current_keyboard_focus() == Some(state.focus_id),
            content_bounds: spec.rect.inset(s.border.map_or(0.0, |b| b.width)),
            cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NumberEditStyle {
    pub height: f32,
    pub text_pad_x: f32,
    pub text_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Option<Stroke>,
    pub focus: Option<Outline>,
    pub value_text: Color,
    pub value_fill: Color,
    pub text_edit_style: TextEditStyle,
    pub disabled_alpha: f32,
}

impl NumberEditStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        let mut text_edit_style = TextEditStyle::from_theme(theme);
        text_edit_style.min_height = theme.h_md;
        text_edit_style.padding_y = 0.0;
        text_edit_style.border = None;
        text_edit_style.focus_border = None;
        text_edit_style.background = Color::TRANSPARENT;
        text_edit_style.background_hovered = Color::TRANSPARENT;
        text_edit_style.text_color = theme.ink;
        text_edit_style.caret_color = theme.rust;

        Self {
            height: theme.h_md,
            text_pad_x: 10.0,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_mono,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                -theme.focus_offset_tight,
            )),
            value_text: theme.ink,
            value_fill: theme.rust_soft_on_paper_elev,
            text_edit_style,
            disabled_alpha: 0.35,
        }
    }
}

impl Default for NumberEditStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NumberEditState {
    pub value: f32,
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_value: f32,
    pub is_arrow_stepping: bool,
    pub arrow_step_start_mouse_pos: Vec2,
    pub arrow_step_direction: Option<NumberEditStepDirection>,
    pub next_repeat_time: f64,
    pub focus_id: FocusId,
    pub edit: NumberEditEditState,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum NumberEditEditState {
    /// No active editor and no remembered invalid draft.
    ///
    /// Entering edit mode from this state starts from the committed numeric
    /// `NumberEditState::value`, formatted as raw editable text.
    Inactive,

    /// The value field is currently being edited by the embedded raw TextEdit.
    ///
    /// The committed numeric value remains in `NumberEditState::value` until a
    /// valid commit succeeds. Invalid Enter keeps this state active and sets
    /// `error = true`.
    Editing {
        text_edit: TextEditState,
        error: bool,
    },

    /// A previous edit was abandoned by click-away or focus loss while invalid.
    ///
    /// The widget is not currently editing and should not keep keyboard focus
    /// because of this state. The committed numeric value remains in
    /// `NumberEditState::value`. The `draft` should be restored the next time
    /// the user enters edit mode.
    Remembered { draft: String },
}

#[allow(clippy::derivable_impls)]
impl Default for NumberEditEditState {
    fn default() -> Self {
        Self::Inactive
    }
}

impl NumberEditEditState {
    fn is_editing(&self) -> bool {
        matches!(self, Self::Editing { .. })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberEditStepDirection {
    Decrement,
    Increment,
}

fn step_value(
    state: &mut NumberEditState,
    direction: NumberEditStepDirection,
    step: f32,
    clamp_min: f32,
    clamp_max: f32,
) {
    let delta = match direction {
        NumberEditStepDirection::Decrement => -step,
        NumberEditStepDirection::Increment => step,
    };
    state.value = (state.value + delta).clamp(clamp_min, clamp_max);
}

fn number_edit_raw_edit_text(value: f32) -> String {
    value.to_string()
}

fn make_number_edit_text_edit_state(state: &NumberEditState, draft: &str) -> TextEditState {
    let mut text_edit = TextEditState::new(draft);
    text_edit.focus_id = state.focus_id;
    text_edit
}

fn enter_number_edit_mode(state: &mut NumberEditState) {
    let draft = match std::mem::take(&mut state.edit) {
        NumberEditEditState::Inactive => number_edit_raw_edit_text(state.value),
        NumberEditEditState::Remembered { draft } => draft,
        NumberEditEditState::Editing { .. } => unreachable!("guarded by !is_editing"),
    };
    let text_edit = make_number_edit_text_edit_state(state, &draft);
    state.edit = NumberEditEditState::Editing {
        text_edit,
        error: false,
    };
    state.is_dragging = false;
    state.is_arrow_stepping = false;
    state.arrow_step_direction = None;
}

fn parse_number_edit_text(text: &str) -> Option<f32> {
    let value = text.trim().parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

fn try_commit_number_edit(state: &mut NumberEditState, clamp_min: f32, clamp_max: f32) -> bool {
    let NumberEditEditState::Editing { text_edit, error } = &mut state.edit else {
        return true;
    };
    if let Some(value) = parse_number_edit_text(&text_edit.value) {
        state.value = value.clamp(clamp_min, clamp_max);
        state.edit = NumberEditEditState::Inactive;
        true
    } else {
        *error = true;
        false
    }
}

fn commit_or_remember_number_edit_on_focus_loss(
    state: &mut NumberEditState,
    clamp_min: f32,
    clamp_max: f32,
) {
    let edit = std::mem::take(&mut state.edit);

    match edit {
        NumberEditEditState::Editing { text_edit, .. } => {
            if let Some(value) = parse_number_edit_text(&text_edit.value) {
                state.value = value.clamp(clamp_min, clamp_max);
                state.edit = NumberEditEditState::Inactive;
            } else {
                state.edit = NumberEditEditState::Remembered {
                    draft: text_edit.value,
                };
            }
        }
        other => {
            state.edit = other;
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct NumberEditResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

pub type DefaultNumberEditValueFormatter = fn(f32) -> String;

pub fn default_number_edit_value_formatter(value: f32) -> String {
    format!("{value:.2}")
}

#[derive(Debug, Clone, PartialEq)]
pub struct NumberEditSpec<F = DefaultNumberEditValueFormatter>
where
    F: Fn(f32) -> String,
{
    pub style: NumberEditStyle,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub page_step: f32,
    pub value_formatter: F,
    pub disabled: bool,
}

impl Default for NumberEditSpec<DefaultNumberEditValueFormatter> {
    fn default() -> Self {
        Self {
            style: NumberEditStyle::default(),
            min: 0.0,
            max: 100.0,
            step: 1.0,
            page_step: 10.0,
            value_formatter: default_number_edit_value_formatter,
            disabled: false,
        }
    }
}

impl NumberEditSpec<DefaultNumberEditValueFormatter> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::new().theme(theme)
    }

    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }
}

impl<F> NumberEditSpec<F>
where
    F: Fn(f32) -> String,
{
    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = NumberEditStyle::from_theme(theme);
        self
    }

    pub fn style(mut self, style: NumberEditStyle) -> Self {
        self.style = style;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn page_step(mut self, page_step: f32) -> Self {
        self.page_step = page_step;
        self
    }

    pub fn value_formatter<G>(self, value_formatter: G) -> NumberEditSpec<G>
    where
        G: Fn(f32) -> String,
    {
        NumberEditSpec {
            style: self.style,
            min: self.min,
            max: self.max,
            step: self.step,
            page_step: self.page_step,
            value_formatter,
            disabled: self.disabled,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

fn run_number_edit_resolved_rect<T: TextBackend, S: LayoutState, CF, F>(
    spec: NumberEditSpec<F>,
    rect: Rect,
    pre_layout: raw::NumberEditPreLayoutResult,
    state: &mut NumberEditState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> NumberEditResult
where
    F: Fn(f32) -> String,
{
    let raw_spec = raw::NumberEditSpec {
        layer: ctx.layer,
        rect,
        style: spec.style,
        min: spec.min,
        max: spec.max,
        step: spec.step,
        page_step: spec.page_step,
        value_formatter: spec.value_formatter,
        time: ctx.time,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_number_edit(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    if let Some(cursor_icon) = result.cursor_icon {
        ctx.output.cursor_icon = Some(cursor_icon);
    }

    NumberEditResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

/// High-level number edit widget function using `WidgetContext`.
///
/// Runs the raw pre-layout phase to obtain a `SizeRequest`, resolves the final
/// rect with layout, then runs the raw post-layout phase.
pub fn number_edit<T: TextBackend, S: LayoutState, CF, F>(
    spec: NumberEditSpec<F>,
    layout_params: S::Params,
    state: &mut NumberEditState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> NumberEditResult
where
    F: Fn(f32) -> String,
{
    let pre_layout_spec = raw::NumberEditPreLayoutSpec {
        style: spec.style,
        min: spec.min,
        max: spec.max,
        value_formatter: &spec.value_formatter,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_number_edit(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    run_number_edit_resolved_rect(spec, rect, pre_layout, state, ctx)
}

/// High-level number edit with an integrated leading prefix segment.
///
/// The prefix is control chrome, not an external label: it shares the same outer
/// background, border, and focus outline as the numeric value editor.
pub fn prefixed_number_edit<T: TextBackend, S: LayoutState, CF, F>(
    prefix: &str,
    spec: NumberEditSpec<F>,
    layout_params: S::Params,
    state: &mut NumberEditState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> NumberEditResult
where
    F: Fn(f32) -> String,
{
    let prefix_style = PrefixedControlStyle {
        background: spec.style.background,
        border: spec.style.border,
        focus: spec.style.focus,
        disabled_alpha: spec.style.disabled_alpha,
        ..PrefixedControlStyle::from_theme(&ctx.theme)
    };
    let prefix_width = prefixed_control_prefix_width(prefix, prefix_style, ctx.text_backend);
    let offer = ctx.peek_offer(layout_params.clone());
    let child_offer = prefixed_control_child_offer(offer, prefix_width);
    let pre_layout_spec = raw::NumberEditPreLayoutSpec {
        style: spec.style,
        min: spec.min,
        max: spec.max,
        value_formatter: &spec.value_formatter,
    };
    let pre_layout = raw::pre_layout_number_edit(&pre_layout_spec, child_offer, ctx.text_backend);
    let size_request = prefixed_control_size_request(pre_layout.size_request, prefix_width);
    let outer_rect = ctx.layout(layout_params, size_request);
    let layout = layout_prefixed_control(outer_rect, prefix_width);

    let is_visible = ctx
        .clip_rect
        .is_none_or(|c| c.contains(ctx.input.mouse_pos));
    let prefix_contains = layout.prefix_rect.contains(ctx.input.mouse_pos) && is_visible;
    if prefix_contains && !spec.disabled {
        ctx.focus_system.claim_hover(state.focus_id);
        if ctx.input.mouse_pressed {
            ctx.focus_system.take_keyboard_focus(state.focus_id);
        }
    }

    draw_prefixed_control_base(
        layout,
        prefix,
        prefix_style,
        spec.disabled,
        ctx.layer,
        ctx.text_backend,
        ctx.cmds,
    );

    let mut child_style = spec.style;
    child_style.background = Color::TRANSPARENT;
    child_style.border = None;
    child_style.focus = None;
    child_style.text_edit_style.background = Color::TRANSPARENT;
    child_style.text_edit_style.background_hovered = Color::TRANSPARENT;
    child_style.text_edit_style.border = None;
    child_style.text_edit_style.focus_border = None;

    let child_spec = NumberEditSpec {
        style: child_style,
        min: spec.min,
        max: spec.max,
        step: spec.step,
        page_step: spec.page_step,
        value_formatter: spec.value_formatter,
        disabled: spec.disabled,
    };
    let result =
        run_number_edit_resolved_rect(child_spec, layout.child_rect, pre_layout, state, ctx);

    draw_prefixed_control_chrome(
        outer_rect,
        prefix_style,
        result.focused || state.is_dragging || state.is_arrow_stepping,
        spec.disabled,
        ctx.layer,
        ctx.cmds,
    );

    NumberEditResult {
        layout: LayoutInfo::new(outer_rect, result.layout.content_bounds),
        input: InputInfo {
            hovered: result.input.hovered || prefix_contains,
            pressed: result.input.pressed,
            clicked: result.input.clicked,
        },
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "number_edit_tests.rs"]
mod tests;
