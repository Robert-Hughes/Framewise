use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    text::TextBackend,
    types::{ClipRect, Color, Layer, Rect, Vec2},
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
        pub thumb_size_ratio: Option<f32>, // 0.0 to 1.0 (for scrollbars)
        pub style: super::SliderStyle,
        pub clip_rect: ClipRect,
        /// When `true` (default for standalone sliders), the slider always claims
        /// both scroll directions from the hover system — even at its limits —
        /// preventing scroll events from propagating to any parent scroll area.
        ///
        /// Set to `false` for the internal scrollbar slider inside `ScrollArea`,
        /// so that when the content is fully scrolled the parent can hand off
        /// the event to an outer scroll area.
        pub claim_scroll_at_ends: bool,
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

        // Use proportional thumb size, or fallback to fixed size
        let thumb_len = if let Some(ratio) = spec.thumb_size_ratio {
            (track_len * ratio.clamp(0.0, 1.0)).max(24.0)
        } else {
            spec.style.thumb_size
        };

        // Usable track length for the thumb's top/left edge
        let usable_track = (track_len - thumb_len).max(0.0);

        let val_ratio = if range > 0.0 {
            (state.value - min) / range
        } else {
            0.0
        };

        let thumb_pos =
            (if is_vert { track_rect.y } else { track_rect.x }) + (val_ratio * usable_track);
        let track_cross_size = if is_vert { track_rect.w } else { track_rect.h };

        // Thumb hit rect — centered on the track axis.
        let thumb_rect = if spec.style.scrollbar_mode {
            let margin = spec.style.scrollbar_thumb_margin;
            let cross = (track_cross_size - margin * 2.0).max(1.0);
            if is_vert {
                Rect::new(track_rect.x + margin, thumb_pos, cross, thumb_len)
            } else {
                Rect::new(thumb_pos, track_rect.y + margin, thumb_len, cross)
            }
        } else {
            let half = spec.style.thumb_size * 0.5;
            let center = if is_vert {
                track_rect.x + track_rect.w * 0.5
            } else {
                track_rect.y + track_rect.h * 0.5
            };
            if is_vert {
                Rect::new(
                    center - half,
                    thumb_pos,
                    spec.style.thumb_size,
                    spec.style.thumb_size,
                )
            } else {
                Rect::new(
                    thumb_pos,
                    center - half,
                    spec.style.thumb_size,
                    spec.style.thumb_size,
                )
            }
        };

        // Disabled: draw the track + thumb tinted, take no input, claim nothing.
        // The reserved track space is preserved so layout is unaffected.
        if spec.disabled {
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            if spec.style.scrollbar_mode {
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: track_rect,
                    color: tint(spec.style.track_color),
                    z: spec.layer.get_z(),
                });
                if let Some(border_color) = spec.style.track_border_color {
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
                    cmds.push(DrawCmd::StrokeLine {
                        anti_alias: false,
                        p0,
                        p1,
                        color: tint(border_color),
                        width: 1.0,
                        z: spec.layer.get_z(),
                    });
                }
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: thumb_rect,
                    color: tint(spec.style.thumb_color),
                    z: spec.layer.get_z(),
                });
            } else {
                let half_thick = spec.style.thickness * 0.5;
                let track_line = if is_vert {
                    let cx = track_rect.x + track_rect.w * 0.5;
                    Rect::new(
                        cx - half_thick,
                        track_rect.y,
                        spec.style.thickness,
                        track_rect.h,
                    )
                } else {
                    let cy = track_rect.y + track_rect.h * 0.5;
                    Rect::new(
                        track_rect.x,
                        cy - half_thick,
                        track_rect.w,
                        spec.style.thickness,
                    )
                };
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: track_line,
                    color: tint(spec.style.track_color),
                    z: spec.layer.get_z(),
                });
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: thumb_rect,
                    color: tint(spec.style.thumb_color),
                    z: spec.layer.get_z(),
                });
                if spec.style.thumb_border_width > 0.0 {
                    cmds.push(DrawCmd::StrokeRect {
                        anti_alias: false,
                        rect: thumb_rect,
                        color: tint(spec.style.thumb_border_color),
                        width: spec.style.thumb_border_width,
                        z: spec.layer.get_z(),
                    });
                }
            }
            return SliderResult {
                focused: false,
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
            };
        }

        // Helper to get main coordinate
        let get_coord = |v: crate::types::Vec2| if is_vert { v.y } else { v.x };
        let mouse_coord = get_coord(input.mouse_pos);
        let _thumb_start = get_coord(Vec2::new(thumb_rect.x, thumb_rect.y));
        let _thumb_end = if is_vert {
            thumb_rect.bottom()
        } else {
            thumb_rect.right()
        };

        // 2. Input Handling

        const TRACK_DRAG_THRESHOLD: f32 = 4.0;

        // Drag release
        if state.is_dragging && !input.mouse_down {
            state.is_dragging = false;
        }

        // Drag update
        if state.is_dragging && usable_track > 0.0 {
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

            if spec.claim_scroll_at_ends {
                focus_system.claim_scroll_up(state.focus_id);
                focus_system.claim_scroll_down(state.focus_id);
                focus_system.claim_scroll_left(state.focus_id);
                focus_system.claim_scroll_right(state.focus_id);
            } else {
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
            if mouse_coord < _thumb_start {
                state.track_click_direction = Some(PagingDirection::Up);
                state.value = (state.value - spec.page_step).clamp(min, max);
            } else if mouse_coord > _thumb_end {
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
                        if mouse_coord < _thumb_start {
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
                        if mouse_coord > _thumb_end {
                            // Clamp so thumb's leading edge doesn't overshoot cursor (prevents direction flip).
                            let cursor_val = if usable_track > 0.0 {
                                min + ((mouse_coord - track_start) / usable_track).clamp(0.0, 1.0)
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

            if spec.claim_scroll_at_ends {
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
            } else {
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

        // 3. Drawing

        // Focus outline.
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: track_rect.inset(-spec.style.focus_offset),
                color: spec.style.focus,
                width: spec.style.focus_width,
                z: spec.layer.get_focus_z(),
            });
        }

        let thumb_is_hovered = thumb_rect.contains(input.mouse_pos) && is_visible;
        let effective_thumb_fill = if state.is_dragging {
            spec.style.thumb_drag_color
        } else if thumb_is_hovered {
            spec.style.thumb_hover_color
        } else {
            spec.style.thumb_color
        };
        let effective_thumb_border = if state.is_dragging {
            spec.style.thumb_drag_color
        } else {
            spec.style.thumb_border_color
        };

        if spec.style.scrollbar_mode {
            // Scrollbar: filled track background, ink/rust thumb spanning cross-section.
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: track_rect,
                color: spec.style.track_color,
                z: spec.layer.get_z(),
            });
            if let Some(border_color) = spec.style.track_border_color {
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
                cmds.push(DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0,
                    p1,
                    color: border_color,
                    width: 1.0,
                    z: spec.layer.get_z(),
                });
            }
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: thumb_rect,
                color: effective_thumb_fill,
                z: spec.layer.get_z(),
            });
        } else {
            // Standalone slider: hairline track, fill bar, square thumb with border.
            let half_thick = spec.style.thickness * 0.5;
            let fill_len = thumb_pos - (if is_vert { track_rect.y } else { track_rect.x })
                + spec.style.thumb_size * 0.5;

            let (track_line, fill_bar) = if is_vert {
                let cx = track_rect.x + track_rect.w * 0.5;
                (
                    Rect::new(
                        cx - half_thick,
                        track_rect.y,
                        spec.style.thickness,
                        track_rect.h,
                    ),
                    Rect::new(
                        cx - half_thick,
                        track_rect.y,
                        spec.style.thickness,
                        fill_len.max(0.0),
                    ),
                )
            } else {
                let cy = track_rect.y + track_rect.h * 0.5;
                (
                    Rect::new(
                        track_rect.x,
                        cy - half_thick,
                        track_rect.w,
                        spec.style.thickness,
                    ),
                    Rect::new(
                        track_rect.x,
                        cy - half_thick,
                        fill_len.max(0.0),
                        spec.style.thickness,
                    ),
                )
            };

            // Full track (ink).
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: track_line,
                color: spec.style.track_color,
                z: spec.layer.get_z(),
            });

            // Fill bar (rust when dragging, same as track otherwise — overlays track).
            let fill_color = if state.is_dragging {
                spec.style.thumb_drag_color
            } else {
                spec.style.track_color
            };
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: fill_bar,
                color: fill_color,
                z: spec.layer.get_z(),
            });

            // Square thumb.
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: thumb_rect,
                color: effective_thumb_fill,
                z: spec.layer.get_z(),
            });
            if spec.style.thumb_border_width > 0.0 {
                cmds.push(DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: thumb_rect,
                    color: effective_thumb_border,
                    width: spec.style.thumb_border_width,
                    z: spec.layer.get_z(),
                });
            }
        }

        SliderResult {
            focused,
            input: InputInfo {
                hovered: track_rect.contains(input.mouse_pos) && is_visible && is_hover_active,
                pressed: state.is_dragging || state.is_track_clicking,
                clicked: false,
            },
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    pub track_color: Color,
    pub thumb_color: Color,        // fill when idle
    pub thumb_border_color: Color, // border when idle/hover
    pub thumb_border_width: f32,
    pub thumb_hover_color: Color, // fill on hover (standalone only)
    pub thumb_drag_color: Color,  // fill + border when dragging
    pub focus: Color,
    pub focus_width: f32,
    pub focus_offset: f32,
    /// Track line thickness for standalone sliders; ignored in scrollbar mode.
    pub thickness: f32,
    /// Square thumb side length for standalone sliders.
    pub thumb_size: f32,
    /// When true, renders as a scrollbar (filled track bg, proportional thumb
    /// that fills the cross-section). When false, renders as a standalone
    /// slider (hairline track, fill bar, square thumb with border).
    pub scrollbar_mode: bool,
    /// Alpha multiplier applied to every color when the slider is disabled.
    pub disabled_alpha: f32,
    /// Separator border color for scrollbar tracks.
    pub track_border_color: Option<Color>,
    /// Margin between the scrollbar thumb and the track edge.
    pub scrollbar_thumb_margin: f32,
}

