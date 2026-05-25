use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    input::Input,
};

pub mod raw {
    use super::*;

    /// Low-level drag number widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn drag_number<'a, T: crate::text::TextSystem>(
        mut state: DragNumberState,
        spec: DragNumberSpec<'a, T>,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
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
        let label_layout = spec.ts.prepare(spec.label, s.text_size, spec.font);
        let label_w = label_layout.size.x + s.label_pad_x * 2.0;
        let value_x = spec.rect.x + label_w;
        let value_w = (spec.rect.w - label_w).max(20.0);

        // Mouse drag interaction
        if !spec.disabled {
            let is_visible = spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos));
            let hovered_value_area = is_visible && input.mouse_pos.x >= value_x && input.mouse_pos.x <= spec.rect.x + spec.rect.w && input.mouse_pos.y >= spec.rect.y && input.mouse_pos.y <= spec.rect.y + spec.rect.h;

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

        // Label section (ink/rust bg, paper text).
        let label_rect = Rect::new(spec.rect.x, spec.rect.y, label_w, spec.rect.h);
        let label_bg = if visually_active {
            s.active_label_bg
        } else {
            s.label_bg
        };
        cmds.push(DrawCmd::FillRect {
            rect: label_rect,
            color: tint(label_bg),
        });

        let lty = spec.rect.y + (spec.rect.h - label_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(
                spec.rect.x + s.label_pad_x,
                lty,
                label_layout.size.x,
                label_layout.size.y,
            ),
            color: tint(s.label_text),
            handle: label_layout.handle,
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
        let val_layout = spec.ts.prepare(&value_text, s.text_size, spec.font);
        let vtx = value_x + (value_w - val_layout.size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - val_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(vtx, vty, val_layout.size.x, val_layout.size.y),
            color: tint(s.value_text),
            handle: val_layout.handle,
        });

        DragNumberResult {
            draw: cmds,
            layout: LayoutInfo::new(spec.rect, spec.rect.inset(s.border_width)),
            input: InputInfo {
                hovered: spec.rect.contains(input.mouse_pos) && spec.clip_rect.map_or(true, |c| c.contains(input.mouse_pos)),
                pressed: state.is_dragging,
                clicked: clicked && !state.is_dragging,
            },
            state,
            focused,
        }
    }
}

