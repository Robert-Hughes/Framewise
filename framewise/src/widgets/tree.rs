use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    widget::WidgetContext,
};

pub mod raw {
    use super::*;

    /// Low-level tree widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tree<'a, T: crate::text::TextSystem>(spec: TreeSpec<'a>,
        text_system: &mut T) -> TreeResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let row_h = s.row_height;
        let indent_w = s.indent_width;
        let caret_w = s.caret_width;
        let pad_x = s.pad_x;
        let total_h = spec.rows.len() as f32 * row_h + s.pad_y * 2.0;
        let w = spec.rect.w.max(s.min_width);
        let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

        cmds.push(DrawCmd::FillRect {
            rect: outer,
            color: s.background,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: outer,
            color: s.border,
            width: s.border_width,
        });

        let mut y = spec.rect.y + s.pad_y;

        for row in spec.rows {
            let row_rect = Rect::new(outer.x, y, w, row_h);

            if row.selected {
                cmds.push(DrawCmd::FillRect {
                    rect: row_rect,
                    color: s.selected_bg,
                });
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
            let caret_layout = text_system.prepare(caret_sym, s.text_size, spec.font);
            let cty = y + (row_h - caret_layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(indent_x, cty, caret_layout.size.x, caret_layout.size.y),
                color: caret_color,
                handle: caret_layout.handle,
            });

            // Label.
            let label_layout = text_system.prepare(row.label, s.text_size, spec.font);
            let lty = y + (row_h - label_layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(
                    indent_x + caret_w,
                    lty,
                    label_layout.size.x,
                    label_layout.size.y,
                ),
                color: text_color,
                handle: label_layout.handle,
            });

            // Meta (right-aligned).
            if let Some(meta) = row.meta {
                let meta_layout = text_system.prepare(meta, s.text_size, spec.font);
                let mx = outer.x + w - pad_x - meta_layout.size.x;
                let mty = y + (row_h - meta_layout.size.y) * 0.5;
                cmds.push(DrawCmd::Text {
                    rect: Rect::new(mx, mty, meta_layout.size.x, meta_layout.size.y),
                    color: meta_color,
                    handle: meta_layout.handle,
                });
            }

            y += row_h;
        }

        TreeResult { draw: cmds }
    }
}

pub struct TreeRow<'a> {
    pub indent: u32,
    /// None = leaf, true = expanded, false = collapsed.
    pub caret: Option<bool>,
    pub label: &'a str,
    /// Optional right-aligned metadata string.
    pub meta: Option<&'a str>,
    pub selected: bool,
}

pub struct TreeSpec<'a> {
    pub rect: Rect,
    pub rows: &'a [TreeRow<'a>],
    pub font: FontId,
    pub style: TreeStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeStyle {
    pub row_height: f32,
    pub indent_width: f32,
    pub caret_width: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub min_width: f32,
    pub text_size: f32,
    pub background: Color,
    pub border: Color,
    pub selected_bg: Color,
    pub text: Color,
    pub selected_text: Color,
    pub muted: Color,
    pub selected_meta_alpha: f32,
    pub border_width: f32,
}

pub struct TreeResult {
    pub draw: DrawCommands,
}

impl TreeResult {
    pub fn into_parts(self) -> (DrawCommands, ()) {
        (self.draw, ())
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tree widget function using WidgetContext.
///
/// This function accepts a TreeSpec and calls the low-level raw::tree function.
pub fn tree<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    layout_params: S::Params,
    builder: TreeSpecBuilder<'a>,
) {
    let rect = ctx.layout(layout_params);
    let builder = builder
        .with_rect(rect)
        .with_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::tree(spec, ctx.text_system);
    ctx.append_cmds(result.draw.0);
}

pub struct TreeSpecBuilder<'a> {
    pub rows: Option<&'a [TreeRow<'a>]>,
    pub font: Option<FontId>,
    pub style: Option<TreeStyle>,
    pub rect: Option<Rect>,
}

impl<'a> TreeSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            rows: None,
            font: None,
            style: None,
            rect: None,
        }
    }

    pub fn rows(mut self, rows: &'a [TreeRow<'a>]) -> Self {
        self.rows = Some(rows);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: TreeStyle) -> Self {
        self.style = Some(style);
        self
    }
}

impl<'a> TreeSpecBuilder<'a> {
    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = Some(theme.tree_style());
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> TreeSpec<'a> {
        TreeSpec {
            rect: self.rect.unwrap_or_default(),
            rows: self.rows.unwrap(),
            font: self.font.expect("font must be specified or resolved from a theme"),
            style: self.style.expect("TreeStyle is required"),
        }
    }
}
