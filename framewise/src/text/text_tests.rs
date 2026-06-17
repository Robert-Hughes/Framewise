use super::*;
use crate::{
    test_utils::TestTextBackend, Color, DrawCmd, DrawCommands, DrawGlyph, FontId,
    PrepareGlyphRequest, PreparedGlyphHandle, Rect, Vec2,
};

fn style(flow: TextFlow) -> TextStyle {
    TextStyle::new(FontId(0), 12.0, 400, flow)
}

fn layout(text: &str, flow: TextFlow, bounds: TextBounds) -> TextLayout<u32> {
    let mut backend = TestTextBackend;
    layout_text(&mut backend, text, style(flow), bounds)
}

fn visible(layout: &TextLayout<u32>) -> String {
    layout
        .glyphs
        .iter()
        .filter_map(|glyph| char::from_u32(glyph.id))
        .collect()
}

fn line_source(text: &str, layout: &TextLayout<u32>, line_idx: usize) -> String {
    let line = &layout.lines[line_idx];
    layout.clusters[line.cluster_start..line.cluster_end]
        .iter()
        .map(|cluster| &text[cluster.byte_start..cluster.byte_end])
        .collect()
}

fn line_width(text: &str, flow: TextFlow) -> f32 {
    layout(text, flow, TextBounds::UNBOUNDED).lines[0].logical_width
}

fn visual_lines(layout: &TextLayout<u32>) -> Vec<String> {
    layout
        .lines
        .iter()
        .map(|line| {
            layout.glyphs[line.glyph_start..line.glyph_end]
                .iter()
                .filter_map(|glyph| char::from_u32(glyph.id))
                .collect()
        })
        .collect()
}

fn wrap_word_keep() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::WrapWord {
            fallback: WrapWordFallback::Keep,
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn wrap_word_drop() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::WrapWord {
            fallback: WrapWordFallback::Drop,
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn wrap_word_cluster_drop() -> TextFlow {
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

fn wrap_word_cluster_keep() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::WrapWord {
            fallback: WrapWordFallback::WrapCluster {
                fallback: WrapClusterFallback::Keep,
            },
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn wrap_cluster_keep() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::WrapCluster {
            fallback: WrapClusterFallback::Keep,
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn wrap_cluster_drop() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::WrapCluster {
            fallback: WrapClusterFallback::Drop,
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn keep_x_keep_y() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::Keep,
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn drop_x_drop_y() -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::Drop,
        overflow_y: OverflowY::Drop,
        line_align: TextLineAlign::Start,
    }
}

fn ellipsis_x_keep_y(fallback: EllipsisFallback) -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::Ellipsis { fallback },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    }
}

fn keep_x_ellipsis_y(fallback: EllipsisFallback) -> TextFlow {
    TextFlow {
        overflow_x: OverflowX::Keep,
        overflow_y: OverflowY::Ellipsis { fallback },
        line_align: TextLineAlign::Start,
    }
}

fn assert_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() <= 0.01,
        "{label}: expected {expected}, got {actual}"
    );
}

fn assert_line_ranges(layout: &TextLayout<u32>, ranges: &[(usize, usize)]) {
    let actual = layout
        .lines
        .iter()
        .map(|line| (line.byte_start, line.byte_end))
        .collect::<Vec<_>>();
    assert_eq!(actual, ranges);
}

struct ApproxInkBackend;

impl TextBackend for ApproxInkBackend {
    type ShapedGlyphId = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        20.0
    }

    fn shape_text(&mut self, text: &str, _style: TextStyle) -> ShapedText<Self::ShapedGlyphId> {
        ShapedText {
            clusters: vec![ShapedCluster {
                byte_start: 0,
                byte_end: text.len(),
                advance: 30.0,
                is_whitespace: false,
                glyphs: vec![ShapedGlyph {
                    id: 1,
                    x: 2.0,
                    y: -12.0,
                    advance: 30.0,
                    approx_ink_bounds: Some(Rect::new(-4.0, 3.0, 18.0, 10.0)),
                }],
            }],
        }
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphId>,
    ) -> Option<DrawGlyph> {
        Some(DrawGlyph {
            handle: PreparedGlyphHandle(request.glyph),
            top_left: request.glyph_origin,
        })
    }
}

#[test]
fn single_line_is_one_line() {
    let layout = layout(
        "hello world",
        TextFlow::single_line(),
        TextBounds::UNBOUNDED,
    );

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].end_kind, LineEndKind::EndOfText);
    assert!(!layout.metrics().truncated_horizontal);
    assert!(!layout.metrics().truncated_vertical);
    assert!(layout.metrics().logical_size.x > 0.0);
}

