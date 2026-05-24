use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::Rect,
};

pub struct ChipSpec<'a> {
    /// Top-left origin. Height is fixed at 22.
    pub rect:    Rect,
    pub label:   &'a str,
    pub active:  bool,
    pub focused: bool,
}

pub fn chip<T: TextSystem>(spec: ChipSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let h = 22.0_f32;
    let pad_x = 8.0_f32;

    let layout = ts.prepare(spec.label, t.text_sm);
    let w = (layout.size.x + pad_x * 2.0).max(32.0);
    let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

    // Focus ring.
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect:  r.inset(-2.0),
            color: t.rust,
            width: 2.0,
        });
    }

    let bg = if spec.active { t.ink } else { t.paper_elev };
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });
    cmds.push(DrawCmd::StrokeRect { rect: r, color: t.ink, width: 1.0 });

    let text_color = if spec.active { t.paper } else { t.ink };
    let ty = r.y + (h - layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(r.x + pad_x, ty, layout.size.x, layout.size.y),
        color:  text_color,
        handle: layout.handle,
    });

    cmds
}
