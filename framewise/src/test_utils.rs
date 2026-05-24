use crate::text::{FontId, TextHandle, TextLayout, TextSystem};
use crate::types::Vec2;

/// A dummy text system for unit tests that provides representative text dimensions.
/// Assumes each character is 8px wide and 16px tall.
pub struct DummyTextSys;

impl TextSystem for DummyTextSys {
    fn prepare(&mut self, text: &str, _size: f32, _font: FontId) -> TextLayout {
        let width = text.chars().count() as f32 * 8.0;
        let height = 16.0;
        TextLayout {
            handle: TextHandle(0),
            size: Vec2::new(width, height),
        }
    }

    fn measure_byte_x(&self, _handle: TextHandle, byte_index: usize) -> f32 {
        // Just approximate 1 byte = 1 char for tests.
        byte_index as f32 * 8.0
    }

    fn hit_test_x(&self, _handle: TextHandle, x_offset: f32) -> usize {
        // 8px per char/byte.
        (x_offset / 8.0).round() as usize
    }
}
