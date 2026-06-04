use crate::text::{
    CaretGeom, FontId, TextBounds, TextFlow, TextHandle, TextLayout, TextMetrics, TextSystem,
};
use crate::types::{Rect, Vec2};

/// A dummy text system for unit tests that provides representative text dimensions.
/// Assumes each character is 8px wide and 16px tall, single line (no real wrapping).
pub struct DummyTextSys;

impl DummyTextSys {
    fn metrics(text: &str) -> TextMetrics {
        TextMetrics {
            size: Vec2::new(text.chars().count() as f32 * 8.0, 16.0),
            line_count: 1,
            truncated_horizontal: false,
            truncated_vertical: false,
        }
    }
}

impl TextSystem for DummyTextSys {
    fn measure(
        &mut self,
        text: &str,
        _size: f32,
        _font: FontId,
        _flow: TextFlow,
        _bounds: TextBounds,
    ) -> TextMetrics {
        Self::metrics(text)
    }

    fn prepare(
        &mut self,
        text: &str,
        _size: f32,
        _font: FontId,
        _flow: TextFlow,
        _rect: Rect,
    ) -> TextLayout {
        TextLayout {
            handle: TextHandle(0),
            metrics: Self::metrics(text),
        }
    }

    fn caret_geom(&self, _handle: TextHandle, byte_index: usize) -> CaretGeom {
        // Approximate 1 byte = 1 char for tests.
        CaretGeom {
            x: byte_index as f32 * 8.0,
            y_top: 0.0,
            height: 16.0,
        }
    }

    fn hit_test(&self, _handle: TextHandle, pos: Vec2) -> usize {
        // 8px per char/byte; ignore Y (single line).
        (pos.x / 8.0).round() as usize
    }
}
