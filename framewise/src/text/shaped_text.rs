/// Backend-to-Framewise shaped text output.
///
/// This is a logical shaping result only. It contains no renderer resources and
/// no final line layout.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText<G> {
    /// Shaped clusters in source order.
    pub clusters: Vec<ShapedCluster<G>>,
}

/// One indivisible shaped cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedCluster<G> {
    /// Byte range of the original string represented by this cluster.
    pub byte_start: usize,
    pub byte_end: usize,
    /// Logical advance used by wrapping, caret placement, and hit-testing.
    pub advance: f32,
    /// True for Unicode whitespace clusters.
    pub is_whitespace: bool,
    /// Glyphs belonging to this cluster.
    pub glyphs: Vec<ShapedGlyph<G>>,
}

/// One shaped glyph inside a cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph<G> {
    /// Backend-shaped glyph identifier.
    pub id: G,
    /// Position relative to the cluster/glyph run before Framewise wrapping and
    /// final line placement.
    pub x: f32,
    /// Position relative to the line baseline before Framewise wrapping and
    /// final line placement.
    pub y: f32,
    /// Shaped advance used by text flow.
    pub advance: f32,
}
