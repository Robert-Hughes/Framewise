#[cfg(test)]
mod tests {
    use crate::text::{GlyphKey, SampleTextSystem};
    use framewise::{
        EllipsisFallback, FontId, LineHeight, OverflowX, OverflowY, Rect, TextBounds, TextFlow,
        TextHandle, TextLineAlign, TextStyle, TextSystem, Vec2, WrapClusterFallback,
        WrapWordFallback,
    };
    use swash::{shape::ShapeContext, FontRef};

    fn sys() -> SampleTextSystem {
        SampleTextSystem::new()
    }

    fn visible(sys: &SampleTextSystem, h: TextHandle) -> String {
        sys.runs[h.0].glyphs.iter().map(|g| g.parent).collect()
    }

    fn logical_glyph_end(g: &crate::text::types::GlyphPosition) -> f32 {
        g.x + g.advance
    }

    #[test]
    fn glyph_cache_keys_include_font_id() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        let _ = sys.prepare(
            "A",
            TextStyle::new(FontId(0), 12.0, 400, TextFlow::single_line()),
            rect,
        );
        let _ = sys.prepare(
            "A",
            TextStyle::new(FontId(1), 12.0, 400, TextFlow::single_line()),
            rect,
        );

        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 0));
        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 1));
    }

    #[test]
    fn glyph_cache_keys_include_weight_and_opsz() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        // 1. Check weight variations
        let _ = sys.prepare(
            "A",
            TextStyle::new(FontId(1), 12.0, 400, TextFlow::single_line()),
            rect,
        );
        let _ = sys.prepare(
            "A",
            TextStyle::new(FontId(1), 12.0, 700, TextFlow::single_line()),
            rect,
        );

        assert!(sys.glyph_cache.keys().any(|key| key.weight == 400));
        assert!(sys.glyph_cache.keys().any(|key| key.weight == 700));

        // 2. Check optical size variations
        // Preparing with size 14.0 -> opsz = 14
        let _ = sys.prepare(
            "B",
            TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        // Preparing with size 32.0 -> opsz = 32
        let _ = sys.prepare(
            "B",
            TextStyle::new(FontId(1), 32.0, 400, TextFlow::single_line()),
            rect,
        );

        assert!(sys.glyph_cache.keys().any(|key| key.opsz == 14));
        assert!(sys.glyph_cache.keys().any(|key| key.opsz == 32));
    }

    #[test]
    fn weight_variation_affects_metrics() {
        let mut sys = sys();
        let text = "Framewise Font Variation Test";

        let regular_metrics = sys.measure(
            text,
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let bold_metrics = sys.measure(
            text,
            TextStyle::new(FontId(1), 16.0, 700, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );

        // Bold text should be wider than regular text for Inter variable font
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
        let _ = sys.prepare(
            "A",
            TextStyle::new(FontId(0), 12.0, 400, TextFlow::single_line()),
            rect,
        );

        // FontId(0) (JetBrainsMono) has no opsz range, so opsz should be 0 in the glyph cache
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

        let regular_metrics = sys.measure(
            text,
            TextStyle::new(FontId(0), 14.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        let bold_metrics = sys.measure(
            text,
            TextStyle::new(FontId(0), 14.0, 700, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );

        // Monospace width should remain identical despite weight variation
        assert_eq!(
            regular_metrics.logical_size.x, bold_metrics.logical_size.x,
            "Monospace width should remain identical: regular = {}, bold = {}",
            regular_metrics.logical_size.x, bold_metrics.logical_size.x
        );

        // However, they should produce separate cached glyph entries in the atlas
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);
        let _ = sys.prepare(
            "M",
            TextStyle::new(FontId(0), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        let _ = sys.prepare(
            "M",
            TextStyle::new(FontId(0), 14.0, 700, TextFlow::single_line()),
            rect,
        );

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
    fn layout_cache_hits_and_eviction() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        // 1. Initial state: cache is empty
        assert_eq!(sys.layout_cache.len(), 0);

        // 2. Prepare first layout -> cache miss
        let layout1 = sys.prepare(
            "Cache Test",
            TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        assert_eq!(sys.layout_cache.len(), 1);

        // 3. Prepare identical layout -> cache hit
        let layout2 = sys.prepare(
            "Cache Test",
            TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        assert_eq!(sys.layout_cache.len(), 1); // Length did not change
        assert_eq!(layout1.metrics.logical_size, layout2.metrics.logical_size);

        // 4. Prepare with different weight -> cache miss (distinct entry)
        let _ = sys.prepare(
            "Cache Test",
            TextStyle::new(FontId(1), 14.0, 700, TextFlow::single_line()),
            rect,
        );
        assert_eq!(sys.layout_cache.len(), 2);

        // 5. Prepare with different bounds -> cache miss (distinct entry)
        let rect2 = Rect::new(0.0, 0.0, 100.0, 40.0);
        let _ = sys.prepare(
            "Cache Test",
            TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect2,
        );
        assert_eq!(sys.layout_cache.len(), 3);

        // 6. Test eviction (preventing unbounded growth)
        // Let's populate the cache up to the limit of 2000
        for i in 0..2005 {
            let unique_text = format!("text_{i}");
            let _ = sys.prepare(
                &unique_text,
                TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
                rect,
            );
        }
        // Since the limit is 2000, the cache should clear itself when it reaches 2000.
        // After inserting 2005 unique items, the cache should have cleared at least once,
        // and its size should be low (specifically, 6: 1 from the last insert after clear, plus any subsequent ones).
        assert!(sys.layout_cache.len() < 2000);
        assert!(!sys.layout_cache.is_empty());
    }

    #[test]
    fn single_line_is_one_line() {
        let mut sys = sys();
        let m = sys.measure(
            "hello world",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 1);
        assert!(!m.truncated_horizontal && !m.truncated_vertical);
        assert!(m.logical_size.x > 0.0);
    }

    #[test]
    fn hard_breaks_make_lines_without_wrap() {
        let mut sys = sys();
        let m = sys.measure(
            "a\nb\nc",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn test_sample_text_system_line_metrics() {
        let mut sys = sys();
        let m = sys.measure(
            "hello\nworld",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 2);
        assert_eq!(m.lines.len(), 2);

        let l0 = m.lines[0];
        let l1 = m.lines[1];

        assert_eq!(l0.byte_start, 0);
        assert_eq!(l0.byte_end, 6); // inclusive of '\n'
        assert_eq!(l1.byte_start, 6);
        assert_eq!(l1.byte_end, 11);

        assert!(l0.height > 0.0);
        assert_eq!(l0.y_top, 0.0);
        assert_eq!(l1.y_top, l0.height);
    }

    #[test]
    fn test_line_metrics_logical_and_ink_widths() {
        let mut sys = sys();

        // 1. Test Case: Hard-wrapping (explicit '\n')
        let m_hard = sys.measure(
            "hello\nworld",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m_hard.line_count, 2);
        for line in &m_hard.lines {
            assert!(line.logical_width > 0.0, "logical_width should be positive");
            assert!(line.ink_width > 0.0, "ink_width should be positive");
            assert!(
                line.ink_width <= line.logical_width + 10.0,
                "ink_width ({}) should be close to or less than logical_width ({})",
                line.ink_width,
                line.logical_width
            );
        }

        // 2. Test Case: Soft-wrapping (word wrap under narrow boundaries)
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let m_soft = sys.measure(
            "hello wrapping world",
            TextStyle::new(FontId(0), 16.0, 400, flow),
            TextBounds {
                max_width: Some(120.0),
                max_height: None,
            },
        );
        assert!(
            m_soft.line_count > 1,
            "Should have soft-wrapped into multiple lines"
        );
        for line in &m_soft.lines {
            assert!(
                line.logical_width > 0.0,
                "logical_width should be positive for soft wrap"
            );
            assert!(
                line.ink_width > 0.0,
                "ink_width should be positive for soft wrap"
            );
            assert!(
                line.logical_width <= 120.0,
                "logical_width ({}) should respect max_width limit (120.0)",
                line.logical_width
            );
        }
    }

    #[test]
    fn test_word_wrap_preserves_spaces() {
        let mut sys = sys();
        let flow = TextFlow::wrapped();
        let layout = sys.prepare(
            "hello there",
            TextStyle::new(FontId(1), 16.0, 400, flow),
            Rect::new(0.0, 0.0, 500.0, 100.0),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there");
    }

    #[test]
    fn wrapping_splits_a_long_line() {
        let mut sys = sys();
        let unwrapped = sys.measure(
            "the quick brown fox jumps over the lazy dog",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::wrapped()),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(unwrapped.line_count, 1);

        let wrapped = sys.measure(
            "the quick brown fox jumps over the lazy dog",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::wrapped()),
            TextBounds::width(80.0),
        );
        assert!(wrapped.line_count > 1);
        assert!(wrapped.logical_size.x <= 80.0 + 0.5);
    }

    #[test]
    fn prepare_with_measured_logical_bounds_preserves_metrics() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(2), 24.0, 600, TextFlow::wrapped())
            .with_letter_spacing(-0.035)
            .with_line_height(LineHeight::Relative(0.95));
        let text = "Button Demo";

        let measured = sys.measure(text, style, TextBounds::UNBOUNDED);
        let prepared = sys.prepare(
            text,
            style,
            Rect::new(0.0, 0.0, measured.logical_size.x, measured.logical_size.y),
        );

        assert_eq!(prepared.metrics.logical_size, measured.logical_size);
        assert_eq!(prepared.metrics.line_count, measured.line_count);
        assert_eq!(
            prepared.metrics.truncated_horizontal,
            measured.truncated_horizontal
        );
        assert_eq!(
            prepared.metrics.truncated_vertical,
            measured.truncated_vertical
        );
    }

    #[test]
    fn vertical_overflow_truncates_lines() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1), LineHeight::Normal);
        let m = sys.measure(
            "the quick brown fox jumps over the lazy dog again and again",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::wrapped()),
            TextBounds {
                max_width: Some(80.0),
                max_height: Some(lh * 2.0 + 1.0),
            },
        );
        assert_eq!(m.line_count, 2);
        assert!(m.truncated_vertical);
    }

    #[test]
    fn single_line_overflow_truncates_horizontally() {
        let mut sys = sys();
        let m = sys.measure(
            "hello world this is a long line",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::single_line()),
            TextBounds {
                max_width: Some(40.0),
                max_height: Some(100.0),
            },
        );
        assert_eq!(m.line_count, 1);
        assert!(m.truncated_horizontal);
        assert!(m.logical_size.x <= 40.0 + 0.5);
    }

    #[test]
    fn caret_advances_along_single_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 40.0),
        );
        let c0 = sys.caret_geom(layout.handle, 0);
        let c3 = sys.caret_geom(layout.handle, 3);
        assert!(c3.x > c0.x);
        assert_eq!(c0.y_top, c3.y_top);
    }

    #[test]
    fn hit_test_round_trips_to_a_boundary() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 40.0),
        );
        let far = sys.hit_test_caret(layout.handle, Vec2::new(1000.0, 5.0));
        assert_eq!(far, 3);
        let near = sys.hit_test_caret(layout.handle, Vec2::new(-5.0, 5.0));
        assert_eq!(near, 0);
    }

    #[test]
    fn wrap_cluster_keep_does_not_split_combining_mark_cluster() {
        let mut sys = sys();
        let text = "x\u{301}"; // x + COMBINING ACUTE ACCENT
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), 32.0, 400, flow),
            Rect::new(0.0, 0.0, 1.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];

        assert!(
            run.glyphs.len() >= 2,
            "test sample must shape to a multi-glyph cluster to exercise cluster wrapping"
        );
        assert!(
            run.glyphs.iter().all(|g| g.byte_offset == 0),
            "all glyphs in the combining-mark sample should map to the same shaping cluster"
        );

        assert_eq!(
            run.lines.len(),
            1,
            "fallback Keep should keep the whole overflowing cluster on one line"
        );
        assert_eq!(
            run.lines[0].glyph_end - run.lines[0].glyph_start,
            run.glyphs.len(),
            "a wrapping unit must not split base glyphs from combining marks"
        );
    }

    #[test]
    fn word_wrap_breaks_after_tab_whitespace() {
        let mut sys = sys();
        let text = "hello\tthere";
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(0), 16.0, 400, flow),
            Rect::new(0.0, 0.0, 52.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];

        assert_eq!(
            run.lines.len(),
            2,
            "Unicode whitespace such as tab should create a word wrapping opportunity"
        );
        assert_eq!(visible(&sys, layout.handle), text);
    }

    #[test]
    fn hit_test_cannot_target_a_line_made_from_half_a_cluster() {
        let mut sys = sys();
        let text = "x\u{301}"; // x + COMBINING ACUTE ACCENT
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), 32.0, 400, flow),
            Rect::new(0.0, 0.0, 1.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];

        assert!(
            run.glyphs.len() >= 2 && run.glyphs.iter().all(|g| g.byte_offset == 0),
            "test sample must shape to a multi-glyph cluster to exercise cluster hit testing"
        );
        assert_eq!(
            run.lines.len(),
            1,
            "hit testing should never see a visual line containing only part of one indivisible cluster"
        );
    }

    #[test]
    fn shaped_combining_mark_records_one_cluster_with_multiple_glyphs() {
        let mut sys = sys();
        let text = "x\u{301}y";
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), 32.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 300.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        let cluster = run
            .clusters
            .iter()
            .find(|cluster| cluster.byte_start == 0)
            .expect("first source cluster should be recorded");

        assert_eq!(cluster.byte_end, 3);
        assert!(
            cluster.glyph_end - cluster.glyph_start >= 2,
            "combining-mark cluster should keep its base and mark glyphs together"
        );
        assert_eq!(run.clusters.len(), 2);
    }

    #[test]
    fn line_records_include_cluster_ranges() {
        let mut sys = sys();
        let layout = sys.prepare(
            "ab\ncd",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 80.0),
        );
        let run = &sys.runs[layout.handle.0];

        assert_eq!(run.lines.len(), 2);
        for line in &run.lines {
            assert!(line.cluster_start <= line.cluster_end);
            assert!(line.cluster_end <= run.clusters.len());
            assert!(line.glyph_start <= line.glyph_end);
            assert!(line.glyph_end <= run.glyphs.len());
        }
        assert_eq!(
            run.lines[0].cluster_end - run.lines[0].cluster_start,
            3,
            "first line should include a, b, and the hard-break cluster"
        );
        assert_eq!(run.lines[1].cluster_end - run.lines[1].cluster_start, 2);
    }

    #[test]
    fn caret_inside_combining_mark_cluster_clamps_to_cluster_start() {
        let mut sys = sys();
        let text = "x\u{301}y";
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), 32.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 300.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        let first_cluster_x = run.clusters[0].x;

        assert_eq!(sys.caret_geom(layout.handle, 1).x, first_cluster_x);
        assert_eq!(sys.caret_geom(layout.handle, 2).x, first_cluster_x);
    }

    #[test]
    fn hit_test_combining_mark_cluster_returns_cluster_boundaries() {
        let mut sys = sys();
        let text = "x\u{301}y";
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), 32.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 300.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        let cluster = &run.clusters[0];
        let y = run.lines[0].y_top + 1.0;

        assert_eq!(
            sys.hit_test_caret(
                layout.handle,
                Vec2::new(cluster.x + cluster.advance * 0.25, y)
            ),
            0
        );
        assert_eq!(
            sys.hit_test_caret(
                layout.handle,
                Vec2::new(cluster.x + cluster.advance * 0.75, y)
            ),
            cluster.byte_end
        );
    }

    #[test]
    fn vertical_snapping_verification() {
        let mut sys = sys();
        let rect = Rect::new(10.2, 20.7, 500.0, 200.0);
        let layout = sys.prepare(
            "First Line\nSecond Line\nThird Line",
            TextStyle::new(FontId(1), 14.5, 400, TextFlow::single_line()),
            rect,
        );

        let run = &sys.runs[layout.handle.0];

        // Assert that vertical positions are rounded in the local coordinate space
        for g in &run.glyphs {
            assert_eq!(
                g.y.fract(),
                0.0,
                "Glyph vertical coordinate must be snapped to integer: y={}",
                g.y
            );
        }

        // Assert line records are snapped
        for line in &run.lines {
            assert_eq!(
                line.y_top.fract(),
                0.0,
                "Line y_top must be snapped to integer: y_top={}",
                line.y_top
            );
            assert_eq!(
                line.height.fract(),
                0.0,
                "Line height must be snapped to integer: height={}",
                line.height
            );
        }
    }

    #[test]
    fn subpixel_bin_mapping_verification() {
        let mut sys = sys();

        // Test varying starting X coordinates (absolute placements) and verify correct subpixel key mapping:
        // x = 10.0  -> 0 (0.0)
        // x = 10.15 -> 1 (0.25)
        // x = 10.45 -> 2 (0.50)
        // x = 10.85 -> 3 (0.75)
        let test_cases = [(10.0, 0), (10.15, 1), (10.45, 2), (10.85, 3)];

        for (abs_x, expected_bin) in test_cases {
            let rect = Rect::new(abs_x, 20.0, 200.0, 40.0);
            let layout = sys.prepare(
                "A",
                TextStyle::new(FontId(1), 12.0, 400, TextFlow::single_line()),
                rect,
            );
            let run = &sys.runs[layout.handle.0];
            let g = &run.glyphs[0];
            assert_eq!(
                g.subpixel_x, expected_bin,
                "Expected X coordinate {} to map to bin {}, got {}",
                abs_x, expected_bin, g.subpixel_x
            );
        }
    }

    #[test]
    fn subpixel_bins_match_final_glyph_positions_for_body_copy() {
        let mut sys = sys();
        let rect = Rect::new(173.0, 232.0, 600.0, 80.0);
        let layout = sys.prepare(
            "Sharp corners, hairline borders, monospaced numerics. One accent — rust — reserved for focus, drag, and primary action. Every widget describes its state explicitly; nothing is hidden behind animation or chrome.",
            TextStyle::new(FontId(1), 15.0, 400, TextFlow::wrapped())
                .with_line_height(LineHeight::Relative(1.55)),
            rect,
        );
        let run = &sys.runs[layout.handle.0];

        for g in &run.glyphs {
            if g.parent == ' ' || g.parent == '\n' {
                continue;
            }

            let expected = subpixel_bin(rect.x + g.x);
            assert_eq!(
                g.subpixel_x, expected,
                "glyph {:?} at final local x={} absolute x={} stored stale subpixel bin {}, expected {}",
                g.parent,
                g.x,
                rect.x + g.x,
                g.subpixel_x,
                expected,
            );
        }
    }

    #[test]
    fn subpixel_bins_match_final_glyph_positions_after_wrapping() {
        let mut sys = sys();
        let rect = Rect::new(10.15, 20.0, 72.0, 120.0);
        let layout = sys.prepare(
            "hello there hello there",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::wrapped()),
            rect,
        );
        let run = &sys.runs[layout.handle.0];

        assert!(
            run.lines.len() > 1,
            "test should exercise wrapped line relocation"
        );

        for g in &run.glyphs {
            if g.parent == ' ' || g.parent == '\n' {
                continue;
            }

            let expected = subpixel_bin(rect.x + g.x);
            assert_eq!(
                g.subpixel_x, expected,
                "glyph {:?} at final local x={} absolute x={} stored stale subpixel bin {}, expected {}",
                g.parent,
                g.x,
                rect.x + g.x,
                g.subpixel_x,
                expected,
            );
        }
    }

    fn subpixel_bin(abs_x: f32) -> u8 {
        (abs_x.fract() * 4.0).round() as u8 % 4
    }

    #[test]
    fn first_line_glyph_ink_stays_inside_text_rect_top() {
        let mut sys = sys();
        let rect = Rect::new(10.0, 15.0, 180.0, 30.0);
        let layout = sys.prepare(
            "Headless Test.",
            TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        let run = &sys.runs[layout.handle.0];

        let min_relative_top = run
            .glyphs
            .iter()
            .filter_map(|g| {
                let key = GlyphKey {
                    font_id: run.font_id.0,
                    glyph_index: g.key.glyph_index,
                    size: (g.key.px * 10.0) as u32,
                    subpixel_x: g.subpixel_x,
                    weight: g.weight,
                    opsz: g.opsz,
                };
                let info = sys.glyph_cache.get(&key)?;
                (info.atlas_rect.h > 0).then_some(g.y - info.top as f32)
            })
            .fold(f32::INFINITY, f32::min);

        assert!(
            min_relative_top >= 0.0,
            "first line ink should start within the text rect, got relative top {min_relative_top}"
        );
    }

    #[test]
    fn metrics_report_ink_bounds_separately_from_logical_size() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 13.0, 500, TextFlow::single_line());
        let layout = sys.prepare("◎", style, Rect::new(0.0, 0.0, 28.0, 28.0));
        let expected_advance = shaped_advance("◎", 13.0, FontId(0), 500).round();

        assert!(
            (layout.metrics.logical_size.x - expected_advance).abs() < 0.5,
            "logical size should follow shaped advance {expected_advance}, got {:?}",
            layout.metrics.logical_size
        );
        assert!(layout.metrics.ink_bounds.w > 0.0);
        assert!(
            layout.metrics.ink_bounds.x < 0.0,
            "JetBrains Mono ◎ has a negative side bearing, ink bounds should expose it: {:?}",
            layout.metrics.ink_bounds
        );
        assert!(
            layout.metrics.ink_bounds.x + layout.metrics.ink_bounds.w
                > layout.metrics.logical_size.x,
            "ink can protrude outside the logical advance box: {:?}, logical {:?}",
            layout.metrics.ink_bounds,
            layout.metrics.logical_size
        );
    }

    #[test]
    fn metrics_report_whitespace_logical_advance_without_ink() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(1), 13.0, 400, TextFlow::single_line());
        let metrics = sys.measure("   ", style, TextBounds::UNBOUNDED);
        let expected_advance = shaped_advance("   ", 13.0, FontId(1), 400).round();

        assert!(
            (metrics.logical_size.x - expected_advance).abs() < 0.5,
            "whitespace should contribute shaped advance {expected_advance}, got {:?}",
            metrics.logical_size
        );
        assert_eq!(
            metrics.ink_bounds,
            Rect::new(0.0, 0.0, 0.0, 0.0),
            "whitespace has no visible ink"
        );
    }

    #[test]
    fn drop_overflow_uses_logical_advance_not_ink_width_for_single_glyph() {
        let mut sys = sys();
        let text = "◎";
        let size = 13.0;
        let style = TextStyle::new(FontId(0), size, 500, TextFlow::single_line());
        let width = shaped_advance(text, size, FontId(0), 500).round();
        let layout = sys.prepare(text, style, Rect::new(0.0, 0.0, width, 28.0));

        assert_eq!(visible(&sys, layout.handle), text);
        assert!(
            !layout.metrics.truncated_horizontal,
            "ink protrusion outside the logical advance should not count as truncation"
        );
        assert!(
            layout.metrics.ink_bounds.x + layout.metrics.ink_bounds.w
                > layout.metrics.logical_size.x,
            "test glyph should visibly protrude past its logical box"
        );
    }

    #[test]
    fn drop_overflow_uses_logical_advance_not_ink_width_for_final_glyph() {
        let mut sys = sys();
        let text = "Run ◎";
        let size = 13.0;
        let style = TextStyle::new(FontId(0), size, 500, TextFlow::single_line());
        let width = shaped_advance(text, size, FontId(0), 500).round();
        let layout = sys.prepare(text, style, Rect::new(0.0, 0.0, width, 28.0));

        assert_eq!(visible(&sys, layout.handle), text);
        assert!(
            !layout.metrics.truncated_horizontal,
            "final glyph ink protrusion should not drop the last character"
        );
    }

    #[test]
    fn caret_end_uses_shaped_advance_not_bitmap_width() {
        let mut sys = sys();
        let text = "Headless Test.";
        let size = 14.0;
        let layout = sys.prepare(
            text,
            TextStyle::new(FontId(1), size, 400, TextFlow::single_line()),
            Rect::new(10.0, 15.0, 180.0, 30.0),
        );

        let expected_advance = shaped_advance(text, size, FontId(1), 400);
        let caret = sys.caret_geom(layout.handle, text.len());

        assert!(
            (caret.x - expected_advance).abs() < 0.25,
            "caret end x should follow shaped advance {expected_advance}, got {}",
            caret.x
        );
    }

    fn shaped_advance(text: &str, size: f32, font_id: FontId, weight: u16) -> f32 {
        let data = match font_id.0 {
            0 => include_bytes!("../../assets/JetBrains_Mono/JetBrainsMono-VariableFont_wght.ttf")
                as &[u8],
            1 => include_bytes!("../../assets/Inter/Inter-VariableFont_opsz,wght.ttf") as &[u8],
            _ => panic!("unsupported test font id {}", font_id.0),
        };
        let font = FontRef::from_index(data, 0).expect("test font should load");
        let mut shape_context = ShapeContext::new();
        let mut shaper = shape_context.builder(font).size(size);
        if font_id.0 == 0 {
            shaper = shaper.variations(&[("wght", weight as f32)]);
        } else if font_id.0 == 1 {
            let opsz = size.clamp(14.0, 32.0);
            shaper = shaper.variations(&[("wght", weight as f32), ("opsz", opsz)]);
        }
        let mut shaper = shaper.build();
        shaper.add_str(text);

        let mut advance = 0.0;
        shaper.shape_with(|cluster| {
            for glyph in cluster.glyphs {
                advance += glyph.advance;
            }
        });
        advance
    }

    #[test]
    fn metrics_introspection_scaling() {
        let sys = sys();
        let font_id = FontId(1);
        let h1 = sys.line_height(10.0, font_id, LineHeight::Normal);
        let h2 = sys.line_height(20.0, font_id, LineHeight::Normal);

        // Assert that line height scales roughly linearly with font size
        assert!(
            (h2 - h1 * 2.0).abs() < 1.0,
            "Line height should scale linearly: h1={}, h2={}",
            h1,
            h2
        );
    }

    fn logical_run_width(sys: &SampleTextSystem, h: TextHandle) -> f32 {
        sys.runs[h.0]
            .glyphs
            .iter()
            .map(logical_glyph_end)
            .fold(0.0, f32::max)
    }

    #[test]
    fn ellipsis_is_appended_on_single_line_overflow() {
        let mut sys = sys();
        let layout = sys.prepare(
            "hello world this is long",
            TextStyle::new(
                FontId(1),
                16.0,
                400,
                TextFlow {
                    overflow_x: OverflowX::Ellipsis {
                        fallback: EllipsisFallback::Drop,
                    },
                    overflow_y: OverflowY::Drop,
                    line_align: TextLineAlign::Start,
                },
            ),
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(
            text.ends_with('…'),
            "expected trailing ellipsis, got {text:?}"
        );
    }

    #[test]
    fn overflow_ellipsis_uses_same_vertical_position_as_shaped_ellipsis() {
        let mut sys = sys();
        let style = TextStyle::new(
            FontId(1),
            16.0,
            400,
            TextFlow {
                overflow_x: OverflowX::Ellipsis {
                    fallback: EllipsisFallback::Drop,
                },
                overflow_y: OverflowY::Drop,
                line_align: TextLineAlign::Start,
            },
        );

        let direct = sys.prepare("hello…", style, Rect::new(0.0, 0.0, 200.0, 30.0));
        let direct_y = sys.runs[direct.handle.0]
            .glyphs
            .iter()
            .find(|g| g.parent == '…')
            .expect("direct text should contain an ellipsis")
            .y;

        let overflow = sys.prepare(
            "hello world this is long",
            style,
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        let overflow_y = sys.runs[overflow.handle.0]
            .glyphs
            .iter()
            .find(|g| g.parent == '…')
            .expect("overflow text should contain an ellipsis")
            .y;

        assert!(
            (overflow_y - direct_y).abs() < 0.5,
            "overflow ellipsis y {overflow_y} should match shaped ellipsis y {direct_y}",
        );
    }

    #[test]
    fn ellipsis_on_last_line_when_height_clipped() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1), LineHeight::Normal);
        let layout = sys.prepare(
            "the quick brown fox jumps over the lazy dog and then keeps going",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::wrapped()),
            Rect::new(0.0, 0.0, 80.0, lh * 2.0 + 1.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert!(
            text.contains('…'),
            "expected an ellipsis somewhere, got {text:?}"
        );
    }

    #[test]
    fn center_align_centers_a_fitting_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "hi",
            TextStyle::new(
                FontId(1),
                16.0,
                400,
                TextFlow {
                    overflow_x: OverflowX::Drop,
                    overflow_y: OverflowY::Drop,
                    line_align: TextLineAlign::Center,
                },
            ),
            Rect::new(0.0, 0.0, 200.0, 30.0),
        );
        let first_x = sys.runs[layout.handle.0].glyphs[0].x;
        assert!(
            first_x > 50.0,
            "short line should be pushed right when centered, x={first_x}"
        );
    }

    #[test]
    fn caret_on_second_line_is_offset_in_y() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc\ndef",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let c_line2 = sys.caret_geom(layout.handle, 4);
        assert!(
            c_line2.y_top > 1.0,
            "second-line caret should sit below the first"
        );
    }

    #[test]
    fn long_unbreakable_word_is_force_broken() {
        let mut sys = sys();
        let layout = sys.prepare(
            "supercalifragilisticexpialidocious",
            TextStyle::new(
                FontId(1),
                16.0,
                400,
                TextFlow {
                    overflow_x: OverflowX::WrapWord {
                        fallback: WrapWordFallback::WrapCluster {
                            fallback: WrapClusterFallback::Drop,
                        },
                    },
                    overflow_y: OverflowY::Drop,
                    line_align: TextLineAlign::Start,
                },
            ),
            Rect::new(0.0, 0.0, 40.0, 200.0),
        );
        let lines = sys.runs[layout.handle.0].lines.len();
        assert!(
            lines > 1,
            "expected the long word to break across lines, got {lines}"
        );
    }

    #[test]
    fn metrics_width_matches_logical_run_width_after_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Drop,
            line_align: TextLineAlign::Start,
        };
        let rect = Rect::new(0.0, 0.0, 50.0, 30.0);
        let layout = sys.prepare(
            "hello world this is long",
            TextStyle::new(FontId(1), 16.0, 400, flow),
            rect,
        );
        let reported = layout.metrics.logical_size.x;
        let actual = logical_run_width(&sys, layout.handle);
        assert!(
            (reported - actual).abs() < 1.0,
            "metrics width {reported} should match logical run width {actual}",
        );
    }

    #[test]
    fn center_align_keeps_overflowing_line_within_box() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 40.0, 30.0);
        let layout = sys.prepare(
            "hello world this is long",
            TextStyle::new(
                FontId(1),
                16.0,
                400,
                TextFlow {
                    overflow_x: OverflowX::Ellipsis {
                        fallback: EllipsisFallback::Drop,
                    },
                    overflow_y: OverflowY::Drop,
                    line_align: TextLineAlign::Center,
                },
            ),
            rect,
        );
        let left = sys.runs[layout.handle.0]
            .glyphs
            .iter()
            .map(|g| g.x)
            .fold(f32::INFINITY, f32::min);
        assert!(
            left >= -0.5,
            "centered overflow line starts off-box at x={left}"
        );
    }

    #[test]
    fn multiline_hit_test_picks_the_right_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc\ndef",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0), LineHeight::Normal);
        let on_line2 = sys.hit_test_caret(layout.handle, Vec2::new(0.0, lh + lh * 0.5));
        assert_eq!(on_line2, 4);
    }

    // ── Newline-character semantics ──────────────────────────────────────────
    //
    // The following tests assert the contract stated in the TextSystem trait
    // doc-comments.  Some are expected to fail until the implementations are
    // brought in line with the spec.

    /// Clicking far to the right of a `\n`-terminated line must keep the caret
    /// on *that* line, not jump to the start of the next line.
    ///
    /// Contract (from `TextSystem::hit_test`):
    ///   "If the line ends with a hard newline, the result must be the index of
    ///    the `\n` character itself (i.e. `byte_end - 1`), not `byte_end`."
    #[test]
    fn hit_test_right_of_newline_line_stays_on_same_line() {
        let mut sys = sys();
        // "abc\ndef" — line 0 occupies bytes 0..4 (includes the '\n' at byte 3).
        let layout = sys.prepare(
            "abc\ndef",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0), LineHeight::Normal);
        // Click well to the right of "abc" on line 0, still within line 0's Y band.
        let result = sys.hit_test_caret(layout.handle, Vec2::new(1000.0, lh * 0.5));
        // Must land on the '\n' at byte 3, not on byte 4 (start of "def").
        assert_eq!(
            result, 3,
            "hit_test far right of a newline-terminated line should return the \
             index of the '\\n' (byte 3), not byte_end (byte 4)"
        );
    }

    /// Clicking to the right of a blank line (a line that contains only `\n`)
    /// must keep the caret on that blank line, not jump to the next line.
    #[test]
    fn hit_test_right_of_blank_newline_line_stays_on_same_line() {
        let mut sys = sys();
        // "a\n\nb" — line 1 is a blank line whose only content is '\n' at byte 2.
        let layout = sys.prepare(
            "a\n\nb",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0), LineHeight::Normal);
        // Click anywhere on the blank line (line index 1, Y ≈ lh..2*lh).
        let result = sys.hit_test_caret(layout.handle, Vec2::new(100.0, lh + lh * 0.5));
        // Byte 2 is the '\n' that forms the blank line; byte 3 would be the
        // start of the last line ("b").
        assert_eq!(
            result, 2,
            "hit_test on a blank newline-only line should return the index of \
             the '\\n' (byte 2), not byte_end (byte 3)"
        );
    }

    #[test]
    fn test_hit_test_cluster_edge_cases() {
        let mut sys = sys();
        // Text: "a b\nc"
        // Line 0: "a b\n" (bytes 0..4, containing 'a' at 0, ' ' at 1, 'b' at 2, '\n' at 3)
        // Line 1: "c" (byte 4)
        let layout = sys.prepare(
            "a b\nc",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0), LineHeight::Normal);

        // Get the widths of the characters on Line 0 to determine coordinates
        let caret0 = sys.caret_geom(layout.handle, 0); // x of 'a' left
        let caret1 = sys.caret_geom(layout.handle, 1); // x of 'a' right / ' ' left
        let caret2 = sys.caret_geom(layout.handle, 2); // x of ' ' right / 'b' left
        let caret3 = sys.caret_geom(layout.handle, 3); // x of 'b' right / '\n' left

        let y0 = lh * 0.5; // Line 0
        let y1 = lh + lh * 0.5; // Line 1

        // 1. Click on the left/right parts of 'a'
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret0.x + 1.0, y0)),
            0
        );
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret1.x - 1.0, y0)),
            0
        );

        // 2. Click on the left/right parts of ' '
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret1.x + 1.0, y0)),
            1
        );
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret2.x - 1.0, y0)),
            1
        );

        // 3. Click on the left/right parts of 'b'
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret2.x + 1.0, y0)),
            2
        );
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret3.x - 1.0, y0)),
            2
        );

        // 4. Click far to the right of Line 0 (should return '\n' index)
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret3.x + 100.0, y0)),
            3
        );

        // 5. Click on the '\n' itself (caret3.x)
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret3.x + 0.1, y0)),
            3
        );

        // 6. Click far to the left of Line 0
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(-100.0, y0)),
            0
        );

        // 7. Click on Line 1 ('c')
        let caret4 = sys.caret_geom(layout.handle, 4); // 'c' left
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret4.x + 1.0, y1)),
            4
        );
        // Click far to the right of Line 1 (should return 'c' index since there is no '\n')
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(caret4.x + 100.0, y1)),
            4
        );
    }

    /// `caret_geom` at the index of the `\n` character must return a caret on
    /// the *same* line as the preceding text, not the next line.
    #[test]
    fn caret_geom_at_newline_index_stays_on_same_line() {
        let mut sys = sys();
        // "abc\ndef" — '\n' is at byte 3, on line 0.
        let layout = sys.prepare(
            "abc\ndef",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        let line0_y_top = run.lines[0].y_top;
        let line0_height = run.lines[0].height;

        let caret = sys.caret_geom(layout.handle, 3); // index of '\n'
        assert_eq!(
            caret.y_top, line0_y_top,
            "caret at the '\\n' index (byte 3) must sit on line 0 (y_top={line0_y_top}), \
             not line 1"
        );
        assert_eq!(
            caret.height, line0_height,
            "caret height at the '\\n' index must match line 0 height"
        );
    }

    /// `caret_geom` at `byte_end` of a newline-terminated line (i.e. the index
    /// immediately *after* the `\n`) must return a caret on the *next* line.
    #[test]
    fn caret_geom_after_newline_index_is_on_next_line() {
        let mut sys = sys();
        // "abc\ndef" — byte_end of line 0 is 4 (one past the '\n'), which is
        // also byte_start of line 1.
        let layout = sys.prepare(
            "abc\ndef",
            TextStyle::new(FontId(0), 16.0, 400, TextFlow::single_line()),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        let line1_y_top = run.lines[1].y_top;
        let line1_height = run.lines[1].height;

        // byte 4 == byte_end of line 0 == byte_start of line 1.
        let caret = sys.caret_geom(layout.handle, 4);
        assert_eq!(
            caret.y_top, line1_y_top,
            "caret at byte 4 (byte_end of the '\\n' line) must sit on line 1 \
             (y_top={line1_y_top})"
        );
        assert_eq!(
            caret.height, line1_height,
            "caret height at byte 4 must match line 1 height"
        );
    }

    /// For soft-wrapped lines, test:
    /// - Querying `caret_geom` at the visual line's `byte_end` (which equals the `byte_start`
    ///   of the next visual line) resolves to the start of the next visual line.
    /// - To obtain the trailing edge coordinate of the soft-wrapped line, we should use
    ///   `line.logical_width` which represents the actual right edge.
    #[test]
    fn test_caret_geom_soft_wrap_boundaries() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        // "hello wrapping" will soft-wrap. "hello" (5 chars) on line 0, "wrapping" on line 1.
        let layout = sys.prepare(
            "hello wrapping",
            TextStyle::new(FontId(0), 16.0, 400, flow),
            Rect::new(0.0, 0.0, 80.0, 100.0),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);

        let l0 = &run.lines[0];
        let l1 = &run.lines[1];

        // l0.byte_end is the index of the space or the start of "wrapping" (index 6, since "hello " is 6 chars).
        let caret_at_boundary = sys.caret_geom(layout.handle, l0.byte_end);
        assert_eq!(
            caret_at_boundary.y_top, l1.y_top,
            "caret at soft-wrap boundary index ({}) must resolve to visual line 1",
            l0.byte_end
        );
        assert!(
            caret_at_boundary.x < 5.0,
            "caret at soft-wrap boundary index must be at the start of visual line 1 (x={})",
            caret_at_boundary.x
        );

        // Querying immediately before the soft-wrap boundary (i.e. l0.byte_end - 1).
        // It should stay on line 0.
        let caret_before_boundary = sys.caret_geom(layout.handle, l0.byte_end - 1);
        assert_eq!(
            caret_before_boundary.y_top, l0.y_top,
            "caret immediately before soft-wrap boundary must resolve to visual line 0"
        );

        // Verify logical_width represents the trailing edge coordinate of line 0.
        assert!(
            l0.logical_width > 0.0,
            "logical_width of line 0 should be positive"
        );
    }

    #[test]
    fn test_ink_bounds_match_rasterized_glyph_extents() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 500.0, 100.0);
        let layout = sys.prepare(
            "Hello World",
            TextStyle::new(FontId(1), 16.0, 400, TextFlow::single_line()),
            rect,
        );

        let run = &sys.runs[layout.handle.0];
        let mut ink_l = f32::INFINITY;
        let mut ink_r = f32::NEG_INFINITY;
        for g in &run.glyphs {
            let key = GlyphKey {
                font_id: run.font_id.0,
                glyph_index: g.key.glyph_index,
                size: (g.key.px * 10.0) as u32,
                subpixel_x: g.subpixel_x,
                weight: g.weight,
                opsz: g.opsz,
            };
            let info = sys.glyph_cache.get(&key).unwrap();
            if info.atlas_rect.w == 0 || info.atlas_rect.h == 0 {
                continue;
            }
            let l = g.x + info.left as f32;
            let r = l + info.atlas_rect.w as f32;
            ink_l = ink_l.min(l);
            ink_r = ink_r.max(r);
        }

        assert!(
            (layout.metrics.ink_bounds.x - ink_l).abs() < 0.001,
            "ink bounds x should match rasterized glyph extents"
        );
        assert!(
            (layout.metrics.ink_bounds.w - (ink_r - ink_l)).abs() < 0.001,
            "ink bounds width should match rasterized glyph extents"
        );

        let caret = sys.caret_geom(layout.handle, 0);
        assert_eq!(caret.x, 0.0, "Caret at index 0 must be at x = 0.0");

        let idx = sys.hit_test_caret(layout.handle, Vec2::new(0.0, 5.0));
        assert_eq!(idx, 0, "Hit testing near 0.0 must return index 0");
    }

    // ── Systematic unit tests ────────────────────────────────────────────────

    // Keep this test in sync with Card 1 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_drop_y_drop() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Drop,
            overflow_y: OverflowY::Drop,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 25.0, 28.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let run = &sys.runs[layout.handle.0];
        for g in &run.glyphs {
            assert!(logical_glyph_end(g) <= 25.0 + 0.1);
        }
        assert!(!run.glyphs.is_empty());
    }

    // Keep this test in sync with Card 2 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_keep_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 25.0, 28.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        let mut line1_has_overflow = false;
        let mut line2_has_overflow = false;
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if line_glyphs
                .iter()
                .any(|g| logical_glyph_end(g) > 25.0 + 0.1)
            {
                if i == 0 {
                    line1_has_overflow = true;
                }
                if i == 1 {
                    line2_has_overflow = true;
                }
            }
        }
        assert!(line1_has_overflow);
        assert!(line2_has_overflow);
    }

    // Keep this test in sync with Card 3 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_keep_y_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 25.0, 28.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let text = visible(&sys, layout.handle);
        assert!(text.ends_with('…'));
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(logical_glyph_end(last_glyph) <= 25.0 + 0.1);
    }

    // Keep this test in sync with Card 4 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_drop() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 8.0, 28.0),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    // Keep this test in sync with Card 5 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 8.0, 28.0),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "…");
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(logical_glyph_end(last_glyph) > 8.0 + 0.1);
    }

    // Keep this test in sync with Card 6 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_ellipsis_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 23.0, 48.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert!(text.contains('…'));
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(logical_glyph_end(last_g) <= 23.0 + 0.1);
        }
    }

    // Keep this test in sync with Card 7 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_ellipsis_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 8.0, 48.0),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    // Keep this test in sync with Card 8 in Section 4 of sample/src/label_page.rs
    #[test]
    fn test_overflow_x_ellipsis_fallback_keep_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 8.0, 48.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "……");
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(logical_glyph_end(last_g) > 8.0 + 0.1);
        }
        let y0 = run.glyphs[0].y;
        let y1 = run.glyphs[1].y;
        assert!(
            y1 > y0 + 10.0,
            "The second line's ellipsis must be positioned below the first: y0={}, y1={}",
            y0,
            y1
        );
    }

    // Keep this test in sync with Card 1 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_cluster_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 23.0, 63.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello\nhello");
    }

    // Keep this test in sync with Card 2 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_cluster_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 6.0, 68.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 3 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_cluster_fallback_keep_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 4.0, 162.0),
        );
        // Expect 10 lines: 5 lines for each "hello". The newline character '\n'
        // is appended to the end of the first "hello"'s last line (containing 'o'),
        // rather than starting a new blank line.
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 10);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello\nhello");
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            // Since '\n' is appended to the same line as 'o', that line will contain 2 glyphs.
            // Other lines will contain at most 1 glyph.
            assert!(line_glyphs.len() <= 2);
        }
    }

    // Keep this test in sync with Card 4 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 48.0, 68.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 5 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_cluster_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 23.0, 138.0),
        );
        assert!(sys.runs[layout.handle.0].lines.len() > 4);
        let text = visible(&sys, layout.handle);
        let run = &sys.runs[layout.handle.0];
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                println!(
                    "line {}, char={:?}, x={}, raster_w={}",
                    i, g.parent, g.x, g.raster_w
                );
            }
        }
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 6 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_cluster_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 6.0, 138.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 7 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_cluster_fallback_keep_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 4.0, 318.0),
        );
        let text = visible(&sys, layout.handle);
        // Note: The UI height (318.0) divided by line height (16.0) limits the
        // layout to max_lines = 19. The 20th line containing the final character 'e'
        // is truncated/dropped.
        assert_eq!(text, "hello there\nhello ther");
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 19);
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let visible_glyphs: Vec<_> = line_glyphs
                .iter()
                .filter(|g| g.parent != ' ' && g.parent != '\n')
                .collect();
            assert!(visible_glyphs.len() <= 1);
        }
    }

    // Keep this test in sync with Card 8 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 25.0, 68.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                assert!(logical_glyph_end(g) <= 25.0 + 0.1);
            }
        }
    }

    // Keep this test in sync with Card 9 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_keep_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            TextStyle::new(FontId(1), 14.0, 400, flow),
            Rect::new(0.0, 0.0, 25.0, 68.0),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);
        let mut has_overflow = false;
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if let Some(last_g) = line_glyphs.last() {
                if last_g.parent != '\n' && last_g.parent != ' ' && logical_glyph_end(last_g) > 25.0
                {
                    has_overflow = true;
                }
            }
        }
        assert!(has_overflow);
    }

    #[test]
    fn test_letter_spacing_affects_width() {
        let mut sys = sys();
        let text = "Hello World";
        let rect = Rect::new(0.0, 0.0, 500.0, 100.0);

        let style_normal = TextStyle::new(FontId(1), 16.0, 400, TextFlow::single_line());
        let layout_normal = sys.prepare(text, style_normal, rect);
        let normal_width = layout_normal.metrics.logical_size.x;

        // Positive spacing expands the width
        let style_expanded = style_normal.with_letter_spacing(0.1); // 0.1 em
        let layout_expanded = sys.prepare(text, style_expanded, rect);
        let expanded_width = layout_expanded.metrics.logical_size.x;
        assert!(
            expanded_width > normal_width,
            "Expanded width ({}) should be greater than normal width ({})",
            expanded_width,
            normal_width
        );

        // Negative spacing shrinks the width
        let style_condensed = style_normal.with_letter_spacing(-0.05); // -0.05 em
        let layout_condensed = sys.prepare(text, style_condensed, rect);
        let condensed_width = layout_condensed.metrics.logical_size.x;
        assert!(
            condensed_width < normal_width,
            "Condensed width ({}) should be less than normal width ({})",
            condensed_width,
            normal_width
        );
    }

    #[test]
    fn test_relative_line_height_affects_layout() {
        let mut sys = sys();
        let text = "Hello\nWorld";
        let rect = Rect::new(0.0, 0.0, 200.0, 200.0);

        // 1. Normal line height
        let style_normal = TextStyle::new(FontId(1), 16.0, 400, TextFlow::single_line());
        let layout_normal = sys.prepare(text, style_normal, rect);
        let height_normal = layout_normal.metrics.logical_size.y;
        let normal_lh = sys.line_height(16.0, FontId(1), LineHeight::Normal).round();
        assert_eq!(layout_normal.metrics.line_count, 2);
        assert!((height_normal - normal_lh * 2.0).abs() < 0.1);

        // 2. Relative line height (larger multiplier, e.g. 1.8)
        let style_large = style_normal.with_line_height(LineHeight::Relative(1.8));
        let layout_large = sys.prepare(text, style_large, rect);
        let height_large = layout_large.metrics.logical_size.y;
        let large_lh = sys
            .line_height(16.0, FontId(1), LineHeight::Relative(1.8))
            .round();
        assert_eq!(layout_large.metrics.line_count, 2);
        assert!((height_large - large_lh * 2.0).abs() < 0.1);
        assert!(height_large > height_normal);

        // 3. Relative line height (smaller multiplier, e.g. 0.8)
        let style_small = style_normal.with_line_height(LineHeight::Relative(0.8));
        let layout_small = sys.prepare(text, style_small, rect);
        let height_small = layout_small.metrics.logical_size.y;
        let small_lh = sys
            .line_height(16.0, FontId(1), LineHeight::Relative(0.8))
            .round();
        assert_eq!(layout_small.metrics.line_count, 2);
        assert!((height_small - small_lh * 2.0).abs() < 0.1);
        assert!(height_small < height_normal);

        // Verify caret y_top reflects line height change
        let caret_normal = sys.caret_geom(layout_normal.handle, 6); // start of "World"
        let caret_large = sys.caret_geom(layout_large.handle, 6);
        let caret_small = sys.caret_geom(layout_small.handle, 6);

        assert!((caret_normal.y_top - normal_lh).abs() < 0.1);
        assert!((caret_large.y_top - large_lh).abs() < 0.1);
        assert!((caret_small.y_top - small_lh).abs() < 0.1);
    }

    #[test]
    fn test_newline_wrapping_collapse() {
        let mut sys = sys();
        let flow_word = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let flow_cluster = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };

        let rect = Rect::new(0.0, 0.0, 200.0, 200.0);
        let style_word = TextStyle::new(FontId(1), 16.0, 400, flow_word);
        let style_cluster = TextStyle::new(FontId(1), 16.0, 400, flow_cluster);

        // 1. Double newlines (WrapWord)
        let layout = sys.prepare("hello\n\nworld", style_word, rect);
        assert_eq!(
            layout.metrics.line_count, 3,
            "Double newlines should produce 3 lines under WrapWord"
        );

        // 2. Trailing newline (WrapWord)
        let layout = sys.prepare("hello\n", style_word, rect);
        assert_eq!(
            layout.metrics.line_count, 2,
            "Trailing newline should produce 2 lines under WrapWord"
        );

        // 3. Multiple trailing newlines (WrapWord)
        let layout = sys.prepare("hello\n\n", style_word, rect);
        assert_eq!(
            layout.metrics.line_count, 3,
            "Multiple trailing newlines should produce 3 lines under WrapWord"
        );

        // 4. Double newlines (WrapCluster)
        let layout = sys.prepare("hello\n\nworld", style_cluster, rect);
        assert_eq!(
            layout.metrics.line_count, 3,
            "Double newlines should produce 3 lines under WrapCluster"
        );

        // 5. Trailing newline (WrapCluster)
        let layout = sys.prepare("hello\n", style_cluster, rect);
        assert_eq!(
            layout.metrics.line_count, 2,
            "Trailing newline should produce 2 lines under WrapCluster"
        );
    }

    fn wrap_word_preserving_whitespace_flow() -> TextFlow {
        TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        }
    }

    fn wrap_cluster_preserving_whitespace_flow() -> TextFlow {
        TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        }
    }

    fn line_width(sys: &mut SampleTextSystem, text: &str, style: TextStyle) -> f32 {
        sys.measure(text, style, TextBounds::UNBOUNDED).lines[0].logical_width
    }

    fn assert_close(actual: f32, expected: f32, label: &str) {
        assert!(
            (actual - expected).abs() < 0.5,
            "{label}: expected {expected}, got {actual}"
        );
    }

    #[test]
    fn soft_wrap_boundary_space_collapses_between_words() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let hello_w = line_width(&mut sys, "hello", style);

        let layout = sys.prepare(
            "hello world",
            style,
            Rect::new(0.0, 0.0, hello_w + 0.1, 200.0),
        );
        assert_eq!(layout.metrics.line_count, 2);
        assert_eq!(visible(&sys, layout.handle), "hello world");

        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (6, 11));
        assert_close(run.lines[0].logical_width, hello_w, "collapsed line width");
        assert_close(
            layout.metrics.lines[0].logical_width,
            hello_w,
            "collapsed line metrics width",
        );
    }

    #[test]
    fn collapsed_soft_wrap_space_has_newline_like_caret_and_hit_test_behavior() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let hello_w = line_width(&mut sys, "hello", style);
        let layout = sys.prepare(
            "hello world",
            style,
            Rect::new(0.0, 0.0, hello_w + 0.1, 200.0),
        );
        let run = &sys.runs[layout.handle.0];

        let caret_at_space = sys.caret_geom(layout.handle, 5);
        assert_eq!(caret_at_space.y_top, run.lines[0].y_top);
        assert_close(caret_at_space.x, hello_w, "caret at collapsed space");

        let caret_after_space = sys.caret_geom(layout.handle, 6);
        assert_eq!(caret_after_space.y_top, run.lines[1].y_top);
        assert_close(caret_after_space.x, 0.0, "caret after collapsed space");

        let line0_mid_y = run.lines[0].y_top + run.lines[0].height * 0.5;
        assert_eq!(
            sys.hit_test_caret(layout.handle, Vec2::new(1000.0, line0_mid_y)),
            5,
            "hit-testing past a collapsed soft-wrap boundary space should stay on that space"
        );
        assert_eq!(
            sys.hit_test_cluster(layout.handle, Vec2::new(1000.0, line0_mid_y)),
            5,
            "cluster hit-testing past a collapsed soft-wrap boundary space should return it"
        );
    }

    #[test]
    fn only_the_single_boundary_space_collapses() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let hello_w = line_width(&mut sys, "hello", style);
        let space_w = line_width(&mut sys, " ", style);

        let layout = sys.prepare(
            "hello  world",
            style,
            Rect::new(0.0, 0.0, hello_w + 0.1, 200.0),
        );
        assert_eq!(layout.metrics.line_count, 3);
        assert_eq!(visible(&sys, layout.handle), "hello  world");

        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (6, 7));
        assert_eq!((run.lines[2].byte_start, run.lines[2].byte_end), (7, 12));
        assert_close(run.lines[0].logical_width, hello_w, "first line width");
        assert_close(run.lines[1].logical_width, space_w, "second line width");
    }

    #[test]
    fn trailing_boundary_space_collapses_without_creating_a_blank_line() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let hello_w = line_width(&mut sys, "hello", style);

        let layout = sys.prepare("hello ", style, Rect::new(0.0, 0.0, hello_w + 0.1, 200.0));
        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), "hello ");
        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_close(
            run.lines[0].logical_width,
            hello_w,
            "trailing collapsed width",
        );
    }

    #[test]
    fn trailing_spaces_after_the_boundary_space_are_preserved() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let hello_w = line_width(&mut sys, "hello", style);
        let space_w = line_width(&mut sys, " ", style);

        let layout = sys.prepare("hello  ", style, Rect::new(0.0, 0.0, hello_w + 0.1, 200.0));
        assert_eq!(layout.metrics.line_count, 2);
        assert_eq!(visible(&sys, layout.handle), "hello  ");
        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (6, 7));
        assert_close(run.lines[0].logical_width, hello_w, "first line width");
        assert_close(run.lines[1].logical_width, space_w, "second line width");
    }

    #[test]
    fn leading_spaces_are_preserved_and_only_overflowing_spaces_collapse() {
        let mut sys = sys();
        let style = TextStyle::new(FontId(0), 16.0, 400, wrap_word_preserving_whitespace_flow());
        let five_spaces_w = line_width(&mut sys, "     ", style);

        let layout = sys.prepare(
            "     hello",
            style,
            Rect::new(0.0, 0.0, five_spaces_w + 0.1, 200.0),
        );
        assert_eq!(layout.metrics.line_count, 2);
        assert_eq!(visible(&sys, layout.handle), "     hello");
        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 5));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (5, 10));
        assert_close(
            run.lines[0].logical_width,
            five_spaces_w,
            "five spaces width",
        );

        let layout = sys.prepare(
            "      hello",
            style,
            Rect::new(0.0, 0.0, five_spaces_w + 0.1, 200.0),
        );
        assert_eq!(layout.metrics.line_count, 2);
        assert_eq!(visible(&sys, layout.handle), "      hello");
        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (6, 11));
        assert_close(
            run.lines[0].logical_width,
            five_spaces_w,
            "sixth leading space collapses at boundary",
        );
    }

    #[test]
    fn wrap_cluster_collapses_soft_wrap_boundary_space() {
        let mut sys = sys();
        let style = TextStyle::new(
            FontId(0),
            16.0,
            400,
            wrap_cluster_preserving_whitespace_flow(),
        );
        let hello_w = line_width(&mut sys, "hello", style);

        let layout = sys.prepare(
            "hello world",
            style,
            Rect::new(0.0, 0.0, hello_w + 0.1, 200.0),
        );
        assert_eq!(layout.metrics.line_count, 2);
        assert_eq!(visible(&sys, layout.handle), "hello world");

        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (6, 11));
        assert_close(
            run.lines[0].logical_width,
            hello_w,
            "WrapCluster collapsed line width",
        );
    }

    #[test]
    fn wrap_cluster_trailing_boundary_space_does_not_create_blank_line() {
        let mut sys = sys();
        let style = TextStyle::new(
            FontId(0),
            16.0,
            400,
            wrap_cluster_preserving_whitespace_flow(),
        );
        let hello_w = line_width(&mut sys, "hello", style);

        let layout = sys.prepare("hello ", style, Rect::new(0.0, 0.0, hello_w + 0.1, 200.0));
        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), "hello ");
        let run = &sys.runs[layout.handle.0];
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 6));
        assert_close(
            run.lines[0].logical_width,
            hello_w,
            "WrapCluster trailing collapsed width",
        );
    }

    #[test]
    fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_drop_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);

        let layout = sys.prepare(" ", style, Rect::new(0.0, 0.0, 1.0, 200.0));

        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), "");
        assert_eq!(layout.metrics.lines[0].logical_width, 0.0);
    }

    #[test]
    fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_keep_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapCluster {
                    fallback: WrapClusterFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);
        let space_w = line_width(&mut sys, " ", style);

        let layout = sys.prepare(" ", style, Rect::new(0.0, 0.0, 1.0, 200.0));

        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), " ");
        assert_close(
            layout.metrics.lines[0].logical_width,
            space_w,
            "kept overwide whitespace width",
        );
    }

    #[test]
    fn overwide_whitespace_on_empty_line_uses_wrap_word_drop_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);

        let layout = sys.prepare(" ", style, Rect::new(0.0, 0.0, 1.0, 200.0));

        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), "");
        assert_eq!(layout.metrics.lines[0].logical_width, 0.0);
    }

    #[test]
    fn overwide_whitespace_on_empty_line_uses_wrap_word_keep_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);
        let space_w = line_width(&mut sys, " ", style);

        let layout = sys.prepare(" ", style, Rect::new(0.0, 0.0, 1.0, 200.0));

        assert_eq!(layout.metrics.line_count, 1);
        assert_eq!(visible(&sys, layout.handle), " ");
        assert_close(
            layout.metrics.lines[0].logical_width,
            space_w,
            "kept overwide whitespace width",
        );
    }

    #[test]
    fn wrap_word_keep_moves_overlong_word_to_new_line_before_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);
        let prefix_w = line_width(&mut sys, "ok", style);

        let layout = sys.prepare(
            "ok abcdef",
            style,
            Rect::new(0.0, 0.0, prefix_w + 0.1, 200.0),
        );

        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 3));
        assert_eq!((run.lines[1].byte_start, run.lines[1].byte_end), (3, 9));

        let line1_first = &run.clusters[run.lines[1].cluster_start];
        assert_eq!(line1_first.byte_start, 3);
        assert_close(line1_first.x, 0.0, "overlong word starts new line");
    }

    #[test]
    fn wrap_cluster_keep_moves_overwide_cluster_to_new_line_before_fallback() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapCluster {
                fallback: WrapClusterFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            line_align: TextLineAlign::Start,
        };
        let style = TextStyle::new(FontId(0), 16.0, 400, flow);
        let prefix_w = line_width(&mut sys, "ok", style);

        let layout = sys.prepare("ok ◎", style, Rect::new(0.0, 0.0, prefix_w + 0.1, 200.0));

        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);
        assert_eq!((run.lines[0].byte_start, run.lines[0].byte_end), (0, 3));
        assert_eq!(
            (run.lines[1].byte_start, run.lines[1].byte_end),
            (3, "ok ◎".len())
        );

        let line1_first = &run.clusters[run.lines[1].cluster_start];
        assert_eq!(line1_first.byte_start, 3);
        assert_close(line1_first.x, 0.0, "overwide cluster starts new line");
    }
}
