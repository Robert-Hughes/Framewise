use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::{Input, TextEvent},
    layout::{IntrinsicSize, LayoutState},
    text::{TextBounds, TextFlow, TextStyle, TextSystem},
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    widgets::scroll_area::ScrollState,
};

pub mod raw {
    use super::*;
    use crate::widgets::{
        scroll_area::raw::{begin_scroll_area, end_scroll_area, ScrollAreaSpec},
        scroll_area::{ScrollAxis, ScrollExtent, ScrollLen},
        ScrollbarVisibility, SliderStyle,
    };

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditSpec {
        pub rect: Rect,
        pub style: super::TextEditStyle,
        pub clip_rect: ClipRect,
        pub error: bool,
        pub disabled: bool,
        pub time: f64,
        pub layer: Layer,
        pub newline_policy: super::NewlinePolicy,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditCalcIntrinsicSizeSpec {
        pub style: super::TextEditStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub clipboard_action: Option<ClipboardAction>,
    }

    /// Measure a text edit's intrinsic size from its current state and measurement spec.
    pub fn calc_text_edit_intrinsic_size<T: TextSystem>(
        spec: &TextEditCalcIntrinsicSizeSpec,
        state: &TextEditState,
        text_system: &mut T,
    ) -> IntrinsicSize {
        let text = if state.value.is_empty() {
            " "
        } else {
            &state.value
        };
        let metrics = text_system.measure(text, spec.style.text_style, TextBounds::UNBOUNDED);
        let inset = (spec.style.border_width + spec.style.padding) * 2.0;
        IntrinsicSize::preferred(Vec2::new(
            metrics.logical_size.x + inset,
            (metrics.logical_size.y + inset).max(spec.style.min_height),
        ))
    }

    /// Low-level text edit widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn text_edit<T: TextSystem>(
        spec: TextEditSpec,
        state: &mut TextEditState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> TextEditResult {
        let processed = spec.newline_policy.process(&state.value);
        if let std::borrow::Cow::Owned(s) = processed {
            state.value = s;
        }

        let mut clipboard_action = None;
        // selection_only_action tracks whether the selection or caret was changed by a bulk selection
        // event (like double-clicking a word, triple-clicking a line, focus gain, or Ctrl-A).
        // For these actions, we want to minimize scrolling by only adjusting the viewport
        // as much as necessary to bring the selection range into view, rather than jumping
        // to the caret position.
        let mut selection_only_action = false;

        // Disabled: draw at reduced alpha, no interaction.
        if spec.disabled {
            //TODO: update this to match new layout? Perhaps remove this separate branch entirely?
            let tint =
                |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * spec.style.disabled_alpha);
            // Transparent bg per mockup, just border.
            if spec.style.border_width > 0.0 {
                cmds.push(DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: spec.rect,
                    color: tint(spec.style.border),
                    width: spec.style.border_width,
                    z: spec.layer.get_z(),
                });
            }
            let inset = spec.style.border_width + spec.style.padding;
            let content_rect = spec.rect.inset(inset);
            if !state.value.is_empty() {
                let metrics = text_system.measure(
                    &state.value,
                    spec.style.text_style,
                    TextBounds {
                        max_width: Some(content_rect.w),
                        max_height: Some(content_rect.h),
                    },
                );
                let ty = content_rect.y + (content_rect.h - metrics.logical_size.y) / 2.0;
                let text_rect = Rect::new(content_rect.x, ty, content_rect.w, content_rect.h);
                let layout = text_system.prepare(&state.value, spec.style.text_style, text_rect);
                cmds.push(DrawCmd::Text {
                    rect: text_rect,
                    color: tint(spec.style.text_color),
                    handle: layout.handle,
                    z: spec.layer.get_z(),
                });
            }
            return TextEditResult {
                content_bounds: content_rect,
                clipboard_action: None,
                focused: false,
                input: InputInfo::default(),
            };
        }

        let focused = focus_system.register_keyboard(state.focus_id, spec.rect, spec.clip_rect);
        let just_focused = focused && !state.was_focused;

        let old_caret = state.caret_byte;
        let old_selection = state.selection_byte;

