use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    text::FontId,
    types::{ClipRect, Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextSystem,
};

pub mod raw {
    use crate::TextSystem;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipSpec<'a> {
        /// Top-left origin. Height is fixed at 22.
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ChipStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level chip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn chip<'a, T: TextSystem>(
        spec: ChipSpec<'a>,
        state: &mut ChipState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
    ) -> ChipResult {
        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                input,
                focus_system,
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

        let layout = text_system.prepare(spec.text, s.text_size, spec.style.font);
        let w = spec.rect.w.max(32.0);
        let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

        // Focus ring.
        if focused {
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
pub struct ChipStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_size: f32,
    pub font: FontId,
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

impl ChipStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_sm,
            pad_x: 8.0,
            text_size: theme.text_sm,
            font: theme.mono_font,
            background: theme.paper_elev,
            active_bg: theme.ink,
            border: theme.ink,
            text: theme.ink,
            active_text: theme.paper,
            focus: theme.rust,
            border_width: theme.border,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChipState {
    pub active: bool, // toggle state
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ChipResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChipSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub text: Option<&'a str>,
    pub style: Option<ChipStyle>,
    pub disabled: Option<bool>,
    pub clip_rect: Option<ClipRect>,
}

impl<'a> ChipSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
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
            self.style = Some(ChipStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> raw::ChipSpec<'a> {
        raw::ChipSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level chip widget function using WidgetContext.
///
/// This function accepts a ChipSpecBuilder and calls the low-level raw::chip function.
pub fn chip<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ChipSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut ChipState,
) -> ChipResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::chip(spec, state, ctx.input, ctx.focus_system, ctx.text_system);

    ctx.append_cmds(result.draw);

    ChipResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::ChipSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

    fn chip_raw<'a>(spec: ChipSpec<'a>) -> raw::ChipResult {
        raw::chip(
            spec,
            &mut ChipState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_chip_visual_normal() {
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
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
        let mut text_system = DummyTextSys;
        let mut state = ChipState::default();
        state.active = true;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let mut state = state;
        let res = raw::chip(
            spec,
            &mut state,
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_system,
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
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut text_system = DummyTextSys;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let mut state = state;
        let res = raw::chip(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

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
        let mut focus_system = FocusSystem::new();
        let state = ChipState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        focus_system.begin_frame();
        raw::chip(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking chip must request focus"
        );
    }

    #[test]
    fn test_chip_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = ChipState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(10.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let spec = ChipSpec {
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 50.0, 22.0)),
        };

        let mut state = state;
        focus_system.begin_frame();
        raw::chip(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away chip must not take focus"
        );
    }

    #[test]
    fn test_chip_keyboard_toggle() {
        let mut focus_system = FocusSystem::new();
        let mut state = ChipState::default();
        let mut input = Input::default();
        let mut text_system = DummyTextSys;

        // Frame 1: Focus chip
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        raw::chip(
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        // Frame 2: Press Space
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        raw::chip(
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        // Frame 3: Release Space
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        raw::chip(
            ChipSpec {
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert!(state.active, "Spacebar release must toggle chip state");
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = ChipSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(ChipStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = ChipStyle::from_theme(&theme);
        custom_style.text_size = 99.0;
        let builder = ChipSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let mut chip_state = ChipState::default();
        let result = super::chip(
            &mut ctx,
            ChipSpecBuilder::new().text("X").rect(custom_rect),
            layout_rect,
            &mut chip_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
