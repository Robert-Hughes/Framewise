use super::*;
use crate::widgets::label::{raw as raw_label, LabelStyle};
use crate::{
    test_utils::TestTextBackend, Color, DrawCmd, DrawCommands, DrawGlyph, FontId, Layer,
    PrepareGlyphRequest, PreparedGlyphToken, Rect, TextContentPlacement, Vec2,
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
        .iter_resolved_glyphs()
        .filter_map(|glyph| char::from_u32(glyph.id))
        .collect()
}

fn line_source(text: &str, layout: &TextLayout<u32>, line_idx: usize) -> String {
    let line = &layout.lines[line_idx];
    line.clusters
        .iter()
        .map(|cluster| &text[cluster.byte_start..cluster.byte_end])
        .collect()
}

fn visual_line_sources(text: &str, layout: &TextLayout<u32>) -> Vec<String> {
    (0..layout.lines.len())
        .map(|line_idx| line_source(text, layout, line_idx))
        .collect()
}

fn total_clusters(layout: &TextLayout<u32>) -> usize {
    layout.lines.iter().map(|line| line.clusters.len()).sum()
}

fn first_cluster(layout: &TextLayout<u32>) -> &WorkingCluster {
    layout
        .lines
        .iter()
        .find_map(|line| line.clusters.first())
        .expect("layout should contain a cluster")
}

fn line_width(text: &str, flow: TextFlow) -> f32 {
    layout(text, flow, TextBounds::UNBOUNDED).lines[0].logical_width
}

fn visual_lines(layout: &TextLayout<u32>) -> Vec<String> {
    layout
        .lines
        .iter()
        .enumerate()
        .map(|(line_idx, _)| {
            layout
                .iter_resolved_line_glyphs(line_idx)
                .filter_map(|glyph| char::from_u32(glyph.id))
                .collect()
        })
        .collect()
}

fn line_visible_glyph_count(layout: &TextLayout<u32>, line_idx: usize) -> usize {
    layout.iter_resolved_line_glyphs(line_idx).count()
}

fn assert_visible_glyph_count_matches_resolved(layout: &TextLayout<u32>) {
    assert_eq!(
        layout.visible_glyph_count,
        layout.iter_resolved_glyphs().count()
    );
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

/// A compact wrapping fixture that describes visual lines as strings.
///
/// The `expected` rows are not rendered text; they are a source-preserving
/// diagram of the layout. Literal spaces are ordinary preserved whitespace,
/// empty strings are empty visual lines, and `~` is the only marker: a source
/// whitespace cluster that is logically present on the previous visual line but
/// collapsed to zero visual advance because it is a soft-wrap boundary.
///
/// For example, five 8 px columns with `"hello  world"` should be written as:
///
/// ```ignore
/// WrapDiagramCase {
///     name: "two spaces",
///     width_cols: 5,
///     input: "hello  world",
///     expected: &["hello~", " ", "world"],
/// }
/// ```
///
/// That means the first space after `hello` is the collapsed boundary
/// whitespace, while the second source space remains visible on its own line.
struct WrapDiagramCase {
    name: &'static str,
    width_cols: usize,
    input: &'static str,
    expected: &'static [&'static str],
}

/// Converts a concrete layout into the wrap diagram format used by
/// `WrapDiagramCase`.
///
/// Hard newlines are line boundaries, so they are accounted for by byte ranges
/// and line end kinds rather than emitted into the diagram rows.
fn encode_layout_diagram(text: &str, layout: &TextLayout<u32>) -> Vec<String> {
    layout
        .lines
        .iter()
        .map(|line| {
            let mut out = String::new();

            for cluster in &line.clusters {
                if cluster.is_soft_wrap_boundary {
                    out.push('~');
                } else {
                    let source = &text[cluster.byte_start..cluster.byte_end];
                    if source != "\n" {
                        out.push_str(source);
                    }
                }
            }

            out
        })
        .collect()
}

fn assert_wrap_diagram(case: WrapDiagramCase) {
    let layout = layout(
        case.input,
        wrap_word_cluster_drop(),
        TextBounds::width(case.width_cols as f32 * 8.0 + 0.1),
    );

    let actual = encode_layout_diagram(case.input, &layout);
    assert_eq!(actual, case.expected, "{}", case.name);

    assert_wrap_diagram_invariants(case.input, &layout, &actual, case.name);
}

fn assert_wrap_diagram_invariants(
    text: &str,
    layout: &TextLayout<u32>,
    actual: &[String],
    case_name: &str,
) {
    assert_eq!(
        layout.metrics().line_count as usize,
        actual.len(),
        "{case_name}: diagram line count should match metrics"
    );

    assert_eq!(
        layout.lines.len(),
        actual.len(),
        "{case_name}: diagram line count should match stored lines"
    );

    let mut next_byte_start = 0;
    for (line_idx, line) in layout.lines.iter().enumerate() {
        assert_eq!(
            line.byte_start, next_byte_start,
            "{case_name}: line {line_idx} should start where the previous line ended"
        );
        assert!(
            line.byte_end >= line.byte_start,
            "{case_name}: line {line_idx} should have a valid byte range"
        );
        next_byte_start = line.byte_end;

        let diagram_soft_wrap_boundaries = actual[line_idx].matches('~').count();
        let layout_soft_wrap_boundaries = line
            .clusters
            .iter()
            .filter(|cluster| {
                if cluster.is_soft_wrap_boundary {
                    let source = &text[cluster.byte_start..cluster.byte_end];
                    assert!(
                        source.chars().all(char::is_whitespace),
                        "{case_name}: line {line_idx} soft-wrap boundary should be source whitespace"
                    );
                    true
                } else {
                    false
                }
            })
            .count();
        assert_eq!(
            layout_soft_wrap_boundaries, diagram_soft_wrap_boundaries,
            "{case_name}: line {line_idx} should encode every soft-wrap boundary"
        );

        if actual[line_idx].ends_with('~') {
            assert_eq!(
                line.end_kind,
                LineEndKind::SoftWrapWhitespace,
                "{case_name}: line {line_idx} ending in ~ should be a soft-wrap whitespace line"
            );
            assert!(
                line.clusters
                    .last()
                    .is_some_and(|cluster| cluster.is_soft_wrap_boundary),
                "{case_name}: line {line_idx} ending in ~ should end with a boundary cluster"
            );
        }

        if line.end_kind == LineEndKind::SoftWrapWhitespace {
            assert!(
                actual[line_idx].contains('~'),
                "{case_name}: soft-wrap whitespace line {line_idx} should contain ~"
            );
        }

        let expected_logical_width =
            actual[line_idx].chars().filter(|ch| *ch != '~').count() as f32 * 8.0;
        assert_close(
            line.logical_width,
            expected_logical_width,
            &format!("{case_name}: line {line_idx} logical width"),
        );
    }

    assert_eq!(
        next_byte_start,
        text.len(),
        "{case_name}: line ranges should account for the full source text"
    );

    assert_eq!(
        layout.lines.last().map(|line| line.end_kind),
        Some(LineEndKind::EndOfText),
        "{case_name}: final line should end at end of text"
    );
}

