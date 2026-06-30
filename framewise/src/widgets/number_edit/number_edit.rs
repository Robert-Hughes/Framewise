use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem, NavDirections},
    input::Input,
    layout::{Align, AxisBound, LayoutState, SizeOffer},
    text::{layout_text, TextBackend, TextBounds, TextLineAlign},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    widgets::{
        text_edit::{self, NewlinePolicy, TextEditState, TextEditStyle},
        widget_helpers::{
            begin_held_press_drag, begin_immediate_drag, draw_prefixed_control_prefix_and_chrome,
            handle_press_drag_interaction, layout_prefixed_control, prefixed_control_child_offer,
            prefixed_control_prefix_width, prefixed_control_size_request, HeldCursorPolicy,
            PrefixedControlDrawSpec, PrefixedControlStyle, PressDragInteraction,
            PressDragInteractionSpec, PressDragState, RepeatTimer, RepeatTiming,
            DEFAULT_DRAG_THRESHOLD,
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
        /// Bounding rect for the full number edit control.
        pub rect: Rect,
        pub style: super::NumberEditStyle,
        pub min: Option<f32>,
        pub max: Option<f32>,
        pub step: f32,
        pub page_step: f32,
        pub drag_enabled: bool,
        pub value_fill_enabled: bool,
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
        pub value: f32,
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
        let value_text = (spec.value_formatter)(spec.value);
        let value_layout = layout_text(
            text_backend,
            &value_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let value_w = value_layout.metrics().logical_size.x + s.text_pad_x * 2.0;
        let step_button_w = number_edit_step_button_width(s.step_button, text_backend);
        let preferred_w = value_w + step_button_w * 2.0;
        crate::layout::SizeRequest::preferred(Vec2::new(preferred_w, s.height))
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
        let mut press_drag = PressDragInteraction::default();

        if spec.disabled {
            state.is_arrow_stepping = false;
            state.arrow_step_direction = None;
            state.press_drag = PressDragState::default();
            state.edit = NumberEditEditState::Inactive;
        }

        let s = spec.style;
        let step_button_w =
            number_edit_step_button_width(s.step_button, text_backend).min(spec.rect.w * 0.5);

        // All interaction and drawing below split the full control into always
        // present step button regions and the central value/edit area.
        let decrement_rect = Rect::new(spec.rect.x, spec.rect.y, step_button_w, spec.rect.h);
        let increment_rect = Rect::new(
            spec.rect.right() - step_button_w,
            spec.rect.y,
            step_button_w,
            spec.rect.h,
        );
        let value_rect = Rect::new(
            spec.rect.x + step_button_w,
            spec.rect.y,
            (spec.rect.w - step_button_w * 2.0).max(0.0),
            spec.rect.h,
        );

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        let contains = spec.rect.contains(input.mouse_pos) && is_visible;

        if contains && !spec.disabled {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = focus_system.is_hover_active(state.focus_id);
        let decrement_hover = crate::widgets::widget_helpers::handle_hover_interaction(
            decrement_rect,
            spec.clip_rect,
            spec.disabled,
            is_hover_active,
            state.is_arrow_stepping
                && state.arrow_step_direction == Some(NumberEditStepDirection::Decrement),
            Some(crate::output::CursorIcon::Pointer),
            input,
        );
        let increment_hover = crate::widgets::widget_helpers::handle_hover_interaction(
            increment_rect,
            spec.clip_rect,
            spec.disabled,
            is_hover_active,
            state.is_arrow_stepping
                && state.arrow_step_direction == Some(NumberEditStepDirection::Increment),
            Some(crate::output::CursorIcon::Pointer),
            input,
        );
        let value_hover = crate::widgets::widget_helpers::handle_hover_interaction(
            value_rect,
            spec.clip_rect,
            spec.disabled,
            is_hover_active,
            state.press_drag.dragging,
            spec.drag_enabled
                .then_some(crate::output::CursorIcon::EwResize),
            input,
        );

        let (clamp_min, clamp_max) = normalise_optional_bounds(spec.min, spec.max);

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
        let vtx = value_rect.x + (value_rect.w - value_metrics.logical_size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - value_metrics.logical_size.y) * 0.5;
        let value_text_rect = Rect::new(
            vtx,
            vty,
            value_metrics.logical_size.x,
            value_metrics.logical_size.y,
        );

        let mut started_editing_this_frame = false;
        let contains_decrement = decrement_hover.contains;
        let contains_increment = increment_hover.contains;
        let hovered_decrement = decrement_hover.passive_hovered;
        let hovered_increment = increment_hover.passive_hovered;
        let start_step_direction = if decrement_hover.can_start {
            Some(NumberEditStepDirection::Decrement)
        } else if increment_hover.can_start {
            Some(NumberEditStepDirection::Increment)
        } else {
            None
        };
        // The embedded TextEdit reuses the NumberEdit focus id. Entering edit mode
        // happens before normal focus registration so only one widget registers it.
        if !state.edit.is_editing()
            && !spec.disabled
            && input.mouse_pressed
            && input.mouse_click_count == 2
            && value_hover.can_start
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
            crate::widgets::widget_helpers::handle_keyboard_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                spec.disabled,
                crate::focus::FocusTraversalKeys::tab_only(),
                input,
                focus_system,
            )
            .focused
        };

        // Display-mode mouse interaction: arrow stepping, repeat, and scrub drag.
        // Edit mode bypasses this so typed values do not also trigger value changes.
        if !spec.disabled && !state.edit.is_editing() {
            let drag_threshold = if spec.drag_enabled {
                DEFAULT_DRAG_THRESHOLD
            } else {
                f32::INFINITY
            };
            let active_contains =
                active_step_contains(state, contains_decrement, contains_increment);
            press_drag = handle_press_drag_interaction(
                &mut state.press_drag,
                input,
                PressDragInteractionSpec {
                    enabled: state.is_arrow_stepping || spec.drag_enabled,
                    threshold: drag_threshold,
                    held_cursor_policy: HeldCursorPolicy::WhileActiveContains(
                        crate::output::CursorIcon::Pointer,
                    ),
                    active_contains,
                    drag_cursor_icon: Some(crate::output::CursorIcon::EwResize),
                },
            );

            if press_drag.released {
                state.is_arrow_stepping = false;
                state.arrow_step_direction = None;
            }

            if state.is_arrow_stepping && spec.drag_enabled && press_drag.drag_started {
                state.is_arrow_stepping = false;
                state.arrow_step_direction = None;
                state.drag_start_value = state.value;
            }

            if decrement_hover.can_start || increment_hover.can_start || value_hover.can_start {
                focus_system.take_keyboard_focus(state.focus_id);
            }

            if let Some(direction) = start_step_direction {
                step_value(state, direction, spec.step, clamp_min, clamp_max);
                state.is_arrow_stepping = true;
                state.arrow_step_direction = Some(direction);
                begin_held_press_drag(&mut state.press_drag, input.mouse_pos);
                state.repeat_timer.start(spec.time, RepeatTiming::PRESS);
            } else if value_hover.can_start && spec.drag_enabled {
                state.drag_start_value = state.value;
                press_drag = begin_immediate_drag(
                    &mut state.press_drag,
                    input.mouse_pos,
                    Some(crate::output::CursorIcon::EwResize),
                );
            }

            if state.is_arrow_stepping
                && input.mouse_down
                && active_step_contains(state, contains_decrement, contains_increment)
            {
                if let Some(direction) = state.arrow_step_direction {
                    if state
                        .repeat_timer
                        .consume_due(spec.time, RepeatTiming::PRESS)
                    {
                        step_value(state, direction, spec.step, clamp_min, clamp_max);
                    }
                }
            }

            if state.press_drag.dragging && spec.drag_enabled {
                let dx = input.mouse_pos.x - state.press_drag.drag_start_pos.x;
                let value_range = drag_value_range(clamp_min, clamp_max);
                let delta_val = (dx / value_rect.w.max(1.0)) * value_range;
                state.value =
                    clamp_optional(state.drag_start_value + delta_val, clamp_min, clamp_max);
            }
        } else if !spec.drag_enabled {
            state.press_drag = PressDragState::default();
        }

        // Display-mode keyboard stepping. TextEdit consumes caret movement while editing.
        if focused && !spec.disabled && !state.edit.is_editing() {
            focus_system.claim_page_dirs(state.focus_id, NavDirections::ALL);

            if input.key_pressed_left || input.key_pressed_up {
                state.value = clamp_optional(state.value - spec.step, clamp_min, clamp_max);
            }
            if input.key_pressed_right || input.key_pressed_down {
                state.value = clamp_optional(state.value + spec.step, clamp_min, clamp_max);
            }
            if input.key_pressed_page_up {
                state.value = clamp_optional(state.value - spec.page_step, clamp_min, clamp_max);
            }
            if input.key_pressed_page_down {
                state.value = clamp_optional(state.value + spec.page_step, clamp_min, clamp_max);
            }
            if input.key_pressed_home {
                if let Some(min) = clamp_min {
                    state.value = min;
                }
            }
            if input.key_pressed_end {
                if let Some(max) = clamp_max {
                    state.value = max;
                }
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_active = focused
            || state.press_drag.dragging
            || focus_system.current_keyboard_focus() == Some(state.focus_id);
        let draw_outer = cmds.snap_rect_edges_to_physical_pixel(spec.rect);
        let draw_value_x = cmds.snap_to_physical_pixel(value_rect.x);

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
        if spec.value_fill_enabled && !state.edit.is_editing() {
            if let (Some(min), Some(max)) = (clamp_min, clamp_max) {
                if min != max {
                    let frac = ((state.value - min) / (max - min)).clamp(0.0, 1.0);
                    // Keep the leading, top, and bottom edges crisp, but leave the moving
                    // right edge unsnapped so the fill can animate smoothly between pixels.
                    if frac > 0.0 {
                        let draw_value_r = value_rect.x + value_rect.w * frac;
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
                }
            }
        }

        for (direction, rect, hovered) in [
            (
                NumberEditStepDirection::Decrement,
                decrement_rect,
                hovered_decrement,
            ),
            (
                NumberEditStepDirection::Increment,
                increment_rect,
                hovered_increment,
            ),
        ] {
            let is_active_step_button =
                state.is_arrow_stepping && state.arrow_step_direction == Some(direction);
            let active_direction_contains = match direction {
                NumberEditStepDirection::Decrement => contains_decrement,
                NumberEditStepDirection::Increment => contains_increment,
            };
            let pressed = is_active_step_button && active_direction_contains;
            let background = if pressed {
                s.step_button.background_pressed
            } else if hovered {
                s.step_button.background_hovered
            } else {
                s.step_button.background
            };
            if background.a > 0.0 {
                cmds.push_crisp_fill_rect(rect, tint(background), spec.layer.get_z());
            }
            if let Some(border) = s.step_button.border {
                cmds.push_crisp_border_rect(
                    rect,
                    Some(Stroke::new(tint(border.color), border.width)),
                    BorderPlacement::Inside,
                    spec.layer.get_z(),
                );
            }
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

        draw_step_button_glyph(
            decrement_rect,
            s.step_button.decrement_glyph,
            s.step_button,
            tint(s.step_button.glyph_color),
            spec.layer,
            text_backend,
            cmds,
        );
        draw_step_button_glyph(
            increment_rect,
            s.step_button.increment_glyph,
            s.step_button,
            tint(s.step_button.glyph_color),
            spec.layer,
            text_backend,
            cmds,
        );

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

        let hovered = decrement_hover.passive_hovered
            || increment_hover.passive_hovered
            || value_hover.passive_hovered;

        let cursor_icon = if state.edit.is_editing() {
            edit_cursor_icon
        } else {
            press_drag
                .cursor_icon
                .or(decrement_hover.cursor_icon)
                .or(increment_hover.cursor_icon)
                .or(value_hover.cursor_icon)
        };
        let active_step = state.is_arrow_stepping && !spec.disabled;

        NumberEditResult {
            input: edit_input_info.unwrap_or(InputInfo {
                hovered,
                pressed: (state.press_drag.dragging || active_step) && !spec.disabled,
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
pub struct NumberEditStepButtonStyle {
    pub padding_x: f32,
    pub background: Color,
    pub background_hovered: Color,
    pub background_pressed: Color,
    pub border: Option<Stroke>,
    pub glyph_color: Color,
    pub text_style: crate::text::TextStyle,
    pub decrement_glyph: &'static str,
    pub increment_glyph: &'static str,
}

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
    pub step_button: NumberEditStepButtonStyle,
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

        let text_style = crate::text::TextStyle::new(
            theme.mono_font,
            theme.text_mono,
            theme.sans_weight_regular,
            crate::text::TextFlow::single_line(),
        );

        Self {
            height: theme.h_md,
            text_pad_x: 10.0,
            text_style,
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                -theme.focus_offset_tight,
            )),
            value_text: theme.ink,
            value_fill: theme.rust_soft_on_paper_elev,
            step_button: NumberEditStepButtonStyle {
                padding_x: 6.0,
                background: Color::TRANSPARENT,
                background_hovered: Color::TRANSPARENT,
                background_pressed: Color::TRANSPARENT,
                border: None,
                glyph_color: Color::linear_rgba(
                    theme.ink.r,
                    theme.ink.g,
                    theme.ink.b,
                    theme.ink.a * 0.55,
                ),
                text_style,
                decrement_glyph: "\u{2039}",
                increment_glyph: "\u{203A}",
            },
            text_edit_style,
            disabled_alpha: 0.35,
        }
    }

    pub fn compact_stepper_from_theme(theme: &crate::theme::Theme) -> Self {
        let mut style = Self::from_theme(theme);
        style.background = Color::TRANSPARENT;
        style.border = Some(Stroke::new(theme.ink, 1.0));
        style.value_fill = Color::TRANSPARENT;
        style.text_style.size = theme.text_sm;
        style.step_button.decrement_glyph = "\u{2212}";
        style.step_button.increment_glyph = "+";
        style.step_button.background = Color::TRANSPARENT;
        style.step_button.background_hovered = theme.paper_hover;
        style.step_button.background_pressed = theme.paper_press;
        style.step_button.border = Some(Stroke::new(theme.ink, 1.0));
        style.step_button.glyph_color = theme.ink;
        style.step_button.padding_x = 8.0;
        style.step_button.text_style.size = theme.text_sm;
        style
    }

    pub fn button_stepper_from_theme(theme: &crate::theme::Theme) -> Self {
        let mut style = Self::from_theme(theme);
        let text_style = crate::text::TextStyle::new(
            theme.sans_font,
            theme.text_md,
            500,
            crate::text::TextFlow::single_line(),
        );

        style.height = theme.h_md;
        style.background = Color::TRANSPARENT;
        style.border = Some(Stroke::new(theme.ink, 1.0));
        style.focus = Some(Outline::new(
            theme.rust,
            theme.focus_width,
            theme.focus_offset,
        ));
        style.value_fill = Color::TRANSPARENT;
        style.value_text = theme.ink;
        style.text_style = text_style;
        style.text_edit_style.min_height = theme.h_md;
        style.text_edit_style.font = theme.sans_font;
        style.text_edit_style.size = theme.text_md;
        style.text_edit_style.weight = 500;
        style.text_edit_style.text_color = theme.ink;
        style.text_edit_style.background = Color::TRANSPARENT;
        style.text_edit_style.background_hovered = Color::TRANSPARENT;
        style.text_edit_style.border = None;
        style.text_edit_style.focus_border = None;
        style.step_button.decrement_glyph = "\u{2190}";
        style.step_button.increment_glyph = "\u{2192}";
        style.step_button.background = Color::TRANSPARENT;
        style.step_button.background_hovered = theme.paper_hover;
        style.step_button.background_pressed = theme.paper_press;
        style.step_button.border = Some(Stroke::new(theme.ink, 1.0));
        style.step_button.glyph_color = theme.ink;
        style.step_button.padding_x = 14.0;
        style.step_button.text_style = text_style;
        style
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
    pub drag_start_value: f32,
    pub is_arrow_stepping: bool,
    pub arrow_step_direction: Option<NumberEditStepDirection>,
    pub press_drag: PressDragState,
    pub repeat_timer: RepeatTimer,
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
    clamp_min: Option<f32>,
    clamp_max: Option<f32>,
) {
    let delta = match direction {
        NumberEditStepDirection::Decrement => -step,
        NumberEditStepDirection::Increment => step,
    };
    state.value = clamp_optional(state.value + delta, clamp_min, clamp_max);
}

fn normalise_optional_bounds(min: Option<f32>, max: Option<f32>) -> (Option<f32>, Option<f32>) {
    match (min, max) {
        (Some(min), Some(max)) => (Some(min.min(max)), Some(min.max(max))),
        other => other,
    }
}

fn clamp_optional(value: f32, min: Option<f32>, max: Option<f32>) -> f32 {
    let mut value = value;
    if let Some(min) = min {
        value = value.max(min);
    }
    if let Some(max) = max {
        value = value.min(max);
    }
    value
}

fn drag_value_range(min: Option<f32>, max: Option<f32>) -> f32 {
    match (min, max) {
        (Some(min), Some(max)) if max != min => max - min,
        _ => 100.0,
    }
}

fn active_step_contains(
    state: &NumberEditState,
    contains_decrement: bool,
    contains_increment: bool,
) -> bool {
    match state.arrow_step_direction {
        Some(NumberEditStepDirection::Decrement) => contains_decrement,
        Some(NumberEditStepDirection::Increment) => contains_increment,
        None => false,
    }
}

fn number_edit_step_button_width<T: TextBackend>(
    style: NumberEditStepButtonStyle,
    text_backend: &mut T,
) -> f32 {
    let decrement = layout_text(
        text_backend,
        style.decrement_glyph,
        style.text_style,
        TextBounds::UNBOUNDED,
    );
    let increment = layout_text(
        text_backend,
        style.increment_glyph,
        style.text_style,
        TextBounds::UNBOUNDED,
    );
    let ink_w = decrement
        .metrics()
        .approx_ink_bounds
        .w
        .max(increment.metrics().approx_ink_bounds.w);
    ink_w + style.padding_x * 2.0
}

fn draw_step_button_glyph<T: TextBackend>(
    rect: Rect,
    glyph: &str,
    style: NumberEditStepButtonStyle,
    color: Color,
    layer: Layer,
    text_backend: &mut T,
    cmds: &mut DrawCommands,
) {
    let layout = layout_text(text_backend, glyph, style.text_style, TextBounds::UNBOUNDED);
    let metrics = layout.metrics();
    let ink = metrics.approx_ink_bounds;
    if ink.w > 0.0 && ink.h > 0.0 {
        let origin = Vec2::new(
            rect.x + (rect.w - ink.w) * 0.5 - ink.x,
            rect.y + (rect.h - ink.h) * 0.5 - ink.y,
        );
        layout.emit_glyphs(cmds, text_backend, origin, color, layer.get_z());
    } else if metrics.logical_size.x > 0.0 && metrics.logical_size.y > 0.0 {
        let origin = Vec2::new(
            rect.x + (rect.w - metrics.logical_size.x) * 0.5,
            rect.y + (rect.h - metrics.logical_size.y) * 0.5,
        );
        layout.emit_glyphs(cmds, text_backend, origin, color, layer.get_z());
    }
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
    state.is_arrow_stepping = false;
    state.arrow_step_direction = None;
    state.press_drag = PressDragState::default();
}

fn parse_number_edit_text(text: &str) -> Option<f32> {
    let value = text.trim().parse::<f32>().ok()?;
    value.is_finite().then_some(value)
}

fn try_commit_number_edit(
    state: &mut NumberEditState,
    clamp_min: Option<f32>,
    clamp_max: Option<f32>,
) -> bool {
    let NumberEditEditState::Editing { text_edit, error } = &mut state.edit else {
        return true;
    };
    if let Some(value) = parse_number_edit_text(&text_edit.value) {
        state.value = clamp_optional(value, clamp_min, clamp_max);
        state.edit = NumberEditEditState::Inactive;
        true
    } else {
        *error = true;
        false
    }
}

fn commit_or_remember_number_edit_on_focus_loss(
    state: &mut NumberEditState,
    clamp_min: Option<f32>,
    clamp_max: Option<f32>,
) {
    let edit = std::mem::take(&mut state.edit);

    match edit {
        NumberEditEditState::Editing { text_edit, .. } => {
            if let Some(value) = parse_number_edit_text(&text_edit.value) {
                state.value = clamp_optional(value, clamp_min, clamp_max);
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
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub step: f32,
    pub page_step: f32,
    pub drag_enabled: bool,
    pub value_fill_enabled: bool,
    pub value_formatter: F,
    pub disabled: bool,
}

impl Default for NumberEditSpec<DefaultNumberEditValueFormatter> {
    fn default() -> Self {
        Self {
            style: NumberEditStyle::default(),
            min: Some(0.0),
            max: Some(100.0),
            step: 1.0,
            page_step: 10.0,
            drag_enabled: true,
            value_fill_enabled: true,
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
        self.min = Some(min);
        self
    }

    pub fn no_min(mut self) -> Self {
        self.min = None;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    pub fn no_max(mut self) -> Self {
        self.max = None;
        self
    }

    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.min = Some(min);
        self.max = Some(max);
        self
    }

    pub fn unbounded(mut self) -> Self {
        self.min = None;
        self.max = None;
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

    pub fn drag_enabled(mut self, enabled: bool) -> Self {
        self.drag_enabled = enabled;
        self
    }

    pub fn value_fill_enabled(mut self, enabled: bool) -> Self {
        self.value_fill_enabled = enabled;
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
            drag_enabled: self.drag_enabled,
            value_fill_enabled: self.value_fill_enabled,
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
        drag_enabled: spec.drag_enabled,
        value_fill_enabled: spec.value_fill_enabled,
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

    ctx.request_cursor(result.cursor_icon);

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
        value: state.value,
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
        value: state.value,
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
    }
    let prefix_hover = crate::widgets::widget_helpers::handle_hover_interaction(
        layout.prefix_rect,
        ctx.clip_rect,
        spec.disabled,
        ctx.focus_system.is_hover_active(state.focus_id),
        false,
        None,
        ctx.input,
    );
    if prefix_hover.can_start {
        ctx.focus_system.take_keyboard_focus(state.focus_id);
    }

    let mut child_style = spec.style;
    child_style.border = None;
    child_style.focus = None;
    child_style.text_edit_style.border = None;
    child_style.text_edit_style.focus_border = None;

    let child_spec = NumberEditSpec {
        style: child_style,
        min: spec.min,
        max: spec.max,
        step: spec.step,
        page_step: spec.page_step,
        drag_enabled: spec.drag_enabled,
        value_fill_enabled: spec.value_fill_enabled,
        value_formatter: spec.value_formatter,
        disabled: spec.disabled,
    };
    let result =
        run_number_edit_resolved_rect(child_spec, layout.child_rect, pre_layout, state, ctx);

    draw_prefixed_control_prefix_and_chrome(
        PrefixedControlDrawSpec {
            layout,
            prefix,
            style: prefix_style,
            active: result.focused,
            disabled: spec.disabled,
            layer: ctx.layer,
        },
        ctx.text_backend,
        ctx.cmds,
    );

    NumberEditResult {
        layout: LayoutInfo::new(outer_rect, result.layout.content_bounds),
        input: InputInfo {
            hovered: result.input.hovered || prefix_hover.passive_hovered,
            pressed: result.input.pressed,
            clicked: result.input.clicked,
        },
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "number_edit_tests.rs"]
mod tests;
