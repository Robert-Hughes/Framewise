use framewise::PreparedGlyphToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphBaseKey {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32,   // store size as u32 (size * 10.0 as u32) for hashing
    pub weight: u16, // 100-900, variable font weight axis
    pub opsz: u16,   // optical size axis value (typically matches size in pt)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SampleShapedGlyphToken(pub u32);

pub type SampleGlyphToken = SampleShapedGlyphToken;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RasterizedGlyphSlot {
    /// Packed atlas source rectangle for the sample renderer.
    ///
    /// The token format assumes one atlas texture and `u16` x/y/w/h lanes.
    pub token: PreparedGlyphToken,
    /// Screen-placement offsets from the glyph origin/baseline.
    pub left: i32, // x offset from glyph origin to bitmap left edge
    pub top: i32, // y offset from glyph origin/baseline to bitmap top edge
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphSubpixelSlot {
    Unloaded,
    Empty,
    Loaded(RasterizedGlyphSlot),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedGlyph {
    pub key: GlyphBaseKey,
    pub subpixels: [GlyphSubpixelSlot; 4],
}

impl CachedGlyph {
    pub fn unloaded(key: GlyphBaseKey) -> Self {
        Self {
            key,
            subpixels: [GlyphSubpixelSlot::Unloaded; 4],
        }
    }
}

/// Pack a sample single-atlas source rectangle into a prepared glyph token.
///
/// The sample format stores x/y/w/h as four `u16` lanes and therefore assumes a
/// single atlas texture whose source rectangles fit in `u16::MAX`.
pub fn pack_prepared_glyph_token(x: u16, y: u16, w: u16, h: u16) -> PreparedGlyphToken {
    PreparedGlyphToken(((x as u64) << 48) | ((y as u64) << 32) | ((w as u64) << 16) | h as u64)
}

pub fn decode_prepared_glyph_token(token: PreparedGlyphToken) -> (u16, u16, u16, u16) {
    (
        (token.0 >> 48) as u16,
        (token.0 >> 32) as u16,
        (token.0 >> 16) as u16,
        token.0 as u16,
    )
}