struct ApproxInkBackend;

impl TextBackend for ApproxInkBackend {
    type ShapedGlyphToken = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        20.0
    }

    fn shape_text(
        &mut self,
        text: &str,
        _style: TextStyle,
    ) -> SharedShapedText<Self::ShapedGlyphToken> {
        let glyphs = vec![ShapedGlyph {
            token: 1,
            x: 2.0,
            y: -12.0,
            advance: 30.0,
            approx_ink_bounds: Rect::new(-4.0, 3.0, 18.0, 10.0),
        }];
        std::rc::Rc::new(ShapedText {
            clusters: vec![ShapedCluster {
                byte_start: 0,
                byte_end: text.len(),
                advance: 30.0,
                is_whitespace: false,
                approx_ink_bounds: cluster_approx_ink_bounds(&glyphs),
                glyphs,
            }],
        })
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphToken>,
    ) -> Option<DrawGlyph> {
        Some(DrawGlyph {
            token: PreparedGlyphToken(request.glyph as u64),
            top_left: request.glyph_origin,
        })
    }
}

struct CardTextBackend {
    line_height: f32,
}

impl CardTextBackend {
    fn glyph_width(ch: char) -> f32 {
        match ch {
            '\u{0301}' | '\n' => 0.0,
            '\t' => 12.2,
            '…' => 13.0,
            _ => 6.1,
        }
    }
}

impl TextBackend for CardTextBackend {
    type ShapedGlyphToken = u32;

    fn line_height(&mut self, _style: TextStyle) -> f32 {
        self.line_height
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
            let is_whitespace = ch.is_whitespace();
            let glyphs = if is_whitespace {
                Vec::new()
            } else {
                vec![ShapedGlyph {
                    token: ch as u32,
                    x: 0.0,
                    y: 0.0,
                    advance,
                    approx_ink_bounds: Rect::new(0.0, -style.size, advance, self.line_height),
                }]
            };
            clusters.push(ShapedCluster {
                byte_start,
                byte_end,
                advance,
                is_whitespace,
                approx_ink_bounds: cluster_approx_ink_bounds(&glyphs),
                glyphs,
            });
        }

        std::rc::Rc::new(ShapedText { clusters })
    }

    fn prepare_glyph(
        &mut self,
        request: PrepareGlyphRequest<Self::ShapedGlyphToken>,
    ) -> Option<DrawGlyph> {
        if char::from_u32(request.glyph).is_some_and(char::is_whitespace) {
            return None;
        }

        Some(DrawGlyph {
            token: PreparedGlyphToken(request.glyph as u64),
            top_left: request.glyph_origin,
        })
    }
}

fn card_layout(text: &str, flow: TextFlow, bounds: TextBounds) -> TextLayout<u32> {
    let mut backend = CardTextBackend { line_height: 15.0 };
    layout_text(&mut backend, text, style(flow), bounds)
}

fn card_layout_with_line_height(
    text: &str,
    flow: TextFlow,
    bounds: TextBounds,
    line_height: f32,
) -> TextLayout<u32> {
    let mut backend = CardTextBackend { line_height };
    layout_text(&mut backend, text, style(flow), bounds)
}

fn card_label_glyph_counts_by_line(
    text: &str,
    flow: TextFlow,
    rect: Rect,
    line_height: f32,
) -> Vec<usize> {
    let mut backend = CardTextBackend { line_height };
    let mut cmds = DrawCommands::new();
    raw_label::post_layout_label(
        raw_label::LabelSpec {
            layer: Layer::default(),
            rect,
            text,
            style: LabelStyle {
                text_style: style(flow),
                content_placement: TextContentPlacement::TOP_LEFT,
                text_color: Color::BLACK,
                rule: false,
                rule_color: Color::BLACK,
            },
        },
        raw_label::LabelPreLayoutResult {
            size_request: crate::layout::SizeRequest::UNKNOWN,
        },
        &mut backend,
        &mut cmds,
    );

    let mut lines = Vec::<(f32, usize)>::new();
    for glyph in cmds.glyphs() {
        if let Some((_, count)) = lines
            .iter_mut()
            .find(|(y, _)| (*y - glyph.top_left.y).abs() < 0.5)
        {
            *count += 1;
        } else {
            lines.push((glyph.top_left.y, 1));
        }
    }
    lines.into_iter().map(|(_, count)| count).collect()
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
    assert_eq!(CaretPosition::EmptyText.insertion_byte_hint(), 0);
}

#[test]
fn caret_position_insertion_byte_hint_uses_stored_bytes() {
    assert_eq!(
        CaretPosition::BeforeCluster {
            cluster_byte_start: 4,
        }
        .insertion_byte_hint(),
        4
    );
    assert_eq!(
        CaretPosition::AfterCluster {
            cluster_byte_start: 4,
            cluster_byte_end: 9,
        }
        .insertion_byte_hint(),
        9
    );
    assert_eq!(CaretPosition::EmptyText.insertion_byte_hint(), 0);

    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);
    let x_byte = text.find('x').unwrap();

    assert_eq!(
        layout
            .caret_position_at_insertion_byte(x_byte)
            .insertion_byte_hint(),
        x_byte,
        "a caret after the multi-byte combining cluster should carry its byte end"
    );
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
    assert_eq!(layout.lines[1].clusters[0].x, 0.0);
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
    assert_eq!(layout.lines[1].clusters[0].x, 0.0);
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
        assert_eq!(layout.resolved_glyphs().len(), 0);
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
        assert_eq!(total_clusters(&layout), 1);
        assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    }
}

