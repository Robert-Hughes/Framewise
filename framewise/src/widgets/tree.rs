use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreeSpec<'a> {
        pub rect: Rect,
        pub rows: &'a [super::TreeRow<'a>],
        pub font: FontId,
        pub style: super::TreeStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TreeResult {
        pub draw: DrawCommands,
    }

    /// Low-level tree widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tree<'a, T: TextSystem>(spec: TreeSpec<'a>, text_system: &mut T) -> TreeResult {
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

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TreeResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeSpecBuilder<'a> {
    pub rows: Option<&'a [TreeRow<'a>]>,
    pub font: Option<FontId>,
    pub style: Option<TreeStyle>,
    pub rect: Option<Rect>,
}

impl<'a> TreeSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
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

    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(theme.tree_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::TreeSpec<'a> {
        raw::TreeSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            rows: self.rows.expect("rows not set — call .rows()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tree widget function using WidgetContext.
///
/// This function accepts a TreeSpec and calls the low-level raw::tree function.
pub fn tree<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TreeSpecBuilder<'a>,
    layout_params: S::Params,
) -> TreeResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::tree(spec, ctx.text_system);
    ctx.append_cmds(result.draw);
    TreeResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = TreeSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.tree_style()));
        assert_eq!(builder.font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.tree_style();
        custom_style.text_size = 99.0;
        let builder = TreeSpecBuilder::new().style(custom_style).font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        super::tree(
            &mut ctx,
            TreeSpecBuilder::new().rows(&[]).rect(custom_rect),
            layout_rect,
        );
        // First draw command is FillRect for the outer rect at (custom_rect.x, custom_rect.y)
        match &cmds[0] {
            crate::draw::DrawCmd::FillRect { rect, .. } => {
                assert_eq!(rect.x, custom_rect.x);
                assert_eq!(rect.y, custom_rect.y);
            }
            other => panic!("Expected FillRect, got {:?}", other),
        }
    }
}
