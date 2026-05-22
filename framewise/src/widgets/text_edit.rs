use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::{Input, TextEvent},
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
pub struct TextEditStyle {
    pub background:   Color,
    pub border:       Color,
    pub focus_border: Color,
    pub border_width: f32,
    pub padding:      f32,
    pub text_size:    f32,
    pub text_color:   Color,
    pub caret_color:  Color,
    pub select_color: Color,
}

impl Default for TextEditStyle {
    fn default() -> Self {
        Self {
            background:   Color::rgb(0.08, 0.08, 0.1),
            border:       Color::rgb(0.3, 0.3, 0.38),
            focus_border: Color::rgb(0.4, 0.6, 0.9),
            border_width: 1.0,
            padding:      4.0,
            text_size:    14.0,
            text_color:   Color::rgb(0.9, 0.9, 0.95),
            caret_color:  Color::rgb(1.0, 1.0, 1.0),
            select_color: Color::rgb(0.2, 0.4, 0.7),
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TextEditState {
    pub value: String,
    pub cursor_byte: usize,
    pub selection_byte: Option<usize>,
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub last_cursor_move_time: f64,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            value: String::new(),
            cursor_byte: 0,
            selection_byte: None,
            focus_id: FocusId::new(),
            is_dragging: false,
            last_cursor_move_time: 0.0,
        }
    }
}

impl TextEditState {
    pub fn new(initial_text: &str) -> Self {
        Self {
            value: initial_text.to_string(),
            cursor_byte: initial_text.len(),
            ..Default::default()
        }
    }

    fn remove_selection(&mut self) {
        if let Some(sel) = self.selection_byte {
            let start = self.cursor_byte.min(sel);
            let end = self.cursor_byte.max(sel);
            self.value.replace_range(start..end, "");
            self.cursor_byte = start;
            self.selection_byte = None;
        }
    }
}

// ── Spec ──────────────────────────────────────────────────────────────────────

pub struct TextEditSpec {
    pub rect:  Rect,
    pub style: TextEditStyle,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct TextEditResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
    pub state:  TextEditState,
}

pub struct TextEditInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for TextEditResult {
    type Info = TextEditInfo;

