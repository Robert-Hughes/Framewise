use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level tabs widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tabs<'a, T: crate::text::TextSystem>(
        mut state: TabsState,
        spec: TabsSpec<'a>,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_sys: &mut T,
    ) -> TabsResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let tab_h = s.height;
        let pad_x = s.pad_x;
        let underbar_h = s.underbar_height;

        // Sum width of tabs
        let mut total_w = 0.0;
        for label in spec.items.iter() {
            let layout = text_sys.prepare(label, s.text_size, spec.font);
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
                focus_sys,
                crate::focus::FocusTraversalKeys::all(),
                spec.disabled,
            )
        };

        if state.active_index != spec.active_index {
            state.active_index = spec.active_index;
        }

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
                let layout = text_sys.prepare(label, s.text_size, spec.font);
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

            let layout = text_sys.prepare(label, s.text_size, spec.font);
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
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(s.border_width)),
            input: InputInfo {
                hovered: Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h)
                    .contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            state,
            focused,
        }
    }
}

pub struct TabsSpec<'a> {
    /// Bounding rect; only x/y/w used — height is fixed at 36.
    pub rect: Rect,
    pub items: &'a [&'a str],
    pub font: FontId,
    pub active_index: usize,
    pub disabled: bool,
    pub style: TabsStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsStyle {
    pub height: f32,
    pub pad_x: f32,
    pub underbar_height: f32,
    pub text_size: f32,
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

impl Default for TabsStyle {
    fn default() -> Self {
        Self {
            height: 36.0,
            pad_x: 18.0,
            underbar_height: 3.0,
            text_size: 13.0,
            border: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            inactive_text: Color::from_srgb_u8(138, 131, 120, 255),
            accent: Color::from_srgb_u8(194, 90, 44, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TabsState {
    pub active_index: usize,
    pub focus_id: crate::focus::FocusId,
}

pub struct TabsResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: TabsState,
    pub focused: bool,
}

pub struct TabsInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: TabsState,
    pub focused: bool,
}

impl TabsInfo {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn active_index(&self) -> usize {
        self.state.active_index
    }
}

impl TabsResult {
    pub fn into_parts(self) -> (DrawCommands, TabsInfo) {
        (
            self.draw,
            TabsInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tabs widget function using WidgetContext.
///
/// This function accepts a TabsSpec and calls the low-level raw::tabs function.
pub fn tabs<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    state: TabsState,
    layout_params: S::Params,
    builder: TabsSpecBuilder<'a>,
) -> TabsInfo {
    let rect = ctx.layout(layout_params);
    let clip = builder.clip_rect.or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::tabs(state, spec, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw.0);

    TabsInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

pub struct TabsSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub font: Option<FontId>,
    pub style: Option<TabsStyle>,
    pub active_index: Option<usize>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}

impl<'a> Default for TabsSpecBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TabsSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            items: None,
            font: None,
            style: None,
            active_index: None,
            disabled: None,
            rect: None,
            clip_rect: None,
        }
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: TabsStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn active_index(mut self, active_index: usize) -> Self {
        self.active_index = Some(active_index);
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = Some(disabled);
        self
    }
    /// Overrides the clip rectangle. High-level context functions supply this from
    /// the surrounding clip region — only needed when using the raw API directly, or
    /// to clip tighter than the context default.
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
}

impl<'a> TabsSpecBuilder<'a> {
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
            self.style = Some(theme.tabs_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn build(self) -> TabsSpec<'a> {
        TabsSpec {
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
            font: self
                .font
                .expect("font must be specified or resolved from a theme"),
            style: self.style.expect("TabsStyle is required"),
            active_index: self.active_index.unwrap_or(0),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self.clip_rect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    fn tabs_dummy<'a>(spec: TabsSpec<'a>) -> TabsResult {
        raw::tabs(
            TabsState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_tabs_visual_normal() {
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = tabs_dummy(spec);

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
        let state = TabsState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            font: FontId(1),
            active_index: 1,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = raw::tabs(
            state,
            spec,
            &Input::default(),
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

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
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = TabsState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = raw::tabs(state, spec, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking tabs must request focus"
        );
    }

    #[test]
    fn test_tabs_keyboard_navigation() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = TabsState::default();
        let mut input = Input::default();
        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];

        // Focus the widget
        focus_sys.take_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> changes active index to 1
        input.key_pressed_right = true;
        focus_sys.begin_frame();
        let res = raw::tabs(
            state,
            TabsSpec {
                rect: Rect::new(0.0, 0.0, 300.0, 36.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        state = res.state;
        focus_sys.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        focus_sys.begin_frame();
        let res = raw::tabs(
            state,
            TabsSpec {
                rect: Rect::new(0.0, 0.0, 300.0, 36.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert_eq!(res.state.active_index, 0);
    }
}
