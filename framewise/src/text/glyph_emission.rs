use super::{PrepareGlyphRequest, TextBackend, TextLayout, TextStyle};
use crate::{
    draw::DrawCommands,
    types::{Color, Vec2},
};

impl<G: Copy> TextLayout<G> {
    /// Prepare and emit visible layout glyphs into a draw command glyph arena.
    ///
    /// The backend may return `None` for non-drawable glyphs such as spaces,
    /// newlines, zero-area glyphs, or failed rasterisation. Empty prepared
    /// output does not emit a `GlyphRun` command.
    pub fn emit_glyphs<B>(
        &self,
        commands: &mut DrawCommands,
        backend: &mut B,
        origin: Vec2,
        style: TextStyle,
        color: Color,
        z: u32,
    ) where
        B: TextBackend<ShapedGlyphId = G>,
    {
        let glyphs = self.glyphs.iter().filter_map(|glyph| {
            backend.prepare_glyph(PrepareGlyphRequest {
                glyph: glyph.id,
                style,
                glyph_origin: Vec2::new(origin.x + glyph.origin.x, origin.y + glyph.origin.y),
            })
        });
        commands.push_glyph_run(glyphs, color, z);
    }
}
