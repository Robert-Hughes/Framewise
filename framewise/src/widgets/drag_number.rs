use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    WidgetResult,
};

pub struct DragNumberSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Full bounding rect (height typically h_md = 28).
    pub rect: Rect,
    pub label: &'a str,
    pub font: FontId,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub active: bool,
    pub style: DragNumberStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DragNumberStyle {
    pub text_size: f32,
    pub label_pad_x: f32,
    pub background: Color,
    pub border: Color,
    pub focus: Color,
    pub label_bg: Color,
    pub active_label_bg: Color,
    pub label_text: Color,
    pub value_text: Color,
    pub value_fill: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
}

impl Default for DragNumberStyle {
    fn default() -> Self {
        Self {
            text_size: 13.0,
            label_pad_x: 10.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            label_bg: Color::from_srgb_u8(21, 19, 15, 255),
            active_label_bg: Color::from_srgb_u8(194, 90, 44, 255),
            label_text: Color::from_srgb_u8(244, 241, 234, 255),
            value_text: Color::from_srgb_u8(21, 19, 15, 255),
            value_fill: Color::from_srgb_f32(194.0 / 255.0, 90.0 / 255.0, 44.0 / 255.0, 0.14),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 1.0,
        }
    }
}

pub struct DragNumberResult {
    pub draw: DrawCommands,
}

impl WidgetResult for DragNumberResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn drag_number<'a, T: crate::text::TextSystem>(
    spec: DragNumberSpec<'a, T>,
) -> DragNumberResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    // Focus / active ring.
    if spec.active {
        cmds.push(DrawCmd::StrokeRect {
            rect: spec.rect.inset(-s.focus_offset),
            color: s.focus,
            width: s.focus_width,
        });
    }

    cmds.push(DrawCmd::FillRect {
        rect: spec.rect,
        color: s.background,
    });
    cmds.push(DrawCmd::StrokeRect {
        rect: spec.rect,
        color: s.border,
        width: s.border_width,
    });

    // Label section (ink/rust bg, paper text).
    let label_layout = spec.ts.prepare(spec.label, s.text_size, spec.font);
    let label_w = label_layout.size.x + s.label_pad_x * 2.0;
    let label_rect = Rect::new(spec.rect.x, spec.rect.y, label_w, spec.rect.h);
    let label_bg = if spec.active {
        s.active_label_bg
    } else {
        s.label_bg
    };
    cmds.push(DrawCmd::FillRect {
        rect: label_rect,
        color: label_bg,
    });

    let lty = spec.rect.y + (spec.rect.h - label_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(
            spec.rect.x + s.label_pad_x,
            lty,
            label_layout.size.x,
            label_layout.size.y,
        ),
        color: s.label_text,
        handle: label_layout.handle,
    });

    // Value area: rust_soft fill proportional to value fraction.
    let value_x = spec.rect.x + label_w;
    let value_w = spec.rect.w - label_w;
    let frac = ((spec.value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
    if frac > 0.0 {
        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(value_x, spec.rect.y, value_w * frac, spec.rect.h),
            color: s.value_fill,
        });
    }

    let value_text = format!("{:.2}", spec.value);
    let val_layout = spec.ts.prepare(&value_text, s.text_size, spec.font);
    let vtx = value_x + (value_w - val_layout.size.x) * 0.5;
    let vty = spec.rect.y + (spec.rect.h - val_layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(vtx, vty, val_layout.size.x, val_layout.size.y),
        color: s.value_text,
        handle: val_layout.handle,
    });

    DragNumberResult { draw: cmds }
}

pub struct DragNumberSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<DragNumberStyle>,
    pub value: Option<f32>,
    pub min: Option<f32>,
    pub max: Option<f32>,
    pub active: Option<bool>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> DragNumberSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            value: None,
            min: None,
            max: None,
            active: None,
            rect: None,
            ts: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: DragNumberStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }
    pub fn min(mut self, min: f32) -> Self {
        self.min = Some(min);
        self
    }
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for DragNumberSpecBuilder<'a, T>
{
    type Spec = DragNumberSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.drag_number_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        DragNumberSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self.font.unwrap_or(FontId::SANS),
            style: self.style.expect("DragNumberStyle is required"),
            value: self.value.unwrap(),
            min: self.min.unwrap(),
            max: self.max.unwrap(),
            active: self.active.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_drag_number_visual_normal() {
        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId::SANS,
            value: 50.0,
            min: 0.0,
            max: 100.0,
            active: false,
            style: Default::default(),
        };

        let style = spec.style;
        let res = drag_number(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                },
                DrawCmd::Text {
                    rect: Rect::new(54.0, 16.0, 40.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_drag_number_visual_active() {
        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId::SANS,
            value: 50.0,
            min: 0.0,
            max: 100.0,
            active: true,
            style: Default::default(),
        };

        let style = spec.style;
        let res = drag_number(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: Rect::new(9.0, 9.0, 102.0, 30.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.active_label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(38.0, 10.0, 36.0, 28.0),
                    color: style.value_fill,
                },
                DrawCmd::Text {
                    rect: Rect::new(54.0, 16.0, 40.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_drag_number_visual_min_value() {
        let mut text_sys = DummyTextSys;
        let spec = DragNumberSpec {
            ts: &mut text_sys,
            rect: Rect::new(10.0, 10.0, 100.0, 28.0),
            label: "X",
            font: FontId::SANS,
            value: 0.0,
            min: 0.0,
            max: 100.0,
            active: false,
            style: Default::default(),
        };

        let style = spec.style;
        let res = drag_number(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 28.0, 28.0),
                    color: style.label_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(20.0, 16.0, 8.0, 16.0),
                    color: style.label_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(58.0, 16.0, 32.0, 16.0),
                    color: style.value_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }
}


