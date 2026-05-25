use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext, WidgetScope},
    input::Input,
};

pub mod raw {
    use super::*;

    /// Low-level switch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn switch(
        mut state: SwitchState,
        spec: SwitchSpec,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
    ) -> SwitchResult {
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

        // Keep state.on in sync with spec.on if spec.on changed out of band.
        if state.on != spec.on {
            state.on = spec.on;
        }

        if is_clicked {
            state.on = !state.on;
        }

        let mut cmds = DrawCommands::new();
        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(spec.rect.x, spec.rect.y, s.size.0, s.size.1);

        let visually_focused = focused;

        // Focus ring.
        if visually_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        // Track fill.
        let track_fill = if state.on { s.on_fill } else { s.off_fill };
        cmds.push(DrawCmd::FillRect {
            rect: r,
            color: tint(track_fill),
        });

        // Track border.
        cmds.push(DrawCmd::StrokeRect {
            rect: r,
            color: tint(s.border),
            width: s.border_width,
        });

        // Thumb dot (10×10, vertically centered, left/right positioned).
        let dot_y = r.y + (r.h - s.thumb_size) * 0.5;
        let dot_x = if state.on {
            r.x + r.w - s.thumb_size - s.border_width
        } else {
            r.x + s.border_width
        };
        let dot_color = if state.on { s.on_thumb } else { s.off_thumb };
        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(dot_x, dot_y, s.thumb_size, s.thumb_size),
            color: tint(dot_color),
        });

        SwitchResult {
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

pub struct SwitchSpec {
    /// Top-left of the 30×16 bounding area.
    pub rect: Rect,
    pub on: bool,
    pub disabled: bool,
    pub style: SwitchStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone)]
pub struct SwitchState {
    pub on: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: crate::focus::FocusId,
}

impl Default for SwitchState {
    fn default() -> Self {
        Self {
            on: false,
            is_active: false,
            space_is_active: false,
            focus_id: crate::focus::FocusId::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchStyle {
    pub size: (f32, f32),
    pub thumb_size: f32,
    pub off_fill: Color,
    pub on_fill: Color,
    pub border: Color,
    pub off_thumb: Color,
    pub on_thumb: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl Default for SwitchStyle {
    fn default() -> Self {
        Self {
            size: (30.0, 16.0),
            thumb_size: 10.0,
            off_fill: Color::from_srgb_u8(251, 249, 244, 255),
            on_fill: Color::from_srgb_u8(21, 19, 15, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            off_thumb: Color::from_srgb_u8(21, 19, 15, 255),
            on_thumb: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.5,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}


pub struct SwitchSpecBuilder {
    spec: SwitchSpec,
}

impl SwitchSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: SwitchSpec {
                rect: Rect::ZERO,
                on: false,
                disabled: false,
                style: SwitchStyle {
                    size: (30.0, 16.0),
                    thumb_size: 10.0,
                    off_fill: Color::WHITE,
                    on_fill: Color::BLACK,
                    border: Color::BLACK,
                    off_thumb: Color::BLACK,
                    on_thumb: Color::WHITE,
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

    pub fn on(mut self, on: bool) -> Self {
        self.spec.on = on;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.spec.disabled = disabled;
        self
    }

    pub fn style(mut self, style: SwitchStyle) -> Self {
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
        self.spec.style = theme.switch_style();
        self
    }

    pub fn build(self) -> SwitchSpec {
        self.spec
    }
}

pub struct SwitchResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: SwitchState,
    pub focused: bool,
}

pub struct SwitchInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: SwitchState,
    pub focused: bool,
}

impl SwitchInfo {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn on(&self) -> bool {
        self.state.on
    }
}

impl SwitchResult {
    pub fn into_parts(self) -> (DrawCommands, SwitchInfo) {
        (
            self.draw,
            SwitchInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level switch widget function using WidgetContext.
///
/// This function accepts a SwitchSpec and calls the low-level raw::switch function.
pub fn switch<T: crate::text::TextSystem, S: crate::layout::LayoutState, Scope: WidgetScope>(
    ctx: &mut WidgetContext<T, S, Scope>,
    state: SwitchState,
    layout_params: S::Params,
    builder: SwitchSpecBuilder,
) -> SwitchInfo {
    let rect = ctx.layout(layout_params);
    let mut builder = builder
        .with_rect(rect)
        .with_theme(&ctx.theme);
    if builder.spec.clip_rect.is_none() {
        builder.spec.clip_rect = ctx.clip_rect;
    }
    let spec = builder.build();
    let result = raw::switch(state, spec, ctx.input, ctx.focus_sys);

    ctx.append_cmds(result.draw.0);

    SwitchInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Vec2;

    fn swi_tch(spec: SwitchSpec) -> SwitchResult {
        raw::switch(
            SwitchState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_switch_visual_off() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = swi_tch(spec);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_on() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: true,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = swi_tch(spec);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.on_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(28.5, 13.0, 10.0, 10.0),
                    color: s.on_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_focused() {
        let state = SwitchState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = raw::switch(state, spec, &Input::default(), &mut focus_sys);
        focus_sys.end_frame();
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
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
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_disabled() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            disabled: true,
            style: Default::default(),
            clip_rect: None,
        };
        let s = spec.style;
        let res = swi_tch(spec);
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: tint(s.off_fill),
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: tint(s.border),
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: tint(s.off_thumb),
                },
            ])
        );
    }

    #[test]
    fn test_switch_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = SwitchState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = raw::switch(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking switch must request focus"
        );
    }

    #[test]
    fn test_switch_keyboard_toggle() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = SwitchState::default();
        let mut input = Input::default();

        let spec = || SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        // Frame 1: Focus switch
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let res = raw::switch(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 2: Press Space
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_sys.begin_frame();
        let res = raw::switch(state, spec(), &input, &mut focus_sys);
        state = res.state;
        focus_sys.end_frame();

        // Frame 3: Release Space
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_sys.begin_frame();
        let res = raw::switch(state, spec(), &input, &mut focus_sys);
        focus_sys.end_frame();

        assert!(
            res.state.on,
            "Spacebar release must toggle switch state"
        );
    }
}
