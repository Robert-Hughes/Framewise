use crate::types::Vec2;

/// A lightweight application-owned font handle.
///
/// Framewise never loads or owns font files. It only passes this handle to the
/// application's `TextSystem`, which decides how the handle maps to real font
/// data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct FontId(pub u16);

/// Semantic font roles used by themes and builders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontRole {
    Sans,
    Mono,
}

/// An opaque handle to a text layout prepared by the application's text system.
///
/// Framewise does not know how text is shaped or rasterised. It just passes this
/// handle to the renderer via `DrawCmd::Text`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextHandle(pub usize);

/// The geometry and handle for a piece of prepared text.
#[derive(Debug, Clone, Copy)]
pub struct TextLayout {
    /// The opaque handle to give to the renderer.
    pub handle: TextHandle,
    /// The logical size of the text (used for layout and centering).
    pub size: Vec2,
}

/// A trait implemented by the application to prepare text during the UI pass.
///
/// This ensures that the cost of shaping and caching text happens explicitly
/// when the widget is built, keeping the render pass fast and mechanical.
pub trait TextSystem {
    /// Prepare the given string at the specified font size.
    /// Returns a layout containing the text's size and an opaque handle.
    fn prepare(&mut self, text: &str, size: f32, font: FontId) -> TextLayout;

    /// Get the X offset (in logical pixels) of the character at the given byte index.
    fn measure_byte_x(&self, handle: TextHandle, byte_index: usize) -> f32;

    /// Find the closest byte index to the given X pixel offset.
    fn hit_test_x(&self, handle: TextHandle, x_offset: f32) -> usize;
}
