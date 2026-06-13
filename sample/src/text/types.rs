use framewise::FontId;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphRasterConfig {
    pub glyph_index: u16,
    pub px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlyphPosition {
    pub parent: char,
    pub key: GlyphRasterConfig,
    pub x: f32,
    pub y: f32,
    pub raster_w: usize, // raster bitmap width
    pub raster_h: usize,
    pub byte_offset: usize,
    pub subpixel_x: u8, // 0 = 0.0, 1 = 0.25, 2 = 0.50, 3 = 0.75
    pub advance: f32,   // shaped advance for proper text flow
    pub weight: u16,    // font weight used for this glyph
    pub opsz: u16,      // optical size used for this glyph
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    pub font_id: u16,
    pub glyph_index: u16,
    pub size: u32, // store size as u32 (size * 10.0 as u32) for hashing
    pub subpixel_x: u8,
    pub weight: u16, // 100-900, variable font weight axis
    pub opsz: u16,   // optical size axis value (typically matches size in pt)
}

pub struct AtlasRect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

pub struct GlyphInfo {
    pub atlas_rect: AtlasRect,
    pub left: i32,
    pub top: i32,
}

/// One indivisible shaped text unit used for layout and input mapping.
#[derive(Debug, Clone, PartialEq)]
pub struct TextCluster {
    /// Byte range of the original string represented by this cluster.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Range into the run's `glyphs` vec: `[glyph_start, glyph_end)`.
    pub glyph_start: usize,
    pub glyph_end: usize,
    /// Logical leading edge in block-local coordinates.
    pub x: f32,
    /// Logical advance used by wrapping, caret placement, and hit-testing.
    pub advance: f32,
    /// True for explicit hard line break clusters.
    pub is_hard_break: bool,
    /// True for Unicode whitespace clusters.
    pub is_whitespace: bool,
    /// True for a preserved whitespace cluster collapsed at a soft-wrap boundary.
    pub is_soft_wrap_boundary: bool,
}

/// One laid-out line within a prepared run, in block-local coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct LineRec {
    /// Top edge of the line.
    pub y_top: f32,
    /// Line height (advance to the next line).
    pub height: f32,
    /// Range into the run's `glyphs` vec: `[glyph_start, glyph_end)`.
    pub glyph_start: usize,
    pub glyph_end: usize,
    /// Range into the run's `clusters` vec: `[cluster_start, cluster_end)`.
    pub cluster_start: usize,
    pub cluster_end: usize,
    /// Byte range of the original string mapped to this line: `[byte_start, byte_end)`.
    pub byte_start: usize,
    pub byte_end: usize,
    pub logical_width: f32,
    pub ink_width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CachedLayout {
    pub font_id: FontId,
    pub glyphs: Vec<GlyphPosition>,
    pub clusters: Vec<TextCluster>,
    pub lines: Vec<LineRec>,
}