#[test]
fn design_wrapping_examples_width_5() {
    for case in [
        WrapDiagramCase {
            name: "single boundary space",
            width_cols: 5,
            input: "hello world",
            expected: &["hello~", "world"],
        },
        WrapDiagramCase {
            name: "two spaces",
            width_cols: 5,
            input: "hello  world",
            expected: &["hello~", " ", "world"],
        },
        WrapDiagramCase {
            name: "three spaces",
            width_cols: 5,
            input: "hello   world",
            expected: &["hello~", "  ", "world"],
        },
        WrapDiagramCase {
            name: "trailing space",
            width_cols: 5,
            input: "hello ",
            expected: &["hello~", ""],
        },
        WrapDiagramCase {
            name: "two trailing spaces",
            width_cols: 5,
            input: "hello  ",
            expected: &["hello~", " "],
        },
        WrapDiagramCase {
            name: "five leading spaces",
            width_cols: 5,
            input: "     hello",
            expected: &["     ", "hello"],
        },
        WrapDiagramCase {
            name: "six leading spaces",
            width_cols: 5,
            input: "      hello",
            expected: &["     ~", "hello"],
        },
        WrapDiagramCase {
            name: "hard newline",
            width_cols: 5,
            input: "hello\nworld",
            expected: &["hello", "world"],
        },
        WrapDiagramCase {
            name: "double hard newline",
            width_cols: 5,
            input: "hello\n\nworld",
            expected: &["hello", "", "world"],
        },
    ] {
        assert_wrap_diagram(case);
    }
}

#[test]
fn design_wrapping_examples_width_6() {
    for case in [
        WrapDiagramCase {
            name: "width 6 single boundary space",
            width_cols: 6,
            input: "hello world",
            expected: &["hello~", "world"],
        },
        WrapDiagramCase {
            name: "width 6 double space before word",
            width_cols: 6,
            input: "hello  world",
            expected: &["hello ~", "world"],
        },
    ] {
        assert_wrap_diagram(case);
    }
}

#[test]
fn caret_advances_along_single_line() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout
            .caret_geom(CaretPosition::BeforeCluster {
                cluster_byte_start: 0
            })
            .x,
        0.0
    );
    assert_eq!(
        layout
            .caret_geom(CaretPosition::BeforeCluster {
                cluster_byte_start: 1
            })
            .x,
        8.0
    );
    assert_eq!(
        layout
            .caret_geom(CaretPosition::AfterCluster {
                cluster_byte_start: 2,
                cluster_byte_end: 3,
            })
            .x,
        24.0
    );
}

#[test]
fn caret_positions_distinguish_hard_newline_sides() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_start: 1,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_start: 1,
        cluster_byte_end: 2,
    });

    assert_eq!(before.y_top, 0.0);
    assert_eq!(before.x, 8.0);
    assert_eq!(after.y_top, 16.0);
    assert_eq!(after.x, 0.0);
    assert_eq!(
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }
        .insertion_byte_hint(),
        1
    );
    assert_eq!(
        CaretPosition::AfterCluster {
            cluster_byte_start: 1,
            cluster_byte_end: 2,
        }
        .insertion_byte_hint(),
        2
    );
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }
    );
}

#[test]
fn caret_navigation_hard_newline_moves_between_source_distinct_sides() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 0,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }
    );
    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_start: 1,
            cluster_byte_end: 2,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 2,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }
    );
    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_start: 1,
            cluster_byte_end: 2,
        }
    );
}

#[test]
fn caret_positions_distinguish_collapsed_soft_wrap_space_sides() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_start: 5,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_start: 5,
        cluster_byte_end: 6,
    });

    assert_eq!(before.y_top, 0.0);
    assert_eq!(before.x, 40.0);
    assert_eq!(after.y_top, 16.0);
    assert_eq!(after.x, 0.0);
    assert_eq!(
        CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }
        .insertion_byte_hint(),
        5
    );
    assert_eq!(
        CaretPosition::AfterCluster {
            cluster_byte_start: 5,
            cluster_byte_end: 6,
        }
        .insertion_byte_hint(),
        6
    );
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }
    );
}

#[test]
fn caret_positions_distinguish_mid_word_soft_wrap_visual_affinity() {
    let layout = layout("abcde", wrap_word_cluster_drop(), TextBounds::width(16.1));

    let previous_line_end = CaretPosition::AfterCluster {
        cluster_byte_start: 1,
        cluster_byte_end: 2,
    };
    let next_line_start = CaretPosition::BeforeCluster {
        cluster_byte_start: 2,
    };
    let previous_geom = layout.caret_geom(previous_line_end);
    let next_geom = layout.caret_geom(next_line_start);

    assert_eq!(previous_line_end.insertion_byte_hint(), 2);
    assert_eq!(next_line_start.insertion_byte_hint(), 2);
    assert_ne!(previous_geom, next_geom);
    assert_eq!(previous_geom.y_top, 0.0);
    assert_eq!(previous_geom.x, 16.0);
    assert_eq!(next_geom.y_top, 16.0);
    assert_eq!(next_geom.x, 0.0);
    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 1.0)),
        previous_line_end
    );
    assert_eq!(layout.hit_test_caret(Vec2::new(0.0, 17.0)), next_line_start);
}

#[test]
fn visual_line_carets_mid_word_soft_wrap_use_previous_end_and_next_start_affinity() {
    let layout = layout("abcde", wrap_word_cluster_drop(), TextBounds::width(16.1));

    let previous_line_end = CaretPosition::AfterCluster {
        cluster_byte_start: 1,
        cluster_byte_end: 2,
    };
    let next_line_start = CaretPosition::BeforeCluster {
        cluster_byte_start: 2,
    };

    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapNonWhitespace);
    assert_eq!(
        layout.caret_at_visual_line_start(0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0
        }
    );
    assert_eq!(layout.caret_at_visual_line_end(0), previous_line_end);
    assert_eq!(layout.caret_at_visual_line_start(1), next_line_start);
    assert_eq!(layout.visual_line_index_for_caret(previous_line_end), 0);
    assert_eq!(layout.visual_line_index_for_caret(next_line_start), 1);
    assert_eq!(
        previous_line_end.insertion_byte_hint(),
        next_line_start.insertion_byte_hint()
    );
}

