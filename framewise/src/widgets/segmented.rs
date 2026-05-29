use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{ClipRect, Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedSpec<'a> {
        /// Top-left origin. Height is fixed at h_md (28).
        pub rect: Rect,
        pub items: &'a [&'a str],
        pub font: FontId,
        pub active_index: usize,
        pub disabled: bool,
        pub style: super::SegmentedStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level segmented widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn segmented<'a, T: crate::text::TextSystem>(
        spec: SegmentedSpec<'a>,
        state: &mut SegmentedState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_system: &mut T,
    ) -> SegmentedResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        if spec.items.is_empty() {
            return SegmentedResult {
                draw: cmds,
                input: InputInfo {
                    hovered: false,
                    pressed: false,
                    clicked: false,
                },
                focused: false,
                content_bounds: spec.rect,
            };
        }

        let h = s.height;
        let pad_x = s.pad_x;

        // Pre-prepare all labels to get their widths.
        let layouts: Vec<_> = spec
            .items
            .iter()
            .map(|text| text_system.prepare(text, s.text_size, spec.font))
            .collect();
        let widths: Vec<f32> = layouts.iter().map(|l| l.size.x + pad_x * 2.0).collect();
        let total_w: f32 = widths.iter().sum();

        let outer = Rect::new(spec.rect.x, spec.rect.y, total_w, h);

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_focus(
                state.focus_id,
                outer,
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
            for (i, &w) in widths.iter().enumerate() {
                let seg_rect = Rect::new(x, spec.rect.y, w, h);
                let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
                if seg_rect.contains(input.mouse_pos) && is_visible {
                    state.active_index = i;
                    break;
                }
                x += w;
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        cmds.push(DrawCmd::FillRect {
            rect: outer,
            color: tint(s.background),
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: outer,
            color: tint(s.border),
            width: s.border_width,
        });

        let mut x = spec.rect.x;
        for (i, (layout, &w)) in layouts.iter().zip(widths.iter()).enumerate() {
            let is_active = i == state.active_index;
            let seg_rect = Rect::new(x, spec.rect.y, w, h);

            if is_active {
                cmds.push(DrawCmd::FillRect {
                    rect: seg_rect,
                    color: tint(s.active_bg),
                });
            }

            // Focus ring (inset to stay within bounds).
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                cmds.push(DrawCmd::StrokeRect {
                    rect: seg_rect.inset(s.focus_inset),
                    color: tint(s.focus),
                    width: s.focus_width,
                });
            }

            // Divider between segments (right edge, except last).
            if i + 1 < spec.items.len() {
                let div_x = x + w;
                cmds.push(DrawCmd::StrokeLine {
                    p0: Vec2::new(div_x, spec.rect.y),
                    p1: Vec2::new(div_x, spec.rect.y + h),
                    color: tint(s.border),
                    width: s.border_width,
                });
            }

            let text_color = if is_active { s.active_text } else { s.text };
            let ty = spec.rect.y + (h - layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
                color: tint(text_color),
                handle: layout.handle,
            });

            x += w;
        }

        SegmentedResult {
            draw: cmds,
            input: InputInfo {
                hovered: outer.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            focused,
            content_bounds: outer.inset(s.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentedStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_size: f32,
    pub background: Color,
    pub border: Color,
    pub active_bg: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_inset: f32,
    pub disabled_alpha: f32,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SegmentedState {
    pub active_index: usize,
    pub focus_id: crate::focus::FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

impl SegmentedResult {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentedSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub font: Option<FontId>,
    pub style: Option<SegmentedStyle>,
    pub active_index: Option<usize>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<ClipRect>,
}

impl<'a> SegmentedSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: SegmentedStyle) -> Self {
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
            self.style = Some(theme.segmented_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn build(self) -> raw::SegmentedSpec<'a> {
        raw::SegmentedSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            items: self.items.expect("items not set — call .items()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            active_index: self.active_index.unwrap_or(0),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level segmented widget function using WidgetContext.
///
/// This function accepts a SegmentedSpec and calls the low-level raw::segmented function.
pub fn segmented<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SegmentedSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut SegmentedState,
) -> SegmentedResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::segmented(spec, state, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw);

    SegmentedResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::SegmentedSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    fn segmented_dummy<'a>(spec: SegmentedSpec<'a>) -> raw::SegmentedResult {
        raw::segmented(
            spec,
            &mut SegmentedState::default(),
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_segmented_visual_normal() {
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: crate::theme::Theme::framewise().segmented_style(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = segmented_dummy(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_segmented_visual_focused() {
        let mut state = SegmentedState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 1,
            disabled: false,
            style: crate::theme::Theme::framewise().segmented_style(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = raw::segmented(
            spec,
            &mut state,
            &Input::default(),
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(36.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(38.0, 2.0, 32.0, 24.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_segmented_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: crate::theme::Theme::framewise().segmented_style(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        raw::segmented(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking segmented must request focus"
        );
    }

    #[test]
    fn test_segmented_clipped_click_does_not_take_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: crate::theme::Theme::framewise().segmented_style(),
            clip_rect: Some(Rect::new(500.0, 500.0, 200.0, 28.0)),
        };

        focus_sys.begin_frame();
        raw::segmented(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away segmented control must not take focus"
        );
    }

    #[test]
    fn test_segmented_keyboard_navigation() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = SegmentedState::default();
        let mut input = Input::default();
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];

        // Focus the widget
        focus_sys.take_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> changes active index to 1
        input.key_pressed_right = true;
        focus_sys.begin_frame();
        raw::segmented(
            SegmentedSpec {
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: crate::theme::Theme::framewise().segmented_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        focus_sys.begin_frame();
        raw::segmented(
            SegmentedSpec {
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: crate::theme::Theme::framewise().segmented_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert_eq!(state.active_index, 0);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = SegmentedSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.segmented_style()));
        assert_eq!(builder.font, Some(theme.sans_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.segmented_style();
        custom_style.text_size = 99.0;
        let builder = SegmentedSpecBuilder::new()
            .style(custom_style)
            .font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let mut seg_state = SegmentedState::default();
        let result = super::segmented(
            &mut ctx,
            SegmentedSpecBuilder::new().items(&[]).rect(custom_rect),
            layout_rect,
            &mut seg_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
