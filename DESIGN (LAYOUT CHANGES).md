# Framewise: Intrinsic Sizing, Unbounded Axes & Deferred Content — Design Proposal

> **Status:** proposal. To be merged into `DESIGN.md` once implemented.

## Summary

Reduce the manual-sizing burden without abandoning the top-down, bounds-first, one-pass model. Add a small set of explicit primitives — intrinsic size reporting, unbounded axes, deferred scroll content — rather than a bottom-up auto-layout engine. No retained tree, no global registry, no hidden second pass beyond cheap, explicit spec measurement.

## Headline rule

> **If a placement resolves from what's already known — available space, already-placed siblings, and this child — Framewise automates it. If it needs a *future* sibling, you declare the structure up front, or it's not possible.**

Three tiers, in plain UI terms:

- **Automate** (past-only) — "stack these labels, each as tall as its text."
- **Declare** (future sibling, but you said how many) — "split this row into four equal columns."
- **Refuse** (depends on itself / over-constrained) — "size this to its text *and* force it twice its neighbor."

## Emit order, visual position, and focus order are independent

Three orderings are separate, and Framewise already has the machinery for all three:

- **Emit order** — when you call the widget function. Drives draw/compositing (later renders on top) and the cursor in sequential layouts.
- **Visual position** — the resolved `Rect`. Under `ManualLayout` (or an explicit-slot helper) it is fully decoupled from emit order.
- **Focus order** — detached from emit order via `override_next` (see Input Focus in `DESIGN.md`).

This gives a general escape hatch: **reordering emit converts a future-sibling dependency into a past-sibling one.** "First child fills the remaining row width, second is intrinsic" — the fill child depends on a *future* sibling; emit the intrinsic child first, read its size, then emit the fill child at the computed remainder, and `override_next` to restore left-to-right focus. Visually L→R, focus L→R, emitted R→L. This works **today** with `ManualLayout` — no new machinery, just newly sanctioned.

General form: **if dependencies form a DAG, emit in topological order and every dependency is already known.** Cycles (the non-goals below) have no valid topological order and remain impossible.

Two caveats:

1. **Sequential layouts couple emit order to position.** `RowLayout`/`ColumnLayout` advance a cursor by emit order, so emitting the right child first lands it in the left slot. The reorder trick needs `ManualLayout` or an explicit-slot helper, not a naive sequential layout.
2. **Overlapping widgets — reorder changes z.** Safe only when slots don't overlap (the common row/column case). If widgets overlap, emit order is layering and must not be reordered casually.

## Core mechanism

### Three types, three owners

- **`LayoutSpace`** — available space the parent hands **down**. Carries an `AxisBound` per axis.
- **`IntrinsicSize`** — the widget's own measurement, reported **up** by a `calc()` companion to its raw function. Measurement only.
- **`Rect`** — the resolved output. Always fully `Bounded`; honors the existing rule that no `Option` geometry reaches a raw function.

```rust
pub enum AxisBound {
    Bounded(f32),
    Unbounded,
}

pub struct LayoutSpace {
    pub x: f32,
    pub y: f32,
    pub width: AxisBound,
    pub height: AxisBound,
}
```

`Rect` stays unchanged and fully specified.

### `IntrinsicSize` is measurement, never policy

`IntrinsicSize` is the right name even as it grows to a range — this matches CSS intrinsic sizing (min-content / max-content):

- **Belongs in `IntrinsicSize`** (widget-derived facts): `min` (e.g. the longest unbreakable word, below which text clips), `preferred` (the intrinsic unwrapped size), `max` (largest useful size). Fields are optional — a widget may know one axis and not the other; a fully-unknown value needs no separate sentinel.
- **Does NOT belong in `IntrinsicSize`** (caller policy): "can expand", "fill", "grow weight". These live in the layout's `Params`.

The test: if the widget computes it from its own content, it's `IntrinsicSize`; if the caller decides it, it's `Params`. "Should not shrink below 60 because the label clips" is a widget fact → `IntrinsicSize.min`. "Stretch to fill the row" is caller intent → `Params`. Keeping flex flags out of `IntrinsicSize` is what lets the name survive expansion.

### The layout call gains an intrinsic argument

```rust
fn layout(&mut self, params: Self::Params, intrinsic: IntrinsicSize) -> Rect;
```

Three inputs merge: `params` = caller intent (fill / fixed / weight), `intrinsic` = the widget's measurement, the layout's own state = available space. `ManualLayout` ignores `intrinsic` and keeps `Params = Rect`. Intrinsic-aware layouts (column/row/wrap) read it.

