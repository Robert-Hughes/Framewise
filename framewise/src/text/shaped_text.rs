/// Backend-to-Framewise shaped text output.
///
/// This is a logical shaping result only. It contains no renderer resources and
/// no final line layout.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedText<G> {
    pub clusters: Vec<ShapedCluster<G>>,
}

/// One indivisible shaped cluster.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapedCluster<G> {
    pub byte_start: usize,
    pub byte_end: usize,
    pub advance: f32,
    pub is_whitespace: bool,
    pub glyphs: Vec<ShapedGlyph<G>>,
}

/// One shaped glyph inside a cluster.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapedGlyph<G> {
    pub id: G,
    /// Position relative to the shaped text run before Framewise line layout.
    pub x: f32,
    /// Position relative to the line baseline before Framewise line layout.
    pub y: f32,
    pub advance: f32,
}
