use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    types::{Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level checkbox widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn checkbox(
        mut state: CheckboxState,
        spec: CheckboxSpec,
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

        // Keep state.check in sync with spec.state if spec.state changed out of band.
        if state.check != spec.state {
            state.check = spec.state;
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckState {
    Off,
    On,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxSpec {
    /// Top-left of the 14×14 box.
    pub rect: Rect,
    pub state: CheckState,
    pub disabled: bool,
    pub style: CheckboxStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct CheckboxState {
    pub check: CheckState,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

impl Default for CheckboxState {
    fn default() -> Self {
        Self {
            check: CheckState::Off,
            is_active: false,
            space_is_active: false,
            focus_id: crate::focus::FocusId::new(),
        }
    }
}

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

impl Default for CheckboxStyle {
    fn default() -> Self {
        Self {
            size: 14.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            selected_fill: Color::from_srgb_u8(21, 19, 15, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            mark: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.5,
            mark_width: 1.5,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CheckboxSpecBuilder {
    pub state: CheckState,
    pub disabled: bool,
    pub style: Option<CheckboxStyle>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}
impl CheckboxSpecBuilder {
    pub fn new(state: CheckState) -> Self {
        Self {
            state,
            disabled: false,
            style: None,
            rect: None,
            clip_rect: None,
        }
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: CheckboxStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Overrides the clip rectangle. High-level context functions supply this from
    /// the surrounding clip region — only needed when using the raw API directly, or
    /// to clip tighter than the context default.
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
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

    pub fn build(self) -> CheckboxSpec {
        CheckboxSpec {
            rect: self.rect.unwrap_or_default(),
            state: self.state,
            disabled: self.disabled,
            style: self.style.unwrap_or_default(),
            clip_rect: self.clip_rect,
        }
    }
}

pub struct CheckboxResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: CheckboxState,
    pub focused: bool,
}

pub struct CheckboxInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: CheckboxState,
    pub focused: bool,
}

impl CheckboxInfo {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn state(&self) -> CheckState {
        self.state.check
    }
}

impl CheckboxResult {
    pub fn into_parts(self) -> (DrawCommands, CheckboxInfo) {
        (
            self.draw,
            CheckboxInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level checkbox widget function using WidgetContext.
///
/// This function accepts a CheckboxSpec and calls the low-level raw::checkbox function.
pub fn checkbox<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    state: CheckboxState,
    layout_params: S::Params,
    builder: CheckboxSpecBuilder,
) -> CheckboxInfo {
    let rect = ctx.layout(layout_params);
    let clip = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::checkbox(state, spec, ctx.input, ctx.focus_sys);

    ctx.append_cmds(result.draw.0);

    CheckboxInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn checkbox_dummy(spec: CheckboxSpec) -> CheckboxResult {
        raw::checkbox(
            CheckboxState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_checkbox_visual_off() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Off,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec);
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
            state: CheckState::On,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec);
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
            state: CheckState::Indeterminate,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec);
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
            state: CheckState::Off,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let res = raw::checkbox(state, spec, &Input::default(), &mut focus_sys);
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
            state: CheckState::Off,
            disabled: true,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = checkbox_dummy(spec);
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
            state: CheckState::Off,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = raw::checkbox(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking checkbox must request focus"
        );
    }

    #[test]
    fn test_checkbox_keyboard_toggle() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = CheckboxState::default();
        let mut input = Input::default();

        let spec = || CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Off,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the checkbox
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let res = raw::checkbox(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        let res = raw::checkbox(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        let res = raw::checkbox(state, spec(), &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            res.state.check,
            CheckState::On,
            "Spacebar release must toggle checkbox state"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = CheckboxSpecBuilder::new(CheckState::Off);
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.checkbox_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.checkbox_style();
        custom_style.size = 99.0;
        let builder = CheckboxSpecBuilder::new(CheckState::Off).style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().size, 99.0);
    }
}