#[test]
fn hard_breaks_make_lines_without_wrap() {
    let layout = layout("a\nb\nc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.metrics().line_count, 3);
    assert_eq!(visual_lines(&layout), ["a", "b", "c"]);
    assert_eq!(layout.metrics().lines[0].end_kind, LineEndKind::HardNewline);
    assert_eq!(layout.metrics().lines[1].end_kind, LineEndKind::HardNewline);
    assert_eq!(layout.metrics().lines[2].end_kind, LineEndKind::EndOfText);
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 2)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (2, 4)
    );
    assert_eq!(
        (layout.lines[2].byte_start, layout.lines[2].byte_end),
        (4, 5)
    );
}

#[test]
fn empty_text_reports_one_blank_line() {
    let layout = layout("", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().logical_size, Vec2::new(0.0, 16.0));
    assert_eq!(layout.metrics().approx_ink_bounds, Rect::ZERO);
    assert_eq!(layout.lines[0].byte_start, 0);
    assert_eq!(layout.lines[0].byte_end, 0);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EndOfText);
    assert_eq!(
        layout.caret_geom(CaretPosition::EmptyText),
        CaretGeom {
            x: 0.0,
            y_top: 0.0,
            height: 16.0,
        }
    );
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 100.0)),
        CaretPosition::EmptyText
    );
    assert_eq!(layout.caret_insertion_byte(CaretPosition::EmptyText), 0);
}

#[test]
fn wrapping_splits_a_long_line() {
    let unwrapped = layout(
        "the quick brown fox",
        TextFlow::wrapped(),
        TextBounds::UNBOUNDED,
    );
    let wrapped = layout(
        "the quick brown fox",
        TextFlow::wrapped(),
        TextBounds::width(64.0),
    );

    assert_eq!(unwrapped.metrics().line_count, 1);
    assert!(wrapped.metrics().line_count > 1);
    assert!(wrapped.metrics().logical_size.x <= 64.0);
}

#[test]
fn vertical_overflow_truncates_lines() {
    let layout = layout(
        "one two three four five six",
        TextFlow::wrapped(),
        TextBounds {
            max_width: Some(32.0),
            max_height: Some(33.0),
        },
    );

    assert_eq!(layout.metrics().line_count, 2);
    assert!(layout.metrics().truncated_vertical);
}

#[test]
fn single_line_overflow_truncates_horizontally() {
    let layout = layout(
        "abcdef",
        TextFlow::single_line(),
        TextBounds {
            max_width: Some(20.0),
            max_height: None,
        },
    );

    assert!(layout.metrics().truncated_horizontal);
    assert_eq!(visible(&layout), "ab");
    assert_eq!(layout.lines[0].end_kind, LineEndKind::OverflowDrop);
}

#[test]
fn overflow_keep_keeps_first_overflowing_cluster() {
    let flow = TextFlow {
        overflow_x: OverflowX::Keep,
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    };
    let layout = layout("abcdef", flow, TextBounds::width(20.0));

    assert!(layout.metrics().truncated_horizontal);
    assert_eq!(visible(&layout), "abc");
    assert_eq!(layout.lines[0].logical_width, 24.0);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::OverflowKeep);
}

#[test]
fn ellipsis_is_appended_on_single_line_overflow() {
    let flow = TextFlow {
        overflow_x: OverflowX::Ellipsis {
            fallback: EllipsisFallback::Drop,
        },
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Start,
    };

    let layout = layout("abcdef", flow, TextBounds::width(32.0));

    assert!(layout.metrics().truncated_horizontal);
    assert_eq!(visible(&layout), "abc…");
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisX);
    assert!(layout.lines[0].logical_width <= 32.0);
}

#[test]
fn ellipsis_on_last_line_when_height_clipped() {
    let flow = TextFlow {
        overflow_x: OverflowX::WrapCluster {
            fallback: WrapClusterFallback::Drop,
        },
        overflow_y: OverflowY::Ellipsis {
            fallback: EllipsisFallback::Drop,
        },
        line_align: TextLineAlign::Start,
    };

    let layout = layout(
        "abcdefghij",
        flow,
        TextBounds {
            max_width: Some(24.0),
            max_height: Some(16.0),
        },
    );

    assert_eq!(layout.metrics().line_count, 1);
    assert!(layout.metrics().truncated_vertical);
    assert_eq!(visible(&layout), "ab…");
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisY);
}

#[test]
fn wrap_cluster_keep_moves_overwide_cluster_to_new_line_before_fallback() {
    let layout = layout("ok x", wrap_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(layout.lines.len(), 2);
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 3)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (3, 4)
    );
    assert_eq!(layout.clusters[layout.lines[1].cluster_start].x, 0.0);
}

