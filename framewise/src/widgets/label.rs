use crate::{
    draw::{DrawCmd, DrawCommands},
    text::{FontId, TextSystem},
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetResult},
};

// ── Spec ─────────────────────────────────────────────────────────────────────

pub struct LabelSpec {
    pub rect: Rect,
    pub text: String,
    pub size: f32,
    pub font: FontId,
    pub text_color: Color,
    /// Draw a hairline rule at the bottom of the rect.
    pub rule: bool,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct LabelResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct LabelInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for LabelResult {
    type Info = LabelInfo;

    fn into_parts(self) -> (DrawCommands, LabelInfo) {
        (
            self.draw,
            LabelInfo {
                layout: self.layout,
            },
        )
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

pub fn label<T: TextSystem>(spec: LabelSpec, text_system: &mut T) -> LabelResult {
    let mut draw = DrawCommands::new();

    let layout = text_system.prepare(&spec.text, spec.size, spec.font);

    draw.push(DrawCmd::Text {
        rect: spec.rect,
        color: spec.text_color,
        handle: layout.handle,
    });

    if spec.rule {
        let y = spec.rect.y + spec.rect.h;
        draw.push(DrawCmd::StrokeLine {
            p0: Vec2::new(spec.rect.x, y),
            p1: Vec2::new(spec.rect.x + spec.rect.w, y),
            color: Color::linear_rgba(0.0, 0.0, 0.0, 0.12),
            width: 1.0,
        });
    }

    LabelResult {
        draw,
        layout: LayoutInfo::tight(spec.rect),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{test_utils::DummyTextSys, text::TextHandle};

    struct RecordingTextSys {
        font: Option<FontId>,
    }

    impl TextSystem for RecordingTextSys {
        fn prepare(&mut self, _text: &str, _size: f32, font: FontId) -> crate::text::TextLayout {
            self.font = Some(font);
            crate::text::TextLayout {
                handle: TextHandle(0),
                size: Vec2::new(0.0, 0.0),
            }
        }

        fn measure_byte_x(&self, _handle: TextHandle, _byte_index: usize) -> f32 {
            0.0
        }

        fn hit_test_x(&self, _handle: TextHandle, _x_offset: f32) -> usize {
            0
        }
    }

    #[test]
    fn test_label_draws_text() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Hello".to_string(),
            size: 16.0,
            font: FontId::SANS,
            text_color: Color::WHITE,
            rule: false,
        };
        let res = label(spec, &mut sys);

        let (draw, info) = res.into_parts();
        assert_eq!(info.layout.bounds.w, 100.0);

        assert_eq!(
            draw,
            DrawCommands(vec![DrawCmd::Text {
                rect: Rect::new(0.0, 0.0, 100.0, 50.0),
                color: Color::WHITE,
                handle: TextHandle(0),
            }])
        );
    }

    #[test]
    fn test_label_rule() {
        let mut sys = DummyTextSys;
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "Section".to_string(),
            size: 14.0,
            font: FontId::SANS,
            text_color: Color::WHITE,
            rule: true,
        };
        let res = label(spec, &mut sys);
        let (draw, _) = res.into_parts();
        assert_eq!(
            draw,
            DrawCommands(vec![
                DrawCmd::Text {
                    rect: Rect::new(0.0, 0.0, 100.0, 20.0),
                    color: Color::WHITE,
                    handle: TextHandle(0),
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(0.0, 20.0),
                    p1: Vec2::new(100.0, 20.0),
                    color: Color::linear_rgba(0.0, 0.0, 0.0, 0.12),
                    width: 1.0,
                }
            ])
        );
    }

    #[test]
    fn test_label_passes_spec_font_to_text_system() {
        let mut sys = RecordingTextSys { font: None };
        let expected = FontId(42);
        let spec = LabelSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            text: "font".to_string(),
            size: 14.0,
            font: expected,
            text_color: Color::WHITE,
            rule: false,
        };

        let _ = label(spec, &mut sys);

        assert_eq!(sys.font, Some(expected));
    }
}
