use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect, Vec2},
    WidgetResult,
};

pub struct SegmentedSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Top-left origin. Height is fixed at h_md (28).
    pub rect: Rect,
    pub items: &'a [&'a str],
    pub font: FontId,
    pub active_index: usize,
    pub focused: Option<usize>,
    pub style: SegmentedStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentedStyle {
    pub height: f32,
    pub pad_x: f32,
    pub text_size: f32,
    pub background: Color,
    pub border: Color,
    pub active_bg: Color,
    pub text: Color,
    pub active_text: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_inset: f32,
}

impl Default for SegmentedStyle {
    fn default() -> Self {
        Self {
            height: 28.0,
            pad_x: 14.0,
            text_size: 13.0,
            background: Color::from_srgb_u8(251, 249, 244, 255),
            border: Color::from_srgb_u8(21, 19, 15, 255),
            active_bg: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            active_text: Color::from_srgb_u8(244, 241, 234, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_inset: 2.0,
        }
    }
}

pub struct SegmentedResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SegmentedResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn segmented<'a, T: crate::text::TextSystem>(spec: SegmentedSpec<'a, T>) -> SegmentedResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    if spec.items.is_empty() {
        return SegmentedResult { draw: cmds };
    }

    let h = s.height;
    let pad_x = s.pad_x;

    // Pre-prepare all labels to get their widths.
    let layouts: Vec<_> = spec
        .items
        .iter()
        .map(|text| spec.ts.prepare(text, s.text_size, spec.font))
        .collect();
    let widths: Vec<f32> = layouts.iter().map(|l| l.size.x + pad_x * 2.0).collect();
    let total_w: f32 = widths.iter().sum();

    let outer = Rect::new(spec.rect.x, spec.rect.y, total_w, h);

    cmds.push(DrawCmd::FillRect {
        rect: outer,
        color: s.background,
    });
    cmds.push(DrawCmd::StrokeRect {
        rect: outer,
        color: s.border,
        width: s.border_width,
    });

    let mut x = spec.rect.x;
    for (i, (layout, &w)) in layouts.iter().zip(widths.iter()).enumerate() {
        let is_active = i == spec.active_index;
        let seg_rect = Rect::new(x, spec.rect.y, w, h);

        if is_active {
            cmds.push(DrawCmd::FillRect {
                rect: seg_rect,
                color: s.active_bg,
            });
        }

        // Focus ring (inset to stay within bounds).
        if spec.focused == Some(i) {
            cmds.push(DrawCmd::StrokeRect {
                rect: seg_rect.inset(s.focus_inset),
                color: s.focus,
                width: s.focus_width,
            });
        }

        // Divider between segments (right edge, except last).
        if i + 1 < spec.items.len() {
            let div_x = x + w;
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(div_x, spec.rect.y),
                p1: Vec2::new(div_x, spec.rect.y + h),
                color: s.border,
                width: s.border_width,
            });
        }

        let text_color = if is_active { s.active_text } else { s.text };
        let ty = spec.rect.y + (h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
            color: text_color,
            handle: layout.handle,
        });

        x += w;
    }

    SegmentedResult { draw: cmds }
}

pub struct SegmentedSpecBuilder<'a, T: crate::text::TextSystem> {
    pub items: Option<&'a [&'a str]>,
    pub font: Option<FontId>,
    pub style: Option<SegmentedStyle>,
    pub active_index: Option<usize>,
    pub focused: Option<Option<usize>>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> SegmentedSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            items: None,
            font: None,
            style: None,
            active_index: None,
            focused: None,
            rect: None,
            ts: None,
        }
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: SegmentedStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn active_index(mut self, active_index: usize) -> Self {
        self.active_index = Some(active_index);
        self
    }
    pub fn focused(mut self, focused: Option<usize>) -> Self {
        self.focused = Some(focused);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T>
    for SegmentedSpecBuilder<'a, T>
{
    type Spec = SegmentedSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.segmented_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        SegmentedSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
            font: self.font.unwrap_or(FontId::SANS),
            style: self.style.expect("SegmentedStyle is required"),
            active_index: self.active_index.unwrap(),
            focused: self.focused.unwrap(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_segmented_visual_normal() {
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId::SANS,
            active_index: 0,
            focused: None,
            style: Default::default(),
        };
        let style = spec.style;
        let res = segmented(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_segmented_visual_focused() {
        let mut text_sys = DummyTextSys;
        let items = ["A", "B"];
        let spec = SegmentedSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 200.0, 28.0),
            items: &items,
            font: FontId::SANS,
            active_index: 1,
            focused: Some(1),
            style: Default::default(),
        };
        let style = spec.style;
        let res = segmented(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.background,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(36.0, 0.0),
                    p1: Vec2::new(36.0, 28.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, 6.0, 8.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(36.0, 0.0, 36.0, 28.0),
                    color: style.active_bg,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(38.0, 2.0, 32.0, 24.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(50.0, 6.0, 8.0, 16.0),
                    color: style.active_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

}