#[test]
fn wrap_word_keep_moves_overlong_word_to_new_line_before_fallback() {
    let layout = layout("ok abcdef", wrap_word_keep(), TextBounds::width(16.1));

    assert_eq!(layout.lines.len(), 2);
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 3)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (3, 9)
    );
    assert_eq!(layout.clusters[layout.lines[1].cluster_start].x, 0.0);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_drop_fallbacks() {
    for flow in [
        wrap_word_drop(),
        wrap_word_cluster_drop(),
        wrap_cluster_drop(),
    ] {
        let layout = layout(" ", flow, TextBounds::width(1.0));

        assert_eq!(layout.metrics().line_count, 1);
        assert_eq!(layout.glyphs.len(), 0);
        assert_eq!(layout.metrics().lines[0].logical_width, 0.0);
    }
}

#[test]
fn overwide_whitespace_on_empty_line_uses_keep_fallbacks() {
    for flow in [
        wrap_word_keep(),
        wrap_word_cluster_keep(),
        wrap_cluster_keep(),
    ] {
        let layout = layout(" ", flow, TextBounds::width(1.0));

        assert_eq!(layout.metrics().line_count, 1);
        assert_eq!(layout.clusters.len(), 1);
        assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    }
}

#[test]
fn soft_wrap_boundary_space_collapses_between_words() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(visible(&layout), "helloworld");
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 6)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (6, 11)
    );
    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapWhitespace);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert!(layout.clusters[layout.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn trailing_boundary_space_creates_empty_line_under_word_wrap() {
    let layout = layout("hello ", wrap_word_cluster_drop(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 6)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (6, 6)
    );
    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapWhitespace);
    assert_eq!(layout.lines[1].end_kind, LineEndKind::EndOfText);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert_eq!(layout.lines[1].logical_width, 0.0);
    assert!(layout.clusters[layout.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn only_the_single_boundary_space_collapses() {
    let layout = layout(
        "hello  world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(layout.metrics().line_count, 3);
    assert_eq!(
        (layout.lines[0].byte_start, layout.lines[0].byte_end),
        (0, 6)
    );
    assert_eq!(
        (layout.lines[1].byte_start, layout.lines[1].byte_end),
        (6, 7)
    );
    assert_eq!(
        (layout.lines[2].byte_start, layout.lines[2].byte_end),
        (7, 12)
    );
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert_eq!(layout.lines[1].logical_width, 8.0);
}

#[test]
fn leading_spaces_are_preserved_and_only_overflowing_spaces_collapse() {
    let five_spaces = layout(
        "     hello",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(five_spaces.metrics().line_count, 2);
    assert_eq!(
        (
            five_spaces.lines[0].byte_start,
            five_spaces.lines[0].byte_end
        ),
        (0, 5)
    );
    assert_eq!(
        (
            five_spaces.lines[1].byte_start,
            five_spaces.lines[1].byte_end
        ),
        (5, 10)
    );
    assert_eq!(five_spaces.lines[0].logical_width, 40.0);

    let six_spaces = layout(
        "      hello",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(six_spaces.metrics().line_count, 2);
    assert_eq!(
        (six_spaces.lines[0].byte_start, six_spaces.lines[0].byte_end),
        (0, 6)
    );
    assert_eq!(
        (six_spaces.lines[1].byte_start, six_spaces.lines[1].byte_end),
        (6, 11)
    );
    assert_eq!(six_spaces.lines[0].logical_width, 40.0);
    assert!(six_spaces.clusters[six_spaces.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn caret_advances_along_single_line() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout
            .caret_geom(CaretPosition::BeforeCluster {
                cluster_byte_index: 0
            })
            .x,
        0.0
    );
    assert_eq!(
        layout
            .caret_geom(CaretPosition::BeforeCluster {
                cluster_byte_index: 1
            })
            .x,
        8.0
    );
    assert_eq!(
        layout
            .caret_geom(CaretPosition::AfterCluster {
                cluster_byte_index: 2
            })
            .x,
        24.0
    );
}

#[test]
fn hit_test_round_trips_to_boundaries() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_insertion_byte(layout.hit_test_caret(Vec2::new(0.0, 1.0))),
        0
    );
    assert_eq!(
        layout.caret_insertion_byte(layout.hit_test_caret(Vec2::new(7.9, 1.0))),
        1
    );
    assert_eq!(
        layout.caret_insertion_byte(layout.hit_test_caret(Vec2::new(100.0, 1.0))),
        3
    );
}

#[test]
fn caret_position_distinguishes_before_and_after_newline_cluster() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_index: 1,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_index: 1,
    });

    assert_eq!(before.y_top, 0.0);
    assert_eq!(before.x, 8.0);
    assert_eq!(after.y_top, 16.0);
    assert_eq!(after.x, 0.0);
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 1,
        }
    );
}

#[test]
fn caret_navigation_chooses_newline_side_by_direction() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 0,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 1,
        }
    );
    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 1,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_index: 1,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 2,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_index: 1,
        }
    );
}

