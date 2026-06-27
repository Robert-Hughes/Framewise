#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    text::{layout_text, TextBackend},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
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
        pub step: f32,
        pub page_step: f32,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::DragNumberStyle,
        pub min: f32,
        pub max: f32,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct DragNumberResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this drag number would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_drag_number<T: TextBackend>(
        spec: &DragNumberPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> DragNumberPreLayoutResult {
        DragNumberPreLayoutResult {
            size_request: drag_number_size_request(spec, offer, text_backend),
        }
    }

    fn drag_number_size_request<T: TextBackend>(
        spec: &DragNumberPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let s = spec.style;
        let label_layout = layout_text(
            text_backend,
            spec.text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let label_metrics = label_layout.metrics();
        let min_text = format!("{:.2}", spec.min);
        let max_text = format!("{:.2}", spec.max);
        let min_layout = layout_text(
            text_backend,
            &min_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let min_metrics = min_layout.metrics();
        let max_layout = layout_text(
            text_backend,
            &max_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let max_metrics = max_layout.metrics();
        let value_w =
            min_metrics.logical_size.x.max(max_metrics.logical_size.x) + s.text_pad_x * 2.0;
        let label_w = label_metrics.logical_size.x + s.text_pad_x * 2.0;
        crate::layout::SizeRequest::preferred(Vec2::new(label_w + value_w, s.height))
    }

    /// Low-level drag number widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_drag_number<'a, T: TextBackend>(
        spec: DragNumberSpec<'a>,
        _pre_layout: DragNumberPreLayoutResult,
        state: &mut DragNumberState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> DragNumberResult {
        if spec.disabled {
            state.is_dragging = false;
        }

        let (focused, _) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                spec.rect,
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::tab_only(),
                spec.disabled,
            )
        };

        let s = spec.style;

        // Label width calculation
        let text_layout = layout_text(
            text_backend,
            spec.text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let text_metrics = text_layout.metrics();
        let text_w = text_metrics.logical_size.x + s.text_pad_x * 2.0;
        let value_x = spec.rect.x + text_w;
        let value_w = (spec.rect.w - text_w).max(20.0);

        let is_visible = spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        let contains = spec.rect.contains(input.mouse_pos) && is_visible;

        if contains && !spec.disabled {
            focus_system.claim_hover(state.focus_id);
        }
        let is_hover_active = focus_system.is_hover_active(state.focus_id);

        let clamp_min = spec.min.min(spec.max);
        let clamp_max = spec.min.max(spec.max);

        // Mouse drag interaction
        if !spec.disabled {
            let hovered_control_area = contains && is_hover_active;

            if input.mouse_pressed && hovered_control_area {
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
                    state.value = (state.drag_start_value + delta_val).clamp(clamp_min, clamp_max);
                }
            }
        }

        // Keyboard navigation when focused
        if focused && !spec.disabled {
            focus_system.claim_pgup_vert(state.focus_id);
            focus_system.claim_pgdn_vert(state.focus_id);
            focus_system.claim_pgup_horiz(state.focus_id);
            focus_system.claim_pgdn_horiz(state.focus_id);

            if input.key_pressed_left || input.key_pressed_up {
                state.value = (state.value - spec.step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_right || input.key_pressed_down {
                state.value = (state.value + spec.step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_page_up {
                state.value = (state.value - spec.page_step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_page_down {
                state.value = (state.value + spec.page_step).clamp(clamp_min, clamp_max);
            }
            if input.key_pressed_home {
                state.value = clamp_min;
            }
            if input.key_pressed_end {
                state.value = clamp_max;
            }
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let visually_active = focused || state.is_dragging;
        let draw_outer = cmds.snap_rect_edges_to_physical_pixel(spec.rect);
        let draw_split_x = cmds.snap_to_physical_pixel(value_x);
        let draw_label_rect = Rect::from_ltrb(
            draw_outer.x,
            draw_outer.y,
            draw_split_x,
            draw_outer.bottom(),
        );

        // Focus / active ring.
        if visually_active && !spec.disabled {
            if let Some(outline) = s.focus {
                let focus_stroke = Stroke::new(tint(outline.stroke.color), outline.stroke.width);
                cmds.push_crisp_border_rect(
                    spec.rect.inset(-outline.offset),
                    Some(focus_stroke),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        cmds.push_crisp_fill_rect(draw_outer, tint(s.background), spec.layer.get_z());

        // text section (ink/rust bg, paper text).
        let text_bg = if visually_active {
            s.active_text_bg
        } else {
            s.text_bg
        };
        cmds.push_crisp_fill_rect(draw_label_rect, tint(text_bg), spec.layer.get_z());

        let lty = spec.rect.y + (spec.rect.h - text_metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            spec.rect.x + s.text_pad_x,
            lty,
            text_metrics.logical_size.x,
            text_metrics.logical_size.y,
        );
        text_layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            tint(s.text_text),
            spec.layer.get_z(),
        );

        // Value area: rust_soft fill proportional to value fraction.
        let frac = if spec.max > spec.min {
            ((state.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0)
        } else if spec.max < spec.min {
            ((state.value - spec.max) / (spec.min - spec.max)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        if frac > 0.0 {
            let draw_value_r = cmds.snap_to_physical_pixel(value_x + value_w * frac);
            cmds.push_crisp_fill_rect(
                Rect::from_ltrb(
                    draw_split_x,
                    draw_outer.y,
                    draw_value_r,
                    draw_outer.bottom(),
                ),
                tint(s.value_fill),
                spec.layer.get_z(),
            );
        }

        let value_text = format!("{:.2}", state.value);
        let value_layout = layout_text(
            text_backend,
            &value_text,
            s.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        let value_metrics = value_layout.metrics();
        let vtx = value_x + (value_w - value_metrics.logical_size.x) * 0.5;
        let vty = spec.rect.y + (spec.rect.h - value_metrics.logical_size.y) * 0.5;
        let value_rect = Rect::new(
            vtx,
            vty,
            value_metrics.logical_size.x,
            value_metrics.logical_size.y,
        );
        value_layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(value_rect.x, value_rect.y),
            tint(s.value_text),
            spec.layer.get_z(),
        );

        // Border pushed at the very end to draw on top of the value fill.
        let tinted_border = s.border.map(|b| Stroke::new(tint(b.color), b.width));
        cmds.push_crisp_border_rect(
            spec.rect,
            tinted_border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let hovered = contains
            && is_hover_active
            && !spec.disabled
            && (!input.mouse_down || state.is_dragging);

        let cursor_icon = if !spec.disabled && (contains || state.is_dragging) {
            Some(crate::output::CursorIcon::EwResize)
        } else {
            None
        };

        DragNumberResult {
            input: InputInfo {
                hovered,
                pressed: state.is_dragging && !spec.disabled,
                clicked: false,
            },
            focused,
            content_bounds: spec.rect.inset(s.border.map_or(0.0, |b| b.width)),
            cursor_icon,
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
    pub border: Option<Stroke>,
    pub focus: Option<Outline>,
    pub text_bg: Color,
    pub active_text_bg: Color,
    pub text_text: Color,
    pub value_text: Color,
    pub value_fill: Color,
    pub disabled_alpha: f32,
}

impl DragNumberStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_md,
            text_pad_x: 10.0,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_mono,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                -theme.focus_offset_tight,
            )),
            text_bg: theme.ink,
            active_text_bg: theme.rust,
            text_text: theme.paper,
            value_text: theme.ink,
            value_fill: theme.rust_soft_on_paper_elev,
            disabled_alpha: 0.35,
        }
    }
}

impl Default for DragNumberStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
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
    pub step: f32,
    pub page_step: f32,
    pub disabled: bool,
}

impl<'a> Default for DragNumberSpec<'a> {
    fn default() -> Self {
        Self {
            text: "",
            style: DragNumberStyle::default(),
            min: 0.0,
            max: 100.0,
            step: 1.0,
            page_step: 10.0,
            disabled: false,
        }
    }
}

impl<'a> DragNumberSpec<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            ..Self::default()
        }
    }

    pub fn new_from_theme(theme: &crate::theme::Theme, text: &'a str) -> Self {
        Self::new(text).theme(theme)
    }

    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = DragNumberStyle::from_theme(theme);
        self
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }

    pub fn style(mut self, style: DragNumberStyle) -> Self {
        self.style = style;
        self
    }

    pub fn min(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    pub fn max(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = step;
        self
    }

    pub fn page_step(mut self, page_step: f32) -> Self {
        self.page_step = page_step;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level drag number widget function using `WidgetContext`.
///
/// Runs the raw pre-layout phase to obtain a `SizeRequest`, resolves the final
/// rect with layout, then runs the raw post-layout phase.
pub fn drag_number<'a, T: TextBackend, S: LayoutState, CF>(
    spec: DragNumberSpec<'a>,
    layout_params: S::Params,
    state: &mut DragNumberState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> DragNumberResult {
    let pre_layout_spec = raw::DragNumberPreLayoutSpec {
        text: spec.text,
        style: spec.style,
        min: spec.min,
        max: spec.max,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_drag_number(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::DragNumberSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        min: spec.min,
        max: spec.max,
        step: spec.step,
        page_step: spec.page_step,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_drag_number(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    if let Some(cursor_icon) = result.cursor_icon {
        ctx.output.cursor_icon = Some(cursor_icon);
    }

    DragNumberResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "drag_number_tests.rs"]
mod tests;
