use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::{Rect, Vec2},
};

pub struct TabsSpec<'a> {
    /// Bounding rect; only x/y/w used — height is fixed at 36.
    pub rect:         Rect,
    pub items:        &'a [&'a str],
    pub active_index: usize,
    pub focused:      Option<usize>,
}

pub fn tabs<T: TextSystem>(spec: TabsSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let tab_h = 36.0_f32;
    let pad_x = 18.0_f32;
    let underbar_h = 3.0_f32;

    // Bottom border across the full width.
    let border_y = spec.rect.y + tab_h;
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(spec.rect.x, border_y),
        p1:    Vec2::new(spec.rect.x + spec.rect.w, border_y),
        color: t.ink,
        width: 1.0,
    });

    let mut x = spec.rect.x;

    for (i, label) in spec.items.iter().enumerate() {
        let is_active = i == spec.active_index;
        let is_focused = spec.focused == Some(i);

        let layout = ts.prepare(label, t.text_md);
        let tab_w = layout.size.x + pad_x * 2.0;
        let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

        // Focus ring.
        if is_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect:  tab_rect.inset(-2.0),
                color: t.rust,
                width: 2.0,
            });
        }

        let text_color = if is_active { t.ink } else { t.muted };
        let ty = spec.rect.y + (tab_h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
            color:  text_color,
            handle: layout.handle,
        });

        // Active underbar: 3px rust rect sitting on the bottom border.
        if is_active {
            cmds.push(DrawCmd::FillRect {
                rect:  Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                color: t.rust,
            });
        }

        x += tab_w;
    }

    cmds
}
