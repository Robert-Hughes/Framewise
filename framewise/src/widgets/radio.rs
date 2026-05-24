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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_radio_visual_unselected() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            focused: false,
            disabled: false,
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 2, "Unselected should draw background fill and outer ring");
        assert!(matches!(&cmds[0], DrawCmd::FillCircle { radius, .. } if *radius == 7.0));
        assert!(matches!(&cmds[1], DrawCmd::StrokeCircle { radius, .. } if *radius == 7.0));
    }

    #[test]
    fn test_radio_visual_selected() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: true,
            focused: false,
            disabled: false,
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 3, "Selected should draw fill, outer ring, and inner dot");
        assert!(matches!(&cmds[2], DrawCmd::FillCircle { radius, .. } if *radius == 3.0));
    }

    #[test]
    fn test_radio_visual_focused() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            focused: true,
            disabled: false,
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 3, "Focused adds an outer stroke circle");
        assert!(matches!(&cmds[0], DrawCmd::StrokeCircle { radius, width, .. } if *radius == 9.0 && *width == 2.0));
    }

    #[test]
    fn test_radio_visual_disabled() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            focused: false,
            disabled: true,
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 2);
        if let DrawCmd::FillCircle { color, .. } = cmds[0] {
            assert!(color.a < 1.0, "Disabled should be drawn with a tinted alpha");
        } else {
            panic!("Expected FillCircle");
        }
    }
}
