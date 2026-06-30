#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::{layout_text, TextBackend, TextBounds, TextStyle},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
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
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsPreLayoutSpec<'a> {
        pub items: &'a [&'a str],
        pub style: super::TabsStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TabsResult {
        pub input: InputInfo,
        pub focused: bool,
        pub content_bounds: Rect,
        pub cursor_icon: Option<crate::output::CursorIcon>,
    }

    /// Return the size this tabs widget would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_tabs<T: TextBackend>(
        spec: &TabsPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> TabsPreLayoutResult {
        TabsPreLayoutResult {
            size_request: tabs_size_request(spec, offer, text_backend),
        }
    }

    fn tabs_size_request<T: TextBackend>(
        spec: &TabsPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> SizeRequest {
        let s = spec.style;
        let mut total_w = 0.0_f32;
        for label in spec.items.iter() {
            let layout = layout_text(text_backend, label, s.text_style, TextBounds::UNBOUNDED);
            total_w += layout.metrics().logical_size.x + s.pad_x * 2.0;
        }
        SizeRequest::preferred(Vec2::new(total_w, s.height))
    }

    /// Low-level tabs widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_tabs<'a, T: TextBackend>(
        spec: TabsSpec<'a>,
        _pre_layout: TabsPreLayoutResult,
        state: &mut TabsState,
        input: &Input,
        focus_system: &mut FocusSystem,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> TabsResult {
        let s = spec.style;

        let tab_h = s.height;
        let pad_x = s.pad_x;
        let underbar_h = s.underbar_height;

        // Pre-layout all labels.
        let layouts: Vec<_> = spec
            .items
            .iter()
            .map(|label| layout_text(text_backend, label, s.text_style, TextBounds::UNBOUNDED))
            .collect();

        // Calculate tab widths and total width
        let widths: Vec<f32> = layouts
            .iter()
            .map(|l| l.metrics().logical_size.x + pad_x * 2.0)
            .collect();
        let total_w: f32 = widths.iter().sum();

        let tabs_rect = Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h);
        let keyboard = crate::widgets::widget_helpers::handle_keyboard_focus(
            state.focus_id,
            tabs_rect,
            spec.clip_rect,
            spec.disabled,
            crate::focus::FocusTraversalKeys::tab_only(),
            input,
            focus_system,
        );
        let focused = keyboard.focused;

        let raw_contains = tabs_rect.contains(input.mouse_pos)
            && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos));
        if raw_contains && !spec.disabled {
            focus_system.claim_hover(state.focus_id);
        }
        let hover_active = focus_system.is_hover_active(state.focus_id);

        let mut is_clicked = false;
        let mut any_passive_hover = false;
        let mut cursor_icon = None;
        let mut pressed = false;

        // Left/Right keyboard navigation
        if focused && !spec.disabled && !spec.items.is_empty() {
            if input.key_pressed(crate::input::Key::ArrowLeft) && state.active_index > 0 {
                state.active_index -= 1;
                is_clicked = true;
            }
            if input.key_pressed(crate::input::Key::ArrowRight)
                && state.active_index + 1 < spec.items.len()
            {
                state.active_index += 1;
                is_clicked = true;
            }
        }

        let mut hit_x = spec.rect.x;
        for (i, &tab_w) in widths.iter().enumerate() {
            let tab_rect = Rect::new(hit_x, spec.rect.y, tab_w, tab_h);
            // TODO: Tabs do not retain a pointer-active tab while the mouse is held,
            // so active_now/pressed are only true on press-start. Add active-tab
            // state, clear it on release, and pass was_active = active_tab == Some(i).
            let hover = crate::widgets::widget_helpers::handle_hover_interaction(
                tab_rect,
                spec.clip_rect,
                spec.disabled,
                hover_active,
                false,
                Some(crate::output::CursorIcon::Pointer),
                input,
            );
            any_passive_hover |= hover.passive_hovered;
            cursor_icon = cursor_icon.or(hover.cursor_icon);
            pressed |= hover.active_now && input.mouse_down;
            if hover.can_start {
                focus_system.take_keyboard_focus(state.focus_id);
                state.active_index = i;
                is_clicked = true;
            }
            hit_x += tab_w;
        }

        let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

        // Bottom border across the full width.
        let border_width = s.border.map_or(0.0, |stroke| stroke.width);
        let border_y = spec.rect.y + tab_h;
        let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
        cmds.push_crisp_h_rule(
            spec.rect.x,
            border_y - border_width,
            spec.rect.w,
            s.border.map(tint_stroke),
            spec.layer.get_z(),
        );

        let mut draw_edges = Vec::with_capacity(widths.len() + 1);
        let mut edge_x = spec.rect.x;
        draw_edges.push(cmds.snap_to_physical_pixel(edge_x));
        for width in &widths {
            edge_x += *width;
            draw_edges.push(cmds.snap_to_physical_pixel(edge_x));
        }
        let draw_border_y = cmds.snap_to_physical_pixel(border_y);
        let draw_underbar_top = cmds.snap_to_physical_pixel(border_y - underbar_h);
        let draw_uptick_top = cmds.snap_to_physical_pixel(border_y - 9.0);
        let draw_uptick_w = cmds.snap_length_to_physical_pixels(3.0);

        let mut x = spec.rect.x;

        for (i, (_label, layout)) in spec.items.iter().zip(layouts.iter()).enumerate() {
            let is_active = i == state.active_index;

            let metrics = layout.metrics();
            let tab_w = metrics.logical_size.x + pad_x * 2.0;
            let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

            let text_h = metrics.logical_size.y;
            let text_w = metrics.logical_size.x;

            // Focus ring.
            let visually_focused = focused && i == state.active_index;
            if visually_focused && !spec.disabled {
                if let Some(outline) = s.focus {
                    let tint_stroke = |st: Stroke| Stroke::new(tint(st.color), st.width);
                    cmds.push_crisp_border_rect(
                        tab_rect.inset(-outline.offset),
                        Some(tint_stroke(outline.stroke)),
                        BorderPlacement::Outside,
                        spec.layer.get_focus_z(),
                    );
                }
            }

            let text_color = if is_active { s.text } else { s.inactive_text };
            let ty = spec.rect.y + (tab_h - text_h) * 0.5;
            let text_rect = Rect::new(x + pad_x, ty, text_w, text_h);
            layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(text_rect.x, text_rect.y),
                tint(text_color),
                spec.layer.get_z(),
            );

            // Active underbar: 3px rust rect sitting on the bottom border + upticks at the ends.
            if is_active {
                let draw_l = draw_edges[i];
                let draw_r = draw_edges[i + 1];
                cmds.push_crisp_fill_rect(
                    Rect::from_ltrb(draw_l, draw_underbar_top, draw_r, draw_border_y),
                    tint(s.accent),
                    spec.layer.get_z(),
                );
                // Left uptick (3px wide, 9px tall)
                cmds.push_crisp_fill_rect(
                    Rect::from_ltrb(
                        draw_l,
                        draw_uptick_top,
                        draw_l + draw_uptick_w,
                        draw_border_y,
                    ),
                    tint(s.accent),
                    spec.layer.get_z(),
                );
                // Right uptick (3px wide, 9px tall)
                cmds.push_crisp_fill_rect(
                    Rect::from_ltrb(
                        draw_r - draw_uptick_w,
                        draw_uptick_top,
                        draw_r,
                        draw_border_y,
                    ),
                    tint(s.accent),
                    spec.layer.get_z(),
                );
            }

            x += tab_w;
        }

        TabsResult {
            input: InputInfo {
                hovered: any_passive_hover,
                pressed,
                clicked: is_clicked,
            },
            focused,
            content_bounds: Rect::new(spec.rect.x, spec.rect.y, spec.rect.w, tab_h - border_width),
            cursor_icon,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsStyle {
    pub height: f32,
    pub pad_x: f32,
    pub underbar_height: f32,
    pub text_style: TextStyle,
    pub border: Option<Stroke>,
    pub text: Color,
    pub inactive_text: Color,
    pub accent: Color,
    pub focus: Option<Outline>,
    pub disabled_alpha: f32,
}

impl TabsStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            height: 36.0,
            pad_x: 18.0,
            underbar_height: 3.0,
            text_style: TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            border: Some(Stroke::new(theme.ink, theme.border)),
            text: theme.ink,
            inactive_text: theme.muted,
            accent: theme.rust,
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                theme.focus_offset,
            )),
            disabled_alpha: 0.35,
        }
    }
}

