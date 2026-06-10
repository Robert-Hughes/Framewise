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
    pub struct RadioSpec {
        pub layer: Layer,
        /// Top-left of the 14x14 bounding area.
        pub rect: Rect,
        pub disabled: bool,
        pub style: super::RadioStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioCalcIntrinsicSizeSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct RadioResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Compute intrinsic size for Radio. Currently returns UNKNOWN.
    pub fn calc_radio_intrinsic_size(_spec: &RadioCalcIntrinsicSizeSpec) -> IntrinsicSize {
        IntrinsicSize::UNKNOWN
    }

    /// Low-level radio widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn radio(
        spec: RadioSpec,
        state: &mut RadioState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> RadioResult {
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
            state.checked = true;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let cx = spec.rect.x + s.radius;
        let cy = spec.rect.y + s.radius;
        let center = Vec2::new(cx, cy);

        // Focus ring (outset 2px).
        if focused {
            cmds.push(DrawCmd::StrokeCircle {
                center,
                radius: s.radius + s.focus_offset,
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_z(),
            });
        }

        // Background fill.
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: s.radius,
            color: tint(s.background),
            z: spec.layer.get_z(),
        });

        // Outer ring.
        cmds.push(DrawCmd::StrokeCircle {
            center,
            radius: s.radius,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        // Inner dot when selected.
        if state.checked {
            cmds.push(DrawCmd::FillCircle {
                center,
                radius: s.dot_radius,
                color: tint(s.dot),
                z: spec.layer.get_z(),
            });
        }

        RadioResult {
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: (clicked && input.mouse_down) || state.space_is_active,
                clicked: is_clicked,
            },
            focused,
            content_bounds: spec.rect.inset(s.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

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

impl RadioStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            radius: 7.0,
            dot_radius: 3.0,
            background: theme.paper_elev,
            border: theme.ink,
            dot: theme.ink,
            focus: theme.rust,
            border_width: 1.5,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RadioState {
    pub checked: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RadioResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct RadioSpec {
    pub disabled: bool,
    pub style: RadioStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct RadioSpecBuilder {
    pub disabled: Option<bool>,
    pub style: Option<RadioStyle>,
}

impl RadioSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(RadioStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> RadioSpec {
        RadioSpec {
            disabled: self.disabled.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level radio widget function using WidgetContext.
///
/// This function accepts a RadioSpecBuilder and calls the low-level raw::radio function.
pub fn radio<T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: RadioSpecBuilder,
    layout_params: S::Params,
    state: &mut RadioState,
) -> RadioResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::RadioCalcIntrinsicSizeSpec {};
    let intrinsic = raw::calc_radio_intrinsic_size(&calc_spec);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::RadioSpec {
        layer: ctx.layer,
        rect,
        disabled: spec.disabled,
        style: spec.style,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::radio(raw_spec, state, ctx.input, ctx.focus_system, ctx.cmds);

    RadioResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::RadioSpec;
    use super::*;

    #[test]
    fn test_radio_visual_unselected() {
        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
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
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_selected() {
        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
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
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.dot_radius,
                    color: s.dot,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_focused() {
        let state = RadioState::default();
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let mut state = state;
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
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
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius + s.focus_offset,
                    color: s.focus,
                    width: s.focus_width,
                    z: 0,
                },
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: s.background,
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: s.border,
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_visual_disabled() {
        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: true,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let s = spec.style;
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let center = Vec2::new(17.0, 17.0);
        let mut cmds = DrawCommands::new();
        raw::radio(
            spec,
            &mut RadioState {
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
                DrawCmd::FillCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.background),
                    z: 0,
                },
                DrawCmd::StrokeCircle {
                    center,
                    radius: s.radius,
                    color: tint(s.border),
                    width: s.border_width,
                    z: 0,
                },
            ])
        );
    }

    #[test]
    fn test_radio_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let state = RadioState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::radio(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking radio must request focus"
        );
    }

    #[test]
    fn test_radio_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = RadioState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let spec = RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 14.0, 14.0)),
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::radio(spec, &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away radio must not take focus"
        );
    }

    #[test]
    fn test_radio_keyboard_toggle() {
        let mut focus_system = FocusSystem::new();
        let mut state = RadioState::default();
        let mut input = Input::default();

        let spec = || RadioSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            disabled: false,
            style: RadioStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        // Frame 1: Explicitly focus the radio
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::radio(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 2: Press Space key while focused
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        raw::radio(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        // Frame 3: Release Space key
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        raw::radio(spec(), &mut state, &input, &mut focus_system, &mut cmds);
        focus_system.end_frame();

        assert_eq!(
            state.checked, true,
            "Spacebar release must toggle radio state to selected"
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = RadioSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(RadioStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = RadioStyle::from_theme(&theme);
        custom_style.radius = 99.0;
        let builder = RadioSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().radius, 99.0);
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
        let mut radio_state = RadioState::default();
        let result = super::radio(
            &mut ctx,
            RadioSpecBuilder::new(),
            placement,
            &mut radio_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
