use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::{layout_text, TextBackend, TextBounds, TextStyle},
    types::{ClipRect, Color, Layer, Rect, Vec2},
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

        let (focused, clicked) = if spec.disabled {
            (false, false)
        } else {
            crate::focus::handle_widget_keyboard_focus(
                state.focus_id,
                Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h),
                spec.clip_rect,
                input,
                focus_system,
                crate::focus::FocusTraversalKeys::tab_only(),
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
            for (i, &tab_w) in widths.iter().enumerate() {
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
            anti_alias: false,
            p0: Vec2::new(spec.rect.x, border_y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, border_y),
            color: tint(s.border),
            width: s.border_width,
            z: spec.layer.get_z(),
        });

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
                cmds.push(DrawCmd::StrokeRect {
                    anti_alias: false,
                    rect: tab_rect.inset(-(s.focus_offset + s.focus_width)),
                    color: tint(s.focus),
                    width: s.focus_width,
                    z: spec.layer.get_focus_z(),
                });
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
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
                // Left uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
                // Right uptick (3px wide, 9px tall)
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(x + tab_w - 3.0, border_y - 7.5, 3.0, 9.0),
                    color: tint(s.accent),
                    z: spec.layer.get_z(),
                });
            }

            x += tab_w;
        }

        TabsResult {
            input: InputInfo {
                hovered: Rect::new(spec.rect.x, spec.rect.y, total_w, tab_h)
                    .contains(input.mouse_pos)
                    && spec.clip_rect.is_none_or(|c| c.contains(input.mouse_pos)),
                pressed: clicked && input.mouse_down,
                clicked: is_clicked,
            },
            focused,
            content_bounds: Rect::new(
                spec.rect.x,
                spec.rect.y,
                spec.rect.w,
                tab_h - s.border_width,
            ),
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
            border: theme.ink,
            text: theme.ink,
            inactive_text: theme.muted,
            accent: theme.rust,
            focus: theme.rust,
            border_width: theme.border,
            focus_width: theme.focus_width,
            focus_offset: theme.focus_offset,
            disabled_alpha: 0.35,
        }
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

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TabsSpecBuilder<'a> {
    pub items: Option<&'a [&'a str]>,
    pub style: Option<TabsStyle>,
    pub disabled: Option<bool>,
}

impl<'a> TabsSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: TabsStyle) -> Self {
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
            self.style = Some(TabsStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> TabsSpec<'a> {
        TabsSpec {
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            disabled: self.disabled.unwrap_or(false),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tabs widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn tabs<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TabsSpecBuilder<'a>,
    layout_params: S::Params,
    state: &mut TabsState,
) -> TabsResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
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

    TabsResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
        input: result.input,
        focused: result.focused,
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
