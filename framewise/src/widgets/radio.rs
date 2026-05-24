use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::{Color, Rect, Vec2}, widget::{WidgetSpec, WidgetSpecBuilder}
};

pub struct RadioSpec {
    /// Top-left of the 14×14 bounding area.
    pub rect:     Rect,
    pub selected: bool,
    pub focused:  bool,
    pub disabled: bool,
}

impl WidgetSpec for RadioSpec {
    type Builder = RadioSpecBuilder;
}

pub struct RadioSpecBuilder {
    spec: RadioSpec,
}
impl RadioSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: RadioSpec {
                rect: Rect::ZERO,
                selected: false,
                focused: false,
                disabled: false,
            }
        }
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.spec.selected = selected;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.spec.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.spec.disabled = disabled;
        self
    }
}
impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for RadioSpecBuilder {
    type Spec = RadioSpec;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn build(self) -> Self::Spec {
        self.spec
    }
}

pub struct RadioResult {
    pub draw:  DrawCommands,
}
impl WidgetResult for RadioResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}


pub fn radio(spec: RadioSpec) -> RadioResult {
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

    RadioResult { draw: cmds }
}
