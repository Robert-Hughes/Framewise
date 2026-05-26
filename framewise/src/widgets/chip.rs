use crate::{
    draw::{DrawCmd, DrawCommands},
    input::Input,
    text::FontId,
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext, WidgetScope},
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

impl Default for ChipStyle {
    fn default() -> Self {
        Self {
            height: 22.0,
            pad_x: 8.0,
            text_size: 11.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            active_bg: Color::from_srgb_u8(21, 19, 15, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            active_text: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
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
pub fn chip<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState, Scope: WidgetScope>(
    ctx: &mut WidgetContext<T, S, Scope>,
    state: ChipState,
    layout_params: S::Params,
    builder: ChipSpecBuilder<'a>,
) -> ChipInfo {
    let rect = ctx.layout(layout_params);
    let clip = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder.rect(rect).apply_theme(&ctx.theme).clip_rect(clip).build();
    let result = raw::chip(state, spec, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw.0);

    ChipInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

pub struct ChipSpecBuilder<'a> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<ChipStyle>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}

impl<'a> Default for ChipSpecBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> ChipSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            disabled: None,
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
        self.disabled = Some(disabled);
        self
    }
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
}

impl<'a> ChipSpecBuilder<'a> {
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn apply_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = Some(theme.chip_style());
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> ChipSpec<'a> {
        ChipSpec {
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self
                .font
                .expect("font must be specified or resolved from a theme"),
            style: self.style.expect("ChipStyle is required"),
            disabled: self.disabled.unwrap_or(false),
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
            style: Default::default(),
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
            style: Default::default(),
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
            style: Default::default(),
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
            style: Default::default(),
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
                style: Default::default(),
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
                style: Default::default(),
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
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert!(res.state.active, "Spacebar release must toggle chip state");
    }
}
