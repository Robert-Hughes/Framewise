use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::{Input, TextEvent},
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{ClipRect, Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditSpec {
        pub rect: Rect,
        pub style: super::TextEditStyle,
        pub clip_rect: ClipRect,
        pub error: bool,
        pub disabled: bool,
        pub time: f64,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub clipboard_action: Option<ClipboardAction>,
    }

    /// Low-level text edit widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn text_edit<T: TextSystem>(
        spec: TextEditSpec,
        state: &mut TextEditState,
        input: &Input,
        focus_sys: &mut FocusSystem,
        text_system: &mut T,
    ) -> TextEditResult {
        let mut draw = DrawCommands::new();

        let mut clipboard_action = None;

        // Disabled: draw at reduced alpha, no interaction.
        if spec.disabled {
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            // Transparent bg per mockup, just border.
            if spec.style.border_width > 0.0 {
                draw.push(DrawCmd::StrokeRect {
                    rect: spec.rect,
                    color: tint(spec.style.border),
                    width: spec.style.border_width,
                });
            }
            let inset = spec.style.border_width + spec.style.padding;
            let content_rect = spec.rect.inset(inset);
            if !state.value.is_empty() {
                let layout =
                    text_system.prepare(&state.value, spec.style.text_size, spec.style.font);
                let ty = content_rect.y + (content_rect.h - layout.size.y) / 2.0;
                draw.push(DrawCmd::Text {
                    rect: Rect::new(content_rect.x, ty, content_rect.w, content_rect.h),
                    color: tint(spec.style.text_color),
                    handle: layout.handle,
                });
            }
            return TextEditResult {
                draw,
                content_bounds: content_rect,
                clipboard_action: None,
                focused: false,
                input: InputInfo::default(),
            };
        }

        let focused = focus_sys.register(state.focus_id, spec.rect, spec.clip_rect);
        let just_focused = focused && !state.was_focused;

        let old_caret = state.caret_byte;
        let old_selection = state.selection_byte;

        // Hit test mouse
        let is_visible = spec
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
        let contains = spec.rect.contains(input.mouse_pos) && is_visible;

        if just_focused && !(contains && input.mouse_pressed) {
            state.selection_byte = Some(0);
            state.caret_byte = state.value.len();
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
                            state.caret_byte =
                                find_word_boundary(&state.value, state.caret_byte, false);
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
                            state.caret_byte =
                                find_word_boundary(&state.value, state.caret_byte, true);
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
                                clipboard_action = Some(ClipboardAction::Copy(
                                    state.value[start..end].to_string(),
                                ));
                            }
                        }
                    }
                    TextEvent::Cut => {
                        if let Some(sel) = state.selection_byte {
                            let start = state.caret_byte.min(sel);
                            let end = state.caret_byte.max(sel);
                            if start < end {
                                clipboard_action =
                                    Some(ClipboardAction::Cut(state.value[start..end].to_string()));
                                state.remove_selection();
                            }
                        }
                    }
                    TextEvent::Paste(text) => {
                        state.remove_selection();
                        state.value.insert_str(state.caret_byte, text);
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
        let text_content = if state.value.is_empty() {
            " "
        } else {
            &state.value
        };
        let layout = text_system.prepare(text_content, spec.style.text_size, spec.style.font);
        let handle = layout.handle;

        let inset = spec.style.border_width + spec.style.padding;
        let mut content_rect = spec.rect.inset(inset);
        if spec.error {
            // shift content right to clear the 4px error stripe
            content_rect.x += spec.style.error_stripe_width;
            content_rect.w -= spec.style.error_stripe_width;
        }
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
            state.last_caret_move_time = spec.time;
        }

        // Drawing Background
        let bg_color = if spec.error {
            spec.style.error_background
        } else {
            spec.style.background
        };
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: bg_color,
        });

        // Error: 4px rust left stripe
        if spec.error {
            let stripe = Rect::new(spec.rect.x, spec.rect.y, spec.style.error_stripe_width, spec.rect.h);
            draw.push(DrawCmd::FillRect {
                rect: stripe,
                color: spec.style.error_border,
            });
        }

        // Border
        if spec.style.border_width > 0.0 {
            let b_color = if spec.error {
                spec.style.error_border
            } else if focused {
                spec.style.focus_border
            } else {
                spec.style.border
            };
            draw.push(DrawCmd::StrokeRect {
                rect: spec.rect,
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
                rect: Rect::new(content_rect.x, text_y, content_rect.w, content_rect.h),
                color: spec.style.text_color,
                handle,
            });
        }

        // Caret
        if focused && state.selection_byte.is_none_or(|s| s == state.caret_byte) {
            let time_since_move = spec.time - state.last_caret_move_time;
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
                    content_rect.h - spec.style.error_stripe_width,
                );
                draw.push(DrawCmd::FillRect {
                    rect: caret_rect,
                    color: spec.style.caret_color,
                });
            }
        }

        // Text edit owns all arrow keys (caret movement via TextEvent); only Tab navigates focus.
        focus_sys.handle_traversal(focused, input, crate::focus::FocusTraversalKeys::tab_only());

        state.was_focused = focused || (contains && input.mouse_pressed);

        TextEditResult {
            draw,
            content_bounds: content_rect,
            clipboard_action,
            focused,
            input: InputInfo {
                hovered: contains,
                pressed: input.mouse_down && contains,
                clicked: input.mouse_clicked && contains,
            },
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextEditStyle {
    pub background: Color,
    pub error_background: Color,
    pub border: Color,
    pub focus_border: Color,
    pub border_width: f32,
    pub error_border: Color,
    pub error_stripe_width: f32,
    pub padding: f32,
    pub text_size: f32,
    pub font: FontId,
    pub text_color: Color,
    pub caret_color: Color,
    pub select_color: Color,
    pub disabled_alpha: f32,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
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

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ClipboardAction {
    Copy(String),
    Cut(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
    pub clipboard_action: Option<ClipboardAction>,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextEditSpecBuilder {
    pub rect: Option<Rect>,
    pub style: Option<TextEditStyle>,
    pub clip_rect: Option<ClipRect>,
    pub error: Option<bool>,
    pub disabled: Option<bool>,
    pub time: Option<f64>,
}

impl TextEditSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: TextEditStyle) -> Self {
        self.style = Some(style);
        self
    }
    /// Sets the clip rectangle. High-level context functions supply this automatically — only needed when using the raw API directly.
    pub fn clip_rect(mut self, clip_rect: ClipRect) -> Self {
        self.clip_rect = Some(clip_rect);
        self
    }
    pub fn error(mut self, error: bool) -> Self {
        self.error = Some(error);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
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
            self.style = Some(theme.text_edit_style());
        }
        self
    }

    pub fn build(self) -> raw::TextEditSpec {
        raw::TextEditSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
            error: self.error.unwrap_or(false),
            disabled: self.disabled.unwrap_or(false),
            time: self.time.unwrap_or(0.0),
        }
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
        if current >= text.len() {
            return text.len();
        }
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
        if current == 0 {
            return 0;
        }

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
    if text.is_empty() {
        return (0, 0);
    }
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
        if categorize(pc) != cat {
            break;
        }
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

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level text edit widget function using WidgetContext.
///
/// This function accepts a TextEditSpec and calls the low-level raw::text_edit function.
pub fn text_edit<
    T: TextSystem,
    S: LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TextEditSpecBuilder,
    layout_params: S::Params,
    state: &mut TextEditState,
) -> TextEditResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .time(ctx.time)
        .build();
    let result = raw::text_edit(spec, state, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw);

    TextEditResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        clipboard_action: result.clipboard_action,
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::TextEditSpec;
    use super::*;

    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = TextEditSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        assert_eq!(builder.style.unwrap().font, theme.text_edit_style().font);
        assert_eq!(
            builder.style.unwrap().text_size,
            theme.text_edit_style().text_size
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let custom_style = TextEditStyle {
            font: FontId(99),
            ..crate::theme::Theme::framewise().text_edit_style()
        };
        let builder = TextEditSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().font, FontId(99));
    }

    fn spec() -> TextEditSpec {
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 30.0),
            style: crate::theme::Theme::framewise().text_edit_style(),
            clip_rect: None,
            error: false,
            disabled: false,
            time: 0.0,
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

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.value, "abc");
        assert_eq!(state.caret_byte, 3);

        // Move left
        input.text_events.clear();
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.caret_byte, 2);

        // Insert at cursor
        input.text_events.clear();
        input.text_events.push(TextEvent::Char('x'));
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.value, "helo");
        assert_eq!(state.caret_byte, 2);

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: false });
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.value, "hello rld");
        assert_eq!(state.caret_byte, 6); // end of "hello "

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: true });
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.selection_byte, Some(1));
        assert_eq!(state.caret_byte, 3);

        input.text_events.clear();
        input.text_events.push(TextEvent::Char('a'));
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );
        input.mouse_down = true;
        input.mouse_pressed = true;

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.caret_byte, 5);
        assert!(state.is_dragging);
        state.was_focused = true;

        input.mouse_pressed = false;
        input.mouse_pos.x += 24.0;
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.caret_byte, 8);

        input.mouse_down = false;
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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
        // Click on "rust" (byte index 8 -> pixel 64)
        input.mouse_pos = crate::types::Vec2::new(
            64.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2;

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        // Selection should be "rust" (6 to 10)
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 10);
        assert!(state.is_dragging);
        assert_eq!(state.drag_word_origin, Some((6, 10)));

        // Now drag right to "world" (byte index 14 -> pixel 112)
        input.mouse_pressed = false;
        input.mouse_pos.x = 112.0 + spec().style.padding + spec().style.border_width;
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        // Should select "rust world", so from 6 to 16
        assert_eq!(state.selection_byte, Some(6)); // original start
        assert_eq!(state.caret_byte, 16); // end of "world"

        // Drag left to "hello" (byte index 2 -> pixel 16)
        input.mouse_pos.x = 16.0 + spec().style.padding + spec().style.border_width;
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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

        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should be visible initially");

        let res = raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(!has_caret, "Caret should be hidden during off phase");

        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        let res = raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        assert_eq!(state.last_caret_move_time, 0.6);

        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(
            has_caret,
            "Caret should be visible immediately after moving"
        );

        input.text_events.clear();
        let res = raw::text_edit(
            TextEditSpec {
                time: 1.0,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        let has_caret = res.draw.0.iter().any(|cmd| matches!(cmd, DrawCmd::FillRect { color, .. } if *color == spec().style.caret_color));
        assert!(has_caret, "Caret should stay visible for 0.5s after moving");

        let res = raw::text_edit(
            TextEditSpec {
                time: 1.2,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
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

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
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
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );
        input.mouse_down = true;
        input.mouse_pressed = true;

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);

        focus_sys.end_frame();
        focus_sys.begin_frame();
        input.mouse_pressed = false;

        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);

        assert!(state.was_focused);
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 5);
    }

    #[test]
    fn test_text_edit_click_takes_focus() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(10.0, 15.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        focus_sys.begin_frame();
        raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking text edit must request focus"
        );
    }

    #[test]
    fn test_text_edit_clipped_click_does_not_take_focus() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");

        // Mouse is inside the widget rect but outside the clip_rect.
        let clipped_spec = TextEditSpec {
            clip_rect: Some(Rect::new(500.0, 500.0, 200.0, 30.0)),
            ..spec()
        };

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(10.0, 15.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        focus_sys.begin_frame();
        raw::text_edit(
            clipped_spec,
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away text edit must not take focus"
        );
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
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Copy(s)) if s == "world"));
        assert_eq!(state.value, "hello world");

        input.text_events.clear();
        input.text_events.push(TextEvent::Cut);
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Cut(s)) if s == "world"));
        assert_eq!(state.value, "hello ");
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 6);

        input.text_events.clear();
        input.text_events.push(TextEvent::Paste("rust".to_string()));
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);
        assert!(res.clipboard_action.is_none());
        assert_eq!(state.value, "hello rust");
        assert_eq!(state.caret_byte, 10);
    }

    // ── Visual Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_text_edit_visual_normal() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        let input = Input::default();
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.border,
                    width: spec().style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 190.0, 20.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_focused_caret() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();
        focus_sys.begin_frame();

        state.was_focused = true; // ensure state knows

        let input = Input::default();
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.focus_border,
                    width: spec().style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 190.0, 20.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(45.0, 7.0, 1.0, 16.0),
                    color: spec().style.caret_color,
                },
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_focused_selection() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_sys.take_focus(state.focus_id);
        focus_sys.end_frame();
        focus_sys.begin_frame();

        state.was_focused = true;
        state.selection_byte = Some(0);
        state.caret_byte = 5;

        let input = Input::default();
        let res = raw::text_edit(spec(), &mut state, &input, &mut focus_sys, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.focus_border,
                    width: spec().style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(5.0, 5.0, 40.0, 20.0),
                    color: spec().style.select_color,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 190.0, 20.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_error() {
        let mut text_sys = DummyTextSys;
        let mut focus_sys = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        let mut sp = spec();
        sp.error = true;

        let input = Input::default();
        let res = raw::text_edit(sp.clone(), &mut state, &input, &mut focus_sys, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: sp.style.error_background,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 4.0, 30.0),
                    color: sp.style.error_border,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: sp.style.error_border,
                    width: spec().style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(9.0, 7.0, 186.0, 20.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
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
        let mut te_state = TextEditState::default();
        let result = super::text_edit(
            &mut ctx,
            TextEditSpecBuilder::new().rect(custom_rect),
            layout_rect,
            &mut te_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