    fn into_parts(self) -> (DrawCommands, TextEditInfo) {
        (self.draw, TextEditInfo { layout: self.layout })
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

pub fn text_edit<T: TextSystem>(
    mut state: TextEditState,
    spec: TextEditSpec,
    input: &Input,
    time: f64,
    text_system: &mut T,
    focus_sys: &mut FocusSystem,
) -> TextEditResult {
    let mut draw = DrawCommands::new();

    let focused = focus_sys.register(state.focus_id);

    let old_cursor = state.cursor_byte;
    let old_selection = state.selection_byte;

    // Hit test mouse
    let contains = spec.rect.contains(input.mouse_pos);
    
    // Process keyboard events if focused
    if focused {
        for ev in &input.text_events {
            match ev {
                TextEvent::Char(c) => {
                    if !c.is_control() {
                        state.remove_selection();
                        state.value.insert(state.cursor_byte, *c);
                        state.cursor_byte += c.len_utf8();
                    }
                }
                TextEvent::Backspace => {
                    if state.selection_byte.is_some() {
                        state.remove_selection();
                    } else if state.cursor_byte > 0 {
                        // Find previous char boundary
                        let mut prev = state.cursor_byte - 1;
                        while prev > 0 && !state.value.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        state.value.remove(prev);
                        state.cursor_byte = prev;
                    }
                }
                TextEvent::Delete => {
                    if state.selection_byte.is_some() {
                        state.remove_selection();
                    } else if state.cursor_byte < state.value.len() {
                        state.value.remove(state.cursor_byte);
                    }
                }
                TextEvent::CursorLeft { shift, ctrl: _ } => { // ctrl not implemented yet
                    if *shift {
                        if state.selection_byte.is_none() {
                            state.selection_byte = Some(state.cursor_byte);
                        }
                    } else {
                        state.selection_byte = None;
                    }

                    if state.cursor_byte > 0 {
                        let mut prev = state.cursor_byte - 1;
                        while prev > 0 && !state.value.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        state.cursor_byte = prev;
                    }
                }
                TextEvent::CursorRight { shift, ctrl: _ } => {
                    if *shift {
                        if state.selection_byte.is_none() {
                            state.selection_byte = Some(state.cursor_byte);
                        }
                    } else {
                        state.selection_byte = None;
                    }

                    if state.cursor_byte < state.value.len() {
                        let mut next = state.cursor_byte + 1;
                        while next < state.value.len() && !state.value.is_char_boundary(next) {
                            next += 1;
                        }
                        state.cursor_byte = next;
                    }
                }
                TextEvent::CursorHome { shift } => {
                    if *shift && state.selection_byte.is_none() {
                        state.selection_byte = Some(state.cursor_byte);
                    } else if !*shift {
                        state.selection_byte = None;
                    }
                    state.cursor_byte = 0;
                }
                TextEvent::CursorEnd { shift } => {
                    if *shift && state.selection_byte.is_none() {
                        state.selection_byte = Some(state.cursor_byte);
                    } else if !*shift {
                        state.selection_byte = None;
                    }
                    state.cursor_byte = state.value.len();
                }
                TextEvent::SelectAll => {
                    state.selection_byte = Some(0);
                    state.cursor_byte = state.value.len();
                }
            }
        }
    }

    // Safety checks
    if state.cursor_byte > state.value.len() {
        state.cursor_byte = state.value.len();
    }
    if !state.value.is_char_boundary(state.cursor_byte) {
        state.cursor_byte = 0; // fallback
    }

    // Prepare text to get layout handle
    let text_content = if state.value.is_empty() { " " } else { &state.value };
    let layout = text_system.prepare(text_content, spec.style.text_size);
    let handle = layout.handle;

    let inset = spec.style.border_width + spec.style.padding;
    let content_rect = spec.rect.inset(inset);
    let text_y = content_rect.y + (content_rect.h - layout.size.y) / 2.0;

    // Mouse interaction
    if contains && input.mouse_pressed {
        focus_sys.take_focus(state.focus_id);
        
        let relative_x = input.mouse_pos.x - content_rect.x;
        let clicked_byte = text_system.hit_test_x(handle, relative_x);
        let clicked_byte = clicked_byte.min(state.value.len());

        // Handling double/triple clicks
        if input.mouse_click_count == 2 {
            // Select word (simplified: just select all for now, or find word boundaries)
            state.selection_byte = Some(0);
            state.cursor_byte = state.value.len();
        } else if input.mouse_click_count >= 3 {
            // Select line
            state.selection_byte = Some(0);
            state.cursor_byte = state.value.len();
        } else {
            state.cursor_byte = clicked_byte;
            state.selection_byte = None;
            state.is_dragging = true;
        }
    }

    if state.is_dragging {
        if input.mouse_down {
            let relative_x = input.mouse_pos.x - content_rect.x;
            let current_byte = text_system.hit_test_x(handle, relative_x);
            let current_byte = current_byte.min(state.value.len());
            
            if state.selection_byte.is_none() && current_byte != state.cursor_byte {
                state.selection_byte = Some(state.cursor_byte);
            }
            state.cursor_byte = current_byte;
        } else {
            state.is_dragging = false;
        }
    }

    if state.cursor_byte != old_cursor || state.selection_byte != old_selection {
        state.last_cursor_move_time = time;
    }

    // Drawing Background
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: spec.style.background });

