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
- **`*Spec`** holds everything the caller provides as input to the widget for that frame. Spec fields can vary frame-to-frame (e.g. elapsed time, a label string, an enabled flag driven by app logic), but they are **never mutated by the widget function**. The spec is consumed, not updated.

In short: if a value changes because the user clicked or typed, it belongs in `*State`. If it changes because the app decided something different this frame, it belongs in `*Spec`.

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
- `TextBackend::line_metrics` supplies line height and baseline offset through
  `TextLineLayoutMetrics`.
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

`OverflowX::WrapWord` groups contiguous non-whitespace clusters into word-like
segments. Unicode whitespace creates wrapping opportunities, and each
whitespace cluster is its own breakable word-like segment. Whitespace follows
the same overflow hierarchy as other segments: if it fits, it is admitted; if it
does not fit on a non-empty line, it may cause a soft wrap; if it cannot fit
even on an empty line, the fallback chain applies.

The one exception is the soft-wrap boundary-space rule described above. When a
single whitespace cluster becomes the boundary between two visual lines, that
cluster may remain in the previous line with zero advance rather than producing
a whitespace-only visual line. Adjacent whitespace remains preserved and
participates in wrapping normally.

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
- **Parameters** — order of parameters to raw functions and high-level context functions, including where `&Input`, `&mut *State`, and `*Spec` appear
- **Return types** — if one widget's high-level function returns `layout: LayoutInfo`, all do; if one raw result includes `content_bounds`, equivalent raw results do too. (Exception: [deferred-own-size containers](#deferred-own-size-containers) omit `layout` — they do not know their bounds at `begin`. This is a principled deviation shared by all such containers, not per-widget drift.)
- **Default value handling** — `unwrap_or` vs. `unwrap_or_default` vs. panic in `build()`, applied uniformly based on field semantics
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

