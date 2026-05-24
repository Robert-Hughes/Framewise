use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    theme::Theme,
    types::{Rect, Vec2},
    widget::{LayoutInfo, WidgetResult},
};

pub struct WindowButton {
    pub symbol: &'static str,
}

pub struct WindowSpec<'a> {
    pub rect:        Rect,
    pub title:       &'a str,
    pub buttons:     &'a [WindowButton],
    pub status_bar:  bool,
    pub status_text: &'a str,
}

pub struct WindowResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct WindowInfo {
    pub layout: LayoutInfo,
}

impl WindowInfo {
    pub fn content_rect(&self) -> Rect { self.layout.content_bounds }
}

impl WidgetResult for WindowResult {
    type Info = WindowInfo;
    fn into_parts(self) -> (DrawCommands, WindowInfo) {
        (self.draw, WindowInfo { layout: self.layout })
    }
}

pub fn window<T: TextSystem>(spec: WindowSpec<'_>, ts: &mut T) -> WindowResult {
    let t = Theme::framewise();
    let mut draw = DrawCommands::new();

    let title_h = 26.0_f32;
    let btn_size = 16.0_f32;
    let status_h = if spec.status_bar { 22.0_f32 } else { 0.0 };

    // Body.
    draw.push(DrawCmd::FillRect { rect: spec.rect, color: t.paper_elev });
    draw.push(DrawCmd::StrokeRect { rect: spec.rect, color: t.ink, width: 1.0 });

    // Title bar.
    let title_rect = Rect::new(spec.rect.x, spec.rect.y, spec.rect.w, title_h);
    draw.push(DrawCmd::FillRect { rect: title_rect, color: t.ink });

    let title_upper = spec.title.to_uppercase();
    let title_layout = ts.prepare(&title_upper, t.text_sm);
    let tty = spec.rect.y + (title_h - title_layout.size.y) * 0.5;
    draw.push(DrawCmd::Text {
        rect:   Rect::new(spec.rect.x + 10.0, tty, title_layout.size.x, title_layout.size.y),
        color:  t.paper,
        handle: title_layout.handle,
    });

    // Window buttons (right side).
    let mut btn_x = spec.rect.x + spec.rect.w - 4.0;
    for btn in spec.buttons.iter().rev() {
        btn_x -= btn_size + 2.0;
        let btn_layout = ts.prepare(btn.symbol, t.text_sm);
        let bty = spec.rect.y + (title_h - btn_layout.size.y) * 0.5;
        draw.push(DrawCmd::Text {
            rect:   Rect::new(btn_x, bty, btn_layout.size.x, btn_layout.size.y),
            color:  t.paper,
            handle: btn_layout.handle,
        });
    }

    // Status bar.
    if spec.status_bar {
        let bar_y = spec.rect.y + spec.rect.h - status_h;
        draw.push(DrawCmd::StrokeLine {
            p0:    Vec2::new(spec.rect.x, bar_y),
            p1:    Vec2::new(spec.rect.x + spec.rect.w, bar_y),
            color: t.line,
            width: 1.0,
        });
        let status_layout = ts.prepare(spec.status_text, t.text_sm);
        let sty = bar_y + (status_h - status_layout.size.y) * 0.5;
        draw.push(DrawCmd::Text {
            rect:   Rect::new(spec.rect.x + 10.0, sty, status_layout.size.x, status_layout.size.y),
            color:  t.muted,
            handle: status_layout.handle,
        });
    }

    let content_top = spec.rect.y + title_h + 16.0;
    let content_bottom = spec.rect.y + spec.rect.h - status_h - 16.0;
    let content = Rect::new(
        spec.rect.x + 16.0,
        content_top,
        spec.rect.w - 32.0,
        (content_bottom - content_top).max(0.0),
    );

    WindowResult {
        draw,
        layout: LayoutInfo::new(spec.rect, content),
    }
}