### High-level widget path

Each raw widget gains an independent `calc_*_intrinsic_size()` that may share internals with its raw function but is otherwise unrelated to it. The high-level function:

1. Resolve builder defaults that affect size (style, text).
2. Call `raw::calc_widget_intrinsic_size(...)` → `IntrinsicSize`.
3. `ctx.layout(params, intrinsic)` → concrete `Rect`.
4. Build the final `*Spec` with that rect, call `raw::widget(...)`.

```rust
pub fn button<T, S, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    let style = builder.resolved_style(&ctx.theme);
    let intrinsic = raw::calc_button_intrinsic_size(&builder.text, &style, ctx.text_system);
    let rect = ctx.layout(layout_params, intrinsic);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let r = raw::button(spec, state, ctx.input, ctx.focus_system, ctx.text_system);
    ctx.append_cmds(r.draw.0);
    ButtonResult { layout: LayoutInfo::new(rect, r.content_bounds), input: r.input, focused: r.focused }
}
```

### Known costs (documented, not fixed in v1)

- **Double-shape.** The widget is generic over `S` and cannot know whether the layout consumes `intrinsic`, so `calc()` runs unconditionally. Its text shaping (`text_system.prepare`) then repeats inside `raw::widget`. Wasted under `ManualLayout`. Acceptable for v1; a later `Layout::WANTS_INTRINSIC` associated const can gate it.
- **Call-site churn.** Every `ctx.layout(p)` becomes `ctx.layout(p, intrinsic)` — mechanical across all widget files.

### Two unbounded rules

1. **Fill + Unbounded is illegal.** Filling an infinite axis is undefined; reject it or fall back to the intrinsic size.
2. **Unbounded resolves to concrete at accumulation.** A child laid out in an `Unbounded` axis still advances a concrete f32 cursor, so the final extent is `Bounded`. No infinity ever reaches a `Rect`. This is exactly what a deferred scroll area reads as its content size.

## Scroll areas — single Reserve policy, begin→end rebalance

No AutoHide, no modes. If the user enables a scrollbar on an axis, its width is **always reserved**, even when content turns out to fit. This removes the steals-width feedback loop: child width = `rect.w − scrollbar_w` is known at `begin`, independent of content height.

Everything that needs content extent moves from `begin` to `end`:

