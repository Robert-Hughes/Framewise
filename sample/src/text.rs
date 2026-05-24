use fontdue::{
    layout::{CoordinateSystem, GlyphPosition, Layout, TextStyle},
    Font, FontSettings,
};
use framewise::{FontId, TextHandle, TextLayout, TextSystem, Vec2};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32, // store size as u32 (size * 10.0 as u32) for hashing
}

pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct GlyphInfo {
    pub atlas_rect: AtlasRect,
}

pub struct CachedLayout {
    pub font_id: FontId,
    pub glyphs: Vec<GlyphPosition>,
    pub text_len: usize,
}

pub struct SampleTextSystem {
    pub fonts: Vec<Font>,
    layout: Layout,
    pub runs: Vec<CachedLayout>,

    // Atlas data
    pub glyph_cache: HashMap<GlyphKey, GlyphInfo>,
    pub atlas_data: Vec<u8>,
    pub atlas_size: u32,

    // Simple shelf allocator
    current_x: u32,
    current_y: u32,
    row_height: u32,

    pub atlas_dirty: bool,
}

impl SampleTextSystem {
    pub fn new() -> Self {
        let mono_data = include_bytes!("../assets/JetBrainsMono-Regular.ttf") as &[u8];
        let mono = Font::from_bytes(mono_data, FontSettings::default())
            .expect("failed to load JetBrainsMono font");
        let sans_data = include_bytes!("../assets/InterTight-Regular.ttf") as &[u8];
        let sans = Font::from_bytes(sans_data, FontSettings::default())
            .expect("failed to load InterTight font");
        let sans_bold_data = include_bytes!("../assets/InterTight-Bold.ttf") as &[u8];
        let sans_bold = Font::from_bytes(sans_bold_data, FontSettings::default())
            .expect("failed to load InterTight-Bold font");

        let atlas_size = 1024;
        Self {
            fonts: vec![mono, sans, sans_bold],
            layout: Layout::new(CoordinateSystem::PositiveYDown),
            runs: Vec::new(),
            glyph_cache: HashMap::new(),
            atlas_data: vec![0; (atlas_size * atlas_size) as usize],
            atlas_size,
            current_x: 0,
            current_y: 0,
            row_height: 0,
            atlas_dirty: false,
        }
    }

    /// Called at the start of each frame to reset the text layout handles.
    pub fn begin_frame(&mut self) {
        self.runs.clear();
        self.atlas_dirty = false;
    }

    fn ensure_glyph(&mut self, key: GlyphKey) {
        if self.glyph_cache.contains_key(&key) {
            return;
        }

        let font = self
            .fonts
            .get(key.font_id as usize)
            .expect("unknown font id");
        let (metrics, bitmap) = font.rasterize_indexed(key.glyph_index, key.size as f32 / 10.0);
        let w = metrics.width as u32;
        let h = metrics.height as u32;

        if w == 0 || h == 0 {
            self.glyph_cache.insert(
                key,
                GlyphInfo {
                    atlas_rect: AtlasRect {
                        x: 0,
                        y: 0,
                        w: 0,
                        h: 0,
                    },
                },
            );
            return;
        }

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

            self.atlas_data[dst_start..dst_end].copy_from_slice(&bitmap[src_start..src_end]);
        }

        self.atlas_dirty = true;
        self.glyph_cache.insert(
            key,
            GlyphInfo {
                atlas_rect: AtlasRect { x, y, w, h },
            },
        );
    }
}

impl TextSystem for SampleTextSystem {
    fn prepare(&mut self, text: &str, size: f32, font_id: FontId) -> TextLayout {
        self.layout.clear();
        let font = self.fonts.get(font_id.0 as usize).expect("unknown font id");
        self.layout.append(&[font], &TextStyle::new(text, size, 0));

        let mut width = 0.0_f32;
        let height = font
            .horizontal_line_metrics(size)
            .map(|m| m.new_line_size)
            .unwrap_or(size);

        let glyphs = self.layout.glyphs().to_vec();
        for g in &glyphs {
            let right = g.x + g.width as f32;
            if right > width {
                width = right;
            }

            let key = GlyphKey {
                font_id: font_id.0,
                glyph_index: g.key.glyph_index,
                size: (g.key.px * 10.0) as u32,
            };
            self.ensure_glyph(key);
        }

        let handle_id = self.runs.len();
        self.runs.push(CachedLayout {
            font_id,
            glyphs,
            text_len: text.len(),
        });

        TextLayout {
            handle: TextHandle(handle_id),
            size: Vec2::new(width, height),
        }
    }

    fn measure_byte_x(&self, handle: TextHandle, byte_index: usize) -> f32 {
        let run = &self.runs[handle.0];

        // If we're at the very end of the string
        if byte_index >= run.text_len {
            if let Some(last) = run.glyphs.last() {
                // Approximate end by adding width, or just advance. fontdue gives x + width.
                // Wait, advance might be better, but we don't have it easily. x + width works for simple fonts.
                // Let's just use x + width for the end of the text.
                return last.x + last.width as f32;
            }
            return 0.0;
        }

        for g in &run.glyphs {
            if g.byte_offset >= byte_index {
                return g.x;
            }
        }

        0.0
    }

    fn hit_test_x(&self, handle: TextHandle, x_offset: f32) -> usize {
        let run = &self.runs[handle.0];

        if run.glyphs.is_empty() {
            return 0;
        }

        for g in &run.glyphs {
            let mid = g.x + (g.width as f32 / 2.0);
            if x_offset < mid {
                return g.byte_offset;
            }
        }

        run.text_len
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_cache_keys_include_font_id() {
        let mut sys = SampleTextSystem::new();

        let _ = sys.prepare("A", 12.0, FontId(0));
        let _ = sys.prepare("A", 12.0, FontId(1));

        assert!(sys
            .glyph_cache
            .keys()
            .any(|key| key.font_id == 0));
        assert!(sys
            .glyph_cache
            .keys()
            .any(|key| key.font_id == 1));
    }
}
