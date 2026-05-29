use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    input::Input,
    text::FontId,
    types::{ClipRect, Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberSpec<'a> {
        /// Full bounding rect (height typically h_md = 28).
        pub rect: Rect,
        pub text: &'a str,
        pub font: FontId,
        pub value: f32,
        pub min: f32,
        pub max: f32,
        pub disabled: bool,
        pub style: super::DragNumberStyle,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberResult {
        pub draw: DrawCommands,
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Low-level drag number widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn drag_number<'a, T: crate::text::TextSystem>(
        spec: DragNumberSpec<'a>,
        state: &mut DragNumberState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        text_system: &mut T,
    ) -> DragNumberResult {
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

        if state.value != spec.value && !state.is_dragging {
            state.value = spec.value;
        }

        let s = spec.style;

        // Label width calculation
        let text_layout = text_system.prepare(spec.text, s.text_size, spec.font);
        let text_w = text_layout.size.x + s.text_pad_x * 2.0;
        let value_x = spec.rect.x + text_w;
        let value_w = (spec.rect.w - text_w).max(20.0);

        // Mouse drag interaction
        if !spec.disabled {
            let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
            let hovered_value_area = is_visible
                && input.mouse_pos.x >= value_x
                && input.mouse_pos.x <= spec.rect.x + spec.rect.w
                && input.mouse_pos.y >= spec.rect.y
                && input.mouse_pos.y <= spec.rect.y + spec.rect.h;

            if input.mouse_pressed && hovered_value_area {
                state.is_dragging = true;
                state.drag_start_x = input.mouse_pos.x;
                state.drag_start_value = state.value;
                focus_sys.take_focus(state.focus_id);
            }

            if state.is_dragging {
                if !input.mouse_down {
                    state.is_dragging = false;
                } else {
                    let dx = input.mouse_pos.x - state.drag_start_x;
                    let value_range = spec.max - spec.min;
                    let delta_val = (dx / value_w) * value_range;
                    state.value = (state.drag_start_value + delta_val).clamp(spec.min, spec.max);
                }
            }
        }

        // Keyboard navigation when focused
        if focused && !spec.disabled {
            let step = (spec.max - spec.min) * 0.01;
            if input.key_pressed_left {
                state.value = (state.value - step).clamp(spec.min, spec.max);
            }
            if input.key_pressed_right {
                state.value = (state.value + step).clamp(spec.min, spec.max);
            }
        }

        let mut cmds = DrawCommands::new();
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_active = focused || state.is_dragging;

        // Focus / active ring.
        if visually_active && !spec.disabled {
            cmds.push(DrawCmd::StrokeRect {
                rect: spec.rect.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
            });
        }

        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: tint(s.background),
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: tint(s.border),
            width: s.border_width,
        });

        // text section (ink/rust bg, paper text).
        let text_rect = Rect::new(spec.rect.x, spec.rect.y, text_w, spec.rect.h);
        let text_bg = if visually_active {
            s.active_text_bg
        } else {
            s.text_bg
        };
        cmds.push(DrawCmd::FillRect {
            rect: text_rect,
            color: tint(text_bg),
        });

        let lty = spec.rect.y + (spec.rect.h - text_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(
                spec.rect.x + s.text_pad_x,
                lty,
                text_layout.size.x,
                text_layout.size.y,
            ),
            color: tint(s.text_text),
            handle: text_layout.handle,
        });

        // Value area: rust_soft fill proportional to value fraction.
        let frac = ((state.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
        if frac > 0.0 {
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(value_x, spec.rect.y, value_w * frac, spec.rect.h),
                color: tint(s.value_fill),
            });
        }

        let value_text = format!("{:.2}", state.value);
        let val_layout = text_system.prepare(&value_text, s.text_size, spec.font);
        let vtx = value_x + (value_w - val_layout.size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - val_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(vtx, vty, val_layout.size.x, val_layout.size.y),
            color: tint(s.value_text),
            handle: val_layout.handle,
        });

        DragNumberResult {
            draw: cmds,
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: state.is_dragging,
                clicked: clicked && !state.is_dragging,
            },
            focused,
            content_bounds: spec.rect.inset(s.border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragNumberStyle {
    pub text_size: f32,
    pub text_pad_x: f32,
    pub background: Color,
    pub border: Color,
    pub focus: Color,
    pub text_bg: Color,
    pub active_text_bg: Color,
    pub text_text: Color,
    pub value_text: Color,
    pub value_fill: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DragNumberState {
    pub value: f32,
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_value: f32,
    pub focus_id: crate::focus::FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DragNumberResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DragNumberSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<DragNumberStyle>,
    pub value: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub clip_rect: Option<ClipRect>,
}

impl<'a> DragNumberSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: DragNumberStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
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
            self.style = Some(theme.drag_number_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn build(self) -> raw::DragNumberSpec<'a> {
        raw::DragNumberSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            value: self.value.expect("value not set — call .value()"),
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self
                .clip_rect
                .expect("clip_rect not set — call .clip_rect()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level drag number widget function using WidgetContext.
///
/// This function accepts a DragNumberSpec and calls the low-level raw::drag_number function.
pub fn drag_number<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: DragNumberSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut DragNumberState,
) -> DragNumberResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let clip = builder.clip_rect.unwrap_or(ctx.clip_rect);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .clip_rect(clip)
        .build();
    let result = raw::drag_number(spec, state, ctx.input, ctx.focus_sys, ctx.text_system);

    ctx.append_cmds(result.draw);

    DragNumberResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
mod tests {
    use super::raw::DragNumberSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

    fn drag_num<'a>(spec: DragNumberSpec<'a>) -> raw::DragNumberResult {
        raw::drag_number(
            spec,
            &mut DragNumberState::default(),
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        )
    }

    #[test]
    fn test_drag_number_visual_normal() {
        let spec = DragNumberSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: crate::theme::Theme::framewise().drag_number_style(),
            clip_rect: None,
        };

        let style = spec.style;
        let res = drag_num(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.text_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.text_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                },
                DrawCmd::Text {
                    rect: Rect::new(54.0, 16.0, 40.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_drag_number_visual_active() {
        let mut state = DragNumberState::default();
        state.is_dragging = true;
        state.drag_start_value = 50.0;
        let spec = DragNumberSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: crate::theme::Theme::framewise().drag_number_style(),
            clip_rect: None,
        };

        let style = spec.style;
        let mut input = Input::default();
        input.mouse_down = true;
        let mut state = state;
        let res = raw::drag_number(
            spec,
            &mut state,
            &input,
            &mut crate::focus::FocusSystem::new(),
            &mut DummyTextSys,
        );

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: Rect::new(9.0, 9.0, 102.0, 30.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.active_text_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.text_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                },
                DrawCmd::Text {
                    rect: Rect::new(54.0, 16.0, 40.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_drag_number_visual_min_value() {
        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            font: FontId(1),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: crate::theme::Theme::framewise().drag_number_style(),
            clip_rect: None,
        };

        let style = spec.style;
        let res = raw::drag_number(
            spec,
            &mut DragNumberState::default(),
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
            &mut text_sys,
        );

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.text_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.text_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(58.0, 16.0, 32.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_drag_number_click_takes_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = DragNumberState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: crate::theme::Theme::framewise().drag_number_style(),
            clip_rect: None,
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::drag_number(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(state.focus_id),
            "Clicking drag number must request focus"
        );
    }

    #[test]
    fn test_drag_number_clipped_click_does_not_take_focus() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let state = DragNumberState::default();
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: crate::theme::Theme::framewise().drag_number_style(),
            clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 28.0)),
        };

        let mut state = state;
        focus_sys.begin_frame();
        raw::drag_number(spec, &mut state, &input, &mut focus_sys, &mut text_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            None,
            "Clicking a clipped-away drag number must not take focus"
        );
    }

    #[test]
    fn test_drag_number_keyboard_navigation() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut state = DragNumberState::default();
        let mut input = Input::default();
        let mut text_sys = DummyTextSys;

        // Focus the widget
        focus_sys.take_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> value increases by 1.0 (step = 100 * 0.01)
        input.key_pressed_right = true;
        focus_sys.begin_frame();
        raw::drag_number(
            DragNumberSpec {
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                text: "X",
                font: FontId(1),
                value: 50.0,
                min: 0.0,
                max: 100.0,
                disabled: false,
                style: crate::theme::Theme::framewise().drag_number_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.value, 51.0);

        // Frame 2: Press Arrow Left -> value decreases back to 50.0
        input.key_pressed_left = true;
        focus_sys.begin_frame();
        raw::drag_number(
            DragNumberSpec {
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                text: "X",
                font: FontId(1),
                value: 51.0,
                min: 0.0,
                max: 100.0,
                disabled: false,
                style: crate::theme::Theme::framewise().drag_number_style(),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_sys,
            &mut text_sys,
        );
        focus_sys.end_frame();

        assert_eq!(state.value, 50.0);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = DragNumberSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.drag_number_style()));
        assert_eq!(builder.font, Some(theme.sans_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.drag_number_style();
        custom_style.text_size = 99.0;
        let builder = DragNumberSpecBuilder::new()
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
        let mut dn_state = DragNumberState::default();
        let result = super::drag_number(
            &mut ctx,
            DragNumberSpecBuilder::new()
                .text("x")
                .value(0.0)
                .rect(custom_rect),
            layout_rect,
            &mut dn_state,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
