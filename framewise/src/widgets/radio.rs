use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect, Vec2},
    widget::{WidgetSpec, WidgetSpecBuilder},
    WidgetResult,
};

pub struct RadioSpec {
    /// Top-left of the 14×14 bounding area.
    pub rect: Rect,
    pub selected: bool,
    pub focused: bool,
    pub disabled: bool,
    pub style: RadioStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RadioStyle {
    pub radius: f32,
    pub dot_radius: f32,
    pub background: Color,
    pub border: Color,
    pub dot: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl Default for RadioStyle {
    fn default() -> Self {
        Self {
            radius: 7.0,
            dot_radius: 3.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            dot: Color::from_srgb_u8(21, 19, 15, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.5,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
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
                style: RadioStyle {
                    radius: 7.0,
                    dot_radius: 3.0,
                    background: Color::WHITE,
                    border: Color::BLACK,
                    dot: Color::BLACK,
                    focus: Color::BLACK,
                    border_width: 1.5,
                    focus_width: 2.0,
                    focus_offset: 2.0,
                    disabled_alpha: 0.35,
                },
            },
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

    pub fn style(mut self, style: RadioStyle) -> Self {
        self.spec.style = style;
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

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.spec.style = theme.radio_style();
        self
    }

    fn build(self) -> Self::Spec {
        self.spec
    }
}

pub struct RadioResult {
    pub draw: DrawCommands,
}
impl WidgetResult for RadioResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn radio(spec: RadioSpec) -> RadioResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;
    let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let cx = spec.rect.x + s.radius;
    let cy = spec.rect.y + s.radius;
    let center = Vec2::new(cx, cy);

    // Focus ring (outset 2px).
    if spec.focused {
        cmds.push(DrawCmd::StrokeCircle {
            center,
            radius: s.radius + s.focus_offset,
            color: tint(s.focus),
            width: s.focus_width,
        });
    }

    // Background fill.
    cmds.push(DrawCmd::FillCircle {
        center,
        radius: s.radius,
        color: tint(s.background),
    });

    // Outer ring.
    cmds.push(DrawCmd::StrokeCircle {
        center,
        radius: s.radius,
        color: tint(s.border),
        width: s.border_width,
    });

    // Inner dot when selected.
    if spec.selected {
        cmds.push(DrawCmd::FillCircle {
            center,
            radius: s.dot_radius,
            color: tint(s.dot),
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
            style: Default::default(),
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(
            cmds.len(),
            2,
            "Unselected should draw background fill and outer ring"
        );
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
            style: Default::default(),
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(
            cmds.len(),
            3,
            "Selected should draw fill, outer ring, and inner dot"
        );
        assert!(matches!(&cmds[2], DrawCmd::FillCircle { radius, .. } if *radius == 3.0));
    }

    #[test]
    fn test_radio_visual_focused() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            focused: true,
            disabled: false,
            style: Default::default(),
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 3, "Focused adds an outer stroke circle");
        assert!(
            matches!(&cmds[0], DrawCmd::StrokeCircle { radius, width, .. } if *radius == 9.0 && *width == 2.0)
        );
    }

    #[test]
    fn test_radio_visual_disabled() {
        let spec = RadioSpec {
            rect: Rect::new(10.0, 10.0, 14.0, 14.0),
            selected: false,
            focused: false,
            disabled: true,
            style: Default::default(),
        };
        let res = radio(spec);
        let cmds = res.draw.0;
        assert_eq!(cmds.len(), 2);
        if let DrawCmd::FillCircle { color, .. } = cmds[0] {
            assert!(
                color.a < 1.0,
                "Disabled should be drawn with a tinted alpha"
            );
        } else {
            panic!("Expected FillCircle");
        }
    }
}
