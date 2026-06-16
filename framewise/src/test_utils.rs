use crate::text::{
    PrepareGlyphRequest, ShapedCluster, ShapedGlyph, ShapedText, TextBackend, TextLineAlign,
    TextMetrics, TextStyle,
};
use crate::{DrawGlyph, PreparedGlyphHandle};

/// A dummy text system for unit tests that provides representative text dimensions.
/// Assumes each character is 8px wide and 16px tall, supporting newlines for multi-line layout.
pub struct DummyTextSys {
    pub last_run: Option<(String, TextMetrics)>,
    pub last_rect_width: f32,
    pub last_line_align: TextLineAlign,
}

#[allow(non_upper_case_globals)]
pub const DummyTextSys: DummyTextSys = DummyTextSys {
    last_run: None,
    last_rect_width: 0.0,
    last_line_align: TextLineAlign::Start,
};

impl TextBackend for DummyTextSys {
    type ShapedGlyphId = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        16.0
    }

    fn shape_text(&mut self, text: &str, _style: TextStyle) -> ShapedText<Self::ShapedGlyphId> {
        let mut clusters = Vec::new();
        for (byte_start, ch) in text.char_indices() {
            let byte_end = byte_start + ch.len_utf8();
            let advance = 8.0;
            let is_whitespace = ch.is_whitespace();
            let glyphs = vec![ShapedGlyph {
                id: ch as u32,
                x: 0.0,
                y: 0.0,
                advance,
            }];
            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance,
                is_whitespace,
                glyphs,
            });
        }

        ShapedText { clusters }
    }

    fn shape_ellipsis(&mut self, _style: TextStyle) -> ShapedText<Self::ShapedGlyphId> {
        ShapedText {
            clusters: vec![ShapedCluster {
                byte_start: 0,
                byte_end: 0,
                advance: 8.0,
                is_whitespace: false,
                glyphs: vec![ShapedGlyph {
                    id: '.' as u32,
                    x: 0.0,
                    y: 0.0,
                    advance: 8.0,
                }],
            }],
        }
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph> {
        if request.glyph == ' ' as u32 {
            return None;
        }

        Some(DrawGlyph {
            handle: PreparedGlyphHandle(request.glyph),
            top_left: request.glyph_origin,
        })
    }
}
