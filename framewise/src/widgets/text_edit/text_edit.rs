use crate::output::CursorIcon;
use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::{Input, TextEvent},
    layout::{Align, AxisBound, LayoutState, SizeOffer, SizeRequest},
    text::{
        layout_text, CaretPosition, FontId, LineEndKind, LineHeight, LineMetrics, TextBackend,
        TextBounds, TextFlow, TextLayout, TextLineAlign, TextStyle,
    },
    types::{ClipRect, Color, Layer, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    widgets::scroll_area::{ScrollAreaStyle, ScrollState},
};

const TEXT_EDIT_SCROLLBAR_WIDTH: f32 = 5.0;

pub mod raw {
    use super::*;
    use crate::widgets::{
        scroll_area::raw::{end_scroll_area, ScrollAreaSpec},
        scroll_area::{ScrollAxis, ScrollExtent, ScrollLen},
        ScrollbarVisibility,
    };

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditSpec {
        pub rect: Rect,
        pub style: super::TextEditStyle,
        pub placeholder: Option<String>,
        pub clip_rect: ClipRect,
        pub error: bool,
        pub disabled: bool,
        pub time: f64,
        pub layer: Layer,
        pub newline_policy: super::NewlinePolicy,
        pub wrap: bool,
        pub vertical_align: Align,
        pub line_align: TextLineAlign,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditPreLayoutSpec {
        pub style: super::TextEditStyle,
        pub wrap: bool,
        pub line_align: TextLineAlign,
        pub error: bool,
        pub disabled: bool,
        pub newline_policy: super::NewlinePolicy,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
        caret_state: CaretState,
        old_caret: CaretPosition,
        old_selection_anchor: Option<CaretPosition>,
        clipboard_action: Option<ClipboardAction>,
        processed_text_events: usize,
        newline_inserted_in_pre_layout: bool,
        enter_key_handled_in_pre_layout: bool,
        selection_only_action: bool,
        just_focused_selection_applied: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub clipboard_action: Option<ClipboardAction>,
        pub cursor_icon: Option<CursorIcon>,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum VerticalCaretDirection {
        Up,
        Down,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct VerticalCaretMove {
        caret: CaretPosition,
        byte: usize,
        needs_layout_sync: bool,
    }

    #[allow(clippy::too_many_arguments)]
    fn move_caret_vertical<T: TextBackend>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_backend: &mut T,
        caret: CaretPosition,
        caret_byte: usize,
        caret_is_current: bool,
        start_byte: usize,
        direction: VerticalCaretDirection,
        line_count: usize,
    ) -> VerticalCaretMove {
        let prepared = prepare_text_edit_layout(text_content, spec, text_style, text_backend);
        let layout = &prepared.layout;

        let visual_position = if caret_is_current && start_byte == caret_byte {
            caret
        } else {
            layout.caret_position_at_insertion_byte(start_byte)
        };
        let caret_geom = layout.caret_geom(visual_position);
        let current_line_idx = layout.visual_line_index_for_caret(visual_position);

        let line_len = layout.lines.len();
        let (target_line_idx, target_clamped) = match direction {
            VerticalCaretDirection::Up => (
                current_line_idx.saturating_sub(line_count),
                line_count > current_line_idx,
            ),
            VerticalCaretDirection::Down => (
                (current_line_idx + line_count).min(line_len.saturating_sub(1)),
                current_line_idx + line_count >= line_len,
            ),
        };

        if target_clamped {
            let byte = match direction {
                VerticalCaretDirection::Up => 0,
                VerticalCaretDirection::Down => text_content.len(),
            };
            return VerticalCaretMove {
                caret,
                byte,
                needs_layout_sync: true,
            };
        }

        let new_caret = layout.caret_at_visual_line_x(target_line_idx, caret_geom.x);
        let byte = new_caret.insertion_byte_hint().min(text_content.len());
        VerticalCaretMove {
            caret: new_caret,
            byte,
            needs_layout_sync: false,
        }
    }

    fn page_line_count<T: TextBackend>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_backend: &mut T,
        caret_byte: usize,
        scroll_outer_height: f32,
    ) -> usize {
        let prepared = prepare_text_edit_layout(text_content, spec, text_style, text_backend);
        let layout = &prepared.layout;
        let caret = layout.caret_position_at_insertion_byte(caret_byte);
        let line_height = layout.caret_geom(caret).height.max(1.0);
        (scroll_outer_height / line_height).floor().max(1.0) as usize
    }

    pub(super) fn text_edit_scroll_outer_width(
        outer_width: f32,
        style: TextEditStyle,
        error: bool,
    ) -> f32 {
        let border_width = style.border.map_or(0.0, |s| s.width);
        let mut w = (outer_width - border_width * 2.0).max(0.0);
        if error {
            w = (w - style.error_stripe_width).max(0.0);
        }
        w
    }

    pub(super) fn text_edit_available_text_width(
        scroll_outer_width: f32,
        style: TextEditStyle,
    ) -> f32 {
        (scroll_outer_width - 2.0 * style.padding_x).max(0.0)
    }

    pub(super) fn text_edit_reserved_vertical_width(
        reserve_vertical_scrollbar: bool,
        available_text_width: f32,
    ) -> f32 {
        if reserve_vertical_scrollbar && available_text_width > TEXT_EDIT_SCROLLBAR_WIDTH * 2.0 {
            TEXT_EDIT_SCROLLBAR_WIDTH
        } else {
            0.0
        }
    }

    pub(super) fn text_edit_content_width(
        available_text_width: f32,
        reserve_vertical_scrollbar: bool,
    ) -> f32 {
        let reserved =
            text_edit_reserved_vertical_width(reserve_vertical_scrollbar, available_text_width);
        (available_text_width - reserved).max(0.0)
    }

    // The sizing path and interactive layout path must agree on how much
    // horizontal space is consumed by border, error stripe, padding, and the
    // reserved vertical scrollbar gutter. Otherwise an auto-sized wrapped
    // TextEdit can request a width based on one content width, then be laid out
    // with a different content width and rewrap immediately.
    pub fn pre_layout_text_edit<T: TextBackend>(
        spec: &TextEditPreLayoutSpec,
        offer: SizeOffer,
        state: &mut TextEditState,
        input: &Input,
        focus_system: &FocusSystem,
        text_backend: &mut T,
    ) -> TextEditPreLayoutResult {
        let processed = spec.newline_policy.process(&state.value);
        if processed != state.value {
            state.value = processed.into_owned();
        }

        let old_caret = state.caret;
        let old_selection_anchor = state.selection_anchor;
        let mut caret_state = caret_state_from_text_edit_state(state);
        let focused_at_start = focus_system.current_keyboard_focus() == Some(state.focus_id);
        let just_focused = focused_at_start && !state.had_keyboard_focus;
        let mut clipboard_action = None;
        let mut processed_text_events = 0;
        let mut newline_inserted = false;
        let mut enter_key_handled = false;
        let mut selection_only_action = false;
        let mut just_focused_selection_applied = false;

        if !spec.disabled && just_focused && !state.suppress_select_all_on_next_focus {
            caret_state.selection_byte = Some(0);
            caret_state.caret_byte = state.value.len();
            caret_state.caret_needs_layout_sync = true;
            selection_only_action = true;
            just_focused_selection_applied = true;
        }

        if !spec.disabled && focused_at_start {
            for ev in &input.text_events {
                if process_pre_layout_text_event(
                    ev,
                    state,
                    &mut caret_state,
                    spec.newline_policy,
                    &mut clipboard_action,
                    &mut selection_only_action,
                    &mut newline_inserted,
                ) {
                    processed_text_events += 1;
                } else {
                    break;
                }
            }

            let all_text_events_consumed = processed_text_events == input.text_events.len();
            if all_text_events_consumed && input.key_pressed_enter {
                if !newline_inserted {
                    insert_text_with_newline_policy(
                        &mut state.value,
                        &mut caret_state.caret_byte,
                        &mut caret_state.selection_byte,
                        &mut caret_state.caret_needs_layout_sync,
                        spec.newline_policy,
                        "\n",
                    );
                }
                enter_key_handled = true;
            }
        }

        sanitize_caret_state_for_value(&mut caret_state, &state.value);
        let size_request =
            text_edit_size_request_for_value(spec, offer, &state.value, text_backend);
        TextEditPreLayoutResult {
            size_request,
            caret_state,
            old_caret,
            old_selection_anchor,
            clipboard_action,
            processed_text_events,
            newline_inserted_in_pre_layout: newline_inserted,
            enter_key_handled_in_pre_layout: enter_key_handled,
            selection_only_action,
            just_focused_selection_applied,
        }
    }

    #[cfg(test)]
    pub(super) fn post_layout_only_pre_layout_result(
        state: &mut TextEditState,
    ) -> TextEditPreLayoutResult {
        let mut caret_state = caret_state_from_text_edit_state(state);
        sanitize_caret_state_for_value(&mut caret_state, &state.value);
        TextEditPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
            caret_state,
            old_caret: state.caret,
            old_selection_anchor: state.selection_anchor,
            clipboard_action: None,
            processed_text_events: 0,
            newline_inserted_in_pre_layout: false,
            enter_key_handled_in_pre_layout: false,
            selection_only_action: false,
            just_focused_selection_applied: false,
        }
    }

