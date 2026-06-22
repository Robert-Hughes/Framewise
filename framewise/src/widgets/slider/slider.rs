use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
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
    pub struct SliderPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderResult {
        pub input: InputInfo,
        pub focused: bool,
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
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// High-level wrappers should use this internally.
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
        repair_values(state, min, max, spec.min_gap, spec.max_gap);

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        let is_vert = spec.orientation == Orientation::Vertical;

        let (main_start_padding, main_end_padding) = if !state.value.is_range() {
            if let Some(lower_style) = spec.style.lower_thumb_style {
                let len = main_axis_len(lower_style.cross_axis, spec.rect, is_vert);
                (len * 0.5, len * 0.5)
            } else {
                (0.0, 0.0)
            }
        } else {
            let start_pad = if let Some(lower_style) = spec.style.lower_thumb_style {
                main_axis_len(lower_style.cross_axis, spec.rect, is_vert) * 0.5
            } else {
                0.0
            };
            let end_pad = if let Some(upper_style) = spec.style.upper_thumb_style {
                main_axis_len(upper_style.cross_axis, spec.rect, is_vert) * 0.5
            } else {
                0.0
            };
            (start_pad, end_pad)
        };

        let track_rect = if is_vert {
            let new_y = spec.rect.y + main_start_padding;
            let new_h = (spec.rect.h - (main_start_padding + main_end_padding)).max(0.0);
            Rect::new(spec.rect.x, new_y, spec.rect.w, new_h)
        } else {
            let new_x = spec.rect.x + main_start_padding;
            let new_w = (spec.rect.w - (main_start_padding + main_end_padding)).max(0.0);
            Rect::new(new_x, spec.rect.y, new_w, spec.rect.h)
        };

        let track_len = if is_vert { track_rect.h } else { track_rect.w };
        let track_start = if is_vert { track_rect.y } else { track_rect.x };
        let lower_coord = value_to_coord(state.value.lower(), min, range, track_start, track_len);
        let upper_coord = state
            .value
            .upper()
            .map(|upper| value_to_coord(upper, min, range, track_start, track_len));
        let lower_thumb_rect = spec
            .style
            .lower_thumb_style
            .map(|style| thumb_rect(track_rect, is_vert, lower_coord, style));
        let upper_thumb_rect = spec
            .style
            .upper_thumb_style
            .zip(upper_coord)
            .map(|(style, coord)| thumb_rect(track_rect, is_vert, coord, style));
        let segment_rect = spec
            .style
            .segment_style
            .zip(upper_coord)
            .map(|(style, coord)| segment_rect(track_rect, is_vert, lower_coord, coord, style));

        let hit_part = hit_test_parts(
            input.mouse_pos,
            lower_thumb_rect,
            upper_thumb_rect,
            segment_rect,
        );

        let pointer_over_slider = track_rect.contains(input.mouse_pos) || hit_part.is_some();
        let pointer_over_wheel_area = spec.rect.contains(input.mouse_pos);

        let focused = if spec.disabled || !spec.keyboard_focusable {
            false
        } else {
            focus_system.register_keyboard(state.focus_id, spec.rect, spec.clip_rect)
        };

        if !spec.disabled && (pointer_over_slider || pointer_over_wheel_area) && is_visible {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = !spec.disabled && focus_system.is_hover_active(state.focus_id);

        // 2. Input Handling
        if !spec.disabled {
            // Drag release
            if state.active_part.is_some() && !input.mouse_down {
                state.active_part = None;
            }

            // Drag update
            if let Some(active_part) = state.active_part {
                let mouse_coord = if is_vert {
                    input.mouse_pos.y
                } else {
                    input.mouse_pos.x
                };
                let delta = mouse_coord - state.drag_start_mouse_coord;
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
                );
            }

            // Track click release
            if state.is_track_clicking && !input.mouse_down {
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Track click → drag transition: mouse moved past threshold
            let mouse_coord = if is_vert {
                input.mouse_pos.y
            } else {
                input.mouse_pos.x
            };
            const TRACK_DRAG_THRESHOLD: f32 = 4.0;
            if state.is_track_clicking
                && input.mouse_down
                && (mouse_coord - state.track_click_start_coord).abs() > TRACK_DRAG_THRESHOLD
            {
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
                        SliderPart::Segment => move_segment_center_to(state, value, min, max),
                    }
                    repair_values(state, min, max, spec.min_gap, spec.max_gap);
                    state.drag_start_mouse_coord = mouse_coord;
                    state.drag_start_value = state.value;
                    state.active_part = Some(part);
                }
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Mouse wheel scrolling — suppressed during an active drag so that drag
            // motion is authoritative (otherwise wheel ticks would stack on top of
            // the drag-projected value).
            if is_visible && state.active_part.is_none() && pointer_over_wheel_area {
                let at_min = active_min_value(state) <= min;
                let at_max = active_max_value(state) >= max;

                match spec.scroll_claim {
                    ScrollClaimPolicy::ClaimAllDirections => {
                        focus_system.claim_scroll_up(state.focus_id);
                        focus_system.claim_scroll_down(state.focus_id);
                        focus_system.claim_scroll_left(state.focus_id);
                        focus_system.claim_scroll_right(state.focus_id);
                    }
                    ScrollClaimPolicy::YieldSameAxisAtEnds => {
                        if is_vert {
                            // Vertical slider:
                            // Conditionally claim vertical scrolling to allow same-axis bubbling.
                            if !at_min {
                                focus_system.claim_scroll_up(state.focus_id);
                            }
                            if !at_max {
                                focus_system.claim_scroll_down(state.focus_id);
                            }
                            // Unconditionally claim horizontal scrolling to isolate from the horizontal axis.
                            focus_system.claim_scroll_left(state.focus_id);
                            focus_system.claim_scroll_right(state.focus_id);
                        } else {
                            // Horizontal slider:
                            // Conditionally claim horizontal scrolling to allow same-axis bubbling.
                            if !at_min {
                                focus_system.claim_scroll_left(state.focus_id);
                            }
                            if !at_max {
                                focus_system.claim_scroll_right(state.focus_id);
                            }
                            // Unconditionally claim vertical scrolling to isolate from the vertical axis.
                            focus_system.claim_scroll_up(state.focus_id);
                            focus_system.claim_scroll_down(state.focus_id);
                        }
                    }
                }

                if is_hover_active {
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
                        focus_system.is_active_scroll_up(state.focus_id)
                    } else {
                        focus_system.is_active_scroll_left(state.focus_id)
                            || focus_system.is_active_scroll_up(state.focus_id)
                    };
                    let is_active_down_right = if is_vert {
                        focus_system.is_active_scroll_down(state.focus_id)
                    } else {
                        focus_system.is_active_scroll_right(state.focus_id)
                            || focus_system.is_active_scroll_down(state.focus_id)
                    };

                    if scroll_delta > 0.0 && is_active_up_left {
                        step_active_value(
                            state,
                            -scroll_delta * spec.step,
                            min,
                            max,
                            spec.min_gap,
                            spec.max_gap,
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
                        );
                    }
                }
            }

            // Track click (mouse down on track, not on thumb)
            let active_start = lower_coord.min(upper_coord.unwrap_or(lower_coord));
            let active_end = upper_coord.unwrap_or(lower_coord).max(lower_coord);
            if is_visible
                && is_hover_active
                && input.mouse_pressed
                && hit_part.is_none()
                && track_rect.contains(input.mouse_pos)
            {
                if spec.keyboard_focusable {
                    focus_system.take_keyboard_focus(state.focus_id);
                }
                state.is_track_clicking = true;
                state.track_click_start_coord = mouse_coord;
                state.next_repeat_time = spec.time + 0.5;
                // Page up/down towards mouse
                if mouse_coord < active_start {
                    state.track_click_direction = Some(PagingDirection::Up);
                    page_active_value(state, -spec.page_step, min, max, spec.min_gap, spec.max_gap);
                } else if mouse_coord > active_end {
                    state.track_click_direction = Some(PagingDirection::Down);
                    page_active_value(state, spec.page_step, min, max, spec.min_gap, spec.max_gap);
                }
            }

            // Drag start
            if is_visible && is_hover_active && input.mouse_pressed {
                if let Some(part) = hit_part {
                    if spec.keyboard_focusable {
                        focus_system.take_keyboard_focus(state.focus_id);
                    }
                    state.active_part = Some(part);
                    state.drag_start_mouse_coord = mouse_coord;
                    state.drag_start_value = state.value;
                }
            }

            // Track click repeat logic (time-based paging)
            if state.is_track_clicking && spec.time >= state.next_repeat_time {
                if track_rect.contains(input.mouse_pos) {
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
                                );
                                if !state.value.is_range() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    let val = state.value.lower().max(cursor_val);
                                    state.value = SliderValue::Single(val);
                                }
                                state.next_repeat_time = spec.time + 0.05;
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
                                );
                                if !state.value.is_range() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    let val = state.value.lower().min(cursor_val);
                                    state.value = SliderValue::Single(val);
                                }
                                state.next_repeat_time = spec.time + 0.05;
                            }
                        }
                        None => {}
                    }
                    // else: cursor is now inside the thumb; paging stops but keep
                    // is_track_clicking=true so the drag-transition check can still fire.
                } else {
                    state.is_track_clicking = false;
                    state.track_click_direction = None;
                }
            }

            // Keyboard handling
            if focused {
                let at_min = active_min_value(state) <= min;
                let at_max = active_max_value(state) >= max;

                match spec.scroll_claim {
                    ScrollClaimPolicy::ClaimAllDirections => {
                        if is_vert {
                            focus_system.claim_pgup_vert(state.focus_id);
                            focus_system.claim_pgdn_vert(state.focus_id);
                            focus_system.claim_pgup_horiz(state.focus_id);
                            focus_system.claim_pgdn_horiz(state.focus_id);
                        } else {
                            focus_system.claim_pgup_horiz(state.focus_id);
                            focus_system.claim_pgdn_horiz(state.focus_id);
                            focus_system.claim_pgup_vert(state.focus_id);
                            focus_system.claim_pgdn_vert(state.focus_id);
                        }
                    }
                    ScrollClaimPolicy::YieldSameAxisAtEnds => {
                        if is_vert {
                            if !at_min {
                                focus_system.claim_pgup_vert(state.focus_id);
                            }
                            if !at_max {
                                focus_system.claim_pgdn_vert(state.focus_id);
                            }
                            focus_system.claim_pgup_horiz(state.focus_id);
                            focus_system.claim_pgdn_horiz(state.focus_id);
                        } else {
                            if !at_min {
                                focus_system.claim_pgup_horiz(state.focus_id);
                            }
                            if !at_max {
                                focus_system.claim_pgdn_horiz(state.focus_id);
                            }
                            focus_system.claim_pgup_vert(state.focus_id);
                            focus_system.claim_pgdn_vert(state.focus_id);
                        }
                    }
                }

                let is_active_pgup = if is_vert {
                    focus_system.is_active_pgup_vert(state.focus_id)
                } else {
                    focus_system.is_active_pgup_horiz(state.focus_id)
                };
                let is_active_pgdn = if is_vert {
                    focus_system.is_active_pgdn_vert(state.focus_id)
                } else {
                    focus_system.is_active_pgdn_horiz(state.focus_id)
                };

                if input.key_pressed_page_up && is_active_pgup {
                    step_active_value(state, -spec.page_step, min, max, spec.min_gap, spec.max_gap);
                }
                if input.key_pressed_page_down && is_active_pgdn {
                    step_active_value(state, spec.page_step, min, max, spec.min_gap, spec.max_gap);
                }
                if input.key_pressed_up || input.key_pressed_left {
                    step_active_value(state, -spec.step, min, max, spec.min_gap, spec.max_gap);
                }
                if input.key_pressed_down || input.key_pressed_right {
                    step_active_value(state, spec.step, min, max, spec.min_gap, spec.max_gap);
                }
                if input.key_pressed_home {
                    set_active_min(state, min);
                    repair_values(state, min, max, spec.min_gap, spec.max_gap);
                }
                if input.key_pressed_end {
                    set_active_max(state, min, max);
                    repair_values(state, min, max, spec.min_gap, spec.max_gap);
                }

                // Slider owns all four arrow keys for value adjustment; only Tab navigates focus.
                focus_system.handle_keyboard_traversal(
                    focused,
                    input,
                    crate::focus::FocusTraversalKeys::tab_only(),
                );
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
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: spec.rect,
                color: tint(background_fill),
                z: spec.layer.get_z(),
            });
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

        if let Some((style, rect)) = spec.style.segment_style.zip(segment_rect) {
            let segment_is_hovered = !spec.disabled && rect.contains(input.mouse_pos) && is_visible;
            let fill = effective_fill(
                style.fill,
                spec.disabled,
                state.active_part == Some(SliderPart::Segment),
                segment_is_hovered,
            );
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect,
                color: tint(fill),
                z: spec.layer.get_z(),
            });
            if let Some(border) = style.border {
                let border_color =
                    if !spec.disabled && state.active_part == Some(SliderPart::Segment) {
                        style.fill.dragged
                    } else {
                        border.color
                    };
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_stroke_rect(
                    rect,
                    Some(tint_stroke(Stroke::new(border_color, border.width))),
                    spec.layer.get_z(),
                    false,
                );
            }
        }

        draw_thumb(
            cmds,
            spec.layer,
            lower_thumb_rect,
            spec.style.lower_thumb_style,
            state.active_part == Some(SliderPart::LowerThumb),
            spec.disabled,
            is_visible,
            input.mouse_pos,
            &tint,
        );
        draw_thumb(
            cmds,
            spec.layer,
            upper_thumb_rect,
            spec.style.upper_thumb_style,
            state.active_part == Some(SliderPart::UpperThumb),
            spec.disabled,
            is_visible,
            input.mouse_pos,
            &tint,
        );

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
                cmds.push_stroke_rect(
                    spec.rect.inset(-(outline.offset + outline.stroke.width)),
                    Some(tint_stroke(outline.stroke)),
                    spec.layer.get_focus_z(),
                    false,
                );
            }
        }

        SliderResult {
            focused,
            input: InputInfo {
                hovered: !spec.disabled && pointer_over_slider && is_visible && is_hover_active,
                pressed: !spec.disabled && (state.active_part.is_some() || state.is_track_clicking),
                clicked: false,
            },
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

fn main_axis_len(cross_axis: ThumbCrossAxis, track_rect: Rect, is_vert: bool) -> f32 {
    match cross_axis {
        ThumbCrossAxis::FixedCentered(len) => len,
        ThumbCrossAxis::FillTrack { margin } => {
            let cross = if is_vert { track_rect.w } else { track_rect.h };
            (cross - margin * 2.0).max(1.0)
        }
    }
}

fn cross_axis_rect(
    track_rect: Rect,
    is_vert: bool,
    main_start: f32,
    main_len: f32,
    cross_axis: ThumbCrossAxis,
) -> Rect {
    let track_cross_size = if is_vert { track_rect.w } else { track_rect.h };
    match cross_axis {
        ThumbCrossAxis::FixedCentered(cross_len) => {
            let half = cross_len * 0.5;
            let center = if is_vert {
                track_rect.x + track_rect.w * 0.5
            } else {
                track_rect.y + track_rect.h * 0.5
            };
            if is_vert {
                Rect::new(center - half, main_start, cross_len, main_len)
            } else {
                Rect::new(main_start, center - half, main_len, cross_len)
            }
        }
        ThumbCrossAxis::FillTrack { margin } => {
            let cross = (track_cross_size - margin * 2.0).max(1.0);
            if is_vert {
                Rect::new(track_rect.x + margin, main_start, cross, main_len)
            } else {
                Rect::new(main_start, track_rect.y + margin, main_len, cross)
            }
        }
    }
}

fn thumb_rect(track_rect: Rect, is_vert: bool, coord: f32, style: ThumbStyle) -> Rect {
    let len = main_axis_len(style.cross_axis, track_rect, is_vert);
    cross_axis_rect(
        track_rect,
        is_vert,
        coord - len * 0.5,
        len,
        style.cross_axis,
    )
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
    cross_axis_rect(track_rect, is_vert, start, len, style.cross_axis)
}

fn effective_gaps(min: f32, max: f32, min_gap: Option<f32>, max_gap: Option<f32>) -> (f32, f32) {
    let domain = max - min;
    let min_gap = min_gap.unwrap_or(0.0).clamp(0.0, domain);
    let max_gap = max_gap.unwrap_or(domain).clamp(min_gap, domain);
    (min_gap, max_gap)
}

fn repair_values(
    state: &mut SliderState,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
) {
    match state.value {
        SliderValue::Single(lower) => {
            state.value = SliderValue::Single(lower.clamp(min, max));
        }
        SliderValue::Range {
            mut lower,
            mut upper,
        } => {
            lower = lower.clamp(min, max);
            upper = upper.clamp(min, max);
            if lower > upper {
                core::mem::swap(&mut lower, &mut upper);
            }

            let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
            let gap = (upper - lower).clamp(min_gap_val, max_gap_val);

            upper = (lower + gap).min(max);
            lower = (upper - gap).max(min);
            upper = (lower + gap).min(max);
            state.value = SliderValue::Range { lower, upper };
        }
    }
}

fn apply_drag_delta(
    state: &mut SliderState,
    part: SliderPart,
    val_delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
) {
    match part {
        SliderPart::LowerThumb => match state.value {
            SliderValue::Single(_) => {
                let requested_lower = state.drag_start_value.lower() + val_delta;
                state.value = SliderValue::Single(requested_lower.clamp(min, max));
            }
            SliderValue::Range { upper, .. } => {
                let requested_lower = state.drag_start_value.lower() + val_delta;
                let (min_gap_val, max_gap_val) = effective_gaps(min, max, min_gap, max_gap);
                let min_lower = (upper - max_gap_val).max(min);
                let max_lower = (upper - min_gap_val).min(max);
                let lower = requested_lower.clamp(min_lower, max_lower);
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
                let upper = requested_upper.clamp(min_upper, max_upper);
                state.value = SliderValue::Range { lower, upper };
            }
        },
        SliderPart::Segment => {
            if let SliderValue::Range { .. } = state.value {
                let start_lower = state.drag_start_value.lower();
                let start_upper = state.drag_start_value.upper().unwrap_or(max);
                let gap = start_upper - start_lower;
                let lower = (start_lower + val_delta).clamp(min, max - gap);
                state.value = SliderValue::Range {
                    lower,
                    upper: lower + gap,
                };
            }
        }
    }
}

fn move_segment_center_to(state: &mut SliderState, value: f32, min: f32, max: f32) {
    match state.value {
        SliderValue::Single(_) => {
            state.value = SliderValue::Single(value.clamp(min, max));
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            let new_lower = (value - gap * 0.5).clamp(min, max - gap);
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
) {
    match state.value {
        SliderValue::Single(lower) => {
            state.value = SliderValue::Single((lower + delta).clamp(min, max));
        }
        SliderValue::Range { lower, upper } => {
            let gap = upper - lower;
            let new_lower = (lower + delta).clamp(min, max - gap);
            state.value = SliderValue::Range {
                lower: new_lower,
                upper: new_lower + gap,
            };
        }
    }
    repair_values(state, min, max, min_gap, max_gap);
}

fn page_active_value(
    state: &mut SliderState,
    delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
) {
    step_active_value(state, delta, min, max, min_gap, max_gap);
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

fn hit_test_parts(
    pos: Vec2,
    lower_thumb_rect: Option<Rect>,
    upper_thumb_rect: Option<Rect>,
    segment_rect: Option<Rect>,
) -> Option<SliderPart> {
    if lower_thumb_rect.is_some_and(|rect| rect.contains(pos)) {
        Some(SliderPart::LowerThumb)
    } else if upper_thumb_rect.is_some_and(|rect| rect.contains(pos)) {
        Some(SliderPart::UpperThumb)
    } else if segment_rect.is_some_and(|rect| rect.contains(pos)) {
        Some(SliderPart::Segment)
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
    let thickness = stroke.width;
    let half = thickness * 0.5;
    let rect = if is_vert {
        let cx = track_rect.x + track_rect.w * 0.5;
        Rect::new(cx - half, start, thickness, len)
    } else {
        let cy = track_rect.y + track_rect.h * 0.5;
        Rect::new(start, cy - half, len, thickness)
    };
    cmds.push(DrawCmd::FillRect {
        anti_alias: false,
        rect,
        color: tint(stroke.color),
        z: layer.get_z(),
    });
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

    let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);

    let (p0, p1) = if is_vert {
        (Vec2::new(rect.x, rect.y), Vec2::new(rect.x, rect.bottom()))
    } else {
        (Vec2::new(rect.x, rect.y), Vec2::new(rect.right(), rect.y))
    };

    cmds.push_stroke_line(p0, p1, Some(tint_stroke(separator)), layer.get_z(), false);
}

#[allow(clippy::too_many_arguments)]
fn draw_thumb(
    cmds: &mut DrawCommands,
    layer: Layer,
    rect: Option<Rect>,
    style: Option<ThumbStyle>,
    active: bool,
    disabled: bool,
    is_visible: bool,
    mouse_pos: Vec2,
    tint: &impl Fn(Color) -> Color,
) {
    let Some(rect) = rect else {
        return;
    };
    let Some(style) = style else {
        return;
    };
    let hovered = !disabled && rect.contains(mouse_pos) && is_visible;
    let fill = effective_fill(style.fill, disabled, active, hovered);
    cmds.push(DrawCmd::FillRect {
        anti_alias: false,
        rect,
        color: tint(fill),
        z: layer.get_z(),
    });
    if let Some(border) = style.border {
        let border_color = if !disabled && active {
            style.fill.dragged
        } else {
            border.color
        };
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_stroke_rect(
            rect,
            Some(tint_stroke(Stroke::new(border_color, border.width))),
            layer.get_z(),
            false,
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

    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentStyle {
    pub cross_axis: ThumbCrossAxis,
    pub fill: InteractiveColor,
    pub border: Option<Stroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumbStyle {
    pub cross_axis: ThumbCrossAxis,
    pub fill: InteractiveColor,
    pub border: Option<Stroke>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThumbCrossAxis {
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
            cross_axis: ThumbCrossAxis::FixedCentered(12.0),
            fill: InteractiveColor {
                idle: theme.paper_elev,
                hovered: theme.hover,
                dragged: theme.rust,
            },
            border: Some(Stroke::new(theme.ink, 1.0)),
        };

        Self {
            background_fill: None,
            before_stroke: Some(Stroke::new(theme.ink, 1.5)),
            after_stroke: Some(Stroke::new(theme.line, 1.5)),
            segment_style: None,
            lower_thumb_style: Some(default_thumb),
            upper_thumb_style: None,
            separator_line: None,
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
            cross_axis: ThumbCrossAxis::FixedCentered(12.0),
            fill: InteractiveColor {
                idle: theme.paper_elev,
                hovered: theme.hover,
                dragged: theme.rust,
            },
            border: Some(Stroke::new(theme.ink, 1.0)),
        };

        Self {
            background_fill: None,
            before_stroke: Some(Stroke::new(theme.line, 1.5)),
            after_stroke: Some(Stroke::new(theme.line, 1.5)),
            segment_style: Some(SegmentStyle {
                cross_axis: ThumbCrossAxis::FixedCentered(1.5),
                fill: InteractiveColor {
                    idle: theme.ink,
                    hovered: theme.ink,
                    dragged: theme.rust,
                },
                border: None,
            }),
            lower_thumb_style: Some(default_thumb),
            upper_thumb_style: Some(default_thumb),
            separator_line: None,
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
            background_fill: Some(Color::linear_rgba(
                theme.ink.r,
                theme.ink.g,
                theme.ink.b,
                0.04,
            )),
            before_stroke: None,
            after_stroke: None,
            segment_style: Some(SegmentStyle {
                cross_axis: ThumbCrossAxis::FillTrack { margin: 1.0 },
                fill: InteractiveColor {
                    idle: theme.ink,
                    hovered: theme.rust,
                    dragged: theme.rust,
                },
                border: None,
            }),
            lower_thumb_style: None,
            upper_thumb_style: None,
            separator_line: Some(Stroke::new(theme.line_soft, 1.0)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.4,
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct SliderState {
    pub value: SliderValue,
    pub focus_id: FocusId,
    pub active_part: Option<SliderPart>,
    pub drag_start_mouse_coord: f32,
    pub drag_start_value: SliderValue,
    pub is_track_clicking: bool,
    pub track_click_start_coord: f32,
    pub next_repeat_time: f64,
    pub track_click_direction: Option<PagingDirection>,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            value: SliderValue::default(),
            focus_id: FocusId::default(),
            active_part: None,
            drag_start_mouse_coord: 0.0,
            drag_start_value: SliderValue::default(),
            is_track_clicking: false,
            track_click_start_coord: 0.0,
            next_repeat_time: 0.0,
            track_click_direction: None,
        }
    }
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
    pub page_step: f32,
    pub step: f32,
    pub orientation: Orientation,
    pub style: SliderStyle,
    pub scroll_claim: ScrollClaimPolicy,
    pub disabled: bool,
    pub keyboard_focusable: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SliderSpecBuilder {
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub min_gap: Option<f32>,
    pub max_gap: Option<f32>,
    pub page_step: Option<f32>,
    pub step: Option<f32>,
    pub orientation: Option<Orientation>,
    pub style: Option<SliderStyle>,
    pub scroll_claim: Option<ScrollClaimPolicy>,
    pub disabled: Option<bool>,
    pub keyboard_focusable: Option<bool>,
}

impl SliderSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
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
    pub fn page_step(mut self, page_step: f32) -> Self {
        self.page_step = Some(page_step);
        self
    }
    pub fn step(mut self, step: f32) -> Self {
        self.step = Some(step);
        self
    }
    pub fn orientation(mut self, orientation: Orientation) -> Self {
        self.orientation = Some(orientation);
        self
    }
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn scroll_claim(mut self, scroll_claim: ScrollClaimPolicy) -> Self {
        self.scroll_claim = Some(scroll_claim);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    pub fn keyboard_focusable(mut self, keyboard_focusable: bool) -> Self {
        self.keyboard_focusable = Some(keyboard_focusable);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SliderStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SliderSpec {
        SliderSpec {
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            min_gap: self.min_gap,
            max_gap: self.max_gap,
            page_step: self.page_step.unwrap_or(10.0),
            step: self.step.unwrap_or(1.0),
            orientation: self.orientation.unwrap_or(Orientation::Horizontal),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            scroll_claim: self
                .scroll_claim
                .unwrap_or(ScrollClaimPolicy::ClaimAllDirections),
            disabled: self.disabled.unwrap_or(false),
            keyboard_focusable: self.keyboard_focusable.unwrap_or(true),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level slider widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn slider<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SliderSpecBuilder,
    layout_params: S::Params,
    state: &mut SliderState,
) -> SliderResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::SliderPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_slider(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SliderSpec {
        rect,
        min: spec.min,
        max: spec.max,
        min_gap: spec.min_gap,
        max_gap: spec.max_gap,
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
    SliderResult {
        layout: LayoutInfo::tight(rect),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "slider_tests.rs"]
mod tests;
