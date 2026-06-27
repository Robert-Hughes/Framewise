# Framewise Design

Detailed design decisions and implementation architecture for Framewise.

---

## State Storage

For widgets that maintain state across frames — hover/press tracking, text content, caret position, scroll offset — Framewise defines a transparent `*State` struct per widget type (e.g. `ButtonState`, `TextEditState`). The app allocates and owns this struct; the widget receives it as `&mut *State` and mutates it in place.

The app is responsible for initialising the state and keeping it alive for as long as the widget is displayed. If the app wants the widget's value to stay in sync with its own data store, it synchronises explicitly — reading `state.value` after each frame or writing to it before the widget call.

**Why library-defined state structs, rather than letting the app pass its own fields directly?**

- Widget state is often richer than the app's domain model needs. A `TextEditState` tracks text content, caret position, selection range, and scroll offset — the app typically only cares about the final string. Exposing the full struct lets the widget manage its own bookkeeping without burdening the app with those details.
- Apps commonly want a *draft* value in the widget that is only committed to the data store after validation or an explicit user action (e.g. pressing Save). The `*State` struct naturally holds this draft; the app copies it out when ready, rather than having the widget write directly into app state.

State structs are composed from shared sub-structs based on widget capability — e.g. a `FocusState` sub-struct is common across many widget types, with a shared `handle_focus` helper that manipulates it.

### `*State` vs `*Spec` — What Changes and Who Changes It

The distinction between the two parameter types is about **who mutates what, and when**:

- **`*State`** holds data that the widget function itself may modify as a direct result of user interaction — hover tracking, pressed flags, scroll position, text content, caret position, focus IDs. The caller passes `&mut *State`; the widget mutates it in place.
- **`*Spec`** holds everything the caller provides as input to the widget for that frame. Spec fields can vary frame-to-frame (e.g. elapsed time, a label string, an enabled flag driven by app logic), but they are **never mutated by the widget function**. The spec is consumed, not updated. High-level specs are complete values by the time they are passed to high-level widget functions: they do not represent partially-built intent waiting for the widget to fill missing fields.

In short: if a value changes because the user clicked or typed, it belongs in `*State`. If it changes because the app decided something different this frame, it belongs in `*Spec`. If a value is a piece of caller-supplied code that the widget calls back into for this frame, such as a value-formatting closure, it also belongs in `*Spec`.

---

## Logical Coordinates And DPI

Framewise layout, widget state, input, hit-testing, preferred sizes, text
layout, and `DrawCmd` geometry use logical `f32` coordinates. The display scale
is represented as `physical_pixels_per_logical_pixel`, the OS/device scale
factor. This value describes the physical pixel grid; it does not scale the UI
or convert Framewise layout to physical pixels.

`DrawCommands` stores the raw `physical_pixels_per_logical_pixel: f32` for the
draw list. Widgets that need crisp rectangular chrome should snap at the draw
emission boundary with `DrawCommands` helpers such as
`snap_to_physical_pixel`, `snap_rect_edges_to_physical_pixel`,
`push_crisp_fill_rect`, `push_crisp_border_rect`, and the device hairline
helpers. Alignment is checked with:

```text
logical_coord * physical_pixels_per_logical_pixel is an integer physical pixel coordinate
```

The sample app owns the boundary conversions. It queries the window scale,
converts physical pointer positions and pixel wheel deltas to logical input, and
passes the scale into each root `DrawCommands`. The sample renderer lowers
logical draw commands to physical framebuffer coordinates, including clips and
AA shape data. Text shaping and measurement remain logical; the sample text
backend rasterises glyph bitmaps at physical scale and reports logical
`DrawGlyph::top_left` positions for the renderer to convert back to physical
destinations.

---

## Text Architecture

Framewise owns text layout policy. The application supplies a `TextBackend`,
but that backend does not decide where Framewise lines wrap, which clusters are
admitted by overflow, or how editor caret positions work. Wrapping, hard newline
handling, line records, horizontal and vertical overflow, ellipsis placement,
logical metrics, caret geometry, hit-testing, insertion byte conversion, caret
movement, and glyph-run emission are all Framewise responsibilities.

The backend boundary is deliberately narrow:

- `TextBackend::shape_text` shapes source text into immutable shared
  `Rc<ShapedText>`, made of `ShapedCluster`s and backend-shaped `ShapedGlyph`
  tokens. Each shaped glyph token is opaque to Framewise and should carry the
  origin-independent resource identity the backend needs later for preparation.
  Each shaped glyph also carries mandatory best-effort approximate ink bounds,
  and each shaped cluster carries the union of those bounds in
  cluster/baseline-local coordinates. Framewise also uses this API to shape
  marker text, such as the overflow ellipsis, and never mutates shaped output.
  `shape_text` is called on one hard-line source segment or Framewise-owned
  marker string at a time. Framewise splits hard newlines before backend shaping
  and creates hard-newline layout clusters itself; marker text such as the
  overflow ellipsis is shaped through the same API and later remapped into
  source-text coordinates where needed.
- `TextBackend::line_metrics` supplies whole-logical-pixel line height and
  baseline offset through `TextLineLayoutMetrics`; `line_height` must be at
  least `1`.
- `TextBackend::prepare_glyph` turns one visible laid-out glyph into an
  optional `DrawGlyph`.

Font loading, fallback strategy, Swash state, glyph rasterisation, glyph cache
keys, atlas allocation, texture upload, `PreparedGlyphHandle` allocation, and
`PreparedGlyphHandle` to renderer-resource lookup are backend/application
concerns. Framewise remains dependency-free; Swash, font files, atlas packing,
glyph rasterisation, and WGPU resources stay in the sample/backend.

`layout_text(...)` is the main layout entry point:

```rust
let layout = layout_text(text_backend, text, style, bounds);
let metrics = layout.metrics();
```

It returns an owned `TextLayout<G>`. The layout stores private final
Framewise-owned working line records, each with its own working clusters, plus
shared shaped runs. It does not store a permanent flat glyph vector or a flat
final cluster vector. Widgets query that layout directly for metrics, caret
geometry, hit-testing, insertion byte conversion, caret movement, and draw
emission. Resolved glyphs are produced only when explicitly materialised for
tests/debugging or emitted for drawing. There is no `TextHandle` or
`TextLayoutHandle` indirection, and there is no Framewise-side layout cache by
default. Caching, if an application needs it, belongs above this owned value
API.

Framewise text layout has exactly two conversion boundaries:

1. Backend shaping output -> Framewise working layout representation. The
   backend returns cached immutable `ShapedText` containing backend glyph
   tokens. Framewise converts shaped runs into its own working lines/clusters
   exactly once. These working clusters store source byte ranges, logical
   x/advance, visibility, wrapping state, and source references into shaped
   runs.
2. Framewise working layout representation -> draw commands. When emitting
   text, Framewise resolves final glyph origins from line baseline, cluster x,
   and shaped glyph offsets, then returns those tokens to the backend with final
   origins so it can prepare glyphs and append `DrawGlyph`s into `DrawCommands`.

Between those boundaries, layout mutates and moves the same working cluster
objects and derives metrics, caret positions, and hit-test results. It should
not copy clusters into another intermediate representation. `Working*` types
are Framewise-owned layout-space records. They are not backend shaping output,
and they are not public API records. Some working records are stored inside
`TextLayout` after finalisation.

Drawing is a second step. `TextLayout::emit_glyphs(...)` resolves visible glyph
positions lazily from final line/cluster records plus shared shaped runs, passes
their shaped tokens and final glyph origins to `TextBackend::prepare_glyph`, and
stores any returned `DrawGlyph`s in the `DrawCommands` glyph arena. The draw stream
contains:

```rust
DrawCmd::GlyphRun {
    glyphs: Range<usize>,
    color,
    z,
}
```

The range references a contiguous slice of `DrawCommands::glyphs()`. The
renderer resolves each `DrawGlyph::handle` (`PreparedGlyphHandle`) to atlas or
resource data and performs a no-scale glyph bitmap draw at
`DrawGlyph::top_left`. That `top_left` is the final bitmap top-left. It is not a
text baseline origin, cluster position, or unadjusted glyph origin.

Measurement reports stable logical layout geometry. `layout_text(...)` and
`TextLayout::metrics()` produce `TextMetrics`, whose `logical_size` is suitable
for widget sizing. `TextMetrics::approx_ink_bounds` is an approximate,
conservative layout-coordinate estimate calculated from mandatory backend
best-effort glyph ink estimates; `Rect::ZERO` means known no visible ink. Exact
drawn ink requires emitted `DrawGlyph`s plus image sizes resolved from their
`PreparedGlyphHandle`s. Final raster bounds may depend on draw origin, subpixel
binning, hinting, rasterisation mode, atlas placement, and backend-specific
resource details.

---

## Text Wrapping And Whitespace

Soft-wrapping whitespace has no single obvious answer. CSS has several
different whitespace modes, and native UI frameworks do not all agree on how
preserved spaces should behave at line edges. Some systems collapse whitespace
for prose, some preserve it for editable or preformatted text, some hang
trailing spaces at the end of a line, and some allow spaces to wrap onto lines
by themselves.

Framewise adopts one fixed whitespace policy for text wrapping. The goal is to
keep the model simple and predictable while avoiding the most awkward visual
result in the common case.

The rule is:

```text
- Every source whitespace character is preserved and accounted for.
- Whitespace characters are individually wrappable.
- When soft wrapping a non-empty line, collapse exactly one preserved whitespace
  character if either:
  - that whitespace character is the overflowing unit that causes the soft wrap,
    or
  - the overflowing unit immediately follows that whitespace character, and the
    line already contains non-whitespace content before that whitespace.
- The collapsed whitespace character is kept in the previous visual line's byte
  range and caret/selection model, but assigned zero visual advance and excluded
  from that line's `logical_width`.
- If preserved whitespace at the end of the text overflows and is collapsed at
  a soft-wrap boundary, the layout still creates one following empty visual line.
  This mirrors a trailing hard newline: the boundary whitespace belongs to the
  previous visual line, while the caret position after it is on the next empty
  line. This makes it friendly to text editors, where the user adding a terminal space
  sees the caret move to the next line ready to type. For labels, a trailing space taking up
  a whole extra line like this might be unwanted, but then there shouldn't be a trailing space!
- Adjacent whitespace remains preserved and participates in wrapping normally.
  A soft wrap collapses only the single boundary whitespace character for that
  wrap; later adjacent whitespace is not collapsed unless it independently
  becomes the boundary character of a later soft wrap.
```

This exception exists to avoid turning ordinary single-space prose into a blank
line. Without it, `"hello world"` wrapped to five columns would produce a visual
line containing only the separating space:

```text
Width = 5
. = visible space

Without the boundary-space exception:

+-------------+
| hello       |
| .           |
| world       |
+-------------+
```

With Framewise's policy, the separating space is still present in the source
mapping, caret model, selection model, and byte ranges, but it does not consume
visible width:

```text
Width = 5
~ = logically present soft-wrap boundary space with zero visual advance

+-------------+
| hello~      |
| world       |
+-------------+
```

The same rule also applies when the separating space itself fits, but the next
word causes the soft wrap. A trailing space at the end of a left-aligned line can
look innocuous because it appears after the visible text. For right-aligned text,
the same logical trailing space moves the line's anchor: it is the mirror image
of a leading space on a left-aligned line. In both cases, the visible word is
offset by whitespace that the reader does not perceive as part of the line. A
soft-wrap boundary space is therefore excluded from `logical_width` whether it
was the overflowing unit or the unit immediately before the overflow.

This is deliberately similar to how hard newlines work. A `'\n'` is a real
source character with caret and selection positions, but it has no ordinary
visible glyph advance. Text editing already has to reason about such characters,
so soft-wrap boundary spaces use the same kind of model instead of introducing a
separate "dropped whitespace" concept.

The collapsed boundary space is excluded from the line's logical width. It
remains part of the previous visual line's source byte range and must remain
reachable through caret movement, hit-testing, and selection. A text editor may
draw an explicit selection affordance for it, just as it may draw one for a
selected newline.

At the end of the text, collapsed soft-wrap boundary whitespace also creates a
following empty visual line when that boundary whitespace is the final source
character. This is intentional even though the trailing space has no visual
advance: the empty line is the visible caret, hit-testing, selection, and
editor-feedback position after the whitespace. In an adjacent terminal run,
only the one whitespace character that caused the soft wrap is collapsed; later
whitespace remains preserved and visible on the following line unless it causes
another soft wrap.

Examples:

```text
Legend:
. = visible space
~ = logically present, visually collapsed soft-wrap boundary space
Width = 5 columns

+----------------+-------------------------------+
| Input          | Wrapped output                |
+----------------+-------------------------------+
| hello world    | hello~                        |
|                | world                         |
+----------------+-------------------------------+
| hello  world   | hello~                        |
|                | .                             |
|                | world                         |
+----------------+-------------------------------+
| hello   world  | hello~                        |
|                | ..                            |
|                | world                         |
+----------------+-------------------------------+
| hello.         | hello~                        |
|                | <blank>                       |
+----------------+-------------------------------+
| hello..        | hello~                        |
|                | .                             |
+----------------+-------------------------------+
| .....hello     | .....                         |
|                | hello                         |
+----------------+-------------------------------+
| ......hello    | .....~                        |
|                | hello                         |
+----------------+-------------------------------+
| hello\nworld   | hello                         |
|                | world                         |
+----------------+-------------------------------+
| hello\n\nworld | hello                         |
|                | <blank>                       |
|                | world                         |
+----------------+-------------------------------+
```

The same boundary-space collapse applies when the space fits but the following
unit does not:

```text
Legend:
. = visible space
~ = logically present, visually collapsed soft-wrap boundary space
Width = 6 columns

+----------------+-------------------------------+
| Input          | Wrapped output                |
+----------------+-------------------------------+
| hello world    | hello~                        |
|                | world                         |
+----------------+-------------------------------+
| hello  world   | hello.~                       |
|                | world                         |
+----------------+-------------------------------+
```

The multiple-space cases are intentionally not hidden. If an author enters
multiple spaces between words, Framewise preserves that fact and lets the layout
show it. The single-space exception is only for the whitespace character that
causes a soft wrap, or for the whitespace character immediately before the unit
that causes a soft wrap when the line already contains non-whitespace content,
so the normal well-authored prose case avoids a blank space-only line or a
visually trailing wrap space without turning Framewise into a
whitespace-collapsing text engine.

This behaviour is implemented by Framewise's text layout module. The backend
supplies shaped clusters and glyph tokens; it does not implement the soft-wrap
boundary policy.

---

## Text Overflow And Cluster Boundaries

