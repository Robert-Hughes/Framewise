use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
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
    pub struct SliderCalcIntrinsicSizeSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderResult {
        pub input: InputInfo,
        pub focused: bool,
    }

    /// Measure a slider's intrinsic size from its spec.
    ///
    /// A slider's extent is caller-driven: the track length comes from the layout,
    /// not from content, so there is nothing to report yet — this returns
    /// [`IntrinsicSize::UNKNOWN`]. A later revision may report a cross-axis minimum
    /// derived from `style.thumb_size`.
    ///
    pub fn calc_slider_intrinsic_size(
        spec: &SliderCalcIntrinsicSizeSpec,
    ) -> crate::layout::IntrinsicSize {
        let _ = spec;
        crate::layout::IntrinsicSize::UNKNOWN
    }

    /// Low-level slider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn slider(
        spec: SliderSpec,
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

/// High-level slider widget function using WidgetContext.
///
/// This function accepts a SliderSpecBuilder and calls the low-level raw::slider function.
pub fn slider<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SliderSpecBuilder,
    layout_params: S::Params,
    state: &mut SliderState,
) -> SliderResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::SliderCalcIntrinsicSizeSpec {};
    let intrinsic = raw::calc_slider_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
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

    let result = raw::slider(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);
    SliderResult {
        layout: LayoutInfo::tight(rect),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SliderSpec;
    use super::*;

    #[test]
    fn test_slider_overlapping_hover() {
        let mut state1 = SliderState::default();
        let mut state2 = SliderState::default();

        crate::widgets::test_helpers::assert_overlapping_hover(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            |state1, state2, input, focus_system, cmds| {
                let mut spec1 = test_spec(0.0, 100.0, false);
                spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
                let mut spec2 = test_spec(0.0, 100.0, false);
                spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

                let res1 = raw::slider(spec1, state1, input, focus_system, cmds);
                let res2 = raw::slider(spec2, state2, input, focus_system, cmds);
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_slider_overlapping_click() {
        let mut state1 = SliderState::default();
        let mut state2 = SliderState::default();

        crate::widgets::test_helpers::assert_overlapping_click(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            false,
            |state1, state2, input, focus_system, cmds| {
                let mut spec1 = test_spec(0.0, 100.0, false);
                spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
                let mut spec2 = test_spec(0.0, 100.0, false);
                spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

                let res1 = raw::slider(spec1, state1, input, focus_system, cmds);
                let res2 = raw::slider(spec2, state2, input, focus_system, cmds);
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_slider_page_up_down_keyboard() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Must be focused to receive keyboard events
        focus_system.take_keyboard_focus(state.focus_id);

        // Frame 1: register_keyboard claims
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 2: Page Up
        focus_system.begin_frame();
        input.key_pressed_page_up = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 30.0);
        focus_system.end_frame();

        // Frame 3: Page Down
        focus_system.begin_frame();
        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 50.0);
        focus_system.end_frame();

        input.key_pressed_page_down = false;
        input.key_pressed_home = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 0.0);

        input.key_pressed_home = false;
        input.key_pressed_end = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 100.0);
    }

    #[test]
    fn test_slider_drag() {
        let mut state = SliderState::default();
        let spec = SliderSpec {
            style: SliderStyle {
                thumb_size: 20.0,
                ..SliderStyle::from_theme(&crate::theme::Theme::framewise())
            },
            ..test_spec(0.0, 100.0, true)
        };
        // Thumb is 20px high. Usable track = 100 - 20 = 80px.
        // So moving 40px down should increase value by 50.

        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Warmup frame to establish hover claim
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // 1. Click on thumb (thumb is at y=0 to y=20)
        input.mouse_pressed = true;
        input.mouse_down = true;

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(state.is_dragging);
        assert_eq!(state.drag_start_mouse_coord, 10.0);

        // 2. Drag down by 40px (mouse y = 50)
        input.mouse_pressed = false;
        input.mouse_pos.y = 50.0;
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // 40 / 80 usable track = 0.5 ratio = 50 value
        assert_eq!(state.value, 50.0);
    }

    #[test]
    fn test_slider_track_click_hold() {
        let mut state = SliderState::default();
        let spec = test_spec(0.0, 100.0, true);
        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Warmup frame to establish hover claim
        input.mouse_pos = crate::types::Vec2::new(10.0, 80.0);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // 1. Initial click at bottom of track (y=80)
        input.mouse_pressed = true;
        input.mouse_down = true;

        // Frame 1: time=0.0. Should page down by 20.0
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 20.0);
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5); // wait 500ms

        // Frame 2: time=0.4 (before repeat). No change.
        input.mouse_pressed = false;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.4,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 20.0);

        // Frame 3: time=0.5 (trigger repeat). Should page down to 40.0
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.5,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 40.0);
        assert_eq!(state.next_repeat_time, 0.55); // next in 50ms

        // Frame 4: time=0.6 (trigger repeat again). Should page down to 60.0
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.6,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 60.0);

        // Release mouse -> track clicking ends
        input.mouse_down = false;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.7,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(!state.is_track_clicking);
    }

    #[test]
    fn test_slider_arrow_keys() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        focus_system.take_keyboard_focus(state.focus_id);

        // Up decrements
        input.key_pressed_up = true;
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 45.0);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Up arrow must not move focus away from slider"
        );

        // Down increments
        input.key_pressed_up = false;
        input.key_pressed_down = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 50.0);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Down arrow must not move focus away from slider"
        );

        // Left decrements (same as Up)
        input.key_pressed_down = false;
        input.key_pressed_left = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 45.0);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Left arrow must not move focus away from slider"
        );

        // Right increments (same as Down)
        input.key_pressed_left = false;
        input.key_pressed_right = true;
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(state.value, 50.0);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Right arrow must not move focus away from slider"
        );

        // Left/Right also work on a horizontal slider
        input.key_pressed_right = false;
        let horiz_spec = SliderSpec {
            orientation: Orientation::Horizontal,
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            ..spec.clone()
        };
        let mut horiz_state = SliderState::default();
        horiz_state.value = 50.0;
        focus_system.take_keyboard_focus(horiz_state.focus_id);

        input.key_pressed_left = true;
        raw::slider(
            horiz_spec.clone(),
            &mut horiz_state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(horiz_state.value, 45.0);

        input.key_pressed_left = false;
        input.key_pressed_right = true;
        raw::slider(
            horiz_spec.clone(),
            &mut horiz_state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        assert_eq!(horiz_state.value, 50.0);
    }

    #[test]
    fn test_slider_tab_moves_focus_not_arrows() {
        let mut state_a = SliderState::default();
        state_a.value = 50.0;
        let mut state_b = SliderState::default();
        state_b.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        focus_system.take_keyboard_focus(state_a.focus_id);

        // Frame 1: Tab on focused slider_a — should shift focus to slider_b
        focus_system.begin_frame();
        let mut input = crate::input::Input::new();
        input.key_pressed_tab = true;
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state_a,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        raw::slider(
            spec.clone(),
            &mut state_b,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 2: confirm focus moved to slider_b
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state_a,
            &crate::input::Input::new(),
            &mut focus_system,
            &mut cmds,
        );
        raw::slider(
            spec.clone(),
            &mut state_b,
            &crate::input::Input::new(),
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state_b.focus_id),
            "Tab should move focus from slider_a to slider_b"
        );
        assert_eq!(state_a.value, 50.0, "Value must not change on Tab");
    }

    #[test]
    fn test_slider_click_takes_focus() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        // Click on the track
        let mut input = crate::input::Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);

        // Warmup frame to establish hover claim
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Evaluation frame with mouse pressed
        input.mouse_pressed = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Clicking slider must request focus"
        );
    }

    #[test]
    fn test_slider_clipped_click_does_not_take_focus() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();

        // Mouse is inside the widget rect but outside the clip_rect.
        let mut spec = test_spec(0.0, 100.0, true);
        spec.clip_rect = Some(Rect::new(500.0, 500.0, 20.0, 100.0));

        let mut input = crate::input::Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "Clicking a clipped-away slider must not take focus"
        );
    }

    #[test]
    fn test_slider_mouse_wheel() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Hover over the slider track
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);

        // Frame 1: Register hover
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.value, 50.0); // Hasn't scrolled yet, scroll_delta is 0

        // Frame 2: Mouse wheel spun up (positive delta) -> value should decrease
        input.scroll_delta.y = 2.0;
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // value = 50.0 - 2.0 * 5.0 = 40.0
        assert_eq!(state.value, 40.0);
    }

    /// Track: y=0..100, thumb_len=20, usable=80, value=0 → thumb at y=0..20.
    /// Click empty track at y=50 → page step to 20.0, is_track_clicking.
    /// Move mouse by 5px (> 4px threshold) to y=55 → snaps:
    ///   thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25, switches to drag.
    /// Then drag to y=65 → delta=10 → val_delta=12.5 → value=68.75.
    #[test]
    fn test_track_click_snaps_and_drags() {
        let mut state = SliderState::default();
        let spec = SliderSpec {
            style: SliderStyle {
                thumb_size: 20.0,
                ..SliderStyle::from_theme(&crate::theme::Theme::framewise())
            },
            ..test_spec(0.0, 100.0, true)
        };
        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Warmup frame to establish hover claim
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 1: click empty track at y=50 (thumb is at y=0..20) → page step
        input.mouse_pressed = true;
        input.mouse_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            state.is_track_clicking,
            "should be track-clicking after initial track click"
        );
        assert!(!state.is_dragging, "should not yet be dragging");
        assert_eq!(state.value, 20.0, "page step should fire on click");

        // Frame 2: move mouse 5px (> 4px threshold) while holding → transitions to drag+snap
        input.mouse_pressed = false;
        input.mouse_pos.y = 55.0;
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            state.is_dragging,
            "should switch to dragging after threshold exceeded"
        );
        assert!(!state.is_track_clicking, "track clicking should end");
        // snap: thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25
        assert!(
            (state.value - 56.25).abs() < 0.01,
            "snap to 56.25, got {}",
            state.value
        );

        // Frame 3: drag to y=65 → delta=10 → val_delta=12.5 → value=68.75
        input.mouse_pos.y = 65.0;
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            (state.value - 68.75).abs() < 0.01,
            "drag to 68.75, got {}",
            state.value
        );
    }

    // Regression: paging past the cursor causes direction-flip flicker.
    // Setup: track y=0..100, thumb_len=20, usable=80, page_step=60.
    // value=0 → thumb at y=0..20. Click at y=70 (below thumb).
    // Frame 1 (initial click): page to 60 → thumb at y=48..68.
    // Frame 2 (repeat at t=0.5): cursor y=70 > thumb_end=68, fires.
    //   Buggy: 60+60=120 → clamped to 100 → thumb at 80..100 → cursor < thumb_start → flicker.
    //   Fixed: clamp to cursor position (87.5) so thumb stops at cursor.
    // Frame 3 (repeat at t=0.6): cursor inside thumb → paging stops.
    #[test]
    fn test_track_click_repeat_does_not_overshoot_cursor() {
        let mut state = SliderState::default();
        let spec = SliderSpec {
            page_step: 60.0,
            style: SliderStyle {
                thumb_size: 20.0,
                ..SliderStyle::from_theme(&crate::theme::Theme::framewise())
            },
            ..test_spec(0.0, 100.0, true)
        };
        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Warmup frame to establish hover claim
        input.mouse_pos = crate::types::Vec2::new(10.0, 70.0);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 1: initial click at y=70 (well below thumb at y=0..20).
        input.mouse_pressed = true;
        input.mouse_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 60.0, "initial page: 0 + 60 = 60");
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5);

        // Frame 2: hold, before repeat fires.
        input.mouse_pressed = false;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.4,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 60.0);

        // Frame 3: repeat fires (t=0.5). Thumb at y=48..68, cursor at y=70 > 68 → fires.
        // Expected: value clamps to cursor position (87.5), NOT 100.
        // cursor_val = (70/80) * 100 = 87.5
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.5,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            (state.value - 87.5).abs() < 0.01,
            "repeat should stop at cursor position 87.5, got {}",
            state.value
        );

        // Frame 4: repeat fires again (t=0.6). Thumb now at y=70..90, cursor=70 inside → stop paging.
        // is_track_clicking must remain true so the drag transition can still fire.
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.6,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            (state.value - 87.5).abs() < 0.01,
            "value should not change after thumb reaches cursor, got {}",
            state.value
        );
        assert!(
            state.is_track_clicking,
            "is_track_clicking must stay true so drag is still possible"
        );
        assert!(!state.is_dragging);

        // Frame 5: still holding, move mouse 5px (past 4px threshold from initial click at y=70).
        // Drag transition should fire: thumb snaps to cursor, enters drag mode.
        // snap: mouse_coord=75, track_start=0, thumb_len=20 → snapped=75-10=65 → value=65/80*100=81.25
        input.mouse_pos.y = 75.0;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.65,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            state.is_dragging,
            "should enter drag mode after mouse moves past threshold"
        );
        assert!(!state.is_track_clicking);
        assert!(
            (state.value - 81.25).abs() < 0.01,
            "snap on drag entry: expected 81.25, got {}",
            state.value
        );

        // Frame 6: drag to y=85 → delta=10 → val_delta=12.5 → value=93.75
        input.mouse_pressed = false;
        input.mouse_pos.y = 85.0;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.7,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert!(
            (state.value - 93.75).abs() < 0.01,
            "drag: expected 93.75, got {}",
            state.value
        );
    }

    // Helper to build a standard test spec.
    fn test_spec(min: f32, max: f32, claim_at_ends: bool) -> SliderSpec {
        SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min,
            max,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            claim_scroll_at_ends: claim_at_ends,
            time: 0.0,
            disabled: false,
            keyboard_focusable: true,
            layer: Layer::default(),
        }
    }

    // ── Standalone slider ──────────────────────────────────────────────────────

    #[test]
    fn test_standalone_slider_wheel_at_min_blocks_propagation() {
        // Even when at minimum, a standalone slider claims both directions,
        // so a hypothetical parent scroll area would never see the event.
        let mut state = SliderState::default();
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0; // scroll up

        // Frame 1: slider registers first (inner), parent second (outer)
        focus_system.begin_frame();
        // Standalone slider registers first (inner)
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent registers after (outer, simulating parent's end())
        focus_system.claim_scroll_up(parent_id);
        focus_system.claim_scroll_down(parent_id);
        focus_system.end_frame();

        // Frame 2: parent checks — it should NOT have won either direction
        assert!(
            !focus_system.is_active_scroll_up(parent_id),
            "parent should not win scroll-up; standalone slider blocked it"
        );
        // Value stays at 0.0 (clamped, can't go below min)
        assert_eq!(state.value, 0.0);
    }

    #[test]
    fn test_standalone_slider_wheel_at_max_blocks_propagation() {
        let mut state = SliderState::default();
        state.value = 100.0; // already at max
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (simulating parent's end())
        focus_system.claim_scroll_up(parent_id);
        focus_system.claim_scroll_down(parent_id);
        focus_system.end_frame();

        assert!(
            !focus_system.is_active_scroll_down(parent_id),
            "parent should not win scroll-down; standalone slider blocked it"
        );
        assert_eq!(state.value, 100.0);
    }

    #[test]
    fn test_vertical_standalone_slider_blocks_horizontal_scroll() {
        // Regression: vertical standalone slider inside a horizontal scroll area was
        // letting horizontal scroll events propagate because claim_scroll_at_ends only
        // claimed up/down, not left/right.
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.x = 3.0; // horizontal scroll only

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (simulating parent's end())
        focus_system.claim_scroll_left(parent_id);
        focus_system.claim_scroll_right(parent_id);
        focus_system.end_frame();

        assert!(
            !focus_system.is_active_scroll_left(parent_id),
            "parent should not win scroll-left; vertical standalone slider should block it"
        );
        assert!(
            !focus_system.is_active_scroll_right(parent_id),
            "parent should not win scroll-right; vertical standalone slider should block it"
        );
    }

    // ── Propagating slider (scrollbar-within-scroll-area mode) ─────────────────

    #[test]
    fn test_propagating_slider_at_min_yields_scroll_up_to_parent() {
        let mut state = SliderState::default();
        // value = 0.0 — at min, can't scroll up
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0; // scroll up

        // Frame 1: inner propagating slider first, then parent claims simulating parent's end()
        focus_system.begin_frame();
        // Inner propagating slider at min: skips claim_scroll_up
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (simulating parent's end())
        focus_system.claim_scroll_up(parent_id); // parent can scroll up
        focus_system.claim_scroll_down(parent_id);
        focus_system.end_frame();

        // Parent should have retained the scroll-up claim
        assert!(
            focus_system.is_active_scroll_up(parent_id),
            "parent should win scroll-up when inner is at its minimum"
        );
        assert_eq!(state.value, 0.0, "inner value unchanged");
    }

    #[test]
    fn test_propagating_slider_at_max_yields_scroll_down_to_parent() {
        let mut state = SliderState::default();
        state.value = 100.0; // at max — can't scroll down
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (simulating parent's end())
        focus_system.claim_scroll_up(parent_id);
        focus_system.claim_scroll_down(parent_id); // parent can scroll down
        focus_system.end_frame();

        assert!(
            focus_system.is_active_scroll_down(parent_id),
            "parent should win scroll-down when inner is at its maximum"
        );
        assert_eq!(state.value, 100.0, "inner value unchanged");
    }

    #[test]
    fn test_propagating_slider_mid_range_wins_both_directions() {
        // When not at an end, the inner propagating slider claims both directions
        // and the parent gets neither.
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0;

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (simulating parent's end())
        focus_system.claim_scroll_up(parent_id);
        focus_system.claim_scroll_down(parent_id);
        focus_system.end_frame();

        assert!(
            !focus_system.is_active_scroll_up(parent_id),
            "parent should not win"
        );
        assert!(
            !focus_system.is_active_scroll_down(parent_id),
            "parent should not win"
        );
    }

    // ── Disabled ─────────────────────────────────────────────────────────────

    fn disabled_spec(scrollbar_mode: bool) -> SliderSpec {
        let theme = crate::theme::Theme::framewise();
        let style = if scrollbar_mode {
            SliderStyle::scrollbar_from_theme(&theme)
        } else {
            SliderStyle::from_theme(&theme)
        };
        SliderSpec {
            disabled: true,
            style,
            ..test_spec(0.0, 100.0, true)
        }
    }

    /// A disabled slider ignores mouse press, drag, wheel, and keyboard, and
    /// never takes focus (it isn't registered in the focus order).
    #[test]
    fn test_disabled_slider_ignores_all_input() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = disabled_spec(false);
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // Press on the thumb (thumb is centered around value=50).
        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        input.scroll_delta.y = 5.0;
        input.key_pressed_page_down = true;
        input.key_pressed_end = true;

        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.value, 50.0, "disabled slider must not change value");
        assert!(!state.is_dragging, "disabled slider must not start a drag");
        assert!(!state.is_track_clicking);
        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "disabled slider must not take focus"
        );
    }

    /// A disabled slider does not claim scroll, so a parent scroll area still
    /// wins the wheel even when the cursor is over the (degenerate) bar.
    #[test]
    fn test_disabled_slider_does_not_block_parent_scroll() {
        let mut state = SliderState::default();
        let mut focus_system = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0;

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            disabled_spec(true),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        // Parent claims after (inner-first ordering).
        focus_system.claim_scroll_up(parent_id);
        focus_system.claim_scroll_down(parent_id);
        focus_system.end_frame();

        assert!(
            focus_system.is_active_scroll_up(parent_id),
            "disabled slider must let the parent win the wheel"
        );
    }

    /// A disabled slider still draws (track + thumb), tinted by disabled_alpha,
    /// so it occupies its reserved track.
    #[test]
    fn test_disabled_slider_draws_tinted() {
        let mut state = SliderState::default();
        let spec = disabled_spec(true); // scrollbar mode: track fill + thumb fill
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input_none(),
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        let a = spec.style.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * a);
        // scrollbar-mode: vertical rect (0,0,20,100), ratio None falls back to
        // fixed thumb_size? No — test_spec uses thumb_size_ratio None, so thumb_len
        // = style.thumb_size = 12. We only assert structure + tinted colors here.
        match (&cmds[0], &cmds[1], &cmds[2]) {
            (
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: tr,
                    color: tc,
                    z: 0,
                },
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    color: bc,
                    ..
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    color: hc,
                    ..
                },
            ) => {
                assert_eq!(*tr, spec.rect, "track fill spans the full reserved rect");
                assert_eq!(*tc, tint(spec.style.track_color));
                assert_eq!(*bc, tint(spec.style.track_border_color.unwrap()));
                assert_eq!(*hc, tint(spec.style.thumb_color));
            }
            other => panic!("unexpected draw commands: {:?}", other),
        }
        assert_eq!(
            cmds.len(),
            3,
            "scrollbar-mode disabled draws track + border + thumb"
        );
    }

    fn input_none() -> Input {
        Input::new()
    }

    // ── Visual Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_slider_visual_normal() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let input = Input::new();
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _result = raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
                    z: 0,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_hovered() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _result = raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_hover_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
                    z: 0,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_drag() {
        let mut state = SliderState::default();
        state.is_dragging = true;
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        input.mouse_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _result = raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.thumb_drag_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_drag_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_drag_color,
                    width: spec.style.thumb_border_width,
                    z: 0,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_focused() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_system = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        focus_system.take_keyboard_focus(state.focus_id);

        let input = Input::new();
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        let _result = raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(-2.0, -2.0, 24.0, 104.0),
                    color: spec.style.focus,
                    width: spec.style.focus_width,
                    z: 1,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_color,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
                    z: 0,
                },
            ]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = SliderSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(SliderStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = SliderStyle::from_theme(&theme);
        custom_style.thumb_size = 99.0;
        let builder = SliderSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().thumb_size, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
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
        let mut state = SliderState::default();
        // Under ManualLayout the layout param *is* the rect — the sanctioned way
        // to place a high-level widget explicitly.
        super::slider(&mut ctx, SliderSpecBuilder::new(), placement, &mut state);
        // First draw command for a horizontal slider is the track-line FillRect,
        // whose x starts at the resolved track rect's x = placement.x.
        match &cmds[0] {
            crate::draw::DrawCmd::FillRect {
                anti_alias: false,
                rect,
                ..
            } => {
                assert_eq!(rect.x, placement.x);
            }
            other => panic!("Expected FillRect, got {:?}", other),
        }
    }

    #[test]
    fn test_calc_slider_intrinsic_size() {
        // A slider's size is caller-driven; it reports no intrinsic measurement.
        let spec = raw::SliderCalcIntrinsicSizeSpec {};
        assert_eq!(
            raw::calc_slider_intrinsic_size(&spec),
            crate::layout::IntrinsicSize::UNKNOWN
        );
    }

    #[test]
    fn test_track_click_overshoot_first_page_no_jump_back() {
        let mut state = SliderState::default();
        let spec = SliderSpec {
            page_step: 60.0,
            style: SliderStyle {
                thumb_size: 20.0,
                ..SliderStyle::from_theme(&crate::theme::Theme::framewise())
            },
            ..test_spec(0.0, 100.0, true) // track y=0..100, usable_track=80
        };
        let mut input = Input::new();
        let mut focus_system = FocusSystem::new();

        // Warmup frame
        input.mouse_pos = crate::types::Vec2::new(10.0, 25.0);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 1: Click at y=25 (right next to the initial thumb at y=0..20)
        input.mouse_pressed = true;
        input.mouse_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Moving one page allows overshoot (value goes to 60.0, thumb at y=48..68)
        assert_eq!(state.value, 60.0);
        assert!(state.is_track_clicking);

        // Frame 2: Hold, before repeat fires (t=0.4)
        input.mouse_pressed = false;
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.4,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 60.0);

        // Frame 3: Repeat fires (t=0.5).
        // Since we overshot, the cursor y=25 is now behind the thumb.
        // It must NOT jump back or trigger overshoot protection. Value must stay 60.0.
        focus_system.begin_frame();
        raw::slider(
            SliderSpec {
                time: 0.5,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.value, 60.0, "should not jump back on itself");
        assert!(state.is_track_clicking);
    }

    #[test]
    fn test_non_keyboard_focusable_slider() {
        let mut state = SliderState::default();
        let mut spec = test_spec(0.0, 100.0, true);
        spec.keyboard_focusable = false;

        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();
        let mut input = Input::new();

        // 1. Hovering & Scroll Wheel Claim
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);

        // Frame 1: Register hovers/scrolls
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 2: Check active hovers/scrolls (they are resolved on end_frame/begin_frame transition)
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );

        // Assert: not registered in keyboard focus order
        assert_eq!(focus_system.current_keyboard_focus(), None);

        // Assert: claims hover and scroll up/down
        assert!(focus_system.is_hover_active(state.focus_id));
        assert!(focus_system.is_active_scroll_up(state.focus_id));
        assert!(focus_system.is_active_scroll_down(state.focus_id));
        focus_system.end_frame();

        // 2. Click does NOT take keyboard focus
        input.mouse_pressed = true;
        focus_system.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(focus_system.current_keyboard_focus(), None);
    }
}
