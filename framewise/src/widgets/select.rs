use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    WidgetResult,
};

pub struct SelectSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Bounding rect for the closed box (height h_md = 28).
    pub rect: Rect,
    pub value: &'a str,
    pub font: FontId,
    pub options: &'a [&'a str],
    pub open: bool,
    pub focused: bool,
    /// Index of hovered option when open.
    pub hovered: Option<usize>,
    pub style: SelectStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectStyle {
    pub min_width: f32,
    pub height: f32,
    pub row_height: f32,
    pub popup_gap: f32,
    pub popup_pad_y: f32,
    pub pad_x: f32,
    pub chevron_right: f32,
    pub text_size: f32,
    pub chevron_size: f32,
    pub background: Color,
    pub border: Color,
    pub text: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub hover: Color,
    pub muted: Color,
    pub accent: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
}

pub struct SelectResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SelectResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn select<'a, T: crate::text::TextSystem>(spec: SelectSpec<'a, T>) -> SelectResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    let r = Rect::new(
        spec.rect.x,
        spec.rect.y,
        spec.rect.w.max(s.min_width),
        s.height,
    );

    // Focus / open ring.
    if spec.focused || spec.open {
        cmds.push(DrawCmd::StrokeRect {
            rect: r.inset(-s.focus_offset),
            color: s.accent,
            width: s.focus_width,
        });
    }

    cmds.push(DrawCmd::FillRect {
        rect: r,
        color: s.background,
    });
    cmds.push(DrawCmd::StrokeRect {
        rect: r,
        color: s.border,
        width: s.border_width,
    });

    // Selected value text.
    let val_layout = spec.ts.prepare(spec.value, s.text_size, spec.font);
    let vty = r.y + (s.height - val_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(r.x + s.pad_x, vty, val_layout.size.x, val_layout.size.y),
        color: s.text,
        handle: val_layout.handle,
    });

    // Chevron "v".
    let chev_color = if spec.open { s.accent } else { s.muted };
    let chev_layout = spec.ts.prepare("v", s.chevron_size, spec.font);
    let cty = r.y + (s.height - chev_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(
            r.x + r.w - s.chevron_right,
            cty,
            chev_layout.size.x,
            chev_layout.size.y,
        ),
        color: chev_color,
        handle: chev_layout.handle,
    });

    // Dropdown popup.
    if spec.open && !spec.options.is_empty() {
        let row_h = s.row_height;
        let popup_h = spec.options.len() as f32 * row_h + s.popup_pad_y * 2.0;
        let popup = Rect::new(r.x, r.y + s.height + s.popup_gap, r.w, popup_h);

        cmds.push(DrawCmd::FillRect {
            rect: popup,
            color: s.background,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: popup,
            color: s.border,
            width: s.border_width,
        });

        for (i, opt) in spec.options.iter().enumerate() {
            let is_selected = *opt == spec.value;
            let is_hovered = spec.hovered == Some(i);
            let row_y = popup.y + s.popup_pad_y + i as f32 * row_h;
            let row_rect = Rect::new(popup.x, row_y, popup.w, row_h);

            if is_selected {
                cmds.push(DrawCmd::FillRect {
                    rect: row_rect,
                    color: s.selected_bg,
                });
            } else if is_hovered {
                cmds.push(DrawCmd::FillRect {
                    rect: row_rect,
                    color: s.hover,
                });
            }

            let text_color = if is_selected { s.selected_text } else { s.text };
            let opt_layout = spec.ts.prepare(opt, s.text_size, spec.font);
            let oty = row_y + (row_h - opt_layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect: Rect::new(
                    popup.x + s.pad_x + 2.0,
                    oty,
                    opt_layout.size.x,
                    opt_layout.size.y,
                ),
                color: text_color,
                handle: opt_layout.handle,
            });
        }
    }

    SelectResult { draw: cmds }
}

pub struct SelectSpecBuilder<'a, T: crate::text::TextSystem> {
    pub value: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<SelectStyle>,
    pub options: Option<&'a [&'a str]>,
    pub open: Option<bool>,
    pub focused: Option<bool>,
    pub hovered: Option<Option<usize>>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> SelectSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            value: None,
            font: None,
            style: None,
            options: None,
            open: None,
            focused: None,
            hovered: None,
            rect: None,
            ts: None,
        }
    }

    pub fn value(mut self, value: &'a str) -> Self {
        self.value = Some(value);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: SelectStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn options(mut self, options: &'a [&'a str]) -> Self {
        self.options = Some(options);
        self
    }
    pub fn open(mut self, open: bool) -> Self {
        self.open = Some(open);
        self
    }
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = Some(focused);
        self
    }
    pub fn hovered(mut self, hovered: Option<usize>) -> Self {
        self.hovered = Some(hovered);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for SelectSpecBuilder<'a, T>
{
    type Spec = SelectSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.select_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        SelectSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            value: self.value.unwrap(),
            font: self.font.unwrap_or(FontId::SANS),
            style: self.style.expect("SelectStyle is required"),
            options: self.options.unwrap(),
            open: self.open.unwrap(),
            focused: self.focused.unwrap(),
            hovered: self.hovered.unwrap(),
        }
    }
}
