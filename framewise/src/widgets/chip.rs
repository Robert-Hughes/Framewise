use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level chip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn chip<'a, T: crate::text::TextSystem>(
        mut state: ChipState,
        spec: ChipSpec<'a>,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_sys: &mut T,
    ) -> ChipResult {
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

        let mut is_clicked = clicked;
        if focused && input.key_pressed_enter {
            is_clicked = true;
        }
        if state.space_is_active && input.key_released_space {
            is_clicked = true;
        }

        // Update space activation state for keyboard space press
        if !focused || !input.key_down_space {
            state.space_is_active = false;
        }
        if focused && input.key_pressed_space {
            state.space_is_active = true;
        }

        if is_clicked {
            state.active = !state.active;
        }

        let mut cmds = DrawCommands::new();
        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let h = s.height;
        let pad_x = s.pad_x;

        let layout = text_sys.prepare(spec.label, s.text_size, spec.font);
        let w = spec.rect.w.max(32.0);
        let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

        let visually_focused = focused;

        // Focus ring.
        if visually_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        let bg = if state.active {
            s.active_bg
        } else {
            s.background
        };
        cmds.push(DrawCmd::FillRect {
            rect: r,
            color: tint(bg),
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: r,
            color: tint(s.border),
            width: s.border_width,
        });

        let text_color = if state.active { s.active_text } else { s.text };
        let ty = r.y + (h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(r.x + pad_x, ty, layout.size.x, layout.size.y),
            color: tint(text_color),
            handle: layout.handle,
        });

        ChipResult {
            draw: cmds,
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(s.border_width)),
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: (clicked && input.mouse_down) || state.space_is_active,
                clicked: is_clicked,
            },
            state,
            focused,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChipSpec<'a> {
    /// Top-left origin. Height is fixed at 22.
    pub rect: Rect,
    pub label: &'a str,
    pub font: FontId,
    pub disabled: bool,
    pub style: ChipStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone, Default)]
pub struct ChipState {
    pub active: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_size: f32,
    pub background: Color,
    pub active_bg: Color,
    pub border: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

pub struct ChipResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: ChipState,
    pub focused: bool,
}

pub struct ChipInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: ChipState,
    pub focused: bool,
}

impl ChipInfo {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn active(&self) -> bool {
        self.state.active
    }
}

impl ChipResult {
    pub fn into_parts(self) -> (DrawCommands, ChipInfo) {
        (
            self.draw,
            ChipInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level chip widget function using WidgetContext.
///
/// This function accepts a ChipSpec and calls the low-level raw::chip function.
pub fn chip<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    state: ChipState,
    layout_params: S::Params,
    builder: ChipSpecBuilder<'a>,
) -> ChipInfo {
    let rect = ctx.layout(layout_params);
    let clip = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::chip(state, spec, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw.0);

    ChipInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ChipSpecBuilder<'a> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<ChipStyle>,
    pub disabled: bool,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}

impl<'a> ChipSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            disabled: false,
            rect: None,
            clip_rect: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: ChipStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    /// Overrides the clip rectangle. High-level context functions supply this from
    /// the surrounding clip region — only needed when using the raw API directly, or
    /// to clip tighter than the context default.
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
}

impl<'a> ChipSpecBuilder<'a> {
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
            self.style = Some(theme.chip_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> ChipSpec<'a> {
        ChipSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            label: self.label.expect("label not set — call .label()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled,
            clip_rect: self.clip_rect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

    fn chip_raw<'a>(spec: ChipSpec<'a>) -> ChipResult {
        raw::chip(
            ChipState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_chip_visual_normal() {
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            disabled: false,
            style: crate::theme::Theme::framewise().chip_style(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = chip_raw(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_chip_visual_active() {
        let mut text_sys = DummyTextSys;
        let mut state = ChipState::default();
        state.active = true;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            disabled: false,
            style: crate::theme::Theme::framewise().chip_style(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = raw::chip(
            state,
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut text_sys,
        );

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_chip_visual_focused() {
        let state = ChipState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            disabled: false,
            style: crate::theme::Theme::framewise().chip_style(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = raw::chip(
            state,
            spec,
            &Input::default(),
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        let r = Rect::new(0.0, 0.0, 50.0, 22.0);
        let expected_focus_rect = r.inset(-style.focus_offset);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: expected_focus_rect,
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::FillRect {
                    rect: r,
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_chip_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = ChipState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            disabled: false,
            style: crate::theme::Theme::framewise().chip_style(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = raw::chip(state, spec, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking chip must request focus"
        );
    }

    #[test]
    fn test_chip_keyboard_toggle() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = ChipState::default();
        let mut input = Input::default();
        let mut text_sys = DummyTextSys;

        // Frame 1: Focus chip
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let res = raw::chip(
            state,
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                label: "Tag",
                font: FontId(0),
                disabled: false,
                style: crate::theme::Theme::framewise().chip_style(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        state = res.state;
        focus_sys.end_frame();

        // Frame 2: Press Space
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        let res = raw::chip(
            state,
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                label: "Tag",
                font: FontId(0),
                disabled: false,
                style: crate::theme::Theme::framewise().chip_style(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        state = res.state;
        focus_sys.end_frame();

        // Frame 3: Release Space
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        let res = raw::chip(
            state,
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                label: "Tag",
                font: FontId(0),
                disabled: false,
                style: crate::theme::Theme::framewise().chip_style(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert!(res.state.active, "Spacebar release must toggle chip state");
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = ChipSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.chip_style()));
        assert_eq!(builder.font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.chip_style();
        custom_style.text_size = 99.0;
        let builder = ChipSpecBuilder::new().style(custom_style).font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }
}
