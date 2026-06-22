# Framewise Notes

Working notes, TODOs, open questions, and half-baked ideas.

## Current Work

- Sliders!
  - collapsed range slider looks weird (overlap etc.) and behaves weird (hovering, which gets dragged)
  - markers being drawn
  - fixed steps
  - are we using the widget helpers and or test helpers?

- Focus outline for text box with scrollbars isn't quite right (thinner around scrollbar)


- Go through the spec_page, check/implement/test each widget/aspect to make better match the mock-up and add interactivity as we go (https://claude.ai/design/p/1aab4e86-cbf2-497e-b379-44cf41de2b12?file=Framewise+Widgets.html)
  - Done 01-03
  - Add demo pages for each widget type (or possibly in groups?)
  - Use/add reusable widget helpers (widget_helpers.rs)
  - Use/add reusable widget test helpers (test_helpers.rs)


## Design Principles

- **Keep checking the design/implementation against the manifesto principles so we don't go off track!**

## Widget Results

- `LayoutInfo`, `InputInfo`, `ValueInfo<T>`
- `WidgetResult` trait

- `LayoutInfo` bounds redundancy — is there any point in `LayoutInfo` returning the
  overall bounds, given that's always passed in directly when calling a widget function?
- Should widgets be returning a LayoutInfo with their bounds, when this is one of the thigns that we always(?) pass in? i.e. just copied out.
Is useful when using builder cos the rect is calculated by the layout, so then maybe the bounds should be returned at hte builder level, not hte widget function level?
- Should the returned content_bounds be screen space or relative to something? If screen-space, are they useful for much?

## Input & Focus

- Hit-testing and pointer input

- Hit-testing with overlapping widgets — if a widget drawn later (higher in the visual
  stack) overlaps one drawn earlier, the earlier widget's hit region may still be tested
  first, since it was registered first. We need a clear rule for how draw order, z-order,
  and hit-test priority interact.
  - Update overlapping widget tests to account for layers

- For widgets using handle_widget_focus + writing InputInfo manually: hovered is rect.contains(input.mouse_pos) && clip.is_none_or(...). Plain hit test (no exclusion for mouse-down-elsewhere).
  Button computes hovered = contains && (!input.mouse_down || state.is_active) — excludes hover while dragging from elsewhere. Semantic mismatch: checkbox hovers when you mouse-down on something else and drag over it; button doesn't.

- Mouse-capture flag — inconsistent, contradicts DESIGN
button: is_active
slider/text_edit/drag_number: is_dragging
checkbox/radio/switch/chip/segmented/tabs/select: no capture flag
chip, segmented, tabs, and select should use handle_press_interaction
DESIGN treats mouse-capture-via-state as foundational robustness mechanism. Toggles drop it. Partly justified (toggles fire on click, no drag), BUT button keeps is_active for drag-off-cancel while checkbox doesn't → clicking checkbox + dragging off still fires. UX inconsistency not stated as deliberate. Name is_active vs is_dragging also arbitrary for same concept.

- Panic on re-using FocusId should give more helpful guidance?

- &mut ButtonState::default() — using a temporary state created and dropped every frame makes focus go weird - hard to diagnose bug

- Keyboard shortcuts like Ctrl+N - these probably shouldn't be handled by every widget that can take keyboard focus just to report it back to the caller. Need some kind of 'global' shortcuts or bubbling/nesting system. Many shortcuts like Ctrl+A/C/V are used by text_edits (for example), need to resolve conflicts somehow.

## Activation

- Keyboard space/enter block — copy-pasted, in chip and select, use handle_press_interaction

- Un-factored activation blocks
Focus got a shared helper. Activation did NOT:

- Space/Enter activate (~15 lines): repeated verbatim in checkbox, radio, switch, chip, select.
Left/Right arrow nav: repeated near-verbatim in segmented + tabs.
Up/Down arrow nav: select + drag_number.
All copy-paste. Surprising that focus is DRY but activation is not.

## Layout

- Intrinsic sizing (segmented, tabs)

  Widget computes its own width from content — sum of all segment/tab label widths — ignoring the input rect's width entirely. segmented.rs:71 sets outer.w = total_w from summed label widths. tabs.rs:50-55 does the same. Input rect provides position (x, y) and height, but width is overridden.

- Minimum-width enforcement (menu, select)

  Widget clamps upward: drawn_w = input_rect.w.max(min_width). menu.rs:52, select.rs:103-108. If caller passes a small rect, widget silently draws wider.

- Max-width clamping (tooltip)

- Opposite direction — tooltip caps width at s.max_width (tooltip.rs:48-49), so drawn size can be smaller than input rect.

- Why tooltip/menu report bounds but segmented/tabs/select don't:

- Tooltip and menu must tell the caller their real size — the caller uses it to position adjacent content (e.g., avoid overflow, anchor the tip). So raw result carries bounds: Rect with the actual computed rect.

- Segmented/tabs/select also compute a different size, but high-level just passes the original input rect as LayoutInfo.bounds (segmented.rs:338, tabs.rs:328, select.rs:459). The layout system thinks the widget occupies the requested rect. The actual draw spills outside (or under-fills) that box silently.

- Result: tooltip/menu are "honest" — LayoutInfo.bounds reflects actual draw area. Segmented/tabs/select are "dishonest" — LayoutInfo.bounds is the input rect, not the actual drawn footprint.

- Layout directions
  - Do we want any kind of right-to-left or bottom-to-top layout options? This is related to the option
  of reordering widget calls to the resolve the "must know info before doing layout"-type questions ('reorder trick').
  For example if you want to right-align stuff but don't know the widths up-front, using the right edge
  as the anchor and building from there might be a good solution.
  - Possibly some kind of "anchor system", where the layout space can have one or more anchors like left edge/right edge/limits

- Consider if the new system is good enough - does it make layout usable/ergonomic yet?
Several of the layouts are very similar to one another, can we simplify by combniing some? Perhaps some are supersets of each other?
For SplitRow, do we want an equivlanet SplitCol
For SplitRow, do we want an option to have alignment within each 'cell', like if a widget has a smaller natural size than the cell?

- Each child within a layout has a lot of options (size, align, spacing etc.). Do we want to have any 'defaults' that you can set at the layout level, so you don't need to repeat for every child. e.g. if you want a column of right-aligned widgets.

- Also if you want just a column of widgets with nice spacing between them, rather than the default of them all being tightly packed!

- WrapLayout still panics - these should be layout violation errors! (Check other panics too!)

- Do we want scroll_area to be able to be 'auto-sized if it fits', i.e. if there's enough space it shrink-wraps children like a Panel, but if not enough space it has scroll bars etc.

### How do we handle cases where a layout doesn't have "enough" remaining space in the LayoutSpace to layout the next child - error? fallback? overflow?

Yes. I think the text edit playground exposed one case of a broader underspecified rule: **is `LayoutSpace` a hard containment constraint, or just an offer/anchor for placement?**

Right now Framewise behaves like a mix:

- `AxisBound::AtMost` is a real clamp for `Auto`.
- `AxisBound::Exact` is an anchor frame, but `Auto` can still resolve to its preferred size and overflow it.
- `Fixed` can exceed remaining space without a violation.
- `resolve_space()` clamps a layout’s reported outer size under `AtMost`, even if a child rect overflowed.
- Layout violations currently represent “cannot interpret this placement rule,” not “child did not fit.”

That last point is important. Existing `LayoutViolationKind` covers things like:
- fill with no exact box
- center/end alignment with no exact anchor
- auto with no preferred size
- placing after a closed end-aligned layout

It does **not** cover “requested 180px but only 100px remained.”

So today, “not enough space” generally means **overflow, silently**, unless the specific placement mode itself is unsatisfiable.

I’d frame the design decision like this:

**Option A: Overflow Is Allowed**
LayoutSpace is an offer plus anchor. Children may exceed it. Clipping and scrolling are explicit widgets. This is simple and immediate-mode friendly, but needs docs and debug tooling so overflow is not surprising.

**Option B: Overflow Is A Layout Violation**
If fixed/auto placement resolves larger than remaining bounded space, emit something like:

```rust
LayoutViolationKind::InsufficientSpace {
    axis,
    available,
    requested,
}
```

Fallback could still place the child at requested size, or clamp depending on policy. This gives visibility without forcing a constraint solver.

**Option C: Hard Clamp By Default**
Every bounded parent clips/clamps children. I would avoid this as the default. It hides layout bugs, makes fixed sizing less honest, and pushes scroll/clip behavior into every layout.

My recommendation: **Option B semantics, Option A fallback.**

Concretely:
- Overflow remains allowed and deterministic.
- It becomes a layout violation when a child’s resolved rect exceeds an `Exact`/`AtMost` remaining bound.
- The fallback rect remains the requested rect, so existing UI does not suddenly shrink.
- Debug/highlight/panic policies can surface it.
- Explicit scroll/clip containers remain the way to contain overflow intentionally.

Then separately add a proper ergonomic API for “auto up to max”:

```rust
ColumnLayoutParams::auto().max_x(420.0).max_y(140.0)
```

or a bounded child/layout helper. The workaround I added to the demo is basically hand-building that missing concept with `LayoutSpace::AtMost`.

So yes: I’d call this a missing spec decision, not just a text edit demo issue. The current implementation has reasonable pieces, but the contract around overflow vs insufficient remaining space needs to be made explicit and tested.

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

### Invariants any of this must hold

- **Top-down and immediate.** Parent space known before children; no bottom-up constraint solving.
- **One pass for placement.** The only extra traversal is cheap explicit `calc_*` spec measurement, or measure-all-then-place for grid/match-tallest. Neither retains a widget tree.
- **Layout stays a `WidgetContext`-level concept.** Raw widgets receive fully-resolved `Rect`s, never `LayoutSpace` / `SizeRequest` / `AxisBound`.
- **Determinism and locality.** Every placement depends only on parent space, caller intent, this widget's measurement, and earlier siblings — never later ones.
- **Three orderings stay independent.** Reordering emit (within a DAG) is the sanctioned bridge from Declare down into Automate.

- Grid and table layouts
- Window min/max sizing based on layout
- DPI awareness/scaling etc.

### Layout Validation

#### Layout validation failures

- `LayoutResult::Fallback` currently carries a single `LayoutViolation`. A single
    `layout()` call can fault on multiple sub-resolves (width size + height size +
    alignment); we keep only the first for now. Consider making it plural (e.g.
    `violations: Vec<LayoutViolation>` or a small inline array) later if surfacing all
    of them proves useful.
- A deferred child can fault at *both* `begin_deferred_layout` and `end_deferred_layout` for the same
    reason (e.g. Center on an AtMost cross axis is unsatisfiable at both points), so the
    `Highlight` policy draws two overlapping red outlines for one child. Genuine (two
    resolution points) but noisy. Consider de-duplicating per child/frame, or only
    reacting at one of the two points, if it proves distracting.

- Nothing to tell you that you forgot to .finish() a widgetcontext - leads to strange layout/clipping bugs, hard to diagnose

### Future Layout Ideas

#### Future possibility (not scheduled) — pre-declared slot helper

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

### Non-Goals

#### Tier 3 — Refuse (non-goals, impossible at any phase)

Each asks for a value that only exists *after* the thing it controls is decided (circular), asks two rules to win at once (over-constrained), or fills something with no size. Documented so they aren't mistaken for missing features:

| Case | Why never |
|---|---|
| A caption that wraps into a square-ish block | Width depends on wrapped height, which depends on width. No fixed point in one pass. |
| Three buttons each sized to its label, but the first always exactly 2× the others | "Size to text" and "be 2× the others" contradict. |
| A tooltip that hugs its text while the text re-wraps to fit that shrunk width | Width ↔ content loop at container level. |
| A panel filling the height inside a vertically-infinite scrolling list | Filling "unbounded" is meaningless (the `Placement::Fill` + `Unbounded` rule). |
| Two panes staying equal as you drag a divider, both honoring minimums, both filling the window | Simultaneous multi-variable solve — a constraint solver, not a forward pass. |

This is the same width ↔ content self-dependency that bars **constraint-affecting fit** in fit-to-children containers (Phase 6, `DESIGN.md`).

## Rendering & Layers

- Revisit z-buffer interaction with alpha blending before relying on layers for
  translucent/anti-aliased content. A higher-z translucent command can write depth
  before a lower-z command behind it has contributed colour, producing incorrect
  compositing around text, AA edges, shadows, and translucent popups. Depth alone is
  fine for opaque ordering, but alpha may require layer-group sorting or another
  explicit compositing rule.

- NOw that we have z-buffering, we could draw the draw commands out-of-order if we assign a different Z for each command, to 'simulate' correct draw order even if we actually batch a bunch together or draw them out of order.

## Pixel Snapping

We want to design a unified strategy for pixel snapping for geometry in Framewise.

### Core Philosophy
- **Separation of Concerns**: Pixel snapping determines where a primitive's boundary/center lands on the physical device grid (e.g., aligning to integer pixel boundaries). This is orthogonal to antialiasing, which determines how edge coverage is computed.
- **Semantic Decisions (Widgets)**: Widgets/emitters (inside Framewise) are responsible for deciding if, when, and how to snap. Snapping should **not** be a hidden, renderer-wide heuristic. The renderer shouldn't automatically coerce layout/geometry, as this weakens semantic boundaries and could corrupt layout calculations.
- **Mechanical Execution (Renderer)**: The renderer acts as a predictable, mechanical consumer of explicit draw commands. However, the renderer should provide low-level mathematical helpers (utilizing device scale, framebuffer mapping, and snapping math for centerlines or edges) that widgets can invoke when building draw commands.

### Proposed Snapping API
- Draw commands and primitive styles will explicitly declare their intent:
  - `snap: PixelSnap` where `PixelSnap` has modes like `{ None, AxisAligned, AxisAlignedIfThin, Centerline }`.

### When Snapping + AA (Hybrid) is Useful
- **Perfect Snap (No AA)**: A static axis-aligned 1 px separator line that is perfectly snapped and integer-aligned doesn't need AA.
- **Snap + AA (Hybrid)**: An axis-aligned 1 px horizontal underline whose vertical (Y) position is snapped to avoid blurriness, but whose horizontal (X) endpoints animate fractionally during transitions (e.g., between tabs).
  - The line body benefits from snapping (keeping it sharp), while the moving endpoints benefit from AA to prevent visible 1 px popping/jittering.


## Clipping & Scrolling

- Clipping — for the most part, explicit clipping is rarely needed, as UIs generally
  shouldn't have overflowing content. The primary exception is scroll containers, which
  will require some mechanism for scissor rects or clipping regions in the renderer.

- Scroll areas and virtualisation — the app chooses how many widgets to put inside
  scroll areas, so can choose real vs. virtual. Is this the right approach? How does it
  align with the "no opt-in virtualised list" anti-pattern?

- Middle-click hold-and-drag pan; middle-click without holding.
- Click-and-drag pan (touch / mobile).
- "Flinging" (momentum scrolling).
- Consider if arrow keys, home and end should manipulate scroll bars when an inner widget is focused?
  - Clashes with potential use of arrow keys for changing focus navigation
  arrow keys should be used for chaing focus

- Consider if mouse wheel scrolling should be 'sticky' the control that it is over when it starts, e.g. if you start scrolling up a page
but then a widget comes under the cursor and that widget uses mouse wheel to change a slider value then that is surprising and annoying. Instead it could
continue to scroll up, and only when you move the mouse does it 'reset' onto the widget under the cursor again?
  - This might make scrollbars on text edits more viable?

## Text

- Labels and text measurement
  - All the nice text rendering things like kerning, compositing etc. Text should look great, as good as native OS stuff.
  - Itatlic support - as these are separate .ttf files, we'll need to wrap this up somehow in our SampleTextBackend.
  - Compare our rendered text with a gold-standard OS renderer, ideally include this in our text system integration tests!

- Text cache miss attribution — if a widget was "unlucky" and was the one that had the
  glyph atlas cache miss, that might be awkward to figure out in profiling. Will see how
  this plays out in practice.

- Buttons text auto-ellipses (same for labels etc. All text?), due to top down layout this is more likely to occur so should be handled nicely. Also have a tooltip to show the full text. Reusable component for this functionality?

## State Storage

### Alternative Considered: Two-Approach Model

An earlier design offered apps two routes for widget value state:

1. **Library-Provided Opaque State:** The app stores a library-provided struct (e.g. `ButtonState`, `TextEditState`) in its data model, treating it opaquely and passing it mutably to the widget.
2. **App-Managed State:** The app extracts and passes values directly (simple scalars or sub-fields of its own data structures) to widget specs, keeping synchronisation simple, explicit, and direct without extra trait layers.

This was dropped in favour of a single approach: library-defined *transparent* state structs, always owned by the app. The rationale is in `DESIGN.md` (§ State Storage), but briefly:

- Widget state is often richer than the app wants to store (caret position, scroll offset, selection range) — the app shouldn't be forced to mirror that structure.
- Apps typically want a *draft* value in the widget and only commit it to their data store after validation or an explicit save action. A library-defined struct naturally holds this draft.

The "App-Managed State" option would require either a trait layer (so the widget can read/write through app types) or reducing state to the subset the app happens to store — both of which lose the richness or the simplicity that the single-struct approach provides.

## Widgets

- Buttons and toggles
- Combo boxes
  - Auto-complete drop down for "editable" combo box.
  - Drop downs autopopulated by dynamic code (e.g. internet fetch), showing a loading spinner/icon
- List selects (w/ multi-select)
- Tabs
- Graphics / images
- Animations (spinners, progress bars, animated scrolling)
- Radio buttons
  - Some concept of a 'group', such that only one is selected at a time. Bind to a value/int/enum and handle the individual radios automatically?

- Button group seams
- HTML removes inner right borders in .fw-btngroup. Rust draws adjacent full buttons, so once groups are switched to secondary/default, internal seams will likely look 2px thick unless the rects overlap by 1px or the button API gains per-side borders.

## Popups & Menus

- Popups, menus (window level and context menu), tooltips
- Dialogs (blocking and non-blocking)
- Could provide a "WindowSystem" trait to framewise which allows it to dynamically spawn new OS windows when it needs a tooltip etc. that overhangs the window.

## Text and Fonts

- Support for italics (currently advertised but does nothing I think!)

## Text Editing

- Text editing (`TextEditState`)
- Right-click context menu (copy/paste etc.)
- Scrolling and clipping within the edit field
- Multi-line editing
- Emojis and multi-character clusters etc.
- IME support
- Undo and redo? (Or built into larger app-wide system?)
- adding spaces to the start of wrapped lines is confusing (appends to prev line instead, no visual indication). This is standard wrapped editor behaviour, but maybe we can do better?
  - would be nice to snap the cursor to the end of the prev line, but we can't do that in a faithful way because the previous line ends in the 'visually collapsed' space that's wrapping!
- user-sizing using drag handle (height and/or width). Re-use 'splitter'/dragging infrastructure (doesn't exist yet!)
- 'Enter key policy' - does it insert a newline (depending on the NewlinePolicy) or 'submit' the form/dialog? Perhaps shift/ctrl/enter is needed, to make a newline, or maybe that's how you submit instead?
- tab alignment within text. Maybe need a 'tab policy' like we could for Enter key?
- Should disabled overflowing text still show scrollbars?
- Should clicking on the scrollbar area of a text_edit set focus to the text edit?


## Splitters

- Splitters and drag handles

- Need to handle three- and four-way meeting points.
- Maybe a generic grid layout? Or some kind of hierarchical arrangement?

## Drag and Drop

- Drag and drop

- Within the same app
- To/from the OS
- Between different windows in the same app

## Accessibility

- Accessibility and tab order

- Up / down / left / right for switching focus is quite poor, especially with scroll containers, partially visible widgets.
  - did a little work on this already and there's some tests but it's not good yet.
  - if navigating within a scroll area, it should probably prefer to move to a (currently) not visible widget within that scroll area then to pop out to one outside (which should scroll the new one into view)
- Tabbing to a widget that's inside a scroll area (possibly nested) should scroll to make it visible (across all nested scroll areas!)
  - perhaps only scroll if the viewport could cover more of the area of the focussed widget (e.g. if the widget is huge and already covers the viewport, no point in moving)

## Themes

- Built-in themes that are good
  - A Framewise-specific one - see Framewise Widgets.html (from Claude Design) for a version of this
    - Go through and make our 'spec page' look as similar as possible to the Claude design mockup. This will be a good way to check off features/improvements!
    - 'Dark' variant (like on the mockup)?
  - Windows native lookalike
  - Mac native lookalike

## Sample App

- CHeck how ergonomic the high-level APIs are, by going through the sample app UI code to see how it looks. Can it be improved?

- Sample app still does full rendering even when in background, minimsed, nothignb changing etc.

## Performance

- Off-screen draw cost — currently things can be drawn "off screen" or hidden/clipped
  and might still contribute cost. We should check this.
    - e.g. scroll area with lots of widgets
    - e.g. text edit with lots of text that's clipped.
- Early-out from widget functions if it's offscreen or completely clipped
- Resizing window is v. slow
- Large text edits get very slow!
  - especially clicking and dragging a large selection (or shift + pg down)
- If layout params are fixed in an axis, do not compute an expensive size request for that axis. If both axes are fixed, do not compute a size request at all.

## API Ergonomics

- WidgetContext having lots of generics makes it annoying to pass around, and to write functions that work for different kinds of layout etc.
- Constructing a widget like: label(ctx, LabelStyle { ctx.theme.font_size, .. }) (where you use the ctx to calculate a param) fails due to double-borrow on ctx - annoying!
