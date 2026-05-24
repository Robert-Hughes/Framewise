use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect, Vec2},
    WidgetResult,
};

#[derive(Debug, Clone)]
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

pub struct MenuSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Top-left origin; width is at least 200.
    pub rect: Rect,
    pub items: &'a [MenuItem<'a>],
    pub label_font: FontId,
    pub meta_font: FontId,
    pub style: MenuStyle,
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
    pub label_size: f32,
    pub meta_size: f32,
    pub background: Color,
    pub border: Color,
    pub separator: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub text: Color,
    pub muted: Color,
    pub shortcut_selected_alpha: f32,
    pub disabled_alpha: f32,
    pub border_width: f32,
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
    let mut cmds = DrawCommands::new();
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

    for item in spec.items {
        match item {
            MenuItem::Separator => {
                let sep_y = y + s.separator_y;
                cmds.push(DrawCmd::StrokeLine {
                    p0: Vec2::new(outer.x, sep_y),
                    p1: Vec2::new(outer.x + w, sep_y),
                    color: s.separator,
                    width: s.border_width,
                });
                y += sep_h;
            }
            MenuItem::Group(label) => {
                let layout = spec.ts.prepare(label, s.meta_size, spec.meta_font);
                let ty = y + s.group_text_y;
                cmds.push(DrawCmd::Text {
                    rect: Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                    color: s.muted,
                    handle: layout.handle,
                });
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
                    cmds.push(DrawCmd::FillRect {
                        rect: row_rect,
                        color: tint(s.selected_bg),
                    });
                }

                let text_color = if *selected {
                    tint(s.selected_text)
                } else {
                    tint(s.text)
                };
                let layout = spec.ts.prepare(label, s.label_size, spec.label_font);
                let ty = y + (row_h - layout.size.y) * 0.5;
                cmds.push(DrawCmd::Text {
                    rect: Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                    color: text_color,
                    handle: layout.handle,
                });

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
                    let sc_layout = spec.ts.prepare(sc, s.meta_size, spec.meta_font);
                    let sc_x = outer.x + w - pad_x - sc_layout.size.x;
                    let sc_ty = y + (row_h - sc_layout.size.y) * 0.5;
                    cmds.push(DrawCmd::Text {
                        rect: Rect::new(sc_x, sc_ty, sc_layout.size.x, sc_layout.size.y),
                        color: sc_color,
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
    pub label_font: Option<FontId>,
    pub meta_font: Option<FontId>,
    pub style: Option<MenuStyle>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> MenuSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            items: None,
            label_font: None,
            meta_font: None,
            style: None,
            rect: None,
            ts: None,
        }
    }

    pub fn items(mut self, items: &'a [MenuItem<'a>]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn label_font(mut self, font: FontId) -> Self {
        self.label_font = Some(font);
        self
    }
    pub fn meta_font(mut self, font: FontId) -> Self {
        self.meta_font = Some(font);
        self
    }
    pub fn style(mut self, style: MenuStyle) -> Self {
        self.style = Some(style);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for MenuSpecBuilder<'a, T>
{
    type Spec = MenuSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.menu_style());
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
            label_font: self.label_font.unwrap_or(FontId::SANS),
            meta_font: self.meta_font.unwrap_or(FontId::MONO),
            style: self.style.expect("MenuStyle is required"),
        }
    }
}
