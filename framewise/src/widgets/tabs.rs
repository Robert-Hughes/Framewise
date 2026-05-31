use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{ClipRect, Color, Rect, Vec2},
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
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level tabs widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tabs<'a, T: TextSystem>(
        spec: TabsSpec<'a>,
        state: &mut TabsState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
    ) -> TabsResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let tab_h = s.height;
        let pad_x = s.pad_x;
        let underbar_h = s.underbar_height;

        // Sum width of tabs
        let mut total_w = 0.0;
        for label in spec.items.iter() {
            let layout = text_system.prepare(label, s.text_size, spec.style.font);
            total_w += layout.size.x + pad_x * 2.0;
        }

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_focus(
                state.focus_id,
                Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h),
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::all(),
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
            for (i, label) in spec.items.iter().enumerate() {
                let layout = text_system.prepare(label, s.text_size, spec.style.font);
                let tab_w = layout.size.x + pad_x * 2.0;
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
            p0: Vec2::new(spec.rect.x, border_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, border_y),
            color: tint(s.border),
            width: s.border_width,
        });

        let mut x = spec.rect.x;

        for (i, label) in spec.items.iter().enumerate() {
            let is_active = i == state.active_index;

            let layout = text_system.prepare(label, s.text_size, spec.style.font);
            let tab_w = layout.size.x + pad_x * 2.0;
            let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

            // Focus ring.
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                cmds.push(DrawCmd::StrokeRect {
                    rect: tab_rect.inset(-s.focus_offset),
                    color: tint(s.focus),
                    width: s.focus_width,
                });
            }

            let text_color = if is_active { s.text } else { s.inactive_text };
            let ty = spec.rect.y + (tab_h - layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
                color: tint(text_color),
                handle: layout.handle,
            });

            // Active underbar: 3px rust rect sitting on the bottom border + upticks at the ends.
            if is_active {
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                    color: tint(s.accent),
                });
                // Left uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(x, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                });
                // Right uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(x + tab_w - 3.0, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                });
            }

            x += tab_w;
        }

        TabsResult {
            draw: cmds,
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
    pub text_size: f32,
    pub font: FontId,
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
            text_size: theme.text_md,
            font: theme.sans_font,
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

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TabsSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub items: Option<&'a [&'a str]>,
    pub style: Option<TabsStyle>,
    pub disabled: Option<bool>,
    pub clip_rect: Option<ClipRect>,
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
            self.style = Some(TabsStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> raw::TabsSpec<'a> {
        raw::TabsSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tabs widget function using WidgetContext.
///
/// This function accepts a TabsSpecBuilder and calls the low-level raw::tabs function.
pub fn tabs<'a, T: TextSystem, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TabsSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut TabsState,
) -> TabsResult {
    let layout_rect = ctx.layout_state.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::tabs(spec, state, ctx.input, ctx.focus_system, ctx.text_system);

    ctx.append_cmds(result.draw);

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
    use crate::test_utils::DummyTextSys;

    fn tabs_dummy<'a>(spec: TabsSpec<'a>, active_index: usize) -> raw::TabsResult {
        raw::tabs(
            spec,
            &mut TabsState {
                active_index,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_tabs_visual_normal() {
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let res = tabs_dummy(spec, 0);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(18.0, 10.0, 32.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(65.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::Text {
                    rect: Rect::new(86.0, 10.0, 32.0, 16.0),
                    color: style.inactive_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_tabs_visual_focused() {
        let mut state = TabsState {
            active_index: 1,
            ..Default::default()
        };
        let mut focus_system = FocusSystem::new();
        focus_system.take_focus(state.focus_id);
        focus_system.begin_frame();
        let mut text_system = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };
        let style = spec.style;
        let res = raw::tabs(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(18.0, 10.0, 32.0, 16.0),
                    color: style.inactive_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(66.0, -2.0, 72.0, 40.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(86.0, 10.0, 32.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(68.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(68.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(133.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
            ])
        );
    }

    #[test]
    fn test_tabs_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        focus_system.begin_frame();
        raw::tabs(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            Some(state.focus_id),
            "Clicking tabs must request focus"
        );
    }

    #[test]
    fn test_tabs_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            disabled: false,
            style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 300.0, 36.0)),
        };

        focus_system.begin_frame();
        raw::tabs(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_focus(),
            None,
            "Clicking a clipped-away tabs widget must not take focus"
        );
    }

    #[test]
    fn test_tabs_keyboard_navigation() {
        let mut focus_system = FocusSystem::new();
        let mut state = TabsState::default();
        let mut input = Input::default();
        let mut text_system = DummyTextSys;
        let items = ["Tab1", "Tab2"];

        // Focus the widget
        focus_system.take_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> changes active index to 1
        input.key_pressed_right = true;
        focus_system.begin_frame();
        raw::tabs(
            TabsSpec {
                rect: Rect::new(0.0, 0.0, 300.0, 36.0),
                items: &items,
                disabled: false,
                style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
        );
        focus_system.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        focus_system.begin_frame();
        raw::tabs(
            TabsSpec {
                rect: Rect::new(0.0, 0.0, 300.0, 36.0),
                items: &items,
                disabled: false,
                style: TabsStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
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
        custom_style.text_size = 99.0;
        let builder = TabsSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let mut tabs_state = TabsState::default();
        let result = super::tabs(
            &mut ctx,
            TabsSpecBuilder::new().items(&[]).rect(custom_rect),
            layout_rect,
            &mut tabs_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
