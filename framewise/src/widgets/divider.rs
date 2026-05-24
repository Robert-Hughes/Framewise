use crate::{
    draw::{DrawCmd, DrawCommands},
    theme::Theme,
    types::{Rect, Vec2},
    widget::{LayoutInfo, WidgetResult},
};

pub struct DividerSpec {
    pub rect: Rect,
}

pub struct DividerResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct DividerInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for DividerResult {
    type Info = DividerInfo;
    fn into_parts(self) -> (DrawCommands, DividerInfo) {
        (self.draw, DividerInfo { layout: self.layout })
    }
}

pub fn divider(spec: DividerSpec) -> DividerResult {
    let t = Theme::framewise();
    let mut draw = DrawCommands::new();
    let mid_y = spec.rect.y + spec.rect.h * 0.5;
    draw.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(spec.rect.x, mid_y),
        p1:    Vec2::new(spec.rect.x + spec.rect.w, mid_y),
        color: t.line,
        width: 1.0,
    });
    DividerResult {
        draw,
        layout: LayoutInfo::new(spec.rect, spec.rect),
    }
}
