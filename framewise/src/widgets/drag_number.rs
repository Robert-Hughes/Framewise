use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::Rect,
};

pub struct DragNumberSpec<'a> {
    /// Full bounding rect (height typically h_md = 28).
    pub rect:   Rect,
    pub label:  &'a str,
    pub value:  f32,
    pub min:    f32,
    pub max:    f32,
    pub active: bool,
}

pub fn drag_number<T: TextSystem>(spec: DragNumberSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    // Focus / active ring.
    if spec.active {
        cmds.push(DrawCmd::StrokeRect {
            rect:  spec.rect.inset(-1.0),
            color: t.rust,
            width: 2.0,
        });
    }

    cmds.push(DrawCmd::FillRect { rect: spec.rect, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: spec.rect, color: t.ink, width: 1.0 });

    // Label section (ink/rust bg, paper text).
    let label_layout = ts.prepare(spec.label, t.text_md);
    let label_w = label_layout.size.x + 20.0;
    let label_rect = Rect::new(spec.rect.x, spec.rect.y, label_w, spec.rect.h);
    let label_bg = if spec.active { t.rust } else { t.ink };
    cmds.push(DrawCmd::FillRect { rect: label_rect, color: label_bg });

    let lty = spec.rect.y + (spec.rect.h - label_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(spec.rect.x + 10.0, lty, label_layout.size.x, label_layout.size.y),
        color:  t.paper,
        handle: label_layout.handle,
    });

    // Value area: rust_soft fill proportional to value fraction.
    let value_x = spec.rect.x + label_w;
    let value_w = spec.rect.w - label_w;
    let frac = ((spec.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
    if frac > 0.0 {
        cmds.push(DrawCmd::FillRect {
            rect:  Rect::new(value_x, spec.rect.y, value_w * frac, spec.rect.h),
            color: t.rust_soft,
        });
    }

    let value_text = format!("{:.2}", spec.value);
    let val_layout = ts.prepare(&value_text, t.text_md);
    let vtx = value_x + (value_w - val_layout.size.x) * 0.5;
    let vty = spec.rect.y + (spec.rect.h - val_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(vtx, vty, val_layout.size.x, val_layout.size.y),
        color:  t.ink,
        handle: val_layout.handle,
    });

    cmds
}
