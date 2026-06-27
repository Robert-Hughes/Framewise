use crate::{
    draw::{BorderPlacement, DrawCommands},
    layout::{LayoutState, SizeOffer},
    text::{layout_text, TextBackend},
    types::{Color, Layer, Rect, Stroke, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuSpec<'a> {
        pub layer: Layer,
        /// Top-left origin; width is at least 200.
        pub rect: Rect,
        pub items: &'a [super::MenuItem<'a>],
        pub style: super::MenuStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuPreLayoutSpec<'a> {
        pub items: &'a [super::MenuItem<'a>],
        pub style: super::MenuStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuResult {
        pub bounds: Rect,
        pub content_bounds: Rect,
    }

    /// Return the size this menu would request under `offer`.
    ///
    /// This currently measures text with unbounded bounds; offer-sensitive
    /// wrapping is future work.
    pub fn pre_layout_menu<T: TextBackend>(
        spec: &MenuPreLayoutSpec,
        offer: SizeOffer,
        text_backend: &mut T,
    ) -> MenuPreLayoutResult {
        MenuPreLayoutResult {
            size_request: menu_size_request(spec, offer, text_backend),
        }
    }

    fn menu_size_request<T: TextBackend>(
        spec: &MenuPreLayoutSpec,
        _offer: SizeOffer,
        text_backend: &mut T,
    ) -> crate::layout::SizeRequest {
        let s = spec.style;
        let mut widest: f32 = s.min_width;
        let mut height = s.pad_y * 2.0;
        for item in spec.items {
            match item {
                MenuItem::Item {
                    label, shortcut, ..
                } => {
                    height += s.row_height;
                    let label_layout = layout_text(
                        text_backend,
                        label,
                        s.label_style,
                        crate::text::TextBounds::UNBOUNDED,
                    );
                    let label_w = label_layout.metrics().logical_size.x;
                    let shortcut_w = shortcut
                        .map(|sc| {
                            let sc_layout = layout_text(
                                text_backend,
                                sc,
                                s.meta_style,
                                crate::text::TextBounds::UNBOUNDED,
                            );
                            sc_layout.metrics().logical_size.x
                        })
                        .unwrap_or(0.0);
                    widest = widest.max(label_w + shortcut_w + s.pad_x * 3.0);
                }
                MenuItem::Separator => height += s.separator_height,
                MenuItem::Group(label) => {
                    height += s.group_height;
                    let group_layout = layout_text(
                        text_backend,
                        label,
                        s.meta_style,
                        crate::text::TextBounds::UNBOUNDED,
                    );
                    widest = widest.max(group_layout.metrics().logical_size.x + s.pad_x * 2.0);
                }
            }
        }
        crate::layout::SizeRequest::preferred(Vec2::new(widest, height))
    }

    /// Low-level menu widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_menu<'a, T: TextBackend>(
        spec: MenuSpec<'a>,
        _pre_layout: MenuPreLayoutResult,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> MenuResult {
        let s = spec.style;

        let row_h = s.row_height;
        let sep_h = s.separator_height;
        let group_h = s.group_height;
        let pad_x = s.pad_x;

        let total_h: f32 = spec
            .items
            .iter()
            .map(|item| match item {
                MenuItem::Item { .. } => row_h,
                MenuItem::Separator => sep_h,
                MenuItem::Group(_) => group_h,
            })
            .sum::<f32>()
            + s.pad_y * 2.0;

        let w = spec.rect.w.max(s.min_width);
        let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

        cmds.push_crisp_fill_rect(outer, s.background, spec.layer.get_z());
        let border_width = s.border.map_or(0.0, |stroke| stroke.width);
        cmds.push_crisp_border_rect(outer, s.border, BorderPlacement::Inside, spec.layer.get_z());

        let mut y = spec.rect.y + s.pad_y;

        for item in spec.items {
            match item {
                MenuItem::Separator => {
                    let sep_y = y + s.separator_y;
                    cmds.push_crisp_h_rule(outer.x, sep_y, w, s.separator, spec.layer.get_z());
                    y += sep_h;
                }
                MenuItem::Group(label) => {
                    let ty = y + s.group_text_y;
                    let layout = layout_text(
                        text_backend,
                        label,
                        s.meta_style,
                        crate::text::TextBounds::UNBOUNDED,
                    );
                    let metrics = layout.metrics();
                    let rect = Rect::new(
                        outer.x + pad_x,
                        ty,
                        metrics.logical_size.x,
                        metrics.logical_size.y,
                    );
                    layout.emit_glyphs(
                        cmds,
                        text_backend,
                        Vec2::new(rect.x, rect.y),
                        s.muted,
                        spec.layer.get_z(),
                    );
                    y += group_h;
                }
                MenuItem::Item {
                    label,
                    shortcut,
                    selected,
                    disabled,
                } => {
                    let alpha = if *disabled { s.disabled_alpha } else { 1.0 };
                    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

                    let row_rect = Rect::new(outer.x, y, w, row_h);

                    if *selected {
                        cmds.push_crisp_fill_rect(
                            row_rect,
                            tint(s.selected_bg),
                            spec.layer.get_z(),
                        );
                    }

                    let text_color = if *selected {
                        tint(s.selected_text)
                    } else {
                        tint(s.text)
                    };
                    let layout = layout_text(
                        text_backend,
                        label,
                        s.label_style,
                        crate::text::TextBounds::UNBOUNDED,
                    );
                    let metrics = layout.metrics();
                    let ty = y + (row_h - metrics.logical_size.y) * 0.5;
                    let rect = Rect::new(
                        outer.x + pad_x,
                        ty,
                        metrics.logical_size.x,
                        metrics.logical_size.y,
                    );
                    layout.emit_glyphs(
                        cmds,
                        text_backend,
                        Vec2::new(rect.x, rect.y),
                        text_color,
                        spec.layer.get_z(),
                    );

                    if let Some(sc) = shortcut {
                        let sc_color = if *selected {
                            Color::linear_rgba(
                                s.selected_text.r,
                                s.selected_text.g,
                                s.selected_text.b,
                                s.shortcut_selected_alpha * alpha,
                            )
                        } else {
                            tint(s.muted)
                        };
                        let sc_layout = layout_text(
                            text_backend,
                            sc,
                            s.meta_style,
                            crate::text::TextBounds::UNBOUNDED,
                        );
                        let sc_metrics = sc_layout.metrics();
                        let sc_x = outer.x + w - pad_x - sc_metrics.logical_size.x;
                        let sc_ty = y + (row_h - sc_metrics.logical_size.y) * 0.5;
                        let sc_rect = Rect::new(
                            sc_x,
                            sc_ty,
                            sc_metrics.logical_size.x,
                            sc_metrics.logical_size.y,
                        );
                        sc_layout.emit_glyphs(
                            cmds,
                            text_backend,
                            Vec2::new(sc_rect.x, sc_rect.y),
                            sc_color,
                            spec.layer.get_z(),
                        );
                    }

                    y += row_h;
                }
            }
        }

        let content_bounds = Rect::new(
            outer.x + border_width + s.pad_x,
            outer.y + border_width + s.pad_y,
            outer.w - (border_width + s.pad_x) * 2.0,
            outer.h - (border_width + s.pad_y) * 2.0,
        );

        MenuResult {
            bounds: outer,
            content_bounds,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItem<'a> {
    Item {
        label: &'a str,
        shortcut: Option<&'a str>,
        selected: bool,
        disabled: bool,
    },
    Separator,
    Group(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStyle {
    pub row_height: f32,
    pub separator_height: f32,
    pub group_height: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub group_text_y: f32,
    pub separator_y: f32,
    pub min_width: f32,
    pub label_style: crate::text::TextStyle,
    pub meta_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Option<Stroke>,
    pub separator: Option<Stroke>,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub text: Color,
    pub muted: Color,
    pub shortcut_selected_alpha: f32,
    pub disabled_alpha: f32,
}

impl MenuStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            row_height: theme.row_height,
            separator_height: 9.0,
            group_height: 22.0,
            pad_x: 12.0,
            pad_y: 4.0,
            group_text_y: 8.0,
            separator_y: 4.0,
            min_width: 200.0,
            label_style: crate::text::TextStyle::new(
                theme.sans_font,
                theme.text_md,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            meta_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            separator: Some(Stroke::new(theme.line_on_paper_elev, theme.border)),
            selected_bg: theme.ink,
            selected_text: theme.paper,
            text: theme.ink,
            muted: theme.muted,
            shortcut_selected_alpha: 0.6,
            disabled_alpha: 0.4,
        }
    }
}

impl Default for MenuStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MenuResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MenuSpec<'a> {
    pub items: &'a [MenuItem<'a>],
    pub style: MenuStyle,
}

