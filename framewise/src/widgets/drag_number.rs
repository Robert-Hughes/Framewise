use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::LayoutState,
    text::{emit_text_in_rect, measure_text, TextBackend},
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberSpec<'a> {
        pub layer: Layer,
        /// Full bounding rect (height typically h_md = 28).
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::DragNumberStyle,
        pub min: f32,
        pub max: f32,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberCalcIntrinsicSizeSpec<'a> {
        pub text: &'a str,
        pub style: super::DragNumberStyle,
        pub min: f32,
        pub max: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Measure a drag number's intrinsic size from its measurement spec.
    pub fn calc_drag_number_intrinsic_size<T: TextBackend>(
        spec: &DragNumberCalcIntrinsicSizeSpec,
        text_system: &mut T,
    ) -> crate::layout::IntrinsicSize {
        let s = spec.style;
        let label_metrics = measure_text(
            text_system,
            spec.text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let min_text = format!("{:.2}", spec.min);
        let max_text = format!("{:.2}", spec.max);
        let min_metrics = measure_text(
            text_system,
            &min_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let max_metrics = measure_text(
            text_system,
            &max_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let value_w =
            min_metrics.logical_size.x.max(max_metrics.logical_size.x) + s.text_pad_x * 2.0;
        let label_w = label_metrics.logical_size.x + s.text_pad_x * 2.0;
        crate::layout::IntrinsicSize::preferred(Vec2::new(label_w + value_w, s.height))
    }

    /// Low-level drag number widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn drag_number<'a, T: TextBackend>(
        spec: DragNumberSpec<'a>,
        state: &mut DragNumberState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> DragNumberResult {
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

        let s = spec.style;

        // Label width calculation
        let text_metrics = measure_text(
            text_system,
            spec.text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let text_w = text_metrics.logical_size.x + s.text_pad_x * 2.0;
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
                focus_system.take_keyboard_focus(state.focus_id);
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

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_active = focused || state.is_dragging;

        // Focus / active ring.
        if visually_active && !spec.disabled {
            cmds.push(DrawCmd::StrokeRect {
                anti_alias: false,
                rect: spec.rect.inset(-s.focus_offset),
                color: tint(s.focus),
                width: s.focus_width,
                z: spec.layer.get_z(),
            });
        }

        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: spec.rect,
            color: tint(s.background),
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeRect {
            anti_alias: false,
            rect: spec.rect,
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

        // text section (ink/rust bg, paper text).
        let text_rect = Rect::new(spec.rect.x, spec.rect.y, text_w, spec.rect.h);
        let text_bg = if visually_active {
            s.active_text_bg
        } else {
            s.text_bg
        };
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: text_rect,
            color: tint(text_bg),
            z: spec.layer.get_z(),
        });

        let lty = spec.rect.y + (spec.rect.h - text_metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            spec.rect.x + s.text_pad_x,
            lty,
            text_metrics.logical_size.x,
            text_metrics.logical_size.y,
        );
        emit_text_in_rect(
            cmds,
            text_system,
            spec.text,
            s.text_style,
            text_rect,
            tint(s.text_text),
            spec.layer.get_z(),
        );

        // Value area: rust_soft fill proportional to value fraction.
        let frac = ((state.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
        if frac > 0.0 {
            cmds.push(DrawCmd::FillRect {
                anti_alias: false,
                rect: Rect::new(value_x, spec.rect.y, value_w * frac, spec.rect.h),
                color: tint(s.value_fill),
                z: spec.layer.get_z(),
            });
        }

        let value_text = format!("{:.2}", state.value);
        let value_metrics = measure_text(
            text_system,
            &value_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let vtx = value_x + (value_w - value_metrics.logical_size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - value_metrics.logical_size.y) * 0.5;
        let value_rect = Rect::new(
            vtx,
            vty,
            value_metrics.logical_size.x,
            value_metrics.logical_size.y,
        );
        emit_text_in_rect(
            cmds,
            text_system,
            &value_text,
            s.text_style,
            value_rect,
            tint(s.value_text),
            spec.layer.get_z(),
        );

        DragNumberResult {
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
    pub height: f32,
    pub text_pad_x: f32,
    pub text_style: crate::text::TextStyle,
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

impl DragNumberStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_md,
            text_pad_x: 10.0,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: theme.ink,
            focus: theme.rust,
            text_bg: theme.ink,
            active_text_bg: theme.rust,
            text_text: theme.paper,
            value_text: theme.ink,
            value_fill: theme.rust_soft,
            border_width: theme.border,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset_tight,
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DragNumberState {
    pub value: f32,
    pub is_dragging: bool,
    pub drag_start_x: f32,
    pub drag_start_value: f32,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DragNumberResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct DragNumberSpec<'a> {
    pub text: &'a str,
    pub style: DragNumberStyle,
    pub min: f32,
    pub max: f32,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct DragNumberSpecBuilder<'a> {
    pub text: Option<&'a str>,
    pub style: Option<DragNumberStyle>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub disabled: Option<bool>,
}

impl<'a> DragNumberSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn style(mut self, style: DragNumberStyle) -> Self {
        self.style = Some(style);
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(DragNumberStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> DragNumberSpec<'a> {
        DragNumberSpec {
            text: self.text.expect("text not set — call .text()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            min: self.min.unwrap_or(0.0),
            max: self.max.unwrap_or(100.0),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level drag number widget function using WidgetContext.
///
/// This function accepts a DragNumberSpecBuilder and calls the low-level raw::drag_number function.
pub fn drag_number<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: DragNumberSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut DragNumberState,
) -> DragNumberResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let calc_spec = raw::DragNumberCalcIntrinsicSizeSpec {
        text: spec.text,
        style: spec.style,
        min: spec.min,
        max: spec.max,
    };
    let intrinsic = raw::calc_drag_number_intrinsic_size(&calc_spec, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::DragNumberSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        min: spec.min,
        max: spec.max,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::drag_number(
        raw_spec,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_system,
        ctx.cmds,
    );

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
    use crate::{DrawGlyph, PreparedGlyphHandle};

    fn drag_num<'a>(spec: DragNumberSpec<'a>, value: f32) -> (raw::DragNumberResult, DrawCommands) {
        let mut cmds = DrawCommands::new();
        let mut text_system = DummyTextSys;
        let res = raw::drag_number(
            spec,
            &mut DragNumberState {
                value,
                ..Default::default()
            },
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_system,
            &mut cmds,
        );
        (res, cmds)
    }

    #[test]
    fn test_drag_number_visual_normal() {
        let spec = DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let style = spec.style;
        let (_res, cmds) = drag_num(spec, 50.0);

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.text_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..1,
                    color: style.text_text,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 1..6,
                    color: style.value_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(88),
                    top_left: Vec2 { x: 20.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(53),
                    top_left: Vec2 { x: 54.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 62.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(46),
                    top_left: Vec2 { x: 70.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 78.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 86.0, y: 29.0 },
                },
            ]
        );
    }

    #[test]
    fn test_drag_number_visual_active() {
        let mut state = DragNumberState {
            value: 50.0,
            ..Default::default()
        };
        state.is_dragging = true;
        state.drag_start_value = 50.0;
        let spec = DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let style = spec.style;
        let mut input = Input::default();
        input.mouse_down = true;
        let mut state = state;
        let mut cmds = DrawCommands::new();
        let mut text_system = DummyTextSys;
        let _res = raw::drag_number(
            spec,
            &mut state,
            &input,
            &mut FocusSystem::new(),
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(9.0, 9.0, 102.0, 30.0),
                    color: style.focus,
                    width: style.focus_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.active_text_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..1,
                    color: style.text_text,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 1..6,
                    color: style.value_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(88),
                    top_left: Vec2 { x: 20.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(53),
                    top_left: Vec2 { x: 54.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 62.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(46),
                    top_left: Vec2 { x: 70.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 78.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 86.0, y: 29.0 },
                },
            ]
        );
    }

    #[test]
    fn test_drag_number_visual_min_value() {
        let mut text_system = DummyTextSys;
        let spec = DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let style = spec.style;
        let mut cmds = DrawCommands::new();
        let _res = raw::drag_number(
            spec,
            &mut DragNumberState::default(),
            &Input::default(),
            &mut FocusSystem::new(),
            &mut text_system,
            &mut cmds,
        );

        assert_eq!(
            cmds.commands(),
            vec![
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                    z: 0,
                },
                DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                    z: 0,
                },
                DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.text_bg,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 0..1,
                    color: style.text_text,
                    z: 0,
                },
                DrawCmd::GlyphRun {
                    glyphs: 1..5,
                    color: style.value_text,
                    z: 0,
                },
            ]
        );
        assert_eq!(
            cmds.glyphs(),
            vec![
                DrawGlyph {
                    handle: PreparedGlyphHandle(88),
                    top_left: Vec2 { x: 20.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 58.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(46),
                    top_left: Vec2 { x: 66.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 74.0, y: 29.0 },
                },
                DrawGlyph {
                    handle: PreparedGlyphHandle(48),
                    top_left: Vec2 { x: 82.0, y: 29.0 },
                },
            ]
        );
    }

    #[test]
    fn test_drag_number_click_takes_focus() {
        let mut focus_system = FocusSystem::new();
        let state = DragNumberState {
            value: 50.0,
            ..Default::default()
        };
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let spec = DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: None,
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::drag_number(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            Some(state.focus_id),
            "Clicking drag number must request focus"
        );
    }

    #[test]
    fn test_drag_number_clipped_click_does_not_take_focus() {
        let mut focus_system = FocusSystem::new();
        let state = DragNumberState {
            value: 50.0,
            ..Default::default()
        };
        let mut input = Input::default();
        input.mouse_pos = Vec2::new(15.0, 15.0);
        input.mouse_pressed = true;

        let mut text_system = DummyTextSys;
        let spec = DragNumberSpec {
            layer: Layer::default(),
            rect: Rect::new(0.0, 0.0, 100.0, 28.0),
            text: "X",
            min: 0.0,
            max: 100.0,
            disabled: false,
            style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
            clip_rect: Some(Rect::new(500.0, 500.0, 100.0, 28.0)),
        };

        let mut state = state;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::drag_number(
            spec,
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(
            focus_system.current_keyboard_focus(),
            None,
            "Clicking a clipped-away drag number must not take focus"
        );
    }

    #[test]
    fn test_drag_number_keyboard_navigation() {
        let mut focus_system = FocusSystem::new();
        let mut state = DragNumberState {
            value: 50.0,
            ..Default::default()
        };
        let mut input = Input::default();
        let mut text_system = DummyTextSys;

        // Focus the widget
        focus_system.take_keyboard_focus(state.focus_id);

        // Frame 1: Press Arrow Right -> value increases by 1.0 (step = 100 * 0.01)
        input.key_pressed_right = true;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::drag_number(
            DragNumberSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                text: "X",
                min: 0.0,
                max: 100.0,
                disabled: false,
                style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();
        input.key_pressed_right = false;

        assert_eq!(state.value, 51.0);

        // Frame 2: Press Arrow Left -> value decreases back to 50.0
        input.key_pressed_left = true;
        let mut cmds = DrawCommands::new();
        focus_system.begin_frame();
        raw::drag_number(
            DragNumberSpec {
                layer: Layer::default(),
                rect: Rect::new(0.0, 0.0, 100.0, 28.0),
                text: "X",
                min: 0.0,
                max: 100.0,
                disabled: false,
                style: DragNumberStyle::from_theme(&crate::theme::Theme::framewise()),
                clip_rect: None,
            },
            &mut state,
            &input,
            &mut focus_system,
            &mut text_system,
            &mut cmds,
        );
        focus_system.end_frame();

        assert_eq!(state.value, 50.0);
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = DragNumberSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(DragNumberStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = DragNumberStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = DragNumberSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_high_level_explicit_placement_via_manual_layout() {
        use crate::layouts::ManualLayout;
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
        let mut dn_state = DragNumberState::default();
        let result = super::drag_number(
            &mut ctx,
            DragNumberSpecBuilder::new().text("x"),
            placement,
            &mut dn_state,
        );
        assert_eq!(result.layout.bounds, placement);
    }
}
