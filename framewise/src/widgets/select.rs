use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{ClipRect, Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SelectSpec<'a> {
        /// Bounding rect for the closed box (height h_md = 28).
        pub rect: Rect,
        pub value: &'a str,
        pub font: FontId,
        pub options: &'a [&'a str],
        pub disabled: bool,
        pub style: super::SelectStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SelectResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level select widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn select<'a, T: crate::text::TextSystem>(
        spec: SelectSpec<'a>,
        state: &mut SelectState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_system: &mut T,
    ) -> SelectResult {
        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                input,
                focus_sys,
                crate::focus::FocusTraversalKeys::all(),
                spec.disabled,
            )
        };

        if !spec.options.is_empty() {
            let current_val = if state.selected_index < spec.options.len() {
                spec.options[state.selected_index]
            } else {
                ""
            };
            if current_val != spec.value {
                // Out of band update, search for spec.value in options
                let mut found = false;
                for (i, opt) in spec.options.iter().enumerate() {
                    if *opt == spec.value {
                        state.selected_index = i;
                        found = true;
                        break;
                    }
                }
                if !found {
                    state.selected_index = state.selected_index.min(spec.options.len() - 1);
                }
            }
        }

        let mut is_clicked = clicked;
        if focused && input.key_pressed_enter && !state.open {
            is_clicked = true;
        }
        if state.space_is_active && input.key_released_space && !state.open {
            is_clicked = true;
        }

        if !focused || !input.key_down_space {
            state.space_is_active = false;
        }
        if focused && input.key_pressed_space {
            state.space_is_active = true;
        }

        if is_clicked && !spec.disabled {
            state.open = !state.open;
            if state.open {
                state.hovered = Some(state.selected_index);
            }
        }

        let s = spec.style;
        let r = Rect::new(
            spec.rect.x,
            spec.rect.y,
            spec.rect.w.max(s.min_width),
            s.height,
        );

        // Keyboard navigation when focused
        if focused && !spec.disabled && !spec.options.is_empty() {
            if input.key_pressed_down {
                if state.open {
                    let current = state.hovered.unwrap_or(0);
                    if current + 1 < spec.options.len() {
                        state.hovered = Some(current + 1);
                    }
                } else {
                    if state.selected_index + 1 < spec.options.len() {
                        state.selected_index += 1;
                    }
                }
            }

            if input.key_pressed_up {
                if state.open {
                    let current = state.hovered.unwrap_or(0);
                    if current > 0 {
                        state.hovered = Some(current - 1);
                    }
                } else {
                    if state.selected_index > 0 {
                        state.selected_index -= 1;
                    }
                }
            }

            if state.open && input.key_pressed_enter {
                if let Some(h) = state.hovered {
                    if h < spec.options.len() {
                        state.selected_index = h;
                        state.open = false;
                    }
                }
            }
        }

        // Mouse interaction with popup when open
        if state.open && !spec.disabled && !spec.options.is_empty() {
            let row_h = s.row_height;
            let popup_h = spec.options.len() as f32 * row_h + s.popup_pad_y * 2.0;
            let popup = Rect::new(r.x, r.y + s.height + s.popup_gap, r.w, popup_h);

            let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
            if is_visible && popup.contains(input.mouse_pos) {
                let relative_y = input.mouse_pos.y - (popup.y + s.popup_pad_y);
                let hovered_row = (relative_y / row_h).floor() as i32;
                if hovered_row >= 0 && hovered_row < spec.options.len() as i32 {
                    state.hovered = Some(hovered_row as usize);

                    if input.mouse_pressed {
                        state.selected_index = hovered_row as usize;
                        state.open = false;
                    }
                }
            } else if input.mouse_pressed && !r.contains(input.mouse_pos) {
                state.open = false;
            }
        }

        let mut cmds = DrawCommands::new();
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_focused = focused;

        // Focus / open ring.
        if visually_focused || state.open {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.accent),
                width: s.focus_width,
            });
        }

        cmds.push(DrawCmd::FillRect {
            rect: r,
            color: tint(s.background),
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: r,
            color: tint(s.border),
            width: s.border_width,
        });

        // Selected value text.
        let display_text = if !spec.options.is_empty() && state.selected_index < spec.options.len()
        {
            spec.options[state.selected_index]
        } else {
            spec.value
        };

        let val_layout = text_system.prepare(display_text, s.text_size, spec.font);
        let vty = r.y + (s.height - val_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(r.x + s.pad_x, vty, val_layout.size.x, val_layout.size.y),
            color: tint(s.text),
            handle: val_layout.handle,
        });

        // Chevron "v".
        let chev_color = if state.open { s.accent } else { s.muted };
        let chev_layout = text_system.prepare("v", s.chevron_size, spec.font);
        let cty = r.y + (s.height - chev_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(
                r.x + r.w - s.chevron_right,
                cty,
                chev_layout.size.x,
                chev_layout.size.y,
            ),
            color: tint(chev_color),
            handle: chev_layout.handle,
        });

        // Dropdown popup.
        if state.open && !spec.options.is_empty() {
            let row_h = s.row_height;
            let popup_h = spec.options.len() as f32 * row_h + s.popup_pad_y * 2.0;
            let popup = Rect::new(r.x, r.y + s.height + s.popup_gap, r.w, popup_h);

            cmds.push(DrawCmd::FillRect {
                rect: popup,
                color: tint(s.background),
            });
            cmds.push(DrawCmd::StrokeRect {
                rect: popup,
                color: tint(s.border),
                width: s.border_width,
            });

            for (i, opt) in spec.options.iter().enumerate() {
                let is_selected = i == state.selected_index;
                let is_hovered = state.hovered == Some(i);
                let row_y = popup.y + s.popup_pad_y + i as f32 * row_h;
                let row_rect = Rect::new(popup.x, row_y, popup.w, row_h);

                if is_selected {
                    cmds.push(DrawCmd::FillRect {
                        rect: row_rect,
                        color: tint(s.selected_bg),
                    });
                } else if is_hovered {
                    cmds.push(DrawCmd::FillRect {
                        rect: row_rect,
                        color: tint(s.hover),
                    });
                }

                let text_color = if is_selected { s.selected_text } else { s.text };
                let opt_layout = text_system.prepare(opt, s.text_size, spec.font);
                let oty = row_y + (row_h - opt_layout.size.y) * 0.5;
                cmds.push(DrawCmd::Text {
                    rect: Rect::new(
                        popup.x + s.pad_x + 2.0,
                        oty,
                        opt_layout.size.x,
                        opt_layout.size.y,
                    ),
                    color: tint(text_color),
                    handle: opt_layout.handle,
                });
            }
        }

        SelectResult {
            draw: cmds,
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: (clicked && input.mouse_down) || state.space_is_active,
                clicked: is_clicked,
            },
            focused,
            content_bounds: r.inset(s.border_width),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectStyle {
    pub min_width: f32,
    pub height: f32,
    pub row_height: f32,
    pub popup_gap: f32,
    pub popup_pad_y: f32,
    pub pad_x: f32,
    pub chevron_right: f32,
    pub text_size: f32,
    pub chevron_size: f32,
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub hover: Color,
    pub muted: Color,
    pub accent: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectState {
    pub selected_index: usize,
    pub open: bool,
    pub hovered: Option<usize>,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

impl SelectResult {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectSpecBuilder<'a> {
    pub value: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<SelectStyle>,
    pub options: Option<&'a [&'a str]>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<ClipRect>,
}

impl<'a> SelectSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn options(mut self, options: &'a [&'a str]) -> Self {
        self.options = Some(options);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    /// Sets the clip rectangle. High-level context functions supply this automatically — only needed when using the raw API directly.
    pub fn clip_rect(mut self, clip_rect: ClipRect) -> Self {
        self.clip_rect = Some(clip_rect);
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
            self.style = Some(theme.select_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn build(self) -> raw::SelectSpec<'a> {
        raw::SelectSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            value: self.value.expect("value not set — call .value()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            options: self.options.expect("options not set — call .options()"),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

pub fn select<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SelectSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut SelectState,
) -> SelectResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::select(spec, state, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw);

    SelectResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SelectSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

    fn select_dummy<'a>(spec: SelectSpec<'a>) -> raw::SelectResult {
        raw::select(
            spec,
            &mut SelectState::default(),
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_select_visual_normal() {
        let options = vec!["Option 1", "Option 2", "Option 3"];
        let spec = SelectSpec {
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            font: FontId(0),
            options: &options,
            disabled: false,
            style: crate::theme::Theme::framewise().select_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = select_dummy(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                    color: s.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(10.0, 6.0, 64.0, 16.0),
                    color: s.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(162.0, 6.0, 8.0, 16.0),
                    color: s.muted,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_select_visual_open() {
        let mut text_sys = DummyTextSys;
        let options = vec!["Option 1", "Option 2", "Option 3"];
        let spec = SelectSpec {
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            font: FontId(0),
            options: &options,
            disabled: false,
            style: crate::theme::Theme::framewise().select_style(),
            clip_rect: None,
        };
        let s = spec.style;

        // Pass SelectState { open: true, ... } to simulate open state
        let state = SelectState {
            selected_index: 0,
            open: true,
            hovered: Some(1),
            space_is_active: false,
            focus_id: crate::focus::FocusId::new(),
        };

        let mut state = state;
        let res = raw::select(
            spec,
            &mut state,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut text_sys,
        );

        let r = Rect::new(0.0, 0.0, 180.0, 28.0);
        let popup = Rect::new(0.0, 30.0, 180.0, 86.0);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: r.inset(-s.focus_offset),
                    color: s.accent,
                    width: s.focus_width,
                },
                DrawCmd::FillRect {
                    rect: r,
                    color: s.background,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(10.0, 6.0, 64.0, 16.0),
                    color: s.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(162.0, 6.0, 8.0, 16.0),
                    color: s.accent,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: popup,
                    color: s.background,
                },
                DrawCmd::StrokeRect {
                    rect: popup,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 34.0, 180.0, 26.0),
                    color: s.selected_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(12.0, 39.0, 64.0, 16.0),
                    color: s.selected_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 60.0, 180.0, 26.0),
                    color: s.hover,
                },
                DrawCmd::Text {
                    rect: Rect::new(12.0, 65.0, 64.0, 16.0),
                    color: s.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(12.0, 91.0, 64.0, 16.0),
                    color: s.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_select_click_takes_focus_and_opens() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = SelectState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let options = vec!["Option 1", "Option 2"];
        let spec = SelectSpec {
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            font: FontId(0),
            options: &options,
            disabled: false,
            style: crate::theme::Theme::framewise().select_style(),
            clip_rect: None,
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::select(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking select must request focus"
        );
        assert!(state.open, "Clicking select must open the popup dropdown");
    }

    #[test]
    fn test_select_clipped_click_does_not_take_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = SelectState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let options = vec!["Option 1", "Option 2"];
        let spec = SelectSpec {
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            font: FontId(0),
            options: &options,
            disabled: false,
            style: crate::theme::Theme::framewise().select_style(),
            clip_rect: Some(Rect::new(500.0, 500.0, 180.0, 28.0)),
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::select(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away select must not take focus"
        );
    }

    #[test]
    fn test_select_keyboard_navigation() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = SelectState::default();
        let mut input = Input::default();
        let mut text_sys = DummyTextSys;
        let options = vec!["Option 1", "Option 2", "Option 3"];

        // Focus the widget first
        focus_sys.take_focus(state.focus_id);

        // Frame 1: Press Arrow Down while closed -> selected index changes to 1
        input.key_pressed_down = true;
        focus_sys.begin_frame();
        raw::select(
            SelectSpec {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 1",
                font: FontId(0),
                options: &options,
                disabled: false,
                style: crate::theme::Theme::framewise().select_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();
        input.key_pressed_down = false;

        assert_eq!(state.selected_index, 1);
        assert!(!state.open);

        // Frame 2: Press Space -> opens dropdown
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        raw::select(
            SelectSpec {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                font: FontId(0),
                options: &options,
                disabled: false,
                style: crate::theme::Theme::framewise().select_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        raw::select(
            SelectSpec {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                font: FontId(0),
                options: &options,
                disabled: false,
                style: crate::theme::Theme::framewise().select_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();
        input.key_released_space = false;

        assert!(state.open);
        assert_eq!(state.hovered, Some(1));

        // Frame 3: Press Arrow Down while open -> hovers index 2
        input.key_pressed_down = true;
        focus_sys.begin_frame();
        raw::select(
            SelectSpec {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                font: FontId(0),
                options: &options,
                disabled: false,
                style: crate::theme::Theme::framewise().select_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();
        input.key_pressed_down = false;

        assert_eq!(state.hovered, Some(2));

        // Frame 4: Press Enter while open -> selects hovered (index 2) and closes dropdown
        input.key_pressed_enter = true;
        focus_sys.begin_frame();
        raw::select(
            SelectSpec {
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                font: FontId(0),
                options: &options,
                disabled: false,
                style: crate::theme::Theme::framewise().select_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert!(!state.open);
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = SelectSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.select_style()));
        assert_eq!(builder.font, Some(theme.sans_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.select_style();
        custom_style.text_size = 99.0;
        let builder = SelectSpecBuilder::new()
            .style(custom_style)
            .font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
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
        let mut sel_state = SelectState::default();
        let result = super::select(
            &mut ctx,
            SelectSpecBuilder::new()
                .options(&[])
                .value("")
                .rect(custom_rect),
            layout_rect,
            &mut sel_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
