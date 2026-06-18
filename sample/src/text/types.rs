use framewise::PreparedGlyphHandle;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleGlyphToken {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32,   // store size as u32 (size * 10.0 as u32) for hashing
    pub weight: u16, // 100-900, variable font weight axis
    pub opsz: u16,   // optical size axis value (typically matches size in pt)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32,
    pub weight: u16,
    pub opsz: u16,
    pub subpixel_x: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphInfo {
    pub atlas_rect: AtlasRect, /// where the bitmap lives inside the atlas texture
    pub left: i32, // x offset from glyph origin to bitmap left edge
    pub top: i32, // y offset from glyph origin/baseline to bitmap top edge
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreparedGlyphImage {
    pub atlas_rect: AtlasRect,
}

pub trait PreparedGlyphResources {
    fn resolve_glyph(&self, handle: PreparedGlyphHandle) -> Option<PreparedGlyphImage>;
}
