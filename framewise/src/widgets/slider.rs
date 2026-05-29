use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    types::{ClipRect, Color, Rect, Vec2},
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
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SliderResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
    }

    /// Low-level slider widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn slider(
        spec: SliderSpec,
        state: &mut SliderState,
        input: &Input,
        focus_sys: &mut FocusSystem,
    ) -> SliderResult {
        let mut cmds = Vec::new();

        // Safety clamp min/max
        let min = spec.min.min(spec.max);
        let max = spec.max.max(spec.min);
        state.value = state.value.clamp(min, max);
        let range = max - min;

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));

        // 1. Calculate Thumb Rect
        let track_rect = spec.rect;
        let focused = focus_sys.register(state.focus_id, track_rect, spec.clip_rect);
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
            let margin = 1.0;
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
        }

        // Mouse wheel scrolling — suppressed during an active drag so that drag
        // motion is authoritative (otherwise wheel ticks would stack on top of
        // the drag-projected value).
        if is_visible && !state.is_dragging && track_rect.contains(input.mouse_pos) {
            let at_min = state.value <= min;
            let at_max = state.value >= max;

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

            if spec.claim_scroll_at_ends {
                focus_sys.claim_scroll_up(state.focus_id);
                focus_sys.claim_scroll_down(state.focus_id);
                focus_sys.claim_scroll_left(state.focus_id);
                focus_sys.claim_scroll_right(state.focus_id);
            } else {
                if is_vert {
                    // Vertical slider:
                    // Conditionally claim vertical scrolling to allow same-axis bubbling.
                    if !at_min {
                        focus_sys.claim_scroll_up(state.focus_id);
                    }
                    if !at_max {
                        focus_sys.claim_scroll_down(state.focus_id);
                    }
                    // Unconditionally claim horizontal scrolling to isolate from the horizontal axis.
                    focus_sys.claim_scroll_left(state.focus_id);
                    focus_sys.claim_scroll_right(state.focus_id);
                } else {
                    // Horizontal slider:
                    // Conditionally claim horizontal scrolling to allow same-axis bubbling.
                    if !at_min {
                        focus_sys.claim_scroll_left(state.focus_id);
                    }
                    if !at_max {
                        focus_sys.claim_scroll_right(state.focus_id);
                    }
                    // Unconditionally claim vertical scrolling to isolate from the vertical axis.
                    focus_sys.claim_scroll_up(state.focus_id);
                    focus_sys.claim_scroll_down(state.focus_id);
                }
            }

            let is_active_up_left = if is_vert {
                focus_sys.is_active_scroll_up(state.focus_id)
            } else {
                focus_sys.is_active_scroll_left(state.focus_id)
                    || focus_sys.is_active_scroll_up(state.focus_id)
            };
            let is_active_down_right = if is_vert {
                focus_sys.is_active_scroll_down(state.focus_id)
            } else {
                focus_sys.is_active_scroll_right(state.focus_id)
                    || focus_sys.is_active_scroll_down(state.focus_id)
            };

            if scroll_delta > 0.0 && is_active_up_left {
                state.value = (state.value - scroll_delta * spec.step).clamp(min, max);
            }
            if scroll_delta < 0.0 && is_active_down_right {
                state.value = (state.value - scroll_delta * spec.step).clamp(min, max);
            }
        }

        // Track click (mouse down on track, not on thumb)
        if is_visible
            && input.mouse_pressed
            && !thumb_rect.contains(input.mouse_pos)
            && track_rect.contains(input.mouse_pos)
        {
            focus_sys.take_focus(state.focus_id);
            state.is_track_clicking = true;
            state.track_click_start_coord = mouse_coord;
            state.next_repeat_time = spec.time + 0.5;
            // Page up/down towards mouse
            if mouse_coord < _thumb_start {
                state.value = (state.value - spec.page_step).clamp(min, max);
            } else if mouse_coord > _thumb_end {
                state.value = (state.value + spec.page_step).clamp(min, max);
            }
        }

        // Thumb drag start
        if is_visible && input.mouse_pressed && thumb_rect.contains(input.mouse_pos) {
            focus_sys.take_focus(state.focus_id);
            state.is_dragging = true;
            state.drag_start_mouse_coord = mouse_coord;
            state.drag_start_val = state.value;
        }

        // Track click repeat logic (time-based paging)
        if state.is_track_clicking && spec.time >= state.next_repeat_time {
            if track_rect.contains(input.mouse_pos) {
                let track_start = if is_vert { track_rect.y } else { track_rect.x };
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
                } else if mouse_coord > _thumb_end {
                    // Clamp so thumb's leading edge doesn't overshoot cursor (prevents direction flip).
                    let cursor_val = if usable_track > 0.0 {
                        min + ((mouse_coord - track_start) / usable_track).clamp(0.0, 1.0) * range
                    } else {
                        max
                    };
                    state.value = (state.value + spec.page_step)
                        .min(cursor_val)
                        .clamp(min, max);
                    state.next_repeat_time = spec.time + 0.05;
                }
                // else: cursor is now inside the thumb; paging stops but keep
                // is_track_clicking=true so the drag-transition check can still fire.
            } else {
                state.is_track_clicking = false;
            }
        }

        // Keyboard handling
        if focused {
            let at_min = state.value <= min;
            let at_max = state.value >= max;

            if spec.claim_scroll_at_ends {
                if is_vert {
                    focus_sys.claim_pgup_vert(state.focus_id);
                    focus_sys.claim_pgdn_vert(state.focus_id);
                    focus_sys.claim_pgup_horiz(state.focus_id);
                    focus_sys.claim_pgdn_horiz(state.focus_id);
                } else {
                    focus_sys.claim_pgup_horiz(state.focus_id);
                    focus_sys.claim_pgdn_horiz(state.focus_id);
                    focus_sys.claim_pgup_vert(state.focus_id);
                    focus_sys.claim_pgdn_vert(state.focus_id);
                }
            } else {
                if is_vert {
                    if !at_min {
                        focus_sys.claim_pgup_vert(state.focus_id);
                    }
                    if !at_max {
                        focus_sys.claim_pgdn_vert(state.focus_id);
                    }
                    focus_sys.claim_pgup_horiz(state.focus_id);
                    focus_sys.claim_pgdn_horiz(state.focus_id);
                } else {
                    if !at_min {
                        focus_sys.claim_pgup_horiz(state.focus_id);
                    }
                    if !at_max {
                        focus_sys.claim_pgdn_horiz(state.focus_id);
                    }
                    focus_sys.claim_pgup_vert(state.focus_id);
                    focus_sys.claim_pgdn_vert(state.focus_id);
                }
            }

            let is_active_pgup = if is_vert {
                focus_sys.is_active_pgup_vert(state.focus_id)
            } else {
                focus_sys.is_active_pgup_horiz(state.focus_id)
            };
            let is_active_pgdn = if is_vert {
                focus_sys.is_active_pgdn_vert(state.focus_id)
            } else {
                focus_sys.is_active_pgdn_horiz(state.focus_id)
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
            focus_sys.handle_traversal(
                focused,
                input,
                crate::focus::FocusTraversalKeys::tab_only(),
            );
        }

        // 3. Drawing

        // Focus outline.
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: track_rect.inset(-2.0),
                color: spec.style.focus_outline_color,
                width: 2.0,
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
                rect: track_rect,
                color: spec.style.track_color,
            });
            cmds.push(DrawCmd::FillRect {
                rect: thumb_rect,
                color: effective_thumb_fill,
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
                rect: track_line,
                color: spec.style.track_color,
            });

            // Fill bar (rust when dragging, same as track otherwise — overlays track).
            let fill_color = if state.is_dragging {
                spec.style.thumb_drag_color
            } else {
                spec.style.track_color
            };
            cmds.push(DrawCmd::FillRect {
                rect: fill_bar,
                color: fill_color,
            });

            // Square thumb.
            cmds.push(DrawCmd::FillRect {
                rect: thumb_rect,
                color: effective_thumb_fill,
            });
            if spec.style.thumb_border_width > 0.0 {
                cmds.push(DrawCmd::StrokeRect {
                    rect: thumb_rect,
                    color: effective_thumb_border,
                    width: spec.style.thumb_border_width,
                });
            }
        }

        SliderResult {
            draw: DrawCommands(cmds),
            focused,
            input: InputInfo {
                hovered: track_rect.contains(input.mouse_pos) && is_visible,
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
    pub focus_outline_color: Color,
    /// Track line thickness for standalone sliders; ignored in scrollbar mode.
    pub thickness: f32,
    /// Square thumb side length for standalone sliders.
    pub thumb_size: f32,
    /// When true, renders as a scrollbar (filled track bg, proportional thumb
    /// that fills the cross-section). When false, renders as a standalone
    /// slider (hairline track, fill bar, square thumb with border).
    pub scrollbar_mode: bool,
}

impl SliderStyle {
    pub fn scrollbar() -> Self {
        let ink = Color::from_srgb_u8(21, 19, 15, 255);
        Self {
            track_color: Color::linear_rgba(ink.r, ink.g, ink.b, 0.04),
            thumb_color: ink,
            thumb_border_color: Color::TRANSPARENT,
            thumb_border_width: 0.0,
            thumb_hover_color: Color::from_srgb_u8(194, 90, 44, 255),
            thumb_drag_color: Color::from_srgb_u8(194, 90, 44, 255),
            focus_outline_color: Color::from_srgb_u8(194, 90, 44, 255),
            thickness: 1.5,
            thumb_size: 12.0,
            scrollbar_mode: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
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
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SliderResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SliderSpecBuilder {
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub page_step: Option<f32>,
    pub step: Option<f32>,
    pub orientation: Option<Orientation>,
    pub thumb_size_ratio: Option<f32>,
    pub style: Option<SliderStyle>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<ClipRect>,
    pub claim_scroll_at_ends: Option<bool>,
    pub time: Option<f64>,
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
        self.thumb_size_ratio = thumb_size_ratio;
        self
    }
    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = Some(style);
        self
    }
    /// Sets the clip rectangle. High-level context functions supply this automatically — only needed when using the raw API directly.
    pub fn clip_rect(mut self, clip_rect: ClipRect) -> Self {
        self.clip_rect = Some(clip_rect);
        self
    }
    pub fn claim_scroll_at_ends(mut self, claim_scroll_at_ends: bool) -> Self {
        self.claim_scroll_at_ends = Some(claim_scroll_at_ends);
        self
    }
    pub fn time(mut self, time: f64) -> Self {
        self.time = Some(time);
        self
    }

    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(theme.slider_style());
        }
        self
    }

    pub fn build(self) -> raw::SliderSpec {
        raw::SliderSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            page_step: self.page_step.unwrap_or(10.0),
            step: self.step.unwrap_or(1.0),
            orientation: self.orientation.unwrap_or(Orientation::Horizontal),
            thumb_size_ratio: self.thumb_size_ratio,
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
            claim_scroll_at_ends: self.claim_scroll_at_ends.unwrap_or(true),
            time: self.time.unwrap_or(0.0),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level slider widget function using WidgetContext.
///
/// This function accepts a SliderSpec and calls the low-level raw::slider function.
pub fn slider<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SliderSpecBuilder,
    layout_params: S::Params,
    state: &mut SliderState,
) -> SliderResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .time(ctx.time)
        .build();
    let result = raw::slider(spec, state, ctx.input, ctx.focus_sys);
    ctx.append_cmds(result.draw);
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
    fn test_slider_page_up_down_keyboard() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Must be focused to receive keyboard events
        focus_sys.take_focus(state.focus_id);

        // Frame 1: register claims
        focus_sys.begin_frame();
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2: Page Up
        focus_sys.begin_frame();
        input.key_pressed_page_up = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 30.0);
        focus_sys.end_frame();

        // Frame 3: Page Down
        focus_sys.begin_frame();
        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 50.0);
        focus_sys.end_frame();

        input.key_pressed_page_down = false;
        input.key_pressed_home = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 0.0);

        input.key_pressed_home = false;
        input.key_pressed_end = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 100.0);
    }

    #[test]
    fn test_slider_drag() {
        let mut state = SliderState::default();
        let spec = SliderSpec {
            style: SliderStyle {
                thumb_size: 20.0,
                ..crate::theme::Theme::framewise().slider_style()
            },
            ..test_spec(0.0, 100.0, true)
        };
        // Thumb is 20px high. Usable track = 100 - 20 = 80px.
        // So moving 40px down should increase value by 50.

        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // 1. Click on thumb (thumb is at y=0 to y=20)
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert!(state.is_dragging);
        assert_eq!(state.drag_start_mouse_coord, 10.0);

        // 2. Drag down by 40px (mouse y = 50)
        input.mouse_pressed = false;
        input.mouse_pos.y = 50.0;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);

        // 40 / 80 usable track = 0.5 ratio = 50 value
        assert_eq!(state.value, 50.0);
    }

    #[test]
    fn test_slider_track_click_hold() {
        let mut state = SliderState::default();
        let spec = test_spec(0.0, 100.0, true);
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // 1. Initial click at bottom of track (y=80)
        input.mouse_pos = crate::types::Vec2::new(10.0, 80.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        // Frame 1: time=0.0. Should page down by 20.0
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 20.0);
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5); // wait 500ms

        // Frame 2: time=0.4 (before repeat). No change.
        input.mouse_pressed = false;
        raw::slider(
            SliderSpec {
                time: 0.4,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert_eq!(state.value, 20.0);

        // Frame 3: time=0.5 (trigger repeat). Should page down to 40.0
        raw::slider(
            SliderSpec {
                time: 0.5,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert_eq!(state.value, 40.0);
        assert_eq!(state.next_repeat_time, 0.55); // next in 50ms

        // Frame 4: time=0.6 (trigger repeat again). Should page down to 60.0
        raw::slider(
            SliderSpec {
                time: 0.6,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert_eq!(state.value, 60.0);

        // Release mouse -> track clicking ends
        input.mouse_down = false;
        raw::slider(
            SliderSpec {
                time: 0.7,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert!(!state.is_track_clicking);
    }

    #[test]
    fn test_slider_arrow_keys() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        focus_sys.take_focus(state.focus_id);

        // Up decrements
        input.key_pressed_up = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 45.0);
        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Up arrow must not move focus away from slider"
        );

        // Down increments
        input.key_pressed_up = false;
        input.key_pressed_down = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 50.0);
        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Down arrow must not move focus away from slider"
        );

        // Left decrements (same as Up)
        input.key_pressed_down = false;
        input.key_pressed_left = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 45.0);
        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Left arrow must not move focus away from slider"
        );

        // Right increments (same as Down)
        input.key_pressed_left = false;
        input.key_pressed_right = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 50.0);
        assert_eq!(
            focus_sys.current_focus(),
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
        focus_sys.take_focus(horiz_state.focus_id);

        input.key_pressed_left = true;
        raw::slider(horiz_spec.clone(), &mut horiz_state, &input, &mut focus_sys);
        assert_eq!(horiz_state.value, 45.0);

        input.key_pressed_left = false;
        input.key_pressed_right = true;
        raw::slider(horiz_spec.clone(), &mut horiz_state, &input, &mut focus_sys);
        assert_eq!(horiz_state.value, 50.0);
    }

    #[test]
    fn test_slider_tab_moves_focus_not_arrows() {
        let mut state_a = SliderState::default();
        state_a.value = 50.0;
        let mut state_b = SliderState::default();
        state_b.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        focus_sys.take_focus(state_a.focus_id);

        // Frame 1: Tab on focused slider_a — should shift focus to slider_b
        focus_sys.begin_frame();
        let mut input = crate::input::Input::new();
        input.key_pressed_tab = true;
        raw::slider(spec.clone(), &mut state_a, &input, &mut focus_sys);
        raw::slider(spec.clone(), &mut state_b, &input, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2: confirm focus moved to slider_b
        focus_sys.begin_frame();
        raw::slider(
            spec.clone(),
            &mut state_a,
            &crate::input::Input::new(),
            &mut focus_sys,
        );
        raw::slider(
            spec.clone(),
            &mut state_b,
            &crate::input::Input::new(),
            &mut focus_sys,
        );
        focus_sys.end_frame();
        assert_eq!(
            focus_sys.current_focus(),
            Some(state_b.focus_id),
            "Tab should move focus from slider_a to slider_b"
        );
        assert_eq!(state_a.value, 50.0, "Value must not change on Tab");
    }

    #[test]
    fn test_slider_click_takes_focus() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        // Click on the track
        let mut input = crate::input::Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        focus_sys.begin_frame();
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking slider must request focus"
        );
    }

    #[test]
    fn test_slider_clipped_click_does_not_take_focus() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();

        // Mouse is inside the widget rect but outside the clip_rect.
        let mut spec = test_spec(0.0, 100.0, true);
        spec.clip_rect = Some(Rect::new(500.0, 500.0, 20.0, 100.0));

        let mut input = crate::input::Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        focus_sys.begin_frame();
        raw::slider(spec, &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
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
        let mut focus_sys = FocusSystem::new();

        // Hover over the slider track
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);

        // Frame 1: Register hover
        focus_sys.begin_frame();
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(state.value, 50.0); // Hasn't scrolled yet, scroll_delta is 0

        // Frame 2: Mouse wheel spun up (positive delta) -> value should decrease
        input.scroll_delta.y = 2.0;
        focus_sys.begin_frame();
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

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
                ..crate::theme::Theme::framewise().slider_style()
            },
            ..test_spec(0.0, 100.0, true)
        };
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Frame 1: click empty track at y=50 (thumb is at y=0..20) → page step
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert!(
            state.is_track_clicking,
            "should be track-clicking after initial track click"
        );
        assert!(!state.is_dragging, "should not yet be dragging");
        assert_eq!(state.value, 20.0, "page step should fire on click");

        // Frame 2: move mouse 5px (> 4px threshold) while holding → transitions to drag+snap
        input.mouse_pressed = false;
        input.mouse_pos.y = 55.0;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
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
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
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
                ..crate::theme::Theme::framewise().slider_style()
            },
            ..test_spec(0.0, 100.0, true)
        };
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Frame 1: initial click at y=70 (well below thumb at y=0..20).
        input.mouse_pos = crate::types::Vec2::new(10.0, 70.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        assert_eq!(state.value, 60.0, "initial page: 0 + 60 = 60");
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5);

        // Frame 2: hold, before repeat fires.
        input.mouse_pressed = false;
        raw::slider(
            SliderSpec {
                time: 0.4,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert_eq!(state.value, 60.0);

        // Frame 3: repeat fires (t=0.5). Thumb at y=48..68, cursor at y=70 > 68 → fires.
        // Expected: value clamps to cursor position (87.5), NOT 100.
        // cursor_val = (70/80) * 100 = 87.5
        raw::slider(
            SliderSpec {
                time: 0.5,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
        assert!(
            (state.value - 87.5).abs() < 0.01,
            "repeat should stop at cursor position 87.5, got {}",
            state.value
        );

        // Frame 4: repeat fires again (t=0.6). Thumb now at y=70..90, cursor=70 inside → stop paging.
        // is_track_clicking must remain true so the drag transition can still fire.
        raw::slider(
            SliderSpec {
                time: 0.6,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
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
        raw::slider(
            SliderSpec {
                time: 0.65,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
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
        raw::slider(
            SliderSpec {
                time: 0.7,
                ..spec.clone()
            },
            &mut state,
            &input,
            &mut focus_sys,
        );
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
            style: crate::theme::Theme::framewise().slider_style(),
            clip_rect: None,
            claim_scroll_at_ends: claim_at_ends,
            time: 0.0,
        }
    }

    // ── Standalone slider ──────────────────────────────────────────────────────

    #[test]
    fn test_standalone_slider_wheel_at_min_blocks_propagation() {
        // Even when at minimum, a standalone slider claims both directions,
        // so a hypothetical parent scroll area would never see the event.
        let mut state = SliderState::default();
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0; // scroll up

        // Frame 1: both register
        focus_sys.begin_frame();
        // Parent registers first (outer)
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id);
        // Standalone slider registers second (inner) — overwrites both
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        // Frame 2: parent checks — it should NOT have won either direction
        assert!(
            !focus_sys.is_active_scroll_up(parent_id),
            "parent should not win scroll-up; standalone slider blocked it"
        );
        // Value stays at 0.0 (clamped, can't go below min)
        assert_eq!(state.value, 0.0);
    }

    #[test]
    fn test_standalone_slider_wheel_at_max_blocks_propagation() {
        let mut state = SliderState::default();
        state.value = 100.0; // already at max
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id);
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert!(
            !focus_sys.is_active_scroll_down(parent_id),
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
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.x = 3.0; // horizontal scroll only

        focus_sys.begin_frame();
        focus_sys.claim_scroll_left(parent_id);
        focus_sys.claim_scroll_right(parent_id);
        raw::slider(
            test_spec(0.0, 100.0, true),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert!(
            !focus_sys.is_active_scroll_left(parent_id),
            "parent should not win scroll-left; vertical standalone slider should block it"
        );
        assert!(
            !focus_sys.is_active_scroll_right(parent_id),
            "parent should not win scroll-right; vertical standalone slider should block it"
        );
    }

    // ── Propagating slider (scrollbar-within-scroll-area mode) ─────────────────

    #[test]
    fn test_propagating_slider_at_min_yields_scroll_up_to_parent() {
        let mut state = SliderState::default();
        // value = 0.0 — at min, can't scroll up
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0; // scroll up

        // Frame 1: parent claims, then inner propagating slider
        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id); // parent can scroll up
        focus_sys.claim_scroll_down(parent_id);
        // Inner propagating slider at min: skips claim_scroll_up
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        // Parent should have retained the scroll-up claim
        assert!(
            focus_sys.is_active_scroll_up(parent_id),
            "parent should win scroll-up when inner is at its minimum"
        );
        assert_eq!(state.value, 0.0, "inner value unchanged");
    }

    #[test]
    fn test_propagating_slider_at_max_yields_scroll_down_to_parent() {
        let mut state = SliderState::default();
        state.value = 100.0; // at max — can't scroll down
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id); // parent can scroll down
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert!(
            focus_sys.is_active_scroll_down(parent_id),
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
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0;

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id);
        raw::slider(
            test_spec(0.0, 100.0, false),
            &mut state,
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert!(
            !focus_sys.is_active_scroll_up(parent_id),
            "parent should not win"
        );
        assert!(
            !focus_sys.is_active_scroll_down(parent_id),
            "parent should not win"
        );
    }

    // ── Visual Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_slider_visual_normal() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let input = Input::new();
        focus_sys.begin_frame();
        let result = raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            result.draw.0,
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_hovered() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);

        focus_sys.begin_frame();
        let result = raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            result.draw.0,
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_hover_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_drag() {
        let mut state = SliderState::default();
        state.is_dragging = true;
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        let mut input = Input::new();
        input.mouse_down = true;
        focus_sys.begin_frame();
        let result = raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            result.draw.0,
            vec![
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.thumb_drag_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_drag_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_drag_color,
                    width: spec.style.thumb_border_width,
                },
            ]
        );
    }

    #[test]
    fn test_slider_visual_focused() {
        let mut state = SliderState::default();
        state.value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let spec = test_spec(0.0, 100.0, true);

        focus_sys.take_focus(state.focus_id);

        let input = Input::new();
        focus_sys.begin_frame();
        let result = raw::slider(spec.clone(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            result.draw.0,
            vec![
                DrawCmd::StrokeRect {
                    rect: Rect::new(-2.0, -2.0, 24.0, 104.0),
                    color: spec.style.focus_outline_color,
                    width: 2.0,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 100.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(9.25, 0.0, 1.5, 50.0),
                    color: spec.style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_color,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(4.0, 44.0, 12.0, 12.0),
                    color: spec.style.thumb_border_color,
                    width: spec.style.thumb_border_width,
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
        assert_eq!(builder.style, Some(theme.slider_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.slider_style();
        custom_style.thumb_size = 99.0;
        let builder = SliderSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().thumb_size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let mut state = SliderState::default();
        super::slider(
            &mut ctx,
            SliderSpecBuilder::new().rect(custom_rect),
            layout_rect,
            &mut state,
        );
        // First draw command for horizontal slider is FillRect for the track line,
        // which starts at track_rect.x = custom_rect.x
        match &cmds[0] {
            crate::draw::DrawCmd::FillRect { rect, .. } => {
                assert_eq!(rect.x, custom_rect.x);
            }
            other => panic!("Expected FillRect, got {:?}", other),
        }
    }
}