#[test]
fn collapsed_soft_wrap_space_has_newline_like_caret_and_hit_test_behavior() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_index: 5,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_index: 5,
    });

    assert_eq!(before.y_top, 0.0);
    assert_eq!(before.x, 40.0);
    assert_eq!(after.y_top, 16.0);
    assert_eq!(after.x, 0.0);
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 5,
        }
    );
}

#[test]
fn caret_position_distinguishes_soft_wrap_boundary_space_sides() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(
        layout.caret_insertion_byte(CaretPosition::BeforeCluster {
            cluster_byte_index: 5,
        }),
        5
    );
    assert_eq!(
        layout.caret_insertion_byte(CaretPosition::AfterCluster {
            cluster_byte_index: 5,
        }),
        6
    );
    assert_eq!(
        layout.caret_position_at_insertion_byte(6),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 6,
        }
    );
}

#[test]
fn horizontal_alignment_affects_line_offsets() {
    for (align, expected_x) in [
        (TextLineAlign::Start, 0.0),
        (TextLineAlign::Center, 40.0),
        (TextLineAlign::End, 80.0),
    ] {
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: align,
        };
        let layout = layout("hi", flow, TextBounds::width(96.0));

        assert_close(layout.lines[0].logical_x, expected_x, "logical_x");
        assert_close(layout.clusters[0].x, expected_x, "cluster x");
    }
}

#[test]
fn caret_geom_alignment_empty_lines_and_empty_text() {
    for (align, expected_x) in [
        (TextLineAlign::Start, 0.0),
        (TextLineAlign::Center, 50.0),
        (TextLineAlign::End, 100.0),
    ] {
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: align,
        };

        let empty = layout("", flow, TextBounds::width(100.0));
        assert_close(
            empty.caret_geom(CaretPosition::EmptyText).x,
            expected_x,
            "empty caret",
        );

        let trailing = layout("a\n", flow, TextBounds::width(100.0));
        assert_close(
            trailing
                .caret_geom(CaretPosition::AfterCluster {
                    cluster_byte_index: 1,
                })
                .x,
            expected_x,
            "trailing newline caret",
        );
    }
}

#[test]
fn emit_glyphs_skips_backend_non_drawable_glyphs_and_offsets_origin() {
    let style = style(TextFlow::single_line());
    let mut backend = TestTextBackend;
    let layout = layout_text(&mut backend, "a b", style, TextBounds::UNBOUNDED);
    let mut commands = DrawCommands::new();

    layout.emit_glyphs(
        &mut commands,
        &mut backend,
        Vec2::new(10.0, 20.0),
        style,
        Color::BLACK,
        3,
    );

    assert_eq!(commands.glyphs().len(), 2);
    assert_eq!(commands.glyphs()[0].top_left, Vec2::new(10.0, 32.0));
    assert_eq!(commands.glyphs()[1].top_left, Vec2::new(26.0, 32.0));
    assert_eq!(
        commands.commands(),
        &[DrawCmd::GlyphRun {
            glyphs: 0..2,
            color: Color::BLACK,
            z: 3,
        }]
    );
}

#[test]
fn emit_glyphs_omits_empty_runs() {
    let style = style(TextFlow::single_line());
    let mut backend = TestTextBackend;
    let layout = layout_text(&mut backend, "   ", style, TextBounds::UNBOUNDED);
    let mut commands = DrawCommands::new();

    layout.emit_glyphs(
        &mut commands,
        &mut backend,
        Vec2::ZERO,
        style,
        Color::BLACK,
        0,
    );

    assert!(commands.commands().is_empty());
    assert!(commands.glyphs().is_empty());
}

#[test]
fn test_sample_text_backend_line_metrics() {
    let layout = layout(
        "hello\nworld",
        TextFlow::single_line(),
        TextBounds::UNBOUNDED,
    );

    assert_eq!(layout.metrics().line_count, 2);
    assert_line_ranges(&layout, &[(0, 6), (6, 11)]);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::HardNewline);
    assert_eq!(layout.lines[1].end_kind, LineEndKind::EndOfText);
    assert_eq!(layout.lines[0].y_top, 0.0);
    assert_eq!(layout.lines[1].y_top, 16.0);
}

#[test]
fn test_line_metrics_logical_and_ink_widths() {
    let hard = layout(
        "hello\nworld",
        TextFlow::single_line(),
        TextBounds::UNBOUNDED,
    );
    for line in &hard.metrics().lines {
        assert!(line.logical_width > 0.0);
        assert!(line.approx_ink_width > 0.0);
        assert!(line.approx_ink_width <= line.logical_width);
    }

    let soft = layout(
        "hello wrapping world",
        wrap_word_cluster_drop(),
        TextBounds::width(48.0),
    );
    assert!(soft.metrics().line_count > 1);
    for line in &soft.metrics().lines {
        assert!(line.logical_width <= 48.0);
        assert_eq!(line.approx_ink_width, line.logical_width);
    }
}

