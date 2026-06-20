use crate::text::{
    cluster_approx_ink_bounds, PrepareGlyphRequest, ShapedCluster, ShapedGlyph, SharedShapedText,
    TextBackend, TextLineLayoutMetrics, TextStyle,
};
use crate::{DrawGlyph, PreparedGlyphToken, Rect, Vec2};
use std::rc::Rc;

/// Deterministic text backend for unit tests.
///
/// Each visible character is one 8px cluster, text lines are 16px tall, and
/// whitespace contributes logical advance without producing drawable glyphs.
#[derive(Debug, Clone)]
pub struct TestTextBackend {
    config: TestTextBackendConfig,
    pub observations: TestTextBackendObservations,
}

#[derive(Debug, Clone)]
pub struct TestTextBackendConfig {
    pub line_height: f32,
    pub baseline_offset: Option<f32>,
    pub default_advance: f32,
    pub tab_advance: f32,
    pub char_advances: Vec<(char, f32)>,
    pub glyph_offset: Vec2,
    pub glyph_ink_bounds: TestGlyphInkBounds,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TestGlyphInkBounds {
    Logical,
    Fixed(Rect),
}

#[derive(Debug, Clone, Default)]
pub struct TestTextBackendObservations {
    pub shape_text_calls: usize,
    pub prepare_glyph_calls: usize,
    pub shaped_texts: Vec<String>,
    pub shaped_styles: Vec<TextStyle>,
    pub prepared_glyph_origins: Vec<Vec2>,
    pub prepared_glyph_rects: Vec<Rect>,
}

impl Default for TestTextBackend {
    fn default() -> Self {
        Self {
            config: TestTextBackendConfig::default(),
            observations: TestTextBackendObservations::default(),
        }
    }
}

impl Default for TestTextBackendConfig {
    fn default() -> Self {
        Self {
            line_height: 16.0,
            baseline_offset: None,
            default_advance: 8.0,
            tab_advance: 16.0,
            char_advances: Vec::new(),
            glyph_offset: Vec2::ZERO,
            glyph_ink_bounds: TestGlyphInkBounds::Logical,
        }
    }
}

impl TestTextBackend {
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.config.line_height = line_height;
        self
    }

    pub fn with_baseline_offset(mut self, baseline_offset: f32) -> Self {
        self.config.baseline_offset = Some(baseline_offset);
        self
    }

    pub fn with_default_advance(mut self, advance: f32) -> Self {
        self.config.default_advance = advance;
        self
    }

    pub fn with_tab_advance(mut self, advance: f32) -> Self {
        self.config.tab_advance = advance;
        self
    }

    pub fn with_char_advance(mut self, ch: char, advance: f32) -> Self {
        if let Some((_, existing)) = self
            .config
            .char_advances
            .iter_mut()
            .find(|(candidate, _)| *candidate == ch)
        {
            *existing = advance;
        } else {
            self.config.char_advances.push((ch, advance));
        }
        self
    }

    pub fn with_ellipsis_advance(self, advance: f32) -> Self {
        self.with_char_advance('…', advance)
    }

    pub fn with_glyph_offset(mut self, offset: Vec2) -> Self {
        self.config.glyph_offset = offset;
        self
    }

    pub fn with_glyph_ink_bounds(mut self, bounds: Rect) -> Self {
        self.config.glyph_ink_bounds = TestGlyphInkBounds::Fixed(bounds);
        self
    }

    pub fn reset_observations(&mut self) {
        self.observations = TestTextBackendObservations::default();
    }

    fn glyph_width(&self, ch: char) -> f32 {
        if let Some((_, advance)) = self
            .config
            .char_advances
            .iter()
            .find(|(candidate, _)| *candidate == ch)
        {
            return *advance;
        }

        match ch {
            '\u{0301}' | '\n' => 0.0,
            '\t' => self.config.tab_advance,
            _ => self.config.default_advance,
        }
    }

    fn glyph_ink_bounds(&self, style: TextStyle, advance: f32) -> Rect {
        match self.config.glyph_ink_bounds {
            TestGlyphInkBounds::Logical => {
                Rect::new(0.0, -style.size, advance, self.config.line_height)
            }
            TestGlyphInkBounds::Fixed(bounds) => bounds,
        }
    }
}

impl TextBackend for TestTextBackend {
    type ShapedGlyphToken = u32;

    fn line_metrics(&mut self, style: TextStyle) -> TextLineLayoutMetrics {
        TextLineLayoutMetrics {
            line_height: self.config.line_height.round().max(1.0) as u32,
            baseline_offset: self.config.baseline_offset.unwrap_or(style.size).round() as i32,
        }
    }

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        self.config.line_height
    }

    fn shape_text(
        &mut self,
        text: &str,
        style: TextStyle,
    ) -> SharedShapedText<Self::ShapedGlyphToken> {
        self.observations.shape_text_calls += 1;
        self.observations.shaped_texts.push(text.to_owned());
        self.observations.shaped_styles.push(style);

        let mut clusters: Vec<ShapedCluster<Self::ShapedGlyphToken>> = Vec::new();
        for (byte_start, ch) in text.char_indices() {
            let byte_end = byte_start + ch.len_utf8();
            let advance = self.glyph_width(ch);
            if ch == '\u{0301}' {
                if let Some(previous) = clusters.last_mut() {
                    previous.byte_end = byte_end;
                    previous.glyphs.push(ShapedGlyph {
                        token: ch as u32,
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
                    token: ch as u32,
                    x: self.config.glyph_offset.x,
                    y: self.config.glyph_offset.y,
                    advance,
                    approx_ink_bounds: self.glyph_ink_bounds(style, advance),
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
        self.observations.prepare_glyph_calls += 1;
        self.observations
            .prepared_glyph_origins
            .push(request.glyph_origin);
        self.observations.prepared_glyph_rects.push(Rect::new(
            request.glyph_origin.x,
            request.glyph_origin.y,
            char::from_u32(request.glyph)
                .map(|ch| self.glyph_width(ch))
                .unwrap_or(self.config.default_advance),
            self.config.line_height,
        ));

        if char::from_u32(request.glyph).is_some_and(char::is_whitespace) {
            return None;
        }

        Some(DrawGlyph {
            token: PreparedGlyphToken(request.glyph as u64),
            top_left: request.glyph_origin,
        })
    }
}
