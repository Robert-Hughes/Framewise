use crate::{
    draw::DrawCmd,
    focus::{FocusId, FocusSystem},
    input::Input,
    types::{Color, Rect},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderStyle {
    pub track_color: Color,
    pub thumb_color: Color,
    pub thumb_hover_color: Color,
    pub thumb_drag_color: Color,
    pub focus_outline_color: Color,
    pub width: f32, // Width of track
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: Color::rgb(0.15, 0.15, 0.18),
            thumb_color: Color::rgb(0.4, 0.4, 0.45),
            thumb_hover_color: Color::rgb(0.5, 0.5, 0.55),
            thumb_drag_color: Color::rgb(0.6, 0.6, 0.65),
            focus_outline_color: Color::rgb(0.2, 0.5, 0.9),
            width: 12.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SliderSpec {
    pub rect: Rect,
    pub min: f32,
    pub max: f32,
    pub page_step: f32,
    pub step: f32,
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
    pub claim_hover_scroll_at_ends: bool,
}

#[derive(Debug, Clone)]
pub struct SliderState {
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_start_mouse_y: f32,
    pub drag_start_val: f32,
    pub is_track_clicking: bool,
    pub next_repeat_time: f64,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            focus_id: FocusId::new(),
            is_dragging: false,
            drag_start_mouse_y: 0.0,
            drag_start_val: 0.0,
            is_track_clicking: false,
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
    let track_h = track_rect.h;
    
    // Use proportional thumb size, or fallback to fixed size
    let thumb_h = if let Some(ratio) = spec.thumb_size_ratio {
        (track_h * ratio.clamp(0.0, 1.0)).max(20.0)
    } else {
        spec.style.width.max(20.0)
    };
    
    // Usable track length for the thumb's top edge
    let usable_track = (track_h - thumb_h).max(0.0);
    
    let val_ratio = if range > 0.0 {
        (*value - min) / range
    } else {
        0.0
    };
    
    let thumb_y = track_rect.y + (val_ratio * usable_track);
    let thumb_rect = Rect::new(track_rect.x + 2.0, thumb_y, (spec.style.width - 4.0).max(1.0), thumb_h);
    
    // 2. Input Handling
    
    // Drag release
    if state.is_dragging && !input.mouse_down {
        state.is_dragging = false;
    }
    
    // Drag update
    if state.is_dragging {
        if usable_track > 0.0 {
            let delta_y = input.mouse_pos.y - state.drag_start_mouse_y;
            let val_delta = (delta_y / usable_track) * range;
            *value = (state.drag_start_val + val_delta).clamp(min, max);
        }
    }
    
    // Track click release
    if state.is_track_clicking && !input.mouse_down {
        state.is_track_clicking = false;
    }

    // Mouse wheel scrolling
    if is_visible && track_rect.contains(input.mouse_pos) {
        let at_min = *value <= min;
        let at_max = *value >= max;

        if spec.claim_hover_scroll_at_ends {
            // Standalone slider: always claim both directions, preventing
            // scroll from ever propagating to a parent scroll area.
            focus_sys.claim_scroll_up(state.focus_id);
            focus_sys.claim_scroll_down(state.focus_id);
        } else {
            // Scrollbar-within-scroll-area: only claim directions we can move,
            // so a parent can take the other direction when we're at the limit.
            if !at_min { focus_sys.claim_scroll_up(state.focus_id); }
            if !at_max { focus_sys.claim_scroll_down(state.focus_id); }
        }

        if input.scroll_delta.y > 0.0 && focus_sys.is_active_scroll_up(state.focus_id) {
            *value = (*value - input.scroll_delta.y * spec.step).clamp(min, max);
        }
        if input.scroll_delta.y < 0.0 && focus_sys.is_active_scroll_down(state.focus_id) {
            *value = (*value - input.scroll_delta.y * spec.step).clamp(min, max);
        }
    }

    // Mouse clicks
    if input.mouse_pressed && is_visible {
        if thumb_rect.contains(input.mouse_pos) {
            // Clicked thumb -> start dragging
            state.is_dragging = true;
            state.drag_start_mouse_y = input.mouse_pos.y;
            state.drag_start_val = *value;
            focus_sys.take_focus(state.focus_id);
        } else if track_rect.contains(input.mouse_pos) {
            // Clicked track -> page up/down towards mouse
            if input.mouse_pos.y < thumb_rect.y {
                *value = (*value - spec.page_step).clamp(min, max);
            } else if input.mouse_pos.y > thumb_rect.bottom() {
                *value = (*value + spec.page_step).clamp(min, max);
            }
            focus_sys.take_focus(state.focus_id);
            state.is_track_clicking = true;
            state.next_repeat_time = time + 0.5; // 500ms initial delay
        }
    } else if state.is_track_clicking && time >= state.next_repeat_time {
        if track_rect.contains(input.mouse_pos) {
            if input.mouse_pos.y < thumb_rect.y {
                *value = (*value - spec.page_step).clamp(min, max);
                state.next_repeat_time = time + 0.05; // 50ms repeat delay
            } else if input.mouse_pos.y > thumb_rect.bottom() {
                *value = (*value + spec.page_step).clamp(min, max);
                state.next_repeat_time = time + 0.05;
            } else {
                state.is_track_clicking = false;
            }
        } else {
            state.is_track_clicking = false;
        }
    }
    
    // Keyboard Input (if focused)
    if focused {
        if input.key_pressed_page_up {
            *value = (*value - spec.page_step).clamp(min, max);
        }
        if input.key_pressed_page_down {
            *value = (*value + spec.page_step).clamp(min, max);
        }
        if input.key_pressed_up {
            *value = (*value - spec.step).clamp(min, max);
        }
        if input.key_pressed_down {
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
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: true,
        };
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Must be focused to receive keyboard events
        focus_sys.take_focus(state.focus_id);

        input.key_pressed_page_up = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 30.0);

        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, 0.0, &mut focus_sys);
        assert_eq!(value, 50.0);
        
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
            rect: Rect::new(0.0, 0.0, 20.0, 100.0), // track height 100
            min: 0.0,
            max: 100.0, // range 100
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None, // Will use style.width (12.0) but maxed to 20.0 for thumb_h
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: true,
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
        assert_eq!(state.drag_start_mouse_y, 10.0);

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
            rect: Rect::new(0.0, 0.0, 20.0, 100.0), // track height 100
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: true,
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
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: true,
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
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: true,
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

    // Helper to build a standard test spec.
    fn test_spec(min: f32, max: f32, claim_at_ends: bool) -> SliderSpec {
        SliderSpec {
            rect: Rect::new(0.0, 0.0, 20.0, 100.0),
            min,
            max,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
            claim_hover_scroll_at_ends: claim_at_ends,
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