#[test]
fn test_word_wrap_preserves_spaces() {
    let text = "hello there";
    let layout = layout(text, TextFlow::wrapped(), TextBounds::width(500.0));

    assert_eq!(layout.clusters.len(), text.chars().count());
    assert_eq!(line_source(text, &layout, 0), text);
}

#[test]
fn prepare_with_measured_logical_bounds_preserves_metrics() {
    let text = "Button Demo";
    let style = TextFlow::wrapped();
    let measured = layout(text, style, TextBounds::UNBOUNDED);
    let prepared = layout(
        text,
        style,
        TextBounds {
            max_width: Some(measured.metrics().logical_size.x),
            max_height: Some(measured.metrics().logical_size.y),
        },
    );

    assert_eq!(prepared.metrics(), measured.metrics());
}

#[test]
fn empty_text_prepare_registers_empty_run_with_matching_metrics() {
    let measured = layout("", TextFlow::single_line(), TextBounds::UNBOUNDED);
    let prepared = layout(
        "",
        TextFlow::single_line(),
        TextBounds {
            max_width: Some(100.0),
            max_height: Some(100.0),
        },
    );

    assert_eq!(prepared.metrics().line_count, measured.metrics().line_count);
    assert_eq!(
        prepared.metrics().logical_size.x,
        measured.metrics().logical_size.x
    );
    assert_eq!(prepared.glyphs.len(), 0);
}

#[test]
fn wrap_cluster_keep_does_not_split_combining_mark_cluster() {
    let text = "e\u{0301}x";
    let layout = layout(text, wrap_cluster_keep(), TextBounds::width(4.0));

    assert_eq!(layout.clusters[0].byte_start, 0);
    assert_eq!(layout.clusters[0].byte_end, "e\u{0301}".len());
    assert_eq!(
        layout.clusters[0].glyph_end - layout.clusters[0].glyph_start,
        2
    );
}

#[test]
fn word_wrap_breaks_after_tab_whitespace() {
    let text = "ab\tcd";
    let layout = layout(text, wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert!(layout.metrics().line_count >= 2);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapWhitespace);
    assert_eq!(line_source(text, &layout, 0), "ab\t");
}

#[test]
fn hit_test_cannot_target_a_line_made_from_half_a_cluster() {
    let text = "e\u{0301}x";
    let layout = layout(text, wrap_cluster_keep(), TextBounds::width(4.0));

    assert_eq!(layout.hit_test_cluster(Vec2::new(0.0, 0.0)), 0);
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 0.0)),
        CaretPosition::AfterCluster {
            cluster_byte_index: 0,
        }
    );
}

#[test]
fn line_records_include_cluster_ranges() {
    let layout = layout("ab cd", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert!(layout
        .lines
        .iter()
        .all(|line| line.cluster_start <= line.cluster_end));
    assert!(layout
        .lines
        .iter()
        .all(|line| line.glyph_start <= line.glyph_end));
    assert_eq!(layout.lines[0].cluster_start, 0);
}

#[test]
fn caret_inside_combining_mark_cluster_clamps_to_cluster_start() {
    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_position_at_insertion_byte(1),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 0,
        }
    );
}

#[test]
fn hit_test_combining_mark_cluster_returns_cluster_boundaries() {
    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_insertion_byte(layout.hit_test_caret(Vec2::new(0.0, 0.0))),
        0
    );
    assert_eq!(
        layout.caret_insertion_byte(layout.hit_test_caret(Vec2::new(7.9, 0.0))),
        text.find('x').unwrap()
    );
}

#[test]
fn metrics_report_approx_ink_bounds_separately_from_logical_size() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.metrics().logical_size, Vec2::new(24.0, 16.0));
    assert_eq!(
        layout.metrics().approx_ink_bounds,
        Rect::new(0.0, 0.0, 24.0, 16.0)
    );
}

#[test]
fn measure_text_reports_backend_approx_ink_bounds() {
    let mut backend = ApproxInkBackend;
    let metrics = measure_text(
        &mut backend,
        "x",
        style(TextFlow::single_line()),
        TextBounds::UNBOUNDED,
    );

    assert_eq!(metrics.logical_size, Vec2::new(30.0, 20.0));
    assert_eq!(metrics.approx_ink_bounds, Rect::new(-2.0, 3.0, 18.0, 10.0));
    assert_eq!(metrics.lines[0].approx_ink_x, -2.0);
    assert_eq!(metrics.lines[0].approx_ink_width, 18.0);
}