pub struct DragNumberSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Full bounding rect (height typically h_md = 28).
    pub rect: Rect,
    pub label: &'a str,
    pub font: FontId,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub disabled: bool,
    pub style: DragNumberStyle,
    pub clip_rect: Option<Rect>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragNumberStyle {
    pub text_size: f32,
    pub label_pad_x: f32,
    pub background: Color,
    pub border: Color,
    pub focus: Color,
    pub label_bg: Color,
    pub active_label_bg: Color,
    pub label_text: Color,
    pub value_text: Color,
    pub value_fill: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl Default for DragNumberStyle {
    fn default() -> Self {
        Self {
            text_size: 13.0,
            label_pad_x: 10.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            label_bg: Color::from_srgb_u8(21, 19, 15, 255),
            active_label_bg: Color::from_srgb_u8(194, 90, 44, 255),
            label_text: Color::from_srgb_u8(244, 241, 234, 255),
            value_text: Color::from_srgb_u8(21, 19, 15, 255),
            value_fill: Color::from_srgb_f32(194.0 / 255.0, 90.0 / 255.0, 44.0 / 255.0, 0.14),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 1.0,
            disabled_alpha: 0.35,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DragNumberState {
    pub value: f32,
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_value: f32,
    pub focus_id: crate::focus::FocusId,
}

impl Default for DragNumberState {
    fn default() -> Self {
        Self {
            value: 0.0,
            is_dragging: false,
            drag_start_x: 0.0,
            drag_start_value: 0.0,
            focus_id: crate::focus::FocusId::new(),
        }
    }
}

pub struct DragNumberResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: DragNumberState,
    pub focused: bool,
}

pub struct DragNumberInfo {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub state: DragNumberState,
    pub focused: bool,
}

impl DragNumberInfo {
    pub fn clicked(&self) -> bool {
        self.input.clicked
    }
    pub fn hovered(&self) -> bool {
        self.input.hovered
    }
    pub fn focused(&self) -> bool {
        self.focused
    }
    pub fn value(&self) -> f32 {
        self.state.value
    }
}

impl DragNumberResult {
    pub fn into_parts(self) -> (DrawCommands, DragNumberInfo) {
        (
            self.draw,
            DragNumberInfo {
                layout: self.layout,
                input: self.input,
                state: self.state,
                focused: self.focused,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level drag number widget function using WidgetContext.
///
/// This function accepts a DragNumberSpec and calls the low-level raw::drag_number function.
pub fn drag_number<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: DragNumberState,
    spec: DragNumberSpec<'_, T>,
    input: &Input,
) -> DragNumberInfo {
    let result = raw::drag_number(state, spec, input, ctx.focus_sys);
    
    ctx.append_cmds(result.draw.0);
    
    DragNumberInfo {
        layout: result.layout,
        input: result.input,
        state: result.state,
        focused: result.focused,
    }
}

// ── Re-export raw function for direct use ───────────────────────────────────────────
pub use raw::drag_number as drag_number_raw;

pub struct DragNumberSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<DragNumberStyle>,
    pub value: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub disabled: Option<bool>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
    pub clip_rect: Option<Rect>,
}

impl<'a, T: crate::text::TextSystem> DragNumberSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            value: None,
            min: None,
            max: None,
            disabled: None,
            rect: None,
            ts: None,
            clip_rect: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
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
    pub fn clip_rect(mut self, clip_rect: Option<Rect>) -> Self {
        self.clip_rect = clip_rect;
        self
    }
}

impl<'a, T: crate::text::TextSystem> DragNumberSpecBuilder<'a, T> {
    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = Some(theme.drag_number_style());
        if self.font.is_none() {
            self.font = Some(theme.sans_font);
        }
        self
    }

    pub fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    pub fn build(self) -> DragNumberSpec<'a, T> {
        DragNumberSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self.font.expect("font must be specified or resolved from a theme"),
            style: self.style.expect("DragNumberStyle is required"),
            value: self.value.unwrap_or(0.0),
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            disabled: self.disabled.unwrap_or(false),
            clip_rect: self.clip_rect,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

    fn drag_num<'a, T: crate::text::TextSystem>(spec: DragNumberSpec<'a, T>) -> DragNumberResult {
        drag_number(
            DragNumberState::default(),
            spec,
            &Input::default(),
            &mut crate::focus::FocusSystem::new(),
        )
    }

    #[test]
    fn test_drag_number_visual_normal() {
        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: Default::default(),
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
                    color: style.label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
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
        let mut text_sys = DummyTextSys;
        let mut state = DragNumberState::default();
        state.is_dragging = true;
        state.drag_start_value = 50.0;
        let spec = DragNumberSpec {
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        let style = spec.style;
        let mut input = Input::default();
        input.mouse_down = true;
        let res = drag_number(state, spec, &input, &mut crate::focus::FocusSystem::new());

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
                    color: style.active_label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
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
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId(1),
            value: 0.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: Default::default(),
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
                    color: style.label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
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
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            label: "X",
            font: FontId(1),
            value: 50.0,
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: Default::default(),
            clip_rect: None,
        };

        focus_sys.begin_frame();
        let res = drag_number(state, spec, &input, &mut focus_sys);
        focus_sys.end_frame();

        assert_eq!(
            focus_sys.current_focus(),
            Some(res.state.focus_id),
            "Clicking drag number must request focus"
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
        let res = drag_number(
            state,
            DragNumberSpec {
                ts: &mut text_sys,
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                label: "X",
                font: FontId(1),
                value: 50.0,
                min: 0.0,
                max: 100.0,
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

        assert_eq!(state.value, 51.0);

        // Frame 2: Press Arrow Left -> value decreases back to 50.0
        input.key_pressed_left = true;
        focus_sys.begin_frame();
        let res = drag_number(
            state,
            DragNumberSpec {
                ts: &mut text_sys,
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                label: "X",
                font: FontId(1),
                value: 51.0,
                min: 0.0,
                max: 100.0,
                disabled: false,
                style: Default::default(),
                clip_rect: None,
            },
            &input,
            &mut focus_sys,
        );
        focus_sys.end_frame();

        assert_eq!(res.state.value, 50.0);
    }
}