#[test]
fn visual_line_carets_collapsed_whitespace_soft_wrap_use_boundary_sides() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    let previous_line_end = CaretPosition::BeforeCluster {
        cluster_byte_start: 5,
    };
    let next_line_start = CaretPosition::BeforeCluster {
        cluster_byte_start: 6,
    };

    assert_eq!(layout.lines[0].end_kind, LineEndKind::SoftWrapWhitespace);

    // caret_at_visual_line_end for the previous line still returns BeforeCluster(wrapped_whitespace_boundary)
    assert_eq!(layout.caret_at_visual_line_end(0), previous_line_end);

    // caret_at_visual_line_start after collapsed soft-wrap whitespace returns BeforeCluster(first cluster on following line)
    assert_eq!(layout.caret_at_visual_line_start(1), next_line_start);

    // caret_at_visual_line_x(line, x_at_or_before_start) returns the same Home-style start caret.
    assert_eq!(layout.caret_at_visual_line_x(1, 0.0), next_line_start);
    assert_eq!(layout.caret_at_visual_line_x(1, -10.0), next_line_start);

    assert_eq!(layout.visual_line_index_for_caret(previous_line_end), 0);
    assert_eq!(layout.visual_line_index_for_caret(next_line_start), 1);
    assert_ne!(
        previous_line_end.insertion_byte_hint(),
        next_line_start.insertion_byte_hint()
    );
}

#[test]
fn visual_line_carets_hard_newline_use_boundary_sides() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let previous_line_end = CaretPosition::BeforeCluster {
        cluster_byte_start: 1,
    };
    let next_line_start = CaretPosition::BeforeCluster {
        cluster_byte_start: 2,
    };

    assert_eq!(layout.lines[0].end_kind, LineEndKind::HardNewline);

    // caret_at_visual_line_end for the previous line still returns BeforeCluster(newline_boundary)
    assert_eq!(layout.caret_at_visual_line_end(0), previous_line_end);

    // caret_at_visual_line_start after a hard newline returns BeforeCluster(first cluster on following line)
    assert_eq!(layout.caret_at_visual_line_start(1), next_line_start);

    // caret_at_visual_line_x(line, x_at_or_before_start) returns the same Home-style start caret.
    assert_eq!(layout.caret_at_visual_line_x(1, 0.0), next_line_start);
    assert_eq!(layout.caret_at_visual_line_x(1, -10.0), next_line_start);

    assert_eq!(layout.visual_line_index_for_caret(previous_line_end), 0);
    assert_eq!(layout.visual_line_index_for_caret(next_line_start), 1);
}

#[test]
fn visual_line_carets_single_unwrapped_line_use_text_edges() {
    let layout = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let start = CaretPosition::BeforeCluster {
        cluster_byte_start: 0,
    };
    let end = CaretPosition::AfterCluster {
        cluster_byte_start: 2,
        cluster_byte_end: 3,
    };

    assert_eq!(layout.caret_at_visual_line_start(0), start);
    assert_eq!(layout.caret_at_visual_line_end(0), end);
    assert_eq!(layout.visual_line_index_for_caret(start), 0);
    assert_eq!(layout.visual_line_index_for_caret(end), 0);
}

#[test]
fn visual_line_carets_empty_text_use_empty_caret() {
    let layout = layout("", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_at_visual_line_start(0),
        CaretPosition::EmptyText
    );
    assert_eq!(layout.caret_at_visual_line_end(0), CaretPosition::EmptyText);
    assert_eq!(
        layout.visual_line_index_for_caret(CaretPosition::EmptyText),
        0
    );
}

#[test]
fn caret_navigation_mid_word_soft_wrap_moves_between_insertion_positions() {
    let layout = layout("abcde", wrap_word_cluster_drop(), TextBounds::width(16.1));

    let previous_line_end = CaretPosition::AfterCluster {
        cluster_byte_start: 1,
        cluster_byte_end: 2,
    };
    let next_line_start = CaretPosition::BeforeCluster {
        cluster_byte_start: 2,
    };

    assert_eq!(
        layout.previous_caret_position(next_line_start),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
        }
    );
    assert_eq!(
        layout.next_caret_position(previous_line_end),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 3,
        }
    );
}

#[test]
fn caret_position_at_insertion_byte_uses_following_anchor_for_collapsed_soft_wrap_space() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(
        CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }
        .insertion_byte_hint(),
        5
    );
    assert_eq!(
        CaretPosition::AfterCluster {
            cluster_byte_start: 5,
            cluster_byte_end: 6,
        }
        .insertion_byte_hint(),
        6
    );
    assert_eq!(
        layout.caret_position_at_insertion_byte(6),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 6,
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
        assert_close(first_cluster(&layout).x, expected_x, "cluster x");
    }
}

#[test]
fn unbounded_horizontal_alignment_uses_natural_block_width() {
    let text = "a\nabcd";

    for (align, expected_first_x, expected_second_x) in [
        (TextLineAlign::Start, 0.0, 0.0),
        (TextLineAlign::Center, 12.0, 0.0),
        (TextLineAlign::End, 24.0, 0.0),
    ] {
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: align,
        };
        let layout = layout(text, flow, TextBounds::UNBOUNDED);

        assert_close(
            layout.lines[0].logical_x,
            expected_first_x,
            "short line logical_x",
        );
        assert_close(
            first_cluster(&layout).x,
            expected_first_x,
            "short line cluster x",
        );
        assert_close(
            layout.lines[1].logical_x,
            expected_second_x,
            "widest line logical_x",
        );
        assert_close(
            layout.lines[1].clusters[0].x,
            expected_second_x,
            "widest line cluster x",
        );
        assert_close(layout.metrics().logical_size.x, 32.0, "natural block width");
    }
}

#[test]
fn unbounded_alignment_handles_empty_text_and_empty_lines() {
    for align in [
        TextLineAlign::Start,
        TextLineAlign::Center,
        TextLineAlign::End,
    ] {
        let flow = TextFlow {
            overflow_x: OverflowX::Keep,
            overflow_y: OverflowY::Keep,
            line_align: align,
        };

        let empty = layout("", flow, TextBounds::UNBOUNDED);
        assert_close(empty.metrics().logical_size.x, 0.0, "empty width");
        assert_close(empty.lines[0].logical_x, 0.0, "empty line x");
        assert!(empty.lines[0].logical_x.is_finite());

        let with_empty_line = layout("abcd\n\nx", flow, TextBounds::UNBOUNDED);
        for line in &with_empty_line.lines {
            assert!(line.logical_x.is_finite());
            assert!(line.logical_x >= 0.0);
        }
        assert_close(
            with_empty_line.metrics().logical_size.x,
            32.0,
            "block width with empty line",
        );
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
                    cluster_byte_start: 1,
                    cluster_byte_end: 2,
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
fn resolved_glyphs_match_emitted_layout_origins() {
    let style = style(TextFlow::single_line());
    let mut backend = TestTextBackend;
    let layout = layout_text(&mut backend, "ab", style, TextBounds::UNBOUNDED);
    let origin = Vec2::new(10.0, 20.0);
    let mut commands = DrawCommands::new();

    layout.emit_glyphs(&mut commands, &mut backend, origin, Color::BLACK, 0);

    let resolved = layout.resolved_glyphs();
    assert_eq!(resolved.len(), commands.glyphs().len());
    for (resolved, emitted) in resolved.iter().zip(commands.glyphs()) {
        assert_eq!(
            emitted.top_left,
            Vec2::new(origin.x + resolved.origin.x, origin.y + resolved.origin.y)
        );
    }
}

#[test]
fn visible_glyph_count_excludes_hard_breaks() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.visible_glyph_count, 2);
    assert_visible_glyph_count_matches_resolved(&layout);
}

