use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, text::TextSystem, theme::Theme, types::{Rect, Vec2}
};

pub struct TabsSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Bounding rect; only x/y/w used — height is fixed at 36.
    pub rect:         Rect,
    pub items:        &'a [&'a str],
    pub active_index: usize,
    pub focused:      Option<usize>,
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
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let tab_h = 36.0_f32;
    let pad_x = 18.0_f32;
    let underbar_h = 3.0_f32;

    // Bottom border across the full width.
    let border_y = spec.rect.y + tab_h;
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(spec.rect.x, border_y),
        p1:    Vec2::new(spec.rect.x + spec.rect.w, border_y),
        color: t.ink,
        width: 1.0,
    });

    let mut x = spec.rect.x;

    for (i, label) in spec.items.iter().enumerate() {
        let is_active = i == spec.active_index;
        let is_focused = spec.focused == Some(i);

        let layout = spec.ts.prepare(label, t.text_md);
        let tab_w = layout.size.x + pad_x * 2.0;
        let tab_rect = Rect::new(x, spec.rect.y, tab_w, tab_h);

        // Focus ring.
        if is_focused {
            cmds.push(DrawCmd::StrokeRect {
                rect:  tab_rect.inset(-2.0),
                color: t.rust,
                width: 2.0,
            });
        }

        let text_color = if is_active { t.ink } else { t.muted };
        let ty = spec.rect.y + (tab_h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
            color:  text_color,
            handle: layout.handle,
        });

        // Active underbar: 3px rust rect sitting on the bottom border.
        if is_active {
            cmds.push(DrawCmd::FillRect {
                rect:  Rect::new(x, border_y - underbar_h * 0.5, tab_w, underbar_h),
                color: t.rust,
            });
        }

        x += tab_w;
    }

    TabsResult { draw: cmds }
}




pub struct TabsSpecBuilder<'a, T: crate::text::TextSystem> {
    pub items: Option<&'a [&'a str]>,
    pub active_index: Option<usize>,
    pub focused: Option<Option<usize>>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> TabsSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            items: None,
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
    pub fn active_index(mut self, active_index: usize) -> Self {
        self.active_index = Some(active_index);
        self
    }
    pub fn focused(mut self, focused: Option<usize>) -> Self {
        self.focused = Some(focused);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for TabsSpecBuilder<'a, T> {
    type Spec = TabsSpec<'a, T>;

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
        TabsSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
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
            active_index: 0,
            focused: None,
        };
        let res = tabs(spec);
        let cmds = res.draw.0;

        // Commands:
        // 0: bottom border line
        // Iter 0 (Active):
        // 1: text (ink color)
        // 2: underbar (rust)
        // Iter 1 (Inactive):
        // 3: text (muted color)
        assert_eq!(cmds.len(), 4);
        let t = Theme::framewise();

        assert!(matches!(&cmds[0], DrawCmd::StrokeLine { color, .. } if *color == t.ink)); // bottom border
        
        // Item 0
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == t.ink)); // active text
        assert!(matches!(&cmds[2], DrawCmd::FillRect { color, .. } if *color == t.rust)); // active underbar

        // Item 1
        assert!(matches!(&cmds[3], DrawCmd::Text { color, .. } if *color == t.muted)); // inactive text
    }

    #[test]
    fn test_tabs_visual_focused() {
        let mut text_sys = DummyTextSys;
        let items = ["Tab1", "Tab2"];
        let spec = TabsSpec {
            ts: &mut text_sys,
            rect: Rect::new(0.0, 0.0, 300.0, 36.0),
            items: &items,
            active_index: 1,
            focused: Some(1),
        };
        let res = tabs(spec);
        let cmds = res.draw.0;

        // Commands:
        // 0: bottom border line
        // Iter 0 (Inactive):
        // 1: text (muted color)
        // Iter 1 (Active + Focused):
        // 2: focus ring (rust stroke)
        // 3: text (ink color)
        // 4: underbar (rust)
        assert_eq!(cmds.len(), 5);
        let t = Theme::framewise();

        assert!(matches!(&cmds[0], DrawCmd::StrokeLine { color, .. } if *color == t.ink)); // bottom border
        
        // Item 0
        assert!(matches!(&cmds[1], DrawCmd::Text { color, .. } if *color == t.muted)); // inactive text

        // Item 1
        assert!(matches!(&cmds[2], DrawCmd::StrokeRect { color, width, .. } if *color == t.rust && *width == 2.0)); // focus ring
        assert!(matches!(&cmds[3], DrawCmd::Text { color, .. } if *color == t.ink)); // active text
        assert!(matches!(&cmds[4], DrawCmd::FillRect { color, .. } if *color == t.rust)); // active underbar
    }
}
