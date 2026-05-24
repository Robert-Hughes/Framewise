use crate::{
    draw::{DrawCmd, DrawCommands},
    theme::Theme,
    types::{Color, Rect, Vec2},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CheckState {
    Off,
    On,
    Indeterminate,
}

pub struct CheckboxSpec {
    /// Top-left of the 14×14 box.
    pub rect:     Rect,
    pub state:    CheckState,
    pub focused:  bool,
    pub disabled: bool,
}

pub fn checkbox(spec: CheckboxSpec) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();
    let alpha = if spec.disabled { 0.35_f32 } else { 1.0 };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let r = Rect::new(spec.rect.x, spec.rect.y, 14.0, 14.0);

    // Focus ring (outset 2px).
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect:  r.inset(-2.0),
            color: tint(t.rust),
            width: 2.0,
        });
    }

    // Box fill.
    let fill = match spec.state {
        CheckState::Off => t.paper_elev,
        _ => t.ink,
    };
    cmds.push(DrawCmd::FillRect { rect: r, color: tint(fill) });

    // Box border.
    cmds.push(DrawCmd::StrokeRect {
        rect:  r,
        color: tint(t.ink),
        width: 1.5,
    });

    // Inner mark.
    match spec.state {
        CheckState::On => {
            // Checkmark: two lines forming a tick (√).
            let p0 = Vec2::new(r.x + 2.5, r.y + 7.0);
            let p1 = Vec2::new(r.x + 5.5, r.y + 10.5);
            let p2 = Vec2::new(r.x + 11.5, r.y + 4.0);
            let paper = tint(t.paper);
            cmds.push(DrawCmd::StrokeLine { p0, p1, color: paper, width: 1.5 });
            cmds.push(DrawCmd::StrokeLine { p0: p1, p1: p2, color: paper, width: 1.5 });
        }
        CheckState::Indeterminate => {
            // Horizontal dash.
            cmds.push(DrawCmd::FillRect {
                rect:  Rect::new(r.x + 2.0, r.y + 6.0, 10.0, 2.0),
                color: tint(t.paper),
            });
        }
        CheckState::Off => {}
    }

    cmds
}