    fn process_pre_layout_text_event(
        ev: &TextEvent,
        state: &mut TextEditState,
        caret_state: &mut CaretState,
        newline_policy: NewlinePolicy,
        clipboard_action: &mut Option<ClipboardAction>,
        selection_only_action: &mut bool,
        newline_inserted: &mut bool,
    ) -> bool {
        match ev {
            TextEvent::Char(_)
            | TextEvent::Paste(_)
            | TextEvent::Backspace { .. }
            | TextEvent::Delete { .. } => {
                // Text insertion and deletion only depend on the current logical byte range,
                // selection, newline policy, and word-boundary rules. They do not need the
                // final widget rect or prepared text layout, so applying them before sizing is
                // behavior-preserving for the safe prefix.
                apply_rect_independent_text_event(
                    ev,
                    state,
                    caret_state,
                    newline_policy,
                    clipboard_action,
                    selection_only_action,
                    newline_inserted,
                );
                true
            }
            TextEvent::SelectAll | TextEvent::Copy | TextEvent::Cut => {
                // These operate on the logical selection range. SelectAll affects caret/selection
                // bookkeeping, and Copy/Cut may produce a clipboard action, but none of them need
                // hit-testing, scroll offsets, or visual caret geometry.
                apply_rect_independent_text_event(
                    ev,
                    state,
                    caret_state,
                    newline_policy,
                    clipboard_action,
                    selection_only_action,
                    newline_inserted,
                );
                true
            }
            TextEvent::CaretLeft { .. }
            | TextEvent::CaretRight { .. }
            | TextEvent::CaretHome { .. }
            | TextEvent::CaretEnd { .. } => {
                // Horizontal/home/end movement may consult the prepared layout for visual caret
                // traversal, especially with wrapping and bidirectional visual positions. Leave it
                // in post-layout, and stop the pre-layout prefix so later text events keep order.
                false
            }
            TextEvent::CaretUp { .. } | TextEvent::CaretDown { .. } => {
                // Vertical movement is inherently geometry-dependent because target lines and x
                // positions come from the final prepared layout. It must remain post-layout.
                false
            }
        }
    }

    fn apply_rect_independent_text_event(
        ev: &TextEvent,
        state: &mut TextEditState,
        caret_state: &mut CaretState,
        newline_policy: NewlinePolicy,
        clipboard_action: &mut Option<ClipboardAction>,
        selection_only_action: &mut bool,
        newline_inserted: &mut bool,
    ) {
        match ev {
            TextEvent::Char(c) => {
                let is_newline = *c == '\n' || *c == '\r';
                if !c.is_control() || is_newline {
                    let mut buf = [0; 4];
                    let char_str = c.encode_utf8(&mut buf);
                    insert_text_with_newline_policy(
                        &mut state.value,
                        &mut caret_state.caret_byte,
                        &mut caret_state.selection_byte,
                        &mut caret_state.caret_needs_layout_sync,
                        newline_policy,
                        char_str,
                    );
                    if is_newline {
                        *newline_inserted = true;
                    }
                }
            }
            TextEvent::Backspace { ctrl } => {
                if caret_state.selection_byte.is_some() {
                    remove_selection(
                        &mut state.value,
                        &mut caret_state.caret_byte,
                        &mut caret_state.selection_byte,
                    );
                    caret_state.caret_needs_layout_sync = true;
                } else if *ctrl {
                    let prev = find_word_boundary(&state.value, caret_state.caret_byte, false);
                    state.value.replace_range(prev..caret_state.caret_byte, "");
                    caret_state.caret_byte = prev;
                    caret_state.caret_needs_layout_sync = true;
                } else if caret_state.caret_byte > 0 {
                    let mut prev = caret_state.caret_byte - 1;
                    while prev > 0 && !state.value.is_char_boundary(prev) {
                        prev -= 1;
                    }
                    state.value.remove(prev);
                    caret_state.caret_byte = prev;
                    caret_state.caret_needs_layout_sync = true;
                }
            }
            TextEvent::Delete { ctrl } => {
                if caret_state.selection_byte.is_some() {
                    remove_selection(
                        &mut state.value,
                        &mut caret_state.caret_byte,
                        &mut caret_state.selection_byte,
                    );
                    caret_state.caret_needs_layout_sync = true;
                } else if *ctrl {
                    let next = find_word_boundary(&state.value, caret_state.caret_byte, true);
                    state.value.replace_range(caret_state.caret_byte..next, "");
                    caret_state.caret_needs_layout_sync = true;
                } else if caret_state.caret_byte < state.value.len() {
                    state.value.remove(caret_state.caret_byte);
                    caret_state.caret_needs_layout_sync = true;
                }
            }
            TextEvent::SelectAll => {
                caret_state.selection_byte = Some(0);
                caret_state.caret_byte = state.value.len();
                caret_state.caret_needs_layout_sync = true;
                *selection_only_action = true;
            }
            TextEvent::Copy => {
                if let Some(sel) = caret_state.selection_byte {
                    let start = caret_state.caret_byte.min(sel);
                    let end = caret_state.caret_byte.max(sel);
                    if start < end {
                        *clipboard_action =
                            Some(ClipboardAction::Copy(state.value[start..end].to_string()));
                    }
                }
            }
            TextEvent::Cut => {
                if let Some(sel) = caret_state.selection_byte {
                    let start = caret_state.caret_byte.min(sel);
                    let end = caret_state.caret_byte.max(sel);
                    if start < end {
                        *clipboard_action =
                            Some(ClipboardAction::Cut(state.value[start..end].to_string()));
                        remove_selection(
                            &mut state.value,
                            &mut caret_state.caret_byte,
                            &mut caret_state.selection_byte,
                        );
                        caret_state.caret_needs_layout_sync = true;
                    }
                }
            }
            TextEvent::Paste(text) => {
                insert_text_with_newline_policy(
                    &mut state.value,
                    &mut caret_state.caret_byte,
                    &mut caret_state.selection_byte,
                    &mut caret_state.caret_needs_layout_sync,
                    newline_policy,
                    text,
                );
            }
            TextEvent::CaretLeft { .. }
            | TextEvent::CaretRight { .. }
            | TextEvent::CaretHome { .. }
            | TextEvent::CaretEnd { .. }
            | TextEvent::CaretUp { .. }
            | TextEvent::CaretDown { .. } => {}
        }
    }

    fn text_edit_size_request_for_value<T: TextBackend>(
        spec: &TextEditPreLayoutSpec,
        offer: SizeOffer,
        value: &str,
        text_backend: &mut T,
    ) -> SizeRequest {
        let (max_width, reserved_vertical_width) = if spec.wrap {
            match offer.width {
                AxisBound::Exact(w) | AxisBound::AtMost(w) => {
                    let scroll_outer_width =
                        text_edit_scroll_outer_width(w, spec.style, spec.error);
                    let available_text_width =
                        text_edit_available_text_width(scroll_outer_width, spec.style);
                    let reserve_scrollbar = true;
                    let reserved =
                        text_edit_reserved_vertical_width(reserve_scrollbar, available_text_width);
                    let cw = text_edit_content_width(available_text_width, reserve_scrollbar);
                    (Some(cw), reserved)
                }
                AxisBound::Unbounded => (None, TEXT_EDIT_SCROLLBAR_WIDTH),
            }
        } else {
            (None, 0.0)
        };

        let layout = layout_text(
            text_backend,
            value,
            to_text_style(spec.style, spec.wrap, spec.line_align),
            TextBounds {
                max_width,
                max_height: None,
            },
        );
        let metrics = layout.metrics();

        let border_width = spec.style.border.map_or(0.0, |s| s.width);
        let mut preferred_width =
            metrics.logical_size.x + spec.style.padding_x * 2.0 + border_width * 2.0;
        if spec.error {
            preferred_width += spec.style.error_stripe_width;
        }
        preferred_width += reserved_vertical_width;

        SizeRequest::preferred(Vec2::new(
            preferred_width,
            (metrics.logical_size.y + (border_width + spec.style.padding_y) * 2.0)
                .max(spec.style.min_height),
        ))
    }

