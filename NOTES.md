# Framewise Notes

Working notes, TODOs, open questions, and half-baked ideas.


---

* RowLayout allows you to specify alignment in the main axis - doesn't do anything! Actually maybe it does, or at least it should?
* Ellipses are appearing in the middle of the height?
* Nothing to tell you that you forgot to .finish() a widgetcontext

* Go through the spec_page, check/implement/test each widget/aspect to make better match the mock-up and add interactivity as we go


  - For widgets using handle_widget_focus + writing InputInfo manually: hovered is rect.contains(input.mouse_pos) && clip.is_none_or(...). Plain hit test (no exclusion for mouse-down-elsewhere).
  Button computes hovered = contains && (!input.mouse_down || state.is_active) — excludes hover while dragging from elsewhere. Semantic mismatch: checkbox hovers when you mouse-down on something else and drag over it; button doesn't.

  2. Mouse-capture flag — inconsistent, contradicts DESIGN
button: is_active
slider/text_edit/drag_number: is_dragging
checkbox/radio/switch/chip/segmented/tabs/select: no capture flag
DESIGN treats mouse-capture-via-state as foundational robustness mechanism. Toggles drop it. Partly justified (toggles fire on click, no drag), BUT button keeps is_active for drag-off-cancel while checkbox doesn't → clicking checkbox + dragging off still fires. UX inconsistency not stated as deliberate. Name is_active vs is_dragging also arbitrary for same concept.

3. Keyboard space/enter block — copy-pasted, no helper
Verbatim space_is_active press/release/activation block duplicated in button, checkbox, radio, switch, chip, select. Should be shared fn. segmented/tabs/drag_number use arrow nav instead (justified).

6. Un-factored activation blocks
Focus got a shared helper. Activation did NOT:

Space/Enter activate (~15 lines): repeated verbatim in checkbox, radio, switch, chip, select.
Left/Right arrow nav: repeated near-verbatim in segmented + tabs.
Up/Down arrow nav: select + drag_number.
All copy-paste. Surprising that focus is DRY but activation is not.

LayoutInfo stuff - not sure if this is still relevant given the recent layout work?

1. Intrinsic sizing (segmented, tabs)

Widget computes its own width from content — sum of all segment/tab label widths — ignoring the input rect's width entirely. segmented.rs:71 sets outer.w = total_w from summed label widths. tabs.rs:50-55 does the same. Input rect provides position (x, y) and height, but width is overridden.

2. Minimum-width enforcement (menu, select)

Widget clamps upward: drawn_w = input_rect.w.max(min_width). menu.rs:52, select.rs:103-108. If caller passes a small rect, widget silently draws wider.

3. Max-width clamping (tooltip)

Opposite direction — tooltip caps width at s.max_width (tooltip.rs:48-49), so drawn size can be smaller than input rect.

Why tooltip/menu report bounds but segmented/tabs/select don't:

Tooltip and menu must tell the caller their real size — the caller uses it to position adjacent content (e.g., avoid overflow, anchor the tip). So raw result carries bounds: Rect with the actual computed rect.

Segmented/tabs/select also compute a different size, but high-level just passes the original input rect as LayoutInfo.bounds (segmented.rs:338, tabs.rs:328, select.rs:459). The layout system thinks the widget occupies the requested rect. The actual draw spills outside (or under-fills) that box silently.

Result: tooltip/menu are "honest" — LayoutInfo.bounds reflects actual draw area. Segmented/tabs/select are "dishonest" — LayoutInfo.bounds is the input rect, not the actual drawn footprint.





* Sample app still does full rendering even when in background, minimsed, nothignb changing etc.


## Things Still to Figure Out

- **Z-ordering for fit-to-children containers** — fit-to-children containers only discover their final outer size at `finish()` time (after their children have run). Because immediate-mode rendering relies on emit order, drawing the container background/border *after* its children causes the background to render on top of the children, covering them. We need a mechanism to allow containers to append backgrounds *under* children, possibly by separate command list buffering or vector slot reservation. For now in Phase 6, we leave the layering "incorrect" in the implementation, as we don't draw container backgrounds just yet.

- **Hit-testing with overlapping widgets** — if a widget drawn later (higher in the visual
  stack) overlaps one drawn earlier, the earlier widget's hit region may still be tested
  first, since it was registered first. We need a clear rule for how draw order, z-order,
  and hit-test priority interact.

- **Clipping** — for the most part, explicit clipping is rarely needed, as UIs generally
  shouldn't have overflowing content. The primary exception is scroll containers, which
  will require some mechanism for scissor rects or clipping regions in the renderer.

- **`LayoutInfo` bounds redundancy** — is there any point in `LayoutInfo` returning the
  overall bounds, given that's always passed in directly when calling a widget function?
* Should the returned content_bounds be screen space or relative to something? If screen-space, are they useful for much?

- ** Layout directions **
  Do we want any kind of right-to-left or bottom-to-top layout options? This is related to the option
  of reordering widget calls to the resolve the "must know info before doing layout"-type questions ('reorder trick').
  For example if you want to right-align stuff but don't know the widths up-front, using the right edge
  as the anchor and building from there might be a good solution.
  * Possibly some kind of "anchor system", where the layout space can have one or more anchors like left edge/right edge/limits

