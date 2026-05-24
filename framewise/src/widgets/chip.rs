use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    theme::Theme,
    types::Rect,
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
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let h = 22.0_f32;
    let pad_x = 8.0_f32;

    let layout = spec.ts.prepare(spec.label, t.text_sm, spec.font);
    let w = spec.rect.w.max(32.0);
    let r = Rect::new(spec.rect.x, spec.rect.y, w, h);

    // Focus ring.
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect: r.inset(-2.0),
            color: t.rust,
            width: 2.0,
        });
    }

    let bg = if spec.active { t.ink } else { t.paper_elev };
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });
    cmds.push(DrawCmd::StrokeRect {
        rect: r,
        color: t.ink,
        width: 1.0,
    });

    let text_color = if spec.active { t.paper } else { t.ink };
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

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        ChipSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            font: self.font.unwrap_or(FontId::MONO),
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
            font: FontId::MONO,
            active: false,
            focused: false,
        };
        let res = chip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 3); // bg, border, text
        let t = Theme::framewise();
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == t.paper_elev));
        assert!(matches!(&cmds[1], DrawCmd::StrokeRect { color, .. } if *color == t.ink));
        assert!(matches!(&cmds[2], DrawCmd::Text { color, .. } if *color == t.ink));
    }

    #[test]
    fn test_chip_visual_active() {
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId::MONO,
            active: true,
            focused: false,
        };
        let res = chip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 3);
        let t = Theme::framewise();
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == t.ink)); // active bg
        assert!(matches!(&cmds[2], DrawCmd::Text { color, .. } if *color == t.paper));
        // active text
    }

    #[test]
    fn test_chip_visual_focused() {
        let mut text_sys = DummyTextSys;
        let spec = ChipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 50.0, 22.0),
            label: "Tag",
            font: FontId::MONO,
            active: false,
            focused: true,
        };
        let res = chip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4); // focus ring + 3 normal cmds
        let t = Theme::framewise();
        assert!(
            matches!(&cmds[0], DrawCmd::StrokeRect { color, width, .. } if *color == t.rust && *width == 2.0)
        );
        assert!(matches!(&cmds[1], DrawCmd::FillRect { color, .. } if *color == t.paper_elev));
    }
}
