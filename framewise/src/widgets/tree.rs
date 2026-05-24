use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, text::TextSystem, theme::Theme, types::{Color, Rect}
};

pub struct TreeRow<'a> {
    pub indent:   u32,
    /// None = leaf, true = expanded, false = collapsed.
    pub caret:    Option<bool>,
    pub label:    &'a str,
    /// Optional right-aligned metadata string.
    pub meta:     Option<&'a str>,
    pub selected: bool,
}

pub struct TreeSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect: Rect,
    pub rows: &'a [TreeRow<'a>],
}

pub struct TreeResult {
    pub draw: DrawCommands,
}

impl WidgetResult for TreeResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn tree<'a, T: crate::text::TextSystem>(spec: TreeSpec<'a, T>) -> TreeResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let row_h = 20.0_f32;
    let indent_w = 14.0_f32;
    let caret_w = 12.0_f32;
    let pad_x = 10.0_f32;
    let total_h = spec.rows.len() as f32 * row_h + 8.0;
    let w = spec.rect.w.max(280.0);
    let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

    cmds.push(DrawCmd::FillRect { rect: outer, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: outer, color: t.ink, width: 1.0 });

    let mut y = spec.rect.y + 4.0;

    for row in spec.rows {
        let row_rect = Rect::new(outer.x, y, w, row_h);

        if row.selected {
            cmds.push(DrawCmd::FillRect { rect: row_rect, color: t.ink });
        }

        let text_color = if row.selected { t.paper } else { t.ink };
        let meta_color: Color = if row.selected {
            Color::linear_rgba(t.paper.r, t.paper.g, t.paper.b, 0.7)
        } else {
            t.muted
        };
        let caret_color = if row.selected { meta_color } else { t.muted };

        let indent_x = outer.x + pad_x + row.indent as f32 * indent_w;

        // Caret symbol.
        let caret_sym = match row.caret {
            Some(true)  => "v",
            Some(false) => ">",
            None        => " ",
        };
        let caret_layout = spec.ts.prepare(caret_sym, t.text_sm);
        let cty = y + (row_h - caret_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(indent_x, cty, caret_layout.size.x, caret_layout.size.y),
            color:  caret_color,
            handle: caret_layout.handle,
        });

        // Label.
        let label_layout = spec.ts.prepare(row.label, t.text_sm);
        let lty = y + (row_h - label_layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(indent_x + caret_w, lty, label_layout.size.x, label_layout.size.y),
            color:  text_color,
            handle: label_layout.handle,
        });

        // Meta (right-aligned).
        if let Some(meta) = row.meta {
            let meta_layout = spec.ts.prepare(meta, t.text_sm);
            let mx = outer.x + w - pad_x - meta_layout.size.x;
            let mty = y + (row_h - meta_layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(mx, mty, meta_layout.size.x, meta_layout.size.y),
                color:  meta_color,
                handle: meta_layout.handle,
            });
        }

        y += row_h;
    }

    TreeResult { draw: cmds }
}




pub struct TreeSpecBuilder<'a, T: crate::text::TextSystem> {
    pub rows: Option<&'a [TreeRow<'a>]>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> TreeSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            rows: None,
            rect: None,
            ts: None,
        }
    }

    pub fn rows(mut self, rows: &'a [TreeRow<'a>]) -> Self {
        self.rows = Some(rows);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for TreeSpecBuilder<'a, T> {
    type Spec = TreeSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        TreeSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            rows: self.rows.unwrap(),
        }
    }
}
