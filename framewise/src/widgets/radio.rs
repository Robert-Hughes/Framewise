use crate::{
    draw::{DrawCmd, DrawCommands},
    theme::Theme,
    types::{Color, Rect, Vec2},
};

pub struct RadioSpec {
    /// Top-left of the 14×14 bounding area.
    pub rect:     Rect,
    pub selected: bool,
    pub focused:  bool,
    pub disabled: bool,
}

pub fn radio(spec: RadioSpec) -> DrawCommands {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();
    let alpha = if spec.disabled { 0.35_f32 } else { 1.0 };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let cx = spec.rect.x + 7.0;
    let cy = spec.rect.y + 7.0;
    let center = Vec2::new(cx, cy);

    // Focus ring (outset 2px).
    if spec.focused {
        cmds.push(DrawCmd::StrokeCircle {
            center,
            radius: 7.0 + 2.0,
            color:  tint(t.rust),
            width:  2.0,
        });
    }

    // Background fill.
    cmds.push(DrawCmd::FillCircle {
        center,
        radius: 7.0,
        color:  tint(t.paper_elev),
    });

    // Outer ring.
    cmds.push(DrawCmd::StrokeCircle {
        center,
        radius: 7.0,
        color:  tint(t.ink),
        width:  1.5,
    });

    // Inner dot when selected.
    if spec.selected {
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: 3.0,
            color:  tint(t.ink),
        });
    }

    cmds
}