    // Border
    if spec.style.border_width > 0.0 {
        let b_color = if focused { spec.style.focus_border } else { spec.style.border };
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect,
            color: b_color,
            width: spec.style.border_width,
        });
    }

    // Selection
    if focused {
        if let Some(sel) = state.selection_byte {
            if sel != state.cursor_byte {
                let start = sel.min(state.cursor_byte);
                let end = sel.max(state.cursor_byte);
                
                let start_x = text_system.measure_byte_x(handle, start);
                let end_x = text_system.measure_byte_x(handle, end);
                
                let sel_rect = Rect::new(
                    content_rect.x + start_x,
                    content_rect.y,
                    end_x - start_x,
                    content_rect.h,
                );
                
                draw.push(DrawCmd::FillRect {
                    rect: sel_rect,
                    color: spec.style.select_color,
                });
            }
        }
    }

    // Text
    if !state.value.is_empty() {
        draw.push(DrawCmd::Text {
            rect:  Rect::new(content_rect.x, text_y, content_rect.w, content_rect.h),
            color: spec.style.text_color,
            handle,
        });
    }

    // Caret
    if focused && state.selection_byte.map_or(true, |s| s == state.cursor_byte) {
        let time_since_move = time - state.last_cursor_move_time;
        // Solid for 0.5s after moving, then blink at 1Hz (0.5s on, 0.5s off)
        let blink_on = if time_since_move < 0.5 {
            true
        } else {
            time_since_move.fract() < 0.5
        };
        
        if blink_on {
            let cursor_x = text_system.measure_byte_x(handle, state.cursor_byte);
            let caret_rect = Rect::new(
                content_rect.x + cursor_x,
                content_rect.y + 2.0,
                1.0,
                content_rect.h - 4.0,
            );
            draw.push(DrawCmd::FillRect {
                rect: caret_rect,
                color: spec.style.caret_color,
            });
        }
    }

    TextEditResult {
        draw,
        layout: LayoutInfo::new(spec.rect, content_rect),
        state,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextHandle, TextLayout};
    use crate::types::Vec2;

    struct DummyTextSys;
    impl TextSystem for DummyTextSys {
        fn prepare(&mut self, _text: &str, _size: f32) -> TextLayout {
            TextLayout {
                handle: TextHandle(0),
                size: Vec2::new(100.0, 16.0),
            }
        }
        fn measure_byte_x(&self, _handle: TextHandle, byte_index: usize) -> f32 {
            // Fake 10 pixels per byte
            byte_index as f32 * 10.0
        }
        fn hit_test_x(&self, _handle: TextHandle, x_offset: f32) -> usize {
            // Fake 10 pixels per byte
            (x_offset / 10.0).round() as usize
        }
    }

    fn spec() -> TextEditSpec {
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 30.0),
            style: TextEditStyle::default(),
        }
    }

    #[test]
    fn test_typing_and_cursor() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("");
        
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame(); // Apply focus shift

        let mut input = Input::default();
        input.text_events.push(TextEvent::Char('a'));
        input.text_events.push(TextEvent::Char('b'));
        input.text_events.push(TextEvent::Char('c'));

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "abc");
        assert_eq!(state.cursor_byte, 3);

        // Move left
        input.text_events.clear();
        input.text_events.push(TextEvent::CursorLeft { shift: false, ctrl: false });
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.cursor_byte, 2);

        // Insert at cursor
        input.text_events.clear();
        input.text_events.push(TextEvent::Char('x'));
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "abxc");
        assert_eq!(state.cursor_byte, 3);
    }

    #[test]
    fn test_backspace_and_delete() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.cursor_byte = 3; // After 'l'
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Backspace);
        
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "helo");
        assert_eq!(state.cursor_byte, 2);

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete);
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "heo"); // Deleted the 'l' after cursor
        assert_eq!(state.cursor_byte, 2);
    }

    #[test]
    fn test_selection_and_replacement() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.cursor_byte = 1;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        // Shift+Right to select "e"
        input.text_events.push(TextEvent::CursorRight { shift: true, ctrl: false });
        // Shift+Right to select "l"
        input.text_events.push(TextEvent::CursorRight { shift: true, ctrl: false });
        
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.selection_byte, Some(1));
        assert_eq!(state.cursor_byte, 3);

        // Type 'a' to replace "el"
        input.text_events.clear();
        input.text_events.push(TextEvent::Char('a'));
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "halo");
        assert_eq!(state.cursor_byte, 2);
        assert_eq!(state.selection_byte, None);
    }

    #[test]
    fn test_mouse_clicking_and_dragging() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(50.0 + spec().style.padding + spec().style.border_width, 15.0); // 50px logical = byte 5
        input.mouse_down = true;
        input.mouse_pressed = true;

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.cursor_byte, 5);
        assert!(state.is_dragging);

        // Drag to right
        input.mouse_pressed = false;
        input.mouse_pos.x += 30.0; // byte 8
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.cursor_byte, 8);

        // Release
        input.mouse_down = false;
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(!state.is_dragging);
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.cursor_byte, 8);
    }

    #[test]
    fn test_double_click_selection() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(20.0 + spec().style.padding + spec().style.border_width, 15.0);
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2; // Double click

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.selection_byte, Some(0));
        assert_eq!(state.cursor_byte, 5); // Selected whole string
    }

    #[test]
    fn test_caret_blink_reset_on_move() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.cursor_byte = 5;
        
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        
        // At t=0.0, caret is idle
        let res = text_edit(state.clone(), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should be visible initially");

        // At t=0.6, caret should be hidden (time_since_move = 0.6 -> fract = 0.6 >= 0.5 -> blink_on = false)
        let res = text_edit(state.clone(), spec(), &input, 0.6, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(!has_caret, "Caret should be hidden during off phase");

        // Now move the cursor at t=0.6
        input.text_events.push(TextEvent::CursorLeft { shift: false, ctrl: false });
        let res = text_edit(state.clone(), spec(), &input, 0.6, &mut text_sys, &mut focus_sys);
        state = res.state;
        
        // Since cursor moved, last_cursor_move_time should become 0.6
        assert_eq!(state.last_cursor_move_time, 0.6);
        
        // And caret should immediately become visible again
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should be visible immediately after moving");
        
        // At t=1.0, time_since_move is 0.4, still visible
        input.text_events.clear();
        let res = text_edit(state.clone(), spec(), &input, 1.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should stay visible for 0.5s after moving");

        // At t=1.2, time_since_move is 0.6, hidden again
        let res = text_edit(state.clone(), spec(), &input, 1.2, &mut text_sys, &mut focus_sys);
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(!has_caret, "Caret should hide after 0.5s of idle");
    }
}
