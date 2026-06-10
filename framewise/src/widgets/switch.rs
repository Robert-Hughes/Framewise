use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{IntrinsicSize, LayoutState},
    text::TextSystem,
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchSpec {
        /// Top-left of the 30x16 bounding area.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::SwitchStyle,
        pub clip_rect: ClipRect,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchCalcIntrinsicSizeSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SwitchResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Compute intrinsic size for Switch. Currently returns UNKNOWN.
    pub fn calc_switch_intrinsic_size(_spec: &SwitchCalcIntrinsicSizeSpec) -> IntrinsicSize {
        IntrinsicSize::UNKNOWN
    }

    /// Low-level switch widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn switch(
        spec: SwitchSpec,
        state: &mut SwitchState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> SwitchResult {
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
            state.checked = !state.checked;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let r = Rect::new(spec.rect.x, spec.rect.y, s.size.x, s.size.y);

        // Focus ring.
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: r.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        // Track fill.
        let track_fill = if state.checked { s.on_fill } else { s.off_fill };
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

        // Thumb dot (10x10, vertically centered, left/right positioned).
        let dot_y = r.y + (r.h - s.thumb_size) * 0.5;
        let dot_x = if state.checked {
            r.x + r.w - s.thumb_size - s.border_width
        } else {
            r.x + s.border_width
        };
        let dot_color = if state.checked {
            s.on_thumb
        } else {
            s.off_thumb
        };
        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(dot_x, dot_y, s.thumb_size, s.thumb_size),
            color: tint(dot_color),
        });

        SwitchResult {
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
pub struct SwitchStyle {
    pub size: Vec2,
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

impl SwitchStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            size: Vec2::new(30.0, 16.0),
            thumb_size: 10.0,
            off_fill: theme.paper_elev,
            on_fill: theme.ink,
            border: theme.ink,
            off_thumb: theme.ink,
            on_thumb: theme.paper,
            focus: theme.rust,
            border_width: 1.0,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SwitchState {
    pub checked: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SwitchSpec {
    pub disabled: bool,
    pub style: SwitchStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SwitchSpecBuilder {
    pub disabled: Option<bool>,
    pub style: Option<SwitchStyle>,
}

impl SwitchSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn style(mut self, style: SwitchStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SwitchStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SwitchSpec {
        SwitchSpec {
            disabled: self.disabled.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level switch widget function using WidgetContext.
///
/// This function accepts a SwitchSpecBuilder and calls the low-level raw::switch function.
pub fn switch<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SwitchSpecBuilder,
    layout_params: S::Params,
    state: &mut SwitchState,
) -> SwitchResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::SwitchCalcIntrinsicSizeSpec {};
    let intrinsic = raw::calc_switch_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::SwitchSpec {
        rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };
    let result = raw::switch(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);

    SwitchResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SwitchSpec;
    use super::*;
    use crate::types::Vec2;

    #[test]
    fn test_switch_visual_off() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        let mut cmds = DrawCommands::new();
        raw::switch(
            spec,
            &mut SwitchState {
                checked: false,
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
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.0, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_on() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        let mut cmds = DrawCommands::new();
        raw::switch(
            spec,
            &mut SwitchState {
                checked: true,
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
                    color: s.on_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(29.0, 13.0, 10.0, 10.0),
                    color: s.on_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_focused() {
        let mut state = SwitchState::default();
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };
        let s = spec.style;
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        let mut cmds = DrawCommands::new();
        raw::switch(
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
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.0, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_disabled() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: true,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };
        let s = spec.style;
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        let mut cmds = DrawCommands::new();
        raw::switch(
            spec,
            &mut SwitchState {
                checked: false,
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
                    color: tint(s.off_fill),
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: tint(s.border),
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.0, 13.0, 10.0, 10.0),
                    color: tint(s.off_thumb),
                },
            ])
        );
    }

    #[test]
    fn test_switch_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = SwitchState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };

        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::switch(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking switch must request focus"
        );
    }

    #[test]
    fn test_switch_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = SwitchState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 30.0, 16.0)),
            layer: Layer::default(),
        };

        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::switch(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away switch must not take focus"
        );
    }

    #[test]
    fn test_switch_keyboard_toggle() {
        let mut focus_system = FocusSystem::new();
        let mut state = SwitchState::default();
        let mut input = Input::default();

        let spec = || SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            disabled: false,
            style: SwitchStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        };

        // Frame 1: Focus switch
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::switch(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 2: Press Space
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        raw::switch(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 3: Release Space
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        raw::switch(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert!(state.checked, "Spacebar release must toggle switch state");
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = SwitchSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(SwitchStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = SwitchStyle::from_theme(&theme);
        custom_style.thumb_size = 99.0;
        let builder = SwitchSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().thumb_size, 99.0);
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
        let mut sw_state = SwitchState::default();
        let result = super::switch(&mut ctx, SwitchSpecBuilder::new(), placement, &mut sw_state);
        assert_eq!(result.layout.bounds, placement);
    }
}
