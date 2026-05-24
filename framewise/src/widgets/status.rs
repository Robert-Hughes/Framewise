use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::{Color, Rect},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusVariant {
    Neutral,
    Ok,
    Warn,
    Err,
    Live,
}

pub struct StatusSpec<'a> {
    pub rect:    Rect,
    pub label:   &'a str,
    pub variant: StatusVariant,
}

pub fn status<T: TextSystem>(spec: StatusSpec<'_>, ts: &mut T) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let dot_size = 6.0_f32;
    let gap = 8.0_f32;

    let dot_color = match spec.variant {
        StatusVariant::Neutral => t.muted,
        StatusVariant::Ok      => Color::rgb(0.302, 0.541, 0.227),
        StatusVariant::Warn    => t.rust,
        StatusVariant::Err     => Color::rgb(0.702, 0.145, 0.122),
        StatusVariant::Live    => t.rust,
    };

    cmds.push(DrawCmd::FillRect {
        rect:  Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
        color: dot_color,
    });

    let label_upper = spec.label.to_uppercase();
    let layout = ts.prepare(&label_upper, t.text_sm);
    let ty = spec.rect.y + (dot_size - layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(spec.rect.x + dot_size + gap, ty, layout.size.x, layout.size.y),
        color:  t.muted,
        handle: layout.handle,
    });

    cmds
}