#[test]
fn metrics_report_whitespace_logical_advance_without_ink() {
    let layout = layout("   ", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.metrics().logical_size.x, 24.0);
    assert_eq!(layout.glyphs.len(), 0);
    assert_eq!(layout.metrics().approx_ink_bounds, Rect::ZERO);
}

#[test]
fn drop_overflow_uses_logical_advance_not_ink_width_for_single_glyph() {
    let layout = layout("a", TextFlow::single_line(), TextBounds::width(7.0));

    assert_eq!(layout.glyphs.len(), 0);
    assert!(layout.metrics().truncated_horizontal);
}

#[test]
fn drop_overflow_uses_logical_advance_not_ink_width_for_final_glyph() {
    let layout = layout("ab", TextFlow::single_line(), TextBounds::width(15.0));

    assert_eq!(visible(&layout), "a");
    assert_eq!(layout.metrics().logical_size.x, 8.0);
}

#[test]
fn caret_end_uses_shaped_advance_not_bitmap_width() {
    let layout = layout("ab", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout
            .caret_geom(CaretPosition::AfterCluster {
                cluster_byte_index: 1,
            })
            .x,
        16.0
    );
}

#[test]
fn center_align_centers_a_fitting_line() {
    let flow = TextFlow {
        overflow_x: OverflowX::Keep,
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Center,
    };
    let layout = layout("hi", flow, TextBounds::width(96.0));

    assert_eq!(layout.lines[0].logical_x, 40.0);
}

#[test]
fn caret_on_second_line_is_offset_in_y() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout
            .caret_geom(CaretPosition::BeforeCluster {
                cluster_byte_index: 2,
            })
            .y_top,
        16.0
    );
}

#[test]
fn long_unbreakable_word_is_force_broken() {
    let layout = layout("abcdef", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert!(layout.metrics().line_count > 1);
    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn hit_test_right_of_wrapped_long_word_line_selects_after_last_cluster() {
    let layout = layout("abcdef", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::AfterCluster {
            cluster_byte_index: 1,
        }
    );
}

#[test]
fn metrics_width_matches_logical_run_width_after_ellipsis() {
    let layout = layout(
        "abcdef",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds::width(32.0),
    );

    assert_eq!(
        layout.metrics().logical_size.x,
        layout.lines[0].logical_width.ceil()
    );
    assert_eq!(visible(&layout), "abc\u{2026}");
}

#[test]
fn center_align_keeps_overflowing_line_within_box() {
    let flow = TextFlow {
        overflow_x: OverflowX::Keep,
        overflow_y: OverflowY::Keep,
        line_align: TextLineAlign::Center,
    };
    let layout = layout("abcdef", flow, TextBounds::width(16.0));

    assert_eq!(layout.lines[0].logical_x, 0.0);
    assert!(layout.metrics().logical_size.x > 16.0);
}

#[test]
fn multiline_hit_test_picks_the_right_line() {
    let layout = layout("ab\ncd", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.hit_test_cluster(Vec2::new(1.0, 1.0)), 0);
    assert_eq!(layout.hit_test_cluster(Vec2::new(1.0, 17.0)), 3);
}

#[test]
fn hit_test_right_of_newline_line_stays_on_same_line() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 1,
        }
    );
}

#[test]
fn hit_test_right_of_blank_newline_line_stays_on_same_line() {
    let layout = layout("\n\n", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 17.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 1,
        }
    );
}

#[test]
fn test_hit_test_cluster_edge_cases() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.hit_test_cluster(Vec2::new(-100.0, -100.0)), 0);
    assert_eq!(layout.hit_test_cluster(Vec2::new(100.0, 100.0)), 2);
    assert_eq!(layout.hit_test_cluster(Vec2::new(8.0, 0.0)), 0);
    assert_eq!(layout.hit_test_cluster(Vec2::new(8.1, 0.0)), 1);
}

#[test]
fn caret_geom_at_newline_index_stays_on_same_line() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let caret = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_index: 1,
    });
    assert_eq!(caret.y_top, 0.0);
    assert_eq!(caret.x, 8.0);
}

#[test]
fn caret_geom_after_newline_index_is_on_next_line() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let caret = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_index: 1,
    });
    assert_eq!(caret.y_top, 16.0);
    assert_eq!(caret.x, 0.0);
}

#[test]
fn caret_navigation_moves_by_cluster_boundaries() {
    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);
    let x_byte = text.find('x').unwrap();

    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 0,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_index: x_byte,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::AfterCluster {
            cluster_byte_index: x_byte,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_index: x_byte,
        }
    );
}

