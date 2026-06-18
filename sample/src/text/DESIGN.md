# Sample Text Backend Design

The sample text code is the sample implementation of Framewise's `TextBackend`.
It is not responsible for Framewise layout policy. It supplies the font,
shaping, rasterisation, cache, atlas, and renderer-resource pieces used by
Framewise's dependency-free text layout.

The sample uses Swash for shaping and rasterisation, then stores prepared glyph
bitmaps in a texture atlas for the renderer.

## Role

Framewise owns text layout. The sample backend owns the font and rendering
resources needed to make that layout drawable.

The boundary is:

- the sample backend returns `ShapedText` containing shaped clusters and opaque
  shaped glyph tokens,
- Framewise converts that into private working line/cluster records stored by
  `TextLayout` over the shared shaped runs,
- `TextLayout::emit_glyphs` later calls the backend's `prepare_glyph` for each
  visible drawable layout glyph,
- the renderer resolves the returned `PreparedGlyphHandle`s to atlas data.

There is no sample run table for prepared text layouts, no `TextHandle`, and no
`DrawCmd::Text`.

Framewise keeps this as a two-conversion pipeline: backend-owned cached
`ShapedText` becomes Framewise-owned working layout records once, and those
working records become backend-prepared `DrawGlyph`s when commands are emitted.
Between those steps, Framewise mutates or moves the same clusters for wrapping,
overflow, alignment, metrics, caret geometry, and hit-testing.

## Backend Responsibilities

The sample backend is responsible for:

- loading fonts,
- maintaining Swash shape and scale contexts,
- reporting `TextLineLayoutMetrics`, including line height and baseline offset,
- shaping source text into `ShapedText`,
- preparing individual glyphs for drawing as `DrawGlyph`,
- allocating and resolving `PreparedGlyphHandle`s,
- maintaining the glyph cache and atlas.

Baseline offset should come from font ascent, not from `style.size`. The sample
uses font metrics to compute ascent, descent, leading, and `LineHeight` policy.

## Framewise Responsibilities

Framewise is responsible for:

- hard newline handling,
- wrapping,
- horizontal and vertical overflow,
- ellipsis fitting at cluster boundaries,
- line records,
- logical metrics,
- caret geometry and movement,
- hit-testing and source byte mapping,
- glyph-run emission into `DrawCommands`.

The sample backend must not duplicate those layout policies. It supplies shaped
clusters and drawable resources; Framewise decides how those clusters become
visual lines.

## Glyphs And Clusters

Text layout uses two different units:

- **Glyphs** are font-specific draw primitives selected by shaping. The renderer
  draws prepared glyph bitmaps and caches rasterized glyph images in the atlas.
- **Clusters** are indivisible shaped source units. A cluster may contain
  multiple glyphs, and a glyph may represent multiple source characters.

Framewise uses clusters for wrapping, overflow, caret placement, hit-testing,
and source byte mapping. Glyphs should not answer layout questions by
themselves.

The backend's job is to preserve Swash's shaping cluster boundaries in
`ShapedCluster`. Framewise's job is to preserve those indivisible units during
layout.

## Shaping

Swash emits shaped clusters. The sample maps each Swash cluster to
`ShapedCluster` with:

- source byte range,
- advance,
- whitespace classification,
- shaped `ShapedGlyph` tokens,
- shaped glyph offsets and advances,
- mandatory approximate raster-independent glyph ink bounds,
- cluster approximate ink bounds computed from those glyph bounds.

Hard newlines are handled before backend shaping. Framewise splits source text
into hard-break source lines, calls `shape_text` for each segment, and creates
the hard-break layout clusters and line records itself. The sample backend does
not need to manufacture hard-newline clusters.

Framewise owns overflow policy and chooses marker text such as the Unicode
ellipsis. The sample backend shapes those markers through the same
`shape_text` path as ordinary text, so the normal shaping cache applies. Cache
entries are `Rc<ShapedText<_>>`: a cache hit clones the `Rc`, while layouts can
hold immutable shaped data even if the backend later evicts or clears its cache.

The sample normally derives approximate glyph ink from Swash outline bounds.
Whitespace and empty placeholders use `Rect::ZERO`. If outline bounds are not
available for a visible glyph, the sample synthesizes a conservative
raster-independent rectangle from the glyph advance and style size rather than
returning an unknown result.

## Glyph Preparation

`prepare_glyph` receives a `PrepareGlyphRequest`. The request contains the
backend-shaped glyph token and the final glyph origin after Framewise layout and
caller draw origin have both been applied. The token already carries the
origin-independent glyph resource identity: font, raw glyph index, size, weight,
and optical size.

The backend uses the final origin for horizontal subpixel bin selection. It then
looks up or rasterizes the glyph for the selected font, size, weight, optical
size, and subpixel bin.

`prepare_glyph` returns `Option<DrawGlyph>`. It returns `None` for spaces,
newlines, zero-area glyphs, or failed rasterisation. For drawable glyphs,
`DrawGlyph::top_left` includes glyph bearing and bitmap placement. It is the
final bitmap top-left, not the baseline origin or cluster position.

The renderer later resolves `DrawGlyph::handle` to atlas UVs and image size, and
draws the bitmap at `DrawGlyph::top_left` without scaling.

## Atlas And Handles

`PreparedGlyphHandle` is an opaque renderer-ready glyph resource handle. It is
not a source character, not a cluster, not a text run, and not a font glyph ID by
itself.

In the sample, a `SampleGlyphToken` stores font ID, raw glyph index, size,
weight, and optical size. `prepare_glyph` adds only the origin-dependent
horizontal subpixel bin to form a `GlyphKey`. The backend maps each `GlyphKey`
to a stable `PreparedGlyphHandle`, stores glyph pixels in the atlas, and
resolves handles back to atlas rectangles for the renderer.

## Metrics

The backend supplies line height and baseline offset. Framewise uses those
metrics when building visual lines, caret geometry, and text block metrics.

`measure_text` and `TextLayout::metrics` report stable logical layout metrics.
`TextMetrics::logical_size` is suitable for widget sizing.
`TextMetrics::approx_ink_bounds` is approximate and conservative in layout
coordinates. Exact drawn bounds require emitted `DrawGlyph`s plus the resolved
atlas image sizes for their `PreparedGlyphHandle`s. Final raster ink may depend
on draw origin, subpixel bin, hinting, rasterisation mode, and backend resource
details.

## Invariants

- Backend shaping must not split indivisible shaping clusters.
- Framewise layout must not split clusters.
- Every source character in a shaped segment must be accounted for by shaped
  cluster byte ranges.
- `prepare_glyph` may skip non-drawable glyphs by returning `None`.
- `PreparedGlyphHandle` must remain valid for the renderer resource lifetime
  expected by the sample renderer.
- `DrawGlyph::top_left` is the final bitmap top-left.
- Framewise owns wrapping, overflow, line records, caret semantics, and
  hit-testing.
- The sample backend must not duplicate Framewise layout policy.
- There is no `TextHandle`, no `DrawCmd::Text`, and no sample-owned prepared
  layout cache.
