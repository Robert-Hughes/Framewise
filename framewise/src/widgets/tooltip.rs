use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect, Vec2},
    WidgetResult,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipVariant {
    Dark,
    Rust,
}

pub struct TooltipSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect: Rect,
    pub text: &'a str,
    pub font: FontId,
    pub variant: TooltipVariant,
    pub style: TooltipStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipStyle {
    pub text_size: f32,
    pub pad_x: f32,
    pub pad_y_top: f32,
    pub pad_y_bot: f32,
    pub arrow_h: f32,
    pub arrow_w: f32,
    pub arrow_x: f32,
    pub max_width: f32,
    pub dark_bg: Color,
    pub dark_text: Color,
    pub rust_bg: Color,
    pub rust_text: Color,
    pub arrow_width: f32,
}

impl Default for TooltipStyle {
    fn default() -> Self {
        Self {
            text_size: 11.0,
            pad_x: 8.0,
            pad_y_top: 5.0,
            pad_y_bot: 6.0,
            arrow_h: 4.0,
            arrow_w: 8.0,
            arrow_x: 14.0,
            max_width: 240.0,
            dark_bg: Color::from_srgb_u8(21, 19, 15, 255),
            dark_text: Color::from_srgb_u8(244, 241, 234, 255),
            rust_bg: Color::from_srgb_u8(194, 90, 44, 255),
            rust_text: Color::WHITE,
            arrow_width: 1.5,
        }
    }
}

pub struct TooltipResult {
    pub draw: DrawCommands,
}

impl WidgetResult for TooltipResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn tooltip<'a, T: crate::text::TextSystem>(spec: TooltipSpec<'a, T>) -> TooltipResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    let pad_x = s.pad_x;
    let pad_y_top = s.pad_y_top;
    let pad_y_bot = s.pad_y_bot;
    let arrow_h = s.arrow_h;
    let arrow_w = s.arrow_w;

    let (bg, text_color): (Color, Color) = match spec.variant {
        TooltipVariant::Dark => (s.dark_bg, s.dark_text),
        TooltipVariant::Rust => (s.rust_bg, s.rust_text),
    };

    let layout = spec.ts.prepare(spec.text, s.text_size, spec.font);
    let box_w = (layout.size.x + pad_x * 2.0).min(s.max_width);
    let box_h = layout.size.y + pad_y_top + pad_y_bot;

    let r = Rect::new(spec.rect.x, spec.rect.y, box_w, box_h);
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });

    cmds.push(DrawCmd::Text {
        rect: Rect::new(r.x + pad_x, r.y + pad_y_top, layout.size.x, layout.size.y),
        color: text_color,
        handle: layout.handle,
    });

    // Arrow triangle below (two lines converging to a point).
    let arrow_x = r.x + s.arrow_x;
    let arrow_y = r.y + box_h;
    cmds.push(DrawCmd::StrokeLine {
        p0: Vec2::new(arrow_x, arrow_y),
        p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: s.arrow_width,
    });
    cmds.push(DrawCmd::StrokeLine {
        p0: Vec2::new(arrow_x + arrow_w, arrow_y),
        p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: s.arrow_width,
    });

    TooltipResult { draw: cmds }
}

pub struct TooltipSpecBuilder<'a, T: crate::text::TextSystem> {
    pub text: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<TooltipStyle>,
    pub variant: Option<TooltipVariant>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> TooltipSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            text: None,
            font: None,
            style: None,
            variant: None,
            rect: None,
            ts: None,
        }
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: TooltipStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: TooltipVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for TooltipSpecBuilder<'a, T>
{
    type Spec = TooltipSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.tooltip_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        TooltipSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            text: self.text.unwrap(),
            font: self.font.unwrap_or(FontId::MONO),
            style: self.style.expect("TooltipStyle is required"),
            variant: self.variant.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_tooltip_visual_dark() {
        let mut text_sys = DummyTextSys;
        let spec = TooltipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            font: FontId::MONO,
            variant: TooltipVariant::Dark,
            style: Default::default(),
        };
        let style = spec.style;
        let res = tooltip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4); // bg fill, text, 2 arrow lines

        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == style.dark_bg));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == style.dark_text));
        assert!(
            matches!(&cmds[2], DrawCmd::StrokeLine { color, width, .. } if *color == style.dark_bg && *width == style.arrow_width)
        );
        assert!(
            matches!(&cmds[3], DrawCmd::StrokeLine { color, width, .. } if *color == style.dark_bg && *width == style.arrow_width)
        );
    }

    #[test]
    fn test_tooltip_visual_rust() {
        let mut text_sys = DummyTextSys;
        let spec = TooltipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            font: FontId::MONO,
            variant: TooltipVariant::Rust,
            style: Default::default(),
        };
        let style = spec.style;
        let res = tooltip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4);

        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == style.rust_bg));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == style.rust_text));
        assert!(
            matches!(&cmds[2], DrawCmd::StrokeLine { color, width, .. } if *color == style.rust_bg && *width == style.arrow_width)
        );
        assert!(
            matches!(&cmds[3], DrawCmd::StrokeLine { color, width, .. } if *color == style.rust_bg && *width == style.arrow_width)
        );
    }
}


