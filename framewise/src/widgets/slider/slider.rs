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
        pub page_step: f32,
        pub step: f32,
        pub orientation: super::Orientation,
        pub thumb_len: super::ThumbLen,
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
    /// derived from `style.thumb_size`.
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
        // Safety clamp min/max
        let min = spec.min.min(spec.max);
        let max = spec.max.max(spec.min);
        state.value = state.value.clamp(min, max);
        let range = max - min;

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));

        // 1. Calculate Thumb Rect
        let track_rect = spec.rect;
        // Disabled or non-keyboard-focusable sliders stay out of the focus order entirely,
        // matching the button's disabled path.
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

        // 1. Calculate Thumb Rect
        let thumb_len = match spec.thumb_len {
            ThumbLen::Fixed(n) => n,
            ThumbLen::Proportional { ratio, min_len } => {
                (track_len * ratio.clamp(0.0, 1.0)).max(min_len)
            }
        };

        // Usable track length for the thumb's top/left edge
        let usable_track = (track_len - thumb_len).max(0.0);

        let val_ratio = if range > 0.0 {
            (state.value - min) / range
        } else {
            0.0
        };

        let thumb_pos = ((if is_vert { track_rect.y } else { track_rect.x })
            + (val_ratio * usable_track))
            .round();
        let track_cross_size = if is_vert { track_rect.w } else { track_rect.h };

        let thumb_rect = match spec.style.thumb.cross_axis {
            ThumbCrossAxis::FixedCentered(cross_len) => {
                let half = cross_len * 0.5;
                let center = if is_vert {
                    track_rect.x + track_rect.w * 0.5
                } else {
                    track_rect.y + track_rect.h * 0.5
                };
                if is_vert {
                    Rect::new(center - half, thumb_pos, cross_len, thumb_len)
                } else {
                    Rect::new(thumb_pos, center - half, thumb_len, cross_len)
                }
            }
            ThumbCrossAxis::FillTrack { margin } => {
                let cross = (track_cross_size - margin * 2.0).max(1.0);
                if is_vert {
                    Rect::new(track_rect.x + margin, thumb_pos, cross, thumb_len)
                } else {
                    Rect::new(thumb_pos, track_rect.y + margin, thumb_len, cross)
                }
            }
        };

        // 2. Input Handling
        if !spec.disabled {
            // Drag release
            if state.is_dragging && !input.mouse_down {
                state.is_dragging = false;
            }

            // Drag update
            if state.is_dragging && usable_track > 0.0 {
                let mouse_coord = if is_vert {
                    input.mouse_pos.y
                } else {
                    input.mouse_pos.x
                };
                let delta = mouse_coord - state.drag_start_mouse_coord;
                let val_delta = (delta / usable_track) * range;
                state.value = (state.drag_start_val + val_delta).clamp(min, max);
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
                if usable_track > 0.0 {
                    let track_start = if is_vert { track_rect.y } else { track_rect.x };
                    let snapped =
                        (mouse_coord - track_start - thumb_len / 2.0).clamp(0.0, usable_track);
                    state.value = (min + (snapped / usable_track) * range).clamp(min, max);
                    state.drag_start_mouse_coord = mouse_coord;
                    state.drag_start_val = state.value;
                }
                state.is_dragging = true;
                state.is_track_clicking = false;
                state.track_click_direction = None;
            }

            // Mouse wheel scrolling — suppressed during an active drag so that drag
            // motion is authoritative (otherwise wheel ticks would stack on top of
            // the drag-projected value).
            if is_visible && !state.is_dragging && track_rect.contains(input.mouse_pos) {
                let at_min = state.value <= min;
                let at_max = state.value >= max;

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
                        state.value = (state.value - scroll_delta * spec.step).clamp(min, max);
                    }
                    if scroll_delta < 0.0 && is_active_down_right {
                        state.value = (state.value - scroll_delta * spec.step).clamp(min, max);
                    }
                }
            }

            // Track click (mouse down on track, not on thumb)
            let thumb_start = if is_vert { thumb_rect.y } else { thumb_rect.x };
            let thumb_end = if is_vert {
                thumb_rect.bottom()
            } else {
                thumb_rect.right()
            };
            if is_visible
                && is_hover_active
                && input.mouse_pressed
                && !thumb_rect.contains(input.mouse_pos)
                && track_rect.contains(input.mouse_pos)
            {
                if spec.keyboard_focusable {
                    focus_system.take_keyboard_focus(state.focus_id);
                }
                state.is_track_clicking = true;
                state.track_click_start_coord = mouse_coord;
                state.next_repeat_time = spec.time + 0.5;
                // Page up/down towards mouse
                if mouse_coord < thumb_start {
                    state.track_click_direction = Some(PagingDirection::Up);
                    state.value = (state.value - spec.page_step).clamp(min, max);
                } else if mouse_coord > thumb_end {
                    state.track_click_direction = Some(PagingDirection::Down);
                    state.value = (state.value + spec.page_step).clamp(min, max);
                }
            }

            // Thumb drag start
            if is_visible
                && is_hover_active
                && input.mouse_pressed
                && thumb_rect.contains(input.mouse_pos)
            {
                if spec.keyboard_focusable {
                    focus_system.take_keyboard_focus(state.focus_id);
                }
                state.is_dragging = true;
                state.drag_start_mouse_coord = mouse_coord;
                state.drag_start_val = state.value;
            }

            // Track click repeat logic (time-based paging)
            if state.is_track_clicking && spec.time >= state.next_repeat_time {
                if track_rect.contains(input.mouse_pos) {
                    let track_start = if is_vert { track_rect.y } else { track_rect.x };
                    match state.track_click_direction {
                        Some(PagingDirection::Up) => {
                            if mouse_coord < thumb_start {
                                // Clamp so thumb's trailing edge doesn't overshoot cursor (prevents direction flip).
                                let cursor_val = if usable_track > 0.0 {
                                    min + ((mouse_coord - track_start - thumb_len) / usable_track)
                                        .clamp(0.0, 1.0)
                                        * range
                                } else {
                                    min
                                };
                                state.value = (state.value - spec.page_step)
                                    .max(cursor_val)
                                    .clamp(min, max);
                                state.next_repeat_time = spec.time + 0.05;
                            }
                        }
                        Some(PagingDirection::Down) => {
                            if mouse_coord > thumb_end {
                                // Clamp so thumb's leading edge doesn't overshoot cursor (prevents direction flip).
                                let cursor_val = if usable_track > 0.0 {
                                    min + ((mouse_coord - track_start) / usable_track)
                                        .clamp(0.0, 1.0)
                                        * range
                                } else {
                                    max
                                };
                                state.value = (state.value + spec.page_step)
                                    .min(cursor_val)
                                    .clamp(min, max);
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
                let at_min = state.value <= min;
                let at_max = state.value >= max;

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
                    state.value = (state.value - spec.page_step).clamp(min, max);
                }
                if input.key_pressed_page_down && is_active_pgdn {
                    state.value = (state.value + spec.page_step).clamp(min, max);
                }
                if input.key_pressed_up || input.key_pressed_left {
                    state.value = (state.value - spec.step).clamp(min, max);
                }
                if input.key_pressed_down || input.key_pressed_right {
                    state.value = (state.value + spec.step).clamp(min, max);
                }
                if input.key_pressed_home {
                    state.value = min;
                }
                if input.key_pressed_end {
                    state.value = max;
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
        match spec.style.track {
            TrackStyle::Line {
                stroke,
                fill_before_thumb,
            } => {
                if stroke.is_visible() {
                    let thickness = stroke.width;
                    let half_thick = thickness * 0.5;
                    let track_line = if is_vert {
                        let cx = track_rect.x + track_rect.w * 0.5;
                        Rect::new(cx - half_thick, track_rect.y, thickness, track_rect.h)
                    } else {
                        let cy = track_rect.y + track_rect.h * 0.5;
                        Rect::new(track_rect.x, cy - half_thick, track_rect.w, thickness)
                    };

                    // Full track line (e.g. ink).
                    cmds.push(DrawCmd::FillRect {
                        anti_alias: false,
                        rect: track_line,
                        color: tint(stroke.color),
                        z: spec.layer.get_z(),
                    });

                    if let Some(fill_color) = fill_before_thumb {
                        let fill_len = thumb_pos
                            - (if is_vert { track_rect.y } else { track_rect.x })
                            + thumb_len * 0.5;
                        let fill_bar = if is_vert {
                            let cx = track_rect.x + track_rect.w * 0.5;
                            Rect::new(cx - half_thick, track_rect.y, thickness, fill_len.max(0.0))
                        } else {
                            let cy = track_rect.y + track_rect.h * 0.5;
                            Rect::new(track_rect.x, cy - half_thick, fill_len.max(0.0), thickness)
                        };
                        let fill_color = if !spec.disabled && state.is_dragging {
                            spec.style.thumb.fill.dragged
                        } else {
                            fill_color
                        };
                        // Fill bar (active section before thumb).
                        cmds.push(DrawCmd::FillRect {
                            anti_alias: false,
                            rect: fill_bar,
                            color: tint(fill_color),
                            z: spec.layer.get_z(),
                        });
                    }
                }
            }
            TrackStyle::Rect { color, border } => {
                // Filled track background.
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: track_rect,
                    color: tint(color),
                    z: spec.layer.get_z(),
                });
                if let Some(border) = border {
                    let (p0, p1) = if is_vert {
                        (
                            Vec2::new(track_rect.x, track_rect.y),
                            Vec2::new(track_rect.x, track_rect.y + track_rect.h),
                        )
                    } else {
                        (
                            Vec2::new(track_rect.x, track_rect.y),
                            Vec2::new(track_rect.x + track_rect.w, track_rect.y),
                        )
                    };
                    // Separator border line.
                    let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                    cmds.push_stroke_line(
                        p0,
                        p1,
                        Some(tint_stroke(border)),
                        spec.layer.get_z(),
                        false,
                    );
                }
            }
        }

        let thumb_is_hovered = !spec.disabled && thumb_rect.contains(input.mouse_pos) && is_visible;
        let effective_thumb_fill = if spec.disabled {
            spec.style.thumb.fill.idle
        } else if state.is_dragging {
            spec.style.thumb.fill.dragged
        } else if thumb_is_hovered {
            spec.style.thumb.fill.hovered
        } else {
            spec.style.thumb.fill.idle
        };

        // Thumb rectangle.
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: thumb_rect,
            color: tint(effective_thumb_fill),
            z: spec.layer.get_z(),
        });

        if let Some(border) = spec.style.thumb.border {
            let border_color = if !spec.disabled && state.is_dragging {
                spec.style.thumb.fill.dragged
            } else {
                border.color
            };
            let effective_border = Stroke::new(border_color, border.width);
            let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
            cmds.push_stroke_rect(
                thumb_rect,
                Some(tint_stroke(effective_border)),
                spec.layer.get_z(),
                false,
            );
        }

        SliderResult {
            focused,
            input: InputInfo {
                hovered: !spec.disabled
                    && track_rect.contains(input.mouse_pos)
                    && is_visible
                    && is_hover_active,
                pressed: !spec.disabled && (state.is_dragging || state.is_track_clicking),
                clicked: false,
            },
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThumbLen {
    Fixed(f32),
    Proportional { ratio: f32, min_len: f32 },
}

impl ThumbLen {
    pub fn fixed(len: f32) -> Self {
        Self::Fixed(len)
    }
    pub fn proportional(ratio: f32, min_len: f32) -> Self {
        Self::Proportional { ratio, min_len }
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    pub track: TrackStyle,
    pub thumb: ThumbStyle,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrackStyle {
    Line {
        stroke: Stroke,
        fill_before_thumb: Option<Color>,
    },
    Rect {
        color: Color,
        border: Option<Stroke>,
    },
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
        Self {
            track: TrackStyle::Line {
                stroke: Stroke::new(theme.line, 1.5),
                fill_before_thumb: Some(theme.ink),
            },
            thumb: ThumbStyle {
                cross_axis: ThumbCrossAxis::FixedCentered(12.0),
                fill: InteractiveColor {
                    idle: theme.paper_elev,
                    hovered: theme.paper_elev,
                    dragged: theme.rust,
                },
                border: Some(Stroke::new(theme.ink, 1.0)),
            },
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
            track: TrackStyle::Rect {
                color: Color::linear_rgba(theme.ink.r, theme.ink.g, theme.ink.b, 0.04),
                border: Some(Stroke::new(theme.line_soft, 1.0)),
            },
            thumb: ThumbStyle {
                cross_axis: ThumbCrossAxis::FillTrack { margin: 1.0 },
                fill: InteractiveColor {
                    idle: theme.ink,
                    hovered: theme.rust,
                    dragged: theme.rust,
                },
                border: None,
            },
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

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SliderState {
    pub value: f32,
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_start_mouse_coord: f32,
    pub drag_start_val: f32,
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

#[derive(Debug, Clone, PartialEq)]
pub struct SliderSpec {
    pub min: f32,
    pub max: f32,
    pub page_step: f32,
    pub step: f32,
    pub orientation: Orientation,
    pub thumb_len: ThumbLen,
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
    pub page_step: Option<f32>,
    pub step: Option<f32>,
    pub orientation: Option<Orientation>,
    pub thumb_len: Option<ThumbLen>,
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
    pub fn thumb_len(mut self, thumb_len: ThumbLen) -> Self {
        self.thumb_len = Some(thumb_len);
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
            page_step: self.page_step.unwrap_or(10.0),
            step: self.step.unwrap_or(1.0),
            orientation: self.orientation.unwrap_or(Orientation::Horizontal),
            thumb_len: self.thumb_len.unwrap_or(ThumbLen::Fixed(12.0)),
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
        page_step: spec.page_step,
        step: spec.step,
        orientation: spec.orientation,
        thumb_len: spec.thumb_len,
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