    /// Low-level text edit widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_text_edit<T: TextBackend>(
        spec: TextEditSpec,
        pre_layout: TextEditPreLayoutResult,
        state: &mut TextEditState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> TextEditResult {
        let text_style = to_text_style(spec.style, spec.wrap, spec.line_align);
        let processed = spec.newline_policy.process(&state.value);
        if processed != state.value {
            state.value = processed.into_owned();
        }

        let mut clipboard_action = pre_layout.clipboard_action;
        // selection_only_action tracks whether the selection or caret was changed by a bulk selection
        // event (like double-clicking a word, triple-clicking a line, focus gain, or Ctrl-A).
        // For these actions, we want to minimize scrolling by only adjusting the viewport
        // as much as necessary to bring the selection range into view, rather than jumping
        // to the caret position.
        let mut selection_only_action = pre_layout.selection_only_action;

        // Disabled: draw at reduced alpha, no interaction.
        if spec.disabled {
            //TODO: update this to match new layout? Perhaps remove this separate branch entirely?
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            let tint_stroke = |s: Stroke| Stroke::new(tint(s.color), s.width);
            cmds.push_border_rect(
                spec.rect,
                spec.style.border.map(tint_stroke),
                BorderPlacement::Inside,
                spec.layer.get_z(),
            );
            let border_width = spec.style.border.map_or(0.0, |s| s.width);
            let inset_x = border_width + spec.style.padding_x;
            let inset_y = border_width + spec.style.padding_y;
            let content_rect = Rect::new(
                spec.rect.x + inset_x,
                spec.rect.y + inset_y,
                (spec.rect.w - inset_x * 2.0).max(0.0),
                (spec.rect.h - inset_y * 2.0).max(0.0),
            );
            let disabled_text = if state.value.is_empty() {
                spec.placeholder.as_deref()
            } else {
                Some(state.value.as_str())
            };
            if let Some(text) = disabled_text {
                let layout = layout_text(
                    text_backend,
                    text,
                    text_style,
                    TextBounds {
                        max_width: Some(content_rect.w),
                        max_height: Some(content_rect.h),
                    },
                );
                let metrics = layout.metrics();
                let ty = match spec.vertical_align {
                    Align::Start => content_rect.y,
                    Align::Center => {
                        content_rect.y + (content_rect.h - metrics.logical_size.y) / 2.0
                    }
                    Align::End => content_rect.y + content_rect.h - metrics.logical_size.y,
                };
                let text_rect = Rect::new(content_rect.x, ty, content_rect.w, content_rect.h);
                let color = if state.value.is_empty() {
                    spec.style.placeholder_color
                } else {
                    spec.style.text_color
                };
                layout.emit_glyphs(
                    cmds,
                    text_backend,
                    Vec2::new(text_rect.x, text_rect.y),
                    tint(color),
                    spec.layer.get_z(),
                );
            }
            return TextEditResult {
                content_bounds: content_rect,
                clipboard_action: None,
                focused: false,
                input: InputInfo::default(),
                cursor_icon: None,
            };
        }

        let focused = focus_system.register_keyboard(state.focus_id, spec.rect, spec.clip_rect);
        let just_focused = focused && !state.had_keyboard_focus;

        // Hit test mouse
        let is_visible = spec
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));

        // Hit-test the editable/scrollable interior, not the decorative border.
        // Otherwise the border strip beside an internal scrollbar can make the text edit
        // look hovered for a brief flicker as the cursor moves onto the scrollbar from outside.
        let hit_rect = text_edit_scroll_outer_rect(&spec);
        let contains_raw = hit_rect.contains(input.mouse_pos) && is_visible;

        if contains_raw {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = focus_system.is_hover_active(state.focus_id);
        let contains = contains_raw && is_hover_active;

        let old_caret = pre_layout.old_caret;
        let old_selection = pre_layout.old_selection_anchor;

        let mut caret_state = pre_layout.caret_state;

        if just_focused
            && !pre_layout.just_focused_selection_applied
            && !state.suppress_select_all_on_next_focus
        {
            caret_state.selection_byte = Some(0);
            caret_state.caret_byte = state.value.len();
            caret_state.caret_needs_layout_sync = true;
            selection_only_action = true;
        }

        // Process keyboard events if focused
        if focused {
            let mut newline_inserted = pre_layout.newline_inserted_in_pre_layout;
            for ev in input
                .text_events
                .iter()
                .skip(pre_layout.processed_text_events)
            {
                if process_pre_layout_text_event(
                    ev,
                    state,
                    &mut caret_state,
                    spec.newline_policy,
                    &mut clipboard_action,
                    &mut selection_only_action,
                    &mut newline_inserted,
                ) {
                    continue;
                }

                match ev {
                    TextEvent::CaretLeft { shift, ctrl } => {
                        caret_state.move_horizontal(
                            *shift,
                            *ctrl,
                            MovementDirection::Backward,
                            &state.value,
                            &spec,
                            text_backend,
                        );
                    }
                    TextEvent::CaretRight { shift, ctrl } => {
                        caret_state.move_horizontal(
                            *shift,
                            *ctrl,
                            MovementDirection::Forward,
                            &state.value,
                            &spec,
                            text_backend,
                        );
                    }
                    TextEvent::CaretUp { shift } => {
                        caret_state.move_vertical(
                            *shift,
                            VerticalCaretDirection::Up,
                            1,
                            &state.value,
                            &spec,
                            text_backend,
                        );
                    }
                    TextEvent::CaretDown { shift } => {
                        caret_state.move_vertical(
                            *shift,
                            VerticalCaretDirection::Down,
                            1,
                            &state.value,
                            &spec,
                            text_backend,
                        );
                    }
                    TextEvent::CaretHome { shift, ctrl } => {
                        caret_state.move_home(*shift, *ctrl, &state.value, &spec, text_backend);
                    }
                    TextEvent::CaretEnd { shift, ctrl } => {
                        caret_state.move_end(*shift, *ctrl, &state.value, &spec, text_backend);
                    }
                    TextEvent::Char(_)
                    | TextEvent::Backspace { .. }
                    | TextEvent::Delete { .. }
                    | TextEvent::SelectAll
                    | TextEvent::Copy
                    | TextEvent::Cut
                    | TextEvent::Paste(_) => unreachable!(
                        "rect-independent text events are handled before geometry-dependent events"
                    ),
                }
            }

            if !pre_layout.enter_key_handled_in_pre_layout
                && input.key_pressed_enter
                && !newline_inserted
            {
                insert_text_with_newline_policy(
                    &mut state.value,
                    &mut caret_state.caret_byte,
                    &mut caret_state.selection_byte,
                    &mut caret_state.caret_needs_layout_sync,
                    spec.newline_policy,
                    "\n",
                );
            }

            if input.key_pressed_page_up || input.key_pressed_page_down {
                let direction = if input.key_pressed_page_down {
                    VerticalCaretDirection::Down
                } else {
                    VerticalCaretDirection::Up
                };
                let shift = input.modifier_shift;

                let start_byte = caret_state.get_movement_start_byte(
                    shift,
                    match direction {
                        VerticalCaretDirection::Up => MovementDirection::Backward,
                        VerticalCaretDirection::Down => MovementDirection::Forward,
                    },
                );

                let scroll_outer_rect = text_edit_scroll_outer_rect(&spec);
                let line_count = page_line_count(
                    state.value.as_str(),
                    &spec,
                    text_style,
                    text_backend,
                    start_byte,
                    scroll_outer_rect.h,
                );

                caret_state.move_vertical(
                    shift,
                    direction,
                    line_count,
                    &state.value,
                    &spec,
                    text_backend,
                );
            }
        }

        let mut caret = caret_state.caret;
        let mut caret_byte = caret_state.caret_byte;
        let mut caret_needs_layout_sync = caret_state.caret_needs_layout_sync;
        let mut selection_byte = caret_state.selection_byte;

        let sanitized = {
            let mut sanitized = CaretState {
                caret,
                caret_byte,
                caret_needs_layout_sync,
                selection_byte,
            };
            sanitize_caret_state_for_value(&mut sanitized, &state.value);
            sanitized
        };
        caret = sanitized.caret;
        caret_byte = sanitized.caret_byte;
        caret_needs_layout_sync = sanitized.caret_needs_layout_sync;
        selection_byte = sanitized.selection_byte;

        let text_content = state.value.as_str();
        let prepared = prepare_text_edit_layout(text_content, &spec, text_style, text_backend);
        let metrics = prepared.layout.metrics();
        let scroll_outer_rect = prepared.scroll_outer_rect;
        let inner_scroll_size = prepared.inner_scroll_size;
        let layout = &prepared.layout;

        // Drawing Background
        let bg_color = if spec.error {
            spec.style.error_background
        } else if contains {
            spec.style.background_hovered
        } else {
            spec.style.background
        };
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: bg_color,
            z: spec.layer.get_z(),
        });

        // Error: 4px rust left stripe
        if spec.error {
            let stripe = Rect::new(
                spec.rect.x,
                spec.rect.y,
                spec.style.error_stripe_width,
                spec.rect.h,
            );
            cmds.push(DrawCmd::FillRect {
                rect: stripe,
                color: spec
                    .style
                    .error_border
                    .map_or(Color::TRANSPARENT, |s| s.color),
                z: spec.layer.get_z(),
            });
        }

        // Border
        let border = if spec.error {
            spec.style.error_border
        } else if focused {
            spec.style.focus_border
        } else {
            spec.style.border
        };
        cmds.push_border_rect(
            spec.rect,
            border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let scroll_spec = raw::ScrollAreaSpec {
            rect: scroll_outer_rect,
            horizontal: ScrollAxis {
                extent: ScrollExtent::Exact(ScrollLen::Px(inner_scroll_size.x)),
                vis: ScrollbarVisibility::Auto,
            },
            vertical: ScrollAxis {
                extent: ScrollExtent::Exact(ScrollLen::Px(inner_scroll_size.y)),
                vis: ScrollbarVisibility::Auto,
            },
            clip_rect: spec.clip_rect,
            time: spec.time,
            style: spec.style.scroll_area_style,
            layer: spec.layer,
            keyboard_focusable: false,
        };
        let scroll_result = crate::widgets::scroll_area::raw::begin_scroll_area(
            scroll_spec,
            crate::widgets::scroll_area::raw::ScrollAreaPreLayoutResult {
                size_request: crate::layout::SizeRequest::UNKNOWN,
            },
            &mut state.scroll,
            input,
            focus_system,
            cmds,
        );

        // Mouse input is interpreted using the scroll offset captured at
        // begin_scroll_area(), matching the scroll-area frame model. Programmatic
        // caret reveal can update state.scroll.offset later in this call, and drawing
        // uses that final offset so caret movement/typing does not visually lag by one
        // frame.
        let input_text_origin = text_origin_for_scroll(
            &spec,
            scroll_outer_rect,
            metrics.logical_size,
            prepared.block_align_offset_x,
            scroll_result.offset,
        );

        // Mouse interaction
        if contains && input.mouse_pressed {
            if !focused {
                state.suppress_select_all_on_next_focus = true;
            }
            focus_system.take_keyboard_focus(state.focus_id);

            let relative_pos = Vec2::new(
                input.mouse_pos.x - input_text_origin.x,
                input.mouse_pos.y - input_text_origin.y,
            );
            let clicked_caret = layout.hit_test_caret(relative_pos);
            let clicked_byte = clicked_caret.insertion_byte_hint().min(state.value.len());

            // Handling repeated clicks
            if input.mouse_click_count == 2 {
                let cluster_byte = layout.hit_test_cluster(relative_pos);
                let (start, end) = word_bounds(&state.value, cluster_byte);
                selection_byte = Some(start);
                caret_byte = end;
                caret_needs_layout_sync = true;
                state.is_dragging = true;
                state.drag_word_origin = Some((start, end));
                state.drag_line_origin = None;
                selection_only_action = true;
            } else if input.mouse_click_count == 3 {
                let (start, end) = logical_line_bounds(&state.value, clicked_byte);
                selection_byte = Some(start);
                caret_byte = end;
                caret_needs_layout_sync = true;
                state.is_dragging = true;
                state.drag_word_origin = None;
                state.drag_line_origin = Some((start, end));
                selection_only_action = true;
            } else if input.mouse_click_count >= 4 {
                selection_byte = Some(0);
                caret_byte = state.value.len();
                caret_needs_layout_sync = true;
                state.is_dragging = false;
                state.drag_word_origin = None;
                state.drag_line_origin = None;
                selection_only_action = true;
            } else {
                caret_byte = clicked_byte;
                caret = clicked_caret;
                caret_needs_layout_sync = false;
                selection_byte = None;
                state.is_dragging = true;
                state.drag_word_origin = None;
                state.drag_line_origin = None;
            }
        }

        if state.is_dragging {
            if input.mouse_down {
                let relative_pos = Vec2::new(
                    input.mouse_pos.x - input_text_origin.x,
                    input.mouse_pos.y - input_text_origin.y,
                );
                let current_caret = layout.hit_test_caret(relative_pos);
                let current_byte = current_caret.insertion_byte_hint().min(state.value.len());

                if let Some((orig_start, orig_end)) = state.drag_word_origin {
                    let cluster_byte = layout.hit_test_cluster(relative_pos);
                    let (cur_start, cur_end) = word_bounds(&state.value, cluster_byte);
                    if cluster_byte < orig_start {
                        selection_byte = Some(orig_end);
                        caret_byte = cur_start;
                    } else {
                        selection_byte = Some(orig_start);
                        caret_byte = cur_end;
                    }
                    caret_needs_layout_sync = true;
                } else if let Some((orig_start, orig_end)) = state.drag_line_origin {
                    let (cur_start, cur_end) = logical_line_bounds(&state.value, current_byte);
                    if current_byte < orig_start {
                        selection_byte = Some(orig_end);
                        caret_byte = cur_start;
                    } else {
                        selection_byte = Some(orig_start);
                        caret_byte = cur_end;
                    }
                    caret_needs_layout_sync = true;
                } else {
                    if selection_byte.is_none() && current_byte != caret_byte {
                        selection_byte = Some(caret_byte);
                    }
                    caret_byte = current_byte;
                    caret = current_caret;
                    caret_needs_layout_sync = false;
                }
            } else {
                state.is_dragging = false;
                state.drag_word_origin = None;
                state.drag_line_origin = None;
            }
        }

        if caret_needs_layout_sync {
            caret = layout.caret_position_at_insertion_byte(caret_byte);
        }
        state.caret = caret;
        state.selection_anchor =
            selection_byte.map(|selection| layout.caret_position_at_insertion_byte(selection));

        caret_byte = state.caret.insertion_byte_hint().min(state.value.len());
        selection_byte = state
            .selection_anchor
            .map(CaretPosition::insertion_byte_hint)
            .map(|selection| selection.min(state.value.len()));

        if just_focused || state.caret != old_caret || state.selection_anchor != old_selection {
            state.last_caret_move_time = spec.time;

            let padding = 16.0_f32;
            let viewport = scroll_result.content_bounds;

            let unscrolled_text_origin = text_origin_for_scroll(
                &spec,
                scroll_outer_rect,
                metrics.logical_size,
                prepared.block_align_offset_x,
                Vec2::ZERO,
            );

            let text_origin_in_scroll_content = Vec2::new(
                unscrolled_text_origin.x - viewport.x,
                unscrolled_text_origin.y - viewport.y,
            );

            // Determine the horizontal span of the target we want to keep in view.
            // If this is a bulk selection action with a non-empty selection, we target
            // the full selection span. Otherwise, we target the zero-width caret position.
            let (sel_min_x, sel_max_x) = match (selection_only_action, selection_byte) {
                (true, Some(sel)) if sel != caret_byte => {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);
                    let start_caret =
                        layout.caret_geom(layout.caret_position_at_insertion_byte(start));
                    let end_caret = layout.caret_geom(layout.caret_position_at_insertion_byte(end));
                    (
                        start_caret.x.min(end_caret.x),
                        start_caret.x.max(end_caret.x),
                    )
                }
                _ => {
                    let caret = layout.caret_geom(state.caret);
                    (caret.x, caret.x)
                }
            };

            // Determine the vertical span of the target we want to keep in view.
            let (sel_min_y, sel_max_y) = match (selection_only_action, selection_byte) {
                (true, Some(sel)) if sel != caret_byte => {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);
                    let start_caret =
                        layout.caret_geom(layout.caret_position_at_insertion_byte(start));
                    let end_caret = layout.caret_geom(layout.caret_position_at_insertion_byte(end));
                    (
                        start_caret.y_top.min(end_caret.y_top),
                        (start_caret.y_top + start_caret.height)
                            .max(end_caret.y_top + end_caret.height),
                    )
                }
                _ => {
                    let caret = layout.caret_geom(state.caret);
                    (caret.y_top, caret.y_top + caret.height)
                }
            };

            let target_rect = Rect::from_ltrb(
                text_origin_in_scroll_content.x + sel_min_x,
                text_origin_in_scroll_content.y + sel_min_y,
                text_origin_in_scroll_content.x + sel_max_x,
                text_origin_in_scroll_content.y + sel_max_y,
            );

            state.scroll.offset.x = reveal_axis_scroll(
                state.scroll.offset.x,
                target_rect.x,
                target_rect.right(),
                viewport.w,
                inner_scroll_size.x,
                padding,
            );

            state.scroll.offset.y = reveal_axis_scroll(
                state.scroll.offset.y,
                target_rect.y,
                target_rect.bottom(),
                viewport.h,
                inner_scroll_size.y,
                padding,
            );
        }

        let draw_text_origin = text_origin_for_scroll(
            &spec,
            scroll_outer_rect,
            metrics.logical_size,
            prepared.block_align_offset_x,
            state.scroll.offset,
        );

        // Selection
        if focused {
            if let Some(sel) = selection_byte {
                if sel != caret_byte {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);

                    for line in &layout.metrics().lines {
                        let line_sel_start = start.max(line.byte_start);
                        let line_sel_end = end.min(line.byte_end);

                        if line_sel_start < line_sel_end {
                            let line_start_x = line.logical_x;
                            let start_caret = layout.caret_geom(
                                layout.caret_position_at_insertion_byte(line_sel_start),
                            );

                            let end_x = if line_sel_end == line.byte_end {
                                line_start_x
                                    + line.logical_width
                                    + selected_line_end_affordance_width(line)
                            } else {
                                layout
                                    .caret_geom(
                                        layout.caret_position_at_insertion_byte(line_sel_end),
                                    )
                                    .x
                            };

                            let sel_rect = Rect::new(
                                draw_text_origin.x + start_caret.x.min(end_x),
                                draw_text_origin.y + start_caret.y_top,
                                (end_x - start_caret.x).abs(),
                                start_caret.height,
                            );

                            cmds.push(DrawCmd::FillRect {
                                rect: sel_rect,
                                color: spec.style.select_color,
                                z: spec.layer.get_z(),
                            });
                        }
                    }
                }
            }
        }

        // Text
        if !state.value.is_empty() {
            layout.emit_glyphs(
                cmds,
                text_backend,
                draw_text_origin,
                spec.style.text_color,
                spec.layer.get_z(),
            );
        } else if !focused {
            if let Some(placeholder) = spec.placeholder.as_deref() {
                let text_rect = Rect::new(
                    draw_text_origin.x,
                    draw_text_origin.y,
                    prepared.layout_width,
                    prepared.layout_height,
                );
                let placeholder_layout = layout_text(
                    text_backend,
                    placeholder,
                    text_style,
                    TextBounds {
                        max_width: Some(text_rect.w),
                        max_height: Some(text_rect.h),
                    },
                );
                placeholder_layout.emit_glyphs(
                    cmds,
                    text_backend,
                    Vec2::new(text_rect.x, text_rect.y),
                    spec.style.placeholder_color,
                    spec.layer.get_z(),
                );
            }
        }

        // Caret
        // The caret is drawn even when there is an active selection so the user knows which end
        // of the selection will be extended when pressing Shift+arrow.
        if focused {
            let time_since_move = spec.time - state.last_caret_move_time;
            // Solid for 0.5s after moving, then blink at 1Hz (0.5s on, 0.5s off)
            let blink_on = if time_since_move < 0.5 {
                true
            } else {
                time_since_move.fract() < 0.5
            };

            if blink_on {
                let caret = layout.caret_geom(state.caret);
                let caret_rect = Rect::new(
                    draw_text_origin.x + caret.x,
                    draw_text_origin.y + caret.y_top,
                    spec.style.caret_width,
                    caret.height,
                );
                cmds.push(DrawCmd::FillRect {
                    rect: caret_rect,
                    color: spec.style.caret_color,
                    z: spec.layer.get_z(),
                });
            }
        }

        if focused {
            focus_system.claim_pgup_vert(state.focus_id);
            focus_system.claim_pgdn_vert(state.focus_id);
        }

        end_scroll_area(
            scroll_result.token,
            inner_scroll_size,
            &mut state.scroll,
            input,
            focus_system,
            cmds,
        );

        // Text edit owns all arrow keys (caret movement via TextEvent); only Tab navigates focus.
        focus_system.handle_keyboard_traversal(
            focused,
            input,
            crate::focus::FocusTraversalKeys::tab_only(),
        );

        if just_focused {
            state.suppress_select_all_on_next_focus = false;
        }
        state.had_keyboard_focus = focused;

        TextEditResult {
            content_bounds: scroll_outer_rect,
            clipboard_action,
            focused,
            input: InputInfo {
                hovered: contains,
                pressed: input.mouse_down && contains,
                clicked: input.mouse_clicked && contains,
            },
            cursor_icon: contains.then_some(CursorIcon::Text),
        }
    }

    const SELECTED_BOUNDARY_AFFORDANCE_WIDTH: f32 = 8.0;

    fn selected_line_end_affordance_width(line: &LineMetrics) -> f32 {
        if matches!(
            line.end_kind,
            LineEndKind::HardNewline | LineEndKind::SoftWrapWhitespace
        ) {
            SELECTED_BOUNDARY_AFFORDANCE_WIDTH
        } else {
            0.0
        }
    }

    fn text_origin_for_scroll(
        spec: &TextEditSpec,
        scroll_outer_rect: Rect,
        logical_text_size: Vec2,
        block_align_offset_x: f32,
        scroll_offset: Vec2,
    ) -> Vec2 {
        let text_x =
            scroll_outer_rect.x + spec.style.padding_x + block_align_offset_x - scroll_offset.x;
        let text_y = if logical_text_size.y + 2.0 * spec.style.padding_y <= scroll_outer_rect.h {
            match spec.vertical_align {
                Align::Start => scroll_outer_rect.y + spec.style.padding_y,
                Align::Center => {
                    scroll_outer_rect.y + (scroll_outer_rect.h - logical_text_size.y) / 2.0
                }
                Align::End => {
                    scroll_outer_rect.y + scroll_outer_rect.h
                        - spec.style.padding_y
                        - logical_text_size.y
                }
            }
        } else {
            scroll_outer_rect.y + spec.style.padding_y - scroll_offset.y
        };
        Vec2::new(text_x, text_y)
    }

    /// Unified clamping logic for scrolling:
    /// - If the target span fits within the viewport (target_end <= target_start):
    ///   We clamp the current scroll to [target_end, target_start]. This ensures that the
    ///   entire target (selection or caret) is fully visible in the viewport.
    /// - If the target span is wider than the viewport (target_end > target_start):
    ///   We clamp to [target_start, target_end]. This scrolls only as far as necessary to
    ///   fill the viewport (aligning target_start or target_end depending on which direction
    ///   we are scrolling), or does not scroll at all if the viewport is already fully inside
    ///   the target range.
    fn reveal_axis_scroll(
        current: f32,
        target_min: f32,
        target_max: f32,
        viewport_len: f32,
        content_len: f32,
        padding: f32,
    ) -> f32 {
        let target_start = target_min - padding;
        let target_end = target_max - viewport_len + padding;

        let (s_min, s_max) = if target_end <= target_start {
            (target_end, target_start)
        } else {
            (target_start, target_end)
        };

        let max_scroll = (content_len - viewport_len).max(0.0);
        current.clamp(s_min, s_max).clamp(0.0, max_scroll)
    }

    #[derive(Debug, Clone, PartialEq)]
    pub(super) struct TextEditPreparedLayout<G> {
        pub scroll_outer_rect: Rect,
        pub layout_width: f32,
        pub layout_height: f32,
        pub inner_scroll_size: Vec2,
        pub block_align_offset_x: f32,
        pub reserved_vertical_scrollbar: bool,
        pub layout: TextLayout<G>,
    }

    pub(super) fn text_edit_scroll_outer_rect(spec: &TextEditSpec) -> Rect {
        let border_width = spec.style.border.map_or(0.0, |s| s.width);
        let mut scroll_outer_rect = spec.rect.inset(border_width);
        if spec.error {
            scroll_outer_rect.x += spec.style.error_stripe_width;
            scroll_outer_rect.w -= spec.style.error_stripe_width;
        }
        scroll_outer_rect
    }

    pub(super) fn should_reserve_vertical_scrollbar_gutter<T: TextBackend>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_backend: &mut T,
        scroll_outer_rect: Rect,
    ) -> bool {
        if spec.wrap {
            return true;
        }

        let available_content_height = (scroll_outer_rect.h - 2.0 * spec.style.padding_y).max(0.0);
        let one_line_height = text_backend.line_metrics(text_style).line_height as f32;
        let hard_line_count = text_content
            .as_bytes()
            .iter()
            .filter(|&&byte| byte == b'\n')
            .count()
            + 1;
        available_content_height < one_line_height * hard_line_count as f32
    }

    /// Prepare the reusable layout for the normal interactive TextEdit path.
    ///
    /// The vertical scrollbar gutter is reserved conservatively before layout,
    /// which gives wrapped text a stable width and avoids a measure/reflow
    /// feedback loop. The layout is block-local; scroll offsets change the
    /// screen-space draw/hit-test origin, not the layout. Metrics from this
    /// same layout drive the inner scroll size.
    ///
    /// Unwrapped text uses unbounded horizontal bounds so it remains
    /// horizontally scrollable. The text system aligns lines within their
    /// natural block width; when the viewport is wider than that block,
    /// TextEdit applies one block-level x offset to the origin.
    pub(super) fn prepare_text_edit_layout<T: TextBackend>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_backend: &mut T,
    ) -> TextEditPreparedLayout<T::ShapedGlyphToken> {
        let scroll_outer_rect = text_edit_scroll_outer_rect(spec);
        let reserve_vertical_scrollbar = should_reserve_vertical_scrollbar_gutter(
            text_content,
            spec,
            text_style,
            text_backend,
            scroll_outer_rect,
        );
        let scroll_outer_width = text_edit_scroll_outer_width(spec.rect.w, spec.style, spec.error);
        let available_text_width = text_edit_available_text_width(scroll_outer_width, spec.style);
        let content_width =
            text_edit_content_width(available_text_width, reserve_vertical_scrollbar);

        let layout = if spec.wrap {
            crate::text::layout_text(
                text_backend,
                text_content,
                text_style,
                TextBounds {
                    max_width: Some(content_width),
                    // Deliberately unbounded vertically for the TextEdit source-completeness
                    // invariant. Even with OverflowY::Keep, passing max_height would allow the
                    // layout engine to truncate processed lines, which would break caret and
                    // selection mapping through TextLayout.
                    max_height: None,
                },
            )
        } else {
            crate::text::layout_text(
                text_backend,
                text_content,
                text_style,
                TextBounds {
                    max_width: None,
                    max_height: None,
                },
            )
        };

        let metrics = layout.metrics();
        let layout_width = if spec.wrap {
            content_width
        } else {
            metrics.logical_size.x.max(scroll_outer_rect.w)
        };
        let block_align_offset_x = if spec.wrap {
            0.0
        } else {
            let extra_width = (content_width - metrics.logical_size.x).max(0.0);
            match text_style.flow.line_align {
                TextLineAlign::Start => 0.0,
                TextLineAlign::Center => extra_width * 0.5,
                TextLineAlign::End => extra_width,
            }
        };
        let layout_height = metrics.logical_size.y.max(scroll_outer_rect.h);
        let inner_scroll_size = Vec2::new(
            metrics.logical_size.x + 2.0 * spec.style.padding_x,
            metrics.logical_size.y + 2.0 * spec.style.padding_y,
        );

        TextEditPreparedLayout {
            scroll_outer_rect,
            layout_width,
            layout_height,
            inner_scroll_size,
            block_align_offset_x,
            reserved_vertical_scrollbar: reserve_vertical_scrollbar,
            layout,
        }
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum MovementDirection {
        Backward,
        Forward,
    }

    #[derive(Debug, Clone, PartialEq)]
    struct CaretState {
        caret: CaretPosition,
        caret_byte: usize,
        caret_needs_layout_sync: bool,
        selection_byte: Option<usize>,
    }

    fn caret_state_from_text_edit_state(state: &TextEditState) -> CaretState {
        CaretState {
            caret: state.caret,
            caret_byte: state.caret.insertion_byte_hint().min(state.value.len()),
            caret_needs_layout_sync: false,
            selection_byte: state
                .selection_anchor
                .map(CaretPosition::insertion_byte_hint)
                .map(|selection| selection.min(state.value.len()))
                .filter(|selection| state.value.is_char_boundary(*selection)),
        }
    }

    fn sanitize_caret_state_for_value(caret_state: &mut CaretState, value: &str) {
        if caret_state.caret_byte > value.len() {
            caret_state.caret_byte = value.len();
            caret_state.caret_needs_layout_sync = true;
        }
        if !value.is_char_boundary(caret_state.caret_byte) {
            caret_state.caret_byte = 0;
            caret_state.caret_needs_layout_sync = true;
        }
        if let Some(sel) = caret_state.selection_byte {
            if sel > value.len() {
                caret_state.selection_byte = Some(value.len());
            }
            if let Some(sel) = caret_state.selection_byte {
                if !value.is_char_boundary(sel) {
                    caret_state.selection_byte = None;
                }
            }
        }
    }

    impl CaretState {
        fn resolve_visual_caret<Token>(&self, layout: &TextLayout<Token>) -> CaretPosition {
            if self.caret_needs_layout_sync {
                layout.caret_position_at_insertion_byte(self.caret_byte)
            } else {
                self.caret
            }
        }

        fn apply_moved_caret(&mut self, new_caret: CaretPosition, value_len: usize) {
            self.caret = new_caret;
            self.caret_byte = new_caret.insertion_byte_hint().min(value_len);
            self.caret_needs_layout_sync = false;
        }

        fn apply_moved_byte(&mut self, new_byte: usize) {
            self.caret_byte = new_byte;
            self.caret_needs_layout_sync = true;
        }

        fn prepare_selection(&mut self, shift: bool) {
            if shift {
                if self.selection_byte.is_none() {
                    self.selection_byte = Some(self.caret_byte);
                }
            } else {
                self.selection_byte = None;
            }
        }

        fn has_selection(&self) -> bool {
            self.selection_byte.is_some() && self.selection_byte != Some(self.caret_byte)
        }

        fn get_movement_start_byte(&self, shift: bool, direction: MovementDirection) -> usize {
            let sel_byte = match self.selection_byte {
                Some(s) if s != self.caret_byte => s,
                _ => return self.caret_byte,
            };
            if shift {
                self.caret_byte
            } else {
                match direction {
                    MovementDirection::Backward => self.caret_byte.min(sel_byte),
                    MovementDirection::Forward => self.caret_byte.max(sel_byte),
                }
            }
        }

        fn move_horizontal<T: TextBackend>(
            &mut self,
            shift: bool,
            ctrl: bool,
            direction: MovementDirection,
            value: &str,
            spec: &TextEditSpec,
            text_backend: &mut T,
        ) {
            let has_selection = self.has_selection();
            let start_byte = self.get_movement_start_byte(shift, direction);
            self.prepare_selection(shift);

            let text_style = super::to_text_style(spec.style, spec.wrap, spec.line_align);

            match direction {
                MovementDirection::Backward => {
                    if ctrl {
                        let target_byte = find_word_boundary(value, start_byte, false);
                        self.apply_moved_byte(target_byte);
                    } else if has_selection && !shift {
                        self.apply_moved_byte(start_byte);
                    } else if self.caret_byte > 0 {
                        let prepared =
                            prepare_text_edit_layout(value, spec, text_style, text_backend);
                        let visual_caret = self.resolve_visual_caret(&prepared.layout);
                        let new_caret = prepared.layout.previous_caret_position(visual_caret);
                        self.apply_moved_caret(new_caret, value.len());
                    }
                }
                MovementDirection::Forward => {
                    if ctrl {
                        let target_byte = find_word_boundary(value, start_byte, true);
                        self.apply_moved_byte(target_byte);
                    } else if has_selection && !shift {
                        self.apply_moved_byte(start_byte);
                    } else if self.caret_byte < value.len() {
                        let prepared =
                            prepare_text_edit_layout(value, spec, text_style, text_backend);
                        let visual_caret = self.resolve_visual_caret(&prepared.layout);
                        let new_caret = prepared.layout.next_caret_position(visual_caret);
                        self.apply_moved_caret(new_caret, value.len());
                    }
                }
            }
        }

        fn move_vertical<T: TextBackend>(
            &mut self,
            shift: bool,
            direction: VerticalCaretDirection,
            line_count: usize,
            value: &str,
            spec: &TextEditSpec,
            text_backend: &mut T,
        ) {
            let has_selection = self.has_selection();
            let start_byte = if has_selection && !shift {
                match direction {
                    VerticalCaretDirection::Up => self.caret_byte.min(self.selection_byte.unwrap()),
                    VerticalCaretDirection::Down => {
                        self.caret_byte.max(self.selection_byte.unwrap())
                    }
                }
            } else {
                self.caret_byte
            };

            self.prepare_selection(shift);

            let text_style = super::to_text_style(spec.style, spec.wrap, spec.line_align);

            let moved = move_caret_vertical(
                value,
                spec,
                text_style,
                text_backend,
                self.caret,
                self.caret_byte,
                !self.caret_needs_layout_sync,
                start_byte,
                direction,
                line_count,
            );
            self.caret = moved.caret;
            self.caret_byte = moved.byte;
            self.caret_needs_layout_sync = moved.needs_layout_sync;
        }

        fn move_home<T: TextBackend>(
            &mut self,
            shift: bool,
            ctrl: bool,
            value: &str,
            spec: &TextEditSpec,
            text_backend: &mut T,
        ) {
            self.prepare_selection(shift);
            if ctrl {
                self.apply_moved_byte(0);
            } else if spec.wrap {
                let text_style = super::to_text_style(spec.style, spec.wrap, spec.line_align);
                let prepared = prepare_text_edit_layout(value, spec, text_style, text_backend);
                let visual_caret = self.resolve_visual_caret(&prepared.layout);
                let current_line_idx = prepared.layout.visual_line_index_for_caret(visual_caret);
                let new_caret = prepared.layout.caret_at_visual_line_start(current_line_idx);
                self.apply_moved_caret(new_caret, value.len());
            } else {
                let line_start = value[..self.caret_byte].rfind('\n').map_or(0, |nl| nl + 1);
                self.apply_moved_byte(line_start);
            }
        }

        fn move_end<T: TextBackend>(
            &mut self,
            shift: bool,
            ctrl: bool,
            value: &str,
            spec: &TextEditSpec,
            text_backend: &mut T,
        ) {
            self.prepare_selection(shift);
            if ctrl {
                self.apply_moved_byte(value.len());
            } else if spec.wrap {
                let text_style = super::to_text_style(spec.style, spec.wrap, spec.line_align);
                let prepared = prepare_text_edit_layout(value, spec, text_style, text_backend);
                let visual_caret = self.resolve_visual_caret(&prepared.layout);
                let current_line_idx = prepared.layout.visual_line_index_for_caret(visual_caret);
                let new_caret = prepared.layout.caret_at_visual_line_end(current_line_idx);
                self.apply_moved_caret(new_caret, value.len());
            } else {
                let line_end = value[self.caret_byte..]
                    .find('\n')
                    .map_or(value.len(), |nl| self.caret_byte + nl);
                self.apply_moved_byte(line_end);
            }
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextEditStyle {
    pub background: Color,
    pub background_hovered: Color,
    pub error_background: Color,
    pub border: Option<Stroke>,
    pub focus_border: Option<Stroke>,
    pub error_border: Option<Stroke>,
    pub error_stripe_width: f32,
    pub min_height: f32,
    pub padding_x: f32,
    pub padding_y: f32,
    pub font: FontId,
    pub size: f32,
    pub weight: u16,
    pub italic: bool,
    pub letter_spacing: f32,
    pub line_height: LineHeight,
    pub text_color: Color,
    pub placeholder_color: Color,
    pub caret_color: Color,
    pub caret_width: f32,
    pub select_color: Color,
    pub scroll_area_style: ScrollAreaStyle,
    pub disabled_alpha: f32,
}

impl TextEditStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        let mut scroll_area_style = ScrollAreaStyle::from_theme(theme);
        scroll_area_style.scrollbar_width = TEXT_EDIT_SCROLLBAR_WIDTH;
        scroll_area_style.scrollbar_style.background_fill =
            Some(theme.scrollbar_track_on_paper_elev);
        scroll_area_style.scrollbar_style.separator_line =
            Some(Stroke::new(theme.line_soft_on_paper_elev, 1.0));
        if let Some(segment_style) = &mut scroll_area_style.scrollbar_style.segment_style {
            segment_style.cross_axis =
                crate::widgets::slider::ThumbCrossAxis::FillTrack { margin: 0.0 };
        }

        Self {
            background: theme.paper_elev,
            background_hovered: Color::WHITE,
            error_background: theme.rust_soft_on_paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus_border: Some(Stroke::new(theme.rust, theme.focus_width)),
            error_border: Some(Stroke::new(theme.rust, theme.border)),
            error_stripe_width: 4.0,
            min_height: theme.h_md,
            padding_x: 10.0,
            padding_y: 0.0,
            font: theme.mono_font,
            size: theme.text_mono,
            weight: theme.sans_weight_regular,
            italic: false,
            letter_spacing: 0.0,
            line_height: LineHeight::Normal,
            text_color: theme.ink,
            placeholder_color: theme.muted,
            caret_color: theme.rust,
            caret_width: 2.0,
            select_color: theme.rust_soft_on_paper_elev,
            scroll_area_style,
            disabled_alpha: 0.55,
        }
    }
}

// TextEdit invariant:
// The prepared TextLayout for state.value must be source-complete.
// Editable text may overflow, scroll, wrap, or be clipped by the widget
// viewport, but layout must not drop, ellipsise, or vertically truncate
// clusters. Caret and selection state are mapped through TextLayout, so
// losing clusters can turn non-empty text into EmptyText or reset the
// logical insertion byte.
//
// Therefore wrapped TextEdit uses a clipping/keep flow rather than the
// general paragraph `TextFlow::wrapped()` policy, which is allowed to drop
// over-wide clusters and ellipsise vertical overflow.
pub(crate) fn to_text_style(
    style: TextEditStyle,
    wrap: bool,
    line_align: TextLineAlign,
) -> TextStyle {
    let mut flow = if wrap {
        TextFlow::clipped_viewport()
    } else {
        TextFlow::single_line()
    };
    flow.line_align = line_align;
    TextStyle {
        font: style.font,
        size: style.size,
        weight: style.weight,
        flow,
        italic: style.italic,
        letter_spacing: style.letter_spacing,
        line_height: style.line_height,
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditState {
    pub value: String,
    pub caret: CaretPosition,
    pub selection_anchor: Option<CaretPosition>,
    pub focus_id: FocusId,
    pub is_dragging: bool,
    pub drag_word_origin: Option<(usize, usize)>,
    pub drag_line_origin: Option<(usize, usize)>,
    pub last_caret_move_time: f64,
    pub had_keyboard_focus: bool,
    pub suppress_select_all_on_next_focus: bool,
    pub scroll: ScrollState,
}

impl Default for TextEditState {
    fn default() -> Self {
        Self {
            value: String::new(),
            caret: CaretPosition::EmptyText,
            selection_anchor: None,
            focus_id: FocusId::default(),
            is_dragging: false,
            drag_word_origin: None,
            drag_line_origin: None,
            last_caret_move_time: 0.0,
            had_keyboard_focus: false,
            suppress_select_all_on_next_focus: false,
            scroll: ScrollState::default(),
        }
    }
}

impl TextEditState {
    pub fn new(initial_text: &str) -> Self {
        Self {
            value: initial_text.to_string(),
            caret: caret_position_at_text_end(initial_text),
            scroll: ScrollState::default(),
            ..Default::default()
        }
    }
}

fn remove_selection(
    value: &mut String,
    caret_byte: &mut usize,
    selection_byte: &mut Option<usize>,
) {
    if let Some(sel) = *selection_byte {
        let start = (*caret_byte).min(sel);
        let end = (*caret_byte).max(sel);
        value.replace_range(start..end, "");
        *caret_byte = start;
        *selection_byte = None;
    }
}

/// Inserts `text` into `value` at the current `caret_byte` (replacing any selection in `selection_byte`),
/// after applying the specified `policy`.
///
/// If the processed text is empty (for example, if `TrimAfterFirstNewline` trims everything or Enter is pressed),
/// this function returns early without mutating the text or selection.
fn insert_text_with_newline_policy(
    value: &mut String,
    caret_byte: &mut usize,
    selection_byte: &mut Option<usize>,
    caret_needs_layout_sync: &mut bool,
    policy: NewlinePolicy,
    text: &str,
) {
    let processed = policy.process(text);
    if processed.is_empty() {
        return;
    }
    remove_selection(value, caret_byte, selection_byte);
    value.insert_str(*caret_byte, &processed);
    *caret_byte += processed.len();
    *caret_needs_layout_sync = true;
}

fn caret_position_at_text_end(text: &str) -> CaretPosition {
    text.char_indices()
        .next_back()
        .map(|(cluster_byte_start, ch)| CaretPosition::AfterCluster {
            cluster_byte_start,
            cluster_byte_end: cluster_byte_start + ch.len_utf8(),
        })
        .unwrap_or(CaretPosition::EmptyText)
}

#[cfg(test)]
fn caret_position_at_byte(text: &str, byte_index: usize) -> CaretPosition {
    if text.is_empty() {
        return CaretPosition::EmptyText;
    }
    if byte_index >= text.len() {
        return caret_position_at_text_end(text);
    }
    CaretPosition::BeforeCluster {
        cluster_byte_start: byte_index,
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
}

// ── Spec ─────────────────────────────────────────────────────────────────────

/// Policy governing how newlines in text inputs (e.g. pasted text, typed characters,
/// and programmatic values) are handled, and how Enter-key press events are resolved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NewlinePolicy {
    /// Newlines are preserved. Normalizes `\r\n` and bare `\r` to `\n`.
    /// Pressing Enter inserts a newline character (`\n`).
    Preserve,
    /// Each newline sequence (treating `\r\n` as one sequence) is replaced with a single space.
    /// Pressing Enter inserts a space character (` `).
    #[default]
    ReplaceWithSpace,
    /// Truncates the text before the first newline sequence (treating `\r\n` as one sequence).
    /// Pressing Enter is a no-op (inserts nothing).
    TrimAfterFirstNewline,
}

impl NewlinePolicy {
    /// Sanitizes the input string according to the newline policy.
    pub fn process<'a>(self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if !text.contains('\n') && !text.contains('\r') {
            return std::borrow::Cow::Borrowed(text);
        }
        match self {
            NewlinePolicy::Preserve => {
                std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
            }
            NewlinePolicy::ReplaceWithSpace => {
                std::borrow::Cow::Owned(text.replace("\r\n", " ").replace(['\r', '\n'], " "))
            }
            NewlinePolicy::TrimAfterFirstNewline => {
                if let Some(idx) = text.find(['\n', '\r']) {
                    std::borrow::Cow::Borrowed(&text[..idx])
                } else {
                    std::borrow::Cow::Borrowed(text)
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditSpec {
    pub style: TextEditStyle,
    pub placeholder: Option<String>,
    pub error: bool,
    pub disabled: bool,
    pub newline_policy: NewlinePolicy,
    pub wrap: bool,
    pub vertical_align: Align,
    pub line_align: TextLineAlign,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextEditSpecBuilder {
    pub style: Option<TextEditStyle>,
    pub placeholder: Option<Option<String>>,
    pub error: Option<bool>,
    pub disabled: Option<bool>,
    pub newline_policy: Option<NewlinePolicy>,
    pub wrap: Option<bool>,
    pub vertical_align: Option<Align>,
    pub line_align: Option<TextLineAlign>,
}

impl TextEditSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: TextEditStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(Some(placeholder.into()));
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
    pub fn newline_policy(mut self, newline_policy: NewlinePolicy) -> Self {
        self.newline_policy = Some(newline_policy);
        self
    }
    pub fn wrap(mut self, wrap: bool) -> Self {
        self.wrap = Some(wrap);
        self
    }
    pub fn vertical_align(mut self, vertical_align: Align) -> Self {
        self.vertical_align = Some(vertical_align);
        self
    }
    pub fn line_align(mut self, line_align: TextLineAlign) -> Self {
        self.line_align = Some(line_align);
        self
    }

    /// Standard single-line text field.
    ///
    /// Newlines are replaced with spaces, soft wrapping is disabled, and the
    /// text is vertically centred in the field.
    pub fn single_line(self) -> Self {
        self.newline_policy(NewlinePolicy::ReplaceWithSpace)
            .wrap(false)
            .vertical_align(Align::Center)
            .line_align(TextLineAlign::Start)
    }

    /// Multiline editor with hard newlines preserved and no soft wrapping.
    ///
    /// Long lines can overflow horizontally and be scrolled.
    pub fn multiline_unwrapped(self) -> Self {
        self.newline_policy(NewlinePolicy::Preserve)
            .wrap(false)
            .vertical_align(Align::Start)
            .line_align(TextLineAlign::Start)
    }

    /// Multiline editor with hard newlines preserved and soft wrapping enabled.
    ///
    /// This is the usual textarea-style configuration.
    pub fn multiline_wrapped(self) -> Self {
        self.newline_policy(NewlinePolicy::Preserve)
            .wrap(true)
            .vertical_align(Align::Start)
            .line_align(TextLineAlign::Start)
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            let mut style = TextEditStyle::from_theme(theme);
            let multiline = self.newline_policy.unwrap_or_default() == NewlinePolicy::Preserve
                || self.wrap.unwrap_or(false);
            style.padding_y = if multiline { 8.0 } else { 0.0 };
            self.style = Some(style);
        }
        self
    }

    pub fn build(self) -> TextEditSpec {
        TextEditSpec {
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            placeholder: self.placeholder.unwrap_or(None),
            error: self.error.unwrap_or(false),
            disabled: self.disabled.unwrap_or(false),
            newline_policy: self
                .newline_policy
                .unwrap_or(NewlinePolicy::ReplaceWithSpace),
            wrap: self.wrap.unwrap_or(false),
            vertical_align: self.vertical_align.unwrap_or(Align::Center),
            line_align: self.line_align.unwrap_or(TextLineAlign::Start),
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

pub fn logical_line_bounds(text: &str, byte_index: usize) -> (usize, usize) {
    if text.is_empty() {
        return (0, 0);
    }

    let safe_index = byte_index.min(text.len());
    let left = text[..safe_index]
        .rfind('\n')
        .map_or(0, |newline| newline + 1);
    let right = text[safe_index..]
        .find('\n')
        .map_or(text.len(), |newline| safe_index + newline + 1);

    (left, right)
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level text edit widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn text_edit<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TextEditSpecBuilder,
    layout_params: S::Params,
    state: &mut TextEditState,
) -> TextEditResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::TextEditPreLayoutSpec {
        style: spec.style,
        wrap: spec.wrap,
        line_align: spec.line_align,
        error: spec.error,
        disabled: spec.disabled,
        newline_policy: spec.newline_policy,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_text_edit(
        &pre_layout_spec,
        offer,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
    );
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::TextEditSpec {
        rect,
        style: spec.style,
        placeholder: spec.placeholder,
        clip_rect: ctx.clip_rect,
        error: spec.error,
        disabled: spec.disabled,
        time: ctx.time,
        layer: ctx.layer,
        newline_policy: spec.newline_policy,
        wrap: spec.wrap,
        vertical_align: spec.vertical_align,
        line_align: spec.line_align,
    };
    let result = raw::post_layout_text_edit(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    if let Some(action) = result.clipboard_action.as_ref() {
        match action {
            ClipboardAction::Copy(text) | ClipboardAction::Cut(text) => {
                ctx.output.new_clipboard_contents = Some(text.clone());
            }
        }
    }

    if let Some(cursor_icon) = result.cursor_icon {
        ctx.output.cursor_icon = Some(cursor_icon);
    }

    TextEditResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "text_edit_tests.rs"]
mod tests;