#[test]
fn visible_glyph_count_excludes_dropped_overflow_clusters() {
    let layout = layout("abcdef", drop_x_drop_y(), TextBounds::width(16.1));

    assert_eq!(visible(&layout), "ab");
    assert_eq!(layout.visible_glyph_count, 2);
    assert_visible_glyph_count_matches_resolved(&layout);
}

#[test]
fn visible_glyph_count_includes_visible_ellipsis_glyphs() {
    let layout = layout(
        "abc",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds::width(16.1),
    );

    assert_eq!(visible(&layout), "a\u{2026}");
    assert_eq!(layout.visible_glyph_count, 2);
    assert_visible_glyph_count_matches_resolved(&layout);
}

#[test]
fn emit_glyphs_omits_empty_runs() {
    let style = style(TextFlow::single_line());
    let mut backend = TestTextBackend;
    let layout = layout_text(&mut backend, "   ", style, TextBounds::UNBOUNDED);
    let mut commands = DrawCommands::new();

    layout.emit_glyphs(&mut commands, &mut backend, Vec2::ZERO, Color::BLACK, 0);

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

    assert_eq!(total_clusters(&layout), text.chars().count());
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
    assert_eq!(prepared.resolved_glyphs().len(), 0);
}

#[test]
fn wrap_cluster_keep_does_not_split_combining_mark_cluster() {
    let text = "e\u{0301}x";
    let layout = layout(text, wrap_cluster_keep(), TextBounds::width(4.0));

    assert_eq!(first_cluster(&layout).byte_start, 0);
    assert_eq!(first_cluster(&layout).byte_end, "e\u{0301}".len());
    assert_eq!(line_visible_glyph_count(&layout, 0), 2);
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
            cluster_byte_start: 0,
            cluster_byte_end: text.find('x').unwrap(),
        }
    );
}

#[test]
fn line_records_own_clusters() {
    let layout = layout("ab cd", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert_eq!(layout.lines.len(), layout.metrics().line_count as usize);
    assert!(layout.lines.iter().all(|line| !line.clusters.is_empty()));
    assert_eq!(total_clusters(&layout), 5);
}

#[test]
fn caret_inside_combining_mark_cluster_clamps_to_cluster_start() {
    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.caret_position_at_insertion_byte(1),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0,
        }
    );
}

#[test]
fn hit_test_combining_mark_cluster_returns_cluster_boundaries() {
    let text = "e\u{0301}x";
    let layout = layout(text, TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout
            .hit_test_caret(Vec2::new(0.0, 0.0))
            .insertion_byte_hint(),
        0
    );
    assert_eq!(
        layout
            .hit_test_caret(Vec2::new(7.9, 0.0))
            .insertion_byte_hint(),
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
fn layout_text_metrics_reports_backend_approx_ink_bounds() {
    let mut backend = ApproxInkBackend;
    let layout = layout_text(
        &mut backend,
        "x",
        style(TextFlow::single_line()),
        TextBounds::UNBOUNDED,
    );
    let metrics = layout.metrics();

    assert_eq!(metrics.logical_size, Vec2::new(30.0, 20.0));
    assert_eq!(metrics.approx_ink_bounds, Rect::new(-2.0, 3.0, 18.0, 10.0));
    assert_eq!(metrics.lines[0].approx_ink_x, -2.0);
    assert_eq!(metrics.lines[0].approx_ink_width, 18.0);
}

#[test]
fn metrics_report_whitespace_logical_advance_without_ink() {
    let layout = layout("   ", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(layout.metrics().logical_size.x, 24.0);
    assert_eq!(layout.resolved_glyphs().len(), 0);
    assert_eq!(layout.metrics().approx_ink_bounds, Rect::ZERO);
}

#[test]
fn drop_overflow_uses_logical_advance_not_ink_width_for_single_glyph() {
    let layout = layout("a", TextFlow::single_line(), TextBounds::width(7.0));

    assert_eq!(layout.resolved_glyphs().len(), 0);
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
                cluster_byte_start: 1,
                cluster_byte_end: 2,
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
                cluster_byte_start: 2,
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
            cluster_byte_start: 1,
            cluster_byte_end: 2,
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
            cluster_byte_start: 1,
        }
    );
}

#[test]
fn hit_test_right_of_blank_newline_line_stays_on_same_line() {
    let layout = layout("\n\n", TextFlow::single_line(), TextBounds::UNBOUNDED);

    assert_eq!(
        layout.hit_test_caret(Vec2::new(100.0, 17.0)),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1,
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
        cluster_byte_start: 1,
    });
    assert_eq!(caret.y_top, 0.0);
    assert_eq!(caret.x, 8.0);
}

#[test]
fn caret_geom_after_newline_index_is_on_next_line() {
    let layout = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);

    let caret = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_start: 1,
        cluster_byte_end: 2,
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
            cluster_byte_start: 0,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: x_byte,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::AfterCluster {
            cluster_byte_start: x_byte,
            cluster_byte_end: x_byte + 1,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: x_byte,
        }
    );
}

#[test]
fn caret_geom_collapsed_soft_wrap_space_uses_previous_and_following_lines() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    let before = layout.caret_geom(CaretPosition::BeforeCluster {
        cluster_byte_start: 5,
    });
    let after = layout.caret_geom(CaretPosition::AfterCluster {
        cluster_byte_start: 5,
        cluster_byte_end: 6,
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
fn caret_navigation_collapsed_soft_wrap_space_moves_between_source_distinct_sides() {
    let layout = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );

    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 4,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }
    );
    assert_eq!(
        layout.previous_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 6,
        }),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }
    );
    assert_eq!(
        layout.next_caret_position(CaretPosition::BeforeCluster {
            cluster_byte_start: 5,
        }),
        CaretPosition::AfterCluster {
            cluster_byte_start: 5,
            cluster_byte_end: 6,
        }
    );
}

