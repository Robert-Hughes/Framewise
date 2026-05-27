use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    types::{Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioSpec {
        /// Top-left of the 14×14 bounding area.
        pub rect: Rect,
        pub selected: bool,
        pub disabled: bool,
        pub style: super::RadioStyle,
        pub clip_rect: Option<Rect>,
    }

    /// Low-level radio widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn radio(
        mut state: RadioState,
        spec: RadioSpec,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
    ) -> RadioResult {
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

        // Keep state.selected in sync with spec.selected
        if state.selected != spec.selected {
            state.selected = spec.selected;
        }

        if is_clicked {
            state.selected = true;
        }

        let mut cmds = DrawCommands::new();
        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let cx = spec.rect.x + s.radius;
        let cy = spec.rect.y + s.radius;
        let center = Vec2::new(cx, cy);

        let visually_focused = focused;

        // Focus ring (outset 2px).
        if visually_focused {
            cmds.push(DrawCmd::StrokeCircle {
                center,
                radius: s.radius + s.focus_offset,
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        // Background fill.
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: s.radius,
            color: tint(s.background),
        });

        // Outer ring.
        cmds.push(DrawCmd::StrokeCircle {
            center,
            radius: s.radius,
            color: tint(s.border),
            width: s.border_width,
        });

        // Inner dot when selected.
        if state.selected {
            cmds.push(DrawCmd::FillCircle {
                center,
                radius: s.dot_radius,
                color: tint(s.dot),
            });
        }

        RadioResult {
            draw: cmds,
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

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub state: RadioState,
        pub focused: bool,
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioState {
    pub selected: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioStyle {
    pub radius: f32,
    pub dot_radius: f32,
    pub background: Color,
    pub border: Color,
    pub dot: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RadioSpecBuilder {
    pub selected: bool,
    pub disabled: bool,
    pub style: Option<RadioStyle>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}

impl RadioSpecBuilder {
    pub fn new() -> Self {
        Self {
            selected: false,
            disabled: false,
            style: None,
            rect: None,
            clip_rect: None,
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
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
            self.style = Some(theme.radio_style());
        }
        self
    }

    pub fn build(self) -> raw::RadioSpec {
        raw::RadioSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            selected: self.selected,
            disabled: self.disabled,
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            clip_rect: self.clip_rect,
        }
    }
}

pub struct RadioResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: RadioState,
    pub focused: bool,
}

impl RadioResult {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn selected(&self) -> bool {
        self.state.selected
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level radio widget function using WidgetContext.
///
/// This function accepts a RadioSpec and calls the low-level raw::radio function.
pub fn radio<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    state: RadioState,
    layout_params: S::Params,
    builder: RadioSpecBuilder,
) -> RadioResult {
    let rect = ctx.layout(layout_params);
    let clip = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::radio(state, spec, ctx.input, ctx.focus_sys);

    ctx.append_cmds(result.draw.0);

    RadioResult {
        layout: LayoutInfo::tight(rect),
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::raw::RadioSpec;

    fn rad_io(spec: RadioSpec) -> raw::RadioResult {
        raw::radio(
            RadioState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_radio_visual_unselected() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = rad_io(spec);
        let center = Vec2::new(17.0, 17.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_selected() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: true,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = rad_io(spec);
        let center = Vec2::new(17.0, 17.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.dot_radius,
                    color: s.dot,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_focused() {
        let state = RadioState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = raw::radio(state, spec, &Input::default(), &mut focus_sys);
        focus_sys.end_frame();
        let center = Vec2::new(17.0, 17.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius + s.focus_offset,
                    color: s.focus,
                    width: s.focus_width,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_disabled() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: true,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = rad_io(spec);
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let center = Vec2::new(17.0, 17.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.background),
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.border),
                    width: s.border_width,
                },
            ])
        );
    }

    #[test]
    fn test_radio_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = RadioState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = raw::radio(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking radio must request focus"
        );
    }

    #[test]
    fn test_radio_clipped_click_does_not_take_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = RadioState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
        };

        focus_sys.begin_frame();
        raw::radio(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away radio must not take focus"
        );
    }

    #[test]
    fn test_radio_keyboard_toggle() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = RadioState::default();
        let mut input = Input::default();

        let spec = || RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            disabled: false,
            style: crate::theme::Theme::framewise().radio_style(),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the radio
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let res = raw::radio(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        let res = raw::radio(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        let res = raw::radio(state, spec(), &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            res.state.selected, true,
            "Spacebar release must toggle radio state to selected"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = RadioSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.radio_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.radio_style();
        custom_style.radius = 99.0;
        let builder = RadioSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().radius, 99.0);
    }
}
