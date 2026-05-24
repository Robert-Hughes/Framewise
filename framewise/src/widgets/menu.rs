use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::{Color, Rect, Vec2}
};

#[derive(Debug, Clone)]
pub enum MenuItem<'a> {
    Item {
        label:    &'a str,
        shortcut: Option<&'a str>,
        selected: bool,
        disabled: bool,
    },
    Separator,
    Group(&'a str),
}

pub struct MenuSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Top-left origin; width is at least 200.
    pub rect:  Rect,
    pub items: &'a [MenuItem<'a>],
}

pub struct MenuResult {
    pub draw: DrawCommands,
}

impl WidgetResult for MenuResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn menu<'a, T: crate::text::TextSystem>(spec: MenuSpec<'a, T>) -> MenuResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let row_h = 26.0_f32;
    let sep_h = 9.0_f32;
    let group_h = 22.0_f32;
    let pad_x = 12.0_f32;
    let min_w = 200.0_f32;

    let total_h: f32 = spec.items.iter().map(|item| match item {
        MenuItem::Item { .. } => row_h,
        MenuItem::Separator   => sep_h,
        MenuItem::Group(_)    => group_h,
    }).sum::<f32>() + 8.0;

    let w = spec.rect.w.max(min_w);
    let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

    cmds.push(DrawCmd::FillRect { rect: outer, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: outer, color: t.ink, width: 1.0 });

    let mut y = spec.rect.y + 4.0;

    for item in spec.items {
        match item {
            MenuItem::Separator => {
                let sep_y = y + 4.0;
                cmds.push(DrawCmd::StrokeLine {
                    p0:    Vec2::new(outer.x, sep_y),
                    p1:    Vec2::new(outer.x + w, sep_y),
                    color: t.line,
                    width: 1.0,
                });
                y += sep_h;
            }
            MenuItem::Group(label) => {
                let layout = spec.ts.prepare(label, t.text_sm);
                let ty = y + 8.0;
                cmds.push(DrawCmd::Text {
                    rect:   Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                    color:  t.muted,
                    handle: layout.handle,
                });
                y += group_h;
            }
            MenuItem::Item { label, shortcut, selected, disabled } => {
                let alpha = if *disabled { 0.4_f32 } else { 1.0 };
                let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

                let row_rect = Rect::new(outer.x, y, w, row_h);

                if *selected {
                    cmds.push(DrawCmd::FillRect { rect: row_rect, color: tint(t.ink) });
                }

                let text_color = if *selected { tint(t.paper) } else { tint(t.ink) };
                let layout = spec.ts.prepare(label, t.text_md);
                let ty = y + (row_h - layout.size.y) * 0.5;
                cmds.push(DrawCmd::Text {
                    rect:   Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                    color:  text_color,
                    handle: layout.handle,
                });

                if let Some(sc) = shortcut {
                    let sc_color = if *selected {
                        Color::linear_rgba(t.paper.r, t.paper.g, t.paper.b, 0.6 * alpha)
                    } else {
                        tint(t.muted)
                    };
                    let sc_layout = spec.ts.prepare(sc, t.text_sm);
                    let sc_x = outer.x + w - pad_x - sc_layout.size.x;
                    let sc_ty = y + (row_h - sc_layout.size.y) * 0.5;
                    cmds.push(DrawCmd::Text {
                        rect:   Rect::new(sc_x, sc_ty, sc_layout.size.x, sc_layout.size.y),
                        color:  sc_color,
                        handle: sc_layout.handle,
                    });
                }

                y += row_h;
            }
        }
    }

    MenuResult { draw: cmds }
}




pub struct MenuSpecBuilder<'a, T: crate::text::TextSystem> {
    pub items: Option<&'a [MenuItem<'a>]>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> MenuSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            items: None,
            rect: None,
            ts: None,
        }
    }

    pub fn items(mut self, items: &'a [MenuItem<'a>]) -> Self {
        self.items = Some(items);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for MenuSpecBuilder<'a, T> {
    type Spec = MenuSpec<'a, T>;

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
        MenuSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
        }
    }
}