        // Hit test mouse
        let is_visible = spec
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
        let contains_raw = spec.rect.contains(input.mouse_pos) && is_visible;
        if contains_raw {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = focus_system.is_hover_active(state.focus_id);
        let contains = contains_raw && is_hover_active;

        if just_focused && !(contains && input.mouse_pressed) {
            state.selection_byte = Some(0);
            state.caret_byte = state.value.len();
            selection_only_action = true;
        }

        // Process keyboard events if focused
        if focused {
            let mut newline_inserted = false;
            for ev in &input.text_events {
                match ev {
                    TextEvent::Char(c) => {
                        let is_newline = *c == '\n' || *c == '\r';
                        let ok = if is_newline {
                            spec.newline_policy != NewlinePolicy::Reject
                        } else {
                            !c.is_control()
                        };
                        if ok {
                            state.remove_selection();
                            let char_to_insert = if is_newline {
                                if spec.newline_policy == NewlinePolicy::ReplaceWithSpace {
                                    ' '
                                } else {
                                    '\n'
                                }
                            } else {
                                *c
                            };
                            state.value.insert(state.caret_byte, char_to_insert);
                            state.caret_byte += char_to_insert.len_utf8();
                            if is_newline {
                                newline_inserted = true;
                            }
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
                        let sel_byte = state.selection_byte;
                        let has_selection =
                            sel_byte.is_some() && sel_byte != Some(state.caret_byte);

                        if *shift {
                            if state.selection_byte.is_none() {
                                state.selection_byte = Some(state.caret_byte);
                            }
                        } else {
                            state.selection_byte = None;
                        }

                        if *ctrl {
                            let start_byte = if has_selection && !*shift {
                                state.caret_byte.min(sel_byte.unwrap())
                            } else {
                                state.caret_byte
                            };
                            state.caret_byte = find_word_boundary(&state.value, start_byte, false);
                        } else if has_selection && !*shift {
                            state.caret_byte = state.caret_byte.min(sel_byte.unwrap());
                        } else if state.caret_byte > 0 {
                            let mut prev = state.caret_byte - 1;
                            while prev > 0 && !state.value.is_char_boundary(prev) {
                                prev -= 1;
                            }
                            state.caret_byte = prev;
                        }
                    }
                    TextEvent::CaretRight { shift, ctrl } => {
                        let sel_byte = state.selection_byte;
                        let has_selection =
                            sel_byte.is_some() && sel_byte != Some(state.caret_byte);

                        if *shift {
                            if state.selection_byte.is_none() {
                                state.selection_byte = Some(state.caret_byte);
                            }
                        } else {
                            state.selection_byte = None;
                        }

                        if *ctrl {
                            let start_byte = if has_selection && !*shift {
                                state.caret_byte.max(sel_byte.unwrap())
                            } else {
                                state.caret_byte
                            };
                            state.caret_byte = find_word_boundary(&state.value, start_byte, true);
                        } else if has_selection && !*shift {
                            state.caret_byte = state.caret_byte.max(sel_byte.unwrap());
                        } else if state.caret_byte < state.value.len() {
                            let mut next = state.caret_byte + 1;
                            while next < state.value.len() && !state.value.is_char_boundary(next) {
                                next += 1;
                            }
                            state.caret_byte = next;
                        }
                    }
                    TextEvent::CaretUp { shift } => {
                        if *shift {
                            if state.selection_byte.is_none() {
                                state.selection_byte = Some(state.caret_byte);
                            }
                        } else {
                            state.selection_byte = None;
                        }

                        let text_content = if state.value.is_empty() {
                            " "
                        } else {
                            &state.value
                        };
                        let layout_width = (spec.rect.w - 2.0 * spec.style.border_width).max(1.0);
                        let layout = text_system.prepare(
                            text_content,
                            spec.style.text_style,
                            Rect::new(0.0, 0.0, layout_width, f32::MAX),
                        );
                        let handle = layout.handle;
                        let metrics = layout.metrics;

                        let caret = text_system.caret_geom(handle, state.caret_byte);
                        let current_line_idx = metrics
                            .lines
                            .iter()
                            .rposition(|line| state.caret_byte >= line.byte_start)
                            .unwrap_or(0);

                        if current_line_idx > 0 {
                            let target_line_idx = current_line_idx - 1;
                            let target_line = &metrics.lines[target_line_idx];
                            let pos =
                                Vec2::new(caret.x, target_line.y_top + target_line.height * 0.5);
                            let new_caret = text_system.hit_test_caret(handle, pos);
                            state.caret_byte = new_caret.min(state.value.len());
                        } else {
                            // Already on first visual line, move to start of text
                            state.caret_byte = 0;
                        }
                    }
                    TextEvent::CaretDown { shift } => {
                        if *shift {
                            if state.selection_byte.is_none() {
                                state.selection_byte = Some(state.caret_byte);
                            }
                        } else {
                            state.selection_byte = None;
                        }

                        let text_content = if state.value.is_empty() {
                            " "
                        } else {
                            &state.value
                        };
                        let layout_width = (spec.rect.w - 2.0 * spec.style.border_width).max(1.0);
                        let layout = text_system.prepare(
                            text_content,
                            spec.style.text_style,
                            Rect::new(0.0, 0.0, layout_width, f32::MAX),
                        );
                        let handle = layout.handle;
                        let metrics = layout.metrics;

                        let caret = text_system.caret_geom(handle, state.caret_byte);
                        let current_line_idx = metrics
                            .lines
                            .iter()
                            .rposition(|line| state.caret_byte >= line.byte_start)
                            .unwrap_or(0);

                        if current_line_idx + 1 < metrics.lines.len() {
                            let target_line_idx = current_line_idx + 1;
                            let target_line = &metrics.lines[target_line_idx];
                            let pos =
                                Vec2::new(caret.x, target_line.y_top + target_line.height * 0.5);
                            let new_caret = text_system.hit_test_caret(handle, pos);
                            state.caret_byte = new_caret.min(state.value.len());
                        } else {
                            // Already on last visual line, move to end of text
                            state.caret_byte = state.value.len();
                        }
                    }
                    TextEvent::CaretHome { shift, ctrl } => {
                        if *shift && state.selection_byte.is_none() {
                            state.selection_byte = Some(state.caret_byte);
                        } else if !*shift {
                            state.selection_byte = None;
                        }
                        if *ctrl {
                            state.caret_byte = 0;
                        } else {
                            // Line-aware: scan left for the preceding '\n' (or
                            // the start of the string), then land just after it.
                            let line_start = state.value[..state.caret_byte]
                                .rfind('\n')
                                .map_or(0, |nl| nl + 1);
                            state.caret_byte = line_start;
                        }
                    }
                    TextEvent::CaretEnd { shift, ctrl } => {
                        if *shift && state.selection_byte.is_none() {
                            state.selection_byte = Some(state.caret_byte);
                        } else if !*shift {
                            state.selection_byte = None;
                        }
                        if *ctrl {
                            state.caret_byte = state.value.len();
                        } else {
                            // Line-aware: scan right for the next '\n' and land
                            // just before it (or at the end of the string).
                            let line_end = state.value[state.caret_byte..]
                                .find('\n')
                                .map_or(state.value.len(), |nl| state.caret_byte + nl);
                            state.caret_byte = line_end;
                        }
                    }
                    TextEvent::SelectAll => {
                        state.selection_byte = Some(0);
                        state.caret_byte = state.value.len();
                        selection_only_action = true;
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
                        let processed = spec.newline_policy.process(text);
                        state.remove_selection();
                        state.value.insert_str(state.caret_byte, &processed);
                        state.caret_byte += processed.len();
                    }
                }
            }

            if input.key_pressed_enter
                && !newline_inserted
                && spec.newline_policy == NewlinePolicy::Allow
            {
                state.remove_selection();
                state.value.insert(state.caret_byte, '\n');
                state.caret_byte += 1;
            }
        }

        // Safety checks
        if state.caret_byte > state.value.len() {
            state.caret_byte = state.value.len();
        }
        if !state.value.is_char_boundary(state.caret_byte) {
            state.caret_byte = 0; // fallback
        }
        if let Some(sel) = state.selection_byte {
            if sel > state.value.len() {
                state.selection_byte = Some(state.value.len());
            }
            if let Some(sel) = state.selection_byte {
                if !state.value.is_char_boundary(sel) {
                    state.selection_byte = None;
                }
            }
        }

        let mut scroll_outer_rect = spec.rect.inset(spec.style.border_width);
        if spec.error {
            // shift content right to clear the 4px error stripe
            scroll_outer_rect.x += spec.style.error_stripe_width;
            scroll_outer_rect.w -= spec.style.error_stripe_width;
        }

        // Drawing Background
        let bg_color = if spec.error {
            spec.style.error_background
        } else {
            spec.style.background
        };
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
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
                anti_alias: false,
                rect: stripe,
                color: spec.style.error_border,
                z: spec.layer.get_z(),
            });
        }

        // Border
        if spec.style.border_width > 0.0 {
            let b_color = if spec.error {
                spec.style.error_border
            } else if focused {
                spec.style.focus
            } else {
                spec.style.border
            };
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: spec.rect,
                color: b_color,
                width: spec.style.border_width,
                z: spec.layer.get_z(),
            });
        }

        // Prepare text after content bounds are known so hit testing and caret
        // geometry use the same logical text block that will be drawn.
        let text_content = if state.value.is_empty() {
            " "
        } else {
            &state.value
        };
        let metrics = text_system.measure(
            text_content,
            spec.style.text_style,
            TextBounds {
                max_width: None,
                max_height: None, //TODO: what about for single line?
            },
        );
        let inner_scroll_size = Vec2::new(
            metrics.logical_size.x + 2.0 * spec.style.padding,
            metrics.logical_size.y + 2.0 * spec.style.padding,
        ); // Include padding on either side of text

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
            scrollbar_width: 5.0,
            scrollbar_style: SliderStyle {
                track_color: Color::linear_rgba(
                    spec.style.border.r,
                    spec.style.border.g,
                    spec.style.border.b,
                    0.04,
                ),
                track_border_color: Some(spec.style.border),
                thumb_color: spec.style.border,
                thumb_border_color: Color::TRANSPARENT,
                thumb_border_width: 0.0,
                thumb_hover_color: spec.style.focus,
                thumb_drag_color: spec.style.focus,
                focus: spec.style.focus,
                focus_width: 1.0,
                focus_offset: 1.0,
                thickness: 0.0,
                thumb_size: 0.0,
                scrollbar_mode: true,
                disabled_alpha: 0.4,
                scrollbar_thumb_margin: 0.0,
            },
            layer: spec.layer,
            keyboard_focusable: false,
        };
        let scroll_result =
            raw::begin_scroll_area(scroll_spec, &mut state.scroll, input, focus_system, cmds);

        let text_x = scroll_outer_rect.x + spec.style.padding - scroll_result.offset.x;
        let text_y = scroll_outer_rect.y + (scroll_outer_rect.h - metrics.logical_size.y) / 2.0; //TODO: multi-line
        let text_rect = Rect::new(
            text_x,
            text_y,
            metrics.logical_size.x.max(scroll_outer_rect.w),
            scroll_outer_rect.h,
        );
        let layout = text_system.prepare(text_content, spec.style.text_style, text_rect);
        let handle = layout.handle;

        // Mouse interaction
        if contains && input.mouse_pressed {
            focus_system.take_keyboard_focus(state.focus_id);

            let relative_pos = Vec2::new(
                input.mouse_pos.x - text_rect.x,
                input.mouse_pos.y - text_rect.y,
            );
            let clicked_byte = text_system.hit_test_caret(handle, relative_pos);
            let clicked_byte = clicked_byte.min(state.value.len());

            // Handling double/triple clicks
            if input.mouse_click_count == 2 {
                let cluster_byte = text_system.hit_test_cluster(handle, relative_pos);
                let (start, end) = word_bounds(&state.value, cluster_byte);
                state.selection_byte = Some(start);
                state.caret_byte = end;
                state.is_dragging = true;
                state.drag_word_origin = Some((start, end));
                selection_only_action = true;
            } else if input.mouse_click_count >= 3 {
                // Select line
                state.selection_byte = Some(0);
                state.caret_byte = state.value.len();
                selection_only_action = true;
            } else {
                state.caret_byte = clicked_byte;
                state.selection_byte = None;
                state.is_dragging = true;
                state.drag_word_origin = None;
            }
        }

        if state.is_dragging {
            if input.mouse_down {
                let relative_pos = Vec2::new(
                    input.mouse_pos.x - text_rect.x,
                    input.mouse_pos.y - text_rect.y,
                );
                let current_byte = text_system.hit_test_caret(handle, relative_pos);
                let current_byte = current_byte.min(state.value.len());

                if let Some((orig_start, orig_end)) = state.drag_word_origin {
                    let cluster_byte = text_system.hit_test_cluster(handle, relative_pos);
                    let (cur_start, cur_end) = word_bounds(&state.value, cluster_byte);
                    if cluster_byte < orig_start {
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

            let padding = 16.0_f32;
            let max_scroll_x = (inner_scroll_size.x - scroll_outer_rect.w).max(0.0);

            // Determine the horizontal span of the target we want to keep in view.
            // If this is a bulk selection action with a non-empty selection, we target
            // the full selection span. Otherwise, we target the zero-width caret position.
            let (sel_min_x, sel_max_x) = match (selection_only_action, state.selection_byte) {
                (true, Some(sel)) if sel != state.caret_byte => {
                    let start = sel.min(state.caret_byte);
                    let end = sel.max(state.caret_byte);
                    let start_caret = text_system.caret_geom(handle, start);
                    let end_caret = text_system.caret_geom(handle, end);
                    (
                        start_caret.x.min(end_caret.x),
                        start_caret.x.max(end_caret.x),
                    )
                }
                _ => {
                    let caret = text_system.caret_geom(handle, state.caret_byte);
                    (caret.x, caret.x)
                }
            };

            let target_left = sel_min_x - padding;
            let target_right = sel_max_x - scroll_outer_rect.w + padding;

            // Unified clamping logic for scrolling:
            // - If the target span fits within the viewport (target_right <= target_left):
            //   We clamp the current scroll to [target_right, target_left]. This ensures that the
            //   entire target (selection or caret) is fully visible in the viewport.
            // - If the target span is wider than the viewport (target_right > target_left):
            //   We clamp to [target_left, target_right]. This scrolls only as far as necessary to
            //   fill the viewport (aligning target_left or target_right depending on which direction
            //   we are scrolling), or does not scroll at all if the viewport is already fully inside
            //   the target range.
            let (s_min, s_max) = if target_right <= target_left {
                (target_right, target_left)
            } else {
                (target_left, target_right)
            };

            let target_scroll = state.scroll.offset.x.clamp(s_min, s_max);
            state.scroll.offset.x = target_scroll.clamp(0.0, max_scroll_x);
        }

        // Selection
        if focused {
            if let Some(sel) = state.selection_byte {
                if sel != state.caret_byte {
                    let start = sel.min(state.caret_byte);
                    let end = sel.max(state.caret_byte);

                    for line in &layout.metrics.lines {
                        let line_sel_start = start.max(line.byte_start);
                        let line_sel_end = end.min(line.byte_end);

                        if line_sel_start < line_sel_end {
                            let start_caret = text_system.caret_geom(handle, line_sel_start);

                            let has_newline = line.byte_end > line.byte_start
                                && state.value.as_bytes().get(line.byte_end - 1) == Some(&b'\n');

                            let end_x = if line_sel_end == line.byte_end && has_newline {
                                // Highlight the newline character specifically
                                let newline_width = 8.0;
                                let nl_caret = text_system.caret_geom(handle, line.byte_end - 1);
                                nl_caret.x + newline_width
                            } else {
                                text_system.caret_geom(handle, line_sel_end).x
                            };

                            let sel_rect = Rect::new(
                                text_rect.x + start_caret.x.min(end_x),
                                text_rect.y + start_caret.y_top,
                                (end_x - start_caret.x).abs(),
                                start_caret.height,
                            );

                            cmds.push(DrawCmd::FillRect {
                                anti_alias: false,
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
            cmds.push(DrawCmd::Text {
                rect: text_rect,
                color: spec.style.text_color,
                handle,
                z: spec.layer.get_z(),
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
                let caret = text_system.caret_geom(handle, state.caret_byte);
                let caret_rect = Rect::new(
                    text_rect.x + caret.x,
                    text_rect.y + caret.y_top,
                    1.0,
                    caret.height,
                );
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: caret_rect,
                    color: spec.style.caret_color,
                    z: spec.layer.get_z(),
                });
            }
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

        state.was_focused = focused || (contains && input.mouse_pressed);

        TextEditResult {
            content_bounds: scroll_outer_rect,
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
    pub focus: Color,
    pub border_width: f32,
    pub error_border: Color,
    pub error_stripe_width: f32,
    pub min_height: f32,
    pub padding: f32,
    pub text_style: TextStyle,
    pub text_color: Color,
    pub caret_color: Color,
    pub select_color: Color,
    pub disabled_alpha: f32,
}

impl TextEditStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            error_background: theme.rust_soft,
            border: theme.ink,
            focus: theme.rust,
            error_border: theme.rust,
            error_stripe_width: 4.0,
            border_width: theme.border,
            min_height: theme.h_md,
            padding: 4.0,
            text_style: TextStyle::new(
                theme.mono_font,
                theme.text_mono,
                theme.sans_weight_regular,
                TextFlow::single_line(),
            ),
            text_color: theme.ink,
            caret_color: theme.rust,
            select_color: theme.rust_soft,
            disabled_alpha: 0.55,
        }
    }
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
    pub scroll: ScrollState,
}

impl TextEditState {
    pub fn new(initial_text: &str) -> Self {
        Self {
            value: initial_text.to_string(),
            caret_byte: initial_text.len(),
            scroll: ScrollState::default(),
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

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Copy, Clone, PartialEq, Eq, Default)]
pub enum NewlinePolicy {
    Reject,
    #[default]
    ReplaceWithSpace,
    Allow,
}

impl NewlinePolicy {
    /// Sanitizes the input string according to the newline policy.
    pub fn process<'a>(&self, text: &'a str) -> std::borrow::Cow<'a, str> {
        if !text.contains('\n') && !text.contains('\r') {
            return std::borrow::Cow::Borrowed(text);
        }
        match self {
            NewlinePolicy::Allow => {
                std::borrow::Cow::Owned(text.replace("\r\n", "\n").replace('\r', "\n"))
            }
            NewlinePolicy::ReplaceWithSpace => {
                std::borrow::Cow::Owned(text.replace("\r\n", " ").replace(['\r', '\n'], " "))
            }
            NewlinePolicy::Reject => {
                std::borrow::Cow::Owned(text.chars().filter(|&c| c != '\n' && c != '\r').collect())
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditSpec {
    pub style: TextEditStyle,
    pub error: bool,
    pub disabled: bool,
    pub newline_policy: NewlinePolicy,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextEditSpecBuilder {
    pub style: Option<TextEditStyle>,
    pub error: Option<bool>,
    pub disabled: Option<bool>,
    pub newline_policy: Option<NewlinePolicy>,
}

impl TextEditSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn style(mut self, style: TextEditStyle) -> Self {
        self.style = Some(style);
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(TextEditStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> TextEditSpec {
        TextEditSpec {
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            error: self.error.unwrap_or(false),
            disabled: self.disabled.unwrap_or(false),
            newline_policy: self
                .newline_policy
                .unwrap_or(NewlinePolicy::ReplaceWithSpace),
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
/// This function accepts a TextEditSpecBuilder and calls the low-level raw::text_edit function.
pub fn text_edit<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TextEditSpecBuilder,
    layout_params: S::Params,
    state: &mut TextEditState,
) -> TextEditResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::TextEditCalcIntrinsicSizeSpec { style: spec.style };
    let intrinsic = raw::calc_text_edit_intrinsic_size(&calc_spec, state, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::TextEditSpec {
        rect,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        error: spec.error,
        disabled: spec.disabled,
        time: ctx.time,
        layer: ctx.layer,
        newline_policy: spec.newline_policy,
    };
    let result = raw::text_edit(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_system,
        ctx.cmds,
    );

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
        assert_eq!(
            builder.style.unwrap().text_style.font,
            TextEditStyle::from_theme(&theme).text_style.font
        );
        assert_eq!(
            builder.style.unwrap().text_style.size,
            TextEditStyle::from_theme(&theme).text_style.size
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
        custom_style.text_style.size = 99.0;
        let builder = TextEditSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    fn spec() -> TextEditSpec {
        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 30.0),
            style: TextEditStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            error: false,
            disabled: false,
            time: 0.0,
            layer: Layer::default(),
            newline_policy: NewlinePolicy::ReplaceWithSpace,
        }
    }

    #[test]
    fn test_text_edit_overlapping_hover() {
        let mut text_system = DummyTextSys;
        let mut state1 = TextEditState::default();
        let mut state2 = TextEditState::default();

        crate::widgets::test_helpers::assert_overlapping_hover(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            |state1, state2, input, focus_system, cmds| {
                let mut spec1 = spec();
                spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
                let mut spec2 = spec();
                spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

                let res1 =
                    raw::text_edit(spec1, state1, input, focus_system, &mut text_system, cmds);
                let res2 =
                    raw::text_edit(spec2, state2, input, focus_system, &mut text_system, cmds);
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_text_edit_overlapping_click() {
        let mut text_system = DummyTextSys;
        let mut state1 = TextEditState::default();
        let mut state2 = TextEditState::default();

        crate::widgets::test_helpers::assert_overlapping_click(
            &mut state1,
            &mut state2,
            Vec2::new(75.0, 75.0),
            true,
            |state1, state2, input, focus_system, cmds| {
                let mut spec1 = spec();
                spec1.rect = Rect::new(0.0, 0.0, 100.0, 100.0);
                let mut spec2 = spec();
                spec2.rect = Rect::new(50.0, 50.0, 100.0, 100.0);

                let res1 =
                    raw::text_edit(spec1, state1, input, focus_system, &mut text_system, cmds);
                let res2 =
                    raw::text_edit(spec2, state2, input, focus_system, &mut text_system, cmds);
                (res1.input, res2.input)
            },
        );
    }

    #[test]
    fn test_typing_and_cursor() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("");

        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Char('a'));
        input.text_events.push(TextEvent::Char('b'));
        input.text_events.push(TextEvent::Char('c'));

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "abc");
        assert_eq!(state.caret_byte, 3);

        // Move left
        input.text_events.clear();
        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.caret_byte, 2);

        // Insert at cursor
        input.text_events.clear();
        input.text_events.push(TextEvent::Char('x'));
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "abxc");
        assert_eq!(state.caret_byte, 3);
    }

    #[test]
    fn test_backspace_and_delete() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 3;
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Backspace { ctrl: false });

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "helo");
        assert_eq!(state.caret_byte, 2);

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: false });
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "heo");
        assert_eq!(state.caret_byte, 2);
    }

    #[test]
    fn test_ctrl_backspace_and_delete() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");
        state.caret_byte = 8; // "hello wo|rld"
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::Backspace { ctrl: true });

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "hello rld");
        assert_eq!(state.caret_byte, 6); // end of "hello "

        input.text_events.clear();
        input.text_events.push(TextEvent::Delete { ctrl: true });
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "hello ");
        assert_eq!(state.caret_byte, 6);
    }

    #[test]
    fn test_selection_and_replacement() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 1;
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });
        input.text_events.push(TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        });

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.selection_byte, Some(1));
        assert_eq!(state.caret_byte, 3);

        input.text_events.clear();
        input.text_events.push(TextEvent::Char('a'));
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert_eq!(state.value, "halo");
        assert_eq!(state.caret_byte, 2);
        assert_eq!(state.selection_byte, None);
    }

    #[test]
    fn test_mouse_clicking_and_dragging() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );

        // Frame 1: Warmup to establish hover claim
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 2: Mouse down / press
        input.mouse_down = true;
        input.mouse_pressed = true;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        assert_eq!(state.caret_byte, 5);
        assert!(state.is_dragging);
        state.was_focused = true;

        // Frame 3: Dragging
        input.mouse_pressed = false;
        input.mouse_pos.x += 24.0;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.caret_byte, 8);

        // Frame 4: Mouse up / release
        input.mouse_down = false;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        assert!(!state.is_dragging);
        assert_eq!(state.selection_byte, Some(5));
        assert_eq!(state.caret_byte, 8);
    }

    #[test]
    fn test_double_click_selection_and_drag() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello rust world");

        let mut input = Input::default();
        // Click on "rust" (byte index 8 -> pixel 64)
        input.mouse_pos = crate::types::Vec2::new(
            64.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );

        // Frame 1: Warmup to establish hover claim
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 2: Mouse down / double-press
        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        // Selection should be "rust" (6 to 10)
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 10);
        assert!(state.is_dragging);
        assert_eq!(state.drag_word_origin, Some((6, 10)));

        // Frame 3: Drag right to "world" (byte index 14 -> pixel 112)
        input.mouse_pressed = false;
        input.mouse_pos.x = 112.0 + spec().style.padding + spec().style.border_width;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        // Should select "rust world", so from 6 to 16
        assert_eq!(state.selection_byte, Some(6)); // original start
        assert_eq!(state.caret_byte, 16); // end of "world"

        // Frame 4: Drag left to "hello" (byte index 2 -> pixel 16)
        input.mouse_pos.x = 16.0 + spec().style.padding + spec().style.border_width;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();
        // Should select "hello rust", so from 10 to 0
        assert_eq!(state.selection_byte, Some(10)); // original end
        assert_eq!(state.caret_byte, 0); // start of "hello"
    }

    #[test]
    fn test_double_click_symmetry() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        let mut run_double_click = |x_within_text: f32| -> (Option<usize>, usize) {
            let mut state = TextEditState::new("a b");
            let mut input = Input::default();
            let x_offset = spec().style.padding + spec().style.border_width;
            input.mouse_pos = crate::types::Vec2::new(x_within_text + x_offset, 8.0);

            // Frame 1: Hover
            focus_system.begin_frame();
            raw::text_edit(
                spec(),
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut DrawCommands::new(),
            );
            focus_system.end_frame();

            // Frame 2: Double click
            input.mouse_down = true;
            input.mouse_pressed = true;
            input.mouse_click_count = 2;
            focus_system.begin_frame();
            raw::text_edit(
                spec(),
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut DrawCommands::new(),
            );
            focus_system.end_frame();

            (state.selection_byte, state.caret_byte)
        };

        // Click at various positions in "a b"
        // 1. In 'a' [0.0, 8.0) -> should select "a" (0..1)
        // Left extreme: 1.0
        assert_eq!(run_double_click(1.0), (Some(0), 1));
        // Right extreme: 7.0
        assert_eq!(run_double_click(7.0), (Some(0), 1));

        // 2. In ' ' [8.0, 16.0) -> should select " " (1..2)
        // Left half: 9.0
        assert_eq!(run_double_click(9.0), (Some(1), 2));
        // Right half: 15.0
        assert_eq!(run_double_click(15.0), (Some(1), 2));

        // 3. In 'b' [16.0, 24.0) -> should select "b" (2..3)
        // Left extreme: 17.0
        assert_eq!(run_double_click(17.0), (Some(2), 3));
        // Right extreme: 23.0
        assert_eq!(run_double_click(23.0), (Some(2), 3));
    }

    #[test]
    fn test_double_click_after_line_end() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        let mut run_double_click = |text: &str, y_pos: f32| -> (Option<usize>, usize) {
            let mut state = TextEditState::new(text);
            let mut input = Input::default();
            // Click way past the end of the line (e.g. x = 100.0)
            let x_offset = spec().style.padding + spec().style.border_width;
            input.mouse_pos = crate::types::Vec2::new(100.0 + x_offset, y_pos);

            let edit_spec = TextEditSpec {
                newline_policy: NewlinePolicy::Allow,
                ..spec()
            };

            // Frame 1: Hover
            focus_system.begin_frame();
            raw::text_edit(
                edit_spec.clone(),
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut DrawCommands::new(),
            );
            focus_system.end_frame();

            // Frame 2: Double click
            input.mouse_down = true;
            input.mouse_pressed = true;
            input.mouse_click_count = 2;
            focus_system.begin_frame();
            raw::text_edit(
                edit_spec,
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut DrawCommands::new(),
            );
            focus_system.end_frame();

            (state.selection_byte, state.caret_byte)
        };

        // Case 1: Line has trailing \n. Double-clicking after line end should select just the \n character.
        // "hello\n" -> '\n' is at index 5.
        // First line is "hello\n", so we click on line 0 (y = 8.0).
        assert_eq!(run_double_click("hello\n", 8.0), (Some(5), 6));

        // Case 2: Line has trailing \n and is followed by another line.
        // "hello\nworld" -> '\n' is at index 5.
        // First line is "hello\n", click at y = 8.0.
        assert_eq!(run_double_click("hello\nworld", 8.0), (Some(5), 6));

        // Case 3: Line has no trailing \n. Double-clicking after line end should select the trailing word.
        // "hello" -> trailing word is "hello" (0..5).
        assert_eq!(run_double_click("hello", 8.0), (Some(0), 5));

        // Case 4: Line has no trailing \n but is preceded by a newline.
        // "hello\nworld" -> second line is "world" (6..11), click at y = 24.0.
        assert_eq!(run_double_click("hello\nworld", 24.0), (Some(6), 11));
    }

    #[test]
    fn test_caret_blink_reset_on_move() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        state.caret_byte = 5;
        state.was_focused = true;

        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut input = Input::default();

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        let has_caret = cmds.iter().any(
            |cmd| matches!(cmd, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == spec().style.caret_color),
        );
        assert!(has_caret, "Caret should be visible initially");

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        let has_caret = cmds.iter().any(
            |cmd| matches!(cmd, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == spec().style.caret_color),
        );
        assert!(!has_caret, "Caret should be hidden during off phase");

        input.text_events.push(TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        });
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.last_caret_move_time, 0.6);

        let has_caret = cmds.iter().any(
            |cmd| matches!(cmd, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == spec().style.caret_color),
        );
        assert!(
            has_caret,
            "Caret should be visible immediately after moving"
        );

        input.text_events.clear();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                time: 1.0,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        let has_caret = cmds.iter().any(
            |cmd| matches!(cmd, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == spec().style.caret_color),
        );
        assert!(has_caret, "Caret should stay visible for 0.5s after moving");

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                time: 1.2,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        let has_caret = cmds.iter().any(
            |cmd| matches!(cmd, DrawCmd::FillRect { anti_alias: false, color, .. } if *color == spec().style.caret_color),
        );
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
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let input = Input::default();

        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert!(state.was_focused);
        assert_eq!(state.selection_byte, Some(0));
        assert_eq!(state.caret_byte, 11);
    }

    #[test]
    fn test_mouse_focus_no_select_all() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding + spec().style.border_width,
            15.0,
        );

        // Frame 1: Warmup to establish hover claim
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 2: Mouse down / press
        focus_system.begin_frame();
        input.mouse_down = true;
        input.mouse_pressed = true;
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 3: Mouse release
        focus_system.begin_frame();
        input.mouse_pressed = false;
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert!(state.was_focused);
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 5);
    }

    #[test]
    fn test_text_edit_click_takes_focus() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(10.0, 15.0);

        // Frame 1: Warmup to establish hover claim
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 2: Mouse pressed
        input.mouse_pressed = true;
        input.mouse_down = true;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Clicking text edit must request focus"
        );
    }

    #[test]
    fn test_text_edit_clipped_click_does_not_take_focus() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
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

        focus_system.begin_frame();
        raw::text_edit(
            clipped_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "Clicking a clipped-away text edit must not take focus"
        );
    }

    #[test]
    fn test_clipboard_actions() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        state.selection_byte = Some(6);
        state.caret_byte = 11;
        state.was_focused = true;

        let mut input = Input::default();
        input.text_events.push(TextEvent::Copy);
        let res = raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Copy(s)) if s == "world"));
        assert_eq!(state.value, "hello world");

        input.text_events.clear();
        input.text_events.push(TextEvent::Cut);
        let res = raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert!(matches!(&res.clipboard_action, Some(ClipboardAction::Cut(s)) if s == "world"));
        assert_eq!(state.value, "hello ");
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 6);

        input.text_events.clear();
        input.text_events.push(TextEvent::Paste("rust".to_string()));
        let res = raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        assert!(res.clipboard_action.is_none());
        assert_eq!(state.value, "hello rust");
        assert_eq!(state.caret_byte, 10);
    }

    // ── Visual Tests ───────────────────────────────────────────────────────────

    #[test]
    fn test_text_edit_visual_normal() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.border,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(1.0, 1.0, 198.0, 28.0),
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 198.0, 28.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_focused_caret() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.was_focused = true; // ensure state knows

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.focus,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(1.0, 1.0, 198.0, 28.0),
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 198.0, 28.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(45.0, 7.0, 1.0, 16.0),
                    color: spec().style.caret_color,
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_focused_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.was_focused = true;
        state.selection_byte = Some(0);
        state.caret_byte = 5;

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: spec().style.focus,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(1.0, 1.0, 198.0, 28.0),
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 7.0, 40.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 7.0, 198.0, 28.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_error() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        let mut sp = spec();
        sp.error = true;

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            sp.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: sp.style.error_background,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 4.0, 30.0),
                    color: sp.style.error_border,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 30.0),
                    color: sp.style.error_border,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(5.0, 1.0, 194.0, 28.0),
                },
                DrawCmd::Text {
                    rect: Rect::new(9.0, 7.0, 194.0, 28.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layouts::ManualLayout;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut te_state = TextEditState::default();
        let result = super::text_edit(
            &mut ctx,
            TextEditSpecBuilder::new(),
            custom_rect,
            &mut te_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }

    #[test]
    fn test_text_edit_caret_auto_scrolling() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        focus_system.begin_frame();

        // 36 characters. Width = 36 * 8 = 288. Inner scroll width = 288 + 8 = 296.
        // Viewport width = 200 - 2 = 198.
        // Max scroll = 296 - 198 = 98.
        let mut state = TextEditState::new("hello world how are you today doing");
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        // 1. Caret at start (0): scroll should be 0.0
        state.caret_byte = 0;
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.scroll.offset.x, 0.0);

        // 2. Caret moves from 23 to 24 (x = 192): exceeds right threshold (198 - 16 = 182)
        // Expected scroll = 192 - 198 + 16 = 10.0
        state.caret_byte = 23;
        input.text_events = vec![TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 24);
        assert_eq!(state.scroll.offset.x, 10.0);

        // 3. Caret moves from 34 to 35 (x = 280): exceeds right threshold
        // Expected scroll = 280 - 198 + 16 = 98.0, clamped to max_scroll (90.0)
        state.caret_byte = 34;
        input.text_events = vec![TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 35);
        assert_eq!(state.scroll.offset.x, 90.0);

        // 4. Move caret left from 3 to 2 (x = 16): below left threshold (98.0 + 16 = 114)
        // Expected scroll = 16 - 16 = 0.0
        state.caret_byte = 3;
        input.text_events = vec![TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 2);
        assert_eq!(state.scroll.offset.x, 0.0);
    }

    #[test]
    fn test_selection_aware_auto_scrolling() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        // String: "leftwordoverlappingedge middle rightwordoverlappingedge"
        // Character counts:
        // leftwordoverlappingedge: 23 chars (0..23)
        // space: 1 char (23)
        // middle: 6 chars (24..30)
        // space: 1 char (30)
        // rightwordoverlappingedge: 24 chars (31..55)
        //
        // Widths (at 8px per char):
        // leftwordoverlappingedge: 184px (0.0..184.0)
        // middle: 48px (192.0..240.0)
        // rightwordoverlappingedge: 192px (248.0..440.0)
        //
        // Total text width: 440px
        // Inner scroll size: width = 440 + 2 * padding(16) = 472px
        // Viewport width: 200px (from spec())
        // Max scroll: 472 - 200 = 272px
        let mut state =
            TextEditState::new("leftwordoverlappingedge middle rightwordoverlappingedge");
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        // Warmup frame to establish hover
        input.mouse_pos = Vec2::new(10.0, 15.0);
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Test case 1: Ctrl-A (Select All) should not change the scroll state.
        state.scroll.offset.x = 120.0;
        input.text_events = vec![TextEvent::SelectAll];
        input.mouse_pressed = false;
        input.mouse_click_count = 0;
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.scroll.offset.x, 120.0);

        // Test case 2: Double-clicking the long word on the left should scroll the viewport left
        // just far enough to move the end of that word to the right of the viewport.
        state.scroll.offset.x = 120.0;
        input.text_events = vec![];
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        input.mouse_pos = Vec2::new(
            136.0 + spec().style.padding + spec().style.border_width - 120.0,
            15.0,
        );
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.selection_byte, Some(0));
        assert_eq!(state.caret_byte, 23);
        assert_eq!(state.scroll.offset.x, 2.0);

        // Test case 3: Double-clicking the long word on the right should scroll the viewport right
        // just far enough to align the start of the word with the left edge of the viewport.
        state.scroll.offset.x = 120.0;
        input.text_events = vec![];
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        input.mouse_pos = Vec2::new(
            256.0 + spec().style.padding + spec().style.border_width - 120.0,
            15.0,
        );
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.selection_byte, Some(31));
        assert_eq!(state.caret_byte, 55);
        assert_eq!(state.scroll.offset.x, 232.0);
    }

    #[test]
    fn test_text_edit_scroll_coordinate_translation() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world how are you today doing");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();
        state.was_focused = true;

        // Manually inject a scroll offset of 50.0
        state.scroll.offset.x = 50.0;
        // Selection from index 0 to 5
        state.selection_byte = Some(0);
        state.caret_byte = 5;

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Find and check the coordinates of Text, Caret FillRect, and Selection FillRect
        let mut found_text = false;
        let mut found_selection = false;

        for cmd in cmds.iter() {
            match cmd {
                DrawCmd::Text { rect, .. } => {
                    // Originally, text_x was: outer_rect.x + padding = 1.0 + 4.0 = 5.0
                    // Scrolled left by 50.0 -> 5.0 - 50.0 = -45.0
                    assert_eq!(rect.x, -45.0);
                    found_text = true;
                }
                DrawCmd::FillRect { rect, color, .. } => {
                    if *color == spec().style.select_color {
                        // Selection starts at 0 (x = 0) and ends at 5 (x = 40)
                        // Selection rect.x: text_rect.x + start = -45.0 + 0 = -45.0
                        assert_eq!(rect.x, -45.0);
                        assert_eq!(rect.w, 40.0);
                        found_selection = true;
                    }
                }
                _ => {}
            }
        }

        assert!(found_text);
        assert!(found_selection);
    }

    #[test]
    fn test_text_edit_click_with_scroll_offset() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world how are you today doing");

        // Manually inject a scroll offset of 50.0
        state.scroll.offset.x = 50.0;

        let mut input = Input::default();
        input.mouse_pos = Vec2::new(45.0, 15.0);

        // Frame 1: Warmup to establish hover claim
        focus_system.begin_frame();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        // Frame 2: Mouse pressed
        focus_system.begin_frame();
        input.mouse_pressed = true;
        input.mouse_down = true;

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Caret should have jumped to 11
        assert_eq!(state.caret_byte, 11);
    }

    #[test]
    fn test_text_edit_caret_movement_with_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        focus_system.begin_frame();

        let mut state = TextEditState::new("hello world how are you");
        state.was_focused = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        // 1. Press CaretLeft (shift=false, ctrl=false) -> collapses selection to left edge (6)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretLeft {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 6);

        // 2. Press CaretRight (shift=false, ctrl=false) -> collapses selection to right edge (11)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretRight {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 11);

        // 3. Press Ctrl+CaretLeft (shift=false, ctrl=true) -> starts at left edge (6) and moves one word left (to 5)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretLeft {
            shift: false,
            ctrl: true,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 5);

        // 4. Press Ctrl+CaretRight (shift=false, ctrl=true) -> starts at right edge (11) and moves one word right (to 12)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretRight {
            shift: false,
            ctrl: true,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, None);
        assert_eq!(state.caret_byte, 12);

        // 5. Press Shift+CaretLeft (shift=true, ctrl=false) -> starts at caret (11) and moves one character left (to 10)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretLeft {
            shift: true,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 10);

        // 6. Press Shift+CaretRight (shift=true, ctrl=false) -> starts at caret (11) and moves one character right (to 12)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretRight {
            shift: true,
            ctrl: false,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 12);

        // 7. Press Ctrl+Shift+CaretLeft (shift=true, ctrl=true) -> starts at caret (11) and moves one word left (to 6)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretLeft {
            shift: true,
            ctrl: true,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 6);

        // 8. Press Ctrl+Shift+CaretRight (shift=true, ctrl=true) -> starts at caret (11) and moves one word right (to 12)
        state.selection_byte = Some(6);
        state.caret_byte = 11;
        input.text_events = vec![TextEvent::CaretRight {
            shift: true,
            ctrl: true,
        }];
        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, Some(6));
        assert_eq!(state.caret_byte, 12);
    }

    #[test]
    fn test_newline_policies() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // 1. ReplaceWithSpace (default)
        {
            // Initial value contains \n
            let mut state = TextEditState::new("hello\nworld");
            let mut input = Input::default();
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::ReplaceWithSpace,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "hello world");

            // Paste value containing \n
            state.caret_byte = 5;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Paste("a\nb".to_string())];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::ReplaceWithSpace,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "helloa b world");

            // Type \n -> inserts space
            state.caret_byte = 0;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Char('\n')];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::ReplaceWithSpace,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, " helloa b world");

            // Press Enter -> does not change value (because policy is ReplaceWithSpace, not Allow)
            input.text_events.clear();
            input.key_pressed_enter = true;
            let mut state = TextEditState::new("abc");
            state.was_focused = true;
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::ReplaceWithSpace,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "abc");
        }

        // 2. Reject
        {
            // Initial value contains \n -> removed
            let mut state = TextEditState::new("hello\nworld");
            let mut input = Input::default();
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Reject,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "helloworld");

            // Paste value containing \n -> \n is removed
            state.caret_byte = 5;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Paste("a\nb".to_string())];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Reject,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "helloabworld");

            // Type \n -> ignored
            state.caret_byte = 0;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Char('\n')];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Reject,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "helloabworld");
        }

        // 3. Allow
        {
            // Initial value contains \n -> preserved
            let mut state = TextEditState::new("hello\nworld");
            let mut input = Input::default();
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Allow,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "hello\nworld");

            // Paste value containing \n -> preserved
            state.caret_byte = 5;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Paste("a\nb".to_string())];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Allow,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "helloa\nb\nworld");

            // Type \n -> inserts \n
            state.caret_byte = 0;
            state.selection_byte = None;
            input.text_events = vec![TextEvent::Char('\n')];
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Allow,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "\nhelloa\nb\nworld");

            // Press Enter while focused -> inserts \n
            let mut state = TextEditState::new("abc");
            state.caret_byte = 1;
            state.selection_byte = None;
            state.was_focused = true;
            let mut input = Input::default();
            input.key_pressed_enter = true;
            focus_system.begin_frame();
            focus_system.take_keyboard_focus(state.focus_id);
            raw::text_edit(
                TextEditSpec {
                    newline_policy: NewlinePolicy::Allow,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            assert_eq!(state.value, "a\nbc");
        }
    }

    #[test]
    fn test_caret_up_down_navigation() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        // Initial text: three lines, each 5 characters (excluding newline) -> 8px * 5 = 40px wide per line
        // "line1\nline2\nline3"
        // Line 0: "line1\n" -> byte_start=0, byte_end=6
        // Line 1: "line2\n" -> byte_start=6, byte_end=12
        // Line 2: "line3"   -> byte_start=12, byte_end=17
        let mut state = TextEditState::new("line1\nline2\nline3");
        let mut input = Input::default();
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // Initialize/prepare the layout once
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // 1. Arrow Down from Line 0 to Line 1
        state.caret_byte = 5;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 11);

        // 2. Arrow Up from Line 1 to Line 0
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 5);

        // 3. Boundary Condition: Arrow Up on first line
        state.caret_byte = 2;
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 0);

        // 4. Boundary Condition: Arrow Down on last line
        state.caret_byte = 14;
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.caret_byte, 17);

        // 5. Shift + Arrow Down from Line 0 to Line 1 (extending selection)
        state.caret_byte = 2;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretDown { shift: true }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.selection_byte, Some(2));
        assert_eq!(state.caret_byte, 8);
    }

    // ── Home / End navigation ───────────────────────────────────────────────────
    //
    // Text used throughout: "line1\nline2\nline3"
    //   Line 0: bytes  0 ..  6  ("line1\n")
    //   Line 1: bytes  6 .. 12  ("line2\n")
    //   Line 2: bytes 12 .. 17  ("line3")
    //
    // Expected behaviour
    // ------------------
    // Home (ctrl=false): move caret to the first byte of the *current* line.
    // End  (ctrl=false): move caret to the last byte of the *current* line
    //                    (i.e. just before '\n', or to value.len() on the last line).
    // Home (ctrl=true) : move caret to byte 0 (start of the whole string).
    // End  (ctrl=true) : move caret to value.len() (end of the whole string).
    // Adding Shift extends the selection from the *old* caret position.
    //
    // NOTE: these tests are expected to FAIL with the current implementation.
    // CaretHome / CaretEnd today always jump to 0 / value.len() irrespective of
    // `ctrl`, and they have no line-awareness at all.
    #[test]
    fn test_home_end_multiline() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        let mut state = TextEditState::new("line1\nline2\nline3");
        state.was_focused = true;
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // Warm-up frame so the widget knows the layout.
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        let mut input = Input::default();

        // ── 1. Home (ctrl=false) from middle of Line 1 → start of Line 1 ──────
        state.caret_byte = 9; // "line2|2\n" → inside Line 1
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretHome {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 6,
            "Home (no ctrl) from line 1 mid should move to start of line 1 (byte 6)"
        );
        assert_eq!(state.selection_byte, None);

        // ── 2. End (ctrl=false) from middle of Line 1 → end of Line 1 ──────────
        state.caret_byte = 9; // restore to mid-line-1
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 11,
            "End (no ctrl) from line 1 mid should move to end of line 1 (byte 11, before \\n)"
        );
        assert_eq!(state.selection_byte, None);

        // ── 3. Home (ctrl=false) on Line 0 already at start → stays at 0 ───────
        state.caret_byte = 0;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretHome {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 0,
            "Home (no ctrl) at byte 0 should stay at 0"
        );

        // ── 4. End (ctrl=false) on last line → value.len() ─────────────────────
        state.caret_byte = 14; // inside "line3"
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 17,
            "End (no ctrl) on last line should move to value.len() (byte 17)"
        );

        // ── 5. Shift+Home (ctrl=false) extends selection to start of line ──────
        state.caret_byte = 9; // mid-line-1
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretHome {
            shift: true,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.selection_byte,
            Some(9),
            "Shift+Home (no ctrl) should anchor selection at old caret (9)"
        );
        assert_eq!(
            state.caret_byte, 6,
            "Shift+Home (no ctrl) should move caret to start of current line (6)"
        );

        // ── 6. Shift+End (ctrl=false) extends selection to end of line ──────────
        state.caret_byte = 9; // mid-line-1
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretEnd {
            shift: true,
            ctrl: false,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.selection_byte,
            Some(9),
            "Shift+End (no ctrl) should anchor selection at old caret (9)"
        );
        assert_eq!(
            state.caret_byte, 11,
            "Shift+End (no ctrl) should move caret to end of current line (11)"
        );

        // ── 7. Home (ctrl=true) from mid-string → byte 0 ───────────────────────
        state.caret_byte = 9;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretHome {
            shift: false,
            ctrl: true,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 0,
            "Home (ctrl=true) should always move to byte 0"
        );
        assert_eq!(state.selection_byte, None);

        // ── 8. End (ctrl=true) from mid-string → value.len() ───────────────────
        state.caret_byte = 9;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretEnd {
            shift: false,
            ctrl: true,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.caret_byte, 17,
            "End (ctrl=true) should always move to value.len()"
        );
        assert_eq!(state.selection_byte, None);

        // ── 9. Shift+Ctrl+Home extends selection to byte 0 ──────────────────────
        state.caret_byte = 9;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretHome {
            shift: true,
            ctrl: true,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.selection_byte,
            Some(9),
            "Shift+Ctrl+Home should anchor selection at old caret (9)"
        );
        assert_eq!(
            state.caret_byte, 0,
            "Shift+Ctrl+Home should move caret to byte 0"
        );

        // ── 10. Shift+Ctrl+End extends selection to value.len() ─────────────────
        state.caret_byte = 9;
        state.selection_byte = None;
        input.text_events = vec![TextEvent::CaretEnd {
            shift: true,
            ctrl: true,
        }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(
            state.selection_byte,
            Some(9),
            "Shift+Ctrl+End should anchor selection at old caret (9)"
        );
        assert_eq!(
            state.caret_byte, 17,
            "Shift+Ctrl+End should move caret to value.len()"
        );
    }

    #[test]
    fn test_text_edit_visual_multiline_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello\nworld");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.was_focused = true;
        state.selection_byte = Some(3); // 'l' in "hello"
        state.caret_byte = 9; // 'r' in "world"

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                newline_policy: NewlinePolicy::Allow,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                    color: spec().style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                    color: spec().style.focus,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(1.0, 1.0, 198.0, 98.0),
                },
                // Selection Rect for Line 0: "lo\n"
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(29.0, 34.0, 24.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                // Selection Rect for Line 1: "wo"
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 50.0, 24.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 34.0, 198.0, 98.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_visual_multiline_selection_three_lines() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("one\ntwo\nthree");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.was_focused = true;
        state.selection_byte = Some(2); // 'e' in "one"
        state.caret_byte = 10; // 'r' in "three"

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                newline_policy: NewlinePolicy::Allow,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                    color: spec().style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                    color: spec().style.focus,
                    width: spec().style.border_width,
                    z: 0,
                },
                DrawCmd::PushClip {
                    rect: Rect::new(1.0, 1.0, 198.0, 98.0),
                },
                // Selection Rect for Line 0: "e\n"
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(21.0, 26.0, 16.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                // Selection Rect for Line 1: "two\n"
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 42.0, 32.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                // Selection Rect for Line 2: "th"
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(5.0, 58.0, 16.0, 16.0),
                    color: spec().style.select_color,
                    z: 0,
                },
                DrawCmd::Text {
                    rect: Rect::new(5.0, 26.0, 198.0, 98.0),
                    color: spec().style.text_color,
                    handle: crate::text::TextHandle(0),
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }
}
