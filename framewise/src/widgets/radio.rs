use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    input::Input,
};

pub mod raw {
    use super::*;

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
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(s.border_width)),
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos) && spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos)),
                pressed: (clicked && input.mouse_down) || state.space_is_active,
                clicked: is_clicked,
            },
            state,
            focused,
        }
    }
}

pub struct RadioSpec {
    /// Top-left of the 14×14 bounding area.
    pub rect: Rect,
    pub selected: bool,
    pub disabled: bool,
    pub style: RadioStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct RadioState {
    pub selected: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

impl Default for RadioState {
    fn default() -> Self {
        Self {
            selected: false,
            is_active: false,
            space_is_active: false,
            focus_id: crate::focus::FocusId::new(),
        }
    }
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

impl Default for RadioStyle {
    fn default() -> Self {
        Self {
            radius: 7.0,
            dot_radius: 3.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            dot: Color::from_srgb_u8(21, 19, 15, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.5,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}


pub struct RadioSpecBuilder {
    spec: RadioSpec,
}
impl RadioSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: RadioSpec {
                rect: Rect::ZERO,
                selected: false,
                disabled: false,
                style: RadioStyle {
                    radius: 7.0,
                    dot_radius: 3.0,
                    background: Color::WHITE,
                    border: Color::BLACK,
                    dot: Color::BLACK,
                    focus: Color::BLACK,
                    border_width: 1.5,
                    focus_width: 2.0,
                    focus_offset: 2.0,
                    disabled_alpha: 0.35,
                },
                clip_rect: None,
            },
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.spec.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.spec.disabled = disabled;
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.spec.style = style;
        self
    }

    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.spec.clip_rect = clip_rect;
        self
    }

    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.spec.style = theme.radio_style();
        self
    }

    pub fn build(self) -> RadioSpec {
        self.spec
    }
}

pub struct RadioResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: RadioState,
    pub focused: bool,
}

pub struct RadioInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: RadioState,
    pub focused: bool,
}

impl RadioInfo {
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

impl RadioResult {
    pub fn into_parts(self) -> (DrawCommands, RadioInfo) {
        (
            self.draw,
            RadioInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level radio widget function using WidgetContext.
///
/// This function accepts a RadioSpec and calls the low-level raw::radio function.
pub fn radio<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: RadioState,
    layout_params: S::Params,
    builder: RadioSpecBuilder,
) -> RadioInfo {
    let rect = ctx.layout(layout_params);
    let mut builder = builder
        .with_rect(rect)
        .with_theme(&ctx.theme);
    if builder.spec.clip_rect.is_none() {
        builder.spec.clip_rect = ctx.clip_rect;
    }
    let spec = builder.build();
    let result = raw::radio(state, spec, ctx.input, ctx.focus_sys);

    ctx.append_cmds(result.draw.0);

    RadioInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

// ── Re-export raw function for direct use ───────────────────────────────────────────
pub use raw::radio as radio_raw;

#[cfg(test)]
mod tests {
    use super::*;

    fn rad_io(spec: RadioSpec) -> RadioResult {
        radio_raw(
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
            style: Default::default(),
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
            style: Default::default(),
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
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = radio_raw(state, spec, &Input::default(), &mut focus_sys);
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
            style: Default::default(),
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
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = radio_raw(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking radio must request focus"
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
            style: Default::default(),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the radio
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let res = radio_raw(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        let res = radio_raw(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        let res = radio_raw(state, spec(), &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            res.state.selected,
            true,
            "Spacebar release must toggle radio state to selected"
        );
    }
}
