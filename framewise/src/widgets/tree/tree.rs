#[cfg(test)]
use crate::draw::DrawCmd;
use crate::{
    draw::{BorderPlacement, DrawCommands},
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::{layout_text, TextBackend, TextBounds, TextStyle},
    types::{Color, Layer, Rect, Stroke, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreeSpec<'a> {
        pub rect: Rect,
        pub items: &'a [super::TreeRow<'a>],
        pub style: super::TreeStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreePreLayoutSpec<'a> {
        pub items: &'a [super::TreeRow<'a>],
        pub style: super::TreeStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreePreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreeResult {
        pub bounds: Rect,
        pub content_bounds: Rect,
    }

    /// Return the size this tree widget would request under `offer`.
    ///
    /// The current implementation ignores `offer` because the request is derived
    /// from row count and tree style.
    pub fn pre_layout_tree(spec: &TreePreLayoutSpec, offer: SizeOffer) -> TreePreLayoutResult {
        TreePreLayoutResult {
            size_request: tree_size_request(spec, offer),
        }
    }

    fn tree_size_request(spec: &TreePreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        let s = spec.style;
        let total_h = spec.items.len() as f32 * s.row_height + s.pad_y * 2.0;
        SizeRequest::preferred(Vec2::new(s.min_width, total_h))
    }

    /// Low-level tree widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn post_layout_tree<'a, T: TextBackend>(
        spec: TreeSpec<'a>,
        _pre_layout: TreePreLayoutResult,
        text_backend: &mut T,
        cmds: &mut DrawCommands,
    ) -> TreeResult {
        let s = spec.style;

        let row_h = s.row_height;
        let indent_w = s.indent_width;
        let caret_w = s.caret_width;
        let pad_x = s.pad_x;
        let total_h = spec.items.len() as f32 * row_h + s.pad_y * 2.0;
        let w = spec.rect.w.max(s.min_width);
        let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

        cmds.push_crisp_fill_rect(outer, s.background, spec.layer.get_z());
        let border_width = s.border.map_or(0.0, |stroke| stroke.width);
        cmds.push_crisp_border_rect(outer, s.border, BorderPlacement::Inside, spec.layer.get_z());

        let mut y = spec.rect.y + s.pad_y;

        for row in spec.items {
            let row_rect = Rect::new(outer.x, y, w, row_h);

            if row.selected {
                cmds.push_crisp_fill_rect(row_rect, s.selected_bg, spec.layer.get_z());
            }

            let text_color = if row.selected {
                s.selected_text
            } else {
                s.text
            };
            let meta_color: Color = if row.selected {
                Color::linear_rgba(
                    s.selected_text.r,
                    s.selected_text.g,
                    s.selected_text.b,
                    s.selected_meta_alpha,
                )
            } else {
                s.muted
            };
            let caret_color = if row.selected { meta_color } else { s.muted };

            let indent_x = outer.x + pad_x + row.indent as f32 * indent_w;

            // Caret symbol.
            let caret_sym = match row.caret {
                Some(true) => "v",
                Some(false) => ">",
                None => " ",
            };
            let caret_layout =
                layout_text(text_backend, caret_sym, s.text_style, TextBounds::UNBOUNDED);
            let caret_metrics = caret_layout.metrics();
            let cty = y + (row_h - caret_metrics.logical_size.y) * 0.5;
            let caret_rect = Rect::new(
                indent_x,
                cty,
                caret_metrics.logical_size.x,
                caret_metrics.logical_size.y,
            );
            caret_layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(caret_rect.x, caret_rect.y),
                caret_color,
                spec.layer.get_z(),
            );

            // Label.
            let label_layout =
                layout_text(text_backend, row.label, s.text_style, TextBounds::UNBOUNDED);
            let label_metrics = label_layout.metrics();
            let lty = y + (row_h - label_metrics.logical_size.y) * 0.5;
            let label_rect = Rect::new(
                indent_x + caret_w,
                lty,
                label_metrics.logical_size.x,
                label_metrics.logical_size.y,
            );
            label_layout.emit_glyphs(
                cmds,
                text_backend,
                Vec2::new(label_rect.x, label_rect.y),
                text_color,
                spec.layer.get_z(),
            );

            // Meta (right-aligned).
            if let Some(meta) = row.meta {
                let meta_layout =
                    layout_text(text_backend, meta, s.text_style, TextBounds::UNBOUNDED);
                let meta_metrics = meta_layout.metrics();
                let mx = outer.x + w - pad_x - meta_metrics.logical_size.x;
                let mty = y + (row_h - meta_metrics.logical_size.y) * 0.5;
                let meta_rect = Rect::new(
                    mx,
                    mty,
                    meta_metrics.logical_size.x,
                    meta_metrics.logical_size.y,
                );
                meta_layout.emit_glyphs(
                    cmds,
                    text_backend,
                    Vec2::new(meta_rect.x, meta_rect.y),
                    meta_color,
                    spec.layer.get_z(),
                );
            }

            y += row_h;
        }

        let content_bounds = Rect::new(
            outer.x + border_width + s.pad_x,
            outer.y + border_width + s.pad_y,
            outer.w - (border_width + s.pad_x) * 2.0,
            outer.h - (border_width + s.pad_y) * 2.0,
        );

        TreeResult {
            bounds: outer,
            content_bounds,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TreeRow<'a> {
    pub indent: u32,
    /// None = leaf, true = expanded, false = collapsed.
    pub caret: Option<bool>,
    pub label: &'a str,
    /// Optional right-aligned metadata string.
    pub meta: Option<&'a str>,
    pub selected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeStyle {
    pub row_height: f32,
    pub indent_width: f32,
    pub caret_width: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub min_width: f32,
    pub text_style: TextStyle,
    pub background: Color,
    pub border: Option<Stroke>,
    pub selected_bg: Color,
    pub text: Color,
    pub selected_text: Color,
    pub muted: Color,
    pub selected_meta_alpha: f32,
}

impl TreeStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            row_height: 20.0,
            indent_width: 14.0,
            caret_width: 12.0,
            pad_x: 10.0,
            pad_y: 4.0,
            min_width: 280.0,
            text_style: TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            selected_bg: theme.ink,
            text: theme.ink,
            selected_text: theme.paper,
            muted: theme.muted,
            selected_meta_alpha: 0.7,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TreeResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TreeSpec<'a> {
    pub items: &'a [TreeRow<'a>],
    pub style: TreeStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeSpecBuilder<'a> {
    pub items: Option<&'a [TreeRow<'a>]>,
    pub style: Option<TreeStyle>,
}

impl<'a> TreeSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [TreeRow<'a>]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(TreeStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> TreeSpec<'a> {
        TreeSpec {
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tree widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn tree<'a, T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TreeSpecBuilder<'a>,
    layout_params: S::Params,
) -> TreeResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::TreePreLayoutSpec {
        items: spec.items,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_tree(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::TreeSpec {
        rect,
        items: spec.items,
        style: spec.style,
        layer: ctx.layer,
    };

    let result = raw::post_layout_tree(raw_spec, pre_layout, ctx.text_backend, ctx.cmds);
    TreeResult {
        layout: LayoutInfo::new(result.bounds, result.content_bounds),
    }
}

#[cfg(test)]
#[path = "tree_tests.rs"]
mod tests;
