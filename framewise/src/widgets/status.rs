use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect},
    WidgetResult,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusVariant {
    Neutral,
    Ok,
    Warn,
    Err,
    Live,
}

pub struct StatusSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect: Rect,
    pub label: &'a str,
    pub font: FontId,
    pub variant: StatusVariant,
    pub style: StatusStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusStyle {
    pub dot_size: f32,
    pub gap: f32,
    pub text_size: f32,
    pub neutral: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub live: Color,
    pub text: Color,
}

impl Default for StatusStyle {
    fn default() -> Self {
        Self {
            dot_size: 6.0,
            gap: 8.0,
            text_size: 11.0,
            neutral: Color::from_srgb_u8(138, 131, 120, 255),
            ok: Color::from_srgb_f32(0.302, 0.541, 0.227, 1.0),
            warn: Color::from_srgb_u8(194, 90, 44, 255),
            err: Color::from_srgb_f32(0.702, 0.145, 0.122, 1.0),
            live: Color::from_srgb_u8(194, 90, 44, 255),
            text: Color::from_srgb_u8(138, 131, 120, 255),
        }
    }
}

pub struct StatusResult {
    pub draw: DrawCommands,
}

impl WidgetResult for StatusResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn status<'a, T: crate::text::TextSystem>(spec: StatusSpec<'a, T>) -> StatusResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    let dot_size = s.dot_size;
    let gap = s.gap;

    let dot_color = match spec.variant {
        StatusVariant::Neutral => s.neutral,
        StatusVariant::Ok => s.ok,
        StatusVariant::Warn => s.warn,
        StatusVariant::Err => s.err,
        StatusVariant::Live => s.live,
    };

    cmds.push(DrawCmd::FillRect {
        rect: Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
        color: dot_color,
    });

    let label_upper = spec.label.to_uppercase();
    let layout = spec.ts.prepare(&label_upper, s.text_size, spec.font);
    let ty = spec.rect.y + (dot_size - layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect: Rect::new(
            spec.rect.x + dot_size + gap,
            ty,
            layout.size.x,
            layout.size.y,
        ),
        color: s.text,
        handle: layout.handle,
    });

    StatusResult { draw: cmds }
}

pub struct StatusSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<StatusStyle>,
    pub variant: Option<StatusVariant>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> StatusSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            variant: None,
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
    pub fn style(mut self, style: StatusStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: StatusVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for StatusSpecBuilder<'a, T>
{
    type Spec = StatusSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.status_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        StatusSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self.font.unwrap_or(FontId::MONO),
            style: self.style.expect("StatusStyle is required"),
            variant: self.variant.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_status_visual_ok() {
        let mut text_sys = DummyTextSys;
        let spec = StatusSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            label: "Online",
            font: FontId::MONO,
            variant: StatusVariant::Ok,
            style: Default::default(),
        };
        let style = spec.style;
        let res = status(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);

        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == style.ok));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == style.text));
    }

    #[test]
    fn test_status_visual_warn() {
        let mut text_sys = DummyTextSys;
        let spec = StatusSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            label: "Warning",
            font: FontId::MONO,
            variant: StatusVariant::Warn,
            style: Default::default(),
        };
        let style = spec.style;
        let res = status(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);

        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == style.warn));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == style.text));
    }
}


