use crate::text::types::{decode_prepared_glyph_token, GlyphSubpixelSlot, SampleGlyphToken};
use crate::text::SampleTextBackend;
use framewise::{
    text::layout_text, Color, DrawCommands, DrawGlyph, FontId, LineHeight, PrepareGlyphRequest,
    Rect, TextBackend, TextBounds, TextFlow, TextLineAlign, TextStyle, Vec2,
};
use std::rc::Rc;

fn measure_text<T: TextBackend>(
    backend: &mut T,
    text: &str,
    style: TextStyle,
    bounds: TextBounds,
) -> framewise::TextMetrics {
    layout_text(backend, text, style, bounds).metrics().clone()
}

fn sys() -> SampleTextBackend {
    SampleTextBackend::new()
}

fn style(font: FontId, size: f32, weight: u16, flow: TextFlow) -> TextStyle {
    TextStyle::new(font, size, weight, flow)
}

fn prepare_first_glyph_at(
    sys: &mut SampleTextBackend,
    text: &str,
    style: TextStyle,
    origin: Vec2,
) -> DrawGlyph {
    let shaped = TextBackend::shape_text(sys, text, style);
    let glyph = shaped.clusters[0].glyphs[0];
    TextBackend::prepare_glyph(
        sys,
        PrepareGlyphRequest {
            glyph: glyph.token,
            glyph_origin: origin,
        },
    )
    .expect("glyph should prepare")
}

#[test]
fn text_backend_shapes_text_and_prepares_glyphs() {
    let mut sys = sys();
    let style = style(FontId(1), 16.0, 500, TextFlow::single_line());

    let shaped = TextBackend::shape_text(&mut sys, "Hi", style);
    assert!(!shaped.clusters.is_empty());
    assert!(TextBackend::line_metrics(&mut sys, style).line_height > 0);

    let glyph = shaped.clusters[0].glyphs[0];
    let prepared = TextBackend::prepare_glyph(
        &mut sys,
        PrepareGlyphRequest {
            glyph: glyph.token,
            glyph_origin: Vec2::new(12.25 + glyph.x, 18.0 + glyph.y),
        },
    );
    assert!(prepared.is_some());
}

#[test]
fn physical_scale_increases_rasterized_glyph_size_but_keeps_top_left_logical() {
    let style = style(FontId(1), 16.0, 500, TextFlow::single_line());
    let origin = Vec2::new(20.25, 40.0);

    let mut one_x = sys();
    let one_x_glyph = prepare_first_glyph_at(&mut one_x, "A", style, origin);
    let (_, _, one_x_w, one_x_h) = decode_prepared_glyph_token(one_x_glyph.token);

    let mut two_x = sys();
    two_x.set_physical_pixels_per_logical_pixel(2.0);
    let two_x_glyph = prepare_first_glyph_at(&mut two_x, "A", style, origin);
    let (_, _, two_x_w, two_x_h) = decode_prepared_glyph_token(two_x_glyph.token);

    assert!(two_x_w as f32 >= one_x_w as f32 * 1.7);
    assert!(two_x_h as f32 >= one_x_h as f32 * 1.7);
    assert!((two_x_glyph.top_left.x - origin.x).abs() < 8.0);
    assert!((two_x_glyph.top_left.y - origin.y).abs() < 24.0);
}