impl<'a> MenuSpec<'a> {
    pub fn new(items: &'a [MenuItem<'a>]) -> Self {
        Self {
            items,
            style: MenuStyle::default(),
        }
    }

    pub fn new_from_theme(items: &'a [MenuItem<'a>], theme: &crate::theme::Theme) -> Self {
        Self::new(items).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = MenuStyle::from_theme(theme);
        self
    }

    pub fn items(mut self, items: &'a [MenuItem<'a>]) -> Self {
        self.items = items;
        self
    }

    pub fn style(mut self, style: MenuStyle) -> Self {
        self.style = style;
        self
    }
}

/// High-level menu widget function using `WidgetContext`.
///
/// Consumes a complete `MenuSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
pub fn menu<'a, T: TextBackend, S: LayoutState, CF>(
    spec: MenuSpec<'a>,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> MenuResult {
    let pre_layout_spec = raw::MenuPreLayoutSpec {
        items: spec.items,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_menu(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::MenuSpec {
        layer: ctx.layer,
        rect,
        items: spec.items,
        style: spec.style,
    };
    let result = raw::post_layout_menu(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);
    MenuResult {
        layout: LayoutInfo::new(result.bounds, result.content_bounds),
    }
}

#[cfg(test)]
#[path = "menu_tests.rs"]
mod tests;
