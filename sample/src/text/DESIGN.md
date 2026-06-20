# Sample Text Backend Design

Framewise's text layout contract is specified in the repository root
`DESIGN.md`; this document only describes how the sample Swash backend fulfils
the `TextBackend` side of that contract.

The sample uses Swash for shaping and rasterisation, stores prepared glyph
bitmaps in a texture atlas, and returns renderer-ready glyph resource tokens.

## Scope

The sample backend is responsible for:

- loading bundled fonts,
- maintaining Swash shape and scale contexts,
- reporting raw font metrics as integer `TextLineLayoutMetrics`,
- shaping source text into `ShapedText`,
- maintaining the shaping cache,
- interning glyph identities in the glyph cache,
- selecting horizontal subpixel bins,
- rasterising glyphs into the atlas,
- returning packed `PreparedGlyphToken`s for renderer integration.

Framewise owns wrapping, overflow, alignment, line records, caret semantics,
and hit-testing. The sample backend supplies shaped clusters and drawable
resources for that layout.

## Fonts And Metrics

`SampleTextBackend::new` loads the bundled JetBrains Mono, Inter, and Inter
Tight variable fonts. It records each font's raw `units_per_em`, ascent,
descent, and leading from Swash font metrics.

`line_layout_metrics` converts those raw metrics to whole logical-pixel
`TextLineLayoutMetrics`. `LineHeight::Normal` uses ascent, descent, and
leading; `LineHeight::Relative` multiplies the style size. The returned
`line_height` is rounded and clamped to at least `1`, while `baseline_offset`
is the rounded ascent.

The backend also detects supported `wght` and `opsz` variation axes. Optical
size is clamped to the font's supported range when available and folded into
glyph cache keys.

## Shaping Cache

The shape cache key includes source text, font id, quantised size, weight,
letter spacing, and optical size. Layout bounds, wrapping policy, alignment,
overflow, and draw origin are intentionally absent because they are Framewise
layout inputs, not shaping inputs.

Cache entries are `Rc<ShapedText<_>>`: a cache hit clones the `Rc`, and layouts
may keep immutable shaped data alive after the backend later clears or evicts
cache entries.

## Shaping

Swash emits shaped clusters. The sample maps each Swash cluster to
`ShapedCluster` with:

- source byte range,
- logical advance,
- `chars().all(char::is_whitespace)` whitespace classification,
- shaped glyph tokens,
- shaped glyph offsets and advances,
- approximate raster-independent glyph ink bounds,
- cluster approximate ink bounds computed from glyph bounds.

The sample normally derives approximate glyph ink from Swash outline bounds.
Whitespace and empty placeholders use `Rect::ZERO`. If outline bounds are not
available for a visible glyph, the sample synthesizes a conservative
raster-independent rectangle from glyph advance and style size.

## Glyph Cache And Tokens

`shape_text` interns each origin-independent `GlyphBaseKey` on shape-cache
misses. The resulting `SampleGlyphToken` is a compact index into the append-only
`glyph_cache`, and shaped text may keep that index alive through cached
`ShapedText`.

`GlyphBaseKey` contains font id, raw glyph index, quantised size, weight, and
optical size. `prepare_glyph` adds only the origin-dependent horizontal
subpixel bin before looking up or rasterising the selected slot.

`PreparedGlyphToken` is the renderer-ready atlas token. The sample token format
stores x/y/w/h as four `u16` lanes, so it assumes one atlas texture and source
rectangles no larger than `u16::MAX`.

## Subpixel Binning

The sample uses grayscale antialiasing and four horizontal subpixel bins:
`0.0`, `0.25`, `0.5`, and `0.75`. `prepare_glyph` chooses the bin from the final
glyph origin supplied by Framewise and snaps the returned bitmap top-left to
the corresponding quantised X position. Vertical placement is rounded to whole
pixels.

## Rasterisation And Atlas

Glyph rasterisation is lazy per `(GlyphBaseKey, subpixel_x)` slot. When a slot
is first prepared, the backend rasterises it with Swash, packs the bitmap into
the atlas using a simple shelf allocator, records atlas coordinates in the
slot, and marks the atlas dirty for upload.

`prepare_glyph` returns `None` for non-drawable glyphs, zero-area glyphs, or
failed rasterisation. For drawable glyphs, `DrawGlyph::top_left` is the final
bitmap top-left including glyph bearing and bitmap placement.

## Renderer Integration

The renderer decodes `DrawGlyph::token` to atlas UVs and image size, then draws
the bitmap at `DrawGlyph::top_left` without scaling. Atlas filtering is nearest
neighbor so pre-shifted subpixel glyph bitmaps map cleanly to screen pixels.

## Invariants

- Backend shaping must preserve Swash cluster boundaries.
- Every source character in a shaped segment must be accounted for by shaped
  cluster byte ranges.
- `glyph_cache` entries must remain append-only and stable while cached shaped
  text can reference their `SampleGlyphToken` indices.
- `prepare_glyph` may skip non-drawable glyphs by returning `None`.
- `DrawGlyph::top_left` is the final bitmap top-left, not the baseline origin.
