#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
    widget::{InputInfo, LayoutInfo, WidgetContext},
    TextBackend,
};

pub mod raw {
    use crate::text::layout_text;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipSpec<'a> {
        pub layer: Layer,
        /// Top-left origin. Height is fixed at 22.
        pub rect: Rect,
        pub text: &'a str,
        pub style: super::ChipStyle,
        pub disabled: bool,
        pub clip_rect: ClipRect,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipPreLayoutSpec<'a> {
        pub text: &'a str,
        pub style: super::ChipStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ChipResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this chip would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_chip<T: TextBackend>(
        spec: &ChipPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> ChipPreLayoutResult {
        ChipPreLayoutResult {
            size_request: chip_size_request(spec, offer, text_backend),
        }
    }

    fn chip_size_request<T: TextBackend>(
        spec: &ChipPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds::UNBOUNDED,
        );
        crate::layout::SizeRequest::preferred(layout.metrics().logical_size)
    }

    /// Low-level chip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_chip<'a, T: TextBackend>(
        spec: ChipSpec<'a>,
        _pre_layout: ChipPreLayoutResult,
        state: &mut ChipState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> ChipResult {
        let interaction = crate::widgets::widget_helpers::handle_press_interaction(
            crate::widgets::widget_helpers::PressInteractionSpec {
                focus_id: state.focus_id,
                rect: spec.rect,
                clip_rect: spec.clip_rect,
                disabled: spec.disabled,
                traversal_keys: crate::focus::FocusTraversalKeys::all(),
            },
            input,
            focus_system,
            &mut state.is_active,
            &mut state.space_is_active,
        );
        let focused = interaction.focused;
        let is_clicked = interaction.input.clicked;

        if is_clicked {
            state.checked = !state.checked;
        }

        let s = spec.style;
        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        let h = s.height;
        let pad_x = s.pad_x;

        let layout = layout_text(
            text_backend,
            spec.text,
            spec.style.text_style,
            crate::text::TextBounds {
                max_width: Some(spec.rect.w),
                max_height: Some(spec.rect.h),
            },
        );
        let w = spec.rect.w.max(32.0);
        let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

        // Focus ring.
        if focused {
            if let Some(outline) = s.focus {
                let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                cmds.push_crisp_border_rect(
                    r.inset(-outline.offset),
                    Some(tint_stroke(outline.stroke)),
                    BorderPlacement::Outside,
                    spec.layer.get_focus_z(),
                );
            }
        }

        let bg = if state.checked {
            s.active_bg
        } else {
            s.background
        };
        cmds.push_crisp_fill_rect(r, tint(bg), spec.layer.get_z());

        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_crisp_border_rect(
            r,
            s.border.map(tint_stroke),
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );

        let text_color = if state.checked { s.active_text } else { s.text };
        let metrics = layout.metrics();
        let ty = r.y + (h - metrics.logical_size.y) * 0.5;
        let text_rect = Rect::new(
            r.x + pad_x,
            ty,
            metrics.logical_size.x,
            metrics.logical_size.y,
        );
        layout.emit_glyphs(
            cmds,
            text_backend,
            Vec2::new(text_rect.x, text_rect.y),
            tint(text_color),
            spec.layer.get_z(),
        );

        let cursor_icon = if interaction.input.hovered && !spec.disabled {
            Some(crate::output::CursorIcon::Pointer)
        } else {
            None
        };

        ChipResult {
            input: interaction.input,
            focused,
            content_bounds: r.inset(s.border.map_or(0.0, |st| st.width)),
            cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_style: crate::text::TextStyle,
    pub background: Color,
    pub active_bg: Color,
    pub border: Option<Stroke>,
    pub text: Color,
    pub active_text: Color,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

impl Default for ChipStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

impl ChipStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: theme.h_sm,
            pad_x: 8.0,
            text_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            active_bg: theme.ink,
            border: Some(Stroke::new(theme.ink, theme.border)),
            text: theme.ink,
            active_text: theme.paper,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.35,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChipState {
    pub checked: bool,
    pub is_active: bool,
    pub space_is_active: bool,
    pub focus_id: FocusId,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ChipResult {
    pub layout: LayoutInfo,
    pub input: InputInfo,
    pub focused: bool,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ChipSpec<'a> {
    pub text: &'a str,
    pub style: ChipStyle,
    pub disabled: bool,
}

impl<'a> ChipSpec<'a> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            style: ChipStyle::default(),
            disabled: false,
        }
    }

    pub fn new_from_theme(text: &'a str, theme: &crate::theme::Theme) -> Self {
        Self::new(text).theme(theme)
    }

    /// Overwrites `style` with the style derived from `theme`.
    /// Leaves `text` and `disabled` unchanged.
    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = ChipStyle::from_theme(theme);
        self
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = text;
        self
    }

    pub fn style(mut self, style: ChipStyle) -> Self {
        self.style = style;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level chip widget function using `WidgetContext`.
///
/// Consumes a complete `ChipSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
///
/// The chip is stateful: it tracks checked/unchecked state, keyboard focus,
/// and space-bar activation via `state`. The cursor icon is propagated to
/// `ctx.output.cursor_icon` when the chip is hovered and enabled.
pub fn chip<'a, T: TextBackend, S: LayoutState, CF>(
    spec: ChipSpec<'a>,
    layout_params: S::Params,
    state: &mut ChipState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ChipResult {
    let pre_layout_spec = raw::ChipPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_chip(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ChipSpec {
        layer: ctx.layer,
        rect,
        text: spec.text,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
    };
    let result = raw::post_layout_chip(
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

    ChipResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "chip_tests.rs"]
mod tests;
