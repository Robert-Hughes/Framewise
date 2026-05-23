use crate::{
    draw::DrawCmd,
    focus::{FocusId, FocusSystem},
    input::Input,
    types::{Color, Rect, Vec2},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    pub track_color: Color,
    pub thumb_color: Color,
    pub thumb_hover_color: Color,
    pub thumb_drag_color: Color,
    pub focus_outline_color: Color,
    pub thickness: f32, // Width (for vertical) or height (for horizontal) of track
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: Color::rgb(0.15, 0.15, 0.18),
            thumb_color: Color::rgb(0.4, 0.4, 0.45),
            thumb_hover_color: Color::rgb(0.5, 0.5, 0.55),
            thumb_drag_color: Color::rgb(0.6, 0.6, 0.65),
            focus_outline_color: Color::rgb(0.2, 0.5, 0.9),
            thickness: 12.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Vertical,
    Horizontal,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliderSpec {
    pub rect: Rect,
    pub min: f32,
    pub max: f32,
    pub page_step: f32,
    pub step: f32,
    pub orientation: Orientation,
    pub thumb_size_ratio: Option<f32>, // 0.0 to 1.0 (for scrollbars)
    pub style: SliderStyle,
    pub clip_rect: Option<Rect>,
    /// When `true` (default for standalone sliders), the slider always claims
    /// both scroll directions from the hover system — even at its limits —
    /// preventing scroll events from propagating to any parent scroll area.
    ///
    /// Set to `false` for the internal scrollbar slider inside `ScrollArea`,
    /// so that when the content is fully scrolled the parent can hand off
    /// the event to an outer scroll area.
    pub claim_scroll_at_ends: bool,
}

#[derive(Debug, Clone)]
pub struct SliderState {
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_start_mouse_coord: f32,
    pub drag_start_val: f32,
    pub is_track_clicking: bool,
    pub track_click_start_coord: f32,
    pub next_repeat_time: f64,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            focus_id: FocusId::new(),
            is_dragging: false,
            drag_start_mouse_coord: 0.0,
            drag_start_val: 0.0,
            is_track_clicking: false,
            track_click_start_coord: 0.0,
            next_repeat_time: 0.0,
        }
    }
}

