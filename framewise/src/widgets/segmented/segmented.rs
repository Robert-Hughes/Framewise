use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
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
    pub struct SegmentedSpec<'a> {
        pub layer: Layer,
        /// Top-left origin. Height is fixed at h_md (28).
        pub rect: Rect,
        pub items: &'a [&'a str],
        pub style: super::SegmentedStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedPreLayoutSpec<'a> {
        pub items: &'a [&'a str],
        pub style: super::SegmentedStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SegmentedResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
    }

    /// Return the size this segmented control would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_segmented<T: TextBackend>(
        spec: &SegmentedPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> SegmentedPreLayoutResult {
        SegmentedPreLayoutResult {
            size_request: segmented_size_request(spec, offer, text_backend),
        }
    }

    fn segmented_size_request<T: TextBackend>(
        spec: &SegmentedPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let s = spec.style;
        let total_w = spec
            .items
            .iter()
            .map(|text| {
                let layout = layout_text(
                    text_backend,
                    text,
                    s.text_style,
                    crate::text::TextBounds::UNBOUNDED,
                );
                layout.metrics().logical_size.x + s.pad_x * 2.0
            })
            .sum();
        crate::layout::SizeRequest::preferred(Vec2::new(total_w, s.height))
    }

    /// Low-level segmented widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_segmented<'a, T: TextBackend>(
        spec: SegmentedSpec<'a>,
        _pre_layout: SegmentedPreLayoutResult,
        state: &mut SegmentedState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> SegmentedResult {
        let s = spec.style;

        if spec.items.is_empty() {
            return SegmentedResult {
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

        // Pre-layout all labels to get their widths.
        let layouts: Vec<_> = spec
            .items
            .iter()
            .map(|text| {
                layout_text(
                    text_backend,
                    text,
                    s.text_style,
                    crate::text::TextBounds::UNBOUNDED,
                )
            })
            .collect();
        let widths: Vec<f32> = layouts
            .iter()
            .map(|l| l.metrics().logical_size.x + pad_x * 2.0)
            .collect();
        let total_w: f32 = widths.iter().sum();

        let outer = Rect::new(spec.rect.x, spec.rect.y, total_w, h);

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                outer,
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

        let border_width = s.border.map_or(0.0, |stroke| stroke.width);
        cmds.push(DrawCmd::FillRect {
            rect: outer,
            color: tint(s.background),
            z: spec.layer.get_z(),
        });
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_border_rect(
            outer,
            s.border.map(tint_stroke),
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let mut x = spec.rect.x;
        for (i, ((_label, layout), &w)) in spec
            .items
            .iter()
            .zip(layouts.iter())
            .zip(widths.iter())
            .enumerate()
        {
            let metric = layout.metrics();
            let is_active = i == state.active_index;
            let seg_rect = Rect::new(x, spec.rect.y, w, h);

            if is_active {
                cmds.push(DrawCmd::FillRect {
                    rect: seg_rect,
                    color: tint(s.active_bg),
                    z: spec.layer.get_z(),
                });
            }

            // Focus ring (inset to stay within bounds).
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                if let Some(outline) = s.focus {
                    let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                    cmds.push_border_rect(
                        seg_rect.inset(-outline.offset),
                        Some(tint_stroke(outline.stroke)),
                        BorderPlacement::Outside,
                        spec.layer.get_focus_z(),
                    );
                }
            }

            // Divider between segments (right edge, except last).
            if i + 1 < spec.items.len() {
                let div_x = x + w;
                cmds.push_v_rule(
                    div_x,
                    spec.rect.y,
                    h,
                    s.border.map(tint_stroke),
                    spec.layer.get_z(),
                );
            }

            let text_color = if is_active { s.active_text } else { s.text };
            let ty = spec.rect.y + (h - metric.logical_size.y) * 0.5;
            let text_rect = Rect::new(x + pad_x, ty, metric.logical_size.x, metric.logical_size.y);
            layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(text_rect.x, text_rect.y),
                tint(text_color),
                spec.layer.get_z(),
            );

            x += w;
        }

        SegmentedResult {
            input: InputInfo {
                hovered: outer.contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            focused,
            content_bounds: outer.inset(border_width),
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentedStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Option<Stroke>,
    pub active_bg: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

impl SegmentedStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_md,
            pad_x: 14.0,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            active_bg: theme.ink,
            text: theme.ink,
            active_text: theme.paper,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                -theme.focus_offset - theme.focus_width,
            )),
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SegmentedState {
    pub active_index: usize,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SegmentedSpec<'a> {
    pub items: &'a [&'a str],
    pub style: SegmentedStyle,
    pub disabled: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SegmentedSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub style: Option<SegmentedStyle>,
    pub disabled: Option<bool>,
}

impl<'a> SegmentedSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: SegmentedStyle) -> Self {
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
            self.style = Some(SegmentedStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SegmentedSpec<'a> {
        SegmentedSpec {
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level segmented widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn segmented<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SegmentedSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut SegmentedState,
) -> SegmentedResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::SegmentedPreLayoutSpec {
        items: spec.items,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_segmented(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SegmentedSpec {
        layer: ctx.layer,
        rect,
        items: spec.items,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_segmented(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    SegmentedResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "segmented_tests.rs"]
mod tests;