- **Off-screen draw cost** — currently things can be drawn "off screen" or hidden/clipped
  and might still contribute cost. We should check this.

- **Scroll areas and virtualisation** — the app chooses how many widgets to put inside
  scroll areas, so can choose real vs. virtual. Is this the right approach? How does it
  align with the "no opt-in virtualised list" anti-pattern?

- **Text cache miss attribution** — if a widget was "unlucky" and was the one that had the
  glyph atlas cache miss, that might be awkward to figure out in profiling. Will see how
  this plays out in practice.

  * CHeck how ergonomic the high-level APIs are, by going through the sample app UI code to see how it looks. Can it be improved?

* Consider renaming ButtonInfo (WidgetInfo) to ButtonResult/WidgetResult? for clarity?
* Should widgets be returning a LayoutInfo with their bounds, when this is one of the thigns that we always(?) pass in? i.e. just copied out.
Is useful when using builder cos the rect is calculated by the layout, so then maybe the bounds should be returned at hte builder level, not hte widget function level?

- **Keep checking the design/implementation against the manifesto principles so we don't go off track!**

 * Panic on re-using FocusId should give more helpful guidance?

 * &mut ButtonState::default() — using a temporary state created and dropped every frame makes focus go weird - hard to diagnose bug

---

## Feature Checklist

Features to design and implement, roughly in dependency order:

- [ ] `LayoutInfo`, `InputInfo`, `ValueInfo<T>`
- [ ] `WidgetResult` trait
- [ ] Hit-testing and pointer input
- [ ] Buttons and toggles
- [ ] Labels and text measurement
  * All the nice text rendering things like kerning, compositing etc. Text should look great, as good as native OS stuff.
  * Consider moving some/all of the SampleTextSystem into framewise (or a related crate?)
  * Itatlic support - as these are separate .ttf files, we'll need to wrap this up somehow in our SampleTextSystem.
  * Compare our rendered text with a gold-standard OS renderer, ideally include this in our text system integration tests!
- [ ] Scrolling and scroll regions
- [ ] Splitters and drag handles
- [ ] Text editing (`TextEditState`)
- [ ] Grid and table layouts
- [ ] Clipping and layering
- [ ] Popups, menus (window level and context menu), tooltips
- [ ] Combo boxes
- List selects (w/ multi-select)
- [ ] Drag and drop
- [ ] Accessibility and tab order
- [ ] IME support
- [ ] Dialogs (blocking and non-blocking)
- [ ] Tabs
- [ ] Graphics / images
- [ ] Animations (spinners, progress bars, animated scrolling)
- [ ] Window min/max sizing based on layout
* Built-in themes that are good
  * A Framewise-specific one - see Framewise Widgets.html (from Claude Design) for a version of this
    * Go through and make our 'spec page' look as similar as possible to the Claude design mockup. This will be a good way to check off features/improvements!
  * Windows native lookalike
  * Mac native lookalike

---

## Scrolling — Open Questions & Ideas

- Middle-click hold-and-drag pan; middle-click without holding.
- Click-and-drag pan (touch / mobile).
- "Flinging" (momentum scrolling).
- Consider if arrow keys, home and end should manipulate scroll bars when an inner widget is focused?
  - Clashes with potential use of arrow keys for changing focus navigation
  arrow keys should be used for chaing focus


---

## Splitters

- Need to handle three- and four-way meeting points.
- Maybe a generic grid layout? Or some kind of hierarchical arrangement?

---

## Text Editing

- Right-click context menu (copy/paste etc.)
- Scrolling and clipping within the edit field
- Multi-line editing

---

## Drag and Drop

- Within the same app
- To/from the OS
- Between different windows in the same app

---

## Accessibility

- Up / down / left / right for switching focus is quite poor, especially with scroll containers, partially visible widgets.
  - did a little work on this already and there's some tests but it's not good yet.
  - if navigating within a scroll area, it should probably prefer to move to a (currently) not visible widget within that scroll area then to pop out to one outside (which should scroll the new one into view)
- Tabbing to a widget that's inside a scroll area (possibly nested) should scroll to make it visible (across all nested scroll areas!)
* Buttons text auto-ellipses (same for labels etc. All text?), due to top down layout this is more likely to occur so should be handled nicely. Also have a tooltip to show the full text. Reusable component for this functionality?

---

## Remaining Layout Work

Consider if the new system is good enough - does it make layout usable/ergonomic yet?
Several of the layouts are very similar to one another, can we simplify by combniing some? Perhaps some are supersets of each other?
For SplitRow, do we want an equivlanet SplitCol
For SplitRow, do we want an option to have alignment within each 'cell', like if a widget has a smaller natural size than the cell?