| Computation | Today | Reserve + deferred |
|---|---|---|
| Reserve scrollbar width, push clip | begin | **begin** (all that's left) |
| `max_scroll = content − rect` | begin | **end** |
| offset clamp | begin | **end** |
| `at_*` flags, scroll/page claims | begin | **end** |
| thumb ratio + draw slider | begin | **end** |
| wheel / page-key apply | begin | **end** |

Two accepted consequences:

- **1-frame clamp lag.** Children lay out using the offset captured at `begin`; the clamp now runs at `end`, landing next frame. A hard content shrink yields one frame of possible over-scroll. Consistent with the existing 1-frame claim architecture (see The Three Routing Problems in `DESIGN.md`).
- **Scrollbar draws on top.** The slider is emitted at `end`, after content — correct layering for scrollbars, and free.

This begin→end rebalance is the bulk of the implementation effort; the sizing primitives are comparatively easy.

## Phasing

1. ✅ **Widget intrinsic size reporting** — `IntrinsicSize`, `calc_*` functions, `layout(params, intrinsic)`, intrinsic-aware column/row/wrap.
2. ✅ **Unbounded axes** — `AxisBound`, `LayoutSpace`, the two rules. Prerequisite for deferred scroll.
3. ✅ **Deferred scroll content** — `LayoutState::content_extent`, Reserve policy (drop `Auto`), begin→end rebalance, 1-frame clamp lag. `end_scroll_area` takes the measured extent; `max_scroll` now subtracts the reserved gutter (`content − content_bounds`). See `DESIGN.md` → Scroll Areas.
4. **Declared-structure helpers** — fixed arity, declared count/slots, weighted distribution, grid via measure-all-then-place.

** DESIGN.md should be updated as each phase is complete!

## Supported layout cases

First phase at which each case works. `—` = unchanged from the prior column. ⚠ = works with a caveat.

### Tier 1 — Automate (past-only)

| Case (real scenario) | Today | P1 | P2 | P3 |
|---|---|---|---|---|
| Manual explicit placement | ✅ | — | — | — |
| Stack, caller sizes every child (vert/horiz) | ✅ | — | — | — |
| "Stack these labels, each as tall as its text" | ❌ | ✅ | — | — |
| "Row of chips, each as wide as its label" | ❌ | ✅ | — | — |
| "Fixed-width icon, label takes its intrinsic width" (mixed per-axis) | ❌ | ✅ | — | — |
| "Column fills the panel width, each row auto-height" (fill cross-axis) | ❌ | ✅ | — | — |
| "Tags that wrap onto the next line when the row fills" (flow) | ❌ | ✅ | — | — |
| "A bordered box that hugs its single child plus padding" (decorator) | ❌ | ✅ | — | — |
| "Toolbar: search field eats leftover space, icons stay intrinsic" | ❌ | ✅ | — | — |
| Overlay / absolute children (Manual) | ✅ | — | — | — |
| "Panel fills available height inside a normal (bounded) container" (unbounded axis, non-scroll) | ❌ | ❌ | ✅ | — |
| Scroll, content size known up front | ✅ | — | — | — |
| "Scroll area sized to content discovered only after its children run" | ❌ | ❌ | ❌ | ✅ (Reserve; scrollbar width always reserved) |
| "Infinitely tall list inside a scroll area" (unbounded axis in scroll) | ❌ | ❌ | ⚠ axis ready | ✅ extent at end |
| "Long auto-sized vertical list in a scroll area" | ❌ | ❌ | ❌ | ✅ |
| Nested scrolling + clipping | ✅ | — | — | ⚠ smoother with end-resolved size |

### Tier 2 — Declare (future sibling; needs declared structure, P4)

These are impossible in the plain and measure-only models — leftover/shared space depends on *all* siblings — but become one-pass the moment the user declares count or slots. Declaration converts a future-sibling dependency into a known quantity. Internally a helper measures the declared children, distributes, then emits in dependency order with `override_next` to restore logical focus.

| Case (real scenario) |
|---|
| "Three buttons sharing a row in equal thirds" |
| "Weighted split: left pane 2×, right pane 1×, filling the row" |
| "Space-between: first item left-aligned, last right-aligned, even gaps" |
| "A grid where each column is as wide as its widest cell" (declared column count + measure-all) |
| "A row of cards all stretched to match the tallest one" (declared count + measure-all) |

### Tier 3 — Refuse (non-goals, impossible at any phase)

Each asks for a value that only exists *after* the thing it controls is decided (circular), asks two rules to win at once (over-constrained), or fills something with no size. These don't depend on a *future* sibling — they depend on *themselves*, which is past the wall the headline can reach.

| Case (real scenario) | Why never |
|---|---|
| "A caption that wraps into a neat square-ish block instead of one long line and a stub" | Width depends on the wrapped height, which depends on the width. No fixed point in one pass — pick a width. |
| "Three buttons, each as wide as its own label, but the first always exactly twice the others" | *Size to your text* and *be 2× the others* contradict. Nothing to solve — choose one rule. |
| "A tooltip that shrinks to hug its text, while the text re-wraps to fit that shrunk width" | Same width ↔ content loop, at the container level. |
| "Make this panel fill the height inside a vertically-infinite scrolling list" | The list has no fixed height to fill; filling "unbounded" is meaningless (the fill + Unbounded rule). |
| "Two panes that always stay equal as you drag a divider, both honoring minimums, both filling the window, all at once" | Simultaneous multi-variable solve — a constraint solver, not a forward pass. |

## Future possibility (not in scope) — pre-declared slot helper

A planned-slot API would package Tier 2 ergonomically:

```rust
let mut plan = PlannedRow::new().spacing(6.0);
let title   = plan.slot(Slot::auto());
let _spacer = plan.slot(Slot::flex(1.0));
let save    = plan.slot(Slot::auto());
let cancel  = plan.slot(Slot::auto());
let mut toolbar = begin_planned_row(ctx, plan, bounds);

label_in(&mut toolbar, title, ...);
button_in(&mut toolbar, save, ...);
button_in(&mut toolbar, cancel, ...);
toolbar.finish();
let saved = toolbar.result(save).clicked;   // read after finish
```

Recorded here as a direction, **not** for implementation, because three issues need resolving first:

- **Flex forces deferral.** A flex slot's leftover needs every auto slot's size, so no slot right of it can be placed until all `*_in` calls are in. Draw and interaction must defer to `finish()`, meaning results are read by handle *after* `finish()` — you lose the inline `if button_in(...).clicked`. This is bounded buffering of one row (freed at `finish`), not a retained tree, and state still mutates in frame N. But the ergonomic shift is real.
- **Handles, not strings.** Slot keys should be typed handles returned at declaration (as above), not string names — Framewise rejects string/global IDs, and handles give compile-checked fills with no lookup, allocation, or typo-miss.
- **API surface.** Slot-addressed fills (`label_in`, `button_in`, …) need a twin per widget, which risks violating Widget Consistency if coverage is partial. If pursued, every high-level widget gets an `*_in` twin — no partial coverage.

The reorder trick (emit autos to measure, distribute, place, `override_next`) is the engine such a helper would use internally.

## Invariants held throughout

- **Top-down and immediate.** Parent space is known before children; no bottom-up constraint solving.
- **One pass for placement.** The only extra traversal is cheap, explicit spec measurement (P1) or measure-all-then-place for grid/match-tallest (P4). Neither retains a widget tree.
- **Layout stays a `WidgetContext`-level concept.** Raw widgets receive fully-resolved `Rect`s and never see `LayoutSpace`, `IntrinsicSize`, or `AxisBound`.
- **Determinism and locality.** Every placement depends only on parent space, caller intent, this widget's measurement, and earlier siblings — never later ones.
- **Three orderings stay independent.** Emit, visual, and focus order are decoupled; reordering emit (within a DAG) is the sanctioned bridge from the Declare tier down into Automate.

---

## Phase 5 — Three-state axis bounds (`Exact` / `AtMost` / `Unbounded`)

Phases 1–4 use a binary `AxisBound` (`Bounded(f32) | Unbounded`). Phase 5 splits the bounded case into two semantically distinct kinds of parent knowledge:

```rust
pub enum AxisBound {
    Exact(f32),     // "you live in a box of this width" — limit AND anchor frame
    AtMost(f32),    // "choose your own width, but don't exceed this" — limit only
    Unbounded,      // "no ceiling from me on this axis"
}
```

(`Exact` replaces the old `Bounded`; rename rather than reuse the name, so "concrete resolved extent" never blurs with "constraint ceiling".)

### Why three, not two

`AtMost` is the missing middle between "totally fixed" and "infinite", and it covers very common container semantics that neither `Exact` nor `Unbounded` expresses honestly:

- "Wrap within the panel if needed, but don't force full width."
- "Hug contents, but never grow beyond the viewport."
- Text especially: it rarely wants *infinite* width (which produces pathological preferred sizes) — it wants "measure as naturally as you can, but under this maximum line length." That is `AtMost`, not `Unbounded`.

### The distinction that matters: anchor vs ceiling

`Exact(w)` answers two questions; `AtMost(w)` answers only the first:

1. **How much space may the child consume?** — both answer this.
2. **Relative to what concrete box may it position itself?** — only `Exact` answers this.

`AtMost` is a ceiling with no committed far edge. `Exact` is a ceiling plus an anchor frame. Many simple widgets (a plain label) measure identically under both, but an aligning layout or decorator does not: a right edge only exists if the parent has already committed to one.

So `AtMost` is **not** a weaker `Exact` — it is a different kind of knowledge. The layout API should branch on it explicitly, never silently coerce, or layouts risk doing alignment math against a width that was only ever a cap.

### Unifying rule (generalizes the Phase 2 "fill + Unbounded is illegal")

> **Position & distribution policies — fill, right-align, center, space-between — require `Exact`: a committed frame with a far edge. `AtMost` and `Unbounded` permit only measurement / shrink-wrap decisions.**

The Phase 2 rule "fill + Unbounded is illegal" is now just one case of this. Mental model:

- `Exact(w)` — "You live in a box of width w." (alignment, fill, distribution legal)
- `AtMost(w)` — "Choose your own width, up to w." (measure / shrink-wrap only)
- `Unbounded` — "No width ceiling from me." (accumulation / scroll extent only)

### Resolution semantics

- `Exact(w)` → child uses `w` (or aligns/fills within it).
- `AtMost(w)` → child measures intrinsic, resolves to `min(preferred, w)`.
- `Unbounded` → child resolves to intrinsic `preferred`; the parent accumulates a concrete f32 extent (the Phase 2 accumulation rule). No infinity reaches a `Rect`.

### Invariant preserved

`AxisBound` (all three states) lives in `LayoutSpace` only. By the time a raw widget is called, the high-level path has already resolved a concrete `Rect`. Raw widgets never see `Exact`/`AtMost`/`Unbounded`. The flow:

1. Parent provides `LayoutSpace` with `Exact` / `AtMost` / `Unbounded`.
2. Intrinsic measurement is queried under those constraints.
3. Layout resolves a concrete `Rect`.
4. Raw widget is called with only that `Rect`.

---

## Phase 6 — Fit-to-children containers (extent-only deferral)

A container that sizes itself to its children. Viable in **one direction only**: a container may report its own final outer bounds *after* children run, provided its children did not need those final bounds to lay themselves out.

### Two kinds of fit — only the first is supported

- **Extent-only fit (supported).** `begin` opens a child context with a partially-unbounded `LayoutSpace` (e.g. bounded/`AtMost` on the cross axis, `Unbounded` on the main axis). Children resolve to concrete `Rect`s from known constraints. `end` computes the container's outer bounds from the accumulated extent (cursor / union of child bounds) plus padding/border. Children needed parent *constraints*, never parent *final size*.
- **Constraint-affecting fit (refused).** Children need the fitted result as an input to their own measurement or placement — wrapping text to the fitted width, distributing leftover relative to the final extent, centering against the final far edge. Here child size depends on parent size while parent size depends on children: a self-dependency. This is the same width ↔ content loop already in the Tier 3 non-goals (square-ish captions, text-hugging tooltips). Not supported without buffering or a second pass.

The dividing line is exactly the headline rule: if the container's size is "the union of child bounds after layout," deferral is valid; if the fitted size is *also* an input to child measurement, it is self-referential and refused.

### Relationship to scroll (Phase 3)

This is conceptually closer to the Phase 3 scroll begin→end rebalance than to a generic begin/end widget split: `begin` establishes a provisional coordinate space, `end` seals the container from accumulated child extent. A fit-children container is essentially "the scroll rebalance minus the scrollbar and clip." Phase 6 reuses that machinery.

### API shape

Fit-children is a **container layout primitive**, not a "widgets may lack bounds until end" relaxation. `begin` returns a child context whose `LayoutSpace` is partially unbounded; raw leaf widgets still receive concrete `Rect`s at call time. The strong invariant holds: **containers may defer their own outer rect, but never the child rects passed to raw widgets.**

### Worked example

A bordered box hugging a vertical stack of intrinsic-width labels plus padding: children emitted in an unbounded vertical flow, width = max child width (under the cross-axis cap), height = accumulated cursor, border rect finalized at `end`. (The Tier 1 table already lists "a bordered box that hugs its single child plus padding" as viable once intrinsic sizing + unbounded axes exist — Phase 6 generalizes it to multi-child.)

By contrast, a box that wants to shrink-wrap text *after* wrapping it to the box width does not work: the wrapped-text measurement needs the width the box is trying to discover from that same wrapped text. Tier 3 non-goal.

### Open question (resolve at implementation)

The deferral itself is straightforward. The new machinery is **feeding the fitted extent back into the parent layout** — this is the part to design when we get here:

- A fit-box's top-left is known at `begin` (the parent cursor position) — `Exact`-anchored. Its extent is unknown until `box.finish()`.
- The parent's **next** sibling needs the box's extent to position itself (a past-sibling dependency once the box is emitted — legal by the headline, but mechanically new).
- Today a sequential layout (`ColumnLayout`/`RowLayout`) advances its cursor inside `layout()`, at the child's *begin* — when a fit-box's extent is still unknown. And `finish()` currently returns `()`.
- So Phase 6 needs: **(a)** a fit container's `finish()` to surface its resolved outer bounds, and **(b)** a sequential parent layout to defer cursor-advance for a container child to that child's `finish()` time, using the reported extent.
- Nested fit (box-in-box) chains this bottom-up: inner finishes first (borrow-enforced sequential children), reports extent, outer accumulates, outer finishes, reports to *its* parent. All past-only, so legal — but every level's parent-cursor-advance is deferred to child-finish.

Exact mechanism for (a)/(b) is **deferred to implementation** — flagged here so it is not forgotten.

Another thing to figure out - PushClip would be run as part of begin(), but we don't know the rect yet!

---

## Phase summary (updated)

1. Widget intrinsic size reporting.
2. Unbounded axes (`Bounded` / `Unbounded`).
3. Deferred scroll content (Reserve policy, begin→end rebalance).
4. Declared-structure helpers.
5. Three-state axis bounds (`Exact` / `AtMost` / `Unbounded`) + the position-policy-requires-`Exact` rule.
6. Fit-to-children containers (extent-only deferral) — open question on parent-cursor propagation, to resolve at implementation.
