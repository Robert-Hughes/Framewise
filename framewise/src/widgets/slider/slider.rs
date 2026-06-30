use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem, NavDirections},
    input::Input,
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    widgets::widget_helpers::{
        begin_held_press_drag, begin_immediate_drag, handle_press_drag_interaction,
        HeldCursorPolicy, PressDragInteractionSpec, PressDragState, RepeatTimer, RepeatTiming,
        DEFAULT_DRAG_THRESHOLD,
    },
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderSpec {
        pub rect: Rect,
        pub min: f32,
        pub max: f32,
        /// Minimum allowed distance between `lower` and `upper`.
        ///
        /// Ignored when `state.value` is `SliderValue::Single`.
        /// If both `min_gap` and `max_gap` are set to the same value, the slider
        /// represents a fixed-size span that can be offset but not resized.
        pub min_gap: Option<f32>,
        /// Maximum allowed distance between `lower` and `upper`.
        ///
        /// Ignored when `state.value` is `SliderValue::Single`.
        /// If both `min_gap` and `max_gap` are set to the same value, the slider
        /// represents a fixed-size span that can be offset but not resized.
        pub max_gap: Option<f32>,
        pub value_snap: Option<f32>,
        pub page_step: f32,
        pub step: f32,
        pub orientation: super::Orientation,
        pub style: super::SliderStyle,
        pub clip_rect: ClipRect,
        pub scroll_claim: super::ScrollClaimPolicy,
        pub time: f64,
        /// When `true`, the slider registers nothing in the focus order and
        /// ignores all input — it only draws (tinted by `style.disabled_alpha`)
        /// so it still occupies its reserved track. Used for degenerate
        /// scrollbars (thumb fills the track, nothing to scroll).
        pub disabled: bool,
        pub keyboard_focusable: bool,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderPreLayoutSpec {
        pub orientation: super::Orientation,
        pub style: super::SliderStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderResult {
        pub input: InputInfo,
        pub focused: bool,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    fn slider_nav_claim_dirs(
        orientation: Orientation,
        policy: ScrollClaimPolicy,
        at_min: bool,
        at_max: bool,
    ) -> NavDirections {
        match policy {
            ScrollClaimPolicy::ClaimAllDirections => NavDirections::ALL,
            ScrollClaimPolicy::YieldSameAxisAtEnds => match orientation {
                Orientation::Vertical => NavDirections::ALL.with_up(!at_min).with_down(!at_max),
                Orientation::Horizontal => {
                    NavDirections::ALL.with_left(!at_min).with_right(!at_max)
                }
            },
        }
    }

    /// Return the size this slider would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a slider's extent is
    /// caller-driven: the track length comes from the layout,
    /// not from content, so there is nothing to report yet — this returns
    /// [`SizeRequest::UNKNOWN`]. A later revision may report a cross-axis minimum
    /// derived from the visible thumb or segment style.
    ///
    pub fn pre_layout_slider(
        spec: &SliderPreLayoutSpec,
        offer: SizeOffer,
    ) -> SliderPreLayoutResult {
        SliderPreLayoutResult {
            size_request: slider_size_request(spec, offer),
        }
    }

    fn slider_size_request(
        spec: &SliderPreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let cross = slider_base_cross_axis_request(spec.style)
            + valid_track_marks(spec.style.track_marks)
                .map(|marks| marks.gap.max(0.0) + marks.length)
                .unwrap_or(0.0);

        if cross <= 0.0 {
            crate::layout::SizeRequest::UNKNOWN
        } else if spec.orientation == Orientation::Vertical {
            crate::layout::SizeRequest::preferred(Vec2::new(cross, 0.0))
        } else {
            crate::layout::SizeRequest::preferred(Vec2::new(0.0, cross))
        }
    }

    pub fn post_layout_slider(
        spec: SliderSpec,
        _pre_layout: SliderPreLayoutResult,
        state: &mut SliderState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> SliderResult {
        let min = spec.min.min(spec.max);
        let max = spec.max.max(spec.min);
        let range = max - min;
        repair_values(state, min, max, spec.min_gap, spec.max_gap, spec.value_snap);

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        let is_vert = spec.orientation == Orientation::Vertical;

        let marks_after = valid_track_marks(spec.style.track_marks)
            .map(|marks| marks.gap.max(0.0) + marks.length)
            .unwrap_or(0.0);

        let track_outer_rect = if is_vert {
            Rect::new(
                spec.rect.x,
                spec.rect.y,
                (spec.rect.w - marks_after).max(0.0),
                spec.rect.h,
            )
        } else {
            Rect::new(
                spec.rect.x,
                spec.rect.y,
                spec.rect.w,
                (spec.rect.h - marks_after).max(0.0),
            )
        };

        let (main_start_padding, main_end_padding) = if !state.value.is_range() {
            if let Some(lower_style) = spec.style.lower_thumb_style {
                let len = lower_style.main_axis_length;
                (len * 0.5, len * 0.5)
            } else {
                (0.0, 0.0)
            }
        } else {
            let start_pad = if let Some(lower_style) = spec.style.lower_thumb_style {
                lower_style.main_axis_length * 0.5
            } else {
                0.0
            };
            let end_pad = if let Some(upper_style) = spec.style.upper_thumb_style {
                upper_style.main_axis_length * 0.5
            } else {
                0.0
            };
            (start_pad, end_pad)
        };

        let track_rect = if is_vert {
            let new_y = track_outer_rect.y + main_start_padding;
            let new_h = (track_outer_rect.h - (main_start_padding + main_end_padding)).max(0.0);
            Rect::new(track_outer_rect.x, new_y, track_outer_rect.w, new_h)
        } else {
            let new_x = track_outer_rect.x + main_start_padding;
            let new_w = (track_outer_rect.w - (main_start_padding + main_end_padding)).max(0.0);
            Rect::new(new_x, track_outer_rect.y, new_w, track_outer_rect.h)
        };

        let track_len = if is_vert { track_rect.h } else { track_rect.w };
        let track_start = if is_vert { track_rect.y } else { track_rect.x };
        let lower_coord = value_to_coord(state.value.lower(), min, range, track_start, track_len);
        let upper_coord = state
            .value
            .upper()
            .map(|upper| value_to_coord(upper, min, range, track_start, track_len));

        let lower_start = lower_coord - main_start_padding;
        let mut lower_end = lower_coord + main_start_padding;
        let mut upper_start = upper_coord.map(|c| c - main_end_padding);
        let upper_end = upper_coord.map(|c| c + main_end_padding);

        let mut thumbs_touch = false;
        if let (Some(u_start), Some(_u_end)) = (upper_start, upper_end) {
            if lower_end > u_start {
                thumbs_touch = true;
                let midpoint = (lower_coord + upper_coord.unwrap()) * 0.5;
                lower_end = midpoint + 0.5;
                upper_start = Some(midpoint - 0.5);
            }
        }

        let lower_thumb_rect = spec.style.lower_thumb_style.map(|style| {
            cross_axis_rect(
                track_rect,
                is_vert,
                lower_start,
                (lower_end - lower_start).max(0.0),
                style.cross_axis_size,
            )
        });
        let upper_thumb_rect = spec
            .style
            .upper_thumb_style
            .zip(upper_start)
            .zip(upper_end)
            .map(|((style, u_start), u_end)| {
                cross_axis_rect(
                    track_rect,
                    is_vert,
                    u_start,
                    (u_end - u_start).max(0.0),
                    style.cross_axis_size,
                )
            });
        let segment_rect = spec
            .style
            .segment_style
            .zip(upper_coord)
            .map(|(style, coord)| segment_rect(track_rect, is_vert, lower_coord, coord, style));

        let mouse_coord = if is_vert {
            input.mouse_pos.y
        } else {
            input.mouse_pos.x
        };
        let pointer_over_track = spec.rect.contains(input.mouse_pos);

        let is_over_part_zone = |rect: Rect| -> bool {
            let (start, end) = if is_vert {
                (rect.y, rect.bottom())
            } else {
                (rect.x, rect.right())
            };
            pointer_over_track && mouse_coord >= start && mouse_coord <= end
        };
        let part_zone_rect = |rect: Rect| -> Rect {
            if is_vert {
                Rect::new(spec.rect.x, rect.y, spec.rect.w, rect.h)
            } else {
                Rect::new(rect.x, spec.rect.y, rect.w, spec.rect.h)
            }
        };

        let hit_part = if lower_thumb_rect.is_some_and(is_over_part_zone) {
            Some(SliderPart::LowerThumb)
        } else if upper_thumb_rect.is_some_and(is_over_part_zone) {
            Some(SliderPart::UpperThumb)
        } else if segment_rect.is_some_and(is_over_part_zone) {
            Some(SliderPart::Segment)
        } else {
            None
        };

        let pointer_over_slider = track_rect.contains(input.mouse_pos) || hit_part.is_some();
        let pointer_over_wheel_area = spec.rect.contains(input.mouse_pos);

        let focused = if spec.disabled || !spec.keyboard_focusable {
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

        if !spec.disabled && (pointer_over_slider || pointer_over_wheel_area) && is_visible {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = !spec.disabled && focus_system.is_hover_active(state.focus_id);
        let lower_hover = lower_thumb_rect
            .map(part_zone_rect)
            .map(|rect| {
                crate::widgets::widget_helpers::handle_hover_interaction(
                    rect,
                    spec.clip_rect,
                    spec.disabled,
                    is_hover_active,
                    state.active_part == Some(SliderPart::LowerThumb),
                    Some(crate::output::CursorIcon::Grab),
                    input,
                )
            })
            .unwrap_or_default();
        let upper_hover = upper_thumb_rect
            .map(part_zone_rect)
            .map(|rect| {
                crate::widgets::widget_helpers::handle_hover_interaction(
                    rect,
                    spec.clip_rect,
                    spec.disabled,
                    is_hover_active,
                    state.active_part == Some(SliderPart::UpperThumb),
                    Some(crate::output::CursorIcon::Grab),
                    input,
                )
            })
            .unwrap_or_default();
        let segment_hover = segment_rect
            .map(part_zone_rect)
            .map(|rect| {
                crate::widgets::widget_helpers::handle_hover_interaction(
                    rect,
                    spec.clip_rect,
                    spec.disabled,
                    is_hover_active,
                    state.active_part == Some(SliderPart::Segment),
                    Some(crate::output::CursorIcon::Grab),
                    input,
                )
            })
            .unwrap_or_default();
        let track_hover = crate::widgets::widget_helpers::handle_hover_interaction(
            track_rect,
            spec.clip_rect,
            spec.disabled,
            is_hover_active,
            state.is_track_clicking,
            Some(crate::output::CursorIcon::Pointer),
            input,
        );

        // 2. Input Handling
        let mut press_drag = crate::widgets::widget_helpers::PressDragInteraction::default();

        if spec.disabled {
            state.active_part = None;
            state.is_track_clicking = false;
            state.track_click_direction = None;
            state.press_drag = PressDragState::default();
        }

        if !spec.disabled {
            press_drag = handle_press_drag_interaction(
                &mut state.press_drag,
                input,
                PressDragInteractionSpec {
                    enabled: true,
                    threshold: DEFAULT_DRAG_THRESHOLD,
                    held_cursor_policy: HeldCursorPolicy::None,
                    active_contains: false,
                    drag_cursor_icon: Some(crate::output::CursorIcon::Grabbing),
                },
            );

            if press_drag.released {
                state.active_part = None;
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Track click → drag transition: mouse moved past threshold
            let mouse_coord = if is_vert {
                input.mouse_pos.y
            } else {
                input.mouse_pos.x
            };
            if state.is_track_clicking && press_drag.drag_started {
                let part = first_interactable_part(&spec.style, state.value.is_range());
                if let Some(part) = part {
                    let value = coord_to_value(mouse_coord, min, range, track_start, track_len);
                    match part {
                        SliderPart::LowerThumb => match state.value {
                            SliderValue::Single(_) => state.value = SliderValue::Single(value),
                            SliderValue::Range { upper, .. } => {
                                state.value = SliderValue::Range {
                                    lower: value,
                                    upper,
                                }
                            }
                        },
                        SliderPart::UpperThumb => match state.value {
                            SliderValue::Single(_) => {}
                            SliderValue::Range { lower, .. } => {
                                state.value = SliderValue::Range {
                                    lower,
                                    upper: value,
                                }
                            }
                        },
                        SliderPart::Segment => {
                            move_segment_center_to(state, value, min, max, spec.value_snap)
                        }
                    }
                    repair_values(state, min, max, spec.min_gap, spec.max_gap, spec.value_snap);
                    state.drag_start_value = state.value;
                    state.active_part = Some(part);
                }
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Drag update
            if let Some(active_part) = state.active_part {
                let mouse_coord = if is_vert {
                    input.mouse_pos.y
                } else {
                    input.mouse_pos.x
                };
                let drag_start_coord = if is_vert {
                    state.press_drag.drag_start_pos.y
                } else {
                    state.press_drag.drag_start_pos.x
                };
                let delta = mouse_coord - drag_start_coord;
                let val_delta = if track_len > 0.0 {
                    (delta / track_len) * range
                } else {
                    0.0
                };
                apply_drag_delta(
                    state,
                    active_part,
                    val_delta,
                    min,
                    max,
                    spec.min_gap,
                    spec.max_gap,
                    spec.value_snap,
                );
            }

            // Mouse wheel scrolling — suppressed during an active drag so that drag
            // motion is authoritative (otherwise wheel ticks would stack on top of
            // the drag-projected value).
            if is_visible && state.active_part.is_none() && pointer_over_wheel_area {
                let at_min = active_min_value(state) <= min;
                let at_max = active_max_value(state) >= max;

                focus_system.claim_scroll_dirs(
                    state.focus_id,
                    slider_nav_claim_dirs(spec.orientation, spec.scroll_claim, at_min, at_max),
                );

                if is_hover_active {
                    let active_scroll_dirs = focus_system.active_scroll_dirs(state.focus_id);
                    let scroll_delta = if is_vert {
                        input.scroll_delta.y
                    } else {
                        // For horizontal sliders, listen to horizontal wheel delta.
                        // If horizontal is 0, map vertical wheel to horizontal movement (per user request).
                        if input.scroll_delta.x != 0.0 {
                            input.scroll_delta.x
                        } else {
                            input.scroll_delta.y
                        }
                    };

                    let is_active_up_left = if is_vert {
                        active_scroll_dirs.up
                    } else {
                        active_scroll_dirs.left || active_scroll_dirs.up
                    };
                    let is_active_down_right = if is_vert {
                        active_scroll_dirs.down
                    } else {
                        active_scroll_dirs.right || active_scroll_dirs.down
                    };

                    if scroll_delta > 0.0 && is_active_up_left {
                        step_active_value(
                            state,
                            -scroll_delta * spec.step,
                            min,
                            max,
                            spec.min_gap,
                            spec.max_gap,
                            spec.value_snap,
                        );
                    }
                    if scroll_delta < 0.0 && is_active_down_right {
                        step_active_value(
                            state,
                            -scroll_delta * spec.step,
                            min,
                            max,
                            spec.min_gap,
                            spec.max_gap,
                            spec.value_snap,
                        );
                    }
                }
            }

            // Track click (mouse down on track, not on thumb)
            let active_start = lower_coord.min(upper_coord.unwrap_or(lower_coord));
            let active_end = upper_coord.unwrap_or(lower_coord).max(lower_coord);
            if track_hover.can_start && hit_part.is_none() {
                if spec.keyboard_focusable {
                    focus_system.take_keyboard_focus(state.focus_id);
                }
                state.is_track_clicking = true;
                begin_held_press_drag(&mut state.press_drag, input.mouse_pos);
                state.repeat_timer.start(spec.time, RepeatTiming::PRESS);
                // Page up/down towards mouse
                if mouse_coord < active_start {
                    state.track_click_direction = Some(PagingDirection::Up);
                    page_active_value(
                        state,
                        -spec.page_step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                } else if mouse_coord > active_end {
                    state.track_click_direction = Some(PagingDirection::Down);
                    page_active_value(
                        state,
                        spec.page_step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                }
            }

            // Drag start
            if input.mouse_pressed {
                let start_part = if lower_hover.can_start {
                    Some(SliderPart::LowerThumb)
                } else if upper_hover.can_start {
                    Some(SliderPart::UpperThumb)
                } else if segment_hover.can_start {
                    Some(SliderPart::Segment)
                } else {
                    None
                };
                if let Some(part) = start_part {
                    if spec.keyboard_focusable {
                        focus_system.take_keyboard_focus(state.focus_id);
                    }
                    state.active_part = Some(part);
                    state.drag_start_value = state.value;
                    press_drag = begin_immediate_drag(
                        &mut state.press_drag,
                        input.mouse_pos,
                        Some(crate::output::CursorIcon::Grabbing),
                    );
                }
            }

            // Track click repeat logic (time-based paging)
            if state.is_track_clicking
                && state
                    .repeat_timer
                    .consume_due(spec.time, RepeatTiming::PRESS)
            {
                let drag_dist = press_drag.press_delta.x.hypot(press_drag.press_delta.y);
                if track_rect.contains(input.mouse_pos) || drag_dist <= DEFAULT_DRAG_THRESHOLD {
                    match state.track_click_direction {
                        Some(PagingDirection::Up) => {
                            if mouse_coord < active_start {
                                page_active_value(
                                    state,
                                    -spec.page_step,
                                    min,
                                    max,
                                    spec.min_gap,
                                    spec.max_gap,
                                    spec.value_snap,
                                );
                                if !state.value.is_range() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    let val = snap_value(
                                        state.value.lower().max(cursor_val),
                                        min,
                                        max,
                                        spec.value_snap,
                                    );
                                    state.value = SliderValue::Single(val);
                                }
                            }
                        }
                        Some(PagingDirection::Down) => {
                            if mouse_coord > active_end {
                                page_active_value(
                                    state,
                                    spec.page_step,
                                    min,
                                    max,
                                    spec.min_gap,
                                    spec.max_gap,
                                    spec.value_snap,
                                );
                                if !state.value.is_range() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    let val = snap_value(
                                        state.value.lower().min(cursor_val),
                                        min,
                                        max,
                                        spec.value_snap,
                                    );
                                    state.value = SliderValue::Single(val);
                                }
                            }
                        }
                        None => {}
                    }
                    // else: cursor is now inside the thumb; paging stops but keep
                    // is_track_clicking=true so the drag-transition check can still fire.
                } else {
                    state.track_click_direction = None;
                }
            }

            // Keyboard handling
            if focused {
                let at_min = active_min_value(state) <= min;
                let at_max = active_max_value(state) >= max;

                focus_system.claim_page_dirs(
                    state.focus_id,
                    slider_nav_claim_dirs(spec.orientation, spec.scroll_claim, at_min, at_max),
                );

                let active_page_dirs = focus_system.active_page_dirs(state.focus_id);
                let is_active_pgup = if is_vert {
                    active_page_dirs.up
                } else {
                    active_page_dirs.left
                };
                let is_active_pgdn = if is_vert {
                    active_page_dirs.down
                } else {
                    active_page_dirs.right
                };

                if input.key_pressed(crate::input::Key::PageUp) && is_active_pgup {
                    step_active_value(
                        state,
                        -spec.page_step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                }
                if input.key_pressed(crate::input::Key::PageDown) && is_active_pgdn {
                    step_active_value(
                        state,
                        spec.page_step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                }
                if input.key_pressed(crate::input::Key::ArrowUp)
                    || input.key_pressed(crate::input::Key::ArrowLeft)
                {
                    step_active_value(
                        state,
                        -spec.step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                }
                if input.key_pressed(crate::input::Key::ArrowDown)
                    || input.key_pressed(crate::input::Key::ArrowRight)
                {
                    step_active_value(
                        state,
                        spec.step,
                        min,
                        max,
                        spec.min_gap,
                        spec.max_gap,
                        spec.value_snap,
                    );
                }
                if input.key_pressed(crate::input::Key::Home) {
                    set_active_min(state, min);
                    repair_values(state, min, max, spec.min_gap, spec.max_gap, spec.value_snap);
                }
                if input.key_pressed(crate::input::Key::End) {
                    set_active_max(state, min, max);
                    repair_values(state, min, max, spec.min_gap, spec.max_gap, spec.value_snap);
                }
            }
        }

        // 3. Drawing
        let tint = |c: Color| {
            if spec.disabled {
                Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha)
            } else {
                c
            }
        };

        if let Some(background_fill) = spec.style.background_fill {
            cmds.push_crisp_fill_rect(spec.rect, tint(background_fill), spec.layer.get_z());
        }

        let before_stroke = if !state.value.is_range()
            && state.active_part == Some(SliderPart::LowerThumb)
            && !spec.disabled
        {
            spec.style.before_stroke.map(|s| {
                spec.style.lower_thumb_style.map_or(s, |lower_thumb_style| {
                    Stroke::new(lower_thumb_style.fill.dragged, s.width)
                })
            })
        } else {
            spec.style.before_stroke
        };

        if let Some(before_stroke) = before_stroke {
            draw_track_region(
                cmds,
                spec.layer,
                track_rect,
                is_vert,
                track_start,
                lower_coord,
                before_stroke,
                &tint,
            );
        }
        if let Some(after_stroke) = spec.style.after_stroke {
            draw_track_region(
                cmds,
                spec.layer,
                track_rect,
                is_vert,
                upper_coord.unwrap_or(lower_coord),
                track_start + track_len,
                after_stroke,
                &tint,
            );
        }

        draw_track_marks(
            cmds,
            spec.layer,
            track_rect,
            is_vert,
            min,
            max,
            range,
            track_start,
            track_len,
            spec.style.track_marks,
            &tint,
        );

        if let Some((style, rect)) = spec.style.segment_style.zip(segment_rect) {
            let fill = effective_fill(
                style.fill,
                spec.disabled,
                state.active_part == Some(SliderPart::Segment),
                segment_hover.passive_hovered,
            );
            cmds.push_crisp_fill_rect(rect, tint(fill), spec.layer.get_z());
            if let Some(border) = style.border {
                let border_color =
                    if !spec.disabled && state.active_part == Some(SliderPart::Segment) {
                        style.fill.dragged
                    } else {
                        border.color
                    };
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_crisp_border_rect(
                    rect,
                    Some(tint_stroke(Stroke::new(border_color, border.width))),
                    BorderPlacement::Inside,
                    spec.layer.get_z(),
                );
            }
        }

        let (lower_style, upper_style) = if thumbs_touch {
            let mut l_style = spec.style.lower_thumb_style;
            if let Some(ref mut s) = l_style {
                s.border = None;
            }
            let mut u_style = spec.style.upper_thumb_style;
            if let Some(ref mut s) = u_style {
                s.border = None;
            }
            (l_style, u_style)
        } else {
            (spec.style.lower_thumb_style, spec.style.upper_thumb_style)
        };

        let lower_hovered = lower_hover.passive_hovered;
        let upper_hovered = upper_hover.passive_hovered;

        if thumbs_touch {
            // Note that the crisp pixel rounding logic here needs to be careful
            // to avoid lines jumping around due as the user drags the thumbs closer together
            if upper_coord.is_some() {
                let scale = cmds.physical_pixels_per_logical_pixel();

                let combined_start = lower_start;
                let combined_end = upper_end.unwrap_or(lower_start);

                let start_phys = (combined_start * scale).round();
                let end_phys = (combined_end * scale).round();

                let gap_lower_phys = (spec
                    .style
                    .lower_thumb_style
                    .map_or(0.0, |s| s.main_axis_length * 0.5)
                    * scale)
                    .floor();
                let gap_upper_phys = (spec
                    .style
                    .upper_thumb_style
                    .map_or(0.0, |s| s.main_axis_length * 0.5)
                    * scale)
                    .floor();

                let lower_marker_phys = start_phys + gap_lower_phys;
                let upper_marker_phys = end_phys - gap_upper_phys - 1.0;

                let snapped_start = start_phys / scale;
                let snapped_end = end_phys / scale;
                let snapped_midpoint = lower_marker_phys / scale;

                let cross_rect = cross_axis_rect(
                    track_rect,
                    is_vert,
                    lower_start,
                    (upper_end.unwrap_or(lower_start) - lower_start).max(0.0),
                    spec.style.lower_thumb_style.unwrap().cross_axis_size,
                );

                let combined_rect = if is_vert {
                    Rect::new(
                        cross_rect.x,
                        snapped_start,
                        cross_rect.w,
                        snapped_end - snapped_start,
                    )
                } else {
                    Rect::new(
                        snapped_start,
                        cross_rect.y,
                        snapped_end - snapped_start,
                        cross_rect.h,
                    )
                };

                let lower_rect = if is_vert {
                    Rect::new(
                        cross_rect.x,
                        snapped_start,
                        cross_rect.w,
                        snapped_midpoint - snapped_start,
                    )
                } else {
                    Rect::new(
                        snapped_start,
                        cross_rect.y,
                        snapped_midpoint - snapped_start,
                        cross_rect.h,
                    )
                };

                let upper_rect = if is_vert {
                    Rect::new(
                        cross_rect.x,
                        snapped_midpoint,
                        cross_rect.w,
                        snapped_end - snapped_midpoint,
                    )
                } else {
                    Rect::new(
                        snapped_midpoint,
                        cross_rect.y,
                        snapped_end - snapped_midpoint,
                        cross_rect.h,
                    )
                };

                // Draw lower thumb fill
                if let Some(lower_style) = spec.style.lower_thumb_style {
                    let fill = effective_fill(
                        lower_style.fill,
                        spec.disabled,
                        state.active_part == Some(SliderPart::LowerThumb),
                        lower_hovered,
                    );
                    cmds.push_crisp_fill_rect(lower_rect, tint(fill), spec.layer.get_z());
                }

                // Draw upper thumb fill
                if let Some(upper_style) = spec.style.upper_thumb_style {
                    let fill = effective_fill(
                        upper_style.fill,
                        spec.disabled,
                        state.active_part == Some(SliderPart::UpperThumb),
                        upper_hovered,
                    );
                    cmds.push_crisp_fill_rect(upper_rect, tint(fill), spec.layer.get_z());
                }

                // Draw combined outline/border
                if let Some(lower_style) = spec.style.lower_thumb_style {
                    if let Some(border) = lower_style.border {
                        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                        cmds.push_crisp_border_rect(
                            combined_rect,
                            Some(tint_stroke(Stroke::new(border.color, border.width))),
                            BorderPlacement::Inside,
                            spec.layer.get_z(),
                        );
                    }
                }

                // Draw marker lines
                let mut draw_marker_constant_gap =
                    |marker_coord_phys: f32, style: Option<ThumbStyle>, active: bool| {
                        if let Some(style) = style {
                            let marker_color = if !spec.disabled && active {
                                style.fill.idle
                            } else {
                                style.border.map_or(Color::BLACK, |b| b.color)
                            };
                            let color = tint(marker_color);
                            let marker_coord = marker_coord_phys / scale;

                            if is_vert {
                                cmds.push_device_hairline_h(
                                    combined_rect.x,
                                    marker_coord,
                                    combined_rect.w,
                                    color,
                                    spec.layer.get_z(),
                                );
                            } else {
                                cmds.push_device_hairline_v(
                                    marker_coord,
                                    combined_rect.y,
                                    combined_rect.h,
                                    color,
                                    spec.layer.get_z(),
                                );
                            }
                        }
                    };

                draw_marker_constant_gap(
                    lower_marker_phys,
                    spec.style.lower_thumb_style,
                    state.active_part == Some(SliderPart::LowerThumb),
                );
                draw_marker_constant_gap(
                    upper_marker_phys,
                    spec.style.upper_thumb_style,
                    state.active_part == Some(SliderPart::UpperThumb),
                );
            }
        } else {
            draw_thumb(
                cmds,
                spec.layer,
                lower_thumb_rect,
                lower_style,
                state.active_part == Some(SliderPart::LowerThumb),
                spec.disabled,
                lower_hovered,
                &tint,
            );

            draw_thumb(
                cmds,
                spec.layer,
                upper_thumb_rect,
                upper_style,
                state.active_part == Some(SliderPart::UpperThumb),
                spec.disabled,
                upper_hovered,
                &tint,
            );
        }

        draw_separator_line(
            cmds,
            spec.layer,
            spec.rect,
            is_vert,
            spec.style.separator_line,
            &tint,
        );

        // Focus outline.
        if focused {
            if let Some(outline) = spec.style.focus {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_crisp_border_rect(
                    spec.rect.inset(-outline.offset),
                    Some(tint_stroke(outline.stroke)),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        let cursor_icon = press_drag
            .cursor_icon
            .or(lower_hover.cursor_icon)
            .or(upper_hover.cursor_icon)
            .or(segment_hover.cursor_icon)
            .or(track_hover.cursor_icon);

        SliderResult {
            focused,
            input: InputInfo {
                hovered: lower_hover.passive_hovered
                    || upper_hover.passive_hovered
                    || segment_hover.passive_hovered
                    || track_hover.passive_hovered,
                pressed: !spec.disabled && (state.active_part.is_some() || state.is_track_clicking),
                clicked: false,
            },
            cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

fn value_to_coord(value: f32, min: f32, range: f32, track_start: f32, track_len: f32) -> f32 {
    if range > 0.0 {
        track_start + ((value - min) / range).clamp(0.0, 1.0) * track_len
    } else {
        track_start
    }
}

fn coord_to_value(coord: f32, min: f32, range: f32, track_start: f32, track_len: f32) -> f32 {
    if track_len > 0.0 {
        min + ((coord - track_start) / track_len).clamp(0.0, 1.0) * range
    } else {
        min
    }
}

fn slider_base_cross_axis_request(style: SliderStyle) -> f32 {
    let mut cross: f32 = 0.0;
    for stroke in [
        style.before_stroke,
        style.after_stroke,
        style.separator_line,
    ]
    .into_iter()
    .flatten()
    {
        if stroke.is_visible() && stroke.width.is_finite() {
            cross = cross.max(stroke.width);
        }
    }
    if let Some(style) = style.lower_thumb_style {
        if let CrossAxisSize::FixedCentered(len) = style.cross_axis_size {
            if len.is_finite() && len > 0.0 {
                cross = cross.max(len);
            }
        }
    }
    if let Some(style) = style.upper_thumb_style {
        if let CrossAxisSize::FixedCentered(len) = style.cross_axis_size {
            if len.is_finite() && len > 0.0 {
                cross = cross.max(len);
            }
        }
    }
    if let Some(style) = style.segment_style {
        if let CrossAxisSize::FixedCentered(len) = style.cross_axis_size {
            if len.is_finite() && len > 0.0 {
                cross = cross.max(len);
            }
        }
    }
    cross
}

fn cross_axis_rect(
    track_rect: Rect,
    is_vert: bool,
    main_start: f32,
    main_len: f32,
    cross_axis_size: CrossAxisSize,
) -> Rect {
    let track_cross_size = if is_vert { track_rect.w } else { track_rect.h };
    match cross_axis_size {
        CrossAxisSize::FixedCentered(cross_len) => {
            if is_vert {
                let x = (track_rect.x + (track_rect.w - cross_len) * 0.5).round();
                Rect::new(x, main_start, cross_len, main_len)
            } else {
                let y = (track_rect.y + (track_rect.h - cross_len) * 0.5).round();
                Rect::new(main_start, y, main_len, cross_len)
            }
        }
        CrossAxisSize::FillTrack { margin } => {
            let cross = (track_cross_size - margin * 2.0).max(1.0);
            if is_vert {
                Rect::new(track_rect.x + margin, main_start, cross, main_len)
            } else {
                Rect::new(main_start, track_rect.y + margin, main_len, cross)
            }
        }
    }
}

fn segment_rect(
    track_rect: Rect,
    is_vert: bool,
    lower_coord: f32,
    upper_coord: f32,
    style: SegmentStyle,
) -> Rect {
    let start = lower_coord.min(upper_coord);
    let len = (upper_coord - lower_coord).abs().max(1.0);
    cross_axis_rect(track_rect, is_vert, start, len, style.cross_axis_size)
}

fn effective_gaps(min: f32, max: f32, min_gap: Option<f32>, max_gap: Option<f32>) -> (f32, f32) {
    let domain = max - min;
    let min_gap = min_gap.unwrap_or(0.0).clamp(0.0, domain);
    let max_gap = max_gap.unwrap_or(domain).clamp(min_gap, domain);
    (min_gap, max_gap)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn snap_value(value: f32, min: f32, max: f32, value_snap: Option<f32>) -> f32 {
    // Repair can see externally supplied state, so convert non-finite input to
    // a deterministic in-domain value before doing any arithmetic.
    let value = finite_or(value, min).clamp(min, max);

    // Keep exact endpoints stable. This also preserves Home/End behavior when
    // the snap interval does not divide the domain evenly.
    if value <= min {
        return min;
    }
    if value >= max {
        return max;
    }

    // Snapping is opt-in and invalid snap settings degrade to a continuous
    // clamped slider instead of producing NaN or surprising quantisation.
    let Some(value_snap) = value_snap else {
        return value;
    };
    if !value_snap.is_finite() || value_snap <= 0.0 {
        return value;
    }

    // The grid is anchored at min rather than zero, which keeps non-zero
    // domains aligned to their own value space.
    let snapped = min + ((value - min) / value_snap).round() * value_snap;
    let snapped = snapped.clamp(min, max);

    // Treat max as an extra candidate when it is not on the grid, but only
    // choose it when it is nearer than the ordinary snapped grid value.
    if (max - value).abs() < (value - snapped).abs() {
        max
    } else {
        snapped
    }
}

fn snap_value_in_bounds(
    value: f32,
    min: f32,
    max: f32,
    lower_bound: f32,
    upper_bound: f32,
    value_snap: Option<f32>,
) -> f32 {
    let lower_bound = finite_or(lower_bound, min).clamp(min, max);
    let upper_bound = finite_or(upper_bound, max).clamp(min, max);
    let (lower_bound, upper_bound) = if lower_bound <= upper_bound {
        (lower_bound, upper_bound)
    } else {
        (upper_bound, lower_bound)
    };
    let value = finite_or(value, lower_bound).clamp(lower_bound, upper_bound);

    let Some(candidates) = snapped_candidates(min, max, value_snap) else {
        return value;
    };

    candidates
        .into_iter()
        .filter(|candidate| *candidate >= lower_bound && *candidate <= upper_bound)
        .min_by(|a, b| {
            (value - *a)
                .abs()
                .total_cmp(&(value - *b).abs())
                .then_with(|| a.total_cmp(b))
        })
        // Snapping can be impossible inside an unsnapped gap interval. Preserve
        // the requested bounds in that case rather than returning an invalid
        // snapped value outside the allowed range.
        .unwrap_or(value)
}

fn snapped_candidates(min: f32, max: f32, value_snap: Option<f32>) -> Option<Vec<f32>> {
    const MAX_SNAP_CANDIDATES: usize = 512;

    let value_snap = value_snap?;
    if !value_snap.is_finite() || value_snap <= 0.0 || !min.is_finite() || !max.is_finite() {
        return None;
    }
    let range = max - min;
    if range < 0.0 || !range.is_finite() {
        return None;
    }

    let spacing_count = (range / value_snap).floor();
    if !spacing_count.is_finite() || spacing_count as usize + 2 > MAX_SNAP_CANDIDATES {
        return None;
    }

    let mut candidates = Vec::with_capacity(spacing_count as usize + 2);
    candidates.push(min);
    for i in 1..=spacing_count as usize {
        let candidate = min + i as f32 * value_snap;
        if candidate > min && candidate < max {
            candidates.push(candidate);
        }
    }
    if candidates
        .last()
        .is_none_or(|last| (*last - max).abs() > f32::EPSILON)
    {
        candidates.push(max);
    }
    Some(candidates)
}

fn repair_range_continuous(
    mut lower: f32,
    mut upper: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) -> (f32, f32) {
    lower = snap_value(lower, min, max, value_snap);
    upper = snap_value(upper, min, max, value_snap);
    if lower > upper {
        core::mem::swap(&mut lower, &mut upper);
    }

    let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
    let gap = (upper - lower).clamp(min_gap_val, max_gap_val);

    upper = (lower + gap).min(max);
    lower = (upper - gap).max(min);
    upper = (lower + gap).min(max);
    (lower, upper)
}

fn repair_range_snapped(
    requested_lower: f32,
    requested_upper: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) -> Option<(f32, f32)> {
    let candidates = snapped_candidates(min, max, value_snap)?;
    let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
    let mut target_lower = snap_value(requested_lower, min, max, value_snap);
    let mut target_upper = snap_value(requested_upper, min, max, value_snap);
    if target_lower > target_upper {
        core::mem::swap(&mut target_lower, &mut target_upper);
    }

    let mut best: Option<(f32, f32, f32)> = None;
    for &lower in &candidates {
        for &upper in &candidates {
            let gap = upper - lower;
            if gap < min_gap_val || gap > max_gap_val {
                continue;
            }
            let distance = (lower - target_lower).abs() + (upper - target_upper).abs();
            match best {
                Some((best_distance, best_lower, best_upper))
                    if distance
                        .total_cmp(&best_distance)
                        .then_with(|| lower.total_cmp(&best_lower))
                        .then_with(|| upper.total_cmp(&best_upper))
                        .is_ge() => {}
                _ => best = Some((distance, lower, upper)),
            }
        }
    }
    best.map(|(_, lower, upper)| (lower, upper))
}

fn repair_values(
    state: &mut SliderState,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) {
    match state.value {
        SliderValue::Single(lower) => {
            state.value = SliderValue::Single(snap_value(lower, min, max, value_snap));
        }
        SliderValue::Range {
            mut lower,
            mut upper,
        } => {
            (lower, upper) =
                repair_range_snapped(lower, upper, min, max, min_gap, max_gap, value_snap)
                    .unwrap_or_else(|| {
                        repair_range_continuous(
                            lower, upper, min, max, min_gap, max_gap, value_snap,
                        )
                    });
            state.value = SliderValue::Range { lower, upper };
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn apply_drag_delta(
    state: &mut SliderState,
    part: SliderPart,
    val_delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) {
    match part {
        SliderPart::LowerThumb => match state.value {
            SliderValue::Single(_) => {
                let requested_lower = state.drag_start_value.lower() + val_delta;
                state.value =
                    SliderValue::Single(snap_value(requested_lower, min, max, value_snap));
            }
            SliderValue::Range { upper, .. } => {
                let requested_lower = state.drag_start_value.lower() + val_delta;
                let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
                let min_lower = (upper - max_gap_val).max(min);
                let max_lower = (upper - min_gap_val).min(max);
                let lower = snap_value_in_bounds(
                    requested_lower,
                    min,
                    max,
                    min_lower,
                    max_lower,
                    value_snap,
                );
                state.value = SliderValue::Range { lower, upper };
            }
        },
        SliderPart::UpperThumb => match state.value {
            SliderValue::Single(_) => {}
            SliderValue::Range { lower, .. } => {
                let requested_upper = state.drag_start_value.upper().unwrap_or(max) + val_delta;
                let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
                let min_upper = (lower + min_gap_val).max(min);
                let max_upper = (lower + max_gap_val).min(max);
                let upper = snap_value_in_bounds(
                    requested_upper,
                    min,
                    max,
                    min_upper,
                    max_upper,
                    value_snap,
                );
                state.value = SliderValue::Range { lower, upper };
            }
        },
        SliderPart::Segment => {
            if let SliderValue::Range { .. } = state.value {
                let start_lower = state.drag_start_value.lower();
                let start_upper = state.drag_start_value.upper().unwrap_or(max);
                let gap = start_upper - start_lower;
                let lower =
                    snap_value(start_lower + val_delta, min, max, value_snap).clamp(min, max - gap);
                state.value = SliderValue::Range {
                    lower,
                    upper: lower + gap,
                };
            }
        }
    }
}

fn move_segment_center_to(
    state: &mut SliderState,
    value: f32,
    min: f32,
    max: f32,
    value_snap: Option<f32>,
) {
    match state.value {
        SliderValue::Single(_) => {
            state.value = SliderValue::Single(snap_value(value, min, max, value_snap));
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            let new_lower =
                snap_value(value - gap * 0.5, min, max, value_snap).clamp(min, max - gap);
            state.value = SliderValue::Range {
                lower: new_lower,
                upper: new_lower + gap,
            };
        }
    }
}

fn step_active_value(
    state: &mut SliderState,
    delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) {
    match state.value {
        SliderValue::Single(lower) => {
            state.value = SliderValue::Single(snap_value(lower + delta, min, max, value_snap));
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            let new_lower = snap_value(lower + delta, min, max, value_snap).clamp(min, max - gap);
            state.value = SliderValue::Range {
                lower: new_lower,
                upper: new_lower + gap,
            };
        }
    }
    repair_values(state, min, max, min_gap, max_gap, value_snap);
}

fn page_active_value(
    state: &mut SliderState,
    delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
    value_snap: Option<f32>,
) {
    step_active_value(state, delta, min, max, min_gap, max_gap, value_snap);
}

fn active_min_value(state: &SliderState) -> f32 {
    state.value.lower()
}

fn active_max_value(state: &SliderState) -> f32 {
    state.value.upper().unwrap_or(state.value.lower())
}

fn set_active_min(state: &mut SliderState, min: f32) {
    match state.value {
        SliderValue::Single(_) => {
            state.value = SliderValue::Single(min);
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            state.value = SliderValue::Range {
                lower: min,
                upper: min + gap,
            };
        }
    }
}

fn set_active_max(state: &mut SliderState, min: f32, max: f32) {
    match state.value {
        SliderValue::Single(_) => {
            state.value = SliderValue::Single(max);
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            let new_lower = (max - gap).max(min);
            state.value = SliderValue::Range {
                lower: new_lower,
                upper: max,
            };
        }
    }
}

fn first_interactable_part(style: &SliderStyle, has_upper: bool) -> Option<SliderPart> {
    if style.segment_style.is_some() && has_upper {
        Some(SliderPart::Segment)
    } else if style.lower_thumb_style.is_some() {
        Some(SliderPart::LowerThumb)
    } else if style.upper_thumb_style.is_some() && has_upper {
        Some(SliderPart::UpperThumb)
    } else {
        None
    }
}

fn effective_fill(fill: InteractiveColor, disabled: bool, active: bool, hovered: bool) -> Color {
    if disabled {
        fill.idle
    } else if active {
        fill.dragged
    } else if hovered {
        fill.hovered
    } else {
        fill.idle
    }
}

fn valid_track_marks(track_marks: Option<TrackMarksStyle>) -> Option<TrackMarksStyle> {
    let marks = track_marks?;
    if !marks.value_spacing.is_finite() || marks.value_spacing <= 0.0 {
        return None;
    }
    if !marks.width.is_finite() || marks.width <= 0.0 {
        return None;
    }
    if !marks.length.is_finite() || marks.length <= 0.0 {
        return None;
    }
    Some(TrackMarksStyle {
        gap: if marks.gap.is_finite() {
            marks.gap
        } else {
            0.0
        },
        ..marks
    })
}

#[allow(clippy::too_many_arguments)]
fn draw_track_marks(
    cmds: &mut DrawCommands,
    layer: Layer,
    track_rect: Rect,
    is_vert: bool,
    min: f32,
    max: f32,
    range: f32,
    track_start: f32,
    track_len: f32,
    track_marks: Option<TrackMarksStyle>,
    tint: &impl Fn(Color) -> Color,
) {
    const MAX_TRACK_MARKS: usize = 512;
    let Some(marks) = valid_track_marks(track_marks) else {
        return;
    };
    if !range.is_finite() || range < 0.0 {
        return;
    }

    let spacing_count = if range == 0.0 {
        0
    } else {
        (range / marks.value_spacing).floor() as usize
    };
    let lands_on_max = range == 0.0
        || (min + spacing_count as f32 * marks.value_spacing - max).abs() <= f32::EPSILON;
    let mark_count = spacing_count + 1 + usize::from(!lands_on_max);
    if mark_count > MAX_TRACK_MARKS {
        return;
    }

    for i in 0..mark_count {
        let value = if i <= spacing_count {
            min + i as f32 * marks.value_spacing
        } else {
            max
        };
        let coord = value_to_coord(value, min, range, track_start, track_len);
        let rect = if is_vert {
            Rect::new(
                track_rect.x + track_rect.w + marks.gap,
                coord - marks.width * 0.5,
                marks.length,
                marks.width,
            )
        } else {
            Rect::new(
                coord - marks.width * 0.5,
                track_rect.y + track_rect.h + marks.gap,
                marks.width,
                marks.length,
            )
        };
        cmds.push_crisp_fill_rect(rect, tint(marks.color), layer.get_z());
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_track_region(
    cmds: &mut DrawCommands,
    layer: Layer,
    track_rect: Rect,
    is_vert: bool,
    start: f32,
    end: f32,
    stroke: Stroke,
    tint: &impl Fn(Color) -> Color,
) {
    let len = (end - start).max(0.0);
    if len <= 0.0 {
        return;
    }
    if !stroke.is_visible() {
        return;
    }
    let should_snap = (cmds.physical_pixels_per_logical_pixel() - 1.0).abs() > 0.001;
    let thickness = stroke.width;
    let rect = if is_vert {
        let thickness = if should_snap {
            cmds.snap_length_to_physical_pixels(thickness)
        } else {
            thickness
        };
        let x = cmds.snap_to_physical_pixel(track_rect.x + (track_rect.w - thickness) * 0.5);
        Rect::new(x, start, thickness, len)
    } else {
        let thickness = if should_snap {
            cmds.snap_length_to_physical_pixels(thickness)
        } else {
            thickness
        };
        let y = cmds.snap_to_physical_pixel(track_rect.y + (track_rect.h - thickness) * 0.5);
        Rect::new(start, y, len, thickness)
    };
    cmds.push_crisp_fill_rect(rect, tint(stroke.color), layer.get_z());
}

fn draw_separator_line(
    cmds: &mut DrawCommands,
    layer: Layer,
    rect: Rect,
    is_vert: bool,
    separator: Option<Stroke>,
    tint: &impl Fn(Color) -> Color,
) {
    let Some(separator) = separator else {
        return;
    };

    if !separator.is_visible() {
        return;
    }

    let rule = if is_vert {
        Rect::new(rect.x, rect.y, separator.width, rect.h)
    } else {
        Rect::new(rect.x, rect.y, rect.w, separator.width)
    };
    cmds.push_crisp_fill_rect(rule, tint(separator.color), layer.get_z());
}

#[allow(clippy::too_many_arguments)]
fn draw_thumb(
    cmds: &mut DrawCommands,
    layer: Layer,
    rect: Option<Rect>,
    style: Option<ThumbStyle>,
    active: bool,
    disabled: bool,
    hovered: bool,
    tint: &impl Fn(Color) -> Color,
) {
    let Some(rect) = rect else {
        return;
    };
    let Some(style) = style else {
        return;
    };
    let fill = effective_fill(style.fill, disabled, active, hovered);
    cmds.push_crisp_fill_rect(rect, tint(fill), layer.get_z());
    if let Some(border) = style.border {
        let border_color = if !disabled && active {
            style.fill.dragged
        } else {
            border.color
        };
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_crisp_border_rect(
            rect,
            Some(tint_stroke(Stroke::new(border_color, border.width))),
            BorderPlacement::Inside,
            layer.get_z(),
        );
    }
}

/// Policy determining how a slider/scrollbar claims mouse wheel and keyboard paging events,
/// controlling whether events propagate (bubble) to parent scroll containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScrollClaimPolicy {
    /// Always claim all scroll directions (both axes) from the hover/focus system —
    /// even when the slider is at its minimum or maximum limit. This prevents any scroll
    /// events from propagating to a parent scroll area.
    ///
    /// This is the default for standalone sliders.
    #[default]
    ClaimAllDirections,

    /// Only claim scroll directions that can still adjust the slider's value. When the
    /// slider reaches its limit on the active axis (minimum or maximum), it yields further
    /// scrolling on that axis, allowing the event to propagate (bubble) to a parent
    /// scroll area. Other axes are unconditionally claimed to prevent cross-axis scrolling.
    ///
    /// Typically used for scrollbars inside a scroll area so that when the content is fully
    /// scrolled, the parent can hand off the scroll event to an outer container.
    YieldSameAxisAtEnds,
}

/// Visual parts of the slider.
///
/// The track is always drawn in two stroked regions:
///
/// With no upper endpoint:
///
/// ```text
/// min ---- before_stroke ---- value ---- after_stroke ---- max
/// ```
///
/// With an upper endpoint:
///
/// ```text
/// min -- before_stroke -- lower == segment == upper -- after_stroke -- max
/// ```
///
/// `background_fill`, when present, is drawn behind the whole slider rect.
/// This is mainly useful for scrollbar gutters.
///
/// `separator_line`, when present, is drawn across the whole slider rect as an
/// orientation-aware edge:
///
/// ```text
/// vertical:   left edge
/// horizontal: top edge
/// ```
///
/// The lower thumb, upper thumb, and segment are independently optional.
/// If a part's style is `None`, that part is not drawn and should not be
/// independently interactable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    /// Optional fill behind the whole slider rect.
    ///
    /// Used by scrollbar-like sliders to paint the gutter/background behind
    /// the segment, including any segment margin.
    ///
    /// Drawn before track strokes, segment, thumbs, separator line, and focus.
    pub background_fill: Option<Color>,

    /// Stroke for the track region from min → lower.
    pub before_stroke: Option<Stroke>,

    /// Stroke for the track region from upper/lower → max.
    pub after_stroke: Option<Stroke>,

    /// Optional bar drawn between lower and upper.
    ///
    /// Only drawn when `state.value` is `SliderValue::Range`.
    pub segment_style: Option<SegmentStyle>,

    /// Optional thumb drawn at lower.
    pub lower_thumb_style: Option<ThumbStyle>,

    /// Optional thumb drawn at upper.
    ///
    /// Only drawn when `state.value` is `SliderValue::Range`.
    pub upper_thumb_style: Option<ThumbStyle>,

    /// Orientation-aware separator line for scrollbar-like sliders.
    ///
    /// Drawn across the whole slider rect, independent of lower/upper/segment:
    ///
    /// - vertical slider: line on the left edge
    /// - horizontal slider: line on the top edge
    ///
    /// This restores the old scrollbar/content separator behaviour without
    /// making before/after track regions responsible for the whole scrollbar edge.
    pub separator_line: Option<Stroke>,

    /// Optional non-interactive visual marks drawn after the track.
    pub track_marks: Option<TrackMarksStyle>,

    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackMarksStyle {
    /// Distance between marks in slider value/domain units.
    pub value_spacing: f32,

    /// Mark colour.
    pub color: Color,

    /// Mark thickness along the slider's main axis.
    pub width: f32,

    /// Mark length along the slider's cross axis.
    pub length: f32,

    /// Space between the slider track/thumb area and the marks.
    pub gap: f32,
}

impl TrackMarksStyle {
    pub fn from_theme(theme: &crate::theme::Theme, value_spacing: f32) -> Self {
        Self {
            value_spacing,
            color: theme.line_on_paper,
            width: 2.0,
            length: 4.0,
            gap: 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentStyle {
    pub cross_axis_size: CrossAxisSize,
    pub fill: InteractiveColor,
    pub border: Option<Stroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbStyle {
    pub cross_axis_size: CrossAxisSize,
    pub main_axis_length: f32,
    pub fill: InteractiveColor,
    pub border: Option<Stroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CrossAxisSize {
    FixedCentered(f32),
    FillTrack { margin: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InteractiveColor {
    pub idle: Color,
    pub hovered: Color,
    pub dragged: Color,
}

impl SliderStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        let default_thumb = ThumbStyle {
            cross_axis_size: CrossAxisSize::FixedCentered(12.0),
            main_axis_length: 12.0,
            fill: InteractiveColor {
                idle: theme.paper_elev,
                hovered: theme.paper_elev_hover,
                dragged: theme.rust,
            },
            border: Some(Stroke::new(theme.ink, 1.0)),
        };

        Self {
            background_fill: None,
            before_stroke: Some(Stroke::new(theme.ink, 1.5)),
            after_stroke: Some(Stroke::new(theme.line_on_paper, 1.5)),
            segment_style: None,
            lower_thumb_style: Some(default_thumb),
            upper_thumb_style: None,
            separator_line: None,
            track_marks: None,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.32,
        }
    }

    pub fn range_from_theme(theme: &crate::theme::Theme) -> Self {
        let default_thumb = ThumbStyle {
            cross_axis_size: CrossAxisSize::FixedCentered(12.0),
            main_axis_length: 11.0, // Odd so that there's a symmetrical single-pixel centreline to use for a marker when min == max
            fill: InteractiveColor {
                idle: theme.paper_elev,
                hovered: theme.paper_elev_hover,
                dragged: theme.rust,
            },
            border: Some(Stroke::new(theme.ink, 1.0)),
        };

        Self {
            background_fill: None,
            before_stroke: Some(Stroke::new(theme.line_on_paper, 1.5)),
            after_stroke: Some(Stroke::new(theme.line_on_paper, 1.5)),
            segment_style: Some(SegmentStyle {
                cross_axis_size: CrossAxisSize::FixedCentered(1.5),
                fill: InteractiveColor {
                    idle: theme.ink,
                    hovered: Color::BLACK,
                    dragged: theme.rust,
                },
                border: None,
            }),
            lower_thumb_style: Some(default_thumb),
            upper_thumb_style: Some(default_thumb),
            separator_line: None,
            track_marks: None,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.32,
        }
    }

    pub fn scrollbar_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background_fill: Some(theme.scrollbar_track_on_paper),
            before_stroke: None,
            after_stroke: None,
            segment_style: Some(SegmentStyle {
                cross_axis_size: CrossAxisSize::FillTrack { margin: 1.0 },
                fill: InteractiveColor {
                    idle: theme.ink,
                    hovered: Color::BLACK,
                    dragged: theme.rust,
                },
                border: None,
            }),
            lower_thumb_style: None,
            upper_thumb_style: None,
            separator_line: Some(Stroke::new(theme.line_soft_on_paper, 1.0)),
            track_marks: None,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.4,
        }
    }
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PagingDirection {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SliderPart {
    LowerThumb,
    UpperThumb,
    Segment,
}

// ── Slider Value ─────────────────────────────────────────────────────────────

/// Slider values are modelled as either a single point or an interval.
///
/// Single-value slider:
///
/// ```text
/// min -------- value ---------------- max
///                ^
/// ```
///
/// Range/span/scrollbar-like slider:
///
/// ```text
/// min ---- lower ======== upper ----- max
///           ^              ^
///         start           end
/// ```
///
/// `Single` represents one value. `Range` represents the interval
/// `lower..upper`. Fixed-size spans are represented by `Range` together with
/// equal `min_gap` and `max_gap` constraints on `SliderSpec`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SliderValue {
    Single(f32),
    Range { lower: f32, upper: f32 },
}

impl Default for SliderValue {
    fn default() -> Self {
        SliderValue::Single(0.0)
    }
}

impl SliderValue {
    pub fn single(value: f32) -> Self {
        SliderValue::Single(value)
    }
    pub fn range(lower: f32, upper: f32) -> Self {
        SliderValue::Range { lower, upper }
    }

    pub fn lower(self) -> f32 {
        match self {
            SliderValue::Single(v) => v,
            SliderValue::Range { lower, .. } => lower,
        }
    }
    pub fn upper(self) -> Option<f32> {
        match self {
            SliderValue::Single(_) => None,
            SliderValue::Range { upper, .. } => Some(upper),
        }
    }
    pub fn is_range(self) -> bool {
        match self {
            SliderValue::Single(_) => false,
            SliderValue::Range { .. } => true,
        }
    }

    pub fn as_pair(self) -> (f32, Option<f32>) {
        match self {
            SliderValue::Single(v) => (v, None),
            SliderValue::Range { lower, upper } => (lower, Some(upper)),
        }
    }
    pub fn from_pair(lower: f32, upper: Option<f32>) -> Self {
        if let Some(upper) = upper {
            SliderValue::Range { lower, upper }
        } else {
            SliderValue::Single(lower)
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SliderState {
    pub value: SliderValue,
    pub focus_id: FocusId,
    pub active_part: Option<SliderPart>,
    pub drag_start_value: SliderValue,
    pub is_track_clicking: bool,
    pub press_drag: PressDragState,
    pub repeat_timer: RepeatTimer,
    pub track_click_direction: Option<PagingDirection>,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SliderResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

/// Slider configuration.
///
/// Values are interpreted over a 1D domain from `min` to `max`. A state with
/// `state.value` is `SliderValue::Single` for a point slider; `SliderValue::Range` is an
/// interval slider and may be constrained by `min_gap` and `max_gap`.
#[derive(Debug, Clone, PartialEq)]
pub struct SliderSpec {
    /// Inclusive lower bound of the slider domain.
    pub min: f32,
    /// Inclusive upper bound of the slider domain.
    pub max: f32,
    /// Minimum allowed distance between `lower` and `upper`.
    ///
    /// Ignored when `state.value` is `SliderValue::Single`.
    /// If both `min_gap` and `max_gap` are set to the same value, the slider
    /// represents a fixed-size span that can be offset but not resized.
    pub min_gap: Option<f32>,
    /// Maximum allowed distance between `lower` and `upper`.
    ///
    /// Ignored when `state.value` is `SliderValue::Single`.
    /// If both `min_gap` and `max_gap` are set to the same value, the slider
    /// represents a fixed-size span that can be offset but not resized.
    pub max_gap: Option<f32>,
    pub value_snap: Option<f32>,
    pub page_step: f32,
    pub step: f32,
    pub orientation: Orientation,
    pub style: SliderStyle,
    pub scroll_claim: ScrollClaimPolicy,
    pub disabled: bool,
    pub keyboard_focusable: bool,
}

impl Default for SliderSpec {
    fn default() -> Self {
        Self {
            min: 0.0,
            max: 100.0,
            min_gap: None,
            max_gap: None,
            value_snap: None,
            page_step: 10.0,
            step: 1.0,
            orientation: Orientation::Horizontal,
            style: SliderStyle::default(),
            scroll_claim: ScrollClaimPolicy::ClaimAllDirections,
            disabled: false,
            keyboard_focusable: true,
        }
    }
}

impl SliderSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = SliderStyle::from_theme(theme);
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

    pub fn min_gap(mut self, min_gap: f32) -> Self {
        self.min_gap = Some(min_gap);
        self
    }

    pub fn max_gap(mut self, max_gap: f32) -> Self {
        self.max_gap = Some(max_gap);
        self
    }

    pub fn value_snap(mut self, value_snap: Option<f32>) -> Self {
        self.value_snap = value_snap;
        self
    }

    pub fn page_step(mut self, page_step: f32) -> Self {
        self.page_step = page_step;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = orientation;
        self
    }

    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
    }

    pub fn scroll_claim(mut self, scroll_claim: ScrollClaimPolicy) -> Self {
        self.scroll_claim = scroll_claim;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn keyboard_focusable(mut self, keyboard_focusable: bool) -> Self {
        self.keyboard_focusable = keyboard_focusable;
        self
    }
}

pub type DefaultSliderValueFormatter = fn(SliderValue) -> String;

pub fn default_slider_value_formatter(value: SliderValue) -> String {
    match value {
        SliderValue::Single(v) => format!("{v:.2}"),
        SliderValue::Range { lower, upper } => format!("{lower:.2}–{upper:.2}"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueLabelledSliderSpec<F = DefaultSliderValueFormatter>
where
    F: Fn(SliderValue) -> String,
{
    pub slider: SliderSpec,
    pub value_formatter: F,
    pub label_style: crate::widgets::label::LabelStyle,
    pub gap: f32,
}

impl Default for ValueLabelledSliderSpec<DefaultSliderValueFormatter> {
    fn default() -> Self {
        Self {
            slider: SliderSpec::default(),
            value_formatter: default_slider_value_formatter,
            label_style: crate::widgets::label::LabelStyle::default(),
            gap: 8.0,
        }
    }
}

impl ValueLabelledSliderSpec<DefaultSliderValueFormatter> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::new().theme(theme)
    }
}

impl<F> ValueLabelledSliderSpec<F>
where
    F: Fn(SliderValue) -> String,
{
    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.slider = self.slider.theme(theme);
        self.label_style = crate::widgets::widget_helpers::trailing_label_style_from_theme(
            theme,
            self.slider.disabled,
            self.slider.style.disabled_alpha,
        );
        self
    }

    pub fn slider(mut self, slider: SliderSpec) -> Self {
        self.slider = slider;
        self
    }

    pub fn value_formatter<G>(self, value_formatter: G) -> ValueLabelledSliderSpec<G>
    where
        G: Fn(SliderValue) -> String,
    {
        ValueLabelledSliderSpec {
            slider: self.slider,
            value_formatter,
            label_style: self.label_style,
            gap: self.gap,
        }
    }

    pub fn label_style(mut self, label_style: crate::widgets::label::LabelStyle) -> Self {
        self.label_style = label_style;
        self
    }

    pub fn gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level slider widget function using `WidgetContext`.
///
/// Consumes a complete `SliderSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn slider<T: TextBackend, S: LayoutState, CF>(
    spec: SliderSpec,
    layout_params: S::Params,
    state: &mut SliderState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SliderResult {
    let pre_layout_spec = raw::SliderPreLayoutSpec {
        orientation: spec.orientation,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_slider(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SliderSpec {
        rect,
        min: spec.min,
        max: spec.max,
        min_gap: spec.min_gap,
        max_gap: spec.max_gap,
        value_snap: spec.value_snap,
        page_step: spec.page_step,
        step: spec.step,
        orientation: spec.orientation,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        scroll_claim: spec.scroll_claim,
        time: ctx.time,
        disabled: spec.disabled,
        keyboard_focusable: spec.keyboard_focusable,
        layer: ctx.layer,
    };

    let result = raw::post_layout_slider(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );
    ctx.request_cursor(result.cursor_icon);
    SliderResult {
        layout: LayoutInfo::tight(rect),
        input: result.input,
        focused: result.focused,
    }
}

pub fn value_labelled_slider<T: TextBackend, S: LayoutState, CF, F>(
    spec: ValueLabelledSliderSpec<F>,
    layout_params: S::Params,
    state: &mut SliderState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> SliderResult
where
    F: Fn(SliderValue) -> String,
{
    let label_text = (spec.value_formatter)(state.value);
    let pre_layout_spec = raw::SliderPreLayoutSpec {
        orientation: spec.slider.orientation,
        style: spec.slider.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_slider(&pre_layout_spec, offer);
    let slider_size = pre_layout.size_request.preferred.unwrap_or(Vec2::ZERO);

    let label_pre_layout_spec = crate::widgets::label::raw::LabelPreLayoutSpec {
        text: &label_text,
        style: spec.label_style,
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
            slider_size,
            label_size,
            spec.gap,
        ),
    );
    let control_size = if spec.slider.orientation == Orientation::Vertical {
        Vec2::new(slider_size.x.min(rect.w).max(0.0), rect.h)
    } else {
        Vec2::new((rect.w - spec.gap.max(0.0) - label_size.x).max(0.0), rect.h)
    };
    let layout = crate::widgets::widget_helpers::layout_trailing_label(
        rect,
        control_size,
        label_size,
        spec.gap,
    );

    if !spec.slider.disabled
        && spec.slider.keyboard_focusable
        && ctx
            .clip_rect
            .is_none_or(|c| c.contains(ctx.input.mouse_pos))
        && layout.label_rect.contains(ctx.input.mouse_pos)
        && ctx.input.mouse_pressed
    {
        ctx.focus_system.take_keyboard_focus(state.focus_id);
    }

    let raw_spec = raw::SliderSpec {
        // Pass only the control rect to the raw slider. The value label is a readout:
        // clicking it may focus the slider, but it must not page, drag, or wheel-adjust the value.
        rect: layout.control_rect,
        min: spec.slider.min,
        max: spec.slider.max,
        min_gap: spec.slider.min_gap,
        max_gap: spec.slider.max_gap,
        value_snap: spec.slider.value_snap,
        page_step: spec.slider.page_step,
        step: spec.slider.step,
        orientation: spec.slider.orientation,
        style: spec.slider.style,
        clip_rect: ctx.clip_rect,
        scroll_claim: spec.slider.scroll_claim,
        time: ctx.time,
        disabled: spec.slider.disabled,
        keyboard_focusable: spec.slider.keyboard_focusable,
        layer: ctx.layer,
    };

    let result = raw::post_layout_slider(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );
    ctx.request_cursor(result.cursor_icon);

    crate::widgets::widget_helpers::draw_trailing_label(
        layout.label_rect,
        &label_text,
        spec.label_style,
        label_pre_layout,
        ctx.layer,
        ctx.text_backend,
        ctx.cmds,
    );

    SliderResult {
        layout: LayoutInfo::new(layout.outer_rect, layout.control_rect),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "slider_tests.rs"]
mod tests;
