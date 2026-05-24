use crate::{
    draw::{DrawCmd, DrawCommands},
    types::Color,
    types::{Rect, Vec2},
    widget::{LayoutInfo, WidgetResult},
};

pub struct DividerSpec {
    pub rect: Rect,
    pub color: Color,
    pub width: f32,
}

pub struct DividerResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct DividerInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for DividerResult {
    type Info = DividerInfo;
    fn into_parts(self) -> (DrawCommands, DividerInfo) {
        (
            self.draw,
            DividerInfo {
                layout: self.layout,
            },
        )
    }
}

pub fn divider(spec: DividerSpec) -> DividerResult {
    let mut draw = DrawCommands::new();
    let mid_y = spec.rect.y + spec.rect.h * 0.5;
    draw.push(DrawCmd::StrokeLine {
        p0: Vec2::new(spec.rect.x, mid_y),
        p1: Vec2::new(spec.rect.x + spec.rect.w, mid_y),
        color: spec.color,
        width: spec.width,
    });
    DividerResult {
        draw,
        layout: LayoutInfo::new(spec.rect, spec.rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divider_visual() {
        let spec = DividerSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 10.0),
            color: Color::WHITE,
            width: 1.0,
        };
        let res = divider(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![DrawCmd::StrokeLine {
                p0: Vec2::new(0.0, 5.0),
                p1: Vec2::new(100.0, 5.0),
                color: Color::WHITE,
                width: 1.0,
            }])
        );
    }
}
