use super::{PrepareGlyphRequest, TextBackend, TextLayout, WorkingClusterSource};
use crate::{
    draw::DrawCommands,
    types::{Color, Vec2},
};

impl<G: Copy> TextLayout<G> {
    /// Prepare and emit visible layout glyphs into a draw command glyph arena.
    ///
    /// `origin` is the final screen position (`rect.x`, `rect.y`) passed to the backend so it
    /// can apply subpixel offsets/positioning at the absolute draw location.
    ///
    /// The text backend may produce ink that extends outside the logical layout bounds of the
    /// text. A caller that needs hard containment must apply clipping or provide padding.
    ///
    /// The backend may return `None` for non-drawable glyphs such as spaces,
    /// newlines, zero-area glyphs, or failed rasterisation. Empty prepared
    /// output does not emit a `GlyphRun` command.
    pub fn emit_glyphs<B>(
        &self,
        commands: &mut DrawCommands,
        backend: &mut B,
        origin: Vec2,
        color: Color,
        z: u32,
    ) where
        B: TextBackend<ShapedGlyphToken = G>,
    {
        commands.reserve_glyphs(self.visible_glyph_count);
        commands.reserve_commands(1);

        let glyph_run_start = commands.glyph_run_start();

        for line in &self.lines {
            for cluster in &line.clusters {
                if !cluster.glyphs_visible {
                    continue;
                }

                match cluster.source {
                    WorkingClusterSource::Shaped {
                        run_index,
                        cluster_index,
                    } => {
                        let shaped_cluster = &self.runs[run_index].shaped.clusters[cluster_index];
                        for glyph in &shaped_cluster.glyphs {
                            let glyph_origin = Vec2::new(
                                origin.x + cluster.x + glyph.x,
                                origin.y + line.baseline_y + glyph.y,
                            );
                            if let Some(draw_glyph) = backend.prepare_glyph(PrepareGlyphRequest {
                                glyph: glyph.token,
                                glyph_origin,
                            }) {
                                commands.push_glyph(draw_glyph);
                            }
                        }
                    }
                    WorkingClusterSource::Empty => {}
                }
            }
        }

        commands.finish_glyph_run(glyph_run_start, color, z);
    }
}