#[test]
fn test_overflow_x_drop_y_drop() {
    let layout = card_layout(
        "hello\nhello",
        drop_x_drop_y(),
        TextBounds {
            max_width: Some(25.0),
            max_height: Some(28.0),
        },
    );

    // Keep this test in sync with Card 1 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 1);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::OverflowDrop);
    assert_eq!(visual_lines(&layout), ["hell"]);
    for glyph in layout.resolved_glyphs() {
        assert!(glyph.origin.x + glyph.advance <= 25.0 + 0.1);
    }
    assert!(!layout.resolved_glyphs().is_empty());
    assert!(layout.metrics().truncated_horizontal);
    assert!(layout.metrics().truncated_vertical);
}

#[test]
fn test_overflow_x_keep_y_keep() {
    let layout = card_layout(
        "hello\nhello",
        keep_x_keep_y(),
        TextBounds {
            max_width: Some(25.0),
            max_height: Some(28.0),
        },
    );

    // Keep this test in sync with Card 2 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 2);
    let line_has_overflow = |line_idx| {
        layout
            .iter_resolved_line_glyphs(line_idx)
            .any(|glyph| glyph.origin.x + glyph.advance > 25.0 + 0.1)
    };
    assert!(line_has_overflow(0));
    assert!(line_has_overflow(1));
    assert_eq!(visual_lines(&layout), ["hello", "hello"]);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::OverflowKeep);
    assert_eq!(layout.lines[1].end_kind, LineEndKind::OverflowKeep);
}

#[test]
fn test_overflow_x_keep_y_ellipsis() {
    let layout = card_layout(
        "hello\nhello",
        keep_x_ellipsis_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(25.0),
            max_height: Some(28.0),
        },
    );

    // Keep this test in sync with Card 3 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 1);
    assert!(visible(&layout).ends_with('…'));
    assert_eq!(visual_lines(&layout), ["h…"]);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisY);
    let last = layout.resolved_glyphs().last().copied().unwrap();
    assert!(last.origin.x + last.advance <= 25.0 + 0.1);
    assert!(layout.metrics().truncated_vertical);
}

#[test]
fn test_overflow_x_keep_y_ellipsis_fallback_drop() {
    let layout = card_layout(
        "hello\nhello",
        keep_x_ellipsis_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(8.0),
            max_height: Some(28.0),
        },
    );

    // Keep this test in sync with Card 4 in Section 4 of sample/src/label_page.rs.
    assert!(layout.metrics().truncated_vertical);
    assert_eq!(layout.resolved_glyphs().len(), 0);
    assert_eq!(visual_lines(&layout), [""]);
}

#[test]
fn test_overflow_x_keep_y_ellipsis_fallback_keep() {
    let layout = card_layout(
        "hello\nhello",
        keep_x_ellipsis_y(EllipsisFallback::Keep),
        TextBounds {
            max_width: Some(8.0),
            max_height: Some(28.0),
        },
    );

    // Keep this test in sync with Card 5 in Section 4 of sample/src/label_page.rs.
    assert!(layout.metrics().truncated_vertical);
    assert_eq!(visible(&layout), "…");
    assert_eq!(visual_lines(&layout), ["…"]);
    let last = layout.resolved_glyphs().last().copied().unwrap();
    assert!(last.origin.x + last.advance > 8.0 + 0.1);
}

#[test]
fn test_overflow_x_ellipsis_y_keep() {
    let layout = card_layout(
        "hello\nhello",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(23.0),
            max_height: Some(48.0),
        },
    );

    // Keep this test in sync with Card 6 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 2);
    assert!(visible(&layout).contains('…'));
    assert_eq!(visual_lines(&layout), ["h…", "h…"]);
    assert_eq!(layout.lines[0].end_kind, LineEndKind::EllipsisX);
    assert_eq!(layout.lines[1].end_kind, LineEndKind::EllipsisX);
    for line_idx in 0..layout.lines.len() {
        let last = layout.iter_resolved_line_glyphs(line_idx).last().unwrap();
        assert!(last.origin.x + last.advance <= 23.0 + 0.1);
    }
}

#[test]
fn test_overflow_x_ellipsis_fallback_drop_y_keep() {
    let layout = card_layout(
        "hello\nhello",
        ellipsis_x_keep_y(EllipsisFallback::Drop),
        TextBounds {
            max_width: Some(8.0),
            max_height: Some(48.0),
        },
    );

    // Keep this test in sync with Card 7 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.resolved_glyphs().len(), 0);
    assert_eq!(visual_lines(&layout), ["", ""]);
}

#[test]
fn test_overflow_x_ellipsis_fallback_keep_y_keep() {
    let layout = card_layout(
        "hello\nhello",
        ellipsis_x_keep_y(EllipsisFallback::Keep),
        TextBounds {
            max_width: Some(8.0),
            max_height: Some(48.0),
        },
    );

    // Keep this test in sync with Card 8 in Section 4 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 2);
    assert_eq!(visible(&layout), "……");
    assert_eq!(visual_lines(&layout), ["…", "…"]);
    for line_idx in 0..layout.lines.len() {
        let last = layout.iter_resolved_line_glyphs(line_idx).last().unwrap();
        assert!(last.origin.x + last.advance > 8.0 + 0.1);
    }
    let glyphs = layout.resolved_glyphs();
    assert!(glyphs[1].origin.y > glyphs[0].origin.y + 10.0);
}

#[test]
fn test_wrap_cluster_y_keep() {
    let text = "hello\nhello";
    let layout = card_layout(
        text,
        wrap_cluster_drop(),
        TextBounds {
            max_width: Some(23.0),
            max_height: Some(63.0),
        },
    );

    // Keep this test in sync with Card 1 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 4);
    assert_eq!(visible(&layout), "hellohello");
    assert_eq!(visual_lines(&layout), ["hel", "lo", "hel", "lo"]);
    assert_eq!(
        visual_line_sources(text, &layout),
        ["hel", "lo\n", "hel", "lo"]
    );
    assert_eq!(
        layout
            .lines
            .iter()
            .map(|line| line.end_kind)
            .collect::<Vec<_>>(),
        [
            LineEndKind::SoftWrapNonWhitespace,
            LineEndKind::HardNewline,
            LineEndKind::SoftWrapNonWhitespace,
            LineEndKind::EndOfText,
        ]
    );
}

