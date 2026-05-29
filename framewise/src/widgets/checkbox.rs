use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    types::{ClipRect, Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxSpec {
        /// Top-left of the 14×14 box.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::CheckboxStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level checkbox widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn checkbox(
        spec: CheckboxSpec,
        state: &mut CheckboxState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
    ) -> CheckboxResult {
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
            state.check = match state.check {
                CheckState::Off => CheckState::On,
                CheckState::On => CheckState::Off,
                CheckState::Indeterminate => CheckState::On,
            };
        }

        let mut cmds = DrawCommands::new();
        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(spec.rect.x, spec.rect.y, s.size, s.size);

        let visually_focused = focused;

        // Focus ring (outset 2px).
        if visually_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        // Box fill.
        let fill = match state.check {
            CheckState::Off => s.background,
            _ => s.selected_fill,
        };
        cmds.push(DrawCmd::FillRect {
            rect: r,
            color: tint(fill),
        });

        // Box border.
        cmds.push(DrawCmd::StrokeRect {
            rect: r,
            color: tint(s.border),
            width: s.border_width,
        });

        // Inner mark.
        match state.check {
            CheckState::On => {
                // Checkmark: two lines forming a tick (√).
                let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
                let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
                let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
                let mark = tint(s.mark);
                cmds.push(DrawCmd::StrokeLine {
                    p0,
                    p1,
                    color: mark,
                    width: s.mark_width,
                });
                cmds.push(DrawCmd::StrokeLine {
                    p0: p1,
                    p1: p2,
                    color: mark,
                    width: s.mark_width,
                });
            }
            CheckState::Indeterminate => {
                // Horizontal dash.
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: tint(s.mark),
                });
            }
            CheckState::Off => {}
        }

        CheckboxResult {
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
pub struct CheckboxStyle {
    pub size: f32,
    pub background: Color,
    pub selected_fill: Color,
    pub border: Color,
    pub mark: Color,
    pub focus: Color,
    pub border_width: f32,
    pub mark_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CheckState {
    #[default]
    Off,
    On,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckboxState {
    pub check: CheckState,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CheckboxSpecBuilder {
    pub disabled: Option<bool>,
    pub style: Option<CheckboxStyle>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<ClipRect>,
}
impl CheckboxSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = Some(style);
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
            self.style = Some(theme.checkbox_style());
        }
        self
    }

    pub fn build(self) -> raw::CheckboxSpec {
        raw::CheckboxSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            disabled: self.disabled.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level checkbox widget function using WidgetContext.
///
/// This function accepts a CheckboxSpec and calls the low-level raw::checkbox function.
pub fn checkbox<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: CheckboxSpecBuilder,
    layout_params: S::Params,
    state: &mut CheckboxState,
) -> CheckboxResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::checkbox(spec, state, ctx.input, ctx.focus_sys);

    ctx.append_cmds(result.draw);

    CheckboxResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::CheckboxSpec;
    use super::*;

    fn checkbox_dummy(spec: CheckboxSpec, state: CheckState) -> raw::CheckboxResult {
        raw::checkbox(
            spec,
            &mut CheckboxState {
                check: state,
                ..Default::default()
            },
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_checkbox_visual_off() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec, CheckState::Off);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                    color: s.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 14.0, 14.0),
                    color: s.border,
                    width: s.border_width,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_on() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec, CheckState::On);
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
        let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
        let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.selected_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::StrokeLine {
                    p0,
                    p1,
                    color: s.mark,
                    width: s.mark_width,
                },
                DrawCmd::StrokeLine {
                    p0: p1,
                    p1: p2,
                    color: s.mark,
                    width: s.mark_width,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_indeterminate() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec, CheckState::Indeterminate);
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.selected_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: s.mark,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_focused() {
        let state = CheckboxState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut state = state;
        let res = raw::checkbox(spec, &mut state, &Input::default(), &mut focus_sys);
        focus_sys.end_frame();
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: r.inset(-s.focus_offset),
                    color: s.focus,
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
            ])
        );
    }

    #[test]
    fn test_checkbox_visual_disabled() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: true,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec, CheckState::Off);
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: tint(s.background),
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: tint(s.border),
                    width: s.border_width,
                },
            ])
        );
    }

    #[test]
    fn test_checkbox_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = CheckboxState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::checkbox(spec, &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking checkbox must request focus"
        );
    }

    #[test]
    fn test_checkbox_clipped_click_does_not_take_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = CheckboxState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::checkbox(spec, &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away checkbox must not take focus"
        );
    }

    #[test]
    fn test_checkbox_keyboard_toggle() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = CheckboxState::default();
        let mut input = Input::default();

        let spec = || CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: crate::theme::Theme::framewise().checkbox_style(),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the checkbox
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        raw::checkbox(spec(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        raw::checkbox(spec(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        raw::checkbox(spec(), &mut state, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            state.check,
            CheckState::On,
            "Spacebar release must toggle checkbox state"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = CheckboxSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.checkbox_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.checkbox_style();
        custom_style.size = 99.0;
        let builder = CheckboxSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
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
        let mut cb_state = CheckboxState::default();
        let result = super::checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new().rect(custom_rect),
            layout_rect,
            &mut cb_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