The immediate placement call is `layout(params: S::Params, request: SizeRequest) -> Rect`. It merges three inputs: the caller's `params` (intent - fixed/auto/fill), the widget's size request (reported by a `size_*` companion under a `SizeOffer`, see [Size Offers and Requests](#size-offers-and-requests)), and the layout's own state (available space + cursor). Layouts that don't size from content (`ManualLayout`) ignore `request`; request-aware layouts (column/row/wrap) read it. There is still **no separate measuring pass over a retained tree** - the only extra work is the cheap, explicit size-request query.

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
2. Call `size_*(..., offer) -> SizeRequest`.
3. Call `layout(params, request) -> Rect`.
4. Draw the widget into that `Rect`.

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

**The `size_*` companion.** Each raw widget that participates has an independent `raw::size_*(spec, offer, text_backend) -> SizeRequest`. It takes a dedicated raw size spec such as `raw::ButtonSizeSpec`, containing only the fields needed to size that widget. Geometry, clipping, input state, focus state, and any draw-only fields are absent unless they genuinely affect the size request.

This keeps the type honest: size requesting runs before layout, so the widget rect is not available and cannot appear in the size spec. Callers do not use placeholder rectangles to satisfy a broader raw widget spec; they construct the smaller size spec directly.

**High-level flow.** The high-level widget function: (1) resolves defaults into the high-level `*Spec`; (2) constructs `raw::*SizeSpec` from the size-relevant fields; (3) calls `size_*(&size_spec, offer, ...)`; (4) calls `layout(params, request)` to get the real rect; (5) constructs `raw::*Spec` from the resolved high-level spec plus the layout rect and context clip; (6) calls the raw function. Under `ManualLayout` the size request is computed but ignored — an accepted "double-shape" cost for now (the text is shaped in both sizing and raw draw); a later `Layout::WANTS_REQUEST` const can gate it.

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

### Low-Level: Raw Widget Functions

Plain, low-level functions residing in `raw` submodules (e.g., `widgets::button::raw::button`). They are completely decoupled from `WidgetContext` and the layout system. They receive a fully resolved explicit specification struct, append draw commands directly to a caller-supplied `&mut DrawCommands` buffer, and return a `raw::*Result` containing interaction info. Every input is explicit; the cost is strictly local.

Appending directly to a caller-supplied buffer avoids intermediate `Vec` allocation and copying, and gives callers stable index-based access to the command list (which frame containers rely on for placeholder patching). The `cmds: &mut DrawCommands` parameter is always last, after all other inputs.

```rust
pub fn button<T: TextBackend>(spec: raw::ButtonSpec, state: &mut ButtonState, input: &Input, focus_system: &mut FocusSystem, text_backend: &mut T, cmds: &mut DrawCommands) -> raw::ButtonResult;
pub fn label<T: TextBackend>(spec: raw::LabelSpec, text_backend: &mut T, cmds: &mut DrawCommands) -> raw::LabelResult;
pub fn text_edit<T: TextBackend>(spec: raw::TextEditSpec, state: &mut TextEditState, input: &Input, focus_system: &mut FocusSystem, text_backend: &mut T, cmds: &mut DrawCommands) -> raw::TextEditResult;
```

Each `raw::*Result` is a concrete struct with no trait requirements on callers, no metadata maps, and no dynamic type slots. It does **not** contain a `DrawCommands` field — commands are written directly to the caller's buffer. (Result structs may derive utility traits such as `Debug` for inspection, but callers need not implement any traits to receive or use them.)

### High-Level Freestanding API: Context Integration

A unified `WidgetContext<'a, T, S, CF>` carries style parameters (theme, current text size, colors, clip rectangles, time) and system resources (mutable references `&'a mut T` to the text backend and `&'a mut FocusSystem` to the focus manager). The `CF` parameter is a one-shot cleanup closure (`FnOnce(&mut FocusSystem, &mut DrawCommands, Rect)`) called when the context is finished; it receives the shared command buffer and the layout's resolved space (the `Rect` from `finish()` reading `resolve_space()`), so container cleanup can both emit post-commands and resolve geometry from how large the children turned out. Root contexts use a no-op function pointer, container widgets embed their cleanup in a move closure (see [Scroll Areas and Windows](#scroll-areas-windows-and-symmetrical-container-life-cycles)).

High-level widget APIs are freestanding, highly ergonomic functions that accept a mutable reference to `WidgetContext` along with a high-level spec/state:

```rust
pub fn button<T, S, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult;
```

These freestanding functions automatically:
1. Resolve layout geometries using the context's layout engine.
2. Resolve styling parameters from the context's current settings.
3. Call the low-level `raw` widget functions.
4. Accumulate the returned draw commands inside the `WidgetContext`'s internal buffer.
5. Return a `*Result` to the caller.

### Output Types: `raw::*Result` and `*Result`

Each widget defines two result structs reflecting the two API layers.

**`raw::*Result`** is returned by the low-level raw function. It contains:
- Interaction outputs (`InputInfo`, `focused`, etc.)
- `content_bounds: Rect` when the widget computes an inner area distinct from the input rect (e.g. a widget with a border or padding). The raw function is the authoritative place to compute this, since it has the spec in hand.
- **Not** the input `Rect` itself — the caller supplied it explicitly, echoing it back is redundant
- **Not** `*State` — state is mutated in-place via the `&mut *State` parameter

**`*Result`** is returned by the high-level context function. It contains:
- `layout: LayoutInfo` — includes `bounds` (the rect resolved by the layout engine, which the caller did not know before calling) and `content_bounds`. **Omitted by deferred-own-size containers** (see [Deferred-own-size containers](#deferred-own-size-containers)): a `Frame` does not know its own bounds at `begin`, so it would have nothing honest to put here — its `FrameResult` carries only `ctx`.
- The same interaction outputs as `raw::*Result`
- **Not** `DrawCommands` — accumulated into `WidgetContext` automatically
- **Not** `*State` — mutated in-place

The high-level function maps between them: it resolves builder defaults into a high-level `*Spec`, constructs the smaller raw size spec, computes the `SizeRequest` under the `SizeOffer`, resolves the real rect via `ctx.layout_state.layout(params, request)`, constructs the raw widget spec with the resolved rect and context clip, calls `raw::widget()`, pushes draw commands into the context, then constructs the `*Result` forwarding the interaction fields and adding `LayoutInfo`.

Nesting a child layout is done with `ctx.child_with_layout(placement, inner_layout)`: it resolves `placement` against the *current* layout to get the child's bounds, begins `inner_layout` at those bounds, and returns a child `WidgetContext`. (Container widgets that compute their own bounds — scroll areas, windows — instead use the `child_with_layout_and_on_finish[_and_clip_rect]` variants, which take an already-begun layout state plus a self-derived clip.)

### Spec, Size Spec, Raw Spec, and Builder Pattern

Every widget type follows a consistent layered configuration pattern:

- **High-level `*Spec`**: The ergonomic user-facing configuration struct produced by the builder and used by the high-level context function. It contains only fields that are meaningful for high-level callers, such as content, style, and flags. It does not contain layout-resolved fields such as `rect`, or context-managed fields such as `clip_rect` and `layer`.

- **`*SpecBuilder`**: A builder struct used by high-level callers to construct the high-level `*Spec`. The builder holds optional fields and provides ergonomic setter methods. It applies theme defaults for user-facing values and panics only for required high-level inputs with no sensible default.

- **`raw::*SizeSpec`**: A low-level size specification struct used by `raw::size_*`. It contains only the fields needed to compute a `SizeRequest`. For example, `raw::ButtonSizeSpec` contains the button text and style, but not `rect` or `clip_rect`.

- **`raw::*Spec`**: A fully resolved low-level specification struct used by the raw widget function. All fields are concrete values needed to draw and interact with the widget, including geometry such as `rect` and context-managed values such as `clip_rect` and `layer`. It is defined inside the widget's `pub mod raw {}` submodule (e.g. `button::raw::ButtonSpec`), co-located with the raw function that consumes it, and avoids cluttering the normal module level with details high-level users do not need.

This pattern cleanly separates concerns:
- **Low-level functions** are pure and testable — they receive explicit values and produce explicit results, with no knowledge of themes, layouts, or context.
- **Size requesting** is type-safe — size specs cannot accidentally contain or read fields that are unavailable before layout.
- **High-level functions** are ergonomic and integrated — they resolve defaults, handle layout, bridge from high-level specs to raw specs, and hide low-level geometry/context plumbing.

> [!IMPORTANT]
> **Spec and SpecBuilder Value-Type Rule:** High-level `*Spec`, `*SpecBuilder`, `raw::*SizeSpec`, and `raw::*Spec` structs must contain only basic parameters (colors, fonts, rectangles, strings, numeric values, etc.). They must NOT include references to runtime resources like `Input`, `FocusSystem`, a text backend, or other external state. These structs should be pure value-types with no external references, making them trivially copyable, serializable, and independent of any runtime context.

> [!IMPORTANT]
> **Theme Must Not Appear in Specs:** A high-level `*Spec`, raw size spec, or raw widget spec must never hold a `Theme` field. `Theme` is a high-level convenience that maps semantic intent to concrete values; by the time a spec is constructed, that mapping is complete. The `*SpecBuilder` is the only place `Theme` is touched — its `defaults_from_theme()` method reads the theme and writes resolved colours, sizes, and font handles into the builder's fields. The resulting specs contain only those resolved primitives. This keeps every spec self-contained and renderer-agnostic, and prevents the low-level widget layer from having any dependency on the theme system.

> [!IMPORTANT]
> **Builder Construction Rule:** All `*SpecBuilder` structs use a no-args `new()` constructor. No field is singled out as a required constructor parameter — **every field, including bool flags like `disabled` and `large`, is `Option<T>`** and starts as `None`. `build()` applies defaults for fields that have an obvious, context-independent value (e.g. `disabled` → `unwrap_or(false)`) and panics with a clear message for fields with no sensible default; the message names the missing field and points to the fix (e.g. *"style not set — call .style() or defaults_from_theme()"*). Making every field `Option<T>` is essential: `None` means "the user did not set this", which lets both `defaults_from_theme` and the high-level widget function inject context-aware defaults — something impossible if bools silently default to `false` in `new()`.

### `defaults_from_theme` — Theme as Fallback

Every `*SpecBuilder` exposes a `defaults_from_theme(theme: &Theme)` method. It fills only the fields that are **not already set** — theme values are fallbacks, not overrides. Explicitly set fields always win. This is the fallback rule applied by high-level functions internally:

```rust
// custom style is preserved — defaults_from_theme sees style.is_some() and skips it
let spec = ButtonSpecBuilder::new()
    .text("Save".into())
    .style(my_brand_style)
    .defaults_from_theme(&theme)
    .build();
```

This is the only correct behaviour given the call order: the app sets fields on the builder before passing it to the high-level function, which then calls `defaults_from_theme` internally. If `defaults_from_theme` unconditionally overwrote fields, every explicit customisation would be silently discarded.

**High-level API callers never call `defaults_from_theme` directly.** It is called automatically inside every high-level context function. App code just sets the fields it cares about and passes the builder in.

The high-level function calls `defaults_from_theme` internally before building its high-level `*Spec`. Low-level raw callers do not use high-level builders; they construct `raw::*Spec` and `raw::*SizeSpec` directly, supplying already-resolved styles and geometry. If a raw caller wants themed values, it calls the appropriate `*Style::from_theme` helper and places the resulting concrete style into the raw spec.

Explicit high-level placement is expressed through the layout parameters, not the spec. Under `ManualLayout`, the layout parameter *is* the rect. Callers that want to bypass layout entirely use the low-level `raw::` function and set `rect` directly on `raw::*Spec`.

### SpecBuilder Field Visibility

`*SpecBuilder` fields are currently `pub`. This allows ergonomic struct-literal construction and direct field reads. Builder fields are limited to high-level configuration, so layout-resolved fields like `rect` and context-managed fields like `clip_rect` do not appear on high-level builders.

The alternative is private fields with setter methods only (standard Rust builder pattern). This would make the public API narrower, but all operations are already covered by the existing setter methods.

For now, fields remain `pub`. Framework-managed values are absent from the high-level builder and are introduced later by the high-level context function when it constructs raw specs.

### Default Implementations — Spec, Style, and Builder

None of `*Spec`, `*Style`, or `*SpecBuilder` structs implement `Default`. The reasons differ by type but share a common root: multiple sources of default values creates drift and obscures intent.

**High-level `*Spec` and raw spec structs — no `Default`**

Specs are resolved values for their layer; every field is a concrete value with no `Option<>` unless `None` is itself a meaningful widget value. A `Default` impl must invent values for required content, style, or raw geometry, producing instances that compile but render broken — silent failure instead of an explicit signal. Lifetime-parameterised specs (`MenuSpec<'a>`, `TabsSpec<'a>`, etc.) add a further constraint: they cannot implement `Default` without `'static` bounds, which would be unacceptable. The builder is the correct layer for partial high-level state; raw specs are constructed explicitly.

**`*Style` structs — no `Default`**

The only authoritative source of style defaults is the `*Style::from_theme()` (or `*Style::*_from_theme()` for multi-variant styles) methods defined directly on each style struct. A `*Style` struct is always either caller-supplied or theme-derived; there is no meaningful style independent of the theme. Hardcoded defaults on style structs duplicate the theme, diverge silently when the theme changes, and mask missing `defaults_from_theme()` calls with plausible-looking but wrong colors.

**`*SpecBuilder` structs — `derive(Default)` + `new()` forwarding**

Because every builder field is `Option<T>`, `derive(Default)` produces exactly an all-`None` struct — identical to a hand-written `new()`. All builder structs therefore `#[derive(Default)]` and keep a `new()` constructor that forwards to `Self::default()`. This gives callers both spellings (`ButtonSpecBuilder::new()` and `ButtonSpecBuilder::default()`) with zero drift risk: there is only one source of truth.

**When a high-level `*Spec` field is itself `Option<T>`, the builder field is `Option<Option<T>>`**

Some `*Spec` fields are `Option<T>` not because they are unresolved, but because `None` is a meaningful resolved value (e.g. `thumb_size_ratio: Option<f32>` where `None` means "no scrollbar thumb", or `peak: Option<f32>` where `None` means "no peak marker"). The builder must still distinguish "caller never set this" from "caller explicitly set this to `None`". The solution is `Option<Option<T>>` in the builder:

- **Outer `None`** — field not yet set; `build()` or `defaults_from_theme` may supply a fallback.
- **Inner `None`** — caller explicitly set the field to the "absent" semantic value.

The setter follows the same convention as every other field: it takes `T` (here `Option<f32>`) and wraps it in `Some`:

```rust
pub fn peak(mut self, peak: Option<f32>) -> Self {
    self.peak = Some(peak);  // outer Some = "was set"; inner Option = semantic value
    self
}
```

`build()` unwraps the outer layer with `.unwrap_or(<default>)` to recover the `Option<T>` the spec expects.

**The asymmetry between high-level `*Spec` and `*SpecBuilder` is intentional**

High-level `*Spec` is resolved for the high-level API — no partial state, no unresolved fields, no defaults of any kind. `*SpecBuilder` exists precisely to hold partial state: every field is `Option<T>` and `None` means "not yet set". This distinction enables a three-stage default precedence chain:

1. **User-specified** — fields set by the caller via builder setter methods. Always win.
2. **High-level widget function default** — if a field is still `None` when the high-level function runs, it may inject a context-aware default before calling `build()`. Examples: style defaults derived from the context theme, or a container widget forcing `disabled = true` on all children while it is loading.
3. **`build()` default or panic** — fields still `None` at `build()` time either get a context-independent default (`disabled` → `false`, `large` → `false`) via `unwrap_or`, or cause a panic with a descriptive message if no sensible default exists (`text`, `style`).

This means defaults are applied **as late as possible**, giving higher layers the opportunity to provide sensible context-aware values rather than being silently pre-empted by a `false` baked in at construction time.

### Style Structs

Some widget types group their styling fields into a dedicated `*Style` struct embedded inside `*Spec` and `*SpecBuilder`. The decision rule:

- **Use a `*Style` struct** when the widget has interaction states (hover, press, focus, disabled) or several coordinated color/dimension roles. The style struct keeps the spec readable and lets callers pass a single `ButtonStyle` override rather than setting a dozen fields individually.
- **Embed styling fields directly in `*Spec`** when the widget is purely display-only and has only a small number (roughly ≤ 3) of styling fields. A dedicated struct would be ceremony with no benefit for these simple cases.

The practical dividing line is interaction states: as soon as a widget needs distinct visuals for hover, focus, or disabled, the coordinated color roles naturally belong in a `*Style` struct. Pure display widgets without those states may keep their styling inline.

Example:
```rust
// Low-level: fully resolved, no defaults
pub fn button<T: TextBackend>(spec: raw::ButtonSpec, state: &mut ButtonState, input: &Input, focus_system: &mut FocusSystem, text_backend: &mut T) -> raw::ButtonResult;

// High-level: uses builder to resolve defaults
pub fn button<T, S, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let size_spec = raw::ButtonSizeSpec {
        text: spec.text,
        style: spec.style,
    };
    let offer = ctx.peek_offer(layout_params.clone());
    let request = raw::size_button(&size_spec, offer, ctx.text_backend);
    let rect = ctx.layout(layout_params, request);
    let raw_spec = raw::ButtonSpec {
        rect,
        text: spec.text,
        style: spec.style,
        clip_rect: ctx.clip_rect,
        disabled: spec.disabled,
    };
    let r = raw::button(raw_spec, state, ctx.input, ctx.focus_system, ctx.text_backend, ctx.cmds);
    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}
```

### User-Defined Layouts Are First-Class

Built-in layouts hold no privileged position. The two public traits — `Layout` and `LayoutState` — are the complete extension point:

- **`Layout`** defines the configuration type (`type Params`) and a `begin(space: impl Into<LayoutSpace>) -> Self::State` method that initialises the mutable state.
- **`LayoutState`** is the mutable engine: `layout(params, request) -> Rect` for normal widgets, `begin_deferred_layout` / `end_deferred_layout` for fit-to-children containers, and `resolve_space() -> Rect` so scroll areas and `finish()` can read the accumulated content **resolved against the layout's own `LayoutSpace` bounds** (an `Exact` axis reports the exact extent, `AtMost` caps the measured size, `Unbounded` shrink-wraps to it).

A user-defined layout implements both traits, passes its state type into `WidgetContext::child_with_layout`, and is otherwise identical to `ColumnLayout` or any other built-in. No library modification is required; no registration step exists. The built-ins are examples of the pattern, not gatekeepers of it.

### User-Defined Widgets Are First-Class

Built-in widgets hold no privileged position in the architecture. `Theme` is a library-defined struct — callers cannot add methods to it. If themed style defaults required a `theme.xxx_style()` method on `Theme`, only built-in widgets could participate; user-defined widgets would have no equivalent path.

By placing the conversion on the style struct itself — `*Style::from_theme(&theme)` — the pattern is fully open to extension. A user-defined widget follows exactly the same design as a built-in one:

1. Define a `*Style` struct with the widget's styling fields.
2. Implement `from_theme` (or `*_from_theme` for multi-variant styles) on that struct.
3. Call it from `*SpecBuilder::defaults_from_theme`.

No library modification required. No special registration. The library's own widgets are simply examples of the pattern, not gatekeepers of it.

### Theme and Font Boundaries

`Theme` is part of the high-level API. The `WidgetContext` uses it to resolve ergonomic defaults such as colours, spacing, and semantic font choices, but low-level widget functions must not depend on a theme. A low-level `WidgetSpec` is already fully resolved by the time it is passed to the widget function.

> [!IMPORTANT]
> **Static Check Rule:** Low-level raw widget functions must not import or depend on `theme::Theme`. The builder layer is the correct and only place `Theme` is consumed — `*SpecBuilder::defaults_from_theme` calls `*Style::from_theme` (or `*Style::*_from_theme`) on the widget's style struct, which translates the theme into resolved primitives before any raw function sees them. Because builders and style structs live in the same file as their raw functions, widget files do import `Theme`, but the import is confined to these higher-level layers. All `raw::*` functions in `framewise/src/widgets/*` must accept only fully resolved `*Spec`/`*Style` data and must not reference `Theme` directly.

Fonts follow the same rule. A font is an application-owned handle independent of any theme. A theme references the two handles it wants to use for sans and mono text, but it does not own renderer-specific font data. The context may copy those handles from the theme into widget specs based on widget type; direct low-level callers choose fonts explicitly, often by copying a handle from a theme themselves.

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
- **Other Geometry**: For lines, rectangles, borders, and general widget geometry, we will use a dual solution: **pixel snapping** and **analytical AA**. This hybrid approach provides maximum visual quality with high performance, unlike MSAA (Multi-Sample Anti-Aliasing), which would yield poor visual quality for text/lines and bad performance.

### Renderer vs. Widget Responsibilities
- **Semantic Decisions (Widgets)**: Widgets/emitters (inside Framewise) are responsible for deciding if, when, and how to snap. Snapping should **not** be a hidden, renderer-wide heuristic. The renderer shouldn't automatically coerce layout/geometry, as this weakens semantic boundaries and could corrupt layout calculations.
- **Mechanical Execution (Renderer)**: The renderer acts as a predictable, mechanical consumer of explicit draw commands. However, the renderer should provide low-level mathematical helpers (utilizing device scale, framebuffer mapping, and snapping math for centerlines or edges) that widgets can invoke when building draw commands.
- **Proposed API**: Draw commands and primitive styles will explicitly declare their intent:
  - `snap: PixelSnap` where `PixelSnap` has modes like `{ None, AxisAligned, AxisAlignedIfThin, Centerline }`.
  - `aa: AaMode` where `AaMode` has modes like `{ None, Analytical }`.

### Semantic Choice (Emitters)
- Drawing commands (e.g., `DrawCmd::FillRect`, `DrawCmd::StrokeRect`, `DrawCmd::StrokeLine`, `DrawCmd::FillCircle`, `DrawCmd::StrokeCircle`) accept an explicit `anti_alias: bool` flag.
- Setting `anti_alias: false` routes the geometry to the standard solid-color `quad_pipeline` (rendered via CPU-generated vertex coordinates).
- Setting `anti_alias: true` routes the primitive parameters to the `aa_pipeline` (rendered via GPU-expanded proxy quads and SDF evaluation).

### CPU-Side Processing & Interleaved Batching
- **ShapeData Storage**: Analytical AA shapes do not generate geometry on the CPU. Instead, their parameters (coordinates, color, stroke width, radius, shape type, and depth) are pushed to a `ShapeData` staging buffer that is uploaded to a GPU storage buffer.
- **Interleaved Batching**: To preserve clipping context, transparency, and alpha-blending order across the frame, the renderer's command processor (`Renderer::process_commands`) interleaves `DrawQuads`, `DrawText`, `DrawAA`, and `SetScissor` commands in their true evaluation sequence.
- Sequential commands of the same category are batched into a single GPU draw range. When a command of a different category is encountered, the active batch is flushed, and a new `RenderCommand` is appended to the stream.

### GPU-Side Pipeline & Shader Evaluation
- **Proxy Geometry Expansion**: The vertex shader (`vs_main` in `aa.wgsl`) runs on 6 vertices per instance. It reads primitive data from the storage buffer and expands the quad bounds outward by the stroke width plus a 1-pixel gutter to guarantee coverage of the AA falloff region.
- **Analytical Distance Fields (SDF)**: The fragment shader (`fs_main`) computes the analytical distance to the primitive's boundaries:
  - **Line Segment**: Evaluates the distance to the segment.
  - **Circle (Fill/Stroke)**: Evaluates distance to the radius (or stroke width bounds).
  - **Rectangle (Fill/Stroke)**: Evaluates a signed box distance function.
- **Coverage Blending**: Coverage is calculated as a float value from `0.0` (fully outside) to `1.0` (fully inside). Pixels with zero coverage are discarded (`discard`), while others modulate the color's alpha value for smooth hardware-accelerated alpha blending.
- **Depth Testing**: AA shapes write depth values mapping to a 32-bit depth buffer using the `GreaterEqual` comparison function, ensuring seamless depth-based layering alongside opaque quads and text.
