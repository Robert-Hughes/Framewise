use crate::text::pack_prepared_glyph_token;
use crate::text::types::{
    GlyphBaseKey, GlyphSubpixelSlot, RasterizedGlyphSlot, SampleShapedGlyphToken,
};
use crate::text::SampleTextBackend;
use swash::scale::{Render, Source, StrikeWith};
use zeno::{Format, Vector};

impl SampleTextBackend {
    pub fn prepared_glyph_slot(
        &mut self,
        token: SampleShapedGlyphToken,
        subpixel_x: u8,
    ) -> Option<RasterizedGlyphSlot> {
        let glyph_index = token.0 as usize;
        let slot_index = subpixel_x as usize;
        if matches!(
            self.glyph_cache[glyph_index].subpixels[slot_index],
            GlyphSubpixelSlot::Unloaded
        ) {
            let key = self.glyph_cache[glyph_index].key;
            let slot = self.rasterize_glyph_slot(key, subpixel_x);
            self.glyph_cache[glyph_index].subpixels[slot_index] = slot;
        }

        match self.glyph_cache[glyph_index].subpixels[slot_index] {
            GlyphSubpixelSlot::Loaded(slot) => Some(slot),
            GlyphSubpixelSlot::Empty | GlyphSubpixelSlot::Unloaded => None,
        }
    }

    fn rasterize_glyph_slot(&mut self, key: GlyphBaseKey, subpixel_x: u8) -> GlyphSubpixelSlot {
        let font = self.fonts[key.font_id as usize];
        let size = key.size as f32 / 10.0;

        // Build variation settings if applicable
        let mut vars = Vec::new();
        if self.font_has_wght[key.font_id as usize] {
            vars.push(("wght", key.weight as f32));
        }
        if self.font_has_opsz[key.font_id as usize] && key.opsz > 0 {
            vars.push(("opsz", key.opsz as f32));
        }

        let mut scaler_builder = self.scale_context.builder(font).size(size).hint(true);

        if !vars.is_empty() {
            scaler_builder = scaler_builder.variations(&vars);
        }

        let mut scaler = scaler_builder.build();

        // Calculate horizontal subpixel offset:
        // subpixel_x is 0, 1, 2, or 3 representing 0.0, 0.25, 0.50, 0.75
        let offset_x = subpixel_x as f32 * 0.25;
        let offset = Vector::new(offset_x, 0.0);

        let image = Render::new(&[
            Source::ColorOutline(0),
            Source::ColorBitmap(StrikeWith::BestFit),
            Source::Outline,
        ])
        .format(Format::Alpha)
        .offset(offset)
        .render(&mut scaler, key.glyph_index);

        let (w, h, left, top, data) = match image {
            Some(img) if img.placement.width > 0 && img.placement.height > 0 => (
                img.placement.width,
                img.placement.height,
                img.placement.left,
                img.placement.top,
                img.data,
            ),
            _ => {
                // Empty glyph (like space) or failed render
                return GlyphSubpixelSlot::Empty;
            }
        };

        // Shelf pack
        if self.current_x + w > self.atlas_size {
            self.current_y += self.row_height + 1;
            self.current_x = 0;
            self.row_height = 0;
        }

        if self.current_y + h > self.atlas_size {
            panic!("Atlas full!");
        }

        let x = self.current_x;
        let y = self.current_y;

        self.current_x += w + 1;
        self.row_height = self.row_height.max(h);

        // Copy bitmap to atlas
        for row in 0..h {
            let src_start = (row * w) as usize;
            let src_end = src_start + w as usize;

            let dst_start = ((y + row) * self.atlas_size + x) as usize;
            let dst_end = dst_start + w as usize;

            self.atlas_data[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
        }

        self.atlas_dirty = true;
        let token = pack_prepared_glyph_token(
            u16::try_from(x).expect("glyph atlas x exceeds u16 token limit"),
            u16::try_from(y).expect("glyph atlas y exceeds u16 token limit"),
            u16::try_from(w).expect("glyph atlas width exceeds u16 token limit"),
            u16::try_from(h).expect("glyph atlas height exceeds u16 token limit"),
        );
        GlyphSubpixelSlot::Loaded(RasterizedGlyphSlot { token, left, top })
    }
}