impl Default for TabsStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
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

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TabsSpec<'a> {
    pub items: &'a [&'a str],
    pub style: TabsStyle,
    pub disabled: bool,
}

impl<'a> TabsSpec<'a> {
    pub fn new(items: &'a [&'a str]) -> Self {
        Self {
            items,
            style: TabsStyle::default(),
            disabled: false,
        }
    }

    pub fn new_from_theme(items: &'a [&'a str], theme: &crate::theme::Theme) -> Self {
        Self::new(items).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = TabsStyle::from_theme(theme);
        self
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = items;
        self
    }

    pub fn style(mut self, style: TabsStyle) -> Self {
        self.style = style;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

// -- High-level widget function ---------------------------------------------------

/// High-level tabs widget function using `WidgetContext`.
///
/// Consumes a complete `TabsSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
///
/// The widget is stateful: interaction, focus, and the active index state are
/// tracked via `state`. Cursor icons are propagated to `ctx.output.cursor_icon`
/// when appropriate.
pub fn tabs<'a, T: TextBackend, S: LayoutState, CF>(
    spec: TabsSpec<'a>,
    layout_params: S::Params,
    state: &mut TabsState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> TabsResult {
    let pre_layout_spec = raw::TabsPreLayoutSpec {
        items: spec.items,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_tabs(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::TabsSpec {
        rect,
        items: spec.items,
        style: spec.style,
        disabled: spec.disabled,
        clip_rect: ctx.clip_rect,
        layer: ctx.layer,
    };

    let result = raw::post_layout_tabs(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );

    ctx.request_cursor(result.cursor_icon);

    TabsResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
