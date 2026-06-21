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
        /// Ignored when `upper` is `None`.
        /// If both `min_gap` and `max_gap` are set to the same value, the slider
        /// represents a fixed-size span that can be offset but not resized.
        pub min_gap: Option<f32>,
        /// Maximum allowed distance between `lower` and `upper`.
        ///
        /// Ignored when `upper` is `None`.
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

    /// Low-level slider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
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

        let track_rect = spec.rect;
        let focused = if spec.disabled || !spec.keyboard_focusable {
            false
        } else {
            focus_system.register_keyboard(state.focus_id, track_rect, spec.clip_rect)
        };

        if !spec.disabled && track_rect.contains(input.mouse_pos) && is_visible {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = !spec.disabled && focus_system.is_hover_active(state.focus_id);
        let is_vert = spec.orientation == Orientation::Vertical;

        let track_len = if is_vert { track_rect.h } else { track_rect.w };
        let track_start = if is_vert { track_rect.y } else { track_rect.x };
        let lower_coord = value_to_coord(state.lower, min, range, track_start, track_len);
        let upper_coord = state
            .upper
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
                let part = first_interactable_part(&spec.style, state.upper.is_some());
                if let Some(part) = part {
                    let value = coord_to_value(mouse_coord, min, range, track_start, track_len);
                    match part {
                        SliderPart::LowerThumb => state.lower = value,
                        SliderPart::UpperThumb => state.upper = Some(value),
                        SliderPart::Segment => move_segment_center_to(state, value, min, max),
                    }
                    repair_values(state, min, max, spec.min_gap, spec.max_gap);
                    state.drag_start_mouse_coord = mouse_coord;
                    state.drag_start_lower = state.lower;
                    state.drag_start_upper = state.upper;
                    state.active_part = Some(part);
                }
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Mouse wheel scrolling — suppressed during an active drag so that drag
            // motion is authoritative (otherwise wheel ticks would stack on top of
            // the drag-projected value).
            if is_visible && state.active_part.is_none() && track_rect.contains(input.mouse_pos) {
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
            let hit_part = hit_test_parts(
                input.mouse_pos,
                lower_thumb_rect,
                upper_thumb_rect,
                segment_rect,
            );
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
                    state.drag_start_lower = state.lower;
                    state.drag_start_upper = state.upper;
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
                                if state.upper.is_none() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    state.lower = state.lower.max(cursor_val);
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
                                if state.upper.is_none() {
                                    let cursor_val = coord_to_value(
                                        mouse_coord,
                                        min,
                                        range,
                                        track_start,
                                        track_len,
                                    );
                                    state.lower = state.lower.min(cursor_val);
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

        // Focus outline.
        if focused {
            if let Some(outline) = spec.style.focus {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_stroke_rect(
                    track_rect.inset(-(outline.offset + outline.stroke.width)),
                    Some(tint_stroke(outline.stroke)),
                    spec.layer.get_focus_z(),
                    false,
                );
            }
        }
        draw_track_region(
            cmds,
            spec.layer,
            track_rect,
            is_vert,
            track_start,
            lower_coord,
            spec.style.before_style,
            &tint,
        );
        draw_track_region(
            cmds,
            spec.layer,
            track_rect,
            is_vert,
            upper_coord.unwrap_or(lower_coord),
            track_start + track_len,
            spec.style.after_style,
            &tint,
        );

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

        SliderResult {
            focused,
            input: InputInfo {
                hovered: !spec.disabled
                    && track_rect.contains(input.mouse_pos)
                    && is_visible
                    && is_hover_active,
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

fn repair_values(
    state: &mut SliderState,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
) {
    state.lower = state.lower.clamp(min, max);
    let Some(mut upper) = state.upper else {
        return;
    };

    upper = upper.clamp(min, max);
    if state.lower > upper {
        core::mem::swap(&mut state.lower, &mut upper);
    }

    let domain = max - min;
    let min_gap = min_gap.unwrap_or(0.0).clamp(0.0, domain);
    let max_gap = max_gap.unwrap_or(domain).clamp(min_gap, domain);
    let gap = (upper - state.lower).clamp(min_gap, max_gap);

    upper = (state.lower + gap).min(max);
    state.lower = (upper - gap).max(min);
    upper = (state.lower + gap).min(max);
    state.upper = Some(upper);
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
        SliderPart::LowerThumb => state.lower = state.drag_start_lower + val_delta,
        SliderPart::UpperThumb => {
            if let Some(start_upper) = state.drag_start_upper {
                state.upper = Some(start_upper + val_delta);
            }
        }
        SliderPart::Segment => {
            let Some(start_upper) = state.drag_start_upper else {
                return;
            };
            let gap = start_upper - state.drag_start_lower;
            let lower = (state.drag_start_lower + val_delta).clamp(min, max - gap);
            state.lower = lower;
            state.upper = Some(lower + gap);
        }
    }
    repair_values(state, min, max, min_gap, max_gap);
}

fn move_segment_center_to(state: &mut SliderState, value: f32, min: f32, max: f32) {
    let Some(upper) = state.upper else {
        state.lower = value;
        return;
    };
    let gap = upper - state.lower;
    let lower = (value - gap * 0.5).clamp(min, max - gap);
    state.lower = lower;
    state.upper = Some(lower + gap);
}

fn step_active_value(
    state: &mut SliderState,
    delta: f32,
    min: f32,
    max: f32,
    min_gap: Option<f32>,
    max_gap: Option<f32>,
) {
    if let Some(upper) = state.upper {
        let gap = upper - state.lower;
        let lower = (state.lower + delta).clamp(min, max - gap);
        state.lower = lower;
        state.upper = Some(lower + gap);
    } else {
        state.lower = (state.lower + delta).clamp(min, max);
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
    state.lower
}

fn active_max_value(state: &SliderState) -> f32 {
    state.upper.unwrap_or(state.lower)
}

fn set_active_min(state: &mut SliderState, min: f32) {
    if let Some(upper) = state.upper {
        let gap = upper - state.lower;
        state.lower = min;
        state.upper = Some(min + gap);
    } else {
        state.lower = min;
    }
}

fn set_active_max(state: &mut SliderState, min: f32, max: f32) {
    if let Some(upper) = state.upper {
        let gap = upper - state.lower;
        state.upper = Some(max);
        state.lower = (max - gap).max(min);
    } else {
        state.lower = max;
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
    style: TrackStyle,
    tint: &impl Fn(Color) -> Color,
) {
    let len = (end - start).max(0.0);
    if len <= 0.0 {
        return;
    }
    match style {
        TrackStyle::Line { stroke } => {
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
        TrackStyle::Rect { color, border } => {
            let rect = if is_vert {
                Rect::new(track_rect.x, start, track_rect.w, len)
            } else {
                Rect::new(start, track_rect.y, len, track_rect.h)
            };
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect,
                color: tint(color),
                z: layer.get_z(),
            });
            if let Some(border) = border {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                let (p0, p1) = if is_vert {
                    (Vec2::new(rect.x, rect.y), Vec2::new(rect.x, rect.bottom()))
                } else {
                    (Vec2::new(rect.x, rect.y), Vec2::new(rect.right(), rect.y))
                };
                cmds.push_stroke_line(p0, p1, Some(tint_stroke(border)), layer.get_z(), false);
            }
        }
    }
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
/// The track is always drawn in two regions:
///
/// With no upper endpoint:
///
/// ```text
/// min ---- before_style ---- lower ---- after_style ---- max
/// ```
///
/// With an upper endpoint:
///
/// ```text
/// min -- before_style -- lower == segment == upper -- after_style -- max
/// ```
///
/// The lower thumb, upper thumb, and segment are independently optional.
/// If a part's style is `None`, that part is not drawn and should not be
/// independently interactable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    /// Style for the track region before `lower`.
    pub before_style: TrackStyle,
    /// Style for the track region after `upper`, or after `lower` when
    /// `upper == None`.
    pub after_style: TrackStyle,
    /// Optional bar drawn between `lower` and `upper`.
    ///
    /// Only drawn when `state.upper.is_some()`.
    /// This is the scrollbar thumb/span/range-fill visual.
    pub segment_style: Option<SegmentStyle>,
    /// Optional thumb drawn at `lower`.
    pub lower_thumb_style: Option<ThumbStyle>,
    /// Optional thumb drawn at `upper`.
    ///
    /// Only drawn when `state.upper.is_some()`.
    pub upper_thumb_style: Option<ThumbStyle>,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackStyle {
    Line {
        stroke: Stroke,
    },
    Rect {
        color: Color,
        border: Option<Stroke>,
    },
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
                hovered: theme.paper_elev,
                dragged: theme.rust,
            },
            border: Some(Stroke::new(theme.ink, 1.0)),
        };
        Self {
            before_style: TrackStyle::Line {
                stroke: Stroke::new(theme.ink, 1.5),
            },
            after_style: TrackStyle::Line {
                stroke: Stroke::new(theme.line, 1.5),
            },
            segment_style: None,
            lower_thumb_style: Some(default_thumb),
            upper_thumb_style: None,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.32,
        }
    }

    pub fn scrollbar_from_theme(theme: &crate::theme::Theme) -> Self {
        let gutter_style = TrackStyle::Rect {
            color: Color::linear_rgba(theme.ink.r, theme.ink.g, theme.ink.b, 0.04),
            border: Some(Stroke::new(theme.line_soft, 1.0)),
        };
        Self {
            before_style: gutter_style,
            after_style: gutter_style,
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

// ── State ─────────────────────────────────────────────────────────────────────

/// Slider values are modelled as one required lower endpoint and one optional
/// upper endpoint.
///
/// Point slider:
///
/// ```text
/// min -------- lower ---------------- max
///                ^
///              value
/// ```
///
/// Interval slider:
///
/// ```text
/// min ---- lower ======== upper ----- max
///           ^              ^
///         start           end
/// ```
///
/// When `upper` is `None`, the slider represents a single value at `lower`.
/// When `upper` is `Some`, the slider represents the interval
/// `lower..upper`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SliderState {
    /// Required lower endpoint. For point sliders this is the single value.
    pub lower: f32,
    /// Optional upper endpoint. When present the slider represents an interval.
    pub upper: Option<f32>,
    pub focus_id: FocusId,
    pub active_part: Option<SliderPart>,
    pub drag_start_mouse_coord: f32,
    pub drag_start_lower: f32,
    pub drag_start_upper: Option<f32>,
    pub is_track_clicking: bool,
    pub track_click_start_coord: f32,
    pub next_repeat_time: f64,
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
/// `upper == None` is a point slider; a state with `upper == Some(_)` is an
/// interval slider and may be constrained by `min_gap` and `max_gap`.
#[derive(Debug, Clone, PartialEq)]
pub struct SliderSpec {
    /// Inclusive lower bound of the slider domain.
    pub min: f32,
    /// Inclusive upper bound of the slider domain.
    pub max: f32,
    /// Minimum allowed distance between `lower` and `upper`.
    ///
    /// Ignored when `upper` is `None`.
    /// If both `min_gap` and `max_gap` are set to the same value, the slider
    /// represents a fixed-size span that can be offset but not resized.
    pub min_gap: Option<f32>,
    /// Maximum allowed distance between `lower` and `upper`.
    ///
    /// Ignored when `upper` is `None`.
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
