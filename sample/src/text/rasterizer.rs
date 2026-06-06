use crate::text::types::{AtlasRect, GlyphInfo, GlyphKey};
use crate::text::SampleTextSystem;
use swash::scale::{Render, Source, StrikeWith};
use zeno::{Format, Vector};

impl SampleTextSystem {
    pub fn ensure_glyph(&mut self, key: GlyphKey) {
        if self.glyph_cache.contains_key(&key) {
            return;
        }

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
        let offset_x = key.subpixel_x as f32 * 0.25;
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
                self.glyph_cache.insert(
                    key,
                    GlyphInfo {
                        atlas_rect: AtlasRect {
                            x: 0,
                            y: 0,
                            w: 0,
                            h: 0,
                        },
                        left: 0,
                        top: 0,
                    },
                );
                return;
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
        self.glyph_cache.insert(
            key,
            GlyphInfo {
                atlas_rect: AtlasRect { x, y, w, h },
                left,
                top,
            },
        );
    }

    pub fn get_glyph_metrics(
        &mut self,
        font_id: u16,
        glyph_index: u16,
        size: f32,
        subpixel_x: u8,
        weight: u16,
        opsz: u16,
    ) -> (u32, u32) {
        let key = GlyphKey {
            font_id,
            glyph_index,
            size: (size * 10.0) as u32,
            subpixel_x,
            weight,
            opsz,
        };
        self.ensure_glyph(key);
        let info = self.glyph_cache.get(&key).unwrap();
        (info.atlas_rect.w, info.atlas_rect.h)
    }
}