Framewise text overflow is cluster-based. A cluster is the conservative
indivisible unit returned by shaping: it may contain multiple glyphs, and a
single glyph may represent multiple source characters. Layout must not split a
cluster, because doing so can break combining marks, ligatures, or script-shaped
units.

`OverflowX::WrapCluster` operates on whole clusters:

- if a cluster fits, it is admitted to the current line,
- if it does not fit and the current line is non-empty, it starts a new line,
- if it still does not fit on an empty line, `WrapClusterFallback` chooses
  whether to keep the overflowing cluster or drop it.

`OverflowX::WrapWord` treats each maximal run of non-whitespace,
non-hard-break clusters as one word segment. Each whitespace cluster is an
independent breakable segment. Unicode line-break opportunities other than
whitespace are not currently recognised. Whitespace follows the same overflow
hierarchy as other segments: if it fits, it is admitted; if it does not fit on a
non-empty line, it may cause a soft wrap; if it cannot fit even on an empty
line, the fallback chain applies.

The one exception is the soft-wrap boundary-space rule described above. When a
single whitespace cluster becomes the boundary after earlier non-whitespace
content on the same visual line, that cluster may remain in the previous line
with zero advance. Other whitespace remains preserved and may form its own
visual line.

`OverflowX::Drop`, `OverflowX::Keep`, and ellipsis fitting also operate on whole
clusters. Ellipsis fitting trims whole clusters before appending the Framewise
ellipsis marker, shaped through `TextBackend::shape_text`, stored as a shaped-run
view, and remapped to a zero-length byte range at the source truncation point.
`OverflowY` operates on whole visual lines after hard breaks, wrapping, and
horizontal overflow have been resolved.

Caret placement and hit-testing also resolve against cluster boundaries. A point
inside a cluster maps to either the cluster start or cluster end boundary.
Framewise must not return an insertion byte inside a source range that was
shaped as one indivisible cluster. Editable text may later add finer
grapheme-aware caret stops where the shaper and script rules allow it, but
cluster boundaries are the conservative baseline.

---

## Caret Positions Are Visual Anchors

Some visual line boundaries need more information than a plain insertion byte
can carry. Editor-facing caret APIs therefore use `CaretPosition`, which
anchors the caret before or after a specific shaped cluster.

For a soft wrap inserted between clusters with no source boundary character, the
trailing edge of the previous visual line and the leading edge of the following
visual line can map to the same source insertion byte. `CaretPosition` preserves
that visual affinity:

```text
ab|
|cd
```

- `AfterCluster('b')` is the end of the previous visual line.
- `BeforeCluster('c')` is the start of the following visual line.
- Both carry the same insertion byte.

Hard newline clusters are explicit boundary clusters:

```text
abc\n
```

- `BeforeCluster('\n')` is the end of the previous visual line.
- `AfterCluster('\n')` is the start of the following visual line.
- These are source-distinct insertion positions around the newline cluster.

Collapsed soft-wrap whitespace also has a real source cluster:

```text
abc.    // trailing space collapsed by soft wrap
```

- `BeforeCluster(' ')` is the end of the previous visual line.
- `AfterCluster(' ')` is the start of the following empty visual line when
  terminal wrapped whitespace creates one.
- These are source-distinct insertion positions around the collapsed whitespace
  cluster.

Editing operations still use insertion byte indices for mutation. The visual
caret position is converted to an insertion byte only at the point where text is
inserted, removed, copied, or selected.

---

## Widget Consistency

All widget files must be consistent with each other. A reader browsing from one widget to another should never have to ask "why does widget X do it like this but widget Y does it like that?"

Consistency applies across every dimension of the code:

- **Naming** — struct names, field names, parameter names, local variable names, result field names
- **File structure** — ordering of structs, functions, and sections within the file
- **Derived traits** — the same set of `#[derive(...)]` on equivalent structs (e.g. all `*Spec` structs derive the same traits)
- **Visibility** — `pub`, `pub(crate)`, or private applied consistently to equivalent items
- **Parameters** — order of parameters to raw functions and high-level context functions. High-level widget functions take caller-supplied inputs first and `&mut WidgetContext` last; raw functions pass runtime systems explicitly, with `cmds: &mut DrawCommands` last.
- **Return types** — if one widget's high-level function returns `layout: LayoutInfo`, all do; if one raw result includes `content_bounds`, equivalent raw results do too. (Exception: [deferred-own-size containers](#deferred-own-size-containers) omit `layout` — they do not know their bounds at `begin`. This is a principled deviation shared by all such containers, not per-widget drift.)
- **Default value handling** — `Default`, `new(...)`, `*_from_theme(...)`, and `theme(...)` methods applied uniformly based on field semantics. `Option<T>` is used only for real optional values, not for unset builder state.
- **Composition patterns** — shared sub-structs (e.g. `FocusState`), shared helper functions (e.g. `handle_focus`), used consistently rather than re-implemented per widget
- **Doc comments and inline comments** — same level of documentation for equivalent items; no widget's public API should be substantially better or worse documented than another's

Differences between widgets are acceptable only when driven by **genuine functional differences** — a container widget necessarily has a different lifecycle than a leaf widget, a stateless label necessarily omits `&mut *State`. If two widgets handle the same concern differently, the difference must be justified by a real difference in what they do, not by the order they were written or who wrote them.

When adding a new widget or modifying an existing one, check the other widgets and align with them.

---

## Mouse Capture

A classic challenge in immediate-mode GUIs is "mouse capture". If a user clicks on a button and drags the mouse off it onto a second button, the second button shouldn't accidentally trigger a click when the mouse is released.

Framewise completely rejects global ID registries. Instead, we solve capture by pushing state into the application. Even simple widgets like buttons take a `&mut ButtonState`. The widget itself tracks whether it was the original target of a mouse press, elegantly handling dragging and hover logic purely locally. This requires slightly more boilerplate from the app, but results in a vastly more robust architecture that is completely immune to ID collisions.

### Alternative Considered: Stateless "Mouse Down Pos"

We considered a stateless alternative: storing the initial `mouse_down_pos` in the global `Input` struct, and having each widget check if its rectangle contains that position. This would allow simple buttons to remain entirely stateless and avoid the `ButtonState` boilerplate.

We explicitly chose the app-owned `ButtonState` approach for two key reasons:

1. **Consistency:** Complex widgets (like scrollable regions or text inputs) absolutely require app-owned state anyway. Keeping the architecture consistent — where *every* interactive widget owns its state — is cleaner than mixing stateless tricks with stateful widgets.
2. **Robustness:** The stateless position trick can break in edge cases, such as when the UI layout shifts underneath the mouse while the button is held down (e.g. an element is inserted above it, moving the button out from under the original `mouse_down_pos`). By binding the active state directly to the specific widget's data struct, the capture is strictly guaranteed regardless of how the layout shifts.

---

## Interaction Bounds vs. Visual Bounds

For many widgets (such as Buttons, TextInputs, and Frames), the **visual boundaries** of the widget perfectly match the **layout/interaction boundaries** (`spec.rect`).