#[test]
fn test_caret_geom_soft_wrap_boundaries() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_index: 5,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_index: 5,
    });
    assert_eq!(
        before,
        CaretGeom {
            x: 40.0,
            y_top: 0.0,
            height: 16.0
        }
    );
    assert_eq!(
        after,
        CaretGeom {
            x: 0.0,
            y_top: 16.0,
            height: 16.0
        }
    );
}

#[test]
fn caret_navigation_chooses_soft_wrap_boundary_space_side_by_direction() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 4,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_index: 5,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_index: 6,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_index: 5,
        }
    );
}

#[test]
fn test_overflow_x_drop_y_drop() {
    let layout = layout(
        "abcdef\nghijkl",
        drop_x_drop_y(),
        TextBounds {
            max_width: Some(16.1),
            max_height: Some(16.0),
        },
    );

    assert_eq!(visible(&layout), "ab");
    assert!(layout.metrics().truncated_horizontal);
    assert!(layout.metrics().truncated_vertical);
}

#[test]
fn test_overflow_x_keep_y_keep() {
    let layout = layout(
        "abcdef\nghijkl",
        keep_x_keep_y(),
        TextBounds {
            max_width: Some(16.1),
            max_height: Some(16.0),
        },
    );

    assert_eq!(visual_lines(&layout), ["abc", "ghi"]);
    assert!(layout.metrics().logical_size.y > 16.0);
}

#[test]
fn test_overflow_x_keep_y_ellipsis() {
    let layout = layout(
        "abcdef\nghijkl",
        keep_x_ellipsis_y(EllipsisFallback::Keep),
        TextBounds {
            max_width: Some(16.1),
            max_height: Some(16.0),
        },
    );

    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisY);
    assert!(layout.metrics().truncated_vertical);
}

#[test]
fn test_overflow_x_keep_y_ellipsis_fallback_drop() {
    let layout = layout(
        "a\nb",
        keep_x_ellipsis_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(0.0),
            max_height: Some(16.0),
        },
    );

    assert!(layout.metrics().truncated_vertical);
    assert_eq!(layout.glyphs.len(), 0);
}

#[test]
fn test_overflow_x_keep_y_ellipsis_fallback_keep() {
    let layout = layout(
        "a\nb",
        keep_x_ellipsis_y(EllipsisFallback::Keep),
        TextBounds {
            max_width: Some(0.0),
            max_height: Some(16.0),
        },
    );

    assert!(layout.metrics().truncated_vertical);
    assert!(!layout.clusters.is_empty());
}

#[test]
fn test_overflow_x_ellipsis_y_keep() {
    let layout = layout(
        "abcdef\nghijkl",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(16.1),
            max_height: Some(16.0),
        },
    );

    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisX);
    assert!(layout.metrics().logical_size.y > 16.0);
}

#[test]
fn test_overflow_x_ellipsis_fallback_drop_y_keep() {
    let layout = layout(
        "abc",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds::width(0.0),
    );

    assert_eq!(layout.glyphs.len(), 0);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisX);
}

#[test]
fn test_overflow_x_ellipsis_fallback_keep_y_keep() {
    let layout = layout(
        "abc",
        ellipsis_x_keep_y(EllipsisFallback::Keep),
        TextBounds::width(0.0),
    );

    assert_eq!(visible(&layout), "\u{2026}");
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisX);
}

