use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    input::Input,
};

pub mod raw {
    use super::*;

    /// Low-level segmented widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn segmented<'a, T: crate::text::TextSystem>(
        mut state: SegmentedState,
        spec: SegmentedSpec<'a>,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_system: &mut T,
    ) -> SegmentedResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        if spec.items.is_empty() {
            return SegmentedResult {
                draw: cmds,
                layout: LayoutInfo::new(spec.rect, spec.rect),
                input: InputInfo { hovered: false, pressed: false, clicked: false },
                state,
                focused: false,
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
            if input.key_pressed_left {
                if state.active_index > 0 {
                    state.active_index -= 1;
                    is_clicked = true;
                }
            }
            if input.key_pressed_right {
                if state.active_index + 1 < spec.items.len() {
                    state.active_index += 1;
                    is_clicked = true;
                }
            }
        }

        // Mouse click segment detection
        if clicked && !spec.disabled && !spec.items.is_empty() {
            let mut x = spec.rect.x;
            for (i, &w) in widths.iter().enumerate() {
                let seg_rect = Rect::new(x, spec.rect.y, w, h);
                let is_visible = spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos));
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
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(s.border_width)),
            input: InputInfo {
                hovered: outer.contains(input.mouse_pos) && spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            state,
            focused,
        }
    }
}

pub struct SegmentedSpec<'a> {
    /// Top-left origin. Height is fixed at h_md (28).
    pub rect: Rect,
    pub items: &'a [&'a str],
    pub font: FontId,
    pub active_index: usize,
    pub disabled: bool,
    pub style: SegmentedStyle,
    pub clip_rect: Option<Rect>,
}

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

impl Default for SegmentedStyle {
    fn default() -> Self {
        Self {
            height: 28.0,
            pad_x: 14.0,
            text_size: 13.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            active_bg: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            active_text: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_inset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SegmentedState {
    pub active_index: usize,
    pub focus_id: crate::focus::FocusId,
}

impl Default for SegmentedState {
    fn default() -> Self {
        Self {
            active_index: 0,
            focus_id: crate::focus::FocusId::new(),
        }
    }
}

pub struct SegmentedResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: SegmentedState,
    pub focused: bool,
}

pub struct SegmentedInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: SegmentedState,
    pub focused: bool,
}

impl SegmentedInfo {
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

impl SegmentedResult {
    pub fn into_parts(self) -> (DrawCommands, SegmentedInfo) {
        (
            self.draw,
            SegmentedInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level segmented widget function using WidgetContext.
///
/// This function accepts a SegmentedSpec and calls the low-level raw::segmented function.
pub fn segmented<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: SegmentedState,
    layout_params: S::Params,
    builder: SegmentedSpecBuilder<'a>,
) -> SegmentedInfo {
    let rect = ctx.layout(layout_params);
    let mut builder = builder
        .with_rect(rect)
        .with_theme(&ctx.theme);
    if builder.clip_rect.is_none() {
        builder.clip_rect = ctx.clip_rect;
    }
    let spec = builder.build();
    let result = raw::segmented(state, spec, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw.0);

    SegmentedInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

// ── Re-export raw function for direct use ───────────────────────────────────────────
pub use raw::segmented as segmented_raw;

pub struct SegmentedSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub font: Option<FontId>,
    pub style: Option<SegmentedStyle>,
    pub active_index: Option<usize>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<Rect>,
}

impl<'a> SegmentedSpecBuilder<'a> {
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
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
}

impl<'a> SegmentedSpecBuilder<'a> {
    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = Some(theme.segmented_style());
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn build(self) -> SegmentedSpec<'a> {
        SegmentedSpec {
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
            font: self.font.expect("font must be specified or resolved from a theme"),
            style: self.style.expect("SegmentedStyle is required"),
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

    fn seg_mented<'a>(spec: SegmentedSpec<'a, T>) -> SegmentedResult {
        segmented_raw(
            SegmentedState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_segmented_visual_normal() {
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = seg_mented(spec);

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
        let state = SegmentedState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        focus_sys.take_focus(state.focus_id);
        focus_sys.begin_frame();
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 1,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };
        let style = spec.style;
        let res = segmented_raw(state, spec, &Input::default(), &mut focus_sys);
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
        let state = SegmentedState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(20.0, 10.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId(1),
            active_index: 0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = segmented_raw(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking segmented must request focus"
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
        let res = segmented_raw(
            state,
            SegmentedSpec {
                ts: &mut text_sys,
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
        );
        state = res.state;
        focus_sys.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.active_index, 1);

        // Frame 2: Press Arrow Left -> changes active index back to 0
        input.key_pressed_left = true;
        focus_sys.begin_frame();
        let res = segmented_raw(
            state,
            SegmentedSpec {
                ts: &mut text_sys,
                rect: Rect::new(0.0, 0.0, 200.0, 28.0),
                items: &items,
                font: FontId(1),
                active_index: 0,
                disabled: false,
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert_eq!(res.state.active_index, 0);
    }
}