However, some widgets (such as Checkboxes, Radios, and Switches) decouple these two concepts:
- **Interaction/Layout Bounds (`spec.rect`)**: The parent layout allocates a cell for the widget (which may be larger than the physical control, e.g. a wide row cell to accommodate a checkbox and its label). The entire `spec.rect` is used as the hit-test target (hovering, pressing, clicking) and keyboard focus registry. This maximizes the target area (Fitts's Law), making interactive controls much easier to click.
- **Visual Bounds**: The drawn graphics of the control (e.g. the 14x14 checkbox box, 14x14 radio circle, or 30x16 switch track) are kept at a fixed size and aspect ratio to ensure visual consistency. If `spec.rect` is larger/taller than the control's natural size, the control is automatically centered vertically within `spec.rect` rather than stretching.

This decoupling guarantees both premium UX (generous click targets) and consistent visual alignment in forms without requiring callers to manually calculate offsets.

This is taken advantage of for the 'labelled' version of these widgets (labelled_checkbox etc.)

---

## Layout System

We decouple the **configuration** of a layout from its **mutable state**. This avoids the "pyramid of doom" closure nesting found in many immediate-mode libraries, while maintaining pure, linear predictability.

### Why Top-Down (Bounds-First) Layout

Top-down layout — where the parent dictates the bounds children must fit into — is philosophically natural for GUI applications for a simple reason: **you almost always know the size of your container but not the size of your content.**

A window's dimensions are set by the user or the OS. A panel's width comes from your app's layout. But the content inside — user-typed text, a dynamically-loaded list, a network-fetched image — is fundamentally unknown until it arrives.

Bottom-up ("auto-size") layout inverts this: children measure themselves and report their natural size upward. This is elegant when content drives the layout, but it requires a separate measurement pass, makes constraint propagation complex, and forces every widget to handle the case where content size is genuinely unknown. Scroll areas handle the "content is larger than the view" case cleanly: the content gets its logical bounds, the view clips it.

### The Headline Rule — What Layout Can and Cannot Automate

The reach of the one-pass model is captured in a single rule:

> **If a placement resolves from what's already known — available space, already-placed siblings, and this child — Framewise automates it. If it needs a *future* sibling, you declare the structure up front, or it's not possible.**

Three tiers fall out of it, in plain UI terms:

- **Automate** (past-only) — "stack these labels, each as tall as its text." Resolves from this child's size request + earlier siblings. Handled by `ColumnLayout`/`RowLayout`/`WrapLayout` with size requests.
- **Declare** (future sibling, but you said how many) — "split this row into four equal columns." Leftover/shared space depends on *all* siblings, which is a future-sibling dependency — but declaring the count converts it into a constant resolved from available space alone. This is why `SplitRow` takes a `count` up front. (Weighted/grid/match-tallest variants are not yet built — see `NOTES.md` (Remaining Layout Work).)
- **Refuse** (depends on itself / over-constrained) — "size this to its text *and* force it twice its neighbour." Asks for a value that only exists after the thing it controls is decided. No fixed point in one pass; impossible at any phase. (The constraint-affecting half of fit-to-children sits here too — see [Three-State Axis Bounds](#three-state-axis-bounds--unbounded-axes).)

#### Supported Layout Cases

Real scenarios in the Automate and Declare tiers, and which mechanism handles each. (Refuse-tier non-goals are catalogued in `NOTES.md` (Remaining Layout Work).)

| Case (real scenario) | Status |
|---|---|
| Manual explicit placement | ✅ `ManualLayout` |
| Overlay / absolute children | ✅ `ManualLayout` |
| Stack, caller sizes every child (vert/horiz) | ✅ `ColumnLayout` / `RowLayout` |
| "Stack these labels, each as tall as its text" | ✅ `ColumnLayout` + size request |
| "Row of chips, each as wide as its label" | ✅ `RowLayout` + size request |
| "Fixed-width icon, label takes its requested width" (mixed per-axis) | ✅ `RowLayout` + `RowLayoutParams` |
| "Column fills the panel width, each row auto-height" (fill cross-axis) | ✅ `ColumnLayout` + `Placement::Fill` |
| "Tags that wrap onto the next line when the row fills" (flow) | ✅ `WrapLayout` |
| "A bordered box that hugs its child(ren) plus padding" (decorator) | ✅ `frame` (fit-to-children) |
| "Toolbar: search field eats leftover space, icons use their size requests" | ✅ via emit-reorder + `ManualLayout` (see below) |
| "Panel fills available height inside a normal (bounded) container" | ✅ `Placement::Fill` against `AxisBound::Exact` |
| Scroll, content size known up front | ✅ `begin_scroll_area` (`fixed`/`FIT` extent) |
| "Scroll area sized to content discovered only after its children run" | ✅ `begin_scroll_area` (`SCROLL` extent, resolved at `end`) |
| "Infinitely tall / long auto-sized list in a scroll area" | ✅ `SCROLL` extent + `Unbounded` axis |
| Nested scrolling + clipping | ✅ |
| "Three buttons sharing a row in equal thirds" | ✅ `SplitRow` |
| "Weighted split: left pane 2×, right pane 1×, filling the row" | 🚧 unbuilt — see `NOTES.md` (Remaining Layout Work) |
| "Space-between: first item left, last right, even gaps" | 🚧 unbuilt — see `NOTES.md` (Remaining Layout Work) |
| "A grid where each column is as wide as its widest cell" | 🚧 unbuilt (declared columns + measure-all) |
| "A row of cards all stretched to match the tallest" | 🚧 unbuilt (declared count + measure-all) |

### Emit Order, Visual Position, and Focus Order Are Independent

Three orderings are separate concerns, and Framewise has machinery for all three:

- **Emit order** — when you call the widget function. Drives draw/compositing (later renders on top) and the cursor in sequential layouts.
- **Visual position** — the resolved `Rect`. Under `ManualLayout` it is fully decoupled from emit order.
- **Focus order** — detached from emit order via `override_next` (see [Input Focus](#input-focus)).

This decoupling is a general escape hatch: **reordering emit converts a future-sibling dependency into a past-sibling one.** "First child fills the remaining row width, second uses its size request" — the fill child depends on a *future* sibling; instead emit the size-dependent child first, read its size, then emit the fill child at the computed remainder, and `override_next` to restore left-to-right focus. Visually L→R, focus L→R, emitted R→L. This works **today** with `ManualLayout` — no new machinery.

General form: **if dependencies form a DAG, emit in topological order and every dependency is already known.** Cycles (the Refuse tier) have no valid topological order and remain impossible. Two caveats:

1. **Sequential layouts couple emit order to position.** `RowLayout`/`ColumnLayout` advance a cursor by emit order, so emitting the right child first lands it in the left slot. The reorder trick needs `ManualLayout` (or a future explicit-slot helper), not a naive sequential layout.
2. **Overlapping widgets — reorder changes z.** Safe only when slots don't overlap (the common row/column case). If widgets overlap, emit order *is* layering and must not be reordered casually.

### Layout is a Context-Level Concept

`Layout` and `LayoutState` are high-level abstractions that live exclusively in the `WidgetContext` layer. **Low-level widget functions know nothing about layouts.** They receive and return plain geometry: `Rect`, `Vec2` offset, `Option<Rect>` clip. Layout is a building aid — it helps place widgets in the right position — but it does not change what a widget does or how it draws.

Concretely: `raw::begin_scroll_area` returns `(pre_cmds, token, content_bounds, offset)`. The high-level `begin_scroll_area` captures the token in an `on_finish` closure and wraps these primitives into a child `WidgetContext` parameterized with `OffsetLayout { offset, inner }` to handle offsets and clipping. Low-level widgets receive fully-resolved bounds from this context.

This separation means adding a new layout type (e.g. `GridLayout`) requires zero changes to any widget function.

The split is captured in one line: a **`LayoutSpace`** says *"what space do I have to work with"*; a **`Layout`** says *"and how I want to fill it"*. The two are handed in separately and combined by `Layout::begin(space)` — which is why `WidgetContext::root` and `begin_scroll_area` take a `Layout` plus a `LayoutSpace` rather than a pre-begun state: the caller states intent, the framework wires up the geometry.

We define two traits:

1. **`Layout`**: The user-facing configuration (e.g., `ColumnLayout { spacing: 4.0 }`). It dictates the `Params` required to position a widget and provides a `begin(space: impl Into<LayoutSpace>)` method to instantiate the layout's state. A plain `Rect` is a space with both axes `AxisBound::Exact` (`From<Rect>`), so the common `begin(some_rect)` call is unchanged; an axis only goes unbounded when a caller hands down a `LayoutSpace` that says so (see [Unbounded Axes](#unbounded-axes)).
2. **`LayoutState`**: The mutable engine that lives inside the `WidgetContext`. It accumulates positions as widgets are added.

The immediate placement call is `layout(params: S::Params, request: SizeRequest) -> Rect`. It merges three inputs: the caller's `params` (intent - fixed/auto/fill), the widget's size request (reported by a `raw::pre_layout_*` phase under a `SizeOffer`, see [Size Offers and Requests](#size-offers-and-requests)), and the layout's own state (available space + cursor). Layouts that don't size from content (`ManualLayout`) ignore `request`; request-aware layouts (column/row/wrap) read it. There is still **no separate measuring pass over a retained tree** - the only extra work is the cheap, explicit pre-layout size-request query.

### Built-in Layouts

- **`ManualLayout`**: `Params = Rect`. Explicit layout where the app specifies exact rectangles; ignores `request`. If nested (e.g. inside a scroll view), it treats its bounding box's `top_left` as an offset, so explicit rectangles are correctly shifted relative to their parent. This is also the sanctioned way to place a *high-level* widget at an explicit rect (the rect is the `Params`).
- **`ColumnLayout`**: `Params = ColumnLayoutParams`. Stacks widgets vertically, keeping a Y-axis cursor. Fields `x` and `y` specify the cross-axis (`LinearCross`) and main-axis (`LinearMain`) parameters respectively.
- **`RowLayout`**: `Params = RowLayoutParams`. Stacks widgets horizontally, keeping an X-axis cursor. Fields `x` and `y` specify the main-axis (`LinearMain`) and cross-axis (`LinearCross`) parameters respectively.
- **`WrapLayout`**: `Params = Placement2D`. Flows widgets left-to-right and wraps to the next line when the next child would overflow the available width. Never wraps a child already at the start of a line; an unbounded width has no edge to overflow, so the flow stays on one line.
- **`SplitRow`**: `Params = Placement` (cross-axis height only). A *declared-structure* layout (Phase 4): it takes a `count` up front and divides its width into that many **equal** cells, `(width − spacing·(count−1)) / count` each. Each child's width is imposed (the cell), so children declare only their height. Because dividing space needs a committed far edge, `SplitRow` requires `AxisBound::Exact` width and panics on `AtMost`/`Unbounded` — the same rule that governs `Fill` and alignment. Knowing `count` is what makes the equal split one-pass (no measure-all / emit-reorder): an equal split is otherwise a future-sibling dependency, and the declaration turns it into a constant resolved from available space alone.
- **`OffsetLayout<L>`**: A decorator that shifts the inner layout's `Rect`s by a `Vec2` offset (used by scroll areas). It forwards `Params` and `request` to the inner layout. Scroll areas wrap their content layout in `OffsetLayout { offset, inner }` and push a scissor `clip_rect`.

Because `OffsetLayout` directly shifts the `Rect`s returned during the layout pass, **widgets are physically located at their scrolled screen coordinates when created**. This means standard mouse hit-testing (`rect.contains(mouse_pos)`) works natively without translating input. We only require widgets to optionally test against a `clip_rect` so that hidden, scrolled-out elements aren't accidentally clickable.

### Layout Consistency

All layout files under the `layouts/` directory must maintain structural and stylistic consistency:

- **Naming Conventions**: Concrete layout configurations must use the suffix `Layout` (e.g., `ColumnLayout`, `RowLayout`, `WrapLayout`, `OffsetLayout`), with their accompanying state using the suffix `State` (e.g., `ColumnState`, `RowState`, `WrapState`, `OffsetState`). Custom structural configurations (like `SplitRow` and `SplitRowState`) are allowed exceptions but should remain clear.
- **File Structure**: Layout modules must follow a strict vertical order:
  1. Config struct declaration (with its doc comment and field comments).
  2. `impl Layout for ConfigStruct`.
  3. `State` struct declaration.
  4. Inherent helper `impl` blocks for the state struct.
  5. `impl LayoutState for StateStruct`.
  6. Unit tests module (`#[cfg(test)] mod tests`).
- **Method Ordering inside `impl LayoutState`**: Methods inside the trait implementation must be declared in this exact order:
  1. `peek_offer`
  2. `layout`
  3. `begin_deferred_layout`
  4. `end_deferred_layout`
  5. `resolve_space`
- **Sizing & Parameters Consistency**:
  - Keep doc comments of equal detail across similar layout implementations.
  - Implement identical panic messages when validation fails (e.g., panicking on unbounded dimensions).
  - For methods that consume a widget size request, the layout params come first, followed by `request: SizeRequest`. Deferred end methods take the layout params followed by the resolved `extent: Vec2`.

Differences in layouts are acceptable only when justified by distinct structural models (for example, `SplitRow` taking a declared item count, or `OffsetLayout` serving as a coordinate decorator).

### Size Offers and Requests

Request-aware layouts let a widget be sized from its own content without abandoning the top-down, one-pass model. The terminology separates four related concepts:

- **`SizeOffer`**: the bounds a parent layout offers a hypothetical widget for size calculation. It contains only width and height `AxisBound`s; it has no `x`/`y` origin and is not a placement.
- **`SizeRequest`**: the requested size computed from widget content/style under a `SizeOffer`. It is never final geometry and never layout policy.
- **`layout(params, request)`**: immediate placement. No widget drawing happens before this call returns the final concrete `Rect`.
- **`begin_deferred_layout(params)` / `end_deferred_layout(...)`**: deferred placement for containers where child widgets may be drawn before the container's final size is known.

The non-deferred widget path is:

1. Optionally call `peek_offer(params) -> SizeOffer`.
2. Call `raw::pre_layout_*(&pre_layout_spec, offer, ...) -> *PreLayoutResult` (which calculates `size_request`).
3. Call `layout(params, pre_layout.size_request) -> Rect`.
4. Call `raw::post_layout_*(spec, pre_layout, ...)` to perform interaction and draw the widget.

High-level widgets normally use `WidgetContext::peek_offer`; layout implementations expose the lower-level `LayoutState::peek_offer`. `peek_offer` is optional, and widgets with offer-insensitive requests may skip it.

Because no drawing happens until after `layout` returns, this path may support auto-sized centered or end-aligned widgets. The layout can learn the `SizeRequest`, resolve alignment from the final size, and only then hand a concrete `Rect` to the widget.

The deferred container path is:

1. Call `begin_deferred_layout(params) -> (provisional LayoutSpace, token)`.
2. Draw child widgets inside the provisional `LayoutSpace`.
3. Call `end_deferred_layout(token, extent) -> Rect` to resolve the final container rectangle and advance the parent layout.

Deferred layout is stricter because child output may already have been emitted into the provisional space. A layout must reject any deferred case where the provisional origin might need to move later, such as an auto-sized centered/end-aligned container. This is the same rationale currently embedded in `WidgetContext::child_with_layout`: a nested layout is a container whose final size may depend on its children, so it begins in a provisional `LayoutSpace` preserving `AtMost`/`Unbounded` bounds and advances the parent cursor only when the child finishes.

`LayoutSpace` remains the concrete space used to begin a layout context. Unlike `SizeOffer`, it includes an origin (`x`/`y`) plus width and height `AxisBound`s. `Rect` remains the resolved output handed to raw widgets. It is always fully concrete and honors the rule that no `Option`/unbounded geometry reaches a raw function.

`SizeRequest` is content + style derived, **never policy**: "fill", "grow", and weights are caller intent and live in the layout's `Params`, not in the request. The test for what belongs here: if the widget computes it from its own content under the offer, it is a `SizeRequest`; if the caller decides it, it is `Params`. "Should not shrink below 60 because the label clips" is a widget fact. "Stretch to fill the row" is caller intent.
- **`Placement2D { width: Placement, height: Placement }`** — the caller's per-axis intent handed *down* to a layout (e.g., `WrapLayout`). `Placement` is `Sized { size: Size::Fixed(px), align }`, `Sized { size: Size::Auto, align }`, or `Fill` (span the layout's available extent on that axis). Axes are absolute (width/height), not main/cross, so the same request reads identically regardless of orientation. `From<Vec2>` treats a plain size as fixed on both axes with default `Start` alignment.
- **`RowLayoutParams { x: LinearMain, y: LinearCross }` and `ColumnLayoutParams { x: LinearCross, y: LinearMain }`** — the axis-aware parameters used by `RowLayout` and `ColumnLayout` respectively. They replace `Placement2D` for linear layouts to decouple main-axis flow properties (e.g. `MainAxisAlign::Append` or `MainAxisAlign::End` alignment) from cross-axis placement properties (e.g. cross-axis alignment `Align`). Their field names are `x` and `y` to correspond with physical screen dimensions rather than width/height.
- **Missing size-request policy — recoverable fallback.** When an immediate request-aware layout needs a preferred size request that was never reported (for example, `Auto` against a widget that returns no `preferred`), `Placement::resolve_size` returns `LayoutResult::Fallback` with a safe value and `LayoutViolationKind::MissingPreferredSize`. The default violation policy may still surface that fallback as a panic, but the layout result itself remains recoverable. Deferred cases with no stable provisional origin remain hard panics where no meaningful fallback exists.

### Three-State Axis Bounds & Unbounded Axes

The space a parent hands down is a `LayoutSpace { x, y, width: AxisBound, height: AxisBound }`, where `AxisBound` represents the parent's layout knowledge:

* **`AxisBound::Exact(f32)`** — "You live in a box of exactly this size". This acts as both a hard limit and a committed coordinate anchor, permitting positioning, filling, centering, and right-alignment.
* **`AxisBound::AtMost(f32)`** — "Choose your own size, but do not exceed this maximum". This is a ceiling without a committed far edge. Only measurement and shrink-wrap decisions are permitted.
* **`AxisBound::Unbounded`** — "No ceiling on this axis". This is typically used inside scroll views, allowing content to grow naturally to its preferred size.

Position is always concrete — a layout always knows *where* a child starts — so only the *extent* can be constrained or unbounded. A fully-specified `Rect` converts automatically via `From<Rect>` to a fully `Exact` space, so layouts without dynamic constraints never see `AtMost` or `Unbounded` axes.

**Why three, not two — anchor vs ceiling.** `AtMost` is the missing middle between "totally fixed" and "infinite", and it expresses container semantics neither `Exact` nor `Unbounded` states honestly: "wrap within the panel if needed, but don't force full width", "hug contents, but never grow beyond the viewport". Text especially wants this — it rarely wants *infinite* width (which produces pathological preferred sizes), it wants "measure as naturally as you can, but under this maximum line length". The distinction that matters: `Exact(w)` answers two questions — *how much space may the child consume?* **and** *relative to what concrete box may it position itself?* — while `AtMost(w)` answers only the first. `AtMost` is a ceiling with no committed far edge; `Exact` is a ceiling plus an anchor frame. So `AtMost` is **not** a weaker `Exact` — it is a different *kind* of knowledge, and the layout API branches on it explicitly rather than silently coercing, or alignment math would run against a width that was only ever a cap.

#### The Unifying Rule of Alignment and Distribution

> **Position and distribution policies — fill, right-align, center, space-between — require `AxisBound::Exact`: a committed frame with a far edge. `AtMost` and `Unbounded` bounds permit only size-request / shrink-wrap decisions.**

If a layout (such as `ColumnLayout` or `RowLayout`) is configured with a cross-axis alignment of `Center` or `End`, the request is **unsatisfiable** when:
1. The cross-axis boundary is `AtMost` or `Unbounded` — alignment math has no committed far edge to run against (the boundary was only ever a ceiling or a scroll extent). This is a *recoverable* violation: the layout returns a safe fallback (`Start`, offset `0.0`) tagged with a `LayoutViolation`, and how it surfaces is decided by the [violation policy](#unsatisfiable-requests-layoutresult-and-the-violation-policy) below.
2. The aligned object is a deferred container (such as a `Frame`) with a dynamic size (`Size::Auto`). Deferred layouts position and draw their children during the layout pass, so the container's size would have to be resolved upfront in `begin_deferred_layout`; with `Auto` it is only known once the layout *closes*, and the already-emitted child output cannot be shifted retroactively. There is no meaningful fallback, so this stays an *unrecoverable* hard `panic!` in `begin_deferred_layout`.

Similarly, `WrapLayout` does not support deferred containers with `Size::Auto` widths because line-wrapping decisions must be resolved upfront in `begin_deferred_layout` — also a hard panic, for the same "no safe fallback" reason.

To align or wrap a nested container safely, it must have a concrete size resolved upfront (e.g. `Placement::fixed(px)`, or `Placement::fill()` against a parent of exact bounds).

#### Unsatisfiable Requests: `LayoutResult` and the Violation Policy

Recoverable unsatisfiable requests (the bound-based alignment case above, plus `Fill` against a non-`Exact` axis and `Auto` with no reported size request) are **not** raised as panics deep in the layout math. The two sizing/offset primitives — `Placement::resolve_size` and `Placement::align_offset` — return a `LayoutResult<T>`:

```rust
enum LayoutResult<T> {
    Ok(T),                                              // satisfiable; value is exact
    Fallback { value: T, violation: LayoutViolation },  // unsatisfiable; value is a safe fallback
}
```

The `Fallback` arm always carries a usable value (`Start` offset `0.0` for alignment; size request clamped to the ceiling, or `0.0`, for `Fill`) **and** a `LayoutViolation` describing what was unsatisfiable plus the call site (`#[track_caller]`). The `LayoutState` methods (`layout`, `begin_deferred_layout`, `end_deferred_layout`) compose these — assembling their `Rect`/`LayoutSpace` from the fallback sub-values and keeping the first violation — and return a `LayoutResult` instead of unwrapping internally. Layout math therefore never panics on its own; it *reports*.

**Reaction is a `WidgetContext`-level concern.** Every layout call funnels through `WidgetContext` (which owns the draw buffer, the text backend, and the policy), which reacts according to `layout_policy: LayoutViolationPolicy`:

- **`Panic`** (default) — rethrow the violation's message. Preserves the strict fail-loud contract; used by tests and any caller wanting a hard guarantee.
- **`Highlight`** — draw a red outline over the fallback geometry, label the violation message in red at its corner, and keep running.

For the immediate path the reaction happens inline (the resolved rect is in hand). For a deferred child, the `begin_deferred_layout` violation is carried *on the child* and reacted at the child's own `finish()`, where its resolved rect is concrete — so each child reacts with its own geometry and no sibling violation is dropped.

##### Why a policy, rather than one fixed behaviour

The two obvious single behaviours are each wrong on their own:

- **Always panic** is intolerable for an interactive, immediate-mode UI. Layout runs *every frame*, so one unsatisfiable request crashes the app the instant it renders — the developer can't even see the broken state to reason about it.
- **Always fall back silently** is a debugging trap. A `Center` that quietly degrades to `Start` leaves a subtly-wrong UI with no signal as to why — invisible in both the running app and the console.

A middle ground is required, and *which* one depends on the caller, so it can't be hard-coded:

- **Tests want `Panic`.** CI should fail the moment a layout becomes unsatisfiable — the cheapest place to catch a regression. Hence `Panic` is the default, leaving existing behaviour and test guarantees unchanged.
- **Interactive apps want `Highlight`.** The app keeps running so the developer sees the rest of the UI, but the offending region is unmistakable (red box) and self-describing (the message is drawn on it) — the layout equivalent of a renderer's magenta missing-texture. The sample app sets `Highlight` on every page.

The key separation: keep the *value* deterministic and safe (`Start` / clamped request) while putting the *loudness* in a policy-driven reaction. Because the fallback never moves a widget off-screen or yields a `NaN`, the rest of the frame lays out sanely around a flagged region even under `Highlight`.

**Scope and non-goals (current).** Only `Panic` and `Highlight` exist; `WarnOnce` (log-once-per-call-site, needs cross-frame state) and `Collect` (push violations to a buffer the app reads) are deferred. `Fallback` carries a single violation (first-wins); plural is a possible future direction. The text label is drawn on every reaction path — the `on_finish` closure carries the text backend into the deferred `begin_deferred_layout`/`end_deferred_layout` reactions, so they label the box like the immediate path does. The unrecoverable cases (deferred `Auto` + `Center`/`End`, `WrapLayout` `Auto` deferred) remain hard panics — no safe fallback exists, so the policy does not apply to them.

#### Sizing Resolution Rules

Three key rules keep these bounds from leaking infinity into leaf widget geometry:

1. **`Fill` on non-`Exact` axes acts as `Auto`.** Filling an infinite (`Unbounded`) or unanchored (`AtMost`) axis is undefined since there is no committed extent to fill. In these cases, the layout falls back to the widget's size request (reported as a recoverable `LayoutResult` violation if no size request is available — see [Unsatisfiable Requests](#unsatisfiable-requests-layoutresult-and-the-violation-policy)), matching `Size::Auto` resolution behavior.
2. **`AtMost` caps preferred size.** Under `AxisBound::AtMost(w)`, a widget's size request resolves to `preferred.min(w)`, preventing it from overflowing the ceiling.
3. **Unbounded resolves to concrete at accumulation.** A child laid out in an unbounded axis still resolves to a fully concrete `Rect`. The layout's running cursor stays a concrete `f32`, meaning the accumulated extent remains fully bounded (which is precisely what a deferred scroll area reads as its content size). No infinity ever reaches a `Rect`.

**Reading the accumulated extent — `resolve_space`.** `LayoutState` exposes `fn resolve_space(&self) -> Rect`: the accumulated content resolved against the layout's own `LayoutSpace` bounds (an `Exact` axis reports the exact extent, `AtMost` caps the measured size, `Unbounded` shrink-wraps to it), measured from its origin (so it is independent of any scroll offset, and `OffsetState` forwards its inner's value unchanged). Every layout state implements it — a column reports its widest child and stacked height, `ManualLayout` the max far-edge of placed rects, etc. `WidgetContext::finish()` reads it and hands the resolved `Rect` to the cleanup closure, which is how a deferred scroll area learns how large its children turned out (see [Scroll Areas](#scroll-areas-windows-and-symmetrical-container-life-cycles)). It returns the origin with zero extent before any child is placed.

**The `pre_layout_*` companion.** Each raw widget that participates has an independent `raw::pre_layout_*(spec, offer, ...) -> *PreLayoutResult`. It takes a dedicated raw pre-layout spec such as `raw::ButtonPreLayoutSpec`, containing only the fields needed before final geometry is known. Geometry, clipping, and draw-only fields are absent unless they genuinely affect the size request.

This keeps the type honest: pre-layout runs before layout, so the widget rect is not available and cannot appear in the pre-layout spec. Callers do not use placeholder rectangles to satisfy a broader raw widget spec; they construct the smaller pre-layout spec directly. Future widget work may allow pre-layout functions to take runtime systems or `&mut State` parameters explicitly for rect-independent state changes, but those dependencies still do not belong inside spec structs.

**High-level flow.** The high-level widget function consumes a complete high-level `*Spec` supplied by the caller: (1) constructs `raw::*PreLayoutSpec` from the pre-layout-relevant fields; (2) calls `raw::pre_layout_*(&pre_layout_spec, offer, ...)`; (3) calls `layout(params, pre_layout.size_request)` to get the real rect; (4) constructs `raw::*Spec` from the high-level spec plus the layout rect and context-managed fields such as clip and layer; (5) calls `raw::post_layout_*(spec, pre_layout, ...)` for one-shot widgets, or `raw::begin_*` for scoped containers. The high-level function does not fill missing high-level spec fields. Theme/default application has already happened explicitly during spec construction. Under `ManualLayout` the size request is computed but ignored - an accepted "double-shape" cost for now (the text is shaped in both pre-layout and raw draw); a later `Layout::WANTS_REQUEST` const can gate it.

#### Deferred-own-size containers

Most containers resolve their **own** bounds upfront. `begin_window` and `begin_scroll_area` call `layout(params, request)` at `begin`, construct a raw spec with the resulting concrete `Rect`, and only then call the raw function — so the raw layer always receives a fully-resolved rect, exactly per the High-level flow above. Their `*Result.layout` is a real `LayoutInfo`.

A `Frame` cannot do this: its size depends on its children (e.g. `Size::Auto` height should shrink-wrap its rows), which are not built until *after* `begin` returns. So `begin_frame` takes the deferred path via `child_with_deferred_layout` / `begin_deferred_layout`:

1. It hands the raw function a **provisional** rect — `Rect::pending_extent(x, y)` — at `begin`. The origin is genuinely known (it comes from the layout's `LayoutSpace`, whose origin is always concrete); only the extent is pending, so `w`/`h` are NaN. The raw `begin_frame` stamps placeholder `FillRect`/`PushClip` commands with this rect.
2. Children are built into the inset space.
3. At `end`, the measured content extent (read via `resolve_space`) is added to the chrome to produce the real bounds. `end_frame` patches the placeholder draw commands in place with that resolved rect.

This is why a `Frame` looks like it breaks the "raw receives a fully-resolved `Rect`" rule but does not: its raw begin function specifically accepts a provisional raw spec for a deferred container lifecycle. Normal leaf widgets receive fully resolved raw specs, and size requests use separate sizing specs. **No layout-level type (`LayoutSpace`, `AxisBound`) ever crosses into the raw layer.** The raw function stays completely layout-agnostic; the provisional-then-patch dance lives entirely in the high-level function and the begin/end command-index plumbing.

**Provisional geometry marker:**
- `Rect::pending_extent(x, y)` (origin set, extent NaN) — "origin known, extent pending". Used for a deferred container's provisional rect between `begin` and the `end` patch.

It keeps the loud-on-misuse property: any arithmetic on the NaN extent yields NaN rather than a plausible-looking wrong number. Future deferred-own-size containers follow the `Frame` template: hand raw a `pending_extent` rect, patch at `end`, and omit `layout` from the high-level result.

#### Trait-Decoupled Stateful Spacers

To support variable spacing between children in sequential layouts (like `RowLayout` and `ColumnLayout`) without creating loop clutter (the "dangling spacer at the end" problem), Framewise employs a **stateful, lazy spacer** mechanism decoupled via traits:

1. **Stateful Deferral (Between)**: By default, a `.spacer()` call (using the `LinearSpacer::Between` variant, which is the default when passing a raw `f32`) does not immediately return a `Rect` or advance the cursor. Instead, it registers a `pending_spacing` offset on the layout state.
2. **Lazy Insertion (Between)**: When the next child widget is placed, the layout state shifts the starting coordinate forward by the pending spacing and clears the accumulator.
3. **Double-Ended Margin Elimination (Between)**:
   - **Leading Spacing Elimination**: If a `Between` spacer is registered before the first child has been placed, it is ignored, avoiding unwanted margins at the layout start.
   - **Trailing Spacing Elimination**: Because the container's resolved outer size is updated based on the right/bottom edge of the placed child *before* the cursor is advanced, any trailing spacer registered after the last child is naturally ignored when the layout closes.
   Together, these rules achieve perfect loop ergonomics: you can place the spacer anywhere in the loop body (first or last) and it will only apply between elements.
4. **Forced Spacing (Always)**: To force spacing at the ends of a layout (e.g. for margins or centering offsets), the `LinearSpacer::Always` variant immediately advances the cursor and extends the layout's resolved content size.
5. **Compile-Time Specialization**: To avoid cluttering non-sequential layouts (like `ManualLayout`), we define a specialized `SpacerLayoutState: LayoutState` trait. The `spacer` method is only exposed on `WidgetContext` when the underlying layout state implements `SpacerLayoutState`. This makes calling `.spacer()` on an incompatible container type a compile-time error.
6. Each layout can choose the parameter used to define a spacer for that particular layout. For linear layouts, this is the `LinearSpacer` enum which implements `From<f32>` (mapping to `Between` by default to preserve simple float invocations).


---

## API Shape

Framewise has two layers:

1. **Raw widget functions** in each widget's `raw` submodule. These are explicit,
   layout-agnostic, context-free functions that receive all runtime systems as
   parameters.
2. **High-level freestanding widget functions** that integrate raw widgets with
   `WidgetContext`, layout, clipping, layering, theme-prepared specs, and output
   accumulation.

The high-level layer consumes complete public `*Spec` values. There is no
`*SpecBuilder` layer. Specs are constructed directly by the caller, transformed
imperatively through fluent methods, and then passed to the high-level widget
function.

This follows the immediate-mode model closely: the app decides what the widget
should be this frame, creates a complete description of that input, and calls the
widget. The widget mutates only its `*State` and writes output commands; it does
not complete or mutate the spec.

---

### Low-Level Raw Widget Functions

Plain, low-level functions live in widget `raw` submodules. They are decoupled
from `WidgetContext`, themes, hidden layout state, and retained widget trees.
Every runtime dependency is passed explicitly. Some lifecycle phases may mutate
widget state, but only through explicit `&mut *State` parameters.

Appending directly to a caller-supplied buffer avoids intermediate `Vec`
allocation and copying, and gives callers stable index-based access to the
command list. The `cmds: &mut DrawCommands` parameter is always last, after all
other inputs.

Raw widgets fall into three lifecycle categories.

**One-shot widgets** have no child `WidgetContext` or child-layout scope.
Examples include button, label, text_edit, checkbox, slider, meter, menu, and
select. Menu and select may draw popup-like content, but in the current
implementation they are still one-shot raw widgets because they do not open a
child context.

The lifecycle is:

```rust
raw::pre_layout_foo(...) -> FooPreLayoutResult
ctx.layout(params, pre_layout.size_request) -> Rect
raw::post_layout_foo(spec_with_rect, pre_layout, ...) -> FooResult
```

`pre_layout_*` runs before the final rect is known, returns a
`*PreLayoutResult` that normally contains `size_request`, may eventually perform
rect-independent stateful widget logic through explicit parameters, must not
draw, must not hit-test against the final widget rect, and should not do
geometry-dependent work.

`post_layout_*` receives the final raw spec containing concrete geometry,
consumes the matching pre-layout result, handles geometry-dependent interaction
and drawing, and can mutate widget state through explicit `&mut State`
parameters.

Example:

```rust
pub struct ButtonPreLayoutResult {
    pub size_request: SizeRequest,
}

pub fn pre_layout_button<T: TextBackend>(
    spec: &raw::ButtonPreLayoutSpec,
    offer: SizeOffer,
    text_backend: &mut T,
) -> raw::ButtonPreLayoutResult;

pub fn post_layout_button<T: TextBackend>(
    spec: raw::ButtonSpec,
    pre_layout: raw::ButtonPreLayoutResult,
    state: &mut ButtonState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut T,
    cmds: &mut DrawCommands,
) -> raw::ButtonResult;
```

**Scoped containers with parent-resolved outer rects** resolve their own outer
rect through the parent layout and then open a child scope. Examples include
scroll_area and window.

The lifecycle is:

```rust
raw::pre_layout_container(...) -> ContainerPreLayoutResult
ctx.layout(params, pre_layout.size_request) -> outer Rect
raw::begin_container(spec_with_outer_rect, pre_layout, ...) -> BeginResult/Token/inner space
children run in child WidgetContext
raw::end_container(token, measured child extent, ...)
```

`begin_*` opens the child drawing/layout scope, emits before-children commands,
and computes inner space, clips, and tokens. `end_*` closes the scope after
children are measured, clamps or resolves state, draws after-children chrome,
pops clips, and performs equivalent cleanup.

Example:

```rust
pub fn pre_layout_scroll_area(
    spec: &raw::ScrollAreaPreLayoutSpec,
    offer: SizeOffer,
) -> raw::ScrollAreaPreLayoutResult;

pub fn begin_scroll_area(
    spec: raw::ScrollAreaSpec,
    pre_layout: raw::ScrollAreaPreLayoutResult,
    state: &mut ScrollState,
    input: &Input,
    focus_system: &mut FocusSystem,
    cmds: &mut DrawCommands,
) -> raw::ScrollAreaResult;

pub fn end_scroll_area(
    token: raw::ScrollAreaToken,
    content_extent: Vec2,
    state: &mut ScrollState,
    input: &Input,
    focus_system: &mut FocusSystem,
    cmds: &mut DrawCommands,
);
```

**Deferred-own-size containers** cannot know their final outer size until their
children have been measured. The current example is frame.

The lifecycle is:

```rust
raw::pre_layout_container(...) -> ContainerPreLayoutResult
ctx.child_with_deferred_layout(...) / begin_deferred_layout(...)
raw::begin_container(provisional_spec, pre_layout, ...) -> token/inner space
children run
raw::end_container(token, final_spec/final rect, ...)
```

`FramePreLayoutResult { size_request }` currently exists for lifecycle
consistency and future pre-layout widget logic, but the high-level deferred
frame path does not feed that size request into `ctx.layout`. The actual parent
placement is still resolved through `begin_deferred_layout` /
`end_deferred_layout`.

Each `raw::*Result` is a concrete struct with no trait requirements on callers,
no metadata maps, and no dynamic type slots. It does **not** contain a
`DrawCommands` field — commands are written directly to the caller's buffer.
(Result structs may derive utility traits such as `Debug` for inspection, but
callers need not implement any traits to receive or use them.)

---

### High-Level Freestanding API: Context Integration

A unified `WidgetContext<'a, T, S, CF>` carries style context (theme, clip
rectangles, current layer, time, layout policy) and system resources (mutable
references to the text backend, focus manager, draw command buffer, output, and
layout state). The `CF` parameter is a one-shot cleanup closure called when the
context is finished; it receives the shared systems and the layout's resolved
space, so container cleanup can both emit post-commands and resolve geometry
from how large the children turned out. Root contexts use a no-op cleanup,
container widgets embed their cleanup in a move closure (see [Scroll Areas,
Windows, and Symmetrical Container Life-Cycles](#scroll-areas-windows-and-symmetrical-container-life-cycles)).

High-level widget APIs are freestanding, ergonomic functions that accept a
complete high-level `*Spec`, layout parameters, optional widget state, and then
`&mut WidgetContext` as the final argument:

```rust
pub fn button<T, S, CF>(
    spec: ButtonSpec,
    layout_params: S::Params,
    state: &mut ButtonState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ButtonResult;
```

Stateless widgets omit `state` but keep `ctx` last:

```rust
pub fn label<'a, T, S, CF>(
    spec: LabelSpec<'a>,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> LabelResult;
```

Scoped containers follow the same ordering principle:

```rust
pub fn begin_scroll_area<T, S, L, CF>(
    spec: ScrollAreaSpec,
    layout_params: S::Params,
    state: &mut ScrollState,
    inner_layout: L,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ScrollAreaResult<...>;
```

The context is deliberately last. It is the largest and most exclusive borrow in
the high-level API. Placing it last lets earlier arguments be constructed inline
using short-lived borrows from the context, especially `&ctx.theme`, before the
mutable context borrow is evaluated:

```rust
drag_number(
    DragNumberSpec::new("Width")
        .max(1920.0)
        .theme(&ctx.theme)
        .format_value(|v: f32| format!("{v:.0}px")),
    rect,
    &mut state,
    &mut ctx,
);
```

If `ctx` were first, `&mut ctx` would be evaluated before the spec argument and
could overlap with the immutable `&ctx.theme` borrow used to construct the spec.
With `ctx` last, the spec is built first, the temporary theme borrow ends, and
then the mutable context borrow begins. This is a practical Rust ergonomics rule,
not merely a style preference.

High-level functions automatically:

1. Query the layout offer when needed.
2. Call the low-level pre-layout function to compute a size request.
3. Resolve layout geometry using the context's layout state.
4. Construct raw specs by adding context-managed values such as `rect`,
   `clip_rect`, `layer`, and `time` to the high-level spec data.
5. Call the low-level raw widget lifecycle functions.
6. Accumulate draw commands inside the context's command buffer.
7. Return a high-level `*Result` to the caller.

High-level functions do **not** resolve missing spec defaults. Specs are already
complete when passed in.

---

### Output Types

Each widget defines result structs reflecting lifecycle phase and API layer.

**`raw::*PreLayoutResult`** is returned by `raw::pre_layout_*`. It contains
`size_request`, may later carry explicit pre-layout bookkeeping, and is consumed
by the matching `post_layout_*` or `begin_*` function.

**`raw::*Result`** is returned by post-layout or begin functions. It contains
interaction and content outputs from the raw widget phase, such as `InputInfo`,
`focused`, or `content_bounds: Rect` when the widget computes an inner area
distinct from the input rect. It does not own draw commands, does not echo the
input rect unless that is a real output, and does not contain `*State`; state is
mutated in place through explicit `&mut *State` parameters.

**High-level `*Result`** is returned by context-integrated high-level functions.
It usually contains `LayoutInfo` with the bounds resolved by the layout engine
and content bounds reported by the raw phase. It may contain a child
`WidgetContext` for scoped containers. It may omit `LayoutInfo` for
deferred-own-size containers where no honest final bounds are known at begin;
`FrameResult` carries only the child context.

The high-level function maps between API layers: it consumes a complete
high-level `*Spec`, constructs the raw pre-layout spec, obtains a `SizeRequest`
from `raw::pre_layout_*`, resolves the real rect when the lifecycle permits
immediate placement, constructs the raw widget spec with resolved geometry and
context-managed values, calls `raw::post_layout_*` or `raw::begin_*`, and then
constructs the high-level `*Result`.

Nesting a child layout is done with `ctx.child_with_layout(placement,
inner_layout)`: it resolves `placement` against the current layout to get the
child's bounds, begins `inner_layout` at those bounds, and returns a child
`WidgetContext`. Container widgets that compute their own bounds, such as scroll
areas and windows, instead use the
`child_with_layout_and_on_finish[_and_clip_rect]` variants, which take an
already-begun layout state plus a self-derived clip.

---

### Spec, Pre-Layout Spec, and Raw Spec

Every widget type follows a consistent layered configuration pattern:

- **High-level `*Spec`**: The ergonomic user-facing configuration struct passed
  directly to the high-level context function. It contains only fields that are
  meaningful for high-level callers, such as content, style, behaviour flags,
  domain values, and caller-provided callbacks. It does not contain
  layout-resolved fields such as `rect`, or context-managed fields such as
  `clip_rect`, `layer`, and `time`.

- **`raw::*PreLayoutSpec`**: A low-level pre-layout specification struct used by
  `raw::pre_layout_*`. It contains only value fields needed before final
  geometry is known. For example, `raw::ButtonPreLayoutSpec` contains the button
  text and style, but not `rect` or `clip_rect`.

- **`raw::*Spec`**: A fully resolved low-level specification struct used by the
  raw widget function. All fields are concrete values needed to draw and
  interact with the widget, including geometry such as `rect` and
  context-managed values such as `clip_rect` and `layer`. It is defined inside
  the widget's `pub mod raw {}` submodule (e.g. `button::raw::ButtonSpec`),
  co-located with the raw function that consumes it, and avoids cluttering the
  normal module level with details high-level users do not need.

There are no `*SpecBuilder` structs. Builder-like ergonomics are provided by
methods on the complete high-level `*Spec` itself.

This pattern cleanly separates concerns:

- **High-level specs are complete frame input.** The app constructs exactly the
  widget configuration it wants for the current frame. There is no hidden
  partial state and no late high-level default injection inside the widget call.
- **Low-level raw functions are explicit and testable.** They receive every
  runtime dependency as an explicit parameter and do not depend on
  `WidgetContext`, themes, hidden layout state, or retained widget trees. Some
  lifecycle phases may mutate widget state, but only through explicit
  `&mut *State` parameters.
- **Pre-layout sizing is type-safe.** Pre-layout specs cannot accidentally
  contain or read fields that are unavailable before layout.
- **High-level functions are integrated.** They handle layout, bridge from
  high-level specs to raw specs, and hide low-level geometry/context plumbing.

> [!IMPORTANT]
> **Spec Value-Type Rule:** High-level `*Spec`, `raw::*PreLayoutSpec`, and
> `raw::*Spec` structs must not include references to runtime resources like
> `Input`, `FocusSystem`, a text backend, `WidgetContext`, draw buffers, or
> other external mutable state. These runtime systems are always passed to
> lifecycle functions explicitly. Specs may contain value data such as colours,
> fonts, rectangles, strings, numeric values, and caller-provided callback
> fields. Callback fields are ordinary spec fields: they are frame input supplied
> by the caller, and the widget may call them but must not store references to
> runtime systems inside them.

> [!IMPORTANT]
> **Theme Must Not Appear in Specs:** A high-level `*Spec`, raw pre-layout spec,
> or raw widget spec must never hold a `Theme` field or references into a
> `Theme`. `Theme` is a high-level convenience that maps semantic intent to
> concrete values. A spec may be constructed from a theme, or have theme-derived
> fields overwritten by `.theme(&theme)`, but it stores only the resolved
> primitives such as colours, dimensions, and font handles. This keeps every spec
> self-contained and renderer-agnostic, prevents long-lived borrows of
> `ctx.theme` from conflicting with `&mut WidgetContext`, and keeps the raw layer
> independent of the theme system.

---

### Complete Spec Construction

A high-level `*Spec` is complete immediately after construction. There are no
unset fields, and there is no `build()` step. Fluent setter methods consume and
return `Self` (or a new generic `Self` when changing a callback type), so method
chains are normal imperative transformations on a complete value.

Specs use these construction methods consistently:

- **`Spec::default()`** is available when every field has a meaningful
  context-independent default. It should not invent fake required content just
  to satisfy the type system.
- **`Spec::default_from_theme(&theme)`** is available when `Default` is
  meaningful and the widget has theme-derived fields. It is equivalent to
  `Spec::default().theme(&theme)`.
- **`Spec::new(required_args...)`** is used when the widget has required
  semantic inputs with no honest default, such as button text, label text, menu
  items, or select values. Required inputs are constructor parameters, not
  panics deferred to a later `build()`.
- **`Spec::new_from_theme(&theme, required_args...)`** is the themed equivalent
  of `new(...)`. It is equivalent to `Spec::new(required_args...).theme(&theme)`.

Use `new_from_theme`, not `new_with_theme`, so the name pairs naturally with
`default_from_theme`.

Examples:

```rust
let spec = ButtonSpec::new_from_theme(&ctx.theme, "Save")
    .disabled(is_saving);

let spec = LabelSpec::new("Ready")
    .theme(&ctx.theme);

let spec = ScrollAreaSpec::default_from_theme(&ctx.theme)
    .vertical(ScrollAxis {
        extent: ScrollExtent::SCROLL,
        vis: ScrollbarVisibility::Auto,
    });
```

`Option<T>` remains valid inside specs only when `None` is a real domain value:
`border: Option<Stroke>` means no border, `peak: Option<f32>` means no peak
marker, `value_snap: Option<f32>` means continuous values. `Option<T>` is not
used to mean "the user has not set this field yet".

---

### Theme Application as an Explicit Transformation

Every high-level spec with theme-derived fields provides a fluent
`theme(&Theme) -> Self` method. This method applies theme-derived visual values
to the spec by overwriting the relevant fields:

```rust
impl ButtonSpec {
    pub fn theme(mut self, theme: &Theme) -> Self {
        self.style = ButtonStyle::secondary_from_theme(theme);
        self
    }
}
```

`theme()` is not a fallback and does not know what the user previously set.
Without builder `Option` fields, there is no concept of "missing" values. It is
an ordinary imperative transformation: it reads the current spec, computes the
theme-derived fields, writes them, and returns the updated spec.

The order of calls therefore matters and should be used deliberately:

```rust
// Semantic options first, then theme, then final visual overrides.
let spec = TextEditSpec::new()
    .multiline_wrapped()
    .theme(&ctx.theme)
    .placeholder("Notes");

// This means something different: wrap changes after the theme-derived style
// was computed, so any wrap-dependent style adjustment is not recomputed.
let spec = TextEditSpec::new()
    .theme(&ctx.theme)
    .multiline_wrapped();
```

This is intentional. The API follows normal imperative programming semantics:
later transformations see earlier transformations and may overwrite fields set
by them. This is often clearer than a hidden delayed-default phase because the
source order describes what happens.

The recommended order is:

```rust
WidgetSpec::new(required_args...)
    .semantic_options_that_theme_should_react_to(...)
    .theme(&ctx.theme)
    .visual_overrides(...)
    .callback_overrides(...)
```

For most widgets, `theme()` simply replaces `style` with
`*Style::from_theme(theme)` or a variant such as
`ButtonStyle::secondary_from_theme(theme)`. Some widgets may make theme-derived
style depend on existing semantic fields. For example, `TextEditSpec::theme`
may inspect `wrap` and `newline_policy` before setting the default padding:

```rust
impl TextEditSpec {
    pub fn theme(mut self, theme: &Theme) -> Self {
        let multiline = self.newline_policy == NewlinePolicy::Preserve || self.wrap;

        let mut style = TextEditStyle::from_theme(theme);
        style.padding_y = if multiline { 8.0 } else { 0.0 };

        self.style = style;
        self
    }
}
```

A setter such as `.wrap(true)` should set only the `wrap` field. It should not
silently modify padding or other style details. If the caller wants theme style
to react to `wrap`, they call `.wrap(true)` before `.theme(&theme)`. If the
caller wants to preserve the current style while changing only wrapping, they
call `.wrap(true)` after `.theme(&theme)` or after a custom `.style(...)`.

Theme-derived fields must be copied or cloned out of the theme into the spec.
Specs must not store references into the theme. This is essential for both
layering and borrow-checker ergonomics: inline spec construction may borrow
`&ctx.theme` before passing `&mut ctx` to the widget, and that borrow must end
when spec construction finishes.

---

### Callback Fields in Specs

Caller-provided callbacks are frame input, so they belong directly in the
high-level `*Spec` when a widget needs them. Do not create a separate callbacks
bundle merely to avoid putting code fields in specs. A callback is still part of
"what the caller provides for this frame"; the widget calls it but does not
mutate or retain it.

For example, a drag-number value formatter can be a direct generic field:

```rust
pub type DefaultDragNumberValueFormatter = fn(f32) -> String;

pub fn default_drag_number_value_formatter(value: f32) -> String {
    format!("{value:.2}")
}

pub struct DragNumberSpec<'a, F = DefaultDragNumberValueFormatter>
where
    F: Fn(f32) -> String,
{
    pub text: &'a str,
    pub style: DragNumberStyle,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub page_step: f32,
    pub format_value: F,
    pub disabled: bool,
}
```

The default callback is a real function pointer value, not `None` and not a
hidden fallback inside the widget:

```rust
impl<'a> DragNumberSpec<'a, DefaultDragNumberValueFormatter> {
    pub fn new(text: &'a str) -> Self {
        Self {
            text,
            style: DragNumberStyle::fallback(),
            min: 0.0,
            max: 100.0,
            step: 1.0,
            page_step: 10.0,
            format_value: default_drag_number_value_formatter,
            disabled: false,
        }
    }
}
```

A callback setter may change the spec's generic callback type:

```rust
impl<'a, F> DragNumberSpec<'a, F>
where
    F: Fn(f32) -> String,
{
    pub fn format_value<G>(self, format_value: G) -> DragNumberSpec<'a, G>
    where
        G: Fn(f32) -> String,
    {
        DragNumberSpec {
            text: self.text,
            style: self.style,
            min: self.min,
            max: self.max,
            step: self.step,
            page_step: self.page_step,
            format_value,
            disabled: self.disabled,
        }
    }
}
```

Call site:

```rust
drag_number(
    DragNumberSpec::new("Width")
        .max(1920.0)
        .theme(&ctx.theme)
        .format_value(|v: f32| format!("{v:.0}px")),
    rect,
    &mut state,
    &mut ctx,
);
```

Callbacks should be named as actions the widget performs, such as
`format_value`, `map_drag_delta`, or `accessibility_label`, rather than vague
names such as `formatter` or `callback`. The field name should read well at both
the call site and the use site:

```rust
let value_text = (spec.format_value)(state.value);
```

A separate `*Callbacks` struct is appropriate only if a widget has enough code
hooks that grouping them materially improves readability. Even then, the
callbacks object should be a complete value with real defaults, not a bag of
`Option` slots interpreted later by the widget.

#### Derives and Generic Callback Fields

A spec containing generic callback fields can derive traits only when the
callback type also implements those traits. Function pointers normally support
common traits such as `Copy`, `Clone`, `Debug`, and `PartialEq`; many closure
types do not support `Debug` or `PartialEq`.

This is an accepted consequence of making callbacks part of specs. Equivalent
spec structs should still follow the same derive policy where the fields allow
it, but generic callback specs may have conditional derives or manual impls with
bounds. Avoid designing APIs that require custom closures to be `Debug` or
`PartialEq` just to be usable as widget input.

---

### Spec Field Visibility and Fluent Methods

High-level `*Spec` fields are public unless there is a specific invariant that
requires private fields. Public fields allow ergonomic struct-literal
construction, direct inspection in tests, and easy copying between specs.
Fluent setter methods remain the preferred public construction style because
they encode common transformations and keep call sites readable.

The setter convention is:

```rust
pub fn disabled(mut self, disabled: bool) -> Self {
    self.disabled = disabled;
    self
}
```

Setters should set exactly the named field unless their name clearly describes a
larger preset. For example, `wrap(true)` sets only `wrap`; `multiline_wrapped()`
may set `newline_policy`, `wrap`, vertical alignment, and line alignment because
it is explicitly a preset.

If a setter changes a generic callback field, it may return a new `Spec` type
with a different generic parameter. Keep such setters explicit and rare.

Raw specs and pre-layout specs may also have public fields because they are
plain explicit data passed to raw functions. They are lower-level APIs and do
not provide theme/default conveniences.

---

### Default Implementations — Spec and Style

**High-level `*Spec` structs — complete defaults only when honest**

A high-level `*Spec` may implement `Default` only if all fields have meaningful
context-independent values. Do not invent fake required content just so every
spec can be default-constructed. A label with empty text, a menu with no items,
or a select with no value may be valid in some contexts, but if that is not the
honest common default, the widget should use `new(required_args...)` instead.

When `Default` is implemented, it returns a complete usable spec, including real
callback defaults and non-theme fallback visuals where necessary. The fallback
visuals should be simple and valid, not a duplicate of the theme. The themed
constructor remains the preferred path for normal app UI:

```rust
let spec = SpinnerSpec::default_from_theme(&ctx.theme);
```

**`*Style` structs — theme is the source of normal styling**

The authoritative source of normal style values is the `*Style::from_theme()`
(or `*Style::*_from_theme()` for multi-variant styles) methods defined directly
on each style struct. A `*Style` struct may implement `Default` only if there is
a useful context-independent fallback. Such a fallback must not be treated as
the normal app style and must not duplicate the theme. It exists so specs can be
complete before `.theme(&theme)` is applied, not to replace theme-derived
styling.

**Theme constructors — no hidden fallback phase**

`Spec::default_from_theme(&theme)` and `Spec::new_from_theme(&theme, ...)` are
ordinary constructors. They call `theme()` explicitly as part of construction.
The high-level widget function does not call `theme()` internally, because doing
so would hide ordering, overwrite user visual overrides unexpectedly, and make
callbacks/defaults harder to reason about.

**Real optional fields stay optional**

Some spec fields are `Option<T>` because `None` is a meaningful resolved value:
no border, no focus outline, no peak marker, no value snapping. These fields do
not need an extra `Option<Option<T>>` wrapper, because there is no builder layer
that must distinguish "unset" from "explicitly set to None". The spec contains
the actual semantic value directly.

---

### Style Structs

Some widget types group their styling fields into a dedicated `*Style` struct
embedded inside `*Spec`. The decision rule:

- **Use a `*Style` struct** when the widget has interaction states (hover,
  press, focus, disabled) or several coordinated colour/dimension roles. The
  style struct keeps the spec readable and lets callers pass a single
  `ButtonStyle` override rather than setting a dozen fields individually.
- **Embed styling fields directly in `*Spec`** when the widget is purely
  display-only and has only a small number (roughly ≤ 3) of styling fields. A
  dedicated struct would be ceremony with no benefit for these simple cases.

The practical dividing line is interaction states: as soon as a widget needs
distinct visuals for hover, focus, or disabled, the coordinated colour roles
naturally belong in a `*Style` struct. Pure display widgets without those states
may keep their styling inline.

Style structs own resolved visual values. They do not store themes or borrow
from themes. A style helper such as `ButtonStyle::secondary_from_theme(&theme)`
translates theme tokens into concrete colours, dimensions, and font handles.

Example:

```rust
// Low-level: fully resolved, no theme/defaults.
pub fn pre_layout_button<T: TextBackend>(
    spec: &raw::ButtonPreLayoutSpec,
    offer: SizeOffer,
    text_backend: &mut T,
) -> raw::ButtonPreLayoutResult;

pub fn post_layout_button<T: TextBackend>(
    spec: raw::ButtonSpec,
    pre_layout: raw::ButtonPreLayoutResult,
    state: &mut ButtonState,
    input: &Input,
    focus_system: &mut FocusSystem,
    text_backend: &mut T,
    cmds: &mut DrawCommands,
) -> raw::ButtonResult;

// High-level: consumes a complete ButtonSpec and integrates it with context.
pub fn button<T, S, CF>(
    spec: ButtonSpec,
    layout_params: S::Params,
    state: &mut ButtonState,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ButtonResult {
    let pre_layout_spec = raw::ButtonPreLayoutSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_button(&pre_layout_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ButtonSpec {
        rect,
        text: spec.text,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        disabled: spec.disabled,
        layer: ctx.layer,
    };
    let r = raw::post_layout_button(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.text_backend,
        ctx.cmds,
    );
    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}
```

---

### User-Defined Layouts Are First-Class

Built-in layouts hold no privileged position. The two public traits — `Layout`
and `LayoutState` — are the complete extension point:

- **`Layout`** defines the configuration type (`type Params`) and a
  `begin(space: impl Into<LayoutSpace>) -> Self::State` method that initialises
  the mutable state.
- **`LayoutState`** is the mutable engine: `layout(params, request) -> Rect` for
  normal widgets, `begin_deferred_layout` / `end_deferred_layout` for
  fit-to-children containers, and `resolve_space() -> Rect` so scroll areas and
  `finish()` can read the accumulated content **resolved against the layout's
  own `LayoutSpace` bounds** (an `Exact` axis reports the exact extent, `AtMost`
  caps the measured size, `Unbounded` shrink-wraps to it).

A user-defined layout implements both traits, passes its state type into
`WidgetContext::child_with_layout`, and is otherwise identical to `ColumnLayout`
or any other built-in. No library modification is required; no registration step
exists. The built-ins are examples of the pattern, not gatekeepers of it.

---

### User-Defined Widgets Are First-Class

Built-in widgets hold no privileged position in the architecture. `Theme` is a
library-defined struct — callers cannot add methods to it. If themed style
defaults required a `theme.xxx_style()` method on `Theme`, only built-in widgets
could participate; user-defined widgets would have no equivalent path.

By placing the conversion on the style/spec type itself — `*Style::from_theme`,
`*Style::*_from_theme`, `*Spec::theme`, and `*Spec::*_from_theme` — the pattern
is fully open to extension. A user-defined widget follows exactly the same
design as a built-in one:

1. Define a high-level complete `*Spec` struct containing the caller-facing
   fields.
2. Define a `*Style` struct if the widget has enough coordinated visual fields
   to justify one.
3. Implement `*Style::from_theme(&theme)` or variant style constructors where
   useful.
4. Implement `*Spec::new(...)`, `*Spec::default()` where honest,
   `*Spec::theme(&theme)`, and `*_from_theme` constructors.
5. Define `raw::*PreLayoutSpec`, `raw::*Spec`, and raw lifecycle functions.
6. Define a high-level freestanding widget function that consumes `*Spec`,
   layout params, optional `&mut *State`, and finally `&mut WidgetContext`.

No library modification required. No special registration. The library's own
widgets are simply examples of the pattern, not gatekeepers of it.

---

### Theme and Font Boundaries

`Theme` is part of the high-level API. It maps semantic UI intent to concrete
colours, spacing, sizes, and font handles. It must not leak into raw widget
functions or be stored inside specs. A high-level spec can be produced from a
theme, but by the time any widget lifecycle function receives the spec, the
theme has already been translated into resolved primitives.

> [!IMPORTANT]
> **Static Check Rule:** Low-level raw widget functions must not import or
> depend on `theme::Theme`. Raw functions accept only fully resolved
> `raw::*PreLayoutSpec`, `raw::*Spec`, `*Style`, and runtime system parameters.
> Theme is consumed only by high-level construction helpers such as
> `*Style::from_theme`, `*Style::*_from_theme`, `*Spec::theme`,
> `*Spec::default_from_theme`, and `*Spec::new_from_theme`. Widget files may
> import `Theme` for these higher-level helpers, but the import must not be used
> inside `raw` submodules.

Fonts follow the same rule. A font is an application-owned handle independent of
any theme. A theme references the handles it wants to use for sans and mono
text, but it does not own renderer-specific font data. The spec or style copies
font handles from the theme when theme helpers are called; direct raw callers
choose fonts explicitly, often by copying a handle from a theme themselves.

---

## Input Focus

A core challenge of immediate-mode and one-pass GUI architectures is handling keyboard focus traversal (Tab / Shift+Tab) when the "next" widget might not have been evaluated yet.

Framewise solves this by embracing a **one-frame delay**:

1. Every focusable widget carries a `FocusId` in its app-owned state (like `ButtonState`). This ID is globally unique and persists across frames.
2. The app stores a `FocusSystem` and passes it mutably into widgets.
3. On **Frame N**, as widgets are evaluated, they register their `FocusId` with the `FocusSystem`. The system builds a sequential `current_frame_order`.
4. If the user presses Tab, a shift is requested. At the **end of Frame N**, the `FocusSystem` finds the currently focused widget's index in `current_frame_order` and picks the next (or previous) ID to become the new focus target.
5. On **Frame N+1**, the newly targeted widget registers its ID, sees that it is the focus target, and draws its focus state.

This gives the application total control over focus ordering. The default is implicit call order, but the app can explicitly insert overrides (`override_next`) to jump focus between disconnected parts of the UI without relying on string hashing or retaining a global UI tree.

---

## The Three Routing Problems

Because Framewise lacks a retained UI tree, routing user input to the correct widget requires careful architectural thought. These challenges fall into three categories:

### 1. Persistent Interaction (Mouse Capture)

- **The Problem:** When you click a button and drag the mouse over another button, the second button shouldn't receive a click when you release.
- **The Solution:** *Purely Local State.* We use the app-owned state (e.g., `ButtonState`) to record `pressed = true`. As long as that specific struct remembers it was pressed, it captures the interaction and ignores bounds checks.
- **Why it works:** The interaction starts with a definitive historical event ("Mouse Down") that locks the state. It requires 0 frames of lag and no global ID registry.

### 2. Sequential Interaction (Keyboard Focus Tabbing)

- **The Problem:** Pressing 'Tab' should move focus to the "next" widget, but in top-down evaluation, the "next" widget hasn't been evaluated yet.
- **The Solution:** *1-Frame Delay + Global ID.* Widgets register their `FocusId` in sequence. At the end of Frame N, the `FocusSystem` determines the next ID. In Frame N+1, the new widget claims focus.
- **Why it works:** The spatial relationship ("who is next") is only known *after* the entire UI is evaluated, forcing us to accept a 1-frame delay managed by a central system.

### 3. Spatial Overlap Interaction (Hover & Scrolling)

- **The Problem:** Multiple elements overlap the exact same pixel coordinates under the mouse (e.g., two overlapping window buttons, or a scroll area nested inside another scroll area). How do we route hover, click, and scroll wheel events to the correct widget?
- **The Solution:** *1-Frame Delay + Central Tracking.* Widgets register claims in the central `FocusSystem` during Frame N. In Frame N+1, only the widget holding the active claim is allowed to capture the interaction.
- **Why We Need Distinct Claiming Systems:**
  While it seems like hover, click, and scroll wheel events could share a single claim system, they have fundamentally opposite routing rules that require separate tracking:
  1. **Mouse Hover & Click Claiming (Last-Caller-Wins):**
     * **Rule:** Depth-based. The widget drawn *last* (top-most) should receive the hover state and click inputs.
     * **Mechanism:** As widgets evaluate top-down, if a widget contains the mouse cursor, it registers a hover claim via `focus_system.claim_hover(id)`. Each successive widget containing the mouse overwrites the previous claim, ensuring the last (top-most) widget wins.
  2. **Hover Scroll Claiming (First-Caller-Wins / Bottom-Up):**
     * **Rule:** Hierarchy-based. The *innermost* scrollable container should get first pick of the scroll wheel event. If the innermost container is at its boundary, scrolling should "bubble" up to parent containers.
     * **Mechanism:** Containers finish and teardown bottom-up (innermost first). The innermost container claims the scroll event first (`claim_scroll_up`, etc.). Parent containers finishing later check if a claim has already been registered; if so, they respect it and do not overwrite it. If the child container is at its limit and declines to claim, the parent's claim succeeds, enabling natural nested scrolling.
- **The Guiding Principle:** Why not solve this locally by having the inner widget consume the event bottom-up when its scope closes? Because doing so would mutate the widget's local state *after* it has already laid out its children. This violates a core Framewise principle: **If local state is modified in Frame N, it must visually reflect in Frame N.** If a state change must be delayed to Frame N+1 (due to top-down evaluation constraints), that pending intent must be explicitly stored in a central system (like `FocusSystem` or `InteractionSystem`), not quietly hidden inside local widget state.

- **Composite Focus Propagation:** Hover claims remain authoritative for deciding which overlapping widget owns mouse input. Container/composite widgets such as `text_edit` should not bypass hover claims to handle clicks on child affordances. Instead, hierarchical focus behavior is propagated explicitly through widget result structs: for example, a `ScrollArea` can report that one of its scrollbars is pressed, and the parent `TextEdit` can use that signal to take keyboard focus while the scrollbar continues to own pointer interaction.

- **Disabled Widgets Still Occlude:** Disabled widgets should still make hover claims when the pointer is over their bounds. Otherwise, a disabled widget visually layered above an enabled widget might fail to block mouse input to the enabled widget underneath.

---

## Text Rendering

Text rendering is notoriously complex (shaping, hinting, atlas caching) and is a common source of hidden costs in immediate-mode GUIs. Framewise handles this by strictly separating **preparation** from **rendering**.

To draw text, the widget building pass must have access to a `TextBackend` provided by the application.

- **Layout pass:** The widget calls `layout_text(...)`. Framewise asks the backend to shape text and provide line metrics, then builds an owned `TextLayout` containing nested working line/cluster state plus caret, hit-test, and metrics data.
- **Emission pass:** The widget calls `TextLayout::emit_glyphs(...)`. Framewise asks the backend to prepare each visible drawable layout glyph at its final glyph origin. Returned `DrawGlyph`s are appended to the `DrawCommands` glyph arena and referenced by `DrawCmd::GlyphRun`.
- **Render pass:** The renderer reads each glyph run, resolves every `PreparedGlyphHandle` through backend/application resource tables, and draws each prepared bitmap at `DrawGlyph::top_left`.

Because the `WidgetContext` takes the text backend as a generic parameter (`WidgetContext<'a, T: TextBackend, S>`), Framewise keeps static dispatch and renderer agnosticism without storing renderer-facing text layout handles.

### Logical Layout Bounds and Ink Bounds

A major visual challenge in GUI layouts is aligning text containers perfectly with other visual elements such as borders, button centers, input fields, and card edges. Text has two different kinds of geometry, and treating one as the other produces subtle bugs:

- **Logical layout bounds** describe the space used for text flow: advances, baselines, line height, wrapping, ellipsis, caret placement, selection, and hit-testing.
- **Approximate ink bounds** describe layout-time visible extents estimated from shaped glyph outline/control bounds. Exact raster ink exists only after glyph preparation and emission.

Framewise treats text bounds as **logical layout constraints**, not promises that all ink will be contained inside the supplied rectangle.

For `layout_text(text_backend, text, style, TextBounds)`, `TextBounds` answers: "what logical space is available for shaping, wrapping, alignment, and overflow policy?" A bounded width constrains line breaking and horizontal overflow handling. A bounded height constrains which visual lines are admitted. These inputs are available before final ink is known, so they cannot honestly be tight ink boxes.

For drawing, widgets lay text out against the concrete logical text block size and pass the block origin to `TextLayout::emit_glyphs(...)`. The logical block supplies the wrap width, vertical extent, and alignment frame. The renderer or widget may still choose to clip drawing to this rect, but clipping is a rendering policy; it is not the text layout contract.

The `TextMetrics` returned by the interface reports both the resulting **logical** block size and the resulting **approximate ink bounds** after shaping and overflow policy. Under strict policies (`Drop`, successful wrapping, successful ellipsis fitting), the logical size should stay within the provided logical constraints. Policies that explicitly keep overflowing content (`Keep` fallbacks and `OverflowY::Keep`) may report a logical size that exceeds the input constraints; that is the selected overflow behavior, not a contract violation.

Approximate ink bounds are related to logical bounds but are not contained by them in general. The ink may sit wholly inside the logical box, protrude to any side, be much smaller than the logical box, be empty for whitespace, or extend beyond the logical box due to italic overhangs, negative side bearings, accents, combining marks, symbol glyphs, or custom font behavior. The relationship is intentionally loose. Exact drawn bounds must be derived after `TextLayout::emit_glyphs` from `DrawGlyph::top_left` plus resolved prepared glyph image sizes.

#### Why Logical Bounds Are the Text Input

1. **Wrapping and editing are advance-based.** Text flow is driven by shaped cluster advances. A cluster is the smallest indivisible shaped text unit emitted by the backend; it should not split combining marks, ligatures, or script-shaped units in a way that would corrupt shaping. Spaces have advance but no ink; combining marks may have ink but little or no advance. Wrapping by ink would make ordinary text unstable and would make caret and hit-testing behavior harder to reason about.
2. **The ink box is an output of shaping and rasterization.** The caller cannot provide a tight ink rect before the backend has shaped the string and before glyph preparation has selected subpixel bins, hinting, bitmap placement, and renderer resources.
3. **Overflow policy must be explicit.** A caller that needs hard pixel containment should request clipping or a future ink-fit policy. A caller that passes a logical rect should not assume that visible ink cannot spill outside it.
4. **Different widgets want different alignment bases.** Text labels, editable text, menus, and paragraphs usually want logical centering/alignment. Icon-like glyphs and optical badges may want ink centering. Keeping both concepts explicit lets each widget choose the correct behavior.

#### Practical Consequences

- Regular text layout, wrapping, caret geometry, and hit-testing operate in logical block coordinates.
- Widgets that require strict visual containment must clip, add padding, or use a future ink-fitting policy.
- Widgets that want optical alignment should use `TextMetrics::approx_ink_bounds`, rather than assuming logical metrics describe visible pixels.
- Labels, buttons, and icon-like text can deliberately choose between logical and optical alignment by choosing whether they align against `TextMetrics::logical_size` or `TextMetrics::approx_ink_bounds`.

### Alignment Terminology

Framewise has several alignment concepts that sound similar but operate at different layers. They should stay separate in naming, documentation, and implementation.

1. **`TextFlow::line_align`** positions each shaped line horizontally inside the logical text layout block supplied to Framewise text layout. It is per-line text flow policy. It does not move the widget, does not choose the text block's vertical position, and does not change text measurement, wrapping, or truncation.
2. **Layout `Align`** positions a child widget inside the available parent layout space on one axis. It is parent-to-child widget placement, used through types such as `Placement` and `Placement2D`. It moves the widget's resolved `Rect`.
3. **Widget text/content placement** positions the prepared text block inside the widget's own content rect. It is local to widgets such as labels and buttons. It does not move the widget in its parent, and it should not be implemented by changing `TextFlow::line_align`.

*Note on `TextEdit`*: The editable text input widget (`TextEdit`) makes use of two of these layers:
- It uses **Widget text/content placement** (specifically a `vertical_align: Align` property) to vertically align the entire prepared text block (top, center, or bottom) inside the viewport when the content fits.
- It uses **`TextFlow::line_align`** (specifically a `line_align: TextLineAlign` property forwarded to `TextStyle`) to horizontally align individual lines (left, center, or right) inside the text layout. When `TextBounds::max_width` is provided, text layout aligns lines inside that width. When no maximum width is provided, it aligns lines inside the maximum logical width of the laid-out lines.

For unwrapped text, `TextEdit` lays out using unbounded horizontal bounds so text remains horizontally scrollable. The resulting lines are aligned within their natural block width by the text system. If the viewport is wider than that natural block, `TextEdit` applies a single block-level x-offset to position the whole block according to the requested line alignment. Because all editing logic, caret rendering, selection highlight bounds, and hit-testing in `TextEdit` are evaluated using block-local coordinates relative to the text block's origin, this block offset is applied through the shared origin rather than by mutating line positions.


The proposed label/button property should therefore be named for content placement rather than plain alignment, for example:

```rust
pub struct TextContentPlacement {
    pub x: Align,
    pub y: Align,
    pub basis: TextContentBasis,
}

pub enum TextContentBasis {
    Logical,
    Ink,
}
```

`Align` is already the reusable one-dimensional `Start | Center | End` enum. It can represent horizontal placement (`Start` = left, `End` = right in physical widget coordinates) and vertical placement (`Start` = top, `End` = bottom). Reusing it avoids introducing parallel `Left/Middle/Right` and `Top/Middle/Bottom` enums with identical math.

If this reuse feels confusing in API docs, the fix should be to broaden `Align`'s documentation from "cross-axis layout alignment" to "one-dimensional alignment inside a containing extent", then document that layout and content placement use the same primitive for different owners. The owning struct name (`Placement2D` versus `TextContentPlacement`) carries the semantic distinction.

For text content placement, the `basis` field chooses which measured text geometry is aligned inside the widget content rect:

- `Logical` aligns the text block using `TextMetrics::logical_size`. This is the normal choice for labels, button captions, paragraphs, and editable text.
- `Ink` aligns the approximate visible ink using `TextMetrics::approx_ink_bounds`. This is useful for optical centering of icon-like text, emoji, symbols, and badges whose visible pixels do not match their logical advance box.

The widget should still lay out text against a logical text block rect. Ink-based placement adjusts that rect so the reported approximate ink bounds land at the requested position inside the widget content rect.

---

## Colour Pipeline

Framewise uses a **linear-light colour pipeline** throughout.

### Why linear?

All blending, interpolation (`lerp`), and brightness operations (`darken`) are physically correct only in linear light space. In sRGB space these operations produce dark midpoints — most visibly in hover/press transitions and gradients.

### The contract

* **`Color` always stores linear RGBA.** The struct fields `r`, `g`, `b`, `a` are all linear-light values in [0.0, 1.0].
* **Alpha is never gamma-encoded.** All constructors treat `a` as linear.
* **The renderer framebuffer is sRGB.** The GPU hardware encodes linear values to the sRGB display curve at no CPU cost. The `init_wgpu` function selects the first sRGB surface format available (`surface_caps.formats.iter().find(|f| f.is_srgb())`).

### Constructing colours

| Source                          | Constructor                    |
|---------------------------------|--------------------------------|
| Hex code / 8-bit palette values | `Color::from_srgb_u8(r,g,b,a)` |
| `0xRRGGBB` hex literal          | `Color::from_srgb_hex(0xRRGGBB)` |
| f32 values expressed as sRGB    | `Color::from_srgb_f32(r,g,b,a)` |
| Already-linear f32 values       | `Color::linear_rgba(r,g,b,a)`  |

`Color::linear_rgba` and `Color::linear_rgb` take **linear** inputs and are intended for programmatic construction (e.g. copying components from another `Color` with a different alpha). Do not pass perceptual sRGB values to these; use the `from_srgb_*` variants instead.

### Theme colours

All palette entries in `Theme::framewise()` are defined as sRGB hex/u8 values (matching design-tool exports) and decoded to linear at construction time. Derived colours that carry the ink or rust RGB at reduced opacity use `Color::from_srgb_f32` so the RGB channels receive the same decode.

---

## Draw Pipeline

```
App draw function
  ├── creates root DrawCommands buffer
  ├── widget calls → DrawCommands accumulated into shared buffer
  ├── WidgetContext::finish() → () [appends on_finish post-cmds into same buffer]
  └── App passes &DrawCommands to Renderer
        └── Renderer consumes draw list (batching, GPU submission)
```

The semantic work (layout, interaction, hit-testing) happens entirely in the first stage. The render stage is mechanical: no layout, no binding resolution, no hidden updates.

---

## Scroll Areas, Windows, and Symmetrical Container Life-Cycles

Design decisions around how complex container widgets (Scroll Areas and Windows) interact with layout, clipping, and nested inputs.

- **Decorator Layouts**: Layouts like `OffsetLayout<L>` are pure decorators. They wrap another layout and modify the returned rectangles (e.g. subtracting an offset). They do NOT track rendering state, apply clipping, or hold application state.

- **Fit-to-Children Containers (Opt-in Sizing)**: Container widgets (such as `frame`) can choose to **opt in** to discovering their children's bounds to dynamically size themselves bottom-up. Standard single-pass leaf widgets call `layout(params, request)` to obtain their concrete bounds in one go. In contrast, container widgets that want to fit to their content size opt into the deferred layout pattern using a compile-safe token-borrow model:
  - **The Opt-In Pattern**:
    1. The container calls `begin_deferred_layout(layout_params) -> (LayoutSpace, LayoutToken<'a>)` instead of `layout()`. This mutably borrows the parent `LayoutState` for the lifetime of the returned `LayoutToken`, preventing any sibling layout calls from being made on the parent context while the token lives (statically borrow-enforcing the evaluation sequence).
    2. The container inspects the generic `LayoutSpace` bounds (`AxisBound`) to make its own sizing policy decisions: if an axis is `Unbounded` or `AtMost`, the container will size itself bottom-up to its children; if it is `Exact(w)`, it honors the parent's rigid constraints. It subtracts padding/borders via `space.inset(amount)` to yield the available child space.
    3. The container creates a child `WidgetContext` with a custom `on_finish` closure, capturing the `LayoutToken` by value.
    4. Sibling widgets are laid out sequentially within the child context. When the child context is finished, `finish()` automatically queries the child layout state for its `resolve_space()` (the accumulated content resolved against the layout's bounds) and passes it to the `on_finish` closure.
    5. Inside the closure, the container consumes the token by calling `token.end_deferred_layout(children_extent)`. This resolves the container's final size and visual alignment inside the parent, advances the parent layout cursor, and releases the parent borrow, unlocking the parent context for subsequent sibling widgets.
  - This design decouples the container from concrete layout systems (like `ColumnLayout` or `RowLayout`) and concrete layout parameters (like `Placement2D` or `Rect`), as all sizing policies are decided solely via generic `LayoutSpace` bounds and completed via `LayoutToken`.

- **Container Lifecycle — begin/finish**: Container widgets (`begin_scroll_area`, `begin_window`, `begin_frame`) return a child `WidgetContext` with their cleanup logic embedded as an `on_finish` closure. The caller fills the child context with widgets, then calls `child.finish()`. Commands accumulate directly into the shared buffer and cleanup runs automatically — no explicit high-level `end_*` call or manual command threading needed. The raw layer still exposes `raw::end_scroll_area(token, content_extent, state, input, focus_system)` and `raw::end_window()` for callers that bypass the context system.

- **Shared Command Buffer**: Each `WidgetContext` holds `cmds: &'a mut DrawCommands`, a mutable reference into a buffer that ultimately belongs to the root caller. Child contexts are constructed by reborrowing the parent's `cmds` reference, so all contexts in a tree write into the same buffer in evaluation order. `finish()` returns `()` — there is no `DrawCommands` to thread back up the call stack. This means that the borrow-checker naturally prevents contexts from being used interleaved with each other (which we want to prevent as it would be invalid).

- **`on_finish` in `WidgetContext`**: Every `WidgetContext` carries `on_finish: CF` where `CF: FnOnce(&mut FocusSystem, &mut DrawCommands, Rect)`. Root contexts use a no-op function pointer. Container widgets construct a child via `child_with_layout_and_on_finish(layout, closure)`, passing a move closure that captures the container's token (and, for a scroll area, its `&mut ScrollState` and `&Input`). `finish()` reads the child layout's `resolve_space()`, calls the closure with that resolved `Rect`, and appends post-commands (e.g. `PopClip`, scrollbars, focus claims) into the shared buffer after the child's own accumulated commands. A scroll/frame container reads `(rect.w, rect.h)` from it as the content extent.

- **Caller-described inner layout space — what space, how to fill it**: The scroll area does not hardcode how its content is laid out. The caller describes, **per axis**, the `LayoutSpace` the content lays into — *what space the content has to work with* — and supplies a `Layout` separately — *how to fill it*. The area then derives scrollbar presence and the clipped viewport from that description. Two orthogonal per-axis inputs, bundled in `ScrollAxis { extent, vis }`:
  - **`ScrollExtent` — the content's available extent on this axis.** Lowers to an `AxisBound` once the viewport and gutters are known:
    - `Exact(Viewport)` (alias `FIT`) → `AxisBound::Exact(content_bounds)`: content fills the viewport exactly. Because this is a committed `Exact` frame, **alignment and `Fill` work inside the scroll area** — including on a scrolling axis's *cross* axis (e.g. center content horizontally in a vertically scrolling area). This is the case the old hardcoded-`Unbounded` design could not express.
    - `Unbounded` (alias `SCROLL`) → `AxisBound::Unbounded`: content extends past the viewport, its concrete extent measured at `end` (the "unbounded resolves to concrete at accumulation" rule). The deferred case — the area does not need the content size up front.
    - `Exact(Px(n))` (alias `fixed(n)`) → `AxisBound::Exact(n)`: a pinned content size, restoring the old up-front `content_size` capability.
    - `AtMost(Viewport)` / `AtMost(Px(n))` → `AxisBound::AtMost(…)`: content shrink-wraps but is capped at the viewport (or `n`). No forced fill, no scrollbar when it fits — a capability the old `content_size` API lacked.
  - **`ScrollbarVisibility` — reserve policy `{ Auto, Always }`** (the old `None` is gone — "no bar" is now expressed by a `FIT`/`AtMost` extent that provably fits). `Always` always reserves the gutter and draws the bar (degenerate over fitting content). `Auto` reserves only when overflow can't be ruled out at `begin`: `Unbounded` always reserves (deferred); a provably-fitting `Exact`/`AtMost` reserves nothing; a `Px(n)` reserves iff `n` exceeds the **raw viewport** extent (tested against the gutterless outer extent, not the post-gutter content extent, so the two axes' bar decisions don't mutually depend — a ~12px gutter can't flip the result).
  - **No feedback loop.** `Always` and the provably-fitting cases are decided at `begin` from the rect alone; only `Unbounded` defers, and a deferred axis always reserves. Resolve order: decide each axis's bar from the raw viewport → subtract reserved gutters to get `content_bounds` → lower each `ScrollExtent` against `content_bounds`. `content_bounds` is therefore known at `begin` independent of content size — the old steals-width feedback loop stays broken.
  - **begin → end split.** `begin` reserves gutters, pushes the content clip, and returns the lowered inner `LayoutSpace`. Everything needing the content extent moves to `end`, which receives it via `finish()` as a resolved `Rect` (see `resolve_space`): `max_scroll = content_extent − content_bounds`, the offset clamp, hover scroll claims (this frame's true extent via first-caller-wins at `end()`), wheel/page-key application, scrollbar thumb sizes, and the slider draw (emitted after `PopClip`, so scrollbars sit on top of and outside the content clip). The **one-frame lag** applies only to children laying out against the offset captured at `begin` (a hard content shrink can over-scroll for a single frame); scroll claims always use correct, non-stale boundaries. A `FIT`/`Exact` axis resolves to the exact viewport extent, so its `max_scroll` is 0 — no phantom scroll on a non-scrolling axis.

- **Borrow-Enforced Ordering**: Because child contexts are created from `&mut self`, the borrow checker enforces that only one child can be alive at a time. This is the correct constraint for immediate-mode GUI: draw commands are order-sensitive (later commands render on top), so constructing two sibling children simultaneously and finishing them in arbitrary order would be a footgun. The exclusive borrow makes incorrect ordering a compile error, not a runtime bug. An alternative design — separate owned buffers per context with a raw back-pointer for auto-append on `finish()` — is mechanically possible but loses this guarantee.

- **`ScrollAreaToken` — Dumb State Holder**: `begin_scroll_area` internally produces a `ScrollAreaToken`, a plain struct with private fields holding the geometry resolved at `begin` that `end` needs once the content extent is known — the scroll area's `FocusId`, its `rect` and `content_bounds`, which axes reserved a scrollbar, and the scrollbar style/width/clip/time. It deliberately does **not** carry scroll-limit (`at_*`) flags or `max_scroll`: those depend on the content extent and are computed in `end`. It has no `Drop` impl and no `finish()` method. The high-level `begin_scroll_area` captures this token in a move closure stored on the child `WidgetContext`; the raw API passes the token explicitly to `raw::end_scroll_area`. This design was chosen over a RAII-style `Drop` cleanup because `Drop` cannot receive `&mut FocusSystem` — borrowing it for the token's full lifetime would make the widget API impractical.

- **Why `finish()` vs. explicit `end_scroll_area` — two API layers, two contracts**:

  | | High-level (`WidgetContext::finish()`) | Low-level (`raw::end_scroll_area(token, content_extent, state, input, focus_system)`) |
  |---|---|---|
  | **Who calls it** | App code using the context API | Widget authors writing raw widget functions |
  | **What it knows** | Nothing about scroll areas specifically — just "run cleanup and close this scope" | Exactly what scroll area cleanup requires: pop clip, pop keyboard scope, make focus claims |
  | **How cleanup is delivered** | Via the `on_finish` closure captured at `begin_scroll_area` time | Via the `ScrollAreaToken` carrying the state needed for claims |
  | **Why explicit, not RAII** | `Drop` can't receive `&mut FocusSystem`; borrowing it for the token's lifetime would pollute every widget call site | Same reason |
  | **Ordering guarantee** | Borrow checker enforces sequential children; `finish()` appends directly into the shared buffer | Caller is responsible for matching `begin`/`end` calls; `debug_assert` checks order at runtime |

  At the high level, `finish()` is a uniform teardown verb — the caller doesn't need to know whether the context wraps a scroll area, a window, or nothing. At the raw level, `end_scroll_area(token, content_extent, state, input, focus_system)` is explicit because raw callers have all the information (including the content extent they measured) and are expected to manage lifecycle manually.

- **Bottom-Up Scroll Claims**: To handle nested scroll areas gracefully without immediate-mode input loops, the `FocusSystem` employs a 1-frame delayed "claim" architecture. Inner scroll areas register claims (`claim_scroll_up`, `claim_pgdn`, etc.). Because contexts are finished bottom-up, innermost scroll areas always get first pick of the claim.

- **Standalone Widget Participation**: Standalone widgets like standalone sliders actively participate in this claim system (using `claim_scroll_at_ends`). When hovered or focused, they block scroll inputs from propagating up to outer scroll areas, acting as "hard stops" instead of allowing the parent to suddenly start scrolling when the slider hits its boundary.

---

## Analytical Antialiasing (AA)

For high-performance rendering of lines, circles, and rectangles without the visual trade-offs or cost of MSAA (Multi-Sample Anti-Aliasing), Framewise uses CPU-side proxy quad expansion and GPU-side analytical distance field (SDF) evaluation.

### Core Philosophy & Text Handling
- **Text Backend**: AA for text is handled specially within the backend (e.g., using subpixel or grayscale glyph caching/rasterization), as text rendering is highly specialized and unique.
- **Other Geometry**: For lines, rectangles, borders, and general widget geometry, we use a dual solution: **pixel snapping** and **analytical AA**. This hybrid approach provides maximum visual quality with high performance, unlike MSAA (Multi-Sample Anti-Aliasing), which would yield poor visual quality for text/lines and bad performance.

### Renderer vs. Widget Responsibilities
- **Semantic Decisions (Widgets)**: Widgets/emitters (inside Framewise) are responsible for deciding if, when, and how to snap. Snapping should **not** be a hidden, renderer-wide heuristic. The renderer shouldn't automatically coerce layout/geometry, as this weakens semantic boundaries and could corrupt layout calculations.
- **Mechanical Execution (Renderer)**: The renderer acts as a predictable, mechanical consumer of explicit draw commands. However, the renderer should provide low-level mathematical helpers (utilizing device scale, framebuffer mapping, and snapping math for centerlines or edges) that widgets can invoke when building draw commands.
- **Renderer-Level AA Policy**: To keep the widget API clean and predictable, explicit `anti_alias` flags have been removed from the draw commands. Instead, antialiasing is handled as a structural renderer policy based on primitive type:
  - `FillRect` and `BorderRect` use automatic AA: non-AA quads when rect edges are integer pixel-aligned, and AA rect rendering otherwise.
  - `StrokeLine`, `FillCircle`, and `StrokeCircle` always use analytical AA.

### CPU-Side Processing & Interleaved Batching
- **ShapeData Storage**: Analytical AA shapes do not generate geometry on the CPU. Instead, their parameters (coordinates, color, stroke width, radius, shape type, and depth) are pushed to a `ShapeData` staging buffer that is uploaded to a GPU storage buffer.
- **Interleaved Batching**: To preserve clipping context, transparency, and alpha-blending order across the frame, the renderer's command processor (`Renderer::process_commands`) interleaves `DrawQuads`, `DrawText`, `DrawAA`, and `SetScissor` commands in their true evaluation sequence.
- Sequential commands of the same category are batched into a single GPU draw range. When a command of a different category is encountered, the active batch is flushed, and a new `RenderCommand` is appended to the stream.

### GPU-Side Pipeline & Shader Evaluation
- **Proxy Geometry Expansion**: The vertex shader (`vs_main` in `aa.wgsl`) runs on 6 vertices per instance. It reads primitive data from the storage buffer and expands the quad bounds outward by the stroke width plus a 1-pixel gutter to guarantee coverage of the AA falloff region.
- **Analytical Distance Fields (SDF)**: The fragment shader (`fs_main`) computes the analytical distance to the primitive's boundaries:
  - **Line Segment**: Evaluates the distance to the segment.
  - **Circle (Fill/Stroke)**: Evaluates distance to the radius (or stroke width bounds).
  - **Rectangle (Fill)**: Evaluates a signed box distance function.
- **Coverage Blending**: Coverage is calculated as a float value from `0.0` (fully outside) to `1.0` (fully inside). Pixels with zero coverage are discarded (`discard`), while others modulate the color's alpha value for smooth hardware-accelerated alpha blending.
- **Depth Testing**: AA shapes write depth values mapping to a 32-bit depth buffer using the `GreaterEqual` comparison function, ensuring seamless depth-based layering alongside opaque quads and text.

---

## Coordinate & Rendering Model

Framewise operates under a unified geometric coordinate and rendering model.

### 1. Coordinate System & Geometry
- **Logical Pixels**: All coordinates are continuous `f32` logical pixels.
- **Origin**: The origin `(0.0, 0.0)` is at the top-left of the window or active clipping region. The coordinate `x` increases to the right, and `y` increases downward.
- **Integer Coordinates**: Integer values (e.g. `10.0`, `11.0`) represent the *logical boundaries* between pixels.
- **Pixel Centers**: The centers of pixels lie on half-integer coordinates. For example, the logical pixel column `n` (where `n` is an integer) occupies the half-open interval `[n, n + 1)` and has its center at `n + 0.5`.
- **Rectangles**: A `Rect { x, y, w, h }` represents a region defined by `[x, x + w) × [y, y + h)`.
  - `right()` is computed as `x + w`.
  - `bottom()` is computed as `y + h`.
  - There is no bottom/right "minus one pixel" convention; borders and edges are reasoned about using standard real-number intervals.

### 2. Half-Open Hit-Testing
- **Hit-Testing (`Rect::contains`)**: Hit-testing uses the half-open region model:
  - A point `pos` is inside the rectangle if and only if `pos.x >= x && pos.x < right() && pos.y >= y && pos.y < bottom()`.
  - Points on the left and top edges are considered inside.
  - Points on the right and bottom edges are considered outside.
  - Empty or zero-size rectangles contain no points.
- **Tiling**: Under this half-open convention, adjacent rectangles tiled side-by-side (e.g. adjacent tabs or segmented control cells) share a boundary coordinate, but any given point on that boundary belongs strictly to one of the rectangles, resolving all hit-testing ambiguity.

### 3. Rendering and Antialiasing (AA) Policy
- **Box/UI Geometry**:
  - `FillRect` represents a solid filled box.
  - `BorderRect` represents a box border, which lowers to four filled rectangular strips.
  - Both primitives use automatic AA: if the layout aligns their edges exactly to logical integer pixel boundaries, the renderer draws them without AA (crisp, solid rendering). If they fall on fractional coordinates, they are drawn using AA.
- **Vector Geometry**:
  - `StrokeLine`, `FillCircle`, and `StrokeCircle` represent vector primitives and are always rendered with analytical AA.
- **UI Rules & Separators**:
  - Axis-aligned lines (horizontal/vertical lines, rule markers, separators, widget borders) are UI chrome, not vector shapes. They must be rendered using `FillRect`, `BorderRect`, `push_h_rule`, or `push_v_rule` to ensure crisp pixel-aligned rendering.
  - `StrokeLine` must not be used for axis-aligned UI rule decoration.
- **Vector Centering & Odd-Width Strokes**:
  - A `StrokeLine` centered exactly on an integer coordinate (e.g. a 1px line centered at `x = 5.0`) will straddle two pixel columns (`4.5..5.5`), resulting in partial coverage (blurriness) on both columns under AA.
  - To achieve a crisp 1px vertical line with vector rendering, the line centerline must lie on a half-pixel coordinate (e.g. `x = 5.5` to fully cover pixel column 5).
  - GPU rasterization tie-breaking edge rules are never relied upon. Layouts should either align filled boxes (`FillRect`/rules) to integer boundaries, or place vector stroke centerlines at half-pixel offsets.
