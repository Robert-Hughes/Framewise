use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::{Input, TextEvent},
    layout::{Align, IntrinsicSize, LayoutState},
    text::{
        CaretPosition, FontId, LineEndKind, LineHeight, LineMetrics, TextBounds, TextFlow,
        TextLineAlign, TextMetrics, TextStyle, TextSystem,
    },
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
    pub struct TextEditCalcIntrinsicSizeSpec {
        pub style: super::TextEditStyle,
        pub wrap: bool,
        pub line_align: TextLineAlign,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TextEditResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub clipboard_action: Option<ClipboardAction>,
    }

    fn visual_line_index_at_y(metrics: &TextMetrics, y: f32) -> usize {
        metrics
            .lines
            .iter()
            .position(|line| y >= line.y_top && y < line.y_top + line.height)
            .or_else(|| metrics.lines.iter().rposition(|line| y >= line.y_top))
            .unwrap_or(0)
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
    fn move_caret_vertical<T: TextSystem>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_system: &mut T,
        caret: CaretPosition,
        caret_byte: usize,
        caret_is_current: bool,
        start_byte: usize,
        direction: VerticalCaretDirection,
        line_count: usize,
    ) -> VerticalCaretMove {
        let (_, layout_width, layout_height, _) =
            edit_layout_size(text_content, spec, text_style, text_system);
        let layout = text_system.prepare(
            text_content,
            text_style,
            Rect::new(0.0, 0.0, layout_width, layout_height),
        );
        let handle = layout.handle;
        let metrics = layout.metrics;

        let visual_position = if caret_is_current && start_byte == caret_byte {
            caret
        } else {
            text_system.caret_position_at_insertion_byte(handle, start_byte)
        };
        let caret_geom = text_system.caret_geom(handle, visual_position);
        let current_line_idx = metrics
            .lines
            .iter()
            .rposition(|line| start_byte >= line.byte_start)
            .unwrap_or(0);

        let line_len = metrics.lines.len();
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

        let target_line = &metrics.lines[target_line_idx];
        let pos = Vec2::new(caret_geom.x, target_line.y_top + target_line.height * 0.5);
        let new_caret = text_system.hit_test_caret(handle, pos);
        let byte = text_system
            .caret_insertion_byte(handle, new_caret)
            .min(text_content.len());
        VerticalCaretMove {
            caret: new_caret,
            byte,
            needs_layout_sync: false,
        }
    }

    fn page_line_count<T: TextSystem>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_system: &mut T,
        caret_byte: usize,
        scroll_outer_height: f32,
    ) -> usize {
        let (_, layout_width, layout_height, _) =
            edit_layout_size(text_content, spec, text_style, text_system);
        let layout = text_system.prepare(
            text_content,
            text_style,
            Rect::new(0.0, 0.0, layout_width, layout_height),
        );
        let caret = text_system.caret_position_at_insertion_byte(layout.handle, caret_byte);
        let line_height = text_system.caret_geom(layout.handle, caret).height.max(1.0);
        (scroll_outer_height / line_height).floor().max(1.0) as usize
    }

    /// Measure a text edit's intrinsic size from its current state and measurement spec.
    pub fn calc_text_edit_intrinsic_size<T: TextSystem>(
        spec: &TextEditCalcIntrinsicSizeSpec,
        state: &TextEditState,
        text_system: &mut T,
    ) -> IntrinsicSize {
        let metrics = text_system.measure(
            &state.value,
            to_text_style(spec.style, spec.wrap, spec.line_align),
            TextBounds::UNBOUNDED,
        );
        IntrinsicSize::preferred(Vec2::new(
            metrics.logical_size.x + (spec.style.border_width + spec.style.padding_x) * 2.0,
            (metrics.logical_size.y + (spec.style.border_width + spec.style.padding_y) * 2.0)
                .max(spec.style.min_height),
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
        let text_style = to_text_style(spec.style, spec.wrap, spec.line_align);
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
            let inset_x = spec.style.border_width + spec.style.padding_x;
            let inset_y = spec.style.border_width + spec.style.padding_y;
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
                let metrics = text_system.measure(
                    text,
                    text_style,
                    TextBounds {
                        max_width: Some(content_rect.w),
                        max_height: Some(content_rect.h),
                    },
                );
                let ty = match spec.vertical_align {
                    Align::Start => content_rect.y,
                    Align::Center => {
                        content_rect.y + (content_rect.h - metrics.logical_size.y) / 2.0
                    }
                    Align::End => content_rect.y + content_rect.h - metrics.logical_size.y,
                };
                let text_rect = Rect::new(content_rect.x, ty, content_rect.w, content_rect.h);
                let layout = text_system.prepare(text, text_style, text_rect);
                let color = if state.value.is_empty() {
                    spec.style.placeholder_color
                } else {
                    spec.style.text_color
                };
                cmds.push(DrawCmd::Text {
                    rect: text_rect,
                    color: tint(color),
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
        let just_focused = focused && !state.had_keyboard_focus;

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

        let initial_text_content = state.value.as_str();
        let (_, initial_layout_width, initial_layout_height, _) =
            edit_layout_size(initial_text_content, &spec, text_style, text_system);
        let initial_layout = text_system.prepare(
            initial_text_content,
            text_style,
            Rect::new(0.0, 0.0, initial_layout_width, initial_layout_height),
        );
        let initial_handle = initial_layout.handle;

        let mut caret_byte = text_system
            .caret_insertion_byte(initial_handle, state.caret)
            .min(state.value.len());
        let mut selection_byte = state
            .selection_anchor
            .map(|selection| {
                text_system
                    .caret_insertion_byte(initial_handle, selection)
                    .min(state.value.len())
            })
            .filter(|selection| state.value.is_char_boundary(*selection));

        let old_caret = state.caret;
        let old_selection = state.selection_anchor;
        let mut caret = state.caret;
        let mut caret_needs_layout_sync = false;

        if just_focused && !state.suppress_select_all_on_next_focus {
            selection_byte = Some(0);
            caret_byte = state.value.len();
            caret_needs_layout_sync = true;
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
                            remove_selection(
                                &mut state.value,
                                &mut caret_byte,
                                &mut selection_byte,
                            );
                            let char_to_insert = if is_newline {
                                if spec.newline_policy == NewlinePolicy::ReplaceWithSpace {
                                    ' '
                                } else {
                                    '\n'
                                }
                            } else {
                                *c
                            };
                            state.value.insert(caret_byte, char_to_insert);
                            caret_byte += char_to_insert.len_utf8();
                            caret_needs_layout_sync = true;
                            if is_newline {
                                newline_inserted = true;
                            }
                        }
                    }
                    TextEvent::Backspace { ctrl } => {
                        if selection_byte.is_some() {
                            remove_selection(
                                &mut state.value,
                                &mut caret_byte,
                                &mut selection_byte,
                            );
                            caret_needs_layout_sync = true;
                        } else if *ctrl {
                            let prev = find_word_boundary(&state.value, caret_byte, false);
                            state.value.replace_range(prev..caret_byte, "");
                            caret_byte = prev;
                            caret_needs_layout_sync = true;
                        } else if caret_byte > 0 {
                            // Find previous char boundary
                            let mut prev = caret_byte - 1;
                            while prev > 0 && !state.value.is_char_boundary(prev) {
                                prev -= 1;
                            }
                            state.value.remove(prev);
                            caret_byte = prev;
                            caret_needs_layout_sync = true;
                        }
                    }
                    TextEvent::Delete { ctrl } => {
                        if selection_byte.is_some() {
                            remove_selection(
                                &mut state.value,
                                &mut caret_byte,
                                &mut selection_byte,
                            );
                            caret_needs_layout_sync = true;
                        } else if *ctrl {
                            let next = find_word_boundary(&state.value, caret_byte, true);
                            state.value.replace_range(caret_byte..next, "");
                            caret_needs_layout_sync = true;
                        } else if caret_byte < state.value.len() {
                            state.value.remove(caret_byte);
                            caret_needs_layout_sync = true;
                        }
                    }
                    TextEvent::CaretLeft { shift, ctrl } => {
                        let sel_byte = selection_byte;
                        let has_selection = sel_byte.is_some() && sel_byte != Some(caret_byte);

                        if *shift {
                            if selection_byte.is_none() {
                                selection_byte = Some(caret_byte);
                            }
                        } else {
                            selection_byte = None;
                        }

                        if *ctrl {
                            let start_byte = if has_selection && !*shift {
                                caret_byte.min(sel_byte.unwrap())
                            } else {
                                caret_byte
                            };
                            caret_byte = find_word_boundary(&state.value, start_byte, false);
                            caret_needs_layout_sync = true;
                        } else if has_selection && !*shift {
                            caret_byte = caret_byte.min(sel_byte.unwrap());
                            caret_needs_layout_sync = true;
                        } else if caret_byte > 0 {
                            caret = text_system.previous_caret_position(initial_handle, caret);
                            caret_byte = text_system
                                .caret_insertion_byte(initial_handle, caret)
                                .min(state.value.len());
                            caret_needs_layout_sync = false;
                        }
                    }
                    TextEvent::CaretRight { shift, ctrl } => {
                        let sel_byte = selection_byte;
                        let has_selection = sel_byte.is_some() && sel_byte != Some(caret_byte);

                        if *shift {
                            if selection_byte.is_none() {
                                selection_byte = Some(caret_byte);
                            }
                        } else {
                            selection_byte = None;
                        }

                        if *ctrl {
                            let start_byte = if has_selection && !*shift {
                                caret_byte.max(sel_byte.unwrap())
                            } else {
                                caret_byte
                            };
                            caret_byte = find_word_boundary(&state.value, start_byte, true);
                            caret_needs_layout_sync = true;
                        } else if has_selection && !*shift {
                            caret_byte = caret_byte.max(sel_byte.unwrap());
                            caret_needs_layout_sync = true;
                        } else if caret_byte < state.value.len() {
                            caret = text_system.next_caret_position(initial_handle, caret);
                            caret_byte = text_system
                                .caret_insertion_byte(initial_handle, caret)
                                .min(state.value.len());
                            caret_needs_layout_sync = false;
                        }
                    }
                    TextEvent::CaretUp { shift } => {
                        let sel_byte = selection_byte;
                        let has_selection = sel_byte.is_some() && sel_byte != Some(caret_byte);

                        if *shift {
                            if selection_byte.is_none() {
                                selection_byte = Some(caret_byte);
                            }
                        } else {
                            selection_byte = None;
                        }

                        let start_byte = if has_selection && !*shift {
                            caret_byte.min(sel_byte.unwrap())
                        } else {
                            caret_byte
                        };

                        let moved = move_caret_vertical(
                            state.value.as_str(),
                            &spec,
                            text_style,
                            text_system,
                            caret,
                            caret_byte,
                            !caret_needs_layout_sync,
                            start_byte,
                            VerticalCaretDirection::Up,
                            1,
                        );
                        caret = moved.caret;
                        caret_byte = moved.byte;
                        caret_needs_layout_sync = moved.needs_layout_sync;
                    }
                    TextEvent::CaretDown { shift } => {
                        let sel_byte = selection_byte;
                        let has_selection = sel_byte.is_some() && sel_byte != Some(caret_byte);

                        if *shift {
                            if selection_byte.is_none() {
                                selection_byte = Some(caret_byte);
                            }
                        } else {
                            selection_byte = None;
                        }

                        let start_byte = if has_selection && !*shift {
                            caret_byte.max(sel_byte.unwrap())
                        } else {
                            caret_byte
                        };

                        let moved = move_caret_vertical(
                            state.value.as_str(),
                            &spec,
                            text_style,
                            text_system,
                            caret,
                            caret_byte,
                            !caret_needs_layout_sync,
                            start_byte,
                            VerticalCaretDirection::Down,
                            1,
                        );
                        caret = moved.caret;
                        caret_byte = moved.byte;
                        caret_needs_layout_sync = moved.needs_layout_sync;
                    }
                    TextEvent::CaretHome { shift, ctrl } => {
                        if *shift && selection_byte.is_none() {
                            selection_byte = Some(caret_byte);
                        } else if !*shift {
                            selection_byte = None;
                        }
                        if *ctrl {
                            caret_byte = 0;
                            caret_needs_layout_sync = true;
                        } else if spec.wrap {
                            let text_content = state.value.as_str();
                            let (_, layout_width, layout_height, _) =
                                edit_layout_size(text_content, &spec, text_style, text_system);
                            let layout = text_system.prepare(
                                text_content,
                                text_style,
                                Rect::new(0.0, 0.0, layout_width, layout_height),
                            );
                            let handle = layout.handle;
                            let metrics = layout.metrics;
                            let visual_caret = if caret_needs_layout_sync {
                                text_system.caret_position_at_insertion_byte(handle, caret_byte)
                            } else {
                                caret
                            };
                            let caret_geom = text_system.caret_geom(handle, visual_caret);
                            let caret_mid_y = caret_geom.y_top + caret_geom.height * 0.5;
                            let current_line_idx = visual_line_index_at_y(&metrics, caret_mid_y);
                            let line = &metrics.lines[current_line_idx];
                            let line_mid_y = line.y_top + line.height * 0.5;
                            caret = text_system
                                .hit_test_caret(handle, Vec2::new(line.logical_x, line_mid_y));
                            caret_byte = text_system
                                .caret_insertion_byte(handle, caret)
                                .min(state.value.len());
                            caret_needs_layout_sync = false;
                        } else {
                            // Line-aware: scan left for the preceding '\n' (or
                            // the start of the string), then land just after it.
                            let line_start =
                                state.value[..caret_byte].rfind('\n').map_or(0, |nl| nl + 1);
                            caret_byte = line_start;
                            caret_needs_layout_sync = true;
                        }
                    }
                    TextEvent::CaretEnd { shift, ctrl } => {
                        if *shift && selection_byte.is_none() {
                            selection_byte = Some(caret_byte);
                        } else if !*shift {
                            selection_byte = None;
                        }
                        if *ctrl {
                            caret_byte = state.value.len();
                            caret_needs_layout_sync = true;
                        } else if spec.wrap {
                            let text_content = state.value.as_str();
                            let (_, layout_width, layout_height, _) =
                                edit_layout_size(text_content, &spec, text_style, text_system);
                            let layout = text_system.prepare(
                                text_content,
                                text_style,
                                Rect::new(0.0, 0.0, layout_width, layout_height),
                            );
                            let handle = layout.handle;
                            let metrics = layout.metrics;
                            let visual_caret = if caret_needs_layout_sync {
                                text_system.caret_position_at_insertion_byte(handle, caret_byte)
                            } else {
                                caret
                            };
                            let caret_geom = text_system.caret_geom(handle, visual_caret);
                            let caret_mid_y = caret_geom.y_top + caret_geom.height * 0.5;
                            let current_line_idx = visual_line_index_at_y(&metrics, caret_mid_y);
                            let line = &metrics.lines[current_line_idx];
                            let line_mid_y = line.y_top + line.height * 0.5;
                            let end_cluster = text_system.hit_test_cluster(
                                handle,
                                Vec2::new(line.logical_x + line.logical_width + 1.0, line_mid_y),
                            );
                            caret = if matches!(
                                line.end_kind,
                                LineEndKind::HardNewline | LineEndKind::SoftWrapWhitespace
                            ) {
                                CaretPosition::BeforeCluster {
                                    cluster_byte_index: end_cluster,
                                }
                            } else {
                                let line_end_caret = CaretPosition::AfterCluster {
                                    cluster_byte_index: end_cluster,
                                };
                                let line_end_geom = text_system.caret_geom(handle, line_end_caret);
                                let line_end_idx = visual_line_index_at_y(
                                    &metrics,
                                    line_end_geom.y_top + line_end_geom.height * 0.5,
                                );
                                if line_end_idx == current_line_idx {
                                    line_end_caret
                                } else {
                                    CaretPosition::BeforeCluster {
                                        cluster_byte_index: end_cluster,
                                    }
                                }
                            };
                            caret_byte = text_system
                                .caret_insertion_byte(handle, caret)
                                .min(state.value.len());
                            caret_needs_layout_sync = false;
                        } else {
                            // Line-aware: scan right for the next '\n' and land
                            // just before it (or at the end of the string).
                            let line_end = state.value[caret_byte..]
                                .find('\n')
                                .map_or(state.value.len(), |nl| caret_byte + nl);
                            caret_byte = line_end;
                            caret_needs_layout_sync = true;
                        }
                    }
                    TextEvent::SelectAll => {
                        selection_byte = Some(0);
                        caret_byte = state.value.len();
                        caret_needs_layout_sync = true;
                        selection_only_action = true;
                    }
                    TextEvent::Copy => {
                        if let Some(sel) = selection_byte {
                            let start = caret_byte.min(sel);
                            let end = caret_byte.max(sel);
                            if start < end {
                                clipboard_action = Some(ClipboardAction::Copy(
                                    state.value[start..end].to_string(),
                                ));
                            }
                        }
                    }
                    TextEvent::Cut => {
                        if let Some(sel) = selection_byte {
                            let start = caret_byte.min(sel);
                            let end = caret_byte.max(sel);
                            if start < end {
                                clipboard_action =
                                    Some(ClipboardAction::Cut(state.value[start..end].to_string()));
                                remove_selection(
                                    &mut state.value,
                                    &mut caret_byte,
                                    &mut selection_byte,
                                );
                                caret_needs_layout_sync = true;
                            }
                        }
                    }
                    TextEvent::Paste(text) => {
                        let processed = spec.newline_policy.process(text);
                        remove_selection(&mut state.value, &mut caret_byte, &mut selection_byte);
                        state.value.insert_str(caret_byte, &processed);
                        caret_byte += processed.len();
                        caret_needs_layout_sync = true;
                    }
                }
            }

            if input.key_pressed_enter
                && !newline_inserted
                && spec.newline_policy == NewlinePolicy::Allow
            {
                remove_selection(&mut state.value, &mut caret_byte, &mut selection_byte);
                state.value.insert(caret_byte, '\n');
                caret_byte += 1;
                caret_needs_layout_sync = true;
            }

            if input.key_pressed_page_up || input.key_pressed_page_down {
                let direction = if input.key_pressed_page_down {
                    VerticalCaretDirection::Down
                } else {
                    VerticalCaretDirection::Up
                };
                let sel_byte = selection_byte;
                let has_selection = sel_byte.is_some() && sel_byte != Some(caret_byte);
                let shift = input.modifier_shift;

                if shift {
                    if selection_byte.is_none() {
                        selection_byte = Some(caret_byte);
                    }
                } else {
                    selection_byte = None;
                }

                let start_byte = if has_selection && !shift {
                    match direction {
                        VerticalCaretDirection::Up => caret_byte.min(sel_byte.unwrap()),
                        VerticalCaretDirection::Down => caret_byte.max(sel_byte.unwrap()),
                    }
                } else {
                    caret_byte
                };

                let (_, _, _, scroll_outer_rect) =
                    edit_layout_size(state.value.as_str(), &spec, text_style, text_system);
                let line_count = page_line_count(
                    state.value.as_str(),
                    &spec,
                    text_style,
                    text_system,
                    start_byte,
                    scroll_outer_rect.h,
                );
                let moved = move_caret_vertical(
                    state.value.as_str(),
                    &spec,
                    text_style,
                    text_system,
                    caret,
                    start_byte,
                    !caret_needs_layout_sync && start_byte == caret_byte,
                    start_byte,
                    direction,
                    line_count,
                );
                caret = moved.caret;
                caret_byte = moved.byte;
                caret_needs_layout_sync = moved.needs_layout_sync;
            }
        }

        // Safety checks
        if caret_byte > state.value.len() {
            caret_byte = state.value.len();
            caret_needs_layout_sync = true;
        }
        if !state.value.is_char_boundary(caret_byte) {
            caret_byte = 0; // fallback
            caret_needs_layout_sync = true;
        }
        if let Some(sel) = selection_byte {
            if sel > state.value.len() {
                selection_byte = Some(state.value.len());
            }
            if let Some(sel) = selection_byte {
                if !state.value.is_char_boundary(sel) {
                    selection_byte = None;
                }
            }
        }

        let text_content = state.value.as_str();
        let (metrics, layout_width, layout_height, scroll_outer_rect) =
            edit_layout_size(text_content, &spec, text_style, text_system);

        // Drawing Background
        let bg_color = if spec.error {
            spec.style.error_background
        } else if contains {
            spec.style.background_hovered
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
        let border_width = if focused && !spec.error {
            spec.style.focus_width
        } else {
            spec.style.border_width
        };
        if border_width > 0.0 {
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
                width: border_width,
                z: spec.layer.get_z(),
            });
        }

        // Prepare text after content bounds are known so hit testing and caret
        // geometry use the same logical text block that will be drawn.
        let inner_scroll_size = Vec2::new(
            metrics.logical_size.x + 2.0 * spec.style.padding_x,
            metrics.logical_size.y + 2.0 * spec.style.padding_y,
        ); // Include padding on either side of text.

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

        let text_x = scroll_outer_rect.x + spec.style.padding_x - scroll_result.offset.x;
        let text_y = if metrics.logical_size.y + 2.0 * spec.style.padding_y <= scroll_outer_rect.h {
            match spec.vertical_align {
                Align::Start => scroll_outer_rect.y + spec.style.padding_y,
                Align::Center => {
                    scroll_outer_rect.y + (scroll_outer_rect.h - metrics.logical_size.y) / 2.0
                }
                Align::End => {
                    scroll_outer_rect.y + scroll_outer_rect.h
                        - spec.style.padding_y
                        - metrics.logical_size.y
                }
            }
        } else {
            scroll_outer_rect.y + spec.style.padding_y - scroll_result.offset.y
        };
        let text_rect = Rect::new(text_x, text_y, layout_width, layout_height);
        let layout = text_system.prepare(text_content, text_style, text_rect);
        let handle = layout.handle;

        // Mouse interaction
        if contains && input.mouse_pressed {
            if !focused {
                state.suppress_select_all_on_next_focus = true;
            }
            focus_system.take_keyboard_focus(state.focus_id);

            let relative_pos = Vec2::new(
                input.mouse_pos.x - text_rect.x,
                input.mouse_pos.y - text_rect.y,
            );
            let clicked_caret = text_system.hit_test_caret(handle, relative_pos);
            let clicked_byte = text_system.caret_insertion_byte(handle, clicked_caret);
            let clicked_byte = clicked_byte.min(state.value.len());

            // Handling repeated clicks
            if input.mouse_click_count == 2 {
                let cluster_byte = text_system.hit_test_cluster(handle, relative_pos);
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
                    input.mouse_pos.x - text_rect.x,
                    input.mouse_pos.y - text_rect.y,
                );
                let current_caret = text_system.hit_test_caret(handle, relative_pos);
                let current_byte = text_system.caret_insertion_byte(handle, current_caret);
                let current_byte = current_byte.min(state.value.len());

                if let Some((orig_start, orig_end)) = state.drag_word_origin {
                    let cluster_byte = text_system.hit_test_cluster(handle, relative_pos);
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
            caret = text_system.caret_position_at_insertion_byte(handle, caret_byte);
        }
        state.caret = caret;
        state.selection_anchor = selection_byte
            .map(|selection| text_system.caret_position_at_insertion_byte(handle, selection));

        caret_byte = text_system
            .caret_insertion_byte(handle, state.caret)
            .min(state.value.len());
        selection_byte = state.selection_anchor.map(|selection| {
            text_system
                .caret_insertion_byte(handle, selection)
                .min(state.value.len())
        });

        if just_focused || state.caret != old_caret || state.selection_anchor != old_selection {
            state.last_caret_move_time = spec.time;

            let padding = 16.0_f32;
            let max_scroll_x = (inner_scroll_size.x - scroll_outer_rect.w).max(0.0);

            // Determine the horizontal span of the target we want to keep in view.
            // If this is a bulk selection action with a non-empty selection, we target
            // the full selection span. Otherwise, we target the zero-width caret position.
            let (sel_min_x, sel_max_x) = match (selection_only_action, selection_byte) {
                (true, Some(sel)) if sel != caret_byte => {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);
                    let start_caret = text_system.caret_geom(
                        handle,
                        text_system.caret_position_at_insertion_byte(handle, start),
                    );
                    let end_caret = text_system.caret_geom(
                        handle,
                        text_system.caret_position_at_insertion_byte(handle, end),
                    );
                    (
                        start_caret.x.min(end_caret.x),
                        start_caret.x.max(end_caret.x),
                    )
                }
                _ => {
                    let caret = text_system.caret_geom(handle, state.caret);
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

            let max_scroll_y = (inner_scroll_size.y - scroll_outer_rect.h).max(0.0);

            // Determine the vertical span of the target we want to keep in view.
            let (sel_min_y, sel_max_y) = match (selection_only_action, selection_byte) {
                (true, Some(sel)) if sel != caret_byte => {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);
                    let start_caret = text_system.caret_geom(
                        handle,
                        text_system.caret_position_at_insertion_byte(handle, start),
                    );
                    let end_caret = text_system.caret_geom(
                        handle,
                        text_system.caret_position_at_insertion_byte(handle, end),
                    );
                    (
                        start_caret.y_top.min(end_caret.y_top),
                        (start_caret.y_top + start_caret.height)
                            .max(end_caret.y_top + end_caret.height),
                    )
                }
                _ => {
                    let caret = text_system.caret_geom(handle, state.caret);
                    (caret.y_top, caret.y_top + caret.height)
                }
            };

            let target_top = sel_min_y - padding;
            let target_bottom = sel_max_y - scroll_outer_rect.h + padding;

            let (s_min_y, s_max_y) = if target_bottom <= target_top {
                (target_bottom, target_top)
            } else {
                (target_top, target_bottom)
            };

            let target_scroll_y = state.scroll.offset.y.clamp(s_min_y, s_max_y);
            state.scroll.offset.y = target_scroll_y.clamp(0.0, max_scroll_y);
        }

        // Selection
        if focused {
            if let Some(sel) = selection_byte {
                if sel != caret_byte {
                    let start = sel.min(caret_byte);
                    let end = sel.max(caret_byte);

                    for line in &layout.metrics.lines {
                        let line_sel_start = start.max(line.byte_start);
                        let line_sel_end = end.min(line.byte_end);

                        if line_sel_start < line_sel_end {
                            let line_start_x = line.logical_x;
                            let start_caret = text_system.caret_geom(
                                handle,
                                text_system
                                    .caret_position_at_insertion_byte(handle, line_sel_start),
                            );

                            let end_x = if line_sel_end == line.byte_end {
                                line_start_x
                                    + line.logical_width
                                    + selected_line_end_affordance_width(line)
                            } else {
                                text_system
                                    .caret_geom(
                                        handle,
                                        text_system
                                            .caret_position_at_insertion_byte(handle, line_sel_end),
                                    )
                                    .x
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
        } else if !focused {
            if let Some(placeholder) = spec.placeholder.as_deref() {
                let placeholder_layout = text_system.prepare(placeholder, text_style, text_rect);
                cmds.push(DrawCmd::Text {
                    rect: text_rect,
                    color: spec.style.placeholder_color,
                    handle: placeholder_layout.handle,
                    z: spec.layer.get_z(),
                });
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
                let caret = text_system.caret_geom(handle, state.caret);
                let caret_rect = Rect::new(
                    text_rect.x + caret.x,
                    text_rect.y + caret.y_top,
                    spec.style.caret_width,
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

    pub(super) fn edit_layout_size<T: TextSystem>(
        text_content: &str,
        spec: &TextEditSpec,
        text_style: TextStyle,
        text_system: &mut T,
    ) -> (TextMetrics, f32, f32, Rect) {
        let mut scroll_outer_rect = spec.rect.inset(spec.style.border_width);
        if spec.error {
            scroll_outer_rect.x += spec.style.error_stripe_width;
            scroll_outer_rect.w -= spec.style.error_stripe_width;
        }

        let max_width = if spec.wrap {
            Some((scroll_outer_rect.w - 2.0 * spec.style.padding_x).max(0.0))
        } else {
            None
        };

        let mut metrics = text_system.measure(
            text_content,
            text_style,
            TextBounds {
                max_width,
                max_height: None,
            },
        );

        let mut final_max_width = max_width;

        if spec.wrap {
            // If the wrapped height exceeds the viewport height, a vertical scrollbar
            // will be shown. The vertical scrollbar steals width (5.0px in our scroll spec),
            // so we re-measure the text with a narrower width.
            let content_h = metrics.logical_size.y + 2.0 * spec.style.padding_y;
            if content_h > scroll_outer_rect.h {
                let max_width_narrow =
                    Some(((scroll_outer_rect.w - 2.0 * spec.style.padding_x) - 5.0).max(0.0));
                final_max_width = max_width_narrow;
                metrics = text_system.measure(
                    text_content,
                    text_style,
                    TextBounds {
                        max_width: max_width_narrow,
                        max_height: None,
                    },
                );
            }
        }

        let layout_width = if spec.wrap {
            final_max_width.unwrap_or(0.0)
        } else {
            metrics.logical_size.x.max(scroll_outer_rect.w)
        };
        let layout_height = metrics.logical_size.y.max(scroll_outer_rect.h);
        (metrics, layout_width, layout_height, scroll_outer_rect)
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextEditStyle {
    pub background: Color,
    pub background_hovered: Color,
    pub error_background: Color,
    pub border: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub error_border: Color,
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
    pub disabled_alpha: f32,
}

impl TextEditStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            background_hovered: Color::WHITE,
            error_background: theme.rust_soft,
            border: theme.ink,
            focus: theme.rust,
            error_border: theme.rust,
            error_stripe_width: 4.0,
            border_width: theme.border,
            focus_width: theme.focus_width,
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
            select_color: theme.rust_soft,
            disabled_alpha: 0.55,
        }
    }
}

pub(crate) fn to_text_style(
    style: TextEditStyle,
    wrap: bool,
    line_align: TextLineAlign,
) -> TextStyle {
    let mut flow = if wrap {
        TextFlow::wrapped()
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

fn caret_position_at_text_end(text: &str) -> CaretPosition {
    text.char_indices()
        .next_back()
        .map(|(cluster_byte_index, _)| CaretPosition::AfterCluster { cluster_byte_index })
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
        cluster_byte_index: byte_index,
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            let mut style = TextEditStyle::from_theme(theme);
            let multiline = self.newline_policy.unwrap_or_default() == NewlinePolicy::Allow
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
    let calc_spec = raw::TextEditCalcIntrinsicSizeSpec {
        style: spec.style,
        wrap: spec.wrap,
        line_align: spec.line_align,
    };
    let intrinsic = raw::calc_text_edit_intrinsic_size(&calc_spec, state, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
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

    use crate::{
        test_utils::DummyTextSys,
        text::{CaretGeom, LineEndKind, LineMetrics, TextHandle, TextLayout},
    };

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = TextEditSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        assert_eq!(
            builder.style.unwrap().font,
            TextEditStyle::from_theme(&theme).font
        );
        assert_eq!(
            builder.style.unwrap().size,
            TextEditStyle::from_theme(&theme).size
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_uses_single_line_vertical_padding() {
        let theme = crate::theme::Theme::framewise();
        let builder = TextEditSpecBuilder::new().defaults_from_theme(&theme);

        assert_eq!(builder.style.unwrap().padding_y, 0.0);
    }

    #[test]
    fn test_builder_defaults_from_theme_uses_multiline_vertical_padding() {
        let theme = crate::theme::Theme::framewise();

        let allow_newlines = TextEditSpecBuilder::new()
            .newline_policy(NewlinePolicy::Allow)
            .defaults_from_theme(&theme);
        assert_eq!(allow_newlines.style.unwrap().padding_y, 8.0);

        let wrapped = TextEditSpecBuilder::new()
            .wrap(true)
            .defaults_from_theme(&theme);
        assert_eq!(wrapped.style.unwrap().padding_y, 8.0);
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
        custom_style.size = 99.0;
        let builder = TextEditSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().size, 99.0);
    }

    fn spec() -> TextEditSpec {
        let mut style = TextEditStyle::from_theme(&crate::theme::Theme::framewise());
        style.padding_x = 4.0;
        style.padding_y = 4.0;

        TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 30.0),
            style,
            placeholder: None,
            clip_rect: None,
            error: false,
            disabled: false,
            time: 0.0,
            layer: Layer::default(),
            newline_policy: NewlinePolicy::ReplaceWithSpace,
            wrap: false,
            vertical_align: Align::Center,
            line_align: TextLineAlign::Start,
        }
    }

    fn caret_byte(state: &TextEditState) -> usize {
        insertion_byte_for_position(&state.value, state.caret)
    }

    fn selection_byte(state: &TextEditState) -> Option<usize> {
        state
            .selection_anchor
            .map(|position| insertion_byte_for_position(&state.value, position))
    }

    fn set_caret_byte(state: &mut TextEditState, byte: usize) {
        state.caret = caret_position_at_byte(&state.value, byte);
    }

    fn set_selection_byte(state: &mut TextEditState, byte: Option<usize>) {
        state.selection_anchor = byte.map(|byte| caret_position_at_byte(&state.value, byte));
    }

    fn insertion_byte_for_position(text: &str, position: CaretPosition) -> usize {
        match position {
            CaretPosition::EmptyText => 0,
            CaretPosition::BeforeCluster { cluster_byte_index } => {
                cluster_byte_index.min(text.len())
            }
            CaretPosition::AfterCluster { cluster_byte_index } => text
                .get(cluster_byte_index..)
                .and_then(|tail| tail.chars().next())
                .map_or(cluster_byte_index, |ch| cluster_byte_index + ch.len_utf8())
                .min(text.len()),
        }
    }

    struct VisualBoundaryTextSys;

    impl VisualBoundaryTextSys {
        fn metrics(text: &str) -> TextMetrics {
            TextMetrics {
                logical_size: Vec2::new(8.0, 32.0),
                ink_bounds: Rect::new(0.0, 0.0, 8.0, 32.0),
                line_count: 2,
                truncated_horizontal: false,
                truncated_vertical: false,
                lines: vec![
                    LineMetrics {
                        y_top: 0.0,
                        height: 16.0,
                        logical_width: 8.0,
                        ink_width: 8.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 0,
                        byte_end: 1.min(text.len()),
                        end_kind: LineEndKind::SoftWrapNonWhitespace,
                    },
                    LineMetrics {
                        y_top: 16.0,
                        height: 16.0,
                        logical_width: 8.0,
                        ink_width: 8.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 1.min(text.len()),
                        byte_end: text.len(),
                        end_kind: LineEndKind::EndOfText,
                    },
                ],
            }
        }
    }

    impl TextSystem for VisualBoundaryTextSys {
        fn measure(&mut self, text: &str, _style: TextStyle, _bounds: TextBounds) -> TextMetrics {
            Self::metrics(text)
        }

        fn prepare(&mut self, text: &str, _style: TextStyle, _rect: Rect) -> TextLayout {
            TextLayout {
                handle: TextHandle(0),
                metrics: Self::metrics(text),
            }
        }

        fn caret_geom(&self, _handle: TextHandle, position: CaretPosition) -> CaretGeom {
            match position {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                } => CaretGeom {
                    x: 0.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 0,
                } => CaretGeom {
                    x: 8.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                } => CaretGeom {
                    x: 0.0,
                    y_top: 16.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                } => CaretGeom {
                    x: 8.0,
                    y_top: 16.0,
                    height: 16.0,
                },
                CaretPosition::BeforeCluster { cluster_byte_index } => CaretGeom {
                    x: cluster_byte_index as f32 * 8.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster { cluster_byte_index } => CaretGeom {
                    x: (cluster_byte_index + 1) as f32 * 8.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::EmptyText => CaretGeom {
                    x: 0.0,
                    y_top: 0.0,
                    height: 16.0,
                },
            }
        }

        fn hit_test_caret(&self, _handle: TextHandle, pos: Vec2) -> CaretPosition {
            if pos.y >= 16.0 {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                }
            } else {
                CaretPosition::AfterCluster {
                    cluster_byte_index: 0,
                }
            }
        }

        fn caret_insertion_byte(&self, _handle: TextHandle, position: CaretPosition) -> usize {
            match position {
                CaretPosition::BeforeCluster { cluster_byte_index } => cluster_byte_index,
                CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index + 1,
                CaretPosition::EmptyText => 0,
            }
        }

        fn caret_position_at_insertion_byte(
            &self,
            _handle: TextHandle,
            byte_index: usize,
        ) -> CaretPosition {
            match byte_index {
                0 => CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                },
                1 => CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                },
                _ => CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                },
            }
        }

        fn previous_caret_position(
            &self,
            _handle: TextHandle,
            position: CaretPosition,
        ) -> CaretPosition {
            match position {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                } => CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                },
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                }
                | CaretPosition::AfterCluster {
                    cluster_byte_index: 0,
                } => CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                } => CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                },
                CaretPosition::BeforeCluster { cluster_byte_index }
                | CaretPosition::AfterCluster { cluster_byte_index } => {
                    CaretPosition::BeforeCluster {
                        cluster_byte_index: cluster_byte_index.saturating_sub(1),
                    }
                }
                CaretPosition::EmptyText => CaretPosition::EmptyText,
            }
        }

        fn next_caret_position(
            &self,
            _handle: TextHandle,
            position: CaretPosition,
        ) -> CaretPosition {
            match position {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                } => CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                },
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                }
                | CaretPosition::AfterCluster {
                    cluster_byte_index: 0,
                } => CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                } => CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                },
                CaretPosition::BeforeCluster { cluster_byte_index }
                | CaretPosition::AfterCluster { cluster_byte_index } => {
                    CaretPosition::BeforeCluster {
                        cluster_byte_index: cluster_byte_index + 1,
                    }
                }
                CaretPosition::EmptyText => CaretPosition::EmptyText,
            }
        }

        fn hit_test_cluster(&self, _handle: TextHandle, pos: Vec2) -> usize {
            usize::from(pos.y >= 16.0)
        }
    }

    struct CollapsedTrailingSpaceTextSys;

    impl CollapsedTrailingSpaceTextSys {
        fn metrics() -> TextMetrics {
            TextMetrics {
                logical_size: Vec2::new(8.0, 32.0),
                ink_bounds: Rect::new(0.0, 0.0, 8.0, 32.0),
                line_count: 2,
                truncated_horizontal: false,
                truncated_vertical: false,
                lines: vec![
                    LineMetrics {
                        y_top: 0.0,
                        height: 16.0,
                        logical_width: 8.0,
                        ink_width: 8.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 0,
                        byte_end: 2,
                        end_kind: LineEndKind::SoftWrapWhitespace,
                    },
                    LineMetrics {
                        y_top: 16.0,
                        height: 16.0,
                        logical_width: 8.0,
                        ink_width: 8.0,
                        logical_x: 0.0,
                        ink_x: 0.0,
                        byte_start: 2,
                        byte_end: 3,
                        end_kind: LineEndKind::EndOfText,
                    },
                ],
            }
        }
    }

    impl TextSystem for CollapsedTrailingSpaceTextSys {
        fn measure(&mut self, _text: &str, _style: TextStyle, _bounds: TextBounds) -> TextMetrics {
            Self::metrics()
        }

        fn prepare(&mut self, _text: &str, _style: TextStyle, _rect: Rect) -> TextLayout {
            TextLayout {
                handle: TextHandle(0),
                metrics: Self::metrics(),
            }
        }

        fn caret_geom(&self, _handle: TextHandle, position: CaretPosition) -> CaretGeom {
            match position {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 0,
                } => CaretGeom {
                    x: 0.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                } => CaretGeom {
                    x: 8.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                }
                | CaretPosition::BeforeCluster {
                    cluster_byte_index: 2,
                } => CaretGeom {
                    x: 0.0,
                    y_top: 16.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 2,
                } => CaretGeom {
                    x: 8.0,
                    y_top: 16.0,
                    height: 16.0,
                },
                CaretPosition::AfterCluster {
                    cluster_byte_index: 0,
                } => CaretGeom {
                    x: 8.0,
                    y_top: 0.0,
                    height: 16.0,
                },
                CaretPosition::BeforeCluster { cluster_byte_index }
                | CaretPosition::AfterCluster { cluster_byte_index } => CaretGeom {
                    x: cluster_byte_index as f32 * 8.0,
                    y_top: 16.0,
                    height: 16.0,
                },
                CaretPosition::EmptyText => CaretGeom {
                    x: 0.0,
                    y_top: 0.0,
                    height: 16.0,
                },
            }
        }

        fn hit_test_caret(&self, _handle: TextHandle, pos: Vec2) -> CaretPosition {
            if pos.y >= 16.0 {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 2,
                }
            } else {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: 1,
                }
            }
        }

        fn caret_insertion_byte(&self, _handle: TextHandle, position: CaretPosition) -> usize {
            match position {
                CaretPosition::BeforeCluster { cluster_byte_index } => cluster_byte_index,
                CaretPosition::AfterCluster { cluster_byte_index } => cluster_byte_index + 1,
                CaretPosition::EmptyText => 0,
            }
        }

        fn caret_position_at_insertion_byte(
            &self,
            _handle: TextHandle,
            byte_index: usize,
        ) -> CaretPosition {
            if byte_index >= 3 {
                CaretPosition::AfterCluster {
                    cluster_byte_index: 2,
                }
            } else {
                CaretPosition::BeforeCluster {
                    cluster_byte_index: byte_index,
                }
            }
        }

        fn hit_test_cluster(&self, _handle: TextHandle, pos: Vec2) -> usize {
            if pos.y >= 16.0 {
                2
            } else {
                1
            }
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
        assert_eq!(caret_byte(&state), 3);

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
        assert_eq!(caret_byte(&state), 2);

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
        assert_eq!(caret_byte(&state), 3);
    }

    #[test]
    fn test_backspace_and_delete() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 3);
        state.had_keyboard_focus = true;
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
        assert_eq!(caret_byte(&state), 2);

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
        assert_eq!(caret_byte(&state), 2);
    }

    #[test]
    fn test_ctrl_backspace_and_delete() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");
        set_caret_byte(&mut state, 8); // "hello wo|rld"
        state.had_keyboard_focus = true;
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
        assert_eq!(caret_byte(&state), 6); // end of "hello "

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
        assert_eq!(caret_byte(&state), 6);
    }

    #[test]
    fn test_selection_and_replacement() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 1);
        state.had_keyboard_focus = true;
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
        assert_eq!(selection_byte(&state), Some(1));
        assert_eq!(caret_byte(&state), 3);

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
        assert_eq!(caret_byte(&state), 2);
        assert_eq!(selection_byte(&state), None);
    }

    #[test]
    fn test_text_edit_left_right_skip_same_byte_visual_side() {
        let mut text_system = VisualBoundaryTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("ab");
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        state.caret = CaretPosition::BeforeCluster {
            cluster_byte_index: 0,
        };
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretRight {
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
        assert_eq!(
            state.caret,
            CaretPosition::BeforeCluster {
                cluster_byte_index: 1
            }
        );
        assert_eq!(caret_byte(&state), 1);

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
        assert_eq!(
            state.caret,
            CaretPosition::BeforeCluster {
                cluster_byte_index: 0
            }
        );
        assert_eq!(caret_byte(&state), 0);
    }

    #[test]
    fn test_mouse_release_preserves_visual_side_at_shared_insertion() {
        let mut text_system = VisualBoundaryTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("ab");
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(100.0, 8.0);

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

        assert_eq!(
            state.caret,
            CaretPosition::AfterCluster {
                cluster_byte_index: 0
            }
        );
        assert_eq!(caret_byte(&state), 1);

        input.mouse_down = false;
        input.mouse_pressed = false;
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
            state.caret,
            CaretPosition::AfterCluster {
                cluster_byte_index: 0
            }
        );
        assert_eq!(caret_byte(&state), 1);
    }

    #[test]
    fn test_empty_mouse_click_keeps_empty_caret() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::default();
        let mut input = Input {
            mouse_pos: Vec2::new(120.0, 15.0),
            ..Input::default()
        };

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

        assert_eq!(state.caret, CaretPosition::EmptyText);
        assert_eq!(caret_byte(&state), 0);
    }

    #[test]
    fn test_mouse_clicking_and_dragging() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding_x + spec().style.border_width,
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
        assert_eq!(caret_byte(&state), 5);
        assert!(state.is_dragging);
        state.had_keyboard_focus = true;

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
        assert_eq!(selection_byte(&state), Some(5));
        assert_eq!(caret_byte(&state), 8);

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
        assert_eq!(selection_byte(&state), Some(5));
        assert_eq!(caret_byte(&state), 8);
    }

    #[test]
    fn test_double_click_selection_and_drag() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello rust world");

        let mut input = Input::default();
        // Click on "rust" (byte index 8 -> pixel 64)
        input.mouse_pos = crate::types::Vec2::new(
            64.0 + spec().style.padding_x + spec().style.border_width,
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
        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 10);
        assert!(state.is_dragging);
        assert_eq!(state.drag_word_origin, Some((6, 10)));

        // Frame 3: Drag right to "world" (byte index 14 -> pixel 112)
        input.mouse_pressed = false;
        input.mouse_pos.x = 112.0 + spec().style.padding_x + spec().style.border_width;
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
        assert_eq!(selection_byte(&state), Some(6)); // original start
        assert_eq!(caret_byte(&state), 16); // end of "world"

        // Frame 4: Drag left to "hello" (byte index 2 -> pixel 16)
        input.mouse_pos.x = 16.0 + spec().style.padding_x + spec().style.border_width;
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
        assert_eq!(selection_byte(&state), Some(10)); // original end
        assert_eq!(caret_byte(&state), 0); // start of "hello"
    }

    #[test]
    fn test_double_click_symmetry() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        let mut run_double_click = |x_within_text: f32| -> (Option<usize>, usize) {
            let mut state = TextEditState::new("a b");
            let mut input = Input::default();
            let x_offset = spec().style.padding_x + spec().style.border_width;
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

            (selection_byte(&state), caret_byte(&state))
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
            let x_offset = spec().style.padding_x + spec().style.border_width;
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

            (selection_byte(&state), caret_byte(&state))
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
    fn test_triple_click_selects_logical_line() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("alpha\nbravo\ncharlie");
        let mut input = Input::default();
        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 80.0),
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };

        input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
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

        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 3;
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

        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 12);
        assert!(state.is_dragging);
        assert_eq!(state.drag_line_origin, Some((6, 12)));
    }

    #[test]
    fn test_triple_click_selection_and_drag() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("alpha\nbravo\ncharlie\ndelta");
        let mut input = Input::default();
        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 90.0),
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };

        input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
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

        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 3;
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

        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 12);
        assert_eq!(state.drag_line_origin, Some((6, 12)));

        input.mouse_pressed = false;
        input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 32.0 + 8.0);
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

        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 20);

        input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 8.0);
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

        assert_eq!(selection_byte(&state), Some(12));
        assert_eq!(caret_byte(&state), 0);
    }

    #[test]
    fn test_triple_click_selects_wrapped_logical_line() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst\nzz");
        let mut input = Input::default();
        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 90.0, 80.0),
            newline_policy: NewlinePolicy::Allow,
            wrap: true,
            vertical_align: Align::Start,
            ..spec()
        };

        input.mouse_pos = Vec2::new(5.0 + 16.0, 5.0 + 16.0 + 8.0);
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

        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 3;
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

        assert_eq!(selection_byte(&state), Some(0));
        assert_eq!(caret_byte(&state), 21);
    }

    #[test]
    fn test_quadruple_click_selects_all() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("alpha\nbravo\ncharlie");
        let mut input = Input::default();
        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 80.0),
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };

        input.mouse_pos = Vec2::new(5.0 + 8.0, 5.0 + 16.0 + 8.0);
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

        input.mouse_down = true;
        input.mouse_pressed = true;
        input.mouse_click_count = 4;
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

        assert_eq!(selection_byte(&state), Some(0));
        assert_eq!(caret_byte(&state), state.value.len());
    }

    #[test]
    fn test_caret_blink_reset_on_move() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 5);
        state.had_keyboard_focus = true;

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
    fn test_caret_blink_reset_on_focus_even_without_caret_move() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 5);
        state.selection_anchor = Some(caret_position_at_byte(&state.value, 0));

        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &Input::default(),
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
            "Caret should be visible immediately after gaining focus"
        );
    }

    #[test]
    fn test_caret_blink_reset_on_mouse_focus_even_without_caret_move() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        set_caret_byte(&mut state, 5);

        let mut input = Input {
            mouse_pos: crate::types::Vec2::new(
                40.0 + spec().style.padding_x + spec().style.border_width,
                15.0,
            ),
            ..Input::default()
        };

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

        input.mouse_down = true;
        input.mouse_pressed = true;
        focus_system.begin_frame();
        raw::text_edit(
            TextEditSpec {
                time: 0.6,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        input.mouse_down = false;
        input.mouse_pressed = false;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
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
            "Caret should be visible immediately after gaining focus from a mouse click"
        );
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
    fn test_logical_line_bounds() {
        let text = "alpha\nbravo\ncharlie";
        assert_eq!(logical_line_bounds(text, 0), (0, 6));
        assert_eq!(logical_line_bounds(text, 8), (6, 12));
        assert_eq!(logical_line_bounds(text, text.len()), (12, text.len()));
        assert_eq!(logical_line_bounds("alpha\n", 6), (6, 6));
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
        assert!(state.had_keyboard_focus);
        assert_eq!(selection_byte(&state), Some(0));
        assert_eq!(caret_byte(&state), 11);
    }

    #[test]
    fn test_mouse_focus_no_select_all() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello world");

        let mut input = Input::default();
        input.mouse_pos = crate::types::Vec2::new(
            40.0 + spec().style.padding_x + spec().style.border_width,
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

        assert!(state.had_keyboard_focus);
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 5);
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

        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
        state.had_keyboard_focus = true;

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
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 6);

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
        assert_eq!(caret_byte(&state), 10);
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
    fn test_text_edit_visual_hover_background() {
        let mut text_system = DummyTextSys;
        let mut state = TextEditState::new("hello");
        let input = Input {
            mouse_pos: Vec2::new(100.0, 15.0),
            ..Input::default()
        };
        let mut focus_system = FocusSystem::new_mocked(None, Some(state.focus_id));
        let mut cmds = DrawCommands::new();

        raw::text_edit(
            spec(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert!(matches!(
            cmds.iter().next(),
            Some(DrawCmd::FillRect { color, .. }) if *color == spec().style.background_hovered
        ));
    }

    #[test]
    fn test_text_edit_visual_placeholder() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::default();
        let mut cmds = DrawCommands::new();

        raw::text_edit(
            TextEditSpec {
                placeholder: Some("frame_buffer".to_string()),
                ..spec()
            },
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert!(cmds.iter().any(|cmd| matches!(
            cmd,
            DrawCmd::Text { color, .. } if *color == spec().style.placeholder_color
        )));

        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();
        state.had_keyboard_focus = true;
        let mut focused_cmds = DrawCommands::new();

        raw::text_edit(
            TextEditSpec {
                placeholder: Some("frame_buffer".to_string()),
                ..spec()
            },
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
            &mut focused_cmds,
        );

        assert!(!focused_cmds.iter().any(|cmd| matches!(
            cmd,
            DrawCmd::Text { color, .. } if *color == spec().style.placeholder_color
        )));
    }

    #[test]
    fn test_text_edit_visual_focused_caret() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("hello");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.had_keyboard_focus = true; // ensure state knows

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
                    width: spec().style.focus_width,
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
                    rect: Rect::new(45.0, 7.0, spec().style.caret_width, 16.0),
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

        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(0));
        set_caret_byte(&mut state, 5);

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
                    width: spec().style.focus_width,
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
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(45.0, 7.0, spec().style.caret_width, 16.0),
                    color: spec().style.caret_color,
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_selection_highlight_respects_horizontal_line_alignment() {
        for (line_align, expected_x) in [(TextLineAlign::Center, 84.0), (TextLineAlign::End, 163.0)]
        {
            let mut text_system = DummyTextSys;
            let mut focus_system = FocusSystem::new();
            let mut state = TextEditState::new("hello");
            focus_system.take_keyboard_focus(state.focus_id);
            focus_system.end_frame();
            focus_system.begin_frame();

            state.had_keyboard_focus = true;
            set_selection_byte(&mut state, Some(0));
            set_caret_byte(&mut state, 5);

            let input = Input::default();
            let mut cmds = DrawCommands::new();
            raw::text_edit(
                TextEditSpec {
                    line_align,
                    ..spec()
                },
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );

            let has_aligned_selection = cmds.iter().any(|cmd| {
                matches!(
                    cmd,
                    DrawCmd::FillRect {
                        rect,
                        color,
                        ..
                    } if *color == spec().style.select_color
                        && *rect == Rect::new(expected_x, 7.0, 40.0, 16.0)
                )
            });
            assert!(
                has_aligned_selection,
                "{line_align:?} selection highlight should cover the horizontally aligned text"
            );
        }
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
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        // 1. Caret at start (0): scroll should be 0.0
        set_caret_byte(&mut state, 0);
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
        set_caret_byte(&mut state, 23);
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
        assert_eq!(caret_byte(&state), 24);
        assert_eq!(state.scroll.offset.x, 10.0);

        // 3. Caret moves from 34 to 35 (x = 280): exceeds right threshold
        // Expected scroll = 280 - 198 + 16 = 98.0, clamped to max_scroll (90.0)
        set_caret_byte(&mut state, 34);
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
        assert_eq!(caret_byte(&state), 35);
        assert_eq!(state.scroll.offset.x, 90.0);

        // 4. Move caret left from 3 to 2 (x = 16): below left threshold (98.0 + 16 = 114)
        // Expected scroll = 16 - 16 = 0.0
        set_caret_byte(&mut state, 3);
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
        assert_eq!(caret_byte(&state), 2);
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
        state.had_keyboard_focus = true;
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
            136.0 + spec().style.padding_x + spec().style.border_width - 120.0,
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
        assert_eq!(selection_byte(&state), Some(0));
        assert_eq!(caret_byte(&state), 23);
        assert_eq!(state.scroll.offset.x, 2.0);

        // Test case 3: Double-clicking the long word on the right should scroll the viewport right
        // just far enough to align the start of the word with the left edge of the viewport.
        state.scroll.offset.x = 120.0;
        input.text_events = vec![];
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        input.mouse_pos = Vec2::new(
            256.0 + spec().style.padding_x + spec().style.border_width - 120.0,
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
        assert_eq!(selection_byte(&state), Some(31));
        assert_eq!(caret_byte(&state), 55);
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
        state.had_keyboard_focus = true;

        // Manually inject a scroll offset of 50.0
        state.scroll.offset.x = 50.0;
        // Selection from index 0 to 5
        set_selection_byte(&mut state, Some(0));
        set_caret_byte(&mut state, 5);

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
                    assert_eq!(rect.w, 280.0);
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
        assert_eq!(caret_byte(&state), 11);
    }

    #[test]
    fn test_text_edit_vertical_scroll_coordinate_translation() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("line1\nline2\nline3\nline4");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();
        state.had_keyboard_focus = true;

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // ── Case 1: Scrolled vertically by 20.0 ────────────────────────────────────
        // Since text (64px) is taller than the viewport (28px), we expect top-alignment.
        // Expected text_y = outer_rect.y + padding - offset.y = 1.0 + 4.0 - 20.0 = -15.0
        state.scroll.offset.y = 20.0;
        set_selection_byte(&mut state, Some(0));
        set_caret_byte(&mut state, 5);

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        let mut found_text = false;
        let mut found_selection = false;
        for cmd in cmds.iter() {
            match cmd {
                DrawCmd::Text { rect, .. } => {
                    assert_eq!(rect.y, -15.0);
                    assert_eq!(rect.h, 64.0);
                    found_text = true;
                }
                DrawCmd::FillRect { rect, color, .. } => {
                    if *color == spec().style.select_color {
                        assert_eq!(rect.y, -15.0);
                        assert_eq!(rect.h, 16.0);
                        found_selection = true;
                    }
                }
                _ => {}
            }
        }
        assert!(found_text);
        assert!(found_selection);

        // ── Case 2: Not scrolled (offset = 0.0) ────────────────────────────────────
        // Since text (64px) is taller than the viewport (28px), we expect top-alignment.
        // Expected text_y = outer_rect.y + padding - offset.y = 1.0 + 4.0 - 0.0 = 5.0
        state.scroll.offset.y = 0.0;
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        let mut found_text = false;
        let mut found_selection = false;
        for cmd in cmds.iter() {
            match cmd {
                DrawCmd::Text { rect, .. } => {
                    assert_eq!(rect.y, 5.0);
                    assert_eq!(rect.h, 64.0);
                    found_text = true;
                }
                DrawCmd::FillRect { rect, color, .. } => {
                    if *color == spec().style.select_color {
                        assert_eq!(rect.y, 5.0);
                        assert_eq!(rect.h, 16.0);
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
    fn test_text_edit_vertical_click_with_scroll_offset() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("line1\nline2\nline3\nline4\nline5\nline6");

        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 50.0),
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // Manually inject a vertical scroll offset of 20.0
        state.scroll.offset.y = 20.0;

        let mut input = Input::default();
        // border = 1.0, padding = 4.0, offset.x = 0.0 => text_x = 5.0.
        // Clicking at x = 5.0, y = 38.0.
        // scroll_outer_rect.h = 48.0, metrics.logical_size.y = 96.0.
        // Since text is taller than the viewport, text_y = 1.0 + 4.0 - 20.0 = -15.0.
        // relative_pos.y = 38.0 - (-15.0) = 53.0, which lands on Line 3 ("line4\n", starts at 18)
        input.mouse_pos = Vec2::new(5.0, 38.0);

        // Frame 1: Warmup to establish hover claim
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

        // Frame 2: Mouse pressed
        focus_system.begin_frame();
        input.mouse_pressed = true;
        input.mouse_down = true;

        let mut cmds = DrawCommands::new();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Caret should have jumped to 18 (start of "line4\n")
        assert_eq!(caret_byte(&state), 18);
    }

    #[test]
    fn test_text_edit_vertical_caret_auto_scrolling() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        focus_system.begin_frame();

        // 10 lines of 16px: total height = 160px. Padding = 4px. Inner scroll height = 160 + 8 = 168px.
        // Viewport height = 60 - 2 = 58px.
        // Max scroll = 168 - 58 = 110px.
        let mut state = TextEditState::new("l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9");
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 60.0),
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // 1. Caret at start (Line 0, index 0): scroll should be 0.0
        set_caret_byte(&mut state, 0);
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.scroll.offset.y, 0.0);

        // 2. Caret moves down from Line 2 to Line 3 (index 9, y_top = 48.0, height = 16.0): exceeds bottom threshold (58 - 16 = 42)
        // Expected scroll = 64 - 58 + 16 = 22.0
        set_caret_byte(&mut state, 6);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.scroll.offset.y, 22.0);

        // 3. Caret moves down to Line 9 (index 27, y_top = 144.0, height = 16.0): exceeds bottom threshold
        // Expected scroll = 160 - 58 + 16 = 118.0, clamped to max_scroll (110.0)
        set_caret_byte(&mut state, 27);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.scroll.offset.y, 110.0);

        // 4. Caret moves up from Line 2 to Line 1 (index 3, y_top = 16.0, height = 16.0): below top threshold
        // Expected scroll = 16 - 16 = 0.0
        set_caret_byte(&mut state, 6);
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(state.scroll.offset.y, 0.0);
    }

    #[test]
    fn test_text_edit_vertical_selection_aware_auto_scrolling() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();

        // 10 lines of 16px: total height = 160px.
        // Viewport height = 58px. Max scroll = 110px.
        let mut state = TextEditState::new("l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9");
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        let edit_spec = TextEditSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 60.0),
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // Warmup frame
        focus_system.begin_frame();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        // Test case 1: Ctrl-A (Select All) should not change the vertical scroll state.
        state.scroll.offset.y = 50.0;
        input.text_events = vec![TextEvent::SelectAll];
        focus_system.begin_frame();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(state.scroll.offset.y, 50.0);

        // Test case 2: Double-clicking word on Line 1 (starts at byte index 3, y_top = 16.0)
        // when scroll.y = 20.0.
        // text_y = 5.0 - 20.0 = -15.0.
        // relative_pos.y = 24.0, mouse_pos.y = 24.0 - 15.0 = 9.0 (inside viewport).
        state.scroll.offset.y = 20.0;
        input.text_events = vec![];
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        input.mouse_pos = Vec2::new(5.0, 9.0);
        focus_system.begin_frame();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(selection_byte(&state), Some(3));
        assert_eq!(caret_byte(&state), 5);
        assert_eq!(state.scroll.offset.y, 0.0);

        // Test case 3: Double-clicking word on Line 9 (starts at byte index 27, y_top = 144.0)
        // when scroll.y = 100.0.
        // text_y = 5.0 - 100.0 = -95.0.
        // relative_pos.y = 152.0, mouse_pos.y = 152.0 - 95.0 = 57.0 (inside viewport).
        state.scroll.offset.y = 100.0;
        input.text_events = vec![];
        input.mouse_pressed = true;
        input.mouse_click_count = 2;
        input.mouse_pos = Vec2::new(5.0, 57.0);
        focus_system.begin_frame();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(selection_byte(&state), Some(27));
        assert_eq!(caret_byte(&state), 29);
        assert_eq!(state.scroll.offset.y, 110.0);
    }

    #[test]
    fn test_text_edit_caret_movement_with_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        focus_system.begin_frame();

        let mut state = TextEditState::new("hello world how are you");
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        // 1. Press CaretLeft (shift=false, ctrl=false) -> collapses selection to left edge (6)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 6);

        // 2. Press CaretRight (shift=false, ctrl=false) -> collapses selection to right edge (11)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 11);

        // 3. Press Ctrl+CaretLeft (shift=false, ctrl=true) -> starts at left edge (6) and moves one word left (to 5)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 5);

        // 4. Press Ctrl+CaretRight (shift=false, ctrl=true) -> starts at right edge (11) and moves one word right (to 12)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 12);

        // 5. Press Shift+CaretLeft (shift=true, ctrl=false) -> starts at caret (11) and moves one character left (to 10)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 10);

        // 6. Press Shift+CaretRight (shift=true, ctrl=false) -> starts at caret (11) and moves one character right (to 12)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 12);

        // 7. Press Ctrl+Shift+CaretLeft (shift=true, ctrl=true) -> starts at caret (11) and moves one word left (to 6)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 6);

        // 8. Press Ctrl+Shift+CaretRight (shift=true, ctrl=true) -> starts at caret (11) and moves one word right (to 12)
        set_selection_byte(&mut state, Some(6));
        set_caret_byte(&mut state, 11);
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
        assert_eq!(selection_byte(&state), Some(6));
        assert_eq!(caret_byte(&state), 12);
    }

    #[test]
    fn test_text_edit_vertical_caret_movement_with_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        focus_system.begin_frame();

        // 3 lines: "l0\nl1\nl2"
        // Line 0: "l0\n" (bytes 0..3)
        // Line 1: "l1\n" (bytes 3..6)
        // Line 2: "l2"   (bytes 6..8)
        let mut state = TextEditState::new("l0\nl1\nl2");
        state.had_keyboard_focus = true;
        focus_system.take_keyboard_focus(state.focus_id);

        let mut input = Input::default();
        let mut cmds = DrawCommands::new();

        let edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            ..spec()
        };

        // 1. CaretUp (shift=false): selection from 1 to 7.
        // Left/start boundary is 1 (Line 0).
        // CaretUp without shift collapses selection and moves one line up from start boundary (1).
        // Since 1 is on Line 0 (first visual line), it should move to the start of text (0).
        set_selection_byte(&mut state, Some(1));
        set_caret_byte(&mut state, 7);
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 0);

        // 2. CaretUp (shift=false): selection from 4 to 7.
        // Left/start boundary is 4 (Line 1).
        // CaretUp without shift collapses selection and moves one line up from start boundary (4).
        // Since 4 is on Line 1, moving up should place it on Line 0.
        // The column for byte 4 (on Line 1) is 4 - 3 = 1 character from start, so x = 8.0.
        // On Line 0, x = 8.0 corresponds to byte index 1.
        set_selection_byte(&mut state, Some(4));
        set_caret_byte(&mut state, 7);
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 1);

        // 3. CaretDown (shift=false): selection from 1 to 4.
        // Right/end boundary is 4 (Line 1).
        // CaretDown without shift collapses selection and moves one line down from end boundary (4).
        // Since 4 is on Line 1, moving down should place it on Line 2.
        // The column for byte 4 (on Line 1) is 4 - 3 = 1, so x = 8.0.
        // On Line 2, x = 8.0 corresponds to byte index 7.
        set_selection_byte(&mut state, Some(4));
        set_caret_byte(&mut state, 1);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 7);

        // 4. CaretDown (shift=false): selection from 4 to 7.
        // Right/end boundary is 7 (Line 2).
        // CaretDown without shift collapses selection and moves one line down from end boundary (7).
        // Since 7 is on Line 2 (last visual line), it should move to the end of text (8).
        set_selection_byte(&mut state, Some(7));
        set_caret_byte(&mut state, 4);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), None);
        assert_eq!(caret_byte(&state), 8);
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
            set_caret_byte(&mut state, 5);
            set_selection_byte(&mut state, None);
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
            set_caret_byte(&mut state, 0);
            set_selection_byte(&mut state, None);
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
            state.had_keyboard_focus = true;
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
            set_caret_byte(&mut state, 5);
            set_selection_byte(&mut state, None);
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
            set_caret_byte(&mut state, 0);
            set_selection_byte(&mut state, None);
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
            set_caret_byte(&mut state, 5);
            set_selection_byte(&mut state, None);
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
            set_caret_byte(&mut state, 0);
            set_selection_byte(&mut state, None);
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
            set_caret_byte(&mut state, 1);
            set_selection_byte(&mut state, None);
            state.had_keyboard_focus = true;
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
        set_caret_byte(&mut state, 5);
        set_selection_byte(&mut state, None);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 11);

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
        assert_eq!(caret_byte(&state), 5);

        // 3. Boundary Condition: Arrow Up on first line
        set_caret_byte(&mut state, 2);
        input.text_events = vec![TextEvent::CaretUp { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 0);

        // 4. Boundary Condition: Arrow Down on last line
        set_caret_byte(&mut state, 14);
        input.text_events = vec![TextEvent::CaretDown { shift: false }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 17);

        // 5. Shift + Arrow Down from Line 0 to Line 1 (extending selection)
        set_caret_byte(&mut state, 2);
        set_selection_byte(&mut state, None);
        input.text_events = vec![TextEvent::CaretDown { shift: true }];
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(selection_byte(&state), Some(2));
        assert_eq!(caret_byte(&state), 8);
    }

    #[test]
    fn test_page_up_down_moves_by_outer_scroll_height_whole_lines() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        let mut edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };
        // 1px border on each side leaves a 48px scroll outer height.
        // DummyTextSys lines are 16px tall, so PgUp/PgDown moves three lines.
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

        let mut state = TextEditState::new("line0\nline1\nline2\nline3\nline4\nline5");
        let mut input = Input::default();
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);

        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        set_caret_byte(&mut state, 2);
        set_selection_byte(&mut state, None);
        input.key_pressed_page_down = true;
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 20);
        assert_eq!(selection_byte(&state), None);

        input.key_pressed_page_down = false;
        input.key_pressed_page_up = true;
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 2);

        set_caret_byte(&mut state, 29);
        input.key_pressed_page_up = false;
        input.key_pressed_page_down = true;
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), state.value.len());

        set_caret_byte(&mut state, 9);
        input.key_pressed_page_down = false;
        input.key_pressed_page_up = true;
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 0);
    }

    #[test]
    fn test_page_up_down_preserves_caret_x_with_short_target_lines() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        let mut edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

        let mut state = TextEditState::new("000000\n1\n222222\n333\n444444\n5");
        let mut input = Input::default();
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);

        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Line 0 column 5 pages down to line 3. Line 3 has only three
        // characters, so the closest valid x-position is its end.
        set_caret_byte(&mut state, 5);
        set_selection_byte(&mut state, None);
        input.key_pressed_page_down = true;
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 19);

        // Line 4 column 5 pages up to line 1. Line 1 has one character, so
        // preserving x clamps to that line's end.
        set_caret_byte(&mut state, 25);
        input.key_pressed_page_down = false;
        input.key_pressed_page_up = true;
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        assert_eq!(caret_byte(&state), 8);
    }

    #[test]
    fn test_shift_page_down_extends_selection() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut cmds = DrawCommands::new();

        let mut edit_spec = TextEditSpec {
            newline_policy: NewlinePolicy::Allow,
            vertical_align: Align::Start,
            ..spec()
        };
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 50.0);

        let mut state = TextEditState::new("line0\nline1\nline2\nline3\nline4\nline5");
        let mut input = Input::default();
        focus_system.begin_frame();
        focus_system.take_keyboard_focus(state.focus_id);

        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        set_caret_byte(&mut state, 2);
        set_selection_byte(&mut state, None);
        input.key_pressed_page_down = true;
        input.modifier_shift = true;
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(selection_byte(&state), Some(2));
        assert_eq!(caret_byte(&state), 20);
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
        state.had_keyboard_focus = true;
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
        set_caret_byte(&mut state, 9); // "line2|2\n" → inside Line 1
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            6,
            "Home (no ctrl) from line 1 mid should move to start of line 1 (byte 6)"
        );
        assert_eq!(selection_byte(&state), None);

        // ── 2. End (ctrl=false) from middle of Line 1 → end of Line 1 ──────────
        set_caret_byte(&mut state, 9); // restore to mid-line-1
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            11,
            "End (no ctrl) from line 1 mid should move to end of line 1 (byte 11, before \\n)"
        );
        assert_eq!(selection_byte(&state), None);

        // ── 3. Home (ctrl=false) on Line 0 already at start → stays at 0 ───────
        set_caret_byte(&mut state, 0);
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            0,
            "Home (no ctrl) at byte 0 should stay at 0"
        );

        // ── 4. End (ctrl=false) on last line → value.len() ─────────────────────
        set_caret_byte(&mut state, 14); // inside "line3"
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            17,
            "End (no ctrl) on last line should move to value.len() (byte 17)"
        );

        // ── 5. Shift+Home (ctrl=false) extends selection to start of line ──────
        set_caret_byte(&mut state, 9); // mid-line-1
        set_selection_byte(&mut state, None);
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
            selection_byte(&state),
            Some(9),
            "Shift+Home (no ctrl) should anchor selection at old caret (9)"
        );
        assert_eq!(
            caret_byte(&state),
            6,
            "Shift+Home (no ctrl) should move caret to start of current line (6)"
        );

        // ── 6. Shift+End (ctrl=false) extends selection to end of line ──────────
        set_caret_byte(&mut state, 9); // mid-line-1
        set_selection_byte(&mut state, None);
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
            selection_byte(&state),
            Some(9),
            "Shift+End (no ctrl) should anchor selection at old caret (9)"
        );
        assert_eq!(
            caret_byte(&state),
            11,
            "Shift+End (no ctrl) should move caret to end of current line (11)"
        );

        // ── 7. Home (ctrl=true) from mid-string → byte 0 ───────────────────────
        set_caret_byte(&mut state, 9);
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            0,
            "Home (ctrl=true) should always move to byte 0"
        );
        assert_eq!(selection_byte(&state), None);

        // ── 8. End (ctrl=true) from mid-string → value.len() ───────────────────
        set_caret_byte(&mut state, 9);
        set_selection_byte(&mut state, None);
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
            caret_byte(&state),
            17,
            "End (ctrl=true) should always move to value.len()"
        );
        assert_eq!(selection_byte(&state), None);

        // ── 9. Shift+Ctrl+Home extends selection to byte 0 ──────────────────────
        set_caret_byte(&mut state, 9);
        set_selection_byte(&mut state, None);
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
            selection_byte(&state),
            Some(9),
            "Shift+Ctrl+Home should anchor selection at old caret (9)"
        );
        assert_eq!(
            caret_byte(&state),
            0,
            "Shift+Ctrl+Home should move caret to byte 0"
        );

        // ── 10. Shift+Ctrl+End extends selection to value.len() ─────────────────
        set_caret_byte(&mut state, 9);
        set_selection_byte(&mut state, None);
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
            selection_byte(&state),
            Some(9),
            "Shift+Ctrl+End should anchor selection at old caret (9)"
        );
        assert_eq!(
            caret_byte(&state),
            17,
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

        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(3)); // 'l' in "hello"
        set_caret_byte(&mut state, 9); // 'r' in "world"

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
                    width: spec().style.focus_width,
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
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(29.0, 50.0, spec().style.caret_width, 16.0),
                    color: spec().style.caret_color,
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_selection_highlights_collapsed_trailing_space_affordance() {
        let mut text_system = CollapsedTrailingSpaceTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("a b");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        focus_system.begin_frame();

        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(0));
        set_caret_byte(&mut state, 2);

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            TextEditSpec {
                rect: Rect::new(0.0, 0.0, 200.0, 100.0),
                newline_policy: NewlinePolicy::Allow,
                wrap: true,
                vertical_align: Align::Start,
                ..spec()
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        let has_collapsed_space_affordance = cmds.iter().any(|cmd| {
            matches!(
                cmd,
                DrawCmd::FillRect {
                    rect,
                    color,
                    ..
                } if *color == spec().style.select_color
                    && *rect == Rect::new(5.0, 5.0, 16.0, 16.0)
            )
        });

        assert!(
            has_collapsed_space_affordance,
            "selection highlight should extend past line.logical_width for the collapsed trailing space"
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

        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(2)); // 'e' in "one"
        set_caret_byte(&mut state, 10); // 'r' in "three"

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
                    width: spec().style.focus_width,
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
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(21.0, 58.0, spec().style.caret_width, 16.0),
                    color: spec().style.caret_color,
                    z: 0,
                },
                DrawCmd::PopClip,
            ])
        );
    }

    #[test]
    fn test_text_edit_caret_up_down_width_mismatch() {
        // This test verifies that CaretUp and CaretDown navigation use the correct layout width.
        // Under the layout width mismatch bug, CaretUp and CaretDown events prepare their text layout
        // using the widget's physical border width (spec.rect.w - 2.0 * spec.style.border_width),
        // ignoring any error stripe subtraction or dynamic maximum logical size boundaries used
        // during the draw/render phase.
        // This mismatch leads to incorrect wrapping or line calculations during navigation, causing
        // the caret to jump unexpectedly or land on wrong characters compared to what is rendered.

        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghij");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();

        // Configure style to enable wrapping
        let mut spec_error = spec();
        spec_error.rect = Rect::new(0.0, 0.0, 52.0, 100.0); // 50px width content boundary + 2px borders
        spec_error.style.border_width = 1.0;
        spec_error.style.padding_x = 0.0;
        spec_error.style.padding_y = 0.0;
        spec_error.wrap = true;
        spec_error.error = true;
        spec_error.style.error_stripe_width = 4.0;

        // With spec.error = true and spec.rect.w = 52.0:
        // - Correct layout width is metrics.logical_size.x.max(scroll_outer_rect.w)
        //   where logical_size.x = 40.0, scroll_outer_rect.w = 46.0 (since it wraps at 46.0px max_width).
        //   So correct width is 46.0.
        // - Line 0: "abcde" (bytes 0..5), Line 1: "fghij" (bytes 5..10).
        // - Start caret at index 8 ('i', Line 1). CaretUp should move to index 3 ('d', Line 0).
        // - Buggy event handler layout width is 50.0 (ignoring error stripe). Fits 6 characters per line.
        //   Visual lines under buggy handler: Line 0: "abcdef" (bytes 0..6), Line 1: "ghij" (bytes 6..10).
        //   Under the buggy handler, CaretUp thinks index 8 is column 2 on Line 1, moving it up to
        //   index 2 on Line 0.

        // --- Test CaretUp Mismatch ---
        // Start caret at index 8 ('i'). Since the correct layout has 46.0 width, it wraps as
        // "abcde" and "fghij", so CaretUp should move it to index 3.
        // Due to the bug (layout width 50.0), CaretUp thinks index 8 is on Line 1 of the 50.0 layout,
        // and moves it up to index 2.
        set_caret_byte(&mut state, 8);
        state.had_keyboard_focus = true;
        focus_system.begin_frame();

        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretUp { shift: false });

        raw::text_edit(
            spec_error,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );

        assert_eq!(
            caret_byte(&state),
            3,
            "CaretUp should move caret to index 3 under correct wrapped layout width"
        );
    }

    #[test]
    fn test_text_edit_alignment_combinations() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let input = Input::default();

        // 1. Top-Left (Start, Start)
        {
            let mut state = TextEditState::new("hello");
            let mut cmds = DrawCommands::new();
            let edit_spec = TextEditSpec {
                vertical_align: Align::Start,
                line_align: TextLineAlign::Start,
                ..spec()
            };
            raw::text_edit(
                edit_spec,
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );

            // Inset by border (1.0) and padding (4.0).
            // scroll_outer_rect = (1.0, 1.0, 198.0, 28.0).
            // Align::Start text_y = scroll_outer_rect.y + padding = 1.0 + 4.0 = 5.0.
            let has_text = cmds.iter().any(|cmd| {
                if let DrawCmd::Text { rect, .. } = cmd {
                    rect.y == 5.0
                } else {
                    false
                }
            });
            assert!(has_text, "Align::Start text Y should be 5.0");
        }

        // 2. Center-Center (Center, Center)
        {
            let mut state = TextEditState::new("hello");
            let mut cmds = DrawCommands::new();
            let edit_spec = TextEditSpec {
                vertical_align: Align::Center,
                line_align: TextLineAlign::Center,
                ..spec()
            };
            raw::text_edit(
                edit_spec,
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );

            // Align::Center text_y = scroll_outer_rect.y + (28.0 - 16.0)/2.0 = 1.0 + 6.0 = 7.0.
            let has_text = cmds.iter().any(|cmd| {
                if let DrawCmd::Text { rect, .. } = cmd {
                    rect.y == 7.0
                } else {
                    false
                }
            });
            assert!(has_text, "Align::Center text Y should be 7.0");
        }

        // 3. Bottom-Right (End, End)
        {
            let mut state = TextEditState::new("hello");
            let mut cmds = DrawCommands::new();
            let edit_spec = TextEditSpec {
                vertical_align: Align::End,
                line_align: TextLineAlign::End,
                ..spec()
            };
            raw::text_edit(
                edit_spec,
                &mut state,
                &input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );

            // Align::End text_y = scroll_outer_rect.y + scroll_outer_rect.h - padding - logical_size.y
            // = 1.0 + 28.0 - 4.0 - 16.0 = 9.0.
            let has_text = cmds.iter().any(|cmd| {
                if let DrawCmd::Text { rect, .. } = cmd {
                    rect.y == 9.0
                } else {
                    false
                }
            });
            assert!(has_text, "Align::End text Y should be 9.0");
        }

        // 4. Hit-testing: verify that clicking on aligned text maps to correct caret index
        // Let's test bottom-aligned text (vertical_align = Align::End).
        // Since Y is 9.0, clicking at Y = 17.0 (middle of the line) should hit-test correctly.
        {
            let mut state = TextEditState::new("hello");
            let mut cmds = DrawCommands::new();
            let edit_spec = TextEditSpec {
                vertical_align: Align::End,
                line_align: TextLineAlign::Start,
                ..spec()
            };

            let mut click_input = Input::default();
            // Text is placed at x = 5.0 (scroll_outer_rect.x + padding = 1.0 + 4.0).
            // Character width in DummyTextSys is 8.0px.
            // Click on 'l' (index 3). x offset should be around 5.0 + 3 * 8.0 = 29.0.
            // Let's click at x = 31.0 (between 29.0 and 37.0).
            // Click Y should be in the line: text Y is 9.0, height is 16.0, so middle is 17.0.
            click_input.mouse_pos = Vec2::new(31.0, 17.0);

            // Frame 1: Warmup to claim hover
            focus_system.begin_frame();
            raw::text_edit(
                edit_spec.clone(),
                &mut state,
                &click_input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            focus_system.end_frame();

            // Frame 2: Mouse click
            click_input.mouse_pressed = true;
            click_input.mouse_down = true;
            focus_system.begin_frame();
            raw::text_edit(
                edit_spec,
                &mut state,
                &click_input,
                &mut focus_system,
                &mut text_system,
                &mut cmds,
            );
            focus_system.end_frame();

            assert_eq!(
                caret_byte(&state),
                3,
                "Hit testing should resolve correctly to index 3"
            );
        }
    }

    #[test]
    fn test_edit_layout_size_wrapping() {
        let mut text_system = DummyTextSys;
        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0);
        edit_spec.wrap = true;
        edit_spec.style.border_width = 1.0;
        edit_spec.style.padding_x = 4.0;
        edit_spec.style.padding_y = 4.0;

        // scroll_outer_rect: x=1.0, y=1.0, w=98.0, h=28.0.
        // available text width without scrollbar = 98.0 - 2 * 4.0 = 90.0.
        // In characters: 90.0 / 8.0 = 11.25 -> 11 characters.

        let text_style =
            super::to_text_style(edit_spec.style, edit_spec.wrap, edit_spec.line_align);

        // Case A: Short text that does not overflow height.
        // "abcdefghijk" -> 11 chars. Should fit on 1 line.
        // Height = 16.0. Height + padding = 24.0 <= 28.0 (viewport h).
        // No vertical scrollbar width deduction should happen.
        let (metrics, layout_width, layout_height, _) =
            super::raw::edit_layout_size("abcdefghijk", &edit_spec, text_style, &mut text_system);
        assert_eq!(metrics.line_count, 1);
        assert_eq!(layout_width, 90.0);
        assert_eq!(layout_height, 28.0);

        // Case B: Long text that wraps and overflows height.
        // "abcdefghijklmnopqrst" -> 20 chars.
        // Initial available width = 90.0. Max chars = 11.
        // Visual lines: 11 chars + 9 chars.
        // Height = 32.0. Height + padding = 40.0 > 28.0 (viewport h).
        // Scrollbar will appear and steal 5px.
        // New available width = 85.0. Max chars = 10.
        // Final visual lines: 10 chars ("abcdefghij") + 10 chars ("klmnopqrst").
        // Both lines should have 10 chars.
        let (metrics, layout_width, _layout_height, _) = super::raw::edit_layout_size(
            "abcdefghijklmnopqrst",
            &edit_spec,
            text_style,
            &mut text_system,
        );
        assert_eq!(metrics.line_count, 2);
        assert_eq!(metrics.lines[0].byte_end - metrics.lines[0].byte_start, 10);
        assert_eq!(metrics.lines[1].byte_end - metrics.lines[1].byte_start, 10);
        assert_eq!(layout_width, 85.0);
    }

    #[test]
    fn test_text_edit_error_vertical_scrollbar_layout_and_hit_test() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("line1\nline2\nline3\nline4\nline5");
        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0);
        edit_spec.error = true;
        edit_spec.newline_policy = NewlinePolicy::Allow;

        state.scroll.offset.y = 16.0;

        let input = Input::default();
        let mut cmds = DrawCommands::new();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        assert!(
            cmds.iter().any(|cmd| matches!(
                cmd,
                DrawCmd::PushClip {
                    rect: Rect {
                        x: 5.0,
                        y: 1.0,
                        w: 189.0,
                        h: 38.0
                    }
                }
            )),
            "content clip should account for border, error stripe, and vertical scrollbar"
        );

        assert!(
            cmds.iter().any(|cmd| matches!(
                cmd,
                DrawCmd::Text {
                    rect: Rect {
                        x: 9.0,
                        y: -11.0,
                        w: 194.0,
                        h: 80.0
                    },
                    ..
                }
            )),
            "text origin should be offset by the error stripe and scroll amount"
        );

        assert!(
            cmds.iter().any(|cmd| matches!(
                cmd,
                DrawCmd::FillRect {
                    rect: Rect {
                        x: 194.0,
                        y: 1.0,
                        w: 5.0,
                        h: 38.0
                    },
                    ..
                }
            )),
            "vertical scrollbar should stay tucked against the right edge"
        );

        let mut click_input = Input::default();
        click_input.mouse_pos = Vec2::new(9.0, 37.0);

        focus_system.begin_frame();
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &click_input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        click_input.mouse_pressed = true;
        click_input.mouse_down = true;

        focus_system.begin_frame();
        raw::text_edit(
            edit_spec,
            &mut state,
            &click_input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            caret_byte(&state),
            18,
            "hit testing should use the error stripe-adjusted, scrolled text rect"
        );
    }

    #[test]
    fn test_text_edit_visual_vertical_scrollbar() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("one\ntwo\nthree\nfour\nfive"); // 5 lines, height 80px
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0); // height 40px -> viewport h = 38px
        edit_spec.newline_policy = NewlinePolicy::Allow;

        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Find if a vertical scrollbar track/thumb was drawn.
        // Slider/scrollbar drawing uses FillRect for track/thumb, and since the viewport height
        // is 38px, and text height is 80px + padding = 88px, it overflows.
        // Specifically, let's assert that content bounds width in PushClip is shrunk to 193.0 (200 - 2 border - 5 scrollbar).
        let has_shrunk_clip = cmds.iter().any(|cmd| {
            if let DrawCmd::PushClip { rect } = cmd {
                rect.w == 193.0
            } else {
                false
            }
        });
        assert!(
            has_shrunk_clip,
            "The clip rect width should be shrunk to 193.0 to accommodate the vertical scrollbar"
        );

        // The vertical track has width 5.0 and is placed at x = 194.0
        let has_vertical_track = cmds.iter().any(|cmd| {
            if let DrawCmd::FillRect { rect, .. } = cmd {
                rect.x == 194.0 && rect.w == 5.0
            } else {
                false
            }
        });
        assert!(has_vertical_track, "Should render vertical scrollbar track");
    }

    #[test]
    fn test_text_edit_visual_horizontal_scrollbar() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrstuvwxyz0123"); // 30 chars = 240px wide
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 200.0, 40.0); // width 200px -> viewport w = 198px

        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Since horizontal scrollbar is triggered, the content height is shrunk by 5px (from 38px to 33px)
        let has_shrunk_clip = cmds.iter().any(|cmd| {
            if let DrawCmd::PushClip { rect } = cmd {
                rect.h == 33.0
            } else {
                false
            }
        });
        assert!(
            has_shrunk_clip,
            "The clip rect height should be shrunk to 33.0 to accommodate the horizontal scrollbar"
        );

        // The horizontal track has height 5.0 and is placed at y = 34.0
        let has_horizontal_track = cmds.iter().any(|cmd| {
            if let DrawCmd::FillRect { rect, .. } = cmd {
                rect.y == 34.0 && rect.h == 5.0
            } else {
                false
            }
        });
        assert!(
            has_horizontal_track,
            "Should render horizontal scrollbar track"
        );
    }

    #[test]
    fn test_text_edit_wrapping() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
        let input = Input::default();
        let mut cmds = DrawCommands::new();
        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // available width = 90px without scrollbar (11 chars)
        edit_spec.wrap = true;

        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Verify that the text was prepared with the narrower max_width (85px)
        // so that it fits 10 characters per line.
        if let Some((text, metrics)) = text_system.last_run {
            assert_eq!(text, "abcdefghijklmnopqrst");
            assert_eq!(metrics.line_count, 2);
            assert_eq!(metrics.lines[0].byte_end - metrics.lines[0].byte_start, 10);
            assert_eq!(metrics.lines[1].byte_end - metrics.lines[1].byte_start, 10);
        } else {
            panic!("DummyTextSys did not record last prepared layout run");
        }
    }

    #[test]
    fn test_text_edit_wrapping_home_end() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        state.had_keyboard_focus = true;

        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
        edit_spec.wrap = true;

        // Visual Line 0: "abcdefghij" (index 0..10)
        // Visual Line 1: "klmnopqrst" (index 10..20)

        // Test Home
        set_caret_byte(&mut state, 15); // caret is on 'p' (Line 1)
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretHome {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(caret_byte(&state), 10);

        // Test End
        set_caret_byte(&mut state, 3); // caret is on 'd' (Line 0)
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(caret_byte(&state), 10);
    }

    #[test]
    fn test_text_edit_wrapping_home_end_preserves_visual_line_side() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        state.had_keyboard_focus = true;

        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
        edit_spec.wrap = true;

        // Visual Line 0: "abcdefghij" (bytes 0..10)
        // Visual Line 1: "klmnopqrst" (bytes 10..20)
        //
        // The insertion byte at the wrap boundary is ambiguous:
        // - AfterCluster(9) is the end of visual line 0.
        // - BeforeCluster(10) is the start of visual line 1.

        state.caret = CaretPosition::AfterCluster {
            cluster_byte_index: 9,
        };
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretHome {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            state.caret,
            CaretPosition::BeforeCluster {
                cluster_byte_index: 0
            },
            "Home from the end anchor of visual line 0 should stay on visual line 0"
        );

        state.caret = CaretPosition::BeforeCluster {
            cluster_byte_index: 10,
        };
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            state.caret,
            CaretPosition::AfterCluster {
                cluster_byte_index: 19
            },
            "End from the start anchor of visual line 1 should stay on visual line 1"
        );
    }

    #[test]
    fn test_text_edit_wrapping_home_end_targets_visual_line_anchors() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        state.had_keyboard_focus = true;

        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
        edit_spec.wrap = true;

        state.caret = CaretPosition::BeforeCluster {
            cluster_byte_index: 3,
        };
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec.clone(),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            state.caret,
            CaretPosition::AfterCluster {
                cluster_byte_index: 9
            },
            "End on visual line 0 should use the line-end anchor, not the next line start"
        );

        state.caret = CaretPosition::BeforeCluster {
            cluster_byte_index: 15,
        };
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretHome {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            state.caret,
            CaretPosition::BeforeCluster {
                cluster_byte_index: 10
            },
            "Home on visual line 1 should use the line-start anchor"
        );
    }

    #[test]
    fn test_text_edit_wrapping_end_stays_before_collapsed_boundary_space() {
        let mut text_system = CollapsedTrailingSpaceTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("a b");
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.end_frame();
        state.had_keyboard_focus = true;

        let mut edit_spec = spec();
        edit_spec.wrap = true;

        state.caret = CaretPosition::BeforeCluster {
            cluster_byte_index: 0,
        };
        focus_system.begin_frame();
        let mut input = Input::default();
        input.text_events.push(TextEvent::CaretEnd {
            shift: false,
            ctrl: false,
        });
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut DrawCommands::new(),
        );
        focus_system.end_frame();

        assert_eq!(
            state.caret,
            CaretPosition::BeforeCluster {
                cluster_byte_index: 1
            },
            "End on a line ending with collapsed soft-wrap whitespace should stay on that visual line"
        );
    }

    #[test]
    fn test_text_edit_wrapping_selection_visual() {
        let mut text_system = DummyTextSys;
        let mut focus_system = FocusSystem::new();
        let mut state = TextEditState::new("abcdefghijklmnopqrst"); // 20 chars
        state.had_keyboard_focus = true;
        set_selection_byte(&mut state, Some(5));
        set_caret_byte(&mut state, 15);
        focus_system.take_keyboard_focus(state.focus_id);

        let mut edit_spec = spec();
        edit_spec.rect = Rect::new(0.0, 0.0, 100.0, 30.0); // wraps after 10 chars under scrollbar
        edit_spec.wrap = true;

        let mut cmds = DrawCommands::new();
        let input = Input::default();
        raw::text_edit(
            edit_spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );

        // Assert the selection rectangle for Line 0 (indices 5..10).
        // In DummyTextSys:
        // - start_x = 5 chars * 8px = 40.0px.
        // - end_x = 10 chars * 8px = 80.0px.
        // So the selection rect should start at x = 45.0 (40.0 + 5.0 padding) and have width = 40.0.
        let has_correct_selection = cmds.iter().any(|cmd| {
            if let DrawCmd::FillRect { rect, color, .. } = cmd {
                *color == spec().style.select_color && rect.x == 45.0 && rect.w == 40.0
            } else {
                false
            }
        });
        assert!(
            has_correct_selection,
            "Selection highlight should cover the selected range [5..10] on Line 0"
        );
    }
}