pub fn slider(
    state: &mut SliderState,
    value: &mut f32,
    spec: SliderSpec,
    input: &Input,
    time: f64,
    focus_sys: &mut FocusSystem,
) -> Vec<DrawCmd> {
    let focused = focus_sys.register(state.focus_id);
    let mut cmds = Vec::new();
    
    // Safety clamp min/max
    let min = spec.min.min(spec.max);
    let max = spec.max.max(spec.min);
    *value = value.clamp(min, max);
    let range = max - min;
    
    let is_visible = spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos));
    
    // 1. Calculate Thumb Rect
    let track_rect = spec.rect;
    let is_vert = spec.orientation == Orientation::Vertical;
    
    let track_len = if is_vert { track_rect.h } else { track_rect.w };
    
    // Use proportional thumb size, or fallback to fixed size
    let thumb_len = if let Some(ratio) = spec.thumb_size_ratio {
        (track_len * ratio.clamp(0.0, 1.0)).max(20.0)
    } else {
        spec.style.thickness.max(20.0)
    };
    
    // Usable track length for the thumb's top/left edge
    let usable_track = (track_len - thumb_len).max(0.0);
    
    let val_ratio = if range > 0.0 {
        (*value - min) / range
    } else {
        0.0
    };
    
    let thumb_pos = (if is_vert { track_rect.y } else { track_rect.x }) + (val_ratio * usable_track);
    let track_cross_size = if is_vert { track_rect.w } else { track_rect.h };
    let cross_size = (track_cross_size - 4.0).max(1.0);
    
    let thumb_rect = if is_vert {
        Rect::new(track_rect.x + 2.0, thumb_pos, cross_size, thumb_len)
    } else {
        Rect::new(thumb_pos, track_rect.y + 2.0, thumb_len, cross_size)
    };
    
    // Helper to get main coordinate
    let get_coord = |v: crate::types::Vec2| if is_vert { v.y } else { v.x };
    let mouse_coord = get_coord(input.mouse_pos);
    let thumb_start = get_coord(Vec2::new(thumb_rect.x, thumb_rect.y));
    let thumb_end = if is_vert { thumb_rect.bottom() } else { thumb_rect.right() };
    
    // 2. Input Handling
    
    const TRACK_DRAG_THRESHOLD: f32 = 4.0;

    // Drag release
    if state.is_dragging && !input.mouse_down {
        state.is_dragging = false;
    }

    // Drag update
    if state.is_dragging {
        if usable_track > 0.0 {
            let delta = mouse_coord - state.drag_start_mouse_coord;
            let val_delta = (delta / usable_track) * range;
            *value = (state.drag_start_val + val_delta).clamp(min, max);
        }
    }

    // Track click release
    if state.is_track_clicking && !input.mouse_down {
        state.is_track_clicking = false;
    }

    // Track click → drag transition: mouse moved past threshold
    if state.is_track_clicking && input.mouse_down {
        if (mouse_coord - state.track_click_start_coord).abs() > TRACK_DRAG_THRESHOLD {
            if usable_track > 0.0 {
                let track_start = if is_vert { track_rect.y } else { track_rect.x };
                let snapped = (mouse_coord - track_start - thumb_len / 2.0).clamp(0.0, usable_track);
                *value = (min + (snapped / usable_track) * range).clamp(min, max);
                state.drag_start_mouse_coord = mouse_coord;
                state.drag_start_val = *value;
            }
            state.is_dragging = true;
            state.is_track_clicking = false;
        }
    }
    
    // Mouse wheel scrolling — suppressed during an active drag so that drag
    // motion is authoritative (otherwise wheel ticks would stack on top of
    // the drag-projected value).
    if is_visible && !state.is_dragging && track_rect.contains(input.mouse_pos) {
        let at_min = *value <= min;
        let at_max = *value >= max;
        
        let scroll_delta = if is_vert {
            input.scroll_delta.y
        } else {
            // For horizontal sliders, listen to horizontal wheel delta. 
            // If horizontal is 0, map vertical wheel to horizontal movement (per user request).
            if input.scroll_delta.x != 0.0 { input.scroll_delta.x } else { input.scroll_delta.y }
        };

        if spec.claim_scroll_at_ends {
            if is_vert {
                focus_sys.claim_scroll_up(state.focus_id);
                focus_sys.claim_scroll_down(state.focus_id);
            } else {
                focus_sys.claim_scroll_left(state.focus_id);
                focus_sys.claim_scroll_right(state.focus_id);
                focus_sys.claim_scroll_up(state.focus_id);
                focus_sys.claim_scroll_down(state.focus_id);
            }
        } else {
            if is_vert {
                // Vertical slider:
                // Conditionally claim vertical scrolling to allow same-axis bubbling.
                if !at_min { focus_sys.claim_scroll_up(state.focus_id); }
                if !at_max { focus_sys.claim_scroll_down(state.focus_id); }
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

        let is_active_up_left = if is_vert { focus_sys.is_active_scroll_up(state.focus_id) } else { 
            focus_sys.is_active_scroll_left(state.focus_id) || focus_sys.is_active_scroll_up(state.focus_id) 
        };
        let is_active_down_right = if is_vert { focus_sys.is_active_scroll_down(state.focus_id) } else { 
            focus_sys.is_active_scroll_right(state.focus_id) || focus_sys.is_active_scroll_down(state.focus_id) 
        };

        if scroll_delta > 0.0 && is_active_up_left {
            *value = (*value - scroll_delta * spec.step).clamp(min, max);
        }
        if scroll_delta < 0.0 && is_active_down_right {
            *value = (*value - scroll_delta * spec.step).clamp(min, max);
        }
    }

    // Mouse clicks
    if input.mouse_pressed && is_visible {
        if thumb_rect.contains(input.mouse_pos) {
            // Clicked thumb -> start dragging
            state.is_dragging = true;
            state.drag_start_mouse_coord = mouse_coord;
            state.drag_start_val = *value;
            focus_sys.take_focus(state.focus_id);
        } else if track_rect.contains(input.mouse_pos) {
            // Clicked track -> page up/down towards mouse
            if mouse_coord < thumb_start {
                *value = (*value - spec.page_step).clamp(min, max);
            } else if mouse_coord > thumb_end {
                *value = (*value + spec.page_step).clamp(min, max);
            }
            focus_sys.take_focus(state.focus_id);
            state.is_track_clicking = true;
            state.track_click_start_coord = mouse_coord;
            state.next_repeat_time = time + 0.5;
        }
    } else if state.is_track_clicking && time >= state.next_repeat_time {
        if track_rect.contains(input.mouse_pos) {
            let track_start = if is_vert { track_rect.y } else { track_rect.x };
            if mouse_coord < thumb_start {
                // Clamp so thumb's trailing edge doesn't overshoot cursor (prevents direction flip).
                let cursor_val = if usable_track > 0.0 {
                    min + ((mouse_coord - track_start - thumb_len) / usable_track).clamp(0.0, 1.0) * range
                } else {
                    min
                };
                *value = (*value - spec.page_step).max(cursor_val).clamp(min, max);
                state.next_repeat_time = time + 0.05;
            } else if mouse_coord > thumb_end {
                // Clamp so thumb's leading edge doesn't overshoot cursor (prevents direction flip).
                let cursor_val = if usable_track > 0.0 {
                    min + ((mouse_coord - track_start) / usable_track).clamp(0.0, 1.0) * range
                } else {
                    max
                };
                *value = (*value + spec.page_step).min(cursor_val).clamp(min, max);
                state.next_repeat_time = time + 0.05;
            }
            // else: cursor is now inside the thumb; paging stops but keep
            // is_track_clicking=true so the drag-transition check can still fire.
        } else {
            state.is_track_clicking = false;
        }
    }
    
    // Keyboard Input (if focused)
    if focused {
        let at_min = *value <= min;
        let at_max = *value >= max;

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
                if !at_min { focus_sys.claim_pgup_vert(state.focus_id); }
                if !at_max { focus_sys.claim_pgdn_vert(state.focus_id); }
                focus_sys.claim_pgup_horiz(state.focus_id);
                focus_sys.claim_pgdn_horiz(state.focus_id);
            } else {
                if !at_min { focus_sys.claim_pgup_horiz(state.focus_id); }
                if !at_max { focus_sys.claim_pgdn_horiz(state.focus_id); }
                focus_sys.claim_pgup_vert(state.focus_id);
                focus_sys.claim_pgdn_vert(state.focus_id);
            }
        }

        let is_active_pgup = if is_vert { focus_sys.is_active_pgup_vert(state.focus_id) } else { focus_sys.is_active_pgup_horiz(state.focus_id) };
        let is_active_pgdn = if is_vert { focus_sys.is_active_pgdn_vert(state.focus_id) } else { focus_sys.is_active_pgdn_horiz(state.focus_id) };

        if input.key_pressed_page_up && is_active_pgup {
            *value = (*value - spec.page_step).clamp(min, max);
        }
        if input.key_pressed_page_down && is_active_pgdn {
            *value = (*value + spec.page_step).clamp(min, max);
        }
        if input.key_pressed_up || input.key_pressed_left {
            *value = (*value - spec.step).clamp(min, max);
        }
        if input.key_pressed_down || input.key_pressed_right {
            *value = (*value + spec.step).clamp(min, max);
        }
        if input.key_pressed_home {
            *value = min;
        }
        if input.key_pressed_end {
            *value = max;
        }
    }
    
    // 3. Drawing
    // Focus Outline
    if focused {
        cmds.push(DrawCmd::StrokeRect {
            rect: track_rect.inset(-2.0),
            color: spec.style.focus_outline_color,
            width: 2.0,
        });
    }

    // Track
    cmds.push(DrawCmd::FillRect {
        rect: track_rect,
        color: spec.style.track_color,
    });
    
    // Thumb
    let mut thumb_color = spec.style.thumb_color;
    if state.is_dragging {
        thumb_color = spec.style.thumb_drag_color;
    } else if thumb_rect.contains(input.mouse_pos) && is_visible {
        thumb_color = spec.style.thumb_hover_color;
    }
    
    cmds.push(DrawCmd::FillRect {
        rect: thumb_rect,
        color: thumb_color,
    });
    
    
    cmds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slider_page_up_down_keyboard() {
        let mut state = SliderState::default();
        let mut value = 50.0;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Must be focused to receive keyboard events
        focus_sys.take_focus(state.focus_id);

        // Frame 1: register claims
        focus_sys.begin_frame();
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2: Page Up
        focus_sys.begin_frame();
        input.key_pressed_page_up = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 30.0);
        focus_sys.end_frame();

        // Frame 3: Page Down
        focus_sys.begin_frame();
        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 50.0);
        focus_sys.end_frame();
        
        input.key_pressed_page_down = false;
        input.key_pressed_home = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 0.0);
        
        input.key_pressed_home = false;
        input.key_pressed_end = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 100.0);
    }

    #[test]
    fn test_slider_drag() {
        let mut state = SliderState::default();
        let mut value = 0.0;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0), // track height 100
            min: 0.0,
            max: 100.0, // range 100
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None, // Will use style.width (12.0) but maxed to 20.0 for thumb_h
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        // Thumb is 20px high. Usable track = 100 - 20 = 80px.
        // So moving 40px down should increase value by 50.
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // 1. Click on thumb (thumb is at y=0 to y=20)
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert!(state.is_dragging);
        assert_eq!(state.drag_start_mouse_coord, 10.0);

        // 2. Drag down by 40px (mouse y = 50)
        input.mouse_pressed = false;
        input.mouse_pos.y = 50.0;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        
        // 40 / 80 usable track = 0.5 ratio = 50 value
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_slider_track_click_hold() {
        let mut state = SliderState::default();
        let mut value = 0.0;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0), // track height 100
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // 1. Initial click at bottom of track (y=80)
        input.mouse_pos = crate::types::Vec2::new(10.0, 80.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        // Frame 1: time=0.0. Should page down by 20.0
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 20.0);
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5); // wait 500ms

        // Frame 2: time=0.4 (before repeat). No change.
        input.mouse_pressed = false;
        slider(&mut state, &mut value, spec.clone(), &input, 0.4, &mut focus_sys);
        assert_eq!(value, 20.0);

        // Frame 3: time=0.5 (trigger repeat). Should page down to 40.0
        slider(&mut state, &mut value, spec.clone(), &input, 0.5, &mut focus_sys);
        assert_eq!(value, 40.0);
        assert_eq!(state.next_repeat_time, 0.55); // next in 50ms

        // Frame 4: time=0.6 (trigger repeat again). Should page down to 60.0
        slider(&mut state, &mut value, spec.clone(), &input, 0.6, &mut focus_sys);
        assert_eq!(value, 60.0);

        // Release mouse -> track clicking ends
        input.mouse_down = false;
        slider(&mut state, &mut value, spec.clone(), &input, 0.7, &mut focus_sys);
        assert!(!state.is_track_clicking);
    }

    #[test]
    fn test_slider_arrow_keys() {
        let mut state = SliderState::default();
        let mut value = 50.0;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        focus_sys.take_focus(state.focus_id);

        input.key_pressed_up = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 45.0);

        input.key_pressed_up = false;
        input.key_pressed_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 50.0);
    }

    #[test]
    fn test_slider_mouse_wheel() {
        let mut state = SliderState::default();
        let mut value = 50.0;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };

        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Hover over the slider track
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        
        // Frame 1: Register hover
        focus_sys.begin_frame();
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();
        
        assert_eq!(value, 50.0); // Hasn't scrolled yet, scroll_delta is 0

        // Frame 2: Mouse wheel spun up (positive delta) -> value should decrease
        input.scroll_delta.y = 2.0;
        focus_sys.begin_frame();
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // value = 50.0 - 2.0 * 5.0 = 40.0
        assert_eq!(value, 40.0);
    }

    /// Track: y=0..100, thumb_len=20, usable=80, value=0 → thumb at y=0..20.
    /// Click empty track at y=50 → page step to 20.0, is_track_clicking.
    /// Move mouse by 5px (> 4px threshold) to y=55 → snaps:
    ///   thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25, switches to drag.
    /// Then drag to y=65 → delta=10 → val_delta=12.5 → value=68.75.
    #[test]
    fn test_track_click_snaps_and_drags() {
        let mut state = SliderState::default();
        let mut value = 0.0_f32;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Frame 1: click empty track at y=50 (thumb is at y=0..20) → page step
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert!(state.is_track_clicking, "should be track-clicking after initial track click");
        assert!(!state.is_dragging, "should not yet be dragging");
        assert_eq!(value, 20.0, "page step should fire on click");

        // Frame 2: move mouse 5px (> 4px threshold) while holding → transitions to drag+snap
        input.mouse_pressed = false;
        input.mouse_pos.y = 55.0;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert!(state.is_dragging, "should switch to dragging after threshold exceeded");
        assert!(!state.is_track_clicking, "track clicking should end");
        // snap: thumb_start = 55 - 10 = 45 → val = 45/80*100 = 56.25
        assert!((value - 56.25).abs() < 0.01, "snap to 56.25, got {value}");

        // Frame 3: drag to y=65 → delta=10 → val_delta=12.5 → value=68.75
        input.mouse_pos.y = 65.0;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert!((value - 68.75).abs() < 0.01, "drag to 68.75, got {value}");
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
        let mut value = 0.0_f32;
        let spec = SliderSpec {
            orientation: Orientation::Vertical,
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 60.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true,
        };
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Frame 1: initial click at y=70 (well below thumb at y=0..20).
        input.mouse_pos = crate::types::Vec2::new(10.0, 70.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 60.0, "initial page: 0 + 60 = 60");
        assert!(state.is_track_clicking);
        assert_eq!(state.next_repeat_time, 0.5);

        // Frame 2: hold, before repeat fires.
        input.mouse_pressed = false;
        slider(&mut state, &mut value, spec.clone(), &input, 0.4, &mut focus_sys);
        assert_eq!(value, 60.0);

        // Frame 3: repeat fires (t=0.5). Thumb at y=48..68, cursor at y=70 > 68 → fires.
        // Expected: value clamps to cursor position (87.5), NOT 100.
        // cursor_val = (70/80) * 100 = 87.5
        slider(&mut state, &mut value, spec.clone(), &input, 0.5, &mut focus_sys);
        assert!(
            (value - 87.5).abs() < 0.01,
            "repeat should stop at cursor position 87.5, got {value}"
        );

        // Frame 4: repeat fires again (t=0.6). Thumb now at y=70..90, cursor=70 inside → stop paging.
        // is_track_clicking must remain true so the drag transition can still fire.
        slider(&mut state, &mut value, spec.clone(), &input, 0.6, &mut focus_sys);
        assert!(
            (value - 87.5).abs() < 0.01,
            "value should not change after thumb reaches cursor, got {value}"
        );
        assert!(state.is_track_clicking, "is_track_clicking must stay true so drag is still possible");
        assert!(!state.is_dragging);

        // Frame 5: still holding, move mouse 5px (past 4px threshold from initial click at y=70).
        // Drag transition should fire: thumb snaps to cursor, enters drag mode.
        // snap: mouse_coord=75, track_start=0, thumb_len=20 → snapped=75-10=65 → value=65/80*100=81.25
        input.mouse_pos.y = 75.0;
        slider(&mut state, &mut value, spec.clone(), &input, 0.65, &mut focus_sys);
        assert!(state.is_dragging, "should enter drag mode after mouse moves past threshold");
        assert!(!state.is_track_clicking);
        assert!(
            (value - 81.25).abs() < 0.01,
            "snap on drag entry: expected 81.25, got {value}"
        );

        // Frame 6: drag to y=85 → delta=10 → val_delta=12.5 → value=93.75
        input.mouse_pressed = false;
        input.mouse_pos.y = 85.0;
        slider(&mut state, &mut value, spec.clone(), &input, 0.7, &mut focus_sys);
        assert!(
            (value - 93.75).abs() < 0.01,
            "drag: expected 93.75, got {value}"
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
            style: SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: claim_at_ends,
        }
    }

    // ── Standalone slider ──────────────────────────────────────────────────────

    #[test]
    fn test_standalone_slider_wheel_at_min_blocks_propagation() {
        // Even when at minimum, a standalone slider claims both directions,
        // so a hypothetical parent scroll area would never see the event.
        let mut state = SliderState::default();
        let mut value = 0.0; // already at min
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
        slider(&mut state, &mut value, test_spec(0.0, 100.0, true), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2: parent checks — it should NOT have won either direction
        assert!(!focus_sys.is_active_scroll_up(parent_id),
            "parent should not win scroll-up; standalone slider blocked it");
        // Value stays at 0.0 (clamped, can't go below min)
        assert_eq!(value, 0.0);
    }

    #[test]
    fn test_standalone_slider_wheel_at_max_blocks_propagation() {
        let mut state = SliderState::default();
        let mut value = 100.0; // already at max
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id);
        slider(&mut state, &mut value, test_spec(0.0, 100.0, true), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        assert!(!focus_sys.is_active_scroll_down(parent_id),
            "parent should not win scroll-down; standalone slider blocked it");
        assert_eq!(value, 100.0);
    }

    // ── Propagating slider (scrollbar-within-scroll-area mode) ─────────────────

    #[test]
    fn test_propagating_slider_at_min_yields_scroll_up_to_parent() {
        let mut state = SliderState::default();
        let mut value = 0.0; // at min — can't scroll up
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0; // scroll up

        // Frame 1: parent claims, then inner propagating slider
        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);   // parent can scroll up
        focus_sys.claim_scroll_down(parent_id);
        // Inner propagating slider at min: skips claim_scroll_up
        slider(&mut state, &mut value, test_spec(0.0, 100.0, false), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Parent should have retained the scroll-up claim
        assert!(focus_sys.is_active_scroll_up(parent_id),
            "parent should win scroll-up when inner is at its minimum");
        assert_eq!(value, 0.0, "inner value unchanged");
    }

    #[test]
    fn test_propagating_slider_at_max_yields_scroll_down_to_parent() {
        let mut state = SliderState::default();
        let mut value = 100.0; // at max — can't scroll down
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = -1.0; // scroll down

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id); // parent can scroll down
        slider(&mut state, &mut value, test_spec(0.0, 100.0, false), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        assert!(focus_sys.is_active_scroll_down(parent_id),
            "parent should win scroll-down when inner is at its maximum");
        assert_eq!(value, 100.0, "inner value unchanged");
    }

    #[test]
    fn test_propagating_slider_mid_range_wins_both_directions() {
        // When not at an end, the inner propagating slider claims both directions
        // and the parent gets neither.
        let mut state = SliderState::default();
        let mut value = 50.0;
        let mut focus_sys = FocusSystem::new();
        let parent_id = FocusId::new();

        let mut input = Input::new();
        input.mouse_pos = crate::types::Vec2::new(10.0, 50.0);
        input.scroll_delta.y = 1.0;

        focus_sys.begin_frame();
        focus_sys.claim_scroll_up(parent_id);
        focus_sys.claim_scroll_down(parent_id);
        slider(&mut state, &mut value, test_spec(0.0, 100.0, false), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        assert!(!focus_sys.is_active_scroll_up(parent_id), "parent should not win");
        assert!(!focus_sys.is_active_scroll_down(parent_id), "parent should not win");
    }
}
