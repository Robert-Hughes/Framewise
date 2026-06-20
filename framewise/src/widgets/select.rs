use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    text::{layout_text, TextBackend},
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SelectSpec<'a> {
        pub layer: Layer,
        /// Bounding rect for the closed box (height h_md = 28).
        pub rect: Rect,
        pub value: &'a str,
        pub style: super::SelectStyle,
        pub items: &'a [&'a str],
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SelectCalcSizeRequestSpec<'a> {
        pub value: &'a str,
        pub style: super::SelectStyle,
        pub items: &'a [&'a str],
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SelectResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    pub fn calc_select_intrinsic_size<T: TextBackend>(
        spec: &SelectCalcSizeRequestSpec,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let s = spec.style;
        let mut widest = layout_text(
            text_backend,
            spec.value,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        )
        .metrics()
        .logical_size
        .x;
        for item in spec.items {
            let layout = layout_text(
                text_backend,
                item,
                s.text_style,
                crate::text::TextBounds::UNBOUNDED,
            );
            widest = widest.max(layout.metrics().logical_size.x);
        }
        crate::layout::SizeRequest::preferred(crate::types::Vec2::new(
            (widest + s.pad_x * 2.0 + s.chevron_right).max(s.min_width),
            s.height,
        ))
    }

    /// Low-level select widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn select<'a, T: TextBackend>(
        spec: SelectSpec<'a>,
        state: &mut SelectState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> SelectResult {
        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::all(),
                spec.disabled,
            )
        };

        if !spec.items.is_empty() {
            let current_val = if state.selected_index < spec.items.len() {
                spec.items[state.selected_index]
            } else {
                ""
            };
            if current_val != spec.value {
                // Out of band update, search for spec.value in items
                let mut found = false;
                for (i, opt) in spec.items.iter().enumerate() {
                    if *opt == spec.value {
                        state.selected_index = i;
                        found = true;
                        break;
                    }
                }
                if !found {
                    state.selected_index = state.selected_index.min(spec.items.len() - 1);
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
        if focused && !spec.disabled && !spec.items.is_empty() {
            if input.key_pressed_down {
                if state.open {
                    let current = state.hovered.unwrap_or(0);
                    if current + 1 < spec.items.len() {
                        state.hovered = Some(current + 1);
                    }
                } else {
                    if state.selected_index + 1 < spec.items.len() {
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
                    if h < spec.items.len() {
                        state.selected_index = h;
                        state.open = false;
                    }
                }
            }
        }

        // Mouse interaction with popup when open
        if state.open && !spec.disabled && !spec.items.is_empty() {
            let row_h = s.row_height;
            let popup_h = spec.items.len() as f32 * row_h + s.popup_pad_y * 2.0;
            let popup = Rect::new(r.x, r.y + s.height + s.popup_gap, r.w, popup_h);

            let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
            if is_visible && popup.contains(input.mouse_pos) {
                let relative_y = input.mouse_pos.y - (popup.y + s.popup_pad_y);
                let hovered_row = (relative_y / row_h).floor() as i32;
                if hovered_row >= 0 && hovered_row < spec.items.len() as i32 {
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

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        // Focus / open ring.
        if focused || state.open {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_focus_z(),
            });
        }

        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: r,
            color: tint(s.background),
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: r,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        // Selected value text.
        let display_text = if !spec.items.is_empty() && state.selected_index < spec.items.len() {
            spec.items[state.selected_index]
        } else {
            spec.value
        };

        let val_layout = layout_text(
            text_backend,
            display_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let val_metrics = val_layout.metrics();
        let vty = r.y + (s.height - val_metrics.logical_size.y) * 0.5;
        let val_rect = Rect::new(
            r.x + s.pad_x,
            vty,
            val_metrics.logical_size.x,
            val_metrics.logical_size.y,
        );
        val_layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(val_rect.x, val_rect.y),
            tint(s.text),
            spec.layer.get_z(),
        );

        // Chevron "v".
        let chev_color = if state.open { s.accent } else { s.muted };
        let chev_layout = layout_text(
            text_backend,
            "v",
            s.chevron_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let chev_metrics = chev_layout.metrics();
        let cty = r.y + (s.height - chev_metrics.logical_size.y) * 0.5;
        let chev_rect = Rect::new(
            r.x + r.w - s.chevron_right,
            cty,
            chev_metrics.logical_size.x,
            chev_metrics.logical_size.y,
        );
        chev_layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(chev_rect.x, chev_rect.y),
            tint(chev_color),
            spec.layer.get_z(),
        );

        // Dropdown popup.
        if state.open && !spec.items.is_empty() {
            let row_h = s.row_height;
            let popup_h = spec.items.len() as f32 * row_h + s.popup_pad_y * 2.0;
            let popup = Rect::new(r.x, r.y + s.height + s.popup_gap, r.w, popup_h);

            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: popup,
                color: tint(s.background),
                z: spec.layer.get_z(),
            });
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: popup,
                color: tint(s.border),
                width: s.border_width,
                z: spec.layer.get_z(),
            });

            for (i, opt) in spec.items.iter().enumerate() {
                let is_selected = i == state.selected_index;
                let is_hovered = state.hovered == Some(i);
                let row_y = popup.y + s.popup_pad_y + i as f32 * row_h;
                let row_rect = Rect::new(popup.x, row_y, popup.w, row_h);

                if is_selected {
                    cmds.push(DrawCmd::FillRect {
                        anti_alias: false,
                        rect: row_rect,
                        color: tint(s.selected_bg),
                        z: spec.layer.get_z(),
                    });
                } else if is_hovered {
                    cmds.push(DrawCmd::FillRect {
                        anti_alias: false,
                        rect: row_rect,
                        color: tint(s.hover),
                        z: spec.layer.get_z(),
                    });
                }

                let text_color = if is_selected { s.selected_text } else { s.text };
                let opt_layout = layout_text(
                    text_backend,
                    opt,
                    s.text_style,
                    crate::text::TextBounds::UNBOUNDED,
                );
                let opt_metrics = opt_layout.metrics();
                let oty = row_y + (row_h - opt_metrics.logical_size.y) * 0.5;
                let opt_rect = Rect::new(
                    popup.x + s.pad_x + 2.0,
                    oty,
                    opt_metrics.logical_size.x,
                    opt_metrics.logical_size.y,
                );
                opt_layout.emit_glyphs(
                    cmds,
                    text_backend,
                    Vec2::new(opt_rect.x, opt_rect.y),
                    tint(text_color),
                    spec.layer.get_z(),
                );
            }
        }

        SelectResult {
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

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectStyle {
    pub min_width: f32,
    pub height: f32,
    pub row_height: f32,
    pub popup_gap: f32,
    pub popup_pad_y: f32,
    pub pad_x: f32,
    pub chevron_right: f32,
    pub text_style: crate::text::TextStyle,
    pub chevron_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub hover: Color,
    pub muted: Color,
    pub accent: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl SelectStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            min_width: 180.0,
            height: theme.h_md,
            row_height: theme.row_height,
            popup_gap: 2.0,
            popup_pad_y: 4.0,
            pad_x: 10.0,
            chevron_right: 18.0,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            chevron_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: theme.ink,
            text: theme.ink,
            selected_bg: theme.ink,
            selected_text: theme.paper,
            hover: theme.hover,
            muted: theme.muted,
            accent: theme.rust,
            focus: theme.rust,
            border_width: theme.border,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset_tight,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SelectState {
    pub selected_index: usize,
    pub open: bool,
    pub hovered: Option<usize>,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SelectResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SelectSpec<'a> {
    pub value: &'a str,
    pub style: SelectStyle,
    pub items: &'a [&'a str],
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SelectSpecBuilder<'a> {
    pub value: Option<&'a str>,
    pub style: Option<SelectStyle>,
    pub items: Option<&'a [&'a str]>,
    pub disabled: Option<bool>,
}

impl<'a> SelectSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }
    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SelectStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SelectSpec<'a> {
        SelectSpec {
            value: self.value.expect("value not set — call .value()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            items: self.items.expect("items not set — call .items()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

pub fn select<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SelectSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut SelectState,
) -> SelectResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::SelectCalcSizeRequestSpec {
        value: spec.value,
        style: spec.style,
        items: spec.items,
    };
    let size_request = raw::calc_select_intrinsic_size(&calc_spec, ctx.text_backend);
    let rect = ctx.layout(layout_params, size_request);
    let raw_spec = raw::SelectSpec {
        layer: ctx.layer,
        rect,
        value: spec.value,
        style: spec.style,
        items: spec.items,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::select(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

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
    use crate::test_utils::TestTextBackend;
    use crate::types::Vec2;
    use crate::{DrawGlyph, PreparedGlyphToken};

    fn select_dummy<'a>(spec: SelectSpec<'a>) -> (raw::SelectResult, DrawCommands) {
        let mut cmds = DrawCommands::new();
        let mut text_backend = TestTextBackend;
        let result = raw::select(
            spec,
            &mut SelectState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_backend,
            &mut cmds,
        );
        (result, cmds)
    }

    #[test]
    fn test_select_visual_normal() {
        let items = vec!["Option 1", "Option 2", "Option 3"];
        let spec = SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let (_, cmds) = select_dummy(spec);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: s.text,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 7..8,
                    color: s.muted,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 10.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 18.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 26.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 34.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 42.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 50.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(49),
                    top_left: Vec2 { x: 66.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(118),
                    top_left: Vec2 { x: 162.0, y: 17.0 }
                }
            ]
        );
    }

    #[test]
    fn test_select_visual_open() {
        let mut text_backend = TestTextBackend;
        let items = vec!["Option 1", "Option 2", "Option 3"];
        let spec = SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;

        // Pass SelectState { open: true, ... } to simulate open state
        let state = SelectState {
            selected_index: 0,
            open: true,
            hovered: Some(1),
            space_is_active: false,
            focus_id: FocusId::new(),
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        raw::select(
            spec,
            &mut state,
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_backend,
            &mut cmds,
        );

        let r = Rect::new(0.0, 0.0, 180.0, 28.0);
        let popup = Rect::new(0.0, 30.0, 180.0, 86.0);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r.inset(-s.focus_offset),
                    color: s.focus,
                    width: s.focus_width,
                    z: 1,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..7,
                    color: s.text,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 7..8,
                    color: s.accent,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: popup,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: popup,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 34.0, 180.0, 26.0),
                    color: s.selected_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 8..15,
                    color: s.selected_text,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 60.0, 180.0, 26.0),
                    color: s.hover,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 15..22,
                    color: s.text,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 22..29,
                    color: s.text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 10.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 18.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 26.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 34.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 42.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 50.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(49),
                    top_left: Vec2 { x: 66.0, y: 19.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(118),
                    top_left: Vec2 { x: 162.0, y: 17.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 12.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 20.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 28.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 36.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 44.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 52.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(49),
                    top_left: Vec2 { x: 68.0, y: 52.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 12.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 20.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 28.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 36.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 44.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 52.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(50),
                    top_left: Vec2 { x: 68.0, y: 78.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(79),
                    top_left: Vec2 { x: 12.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(112),
                    top_left: Vec2 { x: 20.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(116),
                    top_left: Vec2 { x: 28.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(105),
                    top_left: Vec2 { x: 36.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(111),
                    top_left: Vec2 { x: 44.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(110),
                    top_left: Vec2 { x: 52.0, y: 104.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(51),
                    top_left: Vec2 { x: 68.0, y: 104.0 }
                }
            ]
        );
    }

    #[test]
    fn test_select_click_takes_focus_and_opens() {
        let mut focus_system = FocusSystem::new();
        let state = SelectState::default();
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let items = vec!["Option 1", "Option 2"];
        let spec = SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Clicking select must request focus"
        );
        assert!(state.open, "Clicking select must open the popup dropdown");
    }

    #[test]
    fn test_select_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = SelectState::default();
        let input = Input {
            mouse_pos: Vec2::new(15.0, 15.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let items = vec!["Option 1", "Option 2"];
        let spec = SelectSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 180.0, 28.0),
            value: "Option 1",
            items: &items,
            disabled: false,
            style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 180.0, 28.0)),
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "Clicking a clipped-away select must not take focus"
        );
    }

    #[test]
    fn test_select_keyboard_navigation() {
        let mut focus_system = FocusSystem::new();
        let mut state = SelectState::default();
        let mut input = Input::default();
        let mut text_backend = TestTextBackend;
        let items = vec!["Option 1", "Option 2", "Option 3"];

        // Focus the widget first
        focus_system.take_keyboard_focus(state.focus_id);

        // Frame 1: Press Arrow Down while closed -> selected index changes to 1
        input.key_pressed_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            SelectSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 1",
                items: &items,
                disabled: false,
                style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_pressed_down = false;

        assert_eq!(state.selected_index, 1);
        assert!(!state.open);

        // Frame 2: Press Space -> opens dropdown
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            SelectSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                items: &items,
                disabled: false,
                style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            SelectSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                items: &items,
                disabled: false,
                style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_released_space = false;

        assert!(state.open);
        assert_eq!(state.hovered, Some(1));

        // Frame 3: Press Arrow Down while open -> hovers index 2
        input.key_pressed_down = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            SelectSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                items: &items,
                disabled: false,
                style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_pressed_down = false;

        assert_eq!(state.hovered, Some(2));

        // Frame 4: Press Enter while open -> selects hovered (index 2) and closes dropdown
        input.key_pressed_enter = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::select(
            SelectSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 180.0, 28.0),
                value: "Option 2",
                items: &items,
                disabled: false,
                style: SelectStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert!(!state.open);
        assert_eq!(state.selected_index, 2);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = SelectSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(SelectStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = SelectStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = SelectSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layouts::ManualLayout;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut sel_state = SelectState::default();
        let result = super::select(
            &mut ctx,
            SelectSpecBuilder::new().items(&[]).value(""),
            custom_rect,
            &mut sel_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