#[test]
fn test_wrap_cluster_y_keep() {
    let layout = layout("abcdef", wrap_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn test_wrap_cluster_fallback_drop_y_keep() {
    let layout = layout("a", wrap_cluster_drop(), TextBounds::width(0.0));

    assert_eq!(visible(&layout), "");
    assert_eq!(layout.metrics().logical_size.x, 0.0);
}

#[test]
fn test_wrap_cluster_fallback_keep_y_keep() {
    let layout = layout("a", wrap_cluster_keep(), TextBounds::width(0.0));

    assert_eq!(visible(&layout), "a");
    assert!(layout.metrics().logical_size.x > 0.0);
}

#[test]
fn label_wrap_cluster_fallback_keep_uses_widget_width() {
    let layout = layout("abcdef", wrap_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn test_wrap_word_y_keep() {
    let layout = layout("ab cd ef", wrap_word_keep(), TextBounds::width(24.1));

    assert_eq!(layout.metrics().line_count, 3);
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_y_keep() {
    let layout = layout("abcdef", wrap_word_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_fallback_drop_y_keep() {
    let layout = layout("abcdef", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_fallback_keep_y_keep() {
    let layout = layout("abcdef", wrap_word_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn label_wrap_word_cluster_keep_uses_widget_width() {
    let layout = layout("abcdef", wrap_word_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
}

#[test]
fn test_wrap_word_fallback_drop_y_keep() {
    let layout = layout("abcdef", wrap_word_drop(), TextBounds::width(16.1));

    assert_eq!(visible(&layout), "ab");
}

#[test]
fn test_wrap_word_fallback_keep_y_keep() {
    let layout = layout("abcdef", wrap_word_keep(), TextBounds::width(16.1));

    assert_eq!(visible(&layout), "abc");
}

#[test]
fn test_newline_wrapping_collapse() {
    let layout = layout(
        "ab\ncd ef",
        wrap_word_cluster_drop(),
        TextBounds::width(16.1),
    );

    assert_eq!(layout.lines[0].end_kind, LineEndKind::HardNewline);
    assert!(layout.metrics().line_count >= 3);
}

#[test]
fn soft_wrap_collapses_fitted_boundary_space_before_next_word() {
    let flow = wrap_word_cluster_drop();
    let wrap_w = line_width("hello ", flow) + 0.1;
    let layout = layout("hello world", flow, TextBounds::width(wrap_w));

    assert_eq!(layout.lines[0].logical_width, line_width("hello", flow));
    assert!(layout.clusters[layout.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn right_aligned_soft_wrap_excludes_fitted_boundary_space_from_line_width() {
    let mut flow = wrap_word_cluster_drop();
    flow.line_align = TextLineAlign::End;
    let wrap_w = line_width("hello ", flow) + 0.1;
    let layout = layout("hello world", flow, TextBounds::width(wrap_w));

    assert_eq!(layout.lines[0].logical_width, line_width("hello", flow));
    assert!(layout.lines[0].logical_x > 0.0);
}

#[test]
fn terminal_trailing_spaces_collapse_only_the_boundary_space() {
    let layout = layout("hello  ", wrap_word_cluster_drop(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert_eq!(layout.lines[1].logical_width, 8.0);
}

#[test]
fn wrap_cluster_collapses_soft_wrap_boundary_space() {
    let layout = layout("hello world", wrap_cluster_drop(), TextBounds::width(40.1));

    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapWhitespace);
    assert_eq!(layout.lines[0].logical_width, 40.0);
}

#[test]
fn wrap_cluster_collapses_fitted_boundary_space_before_next_cluster() {
    let flow = wrap_cluster_drop();
    let wrap_w = line_width("hello ", flow) + 0.1;
    let layout = layout("hello world", flow, TextBounds::width(wrap_w));

    assert_eq!(layout.lines[0].logical_width, line_width("hello", flow));
}

#[test]
fn wrap_cluster_trailing_boundary_space_creates_empty_line() {
    let layout = layout("hello ", wrap_cluster_drop(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(layout.lines[1].logical_width, 0.0);
}

#[test]
fn whitespace_that_wraps_from_non_empty_line_does_not_use_wrap_word_keep_fallback() {
    let layout = layout("hello ", wrap_word_keep(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert!(layout.clusters[layout.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn whitespace_that_wraps_from_non_empty_line_does_not_use_wrap_cluster_keep_fallback() {
    let layout = layout("hello ", wrap_cluster_keep(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert!(layout.clusters[layout.lines[0].cluster_end - 1].is_soft_wrap_boundary);
}

#[test]
fn hit_test_round_trips_to_a_boundary() {
    hit_test_round_trips_to_boundaries();
}

#[test]
fn empty_text_measure_reports_one_blank_line() {
    empty_text_reports_one_blank_line();
}

#[test]
fn empty_text_caret_apis_are_stable() {
    let layout = layout("", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_position_at_insertion_byte(0),
        CaretPosition::EmptyText
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::EmptyText),
        CaretPosition::EmptyText
    );
    assert_eq!(
        layout.next_caret_position(CaretPosition::EmptyText),
        CaretPosition::EmptyText
    );
}

#[test]
fn empty_text_hit_testing_returns_empty_caret_and_byte_zero() {
    let layout = layout("", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let position = layout.hit_test_caret(Vec2::new(100.0, 100.0));
    assert_eq!(position, CaretPosition::EmptyText);
    assert_eq!(layout.caret_insertion_byte(position), 0);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_drop_fallback() {
    let layout = layout(" ", wrap_word_cluster_drop(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 0.0);
    assert!(layout.clusters.is_empty());
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_keep_fallback() {
    let layout = layout(" ", wrap_word_cluster_keep(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    assert_eq!(layout.clusters.len(), 1);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_drop_fallback() {
    let layout = layout(" ", wrap_word_drop(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 0.0);
    assert!(layout.clusters.is_empty());
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_keep_fallback() {
    let layout = layout(" ", wrap_word_keep(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    assert_eq!(layout.clusters.len(), 1);
}

#[test]
fn test_line_metrics_horizontal_alignment() {
    horizontal_alignment_affects_line_offsets();
}

#[test]
fn test_caret_geom_alignment_empty_lines_and_empty_text() {
    caret_geom_alignment_empty_lines_and_empty_text();
}
