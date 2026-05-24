use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::{Color, Rect, Vec2}, widget::{WidgetSpec, WidgetSpecBuilder}
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
impl WidgetSpec for CheckboxSpec {
    type Builder = CheckboxSpecBuilder;
}

pub struct CheckboxSpecBuilder {
    spec: CheckboxSpec,
}
impl CheckboxSpecBuilder {
    pub fn new(state: CheckState) -> Self {
        Self {
            spec: CheckboxSpec {
                rect: Rect::ZERO,
                state,
                focused: false,
                disabled: false,
            }
        }
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
impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for CheckboxSpecBuilder {
    type Spec = CheckboxSpec;

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

pub struct CheckboxResult {
    pub draw:  DrawCommands,
}
impl WidgetResult for CheckboxResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn checkbox(spec: CheckboxSpec) -> CheckboxResult {
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

    CheckboxResult { draw: cmds }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_checkbox_visual_off() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Off,
            focused: false,
            disabled: false,
        };
        let res = checkbox(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 2, "Off state should just draw fill and border");
        assert!(matches!(&cmds[0], DrawCmd::FillRect { rect, .. } if *rect == Rect::new(10.0, 10.0, 14.0, 14.0)));
        assert!(matches!(&cmds[1], DrawCmd::StrokeRect { .. }));
    }

    #[test]
    fn test_checkbox_visual_on() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::On,
            focused: false,
            disabled: false,
        };
        let res = checkbox(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 4, "On state should draw fill, border, and two checkmark lines");
        assert!(matches!(&cmds[2], DrawCmd::StrokeLine { .. }));
        assert!(matches!(&cmds[3], DrawCmd::StrokeLine { .. }));
    }

    #[test]
    fn test_checkbox_visual_indeterminate() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Indeterminate,
            focused: false,
            disabled: false,
        };
        let res = checkbox(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 3, "Indeterminate state should draw fill, border, and a dash");
        assert!(matches!(&cmds[2], DrawCmd::FillRect { rect, .. } if *rect == Rect::new(12.0, 16.0, 10.0, 2.0)));
    }

    #[test]
    fn test_checkbox_visual_focused() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Off,
            focused: true,
            disabled: false,
        };
        let res = checkbox(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 3, "Focused state adds a focus ring");
        assert!(matches!(&cmds[0], DrawCmd::StrokeRect { width, .. } if *width == 2.0));
    }

    #[test]
    fn test_checkbox_visual_disabled() {
        let spec = CheckboxSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            state: CheckState::Off,
            focused: false,
            disabled: true,
        };
        let res = checkbox(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 2);
        if let DrawCmd::FillRect { color, .. } = cmds[0] {
            assert!(color.a < 1.0, "Disabled should be drawn with a tinted alpha");
        } else {
            panic!("Expected FillRect");
        }
    }
}
