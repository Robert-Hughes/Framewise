use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    WidgetResult,
};

pub struct ChipSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Top-left origin. Height is fixed at 22.
    pub rect: Rect,
    pub label: &'a str,
    pub font: FontId,
    pub active: bool,
    pub focused: bool,
    pub style: ChipStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChipStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_size: f32,
    pub background: Color,
    pub active_bg: Color,
    pub border: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
}

impl Default for ChipStyle {
    fn default() -> Self {
        Self {
            height: 22.0,
            pad_x: 8.0,
            text_size: 11.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            active_bg: Color::from_srgb_u8(21, 19, 15, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            active_text: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
        }
    }
}

pub struct ChipResult {
    pub draw: DrawCommands,
}

impl WidgetResult for ChipResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn chip<'a, T: crate::text::TextSystem>(spec: ChipSpec<'a, T>) -> ChipResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    let h = s.height;
    let pad_x = s.pad_x;

    let layout = spec.ts.prepare(spec.label, s.text_size, spec.font);
    let w = spec.rect.w.max(32.0);
    let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

    // Focus ring.
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect: r.inset(-s.focus_offset),
            color: s.focus,
            width: s.focus_width,
        });
    }

    let bg = if spec.active {
        s.active_bg
    } else {
        s.background
    };
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });
    cmds.push(DrawCmd::StrokeRect {
        rect: r,
        color: s.border,
        width: s.border_width,
    });

    let text_color = if spec.active { s.active_text } else { s.text };
    let ty = r.y + (h - layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(r.x + pad_x, ty, layout.size.x, layout.size.y),
        color: text_color,
        handle: layout.handle,
    });

    ChipResult { draw: cmds }
}

pub struct ChipSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<ChipStyle>,
    pub active: Option<bool>,
    pub focused: Option<bool>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> ChipSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            active: None,
            focused: None,
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
    pub fn style(mut self, style: ChipStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }
    pub fn focused(mut self, focused: bool) -> Self {
        self.focused = Some(focused);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for ChipSpecBuilder<'a, T>
{
    type Spec = ChipSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.chip_style());
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        ChipSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self.font.expect("font must be specified or resolved from a theme"),
            style: self.style.expect("ChipStyle is required"),
            active: self.active.unwrap(),
            focused: self.focused.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_chip_visual_normal() {
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            active: false,
            focused: false,
            style: Default::default(),
        };
        let style = spec.style;
        let res = chip(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_chip_visual_active() {
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            active: true,
            focused: false,
            style: Default::default(),
        };
        let style = spec.style;
        let res = chip(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 50.0, 22.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_chip_visual_focused() {
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId(0),
            active: false,
            focused: true,
            style: Default::default(),
        };
        let style = spec.style;
        let res = chip(spec);

        let r = Rect::new(0.0, 0.0, 50.0, 22.0);
        let expected_focus_rect = r.inset(-style.focus_offset);
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeRect {
                    rect: expected_focus_rect,
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::FillRect {
                    rect: r,
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: r,
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 3.0, 24.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }
}


