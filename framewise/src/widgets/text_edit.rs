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

#[derive(Debug)]
pub struct TextEditState {
    pub value: String,
    pub caret_byte: usize,
    pub selection_byte: Option<usize>,
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_word_origin: Option<(usize, usize)>,
    pub last_caret_move_time: f64,
    pub was_focused: bool,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            value: String::new(),
            caret_byte: 0,
            selection_byte: None,
            focus_id: FocusId::new(),
            is_dragging: false,
            drag_word_origin: None,
            last_caret_move_time: 0.0,
            was_focused: false,
        }
    }
}

impl TextEditState {
    pub fn new(initial_text: &str) -> Self {
        Self {
            value: initial_text.to_string(),
            caret_byte: initial_text.len(),
            ..Default::default()
        }
    }

    fn remove_selection(&mut self) {
        if let Some(sel) = self.selection_byte {
            let start = self.caret_byte.min(sel);
            let end = self.caret_byte.max(sel);
            self.value.replace_range(start..end, "");
            self.caret_byte = start;
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

pub enum ClipboardAction {
    Copy(String),
    Cut(String),
}

pub struct TextEditResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
    pub state:  TextEditState,
    pub clipboard_action: Option<ClipboardAction>,
}

pub struct TextEditInfo {
    pub layout: LayoutInfo,
    pub clipboard_action: Option<ClipboardAction>,
    pub state: TextEditState,
}

impl WidgetResult for TextEditResult {
    type Info = TextEditInfo;

    fn into_parts(self) -> (DrawCommands, TextEditInfo) {
        (self.draw, TextEditInfo { layout: self.layout, clipboard_action: self.clipboard_action, state: self.state })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(PartialEq)]
enum CharCategory {
    Space,
    Punctuation,
    Alphanumeric,
}

fn categorize(c: char) -> CharCategory {
    if c.is_whitespace() {
        CharCategory::Space
    } else if c.is_alphanumeric() {
        CharCategory::Alphanumeric
    } else {
        CharCategory::Punctuation
    }
}

pub fn find_word_boundary(text: &str, current: usize, right: bool) -> usize {
    if right {
        if current >= text.len() { return text.len(); }
        let mut it = text[current..].char_indices();
        let (_, first_char) = it.next().unwrap();
        let cat = categorize(first_char);
        
        for (i, c) in it {
            if categorize(c) != cat {
                return current + i;
            }
        }
        text.len()
    } else {
        if current == 0 { return 0; }
        
        let mut prev = current - 1;
        while prev > 0 && !text.is_char_boundary(prev) {
            prev -= 1;
        }
        let first_char = text[prev..].chars().next().unwrap();
        let cat = categorize(first_char);
        
        let mut bounds = prev;
        while prev > 0 {
            let mut check_prev = prev - 1;
            while check_prev > 0 && !text.is_char_boundary(check_prev) {
                check_prev -= 1;
            }
            let c = text[check_prev..].chars().next().unwrap();
            if categorize(c) != cat {
                return bounds;
            }
            bounds = check_prev;
            prev = check_prev;
        }
        
        if prev == 0 {
            let c = text[0..].chars().next().unwrap();
            if categorize(c) == cat {
                return 0;
            }
        }
        
        bounds
    }
}

pub fn word_bounds(text: &str, byte_index: usize) -> (usize, usize) {
    if text.is_empty() { return (0, 0); }
    let safe_index = byte_index.min(text.len() - 1);
    
    let mut start = safe_index;
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    
    let c = text[start..].chars().next().unwrap();
    let cat = categorize(c);
    
    let mut left = start;
    while left > 0 {
        let mut prev = left - 1;
        while prev > 0 && !text.is_char_boundary(prev) {
            prev -= 1;
        }
        let pc = text[prev..].chars().next().unwrap();
        if categorize(pc) != cat { break; }
        left = prev;
    }
    
    let mut right = start + c.len_utf8();
    for (i, nc) in text[right..].char_indices() {
        if categorize(nc) != cat {
            right += i;
            return (left, right);
        }
    }
    (left, text.len())
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

    let mut clipboard_action = None;

    let focused = focus_sys.register(state.focus_id);
    let just_focused = focused && !state.was_focused;

    let old_caret = state.caret_byte;
    let old_selection = state.selection_byte;

    // Hit test mouse
    let contains = spec.rect.contains(input.mouse_pos);
    
    if just_focused {
        if !(contains && input.mouse_pressed) {
            state.selection_byte = Some(0);
            state.caret_byte = state.value.len();
        }
    }
    
    // Process keyboard events if focused
    if focused {
        for ev in &input.text_events {
            match ev {
                TextEvent::Char(c) => {
                    if !c.is_control() {
                        state.remove_selection();
                        state.value.insert(state.caret_byte, *c);
                        state.caret_byte += c.len_utf8();
                    }
                }
                TextEvent::Backspace { ctrl } => {
                    if state.selection_byte.is_some() {
                        state.remove_selection();
                    } else if *ctrl {
                        let prev = find_word_boundary(&state.value, state.caret_byte, false);
                        state.value.replace_range(prev..state.caret_byte, "");
                        state.caret_byte = prev;
                    } else if state.caret_byte > 0 {
                        // Find previous char boundary
                        let mut prev = state.caret_byte - 1;
                        while prev > 0 && !state.value.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        state.value.remove(prev);
                        state.caret_byte = prev;
                    }
                }
                TextEvent::Delete { ctrl } => {
                    if state.selection_byte.is_some() {
                        state.remove_selection();
                    } else if *ctrl {
                        let next = find_word_boundary(&state.value, state.caret_byte, true);
                        state.value.replace_range(state.caret_byte..next, "");
                    } else if state.caret_byte < state.value.len() {
                        state.value.remove(state.caret_byte);
                    }
                }
                TextEvent::CaretLeft { shift, ctrl } => {
                    if *shift {
                        if state.selection_byte.is_none() {
                            state.selection_byte = Some(state.caret_byte);
                        }
                    } else {
                        state.selection_byte = None;
                    }

                    if *ctrl {
                        state.caret_byte = find_word_boundary(&state.value, state.caret_byte, false);
                    } else if state.caret_byte > 0 {
                        let mut prev = state.caret_byte - 1;
                        while prev > 0 && !state.value.is_char_boundary(prev) {
                            prev -= 1;
                        }
                        state.caret_byte = prev;
                    }
                }
                TextEvent::CaretRight { shift, ctrl } => {
                    if *shift {
                        if state.selection_byte.is_none() {
                            state.selection_byte = Some(state.caret_byte);
                        }
                    } else {
                        state.selection_byte = None;
                    }

                    if *ctrl {
                        state.caret_byte = find_word_boundary(&state.value, state.caret_byte, true);
                    } else if state.caret_byte < state.value.len() {
                        let mut next = state.caret_byte + 1;
                        while next < state.value.len() && !state.value.is_char_boundary(next) {
                            next += 1;
                        }
                        state.caret_byte = next;
                    }
                }
                TextEvent::CaretHome { shift } => {
                    if *shift && state.selection_byte.is_none() {
                        state.selection_byte = Some(state.caret_byte);
                    } else if !*shift {
                        state.selection_byte = None;
                    }
                    state.caret_byte = 0;
                }
                TextEvent::CaretEnd { shift } => {
                    if *shift && state.selection_byte.is_none() {
                        state.selection_byte = Some(state.caret_byte);
                    } else if !*shift {
                        state.selection_byte = None;
                    }
                    state.caret_byte = state.value.len();
                }
                TextEvent::SelectAll => {
                    state.selection_byte = Some(0);
                    state.caret_byte = state.value.len();
                }
                TextEvent::Copy => {
                    if let Some(sel) = state.selection_byte {
                        let start = state.caret_byte.min(sel);
                        let end = state.caret_byte.max(sel);
                        if start < end {
                            clipboard_action = Some(ClipboardAction::Copy(state.value[start..end].to_string()));
                        }
                    }
                }
                TextEvent::Cut => {
                    if let Some(sel) = state.selection_byte {
                        let start = state.caret_byte.min(sel);
                        let end = state.caret_byte.max(sel);
                        if start < end {
                            clipboard_action = Some(ClipboardAction::Cut(state.value[start..end].to_string()));
                            state.remove_selection();
                        }
                    }
                }
                TextEvent::Paste(text) => {
                    state.remove_selection();
                    state.value.insert_str(state.caret_byte, &text);
                    state.caret_byte += text.len();
                }
            }
        }
    }

    // Safety checks
    if state.caret_byte > state.value.len() {
        state.caret_byte = state.value.len();
    }
    if !state.value.is_char_boundary(state.caret_byte) {
        state.caret_byte = 0; // fallback
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
            let (start, end) = word_bounds(&state.value, clicked_byte);
            state.selection_byte = Some(start);
            state.caret_byte = end;
            state.is_dragging = true;
            state.drag_word_origin = Some((start, end));
        } else if input.mouse_click_count >= 3 {
            // Select line
            state.selection_byte = Some(0);
            state.caret_byte = state.value.len();
        } else {
            state.caret_byte = clicked_byte;
            state.selection_byte = None;
            state.is_dragging = true;
            state.drag_word_origin = None;
        }
    }

    if state.is_dragging {
        if input.mouse_down {
            let relative_x = input.mouse_pos.x - content_rect.x;
            let current_byte = text_system.hit_test_x(handle, relative_x);
            let current_byte = current_byte.min(state.value.len());
            
            if let Some((orig_start, orig_end)) = state.drag_word_origin {
                let (cur_start, cur_end) = word_bounds(&state.value, current_byte);
                if current_byte < orig_start {
                    state.selection_byte = Some(orig_end);
                    state.caret_byte = cur_start;
                } else {
                    state.selection_byte = Some(orig_start);
                    state.caret_byte = cur_end;
                }
            } else {
                if state.selection_byte.is_none() && current_byte != state.caret_byte {
                    state.selection_byte = Some(state.caret_byte);
                }
                state.caret_byte = current_byte;
            }
        } else {
            state.is_dragging = false;
            state.drag_word_origin = None;
        }
    }

    if state.caret_byte != old_caret || state.selection_byte != old_selection {
        state.last_caret_move_time = time;
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
            if sel != state.caret_byte {
                let start = sel.min(state.caret_byte);
                let end = sel.max(state.caret_byte);
                
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
    if focused && state.selection_byte.map_or(true, |s| s == state.caret_byte) {
        let time_since_move = time - state.last_caret_move_time;
        // Solid for 0.5s after moving, then blink at 1Hz (0.5s on, 0.5s off)
        let blink_on = if time_since_move < 0.5 {
            true
        } else {
            time_since_move.fract() < 0.5
        };
        
        if blink_on {
            let cursor_x = text_system.measure_byte_x(handle, state.caret_byte);
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

    state.was_focused = focused || (contains && input.mouse_pressed);

    TextEditResult {
        draw,
        layout: LayoutInfo::new(spec.rect, content_rect),
        state,
        clipboard_action,
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
            byte_index as f32 * 10.0
        }
        fn hit_test_x(&self, _handle: TextHandle, x_offset: f32) -> usize {
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
        focus_sys.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Char('a'));
        input.text_events.push(TextEvent::Char('b'));
        input.text_events.push(TextEvent::Char('c'));

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "abc");
        assert_eq!(state.caret_byte, 3);

        // Move left
        input.text_events.clear();
        input.text_events.push(TextEvent::CaretLeft { shift: false, ctrl: false });
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.caret_byte, 2);

        // Insert at cursor
        input.text_events.clear();
        input.text_events.push(TextEvent::Char('x'));
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "abxc");
        assert_eq!(state.caret_byte, 3);
    }

    #[test]
    fn test_backspace_and_delete() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 3;
        state.was_focused = true;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Backspace { ctrl: false });
        
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "helo");
        assert_eq!(state.caret_byte, 2);

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: false });
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "heo");
        assert_eq!(state.caret_byte, 2);
    }

    #[test]
    fn test_ctrl_backspace_and_delete() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");
        state.caret_byte = 8; // "hello wo|rld"
        state.was_focused = true;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Backspace { ctrl: true });
        
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "hello rld");
        assert_eq!(state.caret_byte, 6); // end of "hello "

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: true });
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "hello ");
        assert_eq!(state.caret_byte, 6);
    }

    #[test]
    fn test_selection_and_replacement() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 1;
        state.was_focused = true;
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight { shift: true, ctrl: false });
        input.text_events.push(TextEvent::CaretRight { shift: true, ctrl: false });
        
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.selection_byte, Some(1));
        assert_eq!(state.caret_byte, 3);

        input.text_events.clear();
        input.text_events.push(TextEvent::Char('a'));
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.value, "halo");
        assert_eq!(state.caret_byte, 2);
        assert_eq!(state.selection_byte, None);
    }

    #[test]
    fn test_mouse_clicking_and_dragging() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(50.0 + spec().style.padding + spec().style.border_width, 15.0);
        input.mouse_down = true;
        input.mouse_pressed = true;

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.caret_byte, 5);
        assert!(state.is_dragging);
        state.was_focused = true;

        input.mouse_pressed = false;
        input.mouse_pos.x += 30.0;
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.caret_byte, 8);

        input.mouse_down = false;
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(!state.is_dragging);
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.caret_byte, 8);
    }

    #[test]
    fn test_double_click_selection_and_drag() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello rust world");

        let mut input = Input::default();
        // Click on "rust" (byte index 8 -> pixel 80)
        input.mouse_pos = crate::types::Vec2::new(80.0 + spec().style.padding + spec().style.border_width, 15.0);
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2;

        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        // Selection should be "rust" (6 to 10)
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 10);
        assert!(state.is_dragging);
        assert_eq!(state.drag_word_origin, Some((6, 10)));

        // Now drag right to "world" (byte index 14 -> pixel 140)
        input.mouse_pressed = false;
        input.mouse_pos.x = 140.0 + spec().style.padding + spec().style.border_width;
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        // Should select "rust world", so from 6 to 16
        assert_eq!(state.selection_byte, Some(6)); // original start
        assert_eq!(state.caret_byte, 16); // end of "world"

        // Drag left to "hello" (byte index 2 -> pixel 20)
        input.mouse_pos.x = 20.0 + spec().style.padding + spec().style.border_width;
        let res = text_edit(state, spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        // Should select "hello rust", so from 10 to 0
        assert_eq!(state.selection_byte, Some(10)); // original end
        assert_eq!(state.caret_byte, 0); // start of "hello"
    }

    #[test]
    fn test_caret_blink_reset_on_move() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 5;
        state.was_focused = true;
        
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        let mut input = Input::default();
        
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should be visible initially");

        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.6, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(!has_caret, "Caret should be hidden during off phase");

        input.text_events.push(TextEvent::CaretLeft { shift: false, ctrl: false });
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.6, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert_eq!(state.last_caret_move_time, 0.6);
        
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should be visible immediately after moving");
        
        input.text_events.clear();
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 1.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should stay visible for 0.5s after moving");

        let res = text_edit(std::mem::take(&mut state), spec(), &input, 1.2, &mut text_sys, &mut focus_sys);
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(!has_caret, "Caret should hide after 0.5s of idle");
    }

    #[test]
    fn test_word_boundaries() {
        let text = "hello world! 123";
        assert_eq!(word_bounds(text, 0), (0, 5));
        assert_eq!(word_bounds(text, 2), (0, 5));
        assert_eq!(word_bounds(text, 5), (5, 6));
        assert_eq!(word_bounds(text, 6), (6, 11));
        assert_eq!(word_bounds(text, 11), (11, 12));
        assert_eq!(word_bounds(text, 13), (13, 16));

        assert_eq!(find_word_boundary(text, 0, true), 5);
        assert_eq!(find_word_boundary(text, 5, true), 6);
        assert_eq!(find_word_boundary(text, 6, true), 11);

        assert_eq!(find_word_boundary(text, 16, false), 13);
        assert_eq!(find_word_boundary(text, 12, false), 11);
        assert_eq!(find_word_boundary(text, 5, false), 0);
    }

    #[test]
    fn test_focus_select_all() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let input = Input::default();
        
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();
        
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(state.was_focused);
        assert_eq!(state.selection_byte, Some(0));
        assert_eq!(state.caret_byte, 11);
    }

    #[test]
    fn test_mouse_focus_no_select_all() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(50.0 + spec().style.padding + spec().style.border_width, 15.0);
        input.mouse_down = true;
        input.mouse_pressed = true;

        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        
        focus_sys.end_frame();
        focus_sys.begin_frame();
        input.mouse_pressed = false;
        
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;

        assert!(state.was_focused);
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 5);
    }

    #[test]
    fn test_clipboard_actions() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello world");
        
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();

        state.selection_byte = Some(6);
        state.caret_byte = 11;
        state.was_focused = true;

        let mut input = Input::default();
        input.text_events.push(TextEvent::Copy);
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Copy(s)) if s == "world"));
        assert_eq!(state.value, "hello world");

        input.text_events.clear();
        input.text_events.push(TextEvent::Cut);
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Cut(s)) if s == "world"));
        assert_eq!(state.value, "hello ");
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 6);

        input.text_events.clear();
        input.text_events.push(TextEvent::Paste("rust".to_string()));
        let res = text_edit(std::mem::take(&mut state), spec(), &input, 0.0, &mut text_sys, &mut focus_sys);
        state = res.state;
        assert!(res.clipboard_action.is_none());
        assert_eq!(state.value, "hello rust");
        assert_eq!(state.caret_byte, 10);
    }
}
