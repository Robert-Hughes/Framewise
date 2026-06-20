use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::layout_text;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipSpec<'a> {
        pub layer: Layer,
        /// Top-left origin. Height is fixed at 22.
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ChipStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipSizeSpec<'a> {
        pub text: &'a str,
        pub style: super::ChipStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Return the size this chip would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn size_chip<T: TextBackend>(
        spec: &ChipSizeSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        crate::layout::SizeRequest::preferred(layout.metrics().logical_size)
    }

    /// Low-level chip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn chip<'a, T: TextBackend>(
        spec: ChipSpec<'a>,
        state: &mut ChipState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> ChipResult {
        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
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

        let h = s.height;
        let pad_x = s.pad_x;

        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds {
                max_width: Some(spec.rect.w),
                max_height: Some(spec.rect.h),
            },
        );
        let w = spec.rect.w.max(32.0);
        let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

        // Focus ring.
        if focused {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: r.inset(-(s.focus_offset + s.focus_width)),
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_focus_z(),
            });
        }

        let bg = if state.checked {
            s.active_bg
        } else {
            s.background
        };
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: r,
            color: tint(bg),
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: r,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        let text_color = if state.checked { s.active_text } else { s.text };
        let metrics = layout.metrics();
        let ty = r.y + (h - metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            r.x + pad_x,
            ty,
            metrics.logical_size.x,
            metrics.logical_size.y,
        );
        layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            tint(text_color),
            spec.layer.get_z(),
        );

        ChipResult {
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
    pub text_style: crate::text::TextStyle,
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
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
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
    pub checked: bool,
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

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ChipSpec<'a> {
    pub text: &'a str,
    pub style: ChipStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ChipSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<ChipStyle>,
    pub disabled: Option<bool>,
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(ChipStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> ChipSpec<'a> {
        ChipSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level chip widget function using WidgetContext.
///
/// This function accepts a ChipSpecBuilder and calls the low-level raw::chip function.
pub fn chip<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ChipSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut ChipState,
) -> ChipResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let size_spec = raw::ChipSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let size_request = raw::size_chip(&size_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, size_request);
    let raw_spec = raw::ChipSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::chip(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

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
    use crate::test_utils::TestTextBackend;
    use crate::types::Vec2;
    use crate::{DrawGlyph, PreparedGlyphToken};

    fn chip_raw<'a>(spec: ChipSpec<'a>) -> (raw::ChipResult, DrawCommands) {
        let mut cmds = DrawCommands::new();
        let mut text_backend = TestTextBackend;
        let res = raw::chip(
            spec,
            &mut ChipState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_backend,
            &mut cmds,
        );
        (res, cmds)
    }

    #[test]
    fn test_chip_visual_normal() {
        let spec = ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let (_res, cmds) = chip_raw(spec);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: style.text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 8.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 16.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(103),
                    top_left: Vec2 { x: 24.0, y: 16.0 }
                }
            ]
        );
    }

    #[test]
    fn test_chip_visual_active() {
        let mut text_backend = TestTextBackend;
        let mut state = ChipState {
            checked: true,
            ..Default::default()
        };
        let spec = ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        raw::chip(
            spec,
            &mut state,
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_backend,
            &mut cmds,
        );

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.active_bg,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: style.active_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 8.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 16.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(103),
                    top_left: Vec2 { x: 24.0, y: 16.0 }
                }
            ]
        );
    }

    #[test]
    fn test_chip_visual_focused() {
        let state = ChipState::default();
        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.begin_frame();
        let mut text_backend = TestTextBackend;
        let spec = ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let mut state = state;
        let mut cmds = DrawCommands::new();
        raw::chip(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        let r = Rect::new(0.0, 0.0, 50.0, 22.0);
        let expected_focus_rect = r.inset(-(style.focus_offset + style.focus_width));
        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: expected_focus_rect,
                    color: style.focus,
                    width: style.focus_width,
                    z: 1,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: r,
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: r,
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..3,
                    color: style.text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 8.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 16.0, y: 16.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(103),
                    top_left: Vec2 { x: 24.0, y: 16.0 }
                }
            ]
        );
    }

    #[test]
    fn test_chip_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let state = ChipState::default();
        let input = Input {
            mouse_pos: Vec2::new(10.0, 10.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let spec = ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::chip(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Clicking chip must request focus"
        );
    }

    #[test]
    fn test_chip_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = ChipState::default();
        let input = Input {
            mouse_pos: Vec2::new(10.0, 10.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let spec = ChipSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            text: "Tag",
            disabled: false,
            style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 50.0, 22.0)),
        };

        let mut state = state;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::chip(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "Clicking a clipped-away chip must not take focus"
        );
    }

    #[test]
    fn test_chip_keyboard_toggle() {
        let mut focus_system = FocusSystem::new();
        let mut state = ChipState::default();
        let mut input = Input::default();
        let mut text_backend = TestTextBackend;

        // Frame 1: Focus chip
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::chip(
            ChipSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 2: Press Space
        input.key_down_space = true;
        input.key_pressed_space = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::chip(
            ChipSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        // Frame 3: Release Space
        input.key_down_space = false;
        input.key_pressed_space = false;
        input.key_released_space = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::chip(
            ChipSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                text: "Tag",
                disabled: false,
                style: ChipStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert!(state.checked, "Spacebar release must toggle chip state");
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
        custom_style.text_style.size = 99.0;
        let builder = ChipSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
        let mut text_backend = TestTextBackend;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let placement = Rect::new(10.0, 20.0, 200.0, 36.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_backend,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let mut chip_state = ChipState::default();
        let result = super::chip(
            &mut ctx,
            ChipSpecBuilder::new().text("X"),
            placement,
            &mut chip_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
