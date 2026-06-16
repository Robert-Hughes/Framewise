use crate::text::SampleTextBackend;
use framewise::{
    FontId, LineHeight, Rect, ShapedCluster, ShapedGlyph, ShapedText, TextLineLayoutMetrics,
    TextStyle,
};
use swash::scale::Scaler;

fn outline_approx_ink_bounds(scaler: &mut Scaler<'_>, glyph_id: u16) -> Option<Rect> {
    let outline = scaler.scale_outline(glyph_id)?;
    let bounds = outline.bounds();
    if bounds.is_empty() {
        return Some(Rect::ZERO);
    }

    Some(Rect::new(
        bounds.min.x,
        -bounds.max.y,
        bounds.width(),
        bounds.height(),
    ))
}

impl SampleTextBackend {
    /// Get the optical size value for a given pixel size and font.
    /// Returns 0.0 if the font doesn't have opsz axis.
    pub fn opsz_for_size(&self, size: f32, font_id: FontId) -> f32 {
        let (min_opsz, max_opsz) = self.font_opsz_ranges[font_id.0 as usize];
        if max_opsz == 0.0 {
            // Font has no opsz axis
            return 0.0;
        }
        // Clamp size to the font's opsz range
        size.clamp(min_opsz, max_opsz)
    }

    pub fn line_height(&self, size: f32, font_id: FontId, line_height_style: LineHeight) -> f32 {
        self.line_layout_metrics(size, font_id, line_height_style)
            .line_height
    }

    pub fn line_layout_metrics(
        &self,
        size: f32,
        font_id: FontId,
        line_height_style: LineHeight,
    ) -> TextLineLayoutMetrics {
        let font = self.fonts[font_id.0 as usize];

        // For now, get metrics without variations - they should be similar enough
        // TODO: Consider if we need to normalize coords for metrics
        let metrics = font.metrics(&[]);
        let units_per_em = metrics.units_per_em as f32;
        let scale = size / units_per_em;
        let ascent = metrics.ascent * scale;
        let descent = (metrics.descent * scale).abs();
        let line_gap = metrics.leading * scale;

        let line_height = match line_height_style {
            LineHeight::Normal => ascent + descent + line_gap,
            LineHeight::Relative(mult) => size * mult,
        };

        TextLineLayoutMetrics {
            line_height,
            baseline_offset: ascent,
        }
    }
    pub fn shape_text_run(&mut self, text: &str, style: TextStyle) -> ShapedText<u16> {
        let font_id = style.font;
        let size = style.size;
        let weight = style.weight;
        let opsz = self.opsz_for_size(size, font_id);
        let letter_spacing_px = size * style.letter_spacing;
        let font = self.fonts[font_id.0 as usize];

        let mut vars = Vec::new();
        if self.font_has_wght[font_id.0 as usize] {
            vars.push(("wght", weight as f32));
        }
        if self.font_has_opsz[font_id.0 as usize] && opsz > 0.0 {
            vars.push(("opsz", opsz));
        }

        let mut shaper = self.shape_context.builder(font).size(size);
        let mut scaler = self.scale_context.builder(font).size(size).hint(false);
        if !vars.is_empty() {
            shaper = shaper.variations(&vars);
            scaler = scaler.variations(&vars);
        }

        let mut shaper = shaper.build();
        let mut scaler = scaler.build();
        shaper.add_str(text);

        let mut clusters = Vec::new();
        let mut pen_x = 0.0_f32;
        shaper.shape_with(|cluster| {
            let source = cluster.source.to_range();
            let source_text = &text[source.clone()];
            let byte_start = cluster.source.start as usize;
            let byte_end = cluster.source.end as usize;
            let is_whitespace = source_text.chars().all(char::is_whitespace);
            let cluster_x = pen_x;
            let mut cluster_advance = 0.0;
            let mut glyphs = Vec::new();

            for glyph in cluster.glyphs {
                let advance = glyph.advance + letter_spacing_px;
                glyphs.push(ShapedGlyph {
                    id: glyph.id,
                    x: pen_x - cluster_x + glyph.x,
                    y: glyph.y,
                    advance,
                    approx_ink_bounds: if is_whitespace {
                        Some(Rect::ZERO)
                    } else {
                        outline_approx_ink_bounds(&mut scaler, glyph.id)
                    },
                });
                pen_x += advance;
                cluster_advance += advance;
            }

            if glyphs.is_empty() {
                glyphs.push(ShapedGlyph {
                    id: 0,
                    x: 0.0,
                    y: 0.0,
                    advance: 0.0,
                    approx_ink_bounds: Some(Rect::ZERO),
                });
            }

            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance: cluster_advance,
                is_whitespace,
                glyphs,
            });
        });

        ShapedText { clusters }
    }
}
