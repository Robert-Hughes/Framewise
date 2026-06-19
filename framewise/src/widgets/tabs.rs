use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{IntrinsicSize, LayoutState},
    text::{layout_text, TextBackend, TextBounds, TextStyle},
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsSpec<'a> {
        /// Bounding rect; only x/y/w used — height is fixed at 36.
        pub rect: Rect,
        pub items: &'a [&'a str],
        pub style: super::TabsStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsCalcIntrinsicSizeSpec<'a> {
        pub items: &'a [&'a str],
        pub style: super::TabsStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Measure a tabs widget's intrinsic size from its measurement spec.
    pub fn calc_tabs_intrinsic_size<T: TextBackend>(
        spec: &TabsCalcIntrinsicSizeSpec,
        text_backend: &mut T,
    ) -> IntrinsicSize {
        let s = spec.style;
        let mut total_w = 0.0_f32;
        for label in spec.items.iter() {
            let layout = layout_text(text_backend, label, s.text_style, TextBounds::UNBOUNDED);
            total_w += layout.metrics().logical_size.x + s.pad_x * 2.0;
        }
        IntrinsicSize::preferred(Vec2::new(total_w, s.height))
    }

    /// Low-level tabs widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tabs<'a, T: TextBackend>(
        spec: TabsSpec<'a>,
        state: &mut TabsState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> TabsResult {
        let s = spec.style;

        let tab_h = s.height;
        let pad_x = s.pad_x;
        let underbar_h = s.underbar_height;

        // Pre-layout all labels.
        let layouts: Vec<_> = spec
            .items
            .iter()
            .map(|label| layout_text(text_backend, label, s.text_style, TextBounds::UNBOUNDED))
            .collect();

        // Calculate tab widths and total width
        let widths: Vec<f32> = layouts
            .iter()
            .map(|l| l.metrics().logical_size.x + pad_x * 2.0)
            .collect();
        let total_w: f32 = widths.iter().sum();

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h),
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::tab_only(),
                spec.disabled,
            )
        };

        let mut is_clicked = clicked;

        // Left/Right keyboard navigation
        if focused && !spec.disabled && !spec.items.is_empty() {
            if input.key_pressed_left && state.active_index > 0 {
                state.active_index -= 1;
                is_clicked = true;
            }
            if input.key_pressed_right && state.active_index + 1 < spec.items.len() {
                state.active_index += 1;
                is_clicked = true;
            }
        }

        // Mouse click segment detection
        if clicked && !spec.disabled && !spec.items.is_empty() {
            let mut x = spec.rect.x;
            for (i, &tab_w) in widths.iter().enumerate() {
                let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);
                let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
                if tab_rect.contains(input.mouse_pos) && is_visible {
                    state.active_index = i;
                    break;
                }
                x += tab_w;
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        // Bottom border across the full width.
        let border_y = spec.rect.y + tab_h;
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(spec.rect.x, border_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, border_y),
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        let mut x = spec.rect.x;

        for (i, (_label, layout)) in spec.items.iter().zip(layouts.iter()).enumerate() {
            let is_active = i == state.active_index;

            let metrics = layout.metrics();
            let tab_w = metrics.logical_size.x + pad_x * 2.0;
            let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

            let text_h = metrics.logical_size.y;
            let text_w = metrics.logical_size.x;

            // Focus ring.
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                cmds.push(DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: tab_rect.inset(-(s.focus_offset + s.focus_width)),
                    color: tint(s.focus),
                    width: s.focus_width,
                    z: spec.layer.get_focus_z(),
                });
            }

            let text_color = if is_active { s.text } else { s.inactive_text };
            let ty = spec.rect.y + (tab_h - text_h) * 0.5;
            let text_rect = Rect::new(x + pad_x, ty, text_w, text_h);
            layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(text_rect.x, text_rect.y),
                tint(text_color),
                spec.layer.get_z(),
            );

            // Active underbar: 3px rust rect sitting on the bottom border + upticks at the ends.
            if is_active {
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
                // Left uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
                // Right uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x + tab_w - 3.0, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
            }

            x += tab_w;
        }

        TabsResult {
            input: InputInfo {
                hovered: Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h)
                    .contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            focused,
            content_bounds: Rect::new(
                spec.rect.x,
                spec.rect.y,
                spec.rect.w,
                tab_h - s.border_width,
            ),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsStyle {
    pub height: f32,
    pub pad_x: f32,
    pub underbar_height: f32,
    pub text_style: TextStyle,
    pub border: Color,
    pub text: Color,
    pub inactive_text: Color,
    pub accent: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl TabsStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: 36.0,
            pad_x: 18.0,
            underbar_height: 3.0,
            text_style: TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            border: theme.ink,
            text: theme.ink,
            inactive_text: theme.muted,
            accent: theme.rust,
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
pub struct TabsState {
    pub active_index: usize,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TabsResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TabsSpec<'a> {
    pub items: &'a [&'a str],
    pub style: TabsStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TabsSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub style: Option<TabsStyle>,
    pub disabled: Option<bool>,
}

impl<'a> TabsSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: TabsStyle) -> Self {
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
            self.style = Some(TabsStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> TabsSpec<'a> {
        TabsSpec {
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tabs widget function using WidgetContext.
///
/// This function accepts a TabsSpecBuilder and calls the low-level raw::tabs function.
pub fn tabs<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TabsSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut TabsState,
) -> TabsResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::TabsCalcIntrinsicSizeSpec {
        items: spec.items,
        style: spec.style,
    };
    let intrinsic = raw::calc_tabs_intrinsic_size(&calc_spec, ctx.text_backend);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::TabsSpec {
        rect,
        items: spec.items,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };

    let result = raw::tabs(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    TabsResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::TabsSpec;
    use super::*;
    use crate::test_utils::TestTextBackend;
    use crate::{DrawGlyph, PreparedGlyphToken};

    fn make_spec<'a>(items: &'a [&'a str]) -> TabsSpec<'a> {
        TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
            layer: Layer::default(),
        }
    }

    fn tabs_dummy<'a>(spec: TabsSpec<'a>, active_index: usize) -> (DrawCommands, raw::TabsResult) {
        let mut cmds = DrawCommands::new();
        let mut text_backend = TestTextBackend;
        let result = raw::tabs(
            spec,
            &mut TabsState {
                active_index,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_backend,
            &mut cmds,
        );
        (cmds, result)
    }

    #[test]
    fn test_tabs_visual_normal() {
        let items = ["Tab1", "Tab2"];
        let spec = make_spec(&items);
        let style = spec.style;
        let (cmds, _res) = tabs_dummy(spec, 0);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..4,
                    color: style.text,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(0.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(65.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 4..8,
                    color: style.inactive_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 18.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 26.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(98),
                    top_left: Vec2 { x: 34.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(49),
                    top_left: Vec2 { x: 42.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 86.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 94.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(98),
                    top_left: Vec2 { x: 102.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(50),
                    top_left: Vec2 { x: 110.0, y: 23.0 }
                }
            ]
        );
    }

    #[test]
    fn test_tabs_visual_focused() {
        let mut state = TabsState {
            active_index: 1,
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        focus_system.take_keyboard_focus(state.focus_id);
        focus_system.begin_frame();
        let mut text_backend = TestTextBackend;
        let items = ["Tab1", "Tab2"];
        let spec = make_spec(&items);
        let style = spec.style;
        let mut cmds = DrawCommands::new();
        let _res = raw::tabs(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeLine {
                    anti_alias: false,
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..4,
                    color: style.inactive_text,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(64.0, -4.0, 76.0, 44.0),
                    color: style.focus,
                    width: style.focus_width,
                    z: 1,
                },
                DrawCmd::GlyphRun {
                    glyphs: 4..8,
                    color: style.text,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(68.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(68.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(133.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 18.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 26.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(98),
                    top_left: Vec2 { x: 34.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(49),
                    top_left: Vec2 { x: 42.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(84),
                    top_left: Vec2 { x: 86.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(97),
                    top_left: Vec2 { x: 94.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(98),
                    top_left: Vec2 { x: 102.0, y: 23.0 }
                },
                DrawGlyph {
                    token: PreparedGlyphToken(50),
                    top_left: Vec2 { x: 110.0, y: 23.0 }
                }
            ]
        );
    }

    #[test]
    fn test_tabs_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let input = Input {
            mouse_pos: Vec2::new(20.0, 10.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let items = ["Tab1", "Tab2"];
        let spec = make_spec(&items);

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::tabs(
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
            "Clicking tabs must request focus"
        );
    }

    #[test]
    fn test_tabs_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let input = Input {
            mouse_pos: Vec2::new(20.0, 10.0),
            mouse_pressed: true,
            ..Default::default()
        };

        let mut text_backend = TestTextBackend;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 300.0, 36.0)),
            layer: Layer::default(),
        };

        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::tabs(
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
            "Clicking a clipped-away tabs widget must not take focus"
        );
    }

    #[test]
    fn test_tabs_keyboard_navigation() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let mut input = Input::default();
        let mut text_backend = TestTextBackend;
        let items = ["Tab1", "Tab2"];

        // Focus the widget
        focus_system.take_keyboard_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> changes active index to 1
        input.key_pressed_right = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::tabs(
            make_spec(&items),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        focus_system.begin_frame();
        let mut cmds = DrawCommands::new();
        raw::tabs(
            make_spec(&items),
            &mut state,
            &input,
            &mut focus_system,
            &mut text_backend,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.active_index, 0);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = TabsSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(TabsStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = TabsStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = TabsSpecBuilder::new().style(custom_style);
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
        let mut tabs_state = TabsState::default();
        let result = super::tabs(
            &mut ctx,
            TabsSpecBuilder::new().items(&[]),
            placement,
            &mut tabs_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }

    #[test]
    fn test_calc_tabs_intrinsic_size() {
        let mut ts = TestTextBackend;
        let spec = raw::TabsCalcIntrinsicSizeSpec {
            items: &["Tab1", "Tab2"],
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
        };
        // Tab1 = 4 chars * 8px = 32px + 2*18 pad = 68px; Tab2 = same = 68px; total = 136px
        let intrinsic = raw::calc_tabs_intrinsic_size(&spec, &mut ts);
        assert_eq!(intrinsic.preferred, Some(Vec2::new(136.0, 36.0)));
    }
}
