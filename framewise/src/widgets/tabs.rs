use crate::{
    draw::{DrawCmd, DrawCommands},
    text::FontId,
    types::{Color, Rect, Vec2},
    WidgetResult,
};

pub struct TabsSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Bounding rect; only x/y/w used — height is fixed at 36.
    pub rect: Rect,
    pub items: &'a [&'a str],
    pub font: FontId,
    pub active_index: usize,
    pub focused: Option<usize>,
    pub style: TabsStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabsStyle {
    pub height: f32,
    pub pad_x: f32,
    pub underbar_height: f32,
    pub text_size: f32,
    pub border: Color,
    pub text: Color,
    pub inactive_text: Color,
    pub accent: Color,
    pub focus: Color,
    pub border_width: f32,
    pub focus_width: f32,
    pub focus_offset: f32,
}

impl Default for TabsStyle {
    fn default() -> Self {
        Self {
            height: 36.0,
            pad_x: 18.0,
            underbar_height: 3.0,
            text_size: 13.0,
            border: Color::from_srgb_u8(21, 19, 15, 255),
            text: Color::from_srgb_u8(21, 19, 15, 255),
            inactive_text: Color::from_srgb_u8(138, 131, 120, 255),
            accent: Color::from_srgb_u8(194, 90, 44, 255),
            focus: Color::from_srgb_u8(194, 90, 44, 255),
            border_width: 1.0,
            focus_width: 2.0,
            focus_offset: 2.0,
        }
    }
}

pub struct TabsResult {
    pub draw: DrawCommands,
}

impl WidgetResult for TabsResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn tabs<'a, T: crate::text::TextSystem>(spec: TabsSpec<'a, T>) -> TabsResult {
    let mut cmds = DrawCommands::new();
    let s = spec.style;

    let tab_h = s.height;
    let pad_x = s.pad_x;
    let underbar_h = s.underbar_height;

    // Bottom border across the full width.
    let border_y = spec.rect.y + tab_h;
    cmds.push(DrawCmd::StrokeLine {
        p0: Vec2::new(spec.rect.x, border_y),
        p1: Vec2::new(spec.rect.x + spec.rect.w, border_y),
        color: s.border,
        width: s.border_width,
    });

    let mut x = spec.rect.x;

    for (i, label) in spec.items.iter().enumerate() {
        let is_active = i == spec.active_index;
        let is_focused = spec.focused == Some(i);

        let layout = spec.ts.prepare(label, s.text_size, spec.font);
        let tab_w = layout.size.x + pad_x * 2.0;
        let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

        // Focus ring.
        if is_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect: tab_rect.inset(-s.focus_offset),
                color: s.focus,
                width: s.focus_width,
            });
        }

        let text_color = if is_active { s.text } else { s.inactive_text };
        let ty = spec.rect.y + (tab_h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
            color: text_color,
            handle: layout.handle,
        });

        // Active underbar: 3px rust rect sitting on the bottom border + upticks at the ends.
        if is_active {
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                color: s.accent,
            });
            // Left uptick (3px wide, 9px tall)
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(x, border_y - 7.5, 3.0, 9.0),
                color: s.accent,
            });
            // Right uptick (3px wide, 9px tall)
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(x + tab_w - 3.0, border_y - 7.5, 3.0, 9.0),
                color: s.accent,
            });
        }

        x += tab_w;
    }

    TabsResult { draw: cmds }
}

pub struct TabsSpecBuilder<'a, T: crate::text::TextSystem> {
    pub items: Option<&'a [&'a str]>,
    pub font: Option<FontId>,
    pub style: Option<TabsStyle>,
    pub active_index: Option<usize>,
    pub focused: Option<Option<usize>>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> TabsSpecBuilder<'a, T> {
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
    pub fn style(mut self, style: TabsStyle) -> Self {
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
    for TabsSpecBuilder<'a, T>
{
    type Spec = TabsSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.style = Some(theme.tabs_style());
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        TabsSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
            font: self.font.unwrap_or(FontId::SANS),
            style: self.style.expect("TabsStyle is required"),
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
    fn test_tabs_visual_normal() {
        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            font: FontId::SANS,
            active_index: 0,
            focused: None,
            style: Default::default(),
        };
        let style = spec.style;
        let res = tabs(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(18.0, 10.0, 32.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(65.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::Text {
                    rect: Rect::new(86.0, 10.0, 32.0, 16.0),
                    color: style.inactive_text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_tabs_visual_focused() {
        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            font: FontId::SANS,
            active_index: 1,
            focused: Some(1),
            style: Default::default(),
        };
        let style = spec.style;
        let res = tabs(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 36.0),
                    p1: Vec2::new(300.0, 36.0),
                    color: style.border,
                    width: style.border_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(18.0, 10.0, 32.0, 16.0),
                    color: style.inactive_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(66.0, -2.0, 72.0, 40.0),
                    color: style.focus,
                    width: style.focus_width,
                },
                DrawCmd::Text {
                    rect: Rect::new(86.0, 10.0, 32.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::FillRect {
                    rect: Rect::new(68.0, 34.5, 68.0, 3.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(68.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(133.0, 28.5, 3.0, 9.0),
                    color: style.accent,
                },
            ])
        );
    }
}


