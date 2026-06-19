use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use framewise::{
    text::layout_text, Color, DrawCommands, FontId, LineHeight, TextBounds, TextFlow, TextStyle,
    Vec2,
};
use sample::text::SampleTextBackend;

const BENCH_TEXT: &str = "\
Buttons combine label layout, intrinsic measurement, wrapping, truncation, \
and cached glyph preparation. This benchmark intentionally uses enough text \
to exercise line breaking and repeated cluster processing.

A second paragraph forces hard newline handling and another shaped run. \
The hot loop should run after shaping and raster caches are warm.";

fn make_bench_style() -> TextStyle {
    TextStyle::new(FontId(0), 14.0, 400, TextFlow::wrapped())
        .with_line_height(LineHeight::Relative(1.35))
}

fn warm_text_caches(
    backend: &mut SampleTextBackend,
    style: TextStyle,
    bounds: TextBounds,
    origin: Vec2,
) {
    for _ in 0..50 {
        let layout = layout_text(backend, BENCH_TEXT, style, bounds);
        let metrics = layout.metrics();

        let mut commands = DrawCommands::new();
        layout.emit_glyphs(&mut commands, backend, origin, Color::WHITE, 0);

        black_box(metrics.logical_size);
        black_box(commands.len());
        black_box(commands.glyphs().len());
    }
}

fn bench_text_hot_path(c: &mut Criterion) {
    let mut backend = SampleTextBackend::new();
    let style = make_bench_style();
    let bounds = TextBounds {
        max_width: Some(360.0),
        max_height: Some(240.0),
    };
    let origin = Vec2::new(12.0, 34.0);

    warm_text_caches(&mut backend, style, bounds, origin);

    c.bench_function("text_hot_path_measure_layout_emit_warm_cache", |b| {
        b.iter_batched(
            DrawCommands::new,
            |mut commands| {
                let layout = layout_text(&mut backend, black_box(BENCH_TEXT), style, bounds);
                let metrics = layout.metrics();

                layout.emit_glyphs(&mut commands, &mut backend, origin, Color::WHITE, 0);

                black_box(metrics.logical_size);
                black_box(commands.len());
                black_box(commands.glyphs().len());
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, bench_text_hot_path);
criterion_main!(benches);