Phases 1–3, 5, and 6 of the original layout proposal are **implemented and documented in `DESIGN.md`** (intrinsic sizing, three-state `AxisBound`, unbounded axes, deferred scroll, fit-to-children frames). Phase 4 is **partially** done: `SplitRow` (declared count, equal cells) shipped; the weighted/grid/match-tallest cases below did not. This section keeps only what's still unbuilt, plus the conceptual framing that justifies *why* some cases are possible and others never will be.

### Framing (now in DESIGN.md)

The **headline rule** (Automate / Declare / Refuse tiers) and **emit ≠ visual ≠ focus order** independence — including the topological-DAG reorder trick — describe the *implemented* model and live in `DESIGN.md` → Layout System. The short version, for context here:

- **Automate** (past-only) ✅ done (P1–P6).
- **Declare** (future sibling, declared count) 🚧 only `SplitRow` so far — the rest is below.
- **Refuse** (self-dependent / over-constrained) — impossible at any phase (table below).

The reorder trick (emit autos to measure → distribute → place → `override_next` to restore focus) is the engine the unbuilt Declare-tier helpers would use internally.

### Phase 4 remainder — declared-structure helpers (⬜ unbuilt)

`SplitRow` was the easy case: equal + known count means every slot resolves independently up front (no measure-all, no emit-reorder). The rest need a **measure-all-then-place / `override_next`** engine:

- Weighted split (left pane 2×, right pane 1×, filling the row)
- Space-between (first item left, last right, even gaps)
- Match-tallest (a row of cards all stretched to the tallest — declared count + measure-all)
- Grid where each column is as wide as its widest cell (declared column count + measure-all)

All require `AxisBound::Exact` on the divided axis (a committed far edge), the same rule that governs `Placement::Fill` and alignment.

### Tier 3 — Refuse (non-goals, impossible at any phase)

Each asks for a value that only exists *after* the thing it controls is decided (circular), asks two rules to win at once (over-constrained), or fills something with no size. Documented so they aren't mistaken for missing features:

| Case | Why never |
|---|---|
| A caption that wraps into a square-ish block | Width depends on wrapped height, which depends on width. No fixed point in one pass. |
| Three buttons each sized to its label, but the first always exactly 2× the others | "Size to text" and "be 2× the others" contradict. |
| A tooltip that hugs its text while the text re-wraps to fit that shrunk width | Width ↔ content loop at container level. |
| A panel filling the height inside a vertically-infinite scrolling list | Filling "unbounded" is meaningless (the `Placement::Fill` + `Unbounded` rule). |
| Two panes staying equal as you drag a divider, both honoring minimums, both filling the window | Simultaneous multi-variable solve — a constraint solver, not a forward pass. |

This is the same width ↔ content self-dependency that bars **constraint-affecting fit** in fit-to-children containers (Phase 6, `DESIGN.md`).

### Future possibility (not scheduled) — pre-declared slot helper

A planned-slot API would package the Tier 2 cases ergonomically:

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

Recorded as a direction, **not** for implementation — three issues need resolving first:

- **Flex forces deferral.** A flex slot's leftover needs every auto slot's size, so no slot right of it can be placed until all `*_in` calls are in. Draw and interaction defer to `finish()`, so results are read by handle *after* `finish()` — you lose inline `if button_in(...).clicked`. Bounded buffering of one row (freed at `finish`), not a retained tree; state still mutates in frame N. But the ergonomic shift is real.
- **Handles, not strings.** Slot keys are typed handles returned at declaration — Framewise rejects string/global IDs; handles give compile-checked fills with no lookup or typo-miss.
- **API surface.** Slot-addressed fills (`label_in`, `button_in`, …) need a twin per widget. Partial coverage would violate Widget Consistency — if pursued, every high-level widget gets an `*_in` twin.

### Invariants any of this must hold

- **Top-down and immediate.** Parent space known before children; no bottom-up constraint solving.
- **One pass for placement.** The only extra traversal is cheap explicit `calc_*` spec measurement, or measure-all-then-place for grid/match-tallest. Neither retains a widget tree.
- **Layout stays a `WidgetContext`-level concept.** Raw widgets receive fully-resolved `Rect`s, never `LayoutSpace` / `IntrinsicSize` / `AxisBound`.
- **Determinism and locality.** Every placement depends only on parent space, caller intent, this widget's measurement, and earlier siblings — never later ones.
- **Three orderings stay independent.** Reordering emit (within a DAG) is the sanctioned bridge from Declare down into Automate.

### Layout validation failures

  - `LayoutResult::Fallback` currently carries a single `LayoutViolation`. A single
    `layout()` call can fault on multiple sub-resolves (width size + height size +
    alignment); we keep only the first for now. Consider making it plural (e.g.
    `violations: Vec<LayoutViolation>` or a small inline array) later if surfacing all
    of them proves useful.
  - A deferred child can fault at *both* `begin_layout` and `end_layout` for the same
    reason (e.g. Center on an AtMost cross axis is unsatisfiable at both points), so the
    `Highlight` policy draws two overlapping red outlines for one child. Genuine (two
    resolution points) but noisy. Consider de-duplicating per child/frame, or only
    reacting at one of the two points, if it proves distracting.