#[test]
fn test_wrap_cluster_fallback_drop_y_keep() {
    let layout = card_layout(
        "hello\nhello",
        wrap_cluster_drop(),
        TextBounds {
            max_width: Some(6.0),
            max_height: Some(68.0),
        },
    );

    // Keep this test in sync with Card 2 in Section 4.1 of sample/src/label_page.rs.
    assert!(visible(&layout).trim().is_empty());
    assert_eq!(visual_lines(&layout), [""]);
    assert_eq!(layout.metrics().logical_size.x, 0.0);
}

#[test]
fn test_wrap_cluster_fallback_keep_y_keep() {
    let text = "hello\nhello";
    let layout = card_layout(
        text,
        wrap_cluster_keep(),
        TextBounds {
            max_width: Some(4.0),
            max_height: Some(162.0),
        },
    );

    // Keep this test in sync with Card 3 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 10);
    assert_eq!(visible(&layout), "hellohello");
    assert_eq!(
        visual_lines(&layout),
        ["h", "e", "l", "l", "o", "h", "e", "l", "l", "o"]
    );
    assert_eq!(
        visual_line_sources(text, &layout),
        ["h", "e", "l", "l", "o\n", "h", "e", "l", "l", "o"]
    );
    for line_idx in 0..layout.lines.len() {
        assert!(line_visible_glyph_count(&layout, line_idx) <= 1);
    }
}

#[test]
fn label_wrap_cluster_fallback_keep_uses_widget_width() {
    assert_eq!(
        card_label_glyph_counts_by_line(
            "hello\nhello",
            wrap_cluster_keep(),
            Rect::new(0.0, 0.0, 4.0, 162.0),
            15.0,
        ),
        vec![1; 10]
    );
}

