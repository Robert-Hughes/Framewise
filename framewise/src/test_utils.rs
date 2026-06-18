use crate::text::{
    cluster_approx_ink_bounds, PrepareGlyphRequest, ShapedCluster, ShapedGlyph, SharedShapedText,
    TextBackend, TextStyle,
};
use crate::{DrawGlyph, PreparedGlyphHandle};
use std::rc::Rc;

/// Deterministic text backend for unit tests.
///
/// Each visible character is one 8px cluster, text lines are 16px tall, and
/// whitespace contributes logical advance without producing drawable glyphs.
#[derive(Debug, Clone, Copy, Default)]
pub struct TestTextBackend;

impl TestTextBackend {
    fn glyph_width(ch: char) -> f32 {
        match ch {
            '\u{0301}' | '\n' => 0.0,
            '\t' => 16.0,
            _ => 8.0,
        }
    }
}

impl TextBackend for TestTextBackend {
    type ShapedGlyphToken = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        16.0
    }

    fn shape_text(
        &mut self,
        text: &str,
        style: TextStyle,
    ) -> SharedShapedText<Self::ShapedGlyphToken> {
        let mut clusters: Vec<ShapedCluster<Self::ShapedGlyphToken>> = Vec::new();
        for (byte_start, ch) in text.char_indices() {
            let byte_end = byte_start + ch.len_utf8();
            let advance = Self::glyph_width(ch);
            if ch == '\u{0301}' {
                if let Some(previous) = clusters.last_mut() {
                    previous.byte_end = byte_end;
                    previous.glyphs.push(ShapedGlyph {
                        id: ch as u32,
                        x: 0.0,
                        y: -4.0,
                        advance: 0.0,
                        approx_ink_bounds: crate::Rect::new(0.0, -4.0, 8.0, 4.0),
                    });
                    previous.approx_ink_bounds = cluster_approx_ink_bounds(&previous.glyphs);
                    continue;
                }
            }
            let is_whitespace = ch.is_whitespace();
            let glyphs = if is_whitespace {
                Vec::new()
            } else {
                vec![ShapedGlyph {
                    id: ch as u32,
                    x: 0.0,
                    y: 0.0,
                    advance,
                    approx_ink_bounds: crate::Rect::new(0.0, -style.size, advance, 16.0),
                }]
            };
            let approx_ink_bounds = cluster_approx_ink_bounds(&glyphs);
            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance,
                is_whitespace,
                approx_ink_bounds,
                glyphs,
            });
        }

        Rc::new(crate::text::ShapedText { clusters })
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphToken>,
    ) -> Option<DrawGlyph> {
        if char::from_u32(request.glyph).is_some_and(char::is_whitespace) {
            return None;
        }

        Some(DrawGlyph {
            handle: PreparedGlyphHandle(request.glyph),
            top_left: request.glyph_origin,
        })
    }
}