#[test]
fn changing_physical_scale_invalidates_raster_slots_without_clearing_shape_or_glyph_ids() {
    let style = style(FontId(1), 16.0, 500, TextFlow::single_line());
    let mut sys = sys();
    let shaped = TextBackend::shape_text(&mut sys, "A", style);
    let glyph = shaped.clusters[0].glyphs[0];
    let _draw_glyph = TextBackend::prepare_glyph(
        &mut sys,
        PrepareGlyphRequest {
            glyph: glyph.token,
            glyph_origin: Vec2::new(10.0, 20.0),
        },
    )
    .expect("glyph should prepare");

    let shape_cache_len = sys.shape_cache.len();
    let glyph_cache_len = sys.glyph_cache.len();
    let glyph_index_len = sys.glyph_index.len();
    assert!(sys.atlas_data.iter().any(|alpha| *alpha != 0));
    assert!(sys.glyph_cache.iter().any(|cached| {
        cached
            .subpixels
            .iter()
            .any(|slot| matches!(slot, GlyphSubpixelSlot::Loaded(_)))
    }));

    sys.set_physical_pixels_per_logical_pixel(2.0);

    assert_eq!(sys.shape_cache.len(), shape_cache_len);
    assert_eq!(sys.glyph_cache.len(), glyph_cache_len);
    assert_eq!(sys.glyph_index.len(), glyph_index_len);
    assert!(sys.atlas_data.iter().all(|alpha| *alpha == 0));
    assert!(sys.atlas_dirty);
    assert!(sys.glyph_cache.iter().all(|cached| {
        cached
            .subpixels
            .iter()
            .all(|slot| matches!(slot, GlyphSubpixelSlot::Unloaded))
    }));
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
            &mut framewise::DrawCommands::new(1.0),
            &mut sys,
            rect.top_left(),
            framewise::Color::BLACK,
            0,
        );
    }

    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.font_id == 0));
    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.font_id == 1));
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
            &mut framewise::DrawCommands::new(1.0),
            &mut sys,
            rect.top_left(),
            framewise::Color::BLACK,
            0,
        );
    }

    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.weight == 400));
    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.weight == 700));
    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.opsz == 14));
    assert!(sys.glyph_cache.iter().any(|glyph| glyph.key.opsz == 32));
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
        &mut framewise::DrawCommands::new(1.0),
        &mut sys,
        rect.top_left(),
        framewise::Color::BLACK,
        0,
    );

    let keys: Vec<_> = sys
        .glyph_cache
        .iter()
        .map(|glyph| glyph.key)
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
            &mut framewise::DrawCommands::new(1.0),
            &mut sys,
            rect.top_left(),
            framewise::Color::BLACK,
            0,
        );
    }

    assert!(sys
        .glyph_cache
        .iter()
        .any(|glyph| glyph.key.font_id == 0 && glyph.key.weight == 400));
    assert!(sys
        .glyph_cache
        .iter()
        .any(|glyph| glyph.key.font_id == 0 && glyph.key.weight == 700));
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
fn shape_text_cache_hits_for_repeated_text_and_style() {
    let mut sys = sys();
    let style = style(FontId(1), 16.0, 400, TextFlow::single_line());

    let first = TextBackend::shape_text(&mut sys, "cached", style);
    let second = TextBackend::shape_text(&mut sys, "cached", style);

    assert_eq!(first, second);
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(sys.shape_text_run_count, 1);
    assert_eq!(sys.shape_cache.len(), 1);
}

#[test]
fn overflow_ellipsis_uses_shape_text_cache() {
    let mut sys = sys();
    let flow = TextFlow {
        overflow_x: framewise::OverflowX::Ellipsis {
            fallback: framewise::EllipsisFallback::Drop,
        },
        overflow_y: framewise::OverflowY::Keep,
        line_align: framewise::TextLineAlign::Start,
    };
    let style = style(FontId(1), 16.0, 400, flow);

    let first = layout_text(&mut sys, "abcdef", style, TextBounds::width(40.0));
    let second = layout_text(&mut sys, "abcdef", style, TextBounds::width(40.0));

    assert_eq!(
        first.resolved_glyphs().last(),
        second.resolved_glyphs().last()
    );
    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_misses_when_text_changes() {
    let mut sys = sys();
    let style = style(FontId(1), 16.0, 400, TextFlow::single_line());

    TextBackend::shape_text(&mut sys, "cached", style);
    TextBackend::shape_text(&mut sys, "changed", style);

    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_misses_when_font_id_changes() {
    let mut sys = sys();

    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(0), 16.0, 400, TextFlow::single_line()),
    );
    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 400, TextFlow::single_line()),
    );

    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_misses_when_size_changes() {
    let mut sys = sys();

    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 400, TextFlow::single_line()),
    );
    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 18.0, 400, TextFlow::single_line()),
    );

    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_misses_when_weight_changes() {
    let mut sys = sys();

    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 400, TextFlow::single_line()),
    );
    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 700, TextFlow::single_line()),
    );

    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_misses_when_letter_spacing_changes() {
    let mut sys = sys();

    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 400, TextFlow::single_line()),
    );
    TextBackend::shape_text(
        &mut sys,
        "cached",
        style(FontId(1), 16.0, 400, TextFlow::single_line()).with_letter_spacing(0.05),
    );

    assert_eq!(sys.shape_text_run_count, 2);
    assert_eq!(sys.shape_cache.len(), 2);
}

