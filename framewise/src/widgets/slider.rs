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
    pub width: f32, // Width of track
}

impl Default for SliderStyle {
    fn default() -> Self {
        Self {
            track_color: Color::rgb(0.15, 0.15, 0.18),
            thumb_color: Color::rgb(0.4, 0.4, 0.45),
            thumb_hover_color: Color::rgb(0.5, 0.5, 0.55),
            thumb_drag_color: Color::rgb(0.6, 0.6, 0.65),
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
    pub thumb_size_ratio: Option<f32>, // 0.0 to 1.0 (for scrollbars)
    pub style: SliderStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct SliderState {
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_start_mouse_y: f32,
    pub drag_start_val: f32,
}

impl Default for SliderState {
    fn default() -> Self {
        Self {
            focus_id: FocusId::new(),
            is_dragging: false,
            drag_start_mouse_y: 0.0,
            drag_start_val: 0.0,
        }
    }
}

pub fn slider(
    state: &mut SliderState,
    value: &mut f32,
    spec: SliderSpec,
    input: &Input,
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
        if input.key_pressed_home {
            *value = min;
        }
        if input.key_pressed_end {
            *value = max;
        }
    }
    
    // 3. Drawing
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
            thumb_size_ratio: None,
            style: SliderStyle::default(),
            clip_rect: None,
        };
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // Must be focused to receive keyboard events
        focus_sys.take_focus(state.focus_id);

        input.key_pressed_page_up = true;
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
        assert_eq!(value, 30.0);

        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
        assert_eq!(value, 50.0);
        
        input.key_pressed_page_down = false;
        input.key_pressed_home = true;
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
        assert_eq!(value, 0.0);
        
        input.key_pressed_home = false;
        input.key_pressed_end = true;
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
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
            thumb_size_ratio: None, // Will use style.width (12.0) but maxed to 20.0 for thumb_h
            style: SliderStyle::default(),
            clip_rect: None,
        };
        // Thumb is 20px high. Usable track = 100 - 20 = 80px.
        // So moving 40px down should increase value by 50.
        
        let mut input = Input::new();
        let mut focus_sys = FocusSystem::new();

        // 1. Click on thumb (thumb is at y=0 to y=20)
        input.mouse_pos = crate::types::Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
        assert!(state.is_dragging);
        assert_eq!(state.drag_start_mouse_y, 10.0);

        // 2. Drag down by 40px (mouse y = 50)
        input.mouse_pressed = false;
        input.mouse_pos.y = 50.0;
        slider(&mut state, &mut value, spec.clone(), &input, &mut focus_sys);
        
        // 40 / 80 usable track = 0.5 ratio = 50 value
        assert_eq!(value, 50.0);
    }
}
