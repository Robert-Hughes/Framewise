use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{WidgetSpec, WidgetSpecBuilder},
    WidgetResult,
};

pub struct SwitchSpec {
    /// Top-left of the 30×16 bounding area.
    pub rect: Rect,
    pub on: bool,
    pub focused: bool,
    pub disabled: bool,
    pub style: SwitchStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SwitchStyle {
    pub size: (f32, f32),
    pub thumb_size: f32,
    pub off_fill: Color,
    pub on_fill: Color,
    pub border: Color,
    pub off_thumb: Color,
    pub on_thumb: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
    pub disabled_alpha: f32,
}

impl Default for SwitchStyle {
    fn default() -> Self {
        Self {
            size: (30.0, 16.0),
            thumb_size: 10.0,
            off_fill: Color::from_srgb_u8(251, 249, 244, 255),
            on_fill: Color::from_srgb_u8(21, 19, 15, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            off_thumb: Color::from_srgb_u8(21, 19, 15, 255),
            on_thumb: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.5,
            focus_width: 2.0,
            focus_offset: 2.0,
            disabled_alpha: 0.35,
        }
    }
}

impl WidgetSpec for SwitchSpec {
    type Builder = SwitchSpecBuilder;
}

pub struct SwitchSpecBuilder {
    spec: SwitchSpec,
}

impl SwitchSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: SwitchSpec {
                rect: Rect::ZERO,
                on: false,
                focused: false,
                disabled: false,
                style: SwitchStyle {
                    size: (30.0, 16.0),
                    thumb_size: 10.0,
                    off_fill: Color::WHITE,
                    on_fill: Color::BLACK,
                    border: Color::BLACK,
                    off_thumb: Color::BLACK,
                    on_thumb: Color::WHITE,
                    focus: Color::BLACK,
                    border_width: 1.5,
                    focus_width: 2.0,
                    focus_offset: 2.0,
                    disabled_alpha: 0.35,
                },
            },
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.spec.on = on;
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

    pub fn style(mut self, style: SwitchStyle) -> Self {
        self.spec.style = style;
        self
    }
}

impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for SwitchSpecBuilder {
    type Spec = SwitchSpec;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.spec.style = theme.switch_style();
        self
    }

    fn build(self) -> Self::Spec {
        self.spec
    }
}

pub struct SwitchResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SwitchResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn switch(spec: SwitchSpec) -> SwitchResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;
    let alpha = if spec.disabled { s.disabled_alpha } else { 1.0 };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let r = Rect::new(spec.rect.x, spec.rect.y, s.size.0, s.size.1);

    // Focus ring.
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect: r.inset(-s.focus_offset),
            color: tint(s.focus),
            width: s.focus_width,
        });
    }

    // Track fill.
    let track_fill = if spec.on { s.on_fill } else { s.off_fill };
    cmds.push(DrawCmd::FillRect {
        rect: r,
        color: tint(track_fill),
    });

    // Track border.
    cmds.push(DrawCmd::StrokeRect {
        rect: r,
        color: tint(s.border),
        width: s.border_width,
    });

    // Thumb dot (10×10, vertically centered, left/right positioned).
    let dot_y = r.y + (r.h - s.thumb_size) * 0.5;
    let dot_x = if spec.on {
        r.x + r.w - s.thumb_size - s.border_width
    } else {
        r.x + s.border_width
    };
    let dot_color = if spec.on { s.on_thumb } else { s.off_thumb };
    cmds.push(DrawCmd::FillRect {
        rect: Rect::new(dot_x, dot_y, s.thumb_size, s.thumb_size),
        color: tint(dot_color),
    });

    SwitchResult { draw: cmds }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_visual_off() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            focused: false,
            disabled: false,
            style: Default::default(),
        };
        let s = spec.style;
        let res = switch(spec);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_on() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: true,
            focused: false,
            disabled: false,
            style: Default::default(),
        };
        let s = spec.style;
        let res = switch(spec);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: s.on_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(28.5, 13.0, 10.0, 10.0),
                    color: s.on_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_focused() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            focused: true,
            disabled: false,
            style: Default::default(),
        };
        let s = spec.style;
        let res = switch(spec);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: r.inset(-s.focus_offset),
                    color: s.focus,
                    width: s.focus_width,
                },
                DrawCmd::FillRect {
                    rect: r,
                    color: s.off_fill,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: s.border,
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: s.off_thumb,
                },
            ])
        );
    }

    #[test]
    fn test_switch_visual_disabled() {
        let spec = SwitchSpec {
            rect: Rect::new(10.0, 10.0, 30.0, 16.0),
            on: false,
            focused: false,
            disabled: true,
            style: Default::default(),
        };
        let s = spec.style;
        let res = switch(spec);
        let alpha = s.disabled_alpha;
        let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
        let r = Rect::new(10.0, 10.0, 30.0, 16.0);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: r,
                    color: tint(s.off_fill),
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: tint(s.border),
                    width: s.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(11.5, 13.0, 10.0, 10.0),
                    color: tint(s.off_thumb),
                },
            ])
        );
    }
}
