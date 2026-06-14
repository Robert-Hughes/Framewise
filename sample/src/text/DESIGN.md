# Sample Text System Design

The sample text system is a reference implementation for Framewise's
`TextSystem` trait. It is allowed to be simple, but it should model text with the
same concepts exposed by the public API.

## Layers

Text layout uses two different units:

- **Glyphs** are font-specific draw primitives selected by shaping. The renderer
  draws positioned glyph instances and caches rasterized glyph bitmaps in the
  atlas.
- **Clusters** are the smallest indivisible shaped text units used by layout,
  wrapping, truncation, ellipsis, caret placement, and hit-testing. A cluster
  should normally correspond to a shaping cluster emitted by Swash. It must not
  split combining marks, ligatures, or script-shaped units in a way that would
  corrupt shaping.

The text system therefore keeps both:

- a glyph stream for rendering and atlas population,
- a cluster stream for logical layout decisions and source byte mapping.

Glyphs should not answer layout questions by themselves. A single cluster may
contain multiple glyphs, and a single glyph may represent multiple source
characters.

## Shaping

Swash produces shaped clusters. The sample text system records each Swash
cluster with:

- source byte range,
- glyph range,
- logical x position,
- advance,
- whitespace/hard-break classification.

The shaped glyphs inside a cluster remain renderer-facing data. The cluster is
the movement, wrapping, truncation, and hit-test unit.

Hard newlines are represented as hard-break clusters so line records and visible
debug output can still map back to the source string.

## Wrapping And Overflow

`OverflowX::WrapCluster` operates on whole clusters:

- if a cluster fits, it is admitted to the current line,
- if it does not fit and the current line is non-empty, it starts a new line,
- if it still does not fit on an empty line, `WrapClusterFallback` chooses
  whether to keep the whole cluster overflowing or drop the whole cluster.

`OverflowX::WrapWord` groups contiguous non-whitespace clusters into word
segments. Unicode whitespace creates word wrapping opportunities; this includes
tabs, not only ASCII spaces. Future work may use the full Unicode line breaking
algorithm for CJK and punctuation-sensitive breaks.

Whitespace is not grouped into runs. Each whitespace cluster is its own
breakable word-like segment. This means whitespace follows the same overflow
hierarchy as other segments: if it fits, it is admitted; if it does not fit on a
non-empty line, it may cause a soft wrap; if it cannot fit even on an empty
line, the `WrapWordFallback` chain applies.

The one exception is the Framewise soft-wrap boundary rule: when a whitespace
cluster itself overflows a non-empty line, that single whitespace cluster is
attached to the previous visual line with zero advance rather than producing a
whitespace-only line. The same collapse applies when a fitted whitespace cluster
is immediately followed by a segment or cluster that causes the soft wrap and
the line already contains non-whitespace content before that whitespace.
Adjacent whitespace remains preserved and participates in wrapping normally; a
soft wrap collapses only the single boundary whitespace character for that wrap.
Fallback is still evaluated after wrapping: a whitespace cluster on an empty
line uses the selected fallback only if it still cannot fit there.

`OverflowX::Drop`, `OverflowX::Keep`, and ellipsis fitting also operate on
clusters. Ellipsis fitting trims whole clusters before appending the shaped
ellipsis cluster.

## Caret And Hit Testing

Caret placement and hit-testing resolve against cluster boundaries. A point
inside a cluster maps to either the cluster start or cluster end boundary. The
system must not return a byte index inside a source range that was shaped as one
indivisible cluster.

Editable text may later add finer grapheme-aware caret stops where the shaper
and script rules allow it, but cluster boundaries are the conservative baseline.

## Invariants

- No visual line contains only part of a cluster.
- Line records include both glyph and cluster ranges.
- Rendering uses glyph ranges; layout and input use cluster ranges.
- `measure` and `prepare` produce identical metrics for the same logical bounds.
- Cached layouts include enough cluster metadata for caret and hit-test queries.