#[test]
fn shape_text_cache_ignores_layout_only_style_fields() {
    let mut sys = sys();
    let mut centered_wrapped = TextFlow::wrapped();
    centered_wrapped.line_align = TextLineAlign::Center;

    let base = style(FontId(1), 16.0, 400, TextFlow::single_line());
    let layout_only_change =
        style(FontId(1), 16.0, 400, centered_wrapped).with_line_height(LineHeight::Relative(2.0));

    let first = TextBackend::shape_text(&mut sys, "cached", base);
    let second = TextBackend::shape_text(&mut sys, "cached", layout_only_change);

    assert_eq!(first, second);
    assert_eq!(sys.shape_text_run_count, 1);
    assert_eq!(sys.shape_cache.len(), 1);
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
    let mut commands = framewise::DrawCommands::new(1.0);
    let origin = Vec2::new(12.25, 0.0);

    layout.emit_glyphs(&mut commands, &mut sys, origin, framewise::Color::BLACK, 0);

    assert!(!commands.glyphs().is_empty());
    for layout_glyph in layout
        .resolved_glyphs()
        .iter()
        .zip(commands.glyphs())
        .map(|(l, _)| l)
    {
        let expected = subpixel_bin(origin.x + layout_glyph.origin.x);
        assert_loaded_subpixel(&sys, layout_glyph.id, expected);
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
) -> (
    framewise::TextLayout<crate::text::SampleGlyphToken>,
    DrawCommands,
) {
    let layout = layout_text(sys, text, style, bounds);
    let mut commands = DrawCommands::new(1.0);
    layout.emit_glyphs(&mut commands, sys, origin, Color::BLACK, 0);
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

fn raster_ink_bounds_for_glyphs(glyphs: &[DrawGlyph]) -> Rect {
    glyphs
        .iter()
        .filter_map(|glyph| {
            let (_, _, w, h) = decode_prepared_glyph_token(glyph.token);
            if w == 0 || h == 0 {
                return None;
            }
            Some(Rect::new(
                glyph.top_left.x,
                glyph.top_left.y,
                w as f32,
                h as f32,
            ))
        })
        .fold(None, union_rect)
        .unwrap_or(Rect::ZERO)
}

fn caret_geom_at_byte(
    layout: &framewise::TextLayout<SampleGlyphToken>,
    byte_index: usize,
) -> framewise::CaretGeom {
    layout.caret_geom(layout.caret_position_at_insertion_byte(byte_index))
}

fn measured_logical_width(text: &str, style: TextStyle) -> f32 {
    let mut sys = sys();
    measure_text(&mut sys, text, style, TextBounds::UNBOUNDED)
        .logical_size
        .x
}

fn assert_loaded_subpixel(sys: &SampleTextBackend, token: SampleGlyphToken, subpixel_x: u8) {
    assert!(
        matches!(
            sys.glyph_cache[token.0 as usize].subpixels[subpixel_x as usize],
            GlyphSubpixelSlot::Loaded(_)
        ),
        "expected glyph token {:?} subpixel slot {} to be loaded",
        token,
        subpixel_x
    );
}

fn packed_slot_token(
    sys: &SampleTextBackend,
    token: SampleGlyphToken,
    subpixel_x: u8,
) -> Option<framewise::PreparedGlyphToken> {
    let GlyphSubpixelSlot::Loaded(slot) =
        sys.glyph_cache[token.0 as usize].subpixels[subpixel_x as usize]
    else {
        return None;
    };
    Some(slot.token)
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
    assert!(!shaped.clusters[0].glyphs.is_empty());
}

#[test]
fn drop_overflow_uses_logical_advance_not_ink_width_for_single_glyph() {
    let mut sys = sys();
    let text = "◎";
    let style = style(FontId(0), 13.0, 500, TextFlow::single_line());
    let width = measured_logical_width(text, style).round();
    let layout = layout_text(
        &mut sys,
        text,
        style,
        TextBounds {
            max_width: Some(width),
            max_height: Some(28.0),
        },
    );

    assert_eq!(layout.resolved_glyphs().len(), 1);
    assert!(
        !layout.metrics().truncated_horizontal,
        "ink protrusion outside the logical advance should not count as truncation"
    );
    assert!(layout.metrics().approx_ink_bounds.w > 0.0);
}

#[test]
fn drop_overflow_uses_logical_advance_not_ink_width_for_final_glyph() {
    let mut sys = sys();
    let text = "Run ◎";
    let style = style(FontId(0), 13.0, 500, TextFlow::single_line());
    let width = measured_logical_width(text, style).round();
    let layout = layout_text(
        &mut sys,
        text,
        style,
        TextBounds {
            max_width: Some(width),
            max_height: Some(28.0),
        },
    );

    assert_eq!(
        layout.resolved_glyphs().len(),
        TextBackend::shape_text(&mut sys, text, style)
            .clusters
            .iter()
            .map(|cluster| cluster.glyphs.len())
            .sum::<usize>()
    );
    assert!(
        !layout.metrics().truncated_horizontal,
        "final glyph ink protrusion should not drop the last character"
    );
}

#[test]
fn caret_end_uses_shaped_advance_not_bitmap_width() {
    let mut sys = sys();
    let text = "Headless Test.";
    let style = style(FontId(1), 14.0, 400, TextFlow::single_line());
    let layout = layout_text(
        &mut sys,
        text,
        style,
        TextBounds {
            max_width: Some(180.0),
            max_height: Some(30.0),
        },
    );

    let expected_advance = measured_logical_width(text, style);
    let caret = caret_geom_at_byte(&layout, text.len());

    assert!(
        (caret.x - expected_advance).abs() < 0.5,
        "caret end x should follow shaped advance {expected_advance}, got {}",
        caret.x
    );
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
        .resolved_glyphs()
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
        TextBackend::prepare_glyph(
            &mut sys,
            PrepareGlyphRequest {
                glyph: glyph.token,
                glyph_origin: Vec2::new(x, 20.0),
            },
        )
        .expect("A should prepare");
        assert_loaded_subpixel(&sys, glyph.token, expected);
    }
}

#[test]
fn subpixel_bins_match_final_glyph_positions_for_body_copy() {
    let mut sys = sys();
    let style = style(FontId(1), 15.0, 400, TextFlow::single_line());
    let origin = Vec2::new(12.25, 4.0);
    let (layout, commands) = emit(&mut sys, "Body copy", style, TextBounds::UNBOUNDED, origin);

    let resolved_glyphs = layout.resolved_glyphs();
    let mut layout_glyphs = resolved_glyphs.iter();
    for draw_glyph in commands.glyphs() {
        let layout_glyph = layout_glyphs
            .by_ref()
            .find(|glyph| {
                let expected = subpixel_bin(origin.x + glyph.origin.x);
                packed_slot_token(&sys, glyph.id, expected) == Some(draw_glyph.token)
            })
            .expect("emitted glyph should come from layout glyphs");
        let expected = subpixel_bin(origin.x + layout_glyph.origin.x);
        assert_loaded_subpixel(&sys, layout_glyph.id, expected);
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

    let resolved_glyphs = layout.resolved_glyphs();
    let mut layout_glyphs = resolved_glyphs.iter();
    for draw_glyph in commands.glyphs() {
        let layout_glyph = layout_glyphs
            .by_ref()
            .find(|glyph| {
                let expected = subpixel_bin(origin.x + glyph.origin.x);
                packed_slot_token(&sys, glyph.id, expected) == Some(draw_glyph.token)
            })
            .expect("emitted glyph should come from layout glyphs");
        let expected = subpixel_bin(origin.x + layout_glyph.origin.x);
        assert_loaded_subpixel(&sys, layout_glyph.id, expected);
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

    let resolved_glyphs = layout.resolved_glyphs();
    let last = resolved_glyphs.last().expect("ellipsis glyph should exist");
    let first = resolved_glyphs.first().expect("text glyph should exist");
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

    let raster_bounds = raster_ink_bounds_for_glyphs(commands.glyphs());
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
