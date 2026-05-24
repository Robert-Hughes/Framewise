use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, text::TextSystem, theme::Theme, types::{Color, Rect, Vec2}
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipVariant {
    Dark,
    Rust,
}

pub struct TooltipSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect:    Rect,
    pub text:    &'a str,
    pub variant: TooltipVariant,
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
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let pad_x = 8.0_f32;
    let pad_y_top = 5.0_f32;
    let pad_y_bot = 6.0_f32;
    let arrow_h = 4.0_f32;
    let arrow_w = 8.0_f32;

    let (bg, text_color): (Color, Color) = match spec.variant {
        TooltipVariant::Dark => (t.ink, t.paper),
        TooltipVariant::Rust => (t.rust, Color::WHITE),
    };

    let layout = spec.ts.prepare(spec.text, t.text_sm);
    let box_w = (layout.size.x + pad_x * 2.0).min(240.0);
    let box_h = layout.size.y + pad_y_top + pad_y_bot;

    let r = Rect::new(spec.rect.x, spec.rect.y, box_w, box_h);
    cmds.push(DrawCmd::FillRect { rect: r, color: bg });

    cmds.push(DrawCmd::Text {
        rect:   Rect::new(r.x + pad_x, r.y + pad_y_top, layout.size.x, layout.size.y),
        color:  text_color,
        handle: layout.handle,
    });

    // Arrow triangle below (two lines converging to a point).
    let arrow_x = r.x + 14.0;
    let arrow_y = r.y + box_h;
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(arrow_x, arrow_y),
        p1:    Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: 1.5,
    });
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(arrow_x + arrow_w, arrow_y),
        p1:    Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
        color: bg,
        width: 1.5,
    });

    TooltipResult { draw: cmds }
}




pub struct TooltipSpecBuilder<'a, T: crate::text::TextSystem> {
    pub text: Option<&'a str>,
    pub variant: Option<TooltipVariant>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> TooltipSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            text: None,
            variant: None,
            rect: None,
            ts: None,
        }
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn variant(mut self, variant: TooltipVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for TooltipSpecBuilder<'a, T> {
    type Spec = TooltipSpec<'a, T>;

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
        TooltipSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            text: self.text.unwrap(),
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
            variant: TooltipVariant::Dark,
        };
        let res = tooltip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4); // bg fill, text, 2 arrow lines
        let t = Theme::framewise();
        
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == t.ink));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == t.paper));
        assert!(matches!(&cmds[2], DrawCmd::StrokeLine { color, width, .. } if *color == t.ink && *width == 1.5));
        assert!(matches!(&cmds[3], DrawCmd::StrokeLine { color, width, .. } if *color == t.ink && *width == 1.5));
    }

    #[test]
    fn test_tooltip_visual_rust() {
        let mut text_sys = DummyTextSys;
        let spec = TooltipSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            variant: TooltipVariant::Rust,
        };
        let res = tooltip(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 4);
        let t = Theme::framewise();
        
        assert!(matches!(&cmds[0], DrawCmd::FillRect { color, .. } if *color == t.rust));
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == Color::WHITE));
        assert!(matches!(&cmds[2], DrawCmd::StrokeLine { color, width, .. } if *color == t.rust && *width == 1.5));
        assert!(matches!(&cmds[3], DrawCmd::StrokeLine { color, width, .. } if *color == t.rust && *width == 1.5));
    }
}
