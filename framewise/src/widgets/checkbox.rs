use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{IntrinsicSize, LayoutState},
    types::{ClipRect, Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextSystem,
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxSpec {
        /// Top-left of the 14x14 box.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::CheckboxStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct CheckboxResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Compute intrinsic size for Checkbox. Currently returns UNKNOWN.
    pub fn calc_checkbox_intrinsic_size(_spec: &CheckboxSpec) -> IntrinsicSize {
        IntrinsicSize::UNKNOWN
    }

    /// Low-level checkbox widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn checkbox(
        spec: CheckboxSpec,
        state: &mut CheckboxState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> CheckboxResult {
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
            state.checked = match state.checked {
                CheckedState::Unchecked => CheckedState::Checked,
                CheckedState::Checked => CheckedState::Unchecked,
                CheckedState::Indeterminate => CheckedState::Checked,
            };
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(spec.rect.x, spec.rect.y, s.size, s.size);

        // Focus ring (outset 2px).
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        // Box fill.
        let fill = match state.checked {
            CheckedState::Unchecked => s.background,
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
        match state.checked {
            CheckedState::Checked => {
                // Checkmark: two lines forming a tick (v).
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
            CheckedState::Indeterminate => {
                // Horizontal dash.
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                    color: tint(s.mark),
                });
            }
            CheckedState::Unchecked => {}
        }

        CheckboxResult {
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

impl CheckboxStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            size: 14.0,
            background: theme.paper_elev,
            selected_fill: theme.ink,
            border: theme.ink,
            mark: theme.paper,
            focus: theme.rust,
            border_width: 1.0,
            mark_width: 1.5,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum CheckedState {
    #[default]
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct CheckboxState {
    pub checked: CheckedState,
    pub space_is_active: bool,
    pub focus_id: FocusId,
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
    pub rect: Option<Rect>,
    pub disabled: Option<bool>,
    pub style: Option<CheckboxStyle>,
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
            self.style = Some(CheckboxStyle::from_theme(theme));
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
/// This function accepts a CheckboxSpecBuilder and calls the low-level raw::checkbox function.
pub fn checkbox<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: CheckboxSpecBuilder,
    layout_params: S::Params,
    state: &mut CheckboxState,
) -> CheckboxResult {
    // Build a provisional spec with a placeholder rect to compute intrinsic size.
    // Any `rect` set on the builder is ignored by the high-level path — placement
    // is the layout's job (use `ManualLayout`, or the raw fn, for explicit rects).
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let mut spec = builder
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .rect(Rect::PLACEHOLDER)
        .build();
    let intrinsic = raw::calc_checkbox_intrinsic_size(&spec);
    let rect = ctx.layout(layout_params, intrinsic);
    spec.rect = rect;
    let result = raw::checkbox(spec, state, ctx.input, ctx.focus_system, ctx.cmds);

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

    #[test]
    fn test_checkbox_visual_off() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
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
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
        let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
        let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState {
                checked: CheckedState::Checked,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
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
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState {
                checked: CheckedState::Indeterminate,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
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
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut state = state;
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut cmds,
        );
        focus_system.end_frame();
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
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
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 14.0, 14.0);
        let mut cmds = DrawCommands::new();
        raw::checkbox(
            spec,
            &mut CheckboxState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut cmds,
        );
        assert_eq!(
            cmds,
            DrawCommands::from_vec(vec![
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
        let mut focus_system = FocusSystem::new();
        let state = CheckboxState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::checkbox(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking checkbox must request focus"
        );
    }

    #[test]
    fn test_checkbox_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = CheckboxState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::checkbox(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away checkbox must not take focus"
        );
    }

    #[test]
    fn test_checkbox_keyboard_toggle() {
        let mut focus_system = FocusSystem::new();
        let mut state = CheckboxState::default();
        let mut input = Input::default();

        let spec = || CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: CheckboxStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the checkbox
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::checkbox(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        raw::checkbox(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        raw::checkbox(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            state.checked,
            CheckedState::Checked,
            "Spacebar release must toggle checkbox state"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = CheckboxSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(CheckboxStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = CheckboxStyle::from_theme(&theme);
        custom_style.size = 99.0;
        let builder = CheckboxSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().size, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut cb_state = CheckboxState::default();
        let result = super::checkbox(
            &mut ctx,
            CheckboxSpecBuilder::new(),
            placement,
            &mut cb_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
