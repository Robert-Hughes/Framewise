use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::Rect,
};

pub struct SelectSpec<'a> {
    /// Bounding rect for the closed box (height h_md = 28).
    pub rect:    Rect,
    pub value:   &'a str,
    pub options: &'a [&'a str],
    pub open:    bool,
    pub focused: bool,
    /// Index of hovered option when open.
    pub hovered: Option<usize>,
}

pub fn select<T: TextSystem>(spec: SelectSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let r = Rect::new(spec.rect.x, spec.rect.y, spec.rect.w.max(180.0), t.h_md);

    // Focus / open ring.
    if spec.focused || spec.open {
        cmds.push(DrawCmd::StrokeRect {
            rect:  r.inset(-1.0),
            color: t.rust,
            width: 2.0,
        });
    }

    cmds.push(DrawCmd::FillRect { rect: r, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: r, color: t.ink, width: 1.0 });

    // Selected value text.
    let val_layout = ts.prepare(spec.value, t.text_md);
    let vty = r.y + (t.h_md - val_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(r.x + 10.0, vty, val_layout.size.x, val_layout.size.y),
        color:  t.ink,
        handle: val_layout.handle,
    });

    // Chevron "v".
    let chev_color = if spec.open { t.rust } else { t.muted };
    let chev_layout = ts.prepare("v", t.text_sm);
    let cty = r.y + (t.h_md - chev_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(r.x + r.w - 18.0, cty, chev_layout.size.x, chev_layout.size.y),
        color:  chev_color,
        handle: chev_layout.handle,
    });

    // Dropdown popup.
    if spec.open && !spec.options.is_empty() {
        let row_h = 26.0_f32;
        let popup_h = spec.options.len() as f32 * row_h + 8.0;
        let popup = Rect::new(r.x, r.y + t.h_md + 2.0, r.w, popup_h);

        cmds.push(DrawCmd::FillRect { rect: popup, color: t.paper_elev });
        cmds.push(DrawCmd::StrokeRect { rect: popup, color: t.ink, width: 1.0 });

        for (i, opt) in spec.options.iter().enumerate() {
            let is_selected = *opt == spec.value;
            let is_hovered  = spec.hovered == Some(i);
            let row_y = popup.y + 4.0 + i as f32 * row_h;
            let row_rect = Rect::new(popup.x, row_y, popup.w, row_h);

            if is_selected {
                cmds.push(DrawCmd::FillRect { rect: row_rect, color: t.ink });
            } else if is_hovered {
                cmds.push(DrawCmd::FillRect { rect: row_rect, color: t.hover });
            }

            let text_color = if is_selected { t.paper } else { t.ink };
            let opt_layout = ts.prepare(opt, t.text_md);
            let oty = row_y + (row_h - opt_layout.size.y) * 0.5;
            cmds.push(DrawCmd::Text {
                rect:   Rect::new(popup.x + 12.0, oty, opt_layout.size.x, opt_layout.size.y),
                color:  text_color,
                handle: opt_layout.handle,
            });
        }
    }

    cmds
}
