use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

// ── Spec ─────────────────────────────────────────────────────────────────────

pub struct LabelSpec {
    pub rect: Rect,
    pub text: String,
    pub size: f32,
    pub text_color: Color,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct LabelResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct LabelInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for LabelResult {
    type Info = LabelInfo;

    fn into_parts(self) -> (DrawCommands, LabelInfo) {
        (self.draw, LabelInfo { layout: self.layout })
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a label widget.
pub fn label<T: TextSystem>(spec: LabelSpec, text_system: &mut T) -> LabelResult {
    let mut draw = DrawCommands::new();

    let layout = text_system.prepare(&spec.text, spec.size);

    draw.push(DrawCmd::Text {
        rect:  spec.rect,
        color: spec.text_color,
        handle: layout.handle,
    });

    LabelResult {
        draw,
        layout: LayoutInfo::tight(spec.rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::{TextLayout, TextHandle};
    use crate::types::Vec2;

    struct DummyTextSys;
    impl TextSystem for DummyTextSys {
        fn prepare(&mut self, _text: &str, _size: f32) -> TextLayout {
            TextLayout {
                handle: TextHandle(1),
                size: Vec2::new(100.0, 20.0),
            }
        }
        fn measure_byte_x(&self, _handle: TextHandle, _byte_index: usize) -> f32 { 0.0 }
        fn hit_test_x(&self, _handle: TextHandle, _x_offset: f32) -> usize { 0 }
    }

    #[test]
    fn test_label_draws_text() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Hello".to_string(),
            size: 16.0,
            text_color: Color::rgb(1.0, 1.0, 1.0),
        };
        let res = label(spec, &mut sys);
        
        let (draw, info) = res.into_parts();
        assert_eq!(info.layout.bounds.w, 100.0);
        
        // Should have one draw command for the text
        assert_eq!(draw.0.len(), 1);
        match &draw.0[0] {
            DrawCmd::Text { rect, color: _, handle } => {
                assert_eq!(rect.x, 0.0);
                assert_eq!(handle.0, 1);
            }
            _ => panic!("Expected text command"),
        }
    }
}
