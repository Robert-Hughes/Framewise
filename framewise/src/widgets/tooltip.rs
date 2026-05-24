use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::{Color, Rect, Vec2},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipVariant {
    Dark,
    Rust,
}

pub struct TooltipSpec<'a> {
    pub rect:    Rect,
    pub text:    &'a str,
    pub variant: TooltipVariant,
}

pub fn tooltip<T: TextSystem>(spec: TooltipSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let pad_x = 8.0_f32;
    let pad_y_top = 5.0_f32;
    let pad_y_bot = 6.0_f32;
    let arrow_h = 4.0_f32;
    let arrow_w = 8.0_f32;

    let (bg, text_color): (Color, Color) = match spec.variant {
        TooltipVariant::Dark => (t.ink, t.paper),
        TooltipVariant::Rust => (t.rust, Color::WHITE),
    };

    let layout = ts.prepare(spec.text, t.text_sm);
    let box_w = (layout.size.x + pad_x * 2.0).min(240.0);
    let box_h = layout.size.y + pad_y_top + pad_y_bot;

    let r = Rect::new(spec.rect.x, spec.rect.y, box_w, box_h);
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });

    cmds.push(DrawCmd::Text {
        rect:   Rect::new(r.x + pad_x, r.y + pad_y_top, layout.size.x, layout.size.y),
        color:  text_color,
        handle: layout.handle,
    });

    // Arrow triangle below (two lines converging to a point).
    let arrow_x = r.x + 14.0;
    let arrow_y = r.y + box_h;
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(arrow_x, arrow_y),
        p1:    Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: 1.5,
    });
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(arrow_x + arrow_w, arrow_y),
        p1:    Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: 1.5,
    });

    cmds
}