#[test]
fn test_wrap_word_y_keep() {
    let text = "hello there\nhello there";
    let layout = card_layout(
        text,
        wrap_word_drop(),
        TextBounds {
            max_width: Some(48.0),
            max_height: Some(68.0),
        },
    );

    // Keep this test in sync with Card 4 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 4);
    assert_eq!(visible(&layout), "hellotherehellothere");
    assert_eq!(visual_lines(&layout), ["hello", "there", "hello", "there"]);
    assert_eq!(
        visual_line_sources(text, &layout),
        ["hello ", "there\n", "hello ", "there"]
    );
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_y_keep() {
    let text = "hello there\nhello there";
    let layout = card_layout(
        text,
        wrap_word_cluster_drop(),
        TextBounds {
            max_width: Some(23.0),
            max_height: Some(138.0),
        },
    );

    // Keep this test in sync with Card 5 in Section 4.1 of sample/src/label_page.rs.
    assert!(layout.lines.len() > 4);
    assert_eq!(visible(&layout), "hellotherehellothere");
    assert_eq!(
        visual_lines(&layout),
        ["hel", "lo", "the", "re", "hel", "lo", "the", "re"]
    );
    assert_eq!(
        visual_line_sources(text, &layout),
        ["hel", "lo ", "the", "re\n", "hel", "lo ", "the", "re"]
    );
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_fallback_drop_y_keep() {
    let layout = card_layout(
        "hello there\nhello there",
        wrap_word_cluster_drop(),
        TextBounds {
            max_width: Some(6.0),
            max_height: Some(138.0),
        },
    );

    // Keep this test in sync with Card 6 in Section 4.1 of sample/src/label_page.rs.
    assert!(visible(&layout).trim().is_empty());
    assert_eq!(visual_lines(&layout), [""]);
}

#[test]
fn test_wrap_word_fallback_wrap_cluster_fallback_keep_y_keep() {
    let text = "hello there\nhello there";
    let layout = card_layout_with_line_height(
        text,
        wrap_word_cluster_keep(),
        TextBounds {
            max_width: Some(4.0),
            max_height: Some(318.0),
        },
        17.0,
    );

    // Keep this test in sync with Card 7 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(visible(&layout), "hellotherehellother");
    assert_eq!(layout.lines.len(), 19);
    assert_eq!(
        visual_lines(&layout),
        [
            "h", "e", "l", "l", "o", "t", "h", "e", "r", "e", "h", "e", "l", "l", "o", "t", "h",
            "e", "r"
        ]
    );
    assert_eq!(
        visual_line_sources(text, &layout),
        [
            "h", "e", "l", "l", "o ", "t", "h", "e", "r", "e\n", "h", "e", "l", "l", "o ", "t",
            "h", "e", "r"
        ]
    );
    for line_idx in 0..layout.lines.len() {
        assert!(line_visible_glyph_count(&layout, line_idx) <= 1);
    }
}

#[test]
fn oversized_word_cluster_fallback_drop_stops_after_unfittable_cluster() {
    let layout = layout("abc", wrap_word_cluster_drop(), TextBounds::width(0.0));

    assert_eq!(visual_lines(&layout), [""]);
    assert_line_ranges(&layout, &[(0, 3)]);
    assert_eq!(total_clusters(&layout), 0);
}

#[test]
fn oversized_word_fallback_wrapping_diagrams() {
    for case in [
        WrapDiagramCase {
            name: "oversized word fallback open tail accepts following space",
            width_cols: 2,
            input: "abcde f",
            expected: &["ab", "cd", "e~", "f"],
        },
        WrapDiagramCase {
            name: "oversized word fallback terminal space creates empty line",
            width_cols: 2,
            input: "abcdef ",
            expected: &["ab", "cd", "ef~", ""],
        },
    ] {
        assert_wrap_diagram(case);
    }
}

#[test]
fn oversized_word_cluster_fallback_drop_splits_at_byte_ranges() {
    let layout = layout("abcdef", wrap_word_cluster_drop(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
    assert_line_ranges(&layout, &[(0, 2), (2, 4), (4, 6)]);
}

#[test]
fn oversized_word_cluster_fallback_keep_splits_at_byte_ranges() {
    let layout = layout("abcdef", wrap_word_cluster_keep(), TextBounds::width(16.1));

    assert_eq!(visual_lines(&layout), ["ab", "cd", "ef"]);
    assert_line_ranges(&layout, &[(0, 2), (2, 4), (4, 6)]);
}

#[test]
fn label_wrap_word_cluster_keep_uses_widget_width() {
    assert_eq!(
        card_label_glyph_counts_by_line(
            "hello there\nhello there",
            wrap_word_cluster_keep(),
            Rect::new(0.0, 0.0, 4.0, 318.0),
            17.0,
        ),
        vec![1; 19]
    );
}

#[test]
fn test_wrap_word_fallback_drop_y_keep() {
    let layout = card_layout(
        "hello there\nhello there",
        wrap_word_drop(),
        TextBounds {
            max_width: Some(25.0),
            max_height: Some(68.0),
        },
    );

    // Keep this test in sync with Card 8 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 2);
    assert_eq!(visual_lines(&layout), ["hell", "hell"]);
    for glyph in layout.resolved_glyphs() {
        assert!(glyph.origin.x + glyph.advance <= 25.0 + 0.1);
    }
}

#[test]
fn test_wrap_word_fallback_keep_y_keep() {
    let layout = card_layout(
        "hello there\nhello there",
        wrap_word_keep(),
        TextBounds {
            max_width: Some(25.0),
            max_height: Some(68.0),
        },
    );

    // Keep this test in sync with Card 9 in Section 4.1 of sample/src/label_page.rs.
    assert_eq!(layout.lines.len(), 2);
    assert_eq!(visual_lines(&layout), ["hello", "hello"]);
    assert!(layout
        .resolved_glyphs()
        .iter()
        .any(|glyph| glyph.origin.x + glyph.advance > 25.0 + 0.1));
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
fn right_aligned_soft_wrap_excludes_fitted_boundary_space_from_line_width() {
    let mut flow = wrap_word_cluster_drop();
    flow.line_align = TextLineAlign::End;
    let wrap_w = line_width("hello ", flow) + 0.1;
    let layout = layout("hello world", flow, TextBounds::width(wrap_w));

    assert_eq!(layout.lines[0].logical_width, line_width("hello", flow));
    assert!(layout.lines[0].logical_x > 0.0);
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
    assert!(
        layout.lines[0]
            .clusters
            .last()
            .unwrap()
            .is_soft_wrap_boundary
    );
}

#[test]
fn whitespace_that_wraps_from_non_empty_line_does_not_use_wrap_cluster_keep_fallback() {
    let layout = layout("hello ", wrap_cluster_keep(), TextBounds::width(40.1));

    assert_eq!(layout.metrics().line_count, 2);
    assert_eq!(layout.lines[0].logical_width, 40.0);
    assert!(
        layout.lines[0]
            .clusters
            .last()
            .unwrap()
            .is_soft_wrap_boundary
    );
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
    assert_eq!(position.insertion_byte_hint(), 0);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_drop_fallback() {
    let layout = layout(" ", wrap_word_cluster_drop(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 0.0);
    assert_eq!(total_clusters(&layout), 0);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_cluster_keep_fallback() {
    let layout = layout(" ", wrap_word_cluster_keep(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    assert_eq!(total_clusters(&layout), 1);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_drop_fallback() {
    let layout = layout(" ", wrap_word_drop(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 0.0);
    assert_eq!(total_clusters(&layout), 0);
}

#[test]
fn overwide_whitespace_on_empty_line_uses_wrap_word_keep_fallback() {
    let layout = layout(" ", wrap_word_keep(), TextBounds::width(1.0));

    assert_eq!(layout.metrics().line_count, 1);
    assert_eq!(layout.metrics().lines[0].logical_width, 8.0);
    assert_eq!(total_clusters(&layout), 1);
}

#[test]
fn test_line_metrics_horizontal_alignment() {
    horizontal_alignment_affects_line_offsets();
}

#[test]
fn test_caret_geom_alignment_empty_lines_and_empty_text() {
    caret_geom_alignment_empty_lines_and_empty_text();
}

#[test]
fn test_caret_at_visual_line_x_scenarios() {
    // 1. x before, 2. x after, 3. x in middle
    let layout1 = layout("abc", TextFlow::single_line(), TextBounds::UNBOUNDED);

    // x before start -> first caret position (BeforeCluster(0) at x=0.0)
    assert_eq!(
        layout1.caret_at_visual_line_x(0, -10.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0
        }
    );
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 0.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0
        }
    );

    // x after end -> last caret position (AfterCluster(2, 3) at x=24.0)
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 30.0),
        CaretPosition::AfterCluster {
            cluster_byte_start: 2,
            cluster_byte_end: 3
        }
    );
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 24.0),
        CaretPosition::AfterCluster {
            cluster_byte_start: 2,
            cluster_byte_end: 3
        }
    );

    // x in the middle:
    // x=3.9 -> BeforeCluster(0) (dist to 0.0 is 3.9, dist to 8.0 is 4.1)
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 3.9),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 0
        }
    );
    // x=4.1 -> BeforeCluster(1) (dist to 8.0 is 3.9, dist to 0.0 is 4.1)
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 4.1),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1
        }
    );
    // x=12.0 -> BeforeCluster(1) vs BeforeCluster(2). Midpoint between 8 and 16 is 12.0.
    // Ties broken to later candidate -> BeforeCluster(2) at x=16.0
    assert_eq!(
        layout1.caret_at_visual_line_x(0, 12.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 2
        }
    );

    // 4. hard newline boundary
    let layout_hard = layout("a\nb", TextFlow::single_line(), TextBounds::UNBOUNDED);
    // Line 0 is "a\n"
    // Line 1 is "b"
    // x=0 on line 1 clamps to the Home-style visual/content start of that line:
    // before the first cluster on this visual line, not after the newline boundary.
    assert_eq!(
        layout_hard.caret_at_visual_line_x(1, 0.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 2
        } // start of line 1 ('b')
    );
    // caret at x=100 on line 0 (after newline):
    assert_eq!(
        layout_hard.caret_at_visual_line_x(0, 100.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 1
        } // before newline
    );

    // 5. soft-wrap mid-word boundary
    let layout_mid_word = layout("abcde", wrap_word_cluster_drop(), TextBounds::width(16.1));
    // line 0: "ab"
    // line 1: "cde"
    // x=0 on line 1 (the continuation line) returns the leading edge caret on the wrapped continuation:
    assert_eq!(
        layout_mid_word.caret_at_visual_line_x(1, 0.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 2
        } // 'c'
    );

    // 6. soft-wrap collapsed-whitespace boundary
    let layout_whitespace = layout(
        "hello world",
        wrap_word_cluster_drop(),
        TextBounds::width(40.1),
    );
    // line 0: "hello" (with collapsed space)
    // line 1: "world"
    // x=0 on line 1 clamps to the Home-style visual/content start of that line:
    // before the first cluster on this visual line. This is intentionally not the
    // same as the sequential traversal caret after the wrapped whitespace boundary.
    assert_eq!(
        layout_whitespace.caret_at_visual_line_x(1, 0.0),
        CaretPosition::BeforeCluster {
            cluster_byte_start: 6
        } // 'w'
    );
}
