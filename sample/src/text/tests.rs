#[cfg(test)]
mod tests {
    use crate::text::types::PreparedGlyphResources;
    use crate::text::SampleTextBackend;
    use framewise::{
        text::{layout_text, measure_text},
        Color, DrawCommands, DrawGlyph, FontId, LineHeight, PrepareGlyphRequest, Rect, TextBackend,
        TextBounds, TextFlow, TextStyle, Vec2,
    };

    fn sys() -> SampleTextBackend {
        SampleTextBackend::new()
    }

    fn style(font: FontId, size: f32, weight: u16, flow: TextFlow) -> TextStyle {
        TextStyle::new(font, size, weight, flow)
    }

    #[test]
    fn text_backend_shapes_ellipsis_and_prepares_glyphs() {
        let mut sys = sys();
        let style = style(FontId(1), 16.0, 500, TextFlow::single_line());

        let shaped = TextBackend::shape_text(&mut sys, "Hi", style);
        assert!(!shaped.clusters.is_empty());
        assert!(TextBackend::line_height(&mut sys, style) > 0.0);

        let ellipsis = TextBackend::shape_ellipsis(&mut sys, style);
        assert_eq!(ellipsis.clusters.len(), 1);

        let glyph = shaped.clusters[0].glyphs[0];
        let prepared = TextBackend::prepare_glyph(
            &mut sys,
            PrepareGlyphRequest {
                glyph: glyph.id,
                style,
                glyph_origin: Vec2::new(12.25 + glyph.x, 18.0 + glyph.y),
            },
        );
        assert!(prepared.is_some());
    }