impl SliderStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            track_color: theme.ink,
            thumb_color: theme.paper_elev,
            thumb_border_color: theme.ink,
            thumb_border_width: 1.5,
            thumb_hover_color: theme.paper_elev,
            thumb_drag_color: theme.rust,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            thickness: 1.5,
            thumb_size: 12.0,
            scrollbar_mode: false,
            disabled_alpha: 0.32,
            track_border_color: None,
            scrollbar_thumb_margin: 1.0,
        }
    }

    pub fn scrollbar_from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            track_color: Color::linear_rgba(theme.ink.r, theme.ink.g, theme.ink.b, 0.04),
            thumb_color: theme.ink,
            thumb_border_color: Color::TRANSPARENT,
            thumb_border_width: 0.0,
            thumb_hover_color: theme.rust,
            thumb_drag_color: theme.rust,
            focus: theme.rust,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            thickness: 1.5,
            thumb_size: 12.0,
            scrollbar_mode: true,
            disabled_alpha: 0.4,
            track_border_color: Some(theme.line_soft),
            scrollbar_thumb_margin: 1.0,
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
    pub thumb_size_ratio: Option<f32>,
    pub style: SliderStyle,
    pub claim_scroll_at_ends: bool,
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
    pub thumb_size_ratio: Option<Option<f32>>,
    pub style: Option<SliderStyle>,
    pub claim_scroll_at_ends: Option<bool>,
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
    pub fn thumb_size_ratio(mut self, thumb_size_ratio: Option<f32>) -> Self {
        self.thumb_size_ratio = Some(thumb_size_ratio);
        self
    }
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn claim_scroll_at_ends(mut self, claim_scroll_at_ends: bool) -> Self {
        self.claim_scroll_at_ends = Some(claim_scroll_at_ends);
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
            thumb_size_ratio: self.thumb_size_ratio.unwrap_or(None),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            claim_scroll_at_ends: self.claim_scroll_at_ends.unwrap_or(true),
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
        thumb_size_ratio: spec.thumb_size_ratio,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        claim_scroll_at_ends: spec.claim_scroll_at_ends,
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
