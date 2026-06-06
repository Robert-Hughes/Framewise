#[cfg(test)]
mod tests {
    use crate::text::SampleTextSystem;
    use framewise::{
        EllipsisFallback, FontId, HorizontalAlign, OverflowX, OverflowY, Rect, TextBounds,
        TextFlow, TextHandle, TextSystem, Vec2, WrapGlyphFallback, WrapWordFallback,
    };

    fn sys() -> SampleTextSystem {
        SampleTextSystem::new()
    }

    fn visible(sys: &SampleTextSystem, h: TextHandle) -> String {
        sys.runs[h.0].glyphs.iter().map(|g| g.parent).collect()
    }

    #[test]
    fn glyph_cache_keys_include_font_id() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 200.0, 40.0);

        let _ = sys.prepare("A", 12.0, FontId(0), TextFlow::single_line(), rect);
        let _ = sys.prepare("A", 12.0, FontId(1), TextFlow::single_line(), rect);

        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 0));
        assert!(sys.glyph_cache.keys().any(|key| key.font_id == 1));
    }

    #[test]
    fn single_line_is_one_line() {
        let mut sys = sys();
        let m = sys.measure(
            "hello world",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 1);
        assert!(!m.truncated_horizontal && !m.truncated_vertical);
        assert!(m.size.x > 0.0);
    }

    #[test]
    fn hard_breaks_make_lines_without_wrap() {
        let mut sys = sys();
        let m = sys.measure(
            "a\nb\nc",
            16.0,
            FontId(0),
            TextFlow::single_line(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn test_word_wrap_preserves_spaces() {
        let mut sys = sys();
        let flow = TextFlow::wrapped();
        let layout = sys.prepare(
            "hello there",
            16.0,
            FontId(1),
            flow,
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
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            TextBounds::UNBOUNDED,
        );
        assert_eq!(unwrapped.line_count, 1);

        let wrapped = sys.measure(
            "the quick brown fox jumps over the lazy dog",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
            TextBounds::width(80.0),
        );
        assert!(wrapped.line_count > 1);
        assert!(wrapped.size.x <= 80.0 + 0.5);
    }

    #[test]
    fn vertical_overflow_truncates_lines() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let m = sys.measure(
            "the quick brown fox jumps over the lazy dog again and again",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
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
            16.0,
            FontId(1),
            TextFlow::single_line(),
            TextBounds {
                max_width: Some(40.0),
                max_height: Some(100.0),
            },
        );
        assert_eq!(m.line_count, 1);
        assert!(m.truncated_horizontal);
        assert!(m.size.x <= 40.0 + 0.5);
    }

    #[test]
    fn caret_advances_along_single_line() {
        let mut sys = sys();
        let layout = sys.prepare(
            "abc",
            16.0,
            FontId(0),
            TextFlow::single_line(),
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
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 40.0),
        );
        let far = sys.hit_test(layout.handle, Vec2::new(1000.0, 5.0));
        assert_eq!(far, 3);
        let near = sys.hit_test(layout.handle, Vec2::new(-5.0, 5.0));
        assert_eq!(near, 0);
    }

    #[test]
    fn vertical_snapping_verification() {
        let mut sys = sys();
        let rect = Rect::new(10.2, 20.7, 500.0, 200.0);
        let layout = sys.prepare(
            "First Line\nSecond Line\nThird Line",
            14.5,
            FontId(1),
            TextFlow::single_line(),
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
            let layout = sys.prepare("A", 12.0, FontId(1), TextFlow::single_line(), rect);
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
    fn metrics_introspection_scaling() {
        let sys = sys();
        let font_id = FontId(1);
        let h1 = sys.line_height(10.0, font_id);
        let h2 = sys.line_height(20.0, font_id);

        // Assert that line height scales roughly linearly with font size
        assert!(
            (h2 - h1 * 2.0).abs() < 1.0,
            "Line height should scale linearly: h1={}, h2={}",
            h1,
            h2
        );
    }

    fn rendered_width(sys: &SampleTextSystem, h: TextHandle) -> f32 {
        sys.runs[h.0]
            .glyphs
            .iter()
            .map(|g| g.x + g.width as f32)
            .fold(0.0, f32::max)
    }

    #[test]
    fn ellipsis_is_appended_on_single_line_overflow() {
        let mut sys = sys();
        let layout = sys.prepare(
            "hello world this is long",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Ellipsis {
                    fallback: EllipsisFallback::Drop,
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 30.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(
            text.ends_with('…'),
            "expected trailing ellipsis, got {text:?}"
        );
    }

    #[test]
    fn ellipsis_on_last_line_when_height_clipped() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let layout = sys.prepare(
            "the quick brown fox jumps over the lazy dog and then keeps going",
            16.0,
            FontId(1),
            TextFlow::wrapped(),
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
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Drop,
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Center,
            },
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
            16.0,
            FontId(0),
            TextFlow::single_line(),
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
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::WrapWord {
                    fallback: WrapWordFallback::WrapGlyph {
                        fallback: WrapGlyphFallback::Drop,
                    },
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Start,
            },
            Rect::new(0.0, 0.0, 40.0, 200.0),
        );
        let lines = sys.runs[layout.handle.0].lines.len();
        assert!(
            lines > 1,
            "expected the long word to break across lines, got {lines}"
        );
    }

    #[test]
    fn metrics_width_matches_rendered_width_after_ellipsis() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Drop,
            horizontal_align: HorizontalAlign::Start,
        };
        let rect = Rect::new(0.0, 0.0, 50.0, 30.0);
        let layout = sys.prepare("hello world this is long", 16.0, FontId(1), flow, rect);
        let reported = layout.metrics.size.x;
        let actual = rendered_width(&sys, layout.handle);
        assert!(
            (reported - actual).abs() < 1.0,
            "metrics width {reported} should match rendered width {actual}",
        );
    }

    #[test]
    fn center_align_keeps_overflowing_line_within_box() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 40.0, 30.0);
        let layout = sys.prepare(
            "hello world this is long",
            16.0,
            FontId(1),
            TextFlow {
                overflow_x: OverflowX::Ellipsis {
                    fallback: EllipsisFallback::Drop,
                },
                overflow_y: OverflowY::Drop,
                horizontal_align: HorizontalAlign::Center,
            },
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
            16.0,
            FontId(0),
            TextFlow::single_line(),
            Rect::new(0.0, 0.0, 200.0, 100.0),
        );
        let lh = sys.line_height(16.0, FontId(0));
        let on_line2 = sys.hit_test(layout.handle, Vec2::new(0.0, lh + lh * 0.5));
        assert_eq!(on_line2, 4);
    }

    #[test]
    fn test_optical_ink_bounds_alignment() {
        let mut sys = sys();
        let rect = Rect::new(0.0, 0.0, 500.0, 100.0);
        let layout = sys.prepare(
            "Hello World",
            16.0,
            FontId(1),
            TextFlow::single_line(),
            rect,
        );

        let run = &sys.runs[layout.handle.0];
        let l = run.glyphs.iter().map(|g| g.x).fold(f32::INFINITY, f32::min);
        let r = run
            .glyphs
            .iter()
            .map(|g| g.x + g.width as f32)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(l, 0.0, "Leftmost ink pixel must be at 0.0");
        assert!(
            (layout.metrics.size.x - r.round()).abs() < 0.001,
            "Metrics width must match tight ink width"
        );

        let caret = sys.caret_geom(layout.handle, 0);
        assert_eq!(caret.x, 0.0, "Caret at index 0 must be at x = 0.0");

        let idx = sys.hit_test(layout.handle, Vec2::new(0.0, 5.0));
        assert_eq!(idx, 0, "Hit testing near 0.0 must return index 0");
    }

    // ÔöÇÔöÇ Systematic unit tests ÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇÔöÇ

    #[test]
    fn test_overflow_x_drop_y_drop() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Drop,
            overflow_y: OverflowY::Drop,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let run = &sys.runs[layout.handle.0];
        for g in &run.glyphs {
            assert!(g.x + g.width as f32 <= 25.0 + 0.1);
        }
        assert!(!run.glyphs.is_empty());
    }

    #[test]
    fn test_overflow_x_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        let mut line1_has_overflow = false;
        let mut line2_has_overflow = false;
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if line_glyphs
                .iter()
                .any(|g| g.x + g.width as f32 > 25.0 + 0.1)
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

    #[test]
    fn test_overflow_x_keep_y_ellipsis() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 1.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 1);
        let text = visible(&sys, layout.handle);
        assert!(text.ends_with('…'));
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(last_glyph.x + last_glyph.width as f32 <= 25.0 + 0.1);
    }

    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_drop() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 1.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    #[test]
    fn test_overflow_x_keep_y_ellipsis_fallback_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 1.5),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "…");
        let run = &sys.runs[layout.handle.0];
        let last_glyph = run.glyphs.last().unwrap();
        assert!(last_glyph.x + last_glyph.width as f32 > 8.0 + 0.1);
    }

    #[test]
    fn test_overflow_x_ellipsis_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 2.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert!(text.contains('…'));
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(last_g.x + last_g.width as f32 <= 23.0 + 0.1);
        }
    }

    #[test]
    fn test_overflow_x_ellipsis_fallback_drop_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 2.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.glyphs.len(), 0);
    }

    #[test]
    fn test_overflow_x_ellipsis_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::Ellipsis {
                fallback: EllipsisFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 8.0, lh * 2.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "……");
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            let last_g = line_glyphs.last().unwrap();
            assert!(last_g.x + last_g.width as f32 > 8.0 + 0.1);
        }
    }

    // Keep this test in sync with Card 1 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, 65.0),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello\nhello");
    }

    // Keep this test in sync with Card 2 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_fallback_drop_y_keep() {
        let mut sys = sys();
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 6.0, 70.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 3 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_glyph_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapGlyph {
                fallback: WrapGlyphFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello\nhello",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 4.0, lh * 13.0),
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
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 48.0, lh * 4.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 4);
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 5 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 23.0, lh * 10.0),
        );
        assert!(sys.runs[layout.handle.0].lines.len() > 4);
        let text = visible(&sys, layout.handle);
        let run = &sys.runs[layout.handle.0];
        for (i, line) in run.lines.iter().enumerate() {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                println!(
                    "line {}, char={:?}, x={}, width={}",
                    i, g.parent, g.x, g.width
                );
            }
        }
        assert_eq!(text, "hello there\nhello there");
    }

    // Keep this test in sync with Card 6 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_fallback_drop_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Drop,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 6.0, lh * 10.0),
        );
        let text = visible(&sys, layout.handle);
        assert!(text.trim().is_empty());
    }

    // Keep this test in sync with Card 7 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_wrap_glyph_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::WrapGlyph {
                    fallback: WrapGlyphFallback::Keep,
                },
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 4.0, lh * 25.0),
        );
        let text = visible(&sys, layout.handle);
        assert_eq!(text, "hello there\nhello there");
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 20);
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
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Drop,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 5.5),
        );
        assert_eq!(sys.runs[layout.handle.0].lines.len(), 2);
        let run = &sys.runs[layout.handle.0];
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            for g in line_glyphs {
                assert!(g.x + g.width as f32 <= 25.0 + 0.1);
            }
        }
    }

    // Keep this test in sync with Card 9 in Section 4.1 of sample/src/label_page.rs
    #[test]
    fn test_wrap_word_fallback_keep_y_keep() {
        let mut sys = sys();
        let lh = sys.line_height(16.0, FontId(1));
        let flow = TextFlow {
            overflow_x: OverflowX::WrapWord {
                fallback: WrapWordFallback::Keep,
            },
            overflow_y: OverflowY::Keep,
            horizontal_align: HorizontalAlign::Start,
        };
        let layout = sys.prepare(
            "hello there\nhello there",
            16.0,
            FontId(1),
            flow,
            Rect::new(0.0, 0.0, 25.0, lh * 5.5),
        );
        let run = &sys.runs[layout.handle.0];
        assert_eq!(run.lines.len(), 2);
        let mut has_overflow = false;
        for line in &run.lines {
            let line_glyphs = &run.glyphs[line.glyph_start..line.glyph_end];
            if let Some(last_g) = line_glyphs.last() {
                if last_g.parent != '\n'
                    && last_g.parent != ' '
                    && last_g.x + last_g.width as f32 > 25.0
                {
                    has_overflow = true;
                }
            }
        }
        assert!(has_overflow);
    }
}