    #[test]
    fn glyph_cache_keys_include_font_id() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        for font in [FontId(0), FontId(1)] {
            let style = style(font, 12.0, 400, TextFlow::single_line());
            let layout = layout_text(
                &mut sys,
                "A",
                style,
                TextBounds {
                    max_width: Some(rect.w),
                    max_height: Some(rect.h),
                },
            );
            layout.emit_glyphs(
                &mut framewise::DrawCommands::new(),
                &mut sys,
                rect.top_left(),
                style,
                framewise::Color::BLACK,
                0,
            );
        }

        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 0));
        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 1));
    }

    #[test]
    fn glyph_cache_keys_include_weight_and_opsz() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        for (text, size, weight) in [
            ("A", 12.0, 400),
            ("A", 12.0, 700),
            ("B", 14.0, 400),
            ("B", 32.0, 400),
        ] {
            let style = style(FontId(1), size, weight, TextFlow::single_line());
            let layout = layout_text(
                &mut sys,
                text,
                style,
                TextBounds {
                    max_width: Some(rect.w),
                    max_height: Some(rect.h),
                },
            );
            layout.emit_glyphs(
                &mut framewise::DrawCommands::new(),
                &mut sys,
                rect.top_left(),
                style,
                framewise::Color::BLACK,
                0,
            );
        }

        assert!(sys.glyph_cache.keys().any(|key| key.weight == 400));
        assert!(sys.glyph_cache.keys().any(|key| key.weight == 700));
        assert!(sys.glyph_cache.keys().any(|key| key.opsz == 14));
        assert!(sys.glyph_cache.keys().any(|key| key.opsz == 32));
    }

    #[test]
    fn weight_variation_affects_metrics() {
        let mut sys = sys();
        let text = "Framewise Font Variation Test";

        let regular_metrics = measure_text(
            &mut sys,
            text,
            style(FontId(1), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let bold_metrics = measure_text(
            &mut sys,
            text,
            style(FontId(1), 16.0, 700, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );

        assert!(
            bold_metrics.logical_size.x > regular_metrics.logical_size.x,
            "Bold width ({}) should be greater than regular width ({})",
            bold_metrics.logical_size.x,
            regular_metrics.logical_size.x
        );
    }

    #[test]
    fn font_without_opsz_uses_zero_opsz() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);
        let style = style(FontId(0), 12.0, 400, TextFlow::single_line());
        let layout = layout_text(
            &mut sys,
            "A",
            style,
            TextBounds {
                max_width: Some(rect.w),
                max_height: Some(rect.h),
            },
        );
        layout.emit_glyphs(
            &mut framewise::DrawCommands::new(),
            &mut sys,
            rect.top_left(),
            style,
            framewise::Color::BLACK,
            0,
        );

        let keys: Vec<_> = sys
            .glyph_cache
            .keys()
            .filter(|key| key.font_id == 0)
            .collect();
        assert!(!keys.is_empty());
        for key in keys {
            assert_eq!(key.opsz, 0);
        }
    }

    #[test]
    fn jetbrains_mono_weight_preserves_monospace_width() {
        let mut sys = sys();
        let text = "monospace_test_123456";

        let regular_metrics = measure_text(
            &mut sys,
            text,
            style(FontId(0), 14.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let bold_metrics = measure_text(
            &mut sys,
            text,
            style(FontId(0), 14.0, 700, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );

        assert_eq!(regular_metrics.logical_size.x, bold_metrics.logical_size.x);

        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);
        for weight in [400, 700] {
            let style = style(FontId(0), 14.0, weight, TextFlow::single_line());
            let layout = layout_text(
                &mut sys,
                "M",
                style,
                TextBounds {
                    max_width: Some(rect.w),
                    max_height: Some(rect.h),
                },
            );
            layout.emit_glyphs(
                &mut framewise::DrawCommands::new(),
                &mut sys,
                rect.top_left(),
                style,
                framewise::Color::BLACK,
                0,
            );
        }

        assert!(sys
            .glyph_cache
            .keys()
            .any(|key| key.font_id == 0 && key.weight == 400));
        assert!(sys
            .glyph_cache
            .keys()
            .any(|key| key.font_id == 0 && key.weight == 700));
    }

    #[test]
    fn letter_spacing_affects_width() {
        let mut sys = sys();

        let normal = measure_text(
            &mut sys,
            "spacing",
            style(FontId(1), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let spaced = measure_text(
            &mut sys,
            "spacing",
            style(FontId(1), 16.0, 400, TextFlow::single_line()).with_letter_spacing(0.1),
            TextBounds::UNBOUNDED,
        );

        assert!(spaced.logical_size.x > normal.logical_size.x);
    }

    #[test]
    fn relative_line_height_affects_layout() {
        let mut sys = sys();

        let normal = measure_text(
            &mut sys,
            "a\nb",
            style(FontId(1), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let compact = measure_text(
            &mut sys,
            "a\nb",
            style(FontId(1), 16.0, 400, TextFlow::single_line())
                .with_line_height(LineHeight::Relative(0.75)),
            TextBounds::UNBOUNDED,
        );

        assert!(compact.logical_size.y < normal.logical_size.y);
        assert_eq!(normal.line_count, compact.line_count);
    }

    #[test]
    fn subpixel_bins_match_final_glyph_positions() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::single_line());
        let layout = layout_text(&mut sys, "abc", style, TextBounds::UNBOUNDED);
        let mut commands = framewise::DrawCommands::new();
        let origin = Vec2::new(12.25, 0.0);

        layout.emit_glyphs(
            &mut commands,
            &mut sys,
            origin,
            style,
            framewise::Color::BLACK,
            0,
        );

        assert!(!commands.glyphs().is_empty());
        for (layout_glyph, draw_glyph) in layout.glyphs.iter().zip(commands.glyphs()) {
            let key = sys
                .prepared_glyph_keys
                .get(draw_glyph.handle.0 as usize)
                .expect("emitted glyph handle should resolve to a prepared key");
            let expected = subpixel_bin(origin.x + layout_glyph.origin.x);
            assert_eq!(key.subpixel_x, expected);
        }
    }

    fn subpixel_bin(abs_x: f32) -> u8 {
        ((abs_x * 4.0).round() as i32).rem_euclid(4) as u8
    }

    fn emit(
        sys: &mut SampleTextBackend,
        text: &str,
        style: TextStyle,
        bounds: TextBounds,
        origin: Vec2,
    ) -> (framewise::TextLayout<u16>, DrawCommands) {
        let layout = layout_text(sys, text, style, bounds);
        let mut commands = DrawCommands::new();
        layout.emit_glyphs(&mut commands, sys, origin, style, Color::BLACK, 0);
        (layout, commands)
    }

    fn union_rect(acc: Option<Rect>, rect: Rect) -> Option<Rect> {
        if rect.w <= 0.0 || rect.h <= 0.0 {
            return acc;
        }

        Some(match acc {
            Some(existing) => Rect::from_ltrb(
                existing.x.min(rect.x),
                existing.y.min(rect.y),
                existing.right().max(rect.right()),
                existing.bottom().max(rect.bottom()),
            ),
            None => rect,
        })
    }

    fn raster_ink_bounds_for_glyphs(
        glyphs: &[DrawGlyph],
        resources: &impl PreparedGlyphResources,
    ) -> Rect {
        glyphs
            .iter()
            .filter_map(|glyph| {
                let image = resources.resolve_glyph(glyph.handle)?;
                Some(Rect::new(
                    glyph.top_left.x,
                    glyph.top_left.y,
                    image.atlas_rect.w as f32,
                    image.atlas_rect.h as f32,
                ))
            })
            .fold(None, union_rect)
            .unwrap_or(Rect::ZERO)
    }

    #[test]
    fn shaped_combining_mark_records_one_cluster_with_multiple_glyphs() {
        let mut sys = sys();
        let shaped = TextBackend::shape_text(
            &mut sys,
            "e\u{0301}",
            style(FontId(1), 18.0, 400, TextFlow::single_line()),
        );

        assert_eq!(shaped.clusters.len(), 1);
        assert_eq!(shaped.clusters[0].byte_start, 0);
        assert_eq!(shaped.clusters[0].byte_end, "e\u{0301}".len());
        assert!(shaped.clusters[0].glyphs.len() >= 1);
    }

    #[test]
    fn vertical_snapping_verification() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::single_line());
        let (layout, commands) = emit(
            &mut sys,
            "Framewise",
            style,
            TextBounds::UNBOUNDED,
            Vec2::new(0.0, 3.2),
        );

        assert!(layout
            .glyphs
            .iter()
            .all(|glyph| glyph.origin.y.fract() == 0.0));
        assert!(commands
            .glyphs()
            .iter()
            .all(|glyph| glyph.top_left.y.fract() == 0.0));
    }

    #[test]
    fn subpixel_bin_mapping_verification() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::single_line());

        for (x, expected) in [(10.0, 0), (10.25, 1), (10.5, 2), (10.75, 3)] {
            let shaped = TextBackend::shape_text(&mut sys, "A", style);
            let glyph = shaped.clusters[0].glyphs[0];
            let draw = TextBackend::prepare_glyph(
                &mut sys,
                PrepareGlyphRequest {
                    glyph: glyph.id,
                    style,
                    glyph_origin: Vec2::new(x, 20.0),
                },
            )
            .expect("A should prepare");
            let key = sys
                .prepared_glyph_keys
                .get(draw.handle.0 as usize)
                .expect("prepared handle should resolve");
            assert_eq!(key.subpixel_x, expected);
        }
    }

    #[test]
    fn subpixel_bins_match_final_glyph_positions_for_body_copy() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::single_line());
        let origin = Vec2::new(12.25, 4.0);
        let (layout, commands) = emit(&mut sys, "Body copy", style, TextBounds::UNBOUNDED, origin);

        let mut layout_glyphs = layout.glyphs.iter();
        for draw_glyph in commands.glyphs() {
            let key = sys.prepared_glyph_keys[draw_glyph.handle.0 as usize];
            let layout_glyph = layout_glyphs
                .by_ref()
                .find(|glyph| glyph.id == key.glyph_index)
                .expect("emitted glyph should come from layout glyphs");
            assert_eq!(
                key.subpixel_x,
                subpixel_bin(origin.x + layout_glyph.origin.x)
            );
        }
    }

    #[test]
    fn subpixel_bins_match_final_glyph_positions_after_wrapping() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::wrapped());
        let origin = Vec2::new(3.25, 7.0);
        let (layout, commands) = emit(
            &mut sys,
            "Wrapped body copy for subpixel checks",
            style,
            TextBounds::width(90.0),
            origin,
        );

        let mut layout_glyphs = layout.glyphs.iter();
        for draw_glyph in commands.glyphs() {
            let key = sys.prepared_glyph_keys[draw_glyph.handle.0 as usize];
            let layout_glyph = layout_glyphs
                .by_ref()
                .find(|glyph| glyph.id == key.glyph_index)
                .expect("emitted glyph should come from layout glyphs");
            assert_eq!(
                key.subpixel_x,
                subpixel_bin(origin.x + layout_glyph.origin.x)
            );
        }
    }

    #[test]
    fn first_line_glyph_ink_stays_inside_text_rect_top() {
        let mut sys = sys();
        let style = style(FontId(1), 15.0, 400, TextFlow::wrapped());
        let origin = Vec2::new(0.0, 20.0);
        let (_layout, commands) = emit(
            &mut sys,
            "Sharp text",
            style,
            TextBounds::width(200.0),
            origin,
        );

        let first = commands.glyphs().first().expect("text should emit glyphs");
        assert!(first.top_left.y >= origin.y);
    }

    #[test]
    fn metrics_introspection_scaling() {
        let mut sys = sys();
        let small = measure_text(
            &mut sys,
            "Framewise",
            style(FontId(1), 12.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let large = measure_text(
            &mut sys,
            "Framewise",
            style(FontId(1), 24.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );

        assert!(large.logical_size.x > small.logical_size.x);
        assert!(large.logical_size.y > small.logical_size.y);
    }

    #[test]
    fn overflow_ellipsis_uses_same_vertical_position_as_shaped_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: framewise::OverflowX::Ellipsis {
                fallback: framewise::EllipsisFallback::Drop,
            },
            overflow_y: framewise::OverflowY::Keep,
            line_align: framewise::TextLineAlign::Start,
        };
        let style = style(FontId(1), 16.0, 400, flow);
        let layout = layout_text(&mut sys, "abcdef", style, TextBounds::width(40.0));

        let last = layout.glyphs.last().expect("ellipsis glyph should exist");
        let first = layout.glyphs.first().expect("text glyph should exist");
        assert_eq!(last.origin.y, first.origin.y);
    }

    #[test]
    fn measure_text_reports_sample_approx_ink_bounds() {
        let mut sys = sys();
        let style = style(FontId(1), 18.0, 400, TextFlow::single_line());
        let metrics = measure_text(&mut sys, "Ink", style, TextBounds::UNBOUNDED);

        assert!(metrics.approx_ink_bounds.w > 0.0);
        assert!(metrics.approx_ink_bounds.h > 0.0);
        assert!(metrics.approx_ink_bounds.w <= metrics.logical_size.x);
    }

    #[test]
    fn emitted_raster_bounds_are_derived_from_draw_glyphs_and_resources() {
        let mut sys = sys();
        let style = style(FontId(1), 18.0, 400, TextFlow::single_line());
        let (layout, commands) = emit(&mut sys, "Ink", style, TextBounds::UNBOUNDED, Vec2::ZERO);

        assert!(!commands.glyphs().is_empty());
        assert!(layout.metrics().approx_ink_bounds.w > 0.0);
        assert!(layout.metrics().approx_ink_bounds.h > 0.0);

        let raster_bounds = raster_ink_bounds_for_glyphs(commands.glyphs(), &sys);
        assert!(raster_bounds.w > 0.0);
        assert!(raster_bounds.h > 0.0);
    }

    #[test]
    fn test_letter_spacing_affects_width() {
        letter_spacing_affects_width();
    }

    #[test]
    fn test_relative_line_height_affects_layout() {
        relative_line_height_affects_layout();
    }
}
