# Framewise Manifesto

> A Rust GUI library where the app is always in control.

---

## What Framewise Is

Framewise is a small, procedural Rust **library** — not a framework — that helps an
application describe and draw GUI elements for the current frame. It does not retain an
abstract UI tree, does not own an update model, and does not impose a lifecycle on the
application.

The name reflects the core idea: UI is expressed **frame-wise** — described explicitly,
per frame, from current application state.

---

## Core Principles

### 1. The App Is in Control

The application owns its data, its layout decisions, and its performance reasoning.
Framewise provides helpers; it does not own the scene.

- App state, scroll offsets, splitter positions, focus keys, text edit buffers — all live
  in the application's data model, not inside the library.
- If the app stops walking part of its model, that UI simply disappears. There is no
  separate GUI object requiring explicit destruction.

### 2. A Library, Not a Framework

Framewise is not a monolithic `draw_gui()` call that does hundreds of things and can take
50 ms. It is a collection of composable, bounded helpers with obvious, predictable cost.

- No mandatory global context beyond a small explicit draw/input object.
- No framework scheduler or hidden lifecycle.
- Helpers are optional, decomposable, and easy to replace.

### 3. Performance Is Proportional to Visible Work

The cost of a frame should be reasoned about from the current frame's visible structure —
not from hidden history, invalidation chains, or opaque retained trees.

- Emitting 10 widgets this frame costs approximately 10 units of work.
- Emitting the same 10 widgets next frame costs the same, regardless of how many backing
  properties changed.
- A large list and a small list have the same *meaning*; actual cost depends on what is
  visible and what is drawn.
- There is no separate "optimised mode" that silently changes semantics.

### 4. Widget Calls Are Explicit and Final for This Frame

Each widget call receives everything needed to produce that widget for the current frame —
its position, size, text, visual state, and relevant input — and the returned result is
complete immediately.

- No deferred layout passes.
- No reactive bindings or auto-updating property values.
- No hidden second pass that mutates previously constructed widgets.
- The draw list is an **implementation detail**: widgets accumulate draw commands as they
  are constructed, and the final render step is a fast, mechanical serialisation of
  already-decided output.

### 5. Persistent Interaction State Lives in App Data

Some state is genuinely persistent from the user's perspective: keyboard focus, text
selection, cursor position, scroll offsets, draft edit text. Framewise acknowledges this
honestly.

Rather than hiding persistence inside the library (which risks going out of sync) or
refusing to support it (which produces awkward workarounds), Framewise takes a **hybrid**
approach:

- The application stores durable widget state in its own data model — focus keys, text
  edit buffers, scroll positions, active drag states, etc.
- Widget functions receive this state *by value* and return the modified state as part of their result.
- This purely functional pattern `(State, Input) -> (DrawCommands, NewState)` avoids borrow checker conflicts that plague GUI libraries requiring mutable borrows during layout.
- Layout and rendering are still computed on demand, per frame, from current state.
- There is **no** library-owned authoritative widget tree.

#### The Mouse Capture Problem
A classic challenge in immediate-mode GUIs is "mouse capture". If a user clicks on a button and drags the mouse off it onto a second button, the second button shouldn't accidentally trigger a click when the mouse is released. Frameworks usually solve this by hashing strings or positions to generate global IDs, tracking an `active_id` in a central registry. 

Framewise completely rejects global ID registries. Instead, we solve capture by pushing state into the application. Even simple widgets like buttons consume and return a `ButtonState`. The widget itself tracks whether it was the original target of a mouse press, elegantly handling dragging and hover logic purely locally. This requires slightly more boilerplate from the app, but results in a vastly more robust architecture that is completely immune to ID collisions.

#### Alternative Considered: Stateless "Mouse Down Pos"
We considered a stateless alternative to solve capture: storing the initial `mouse_down_pos` in the global `Input` struct, and having each widget check if its rectangle contains that position. This would allow simple buttons to remain entirely stateless and avoid the `ButtonState` boilerplate.

However, we explicitly chose the app-owned `ButtonState` approach for two key reasons:
1. **Consistency:** Complex widgets (like scrollable regions or text inputs) absolutely require app-owned state anyway. Keeping the architecture consistent—where *every* interactive widget owns its state—is cleaner than mixing stateless tricks with stateful widgets.
2. **Robustness:** The stateless position trick can break in edge cases, such as when the UI layout shifts underneath the mouse while the button is held down (e.g. an element is inserted above it, moving the button out from under the original `mouse_down_pos`). By binding the active state directly to the specific widget's data struct, the capture is strictly guaranteed regardless of how the layout shifts.

### 6. Reusable UI Is Composed with Ordinary Rust Functions

A "widget type" in Framewise is simply a function that produces a widget result. Custom
widgets, compound controls, and reusable UI panels are plain Rust functions — no
registration, no subclassing, no plugin API required.

```rust
fn photo_row(photo: &Photo, bounds: Rect, ui: &mut Builder, input: &Input) -> PhotoRowResult {
    // compose labels, images, buttons freely
}
```

### 7. No Magic Layout Engine

Layout is performed explicitly by the application. The library provides helpers for common
patterns (rows, columns, splits, scroll regions, alignment), but these helpers are at most
linear in the widgets they operate on, and the application may perform layout calculations
independently before calling any widget functions.

There is no global layout engine that can surprise the application with non-obvious cost.

---

## What Framewise Avoids

| Anti-pattern | Why it is avoided |
|---|---|
| Hidden widget tree | Breaks app ownership; risks desync with app state |
| Deferred / multi-pass layout | Hides cost; makes performance hard to reason about |
| Reactive bindings / auto-update | Invisible control flow; hard to debug |
| Monolithic `draw_gui()` | Makes the expensive step opaque |
| Mandatory widget IDs or namespaces | Complexity tax for users who do not need them |
| Opt-in "virtualised list" vs. "real list" | A semantic distinction that should not exist |
| Escape hatches that change semantics | `memoized`, `virtualised`, `cached` annotations |
| Framework-owned lifecycle | Focus, visibility, destruction managed by library |

---

## API Shape

Framewise has two layers:

### Low-Level: Pure Widget Functions

Plain functions that receive a full specification and return a typed result composed from
common parts. Every input is explicit; the cost is local.

```rust
fn button(spec: ButtonSpec, input: &Input) -> ButtonResult;
fn label(spec: LabelSpec) -> LabelResult;
fn trackbar(state: &mut TrackbarState, spec: TrackbarSpec, input: &Input) -> TrackbarResult;
```

Widget results are composed from common building blocks:

- **`DrawCommands`** — ordered draw commands for the renderer.
- **`LayoutInfo`** — resolved bounds, content bounds.
- **`InputInfo`** — hovered, pressed, clicked, dragged, focused.
- **`ValueInfo<T>`** — widget-specific semantic result (e.g. trackbar value).

Each widget function returns a concrete struct composed of the parts it actually provides.
No metadata maps. No dynamic type slots.

### Builder Layer: Ergonomic Convenience

A `Builder` carries inherited defaults (theme, font, colours, spacing) and accumulates
draw commands automatically. Child builders copy resolved context from the parent — no
linked lists, no borrow-checker fights.

```rust
let mut ui = Builder::new(input, &theme);
let add   = ui.button(rect, "Add photo");      // → ButtonInfo
let title = ui.label(next_to(add.layout.bounds), "Photos");  // → LabelInfo
```

The builder's `emit` method is generic over `WidgetResult`:

```rust
fn emit<R: WidgetResult>(&mut self, result: R) -> R::Info
```

It extracts draw commands into the frame's draw list and returns the info portion to the
caller. The final render step is a fast, dumb pass over the accumulated draw list.

---

## Input Focus

> ⚠️ Not yet designed — see *Things Still to Figure Out* below.

---

## Text Rendering and Predictability

Text rendering is notoriously complex (shaping, hinting, atlas caching) and is a common source of hidden costs in immediate-mode GUIs. Framewise handles this by strictly separating **preparation** from **rendering**.

To draw text, the widget building pass must have access to a `TextSystem` (provided by the application).

- **Widget pass:** The widget asks the `TextSystem` to prepare a string. The text system shapes the string, updates its internal glyph atlas if there are cache misses, and returns a size and an opaque `TextHandle`. If this is slow, it will be easily attributable to the widget which was responsible for requesting this particular text. If this widget was just 'unlucky' and was the one that had the cache miss, then that might be awkward to figure out, will see how this plays out in practice.
- **Render pass:** The library emits `DrawCmd::Text(TextHandle)`. The renderer blindly draws the pre-cached quads.

Because the `Builder` takes the text system as a generic parameter (`Builder<'a, T: TextSystem>`), we guarantee **static dispatch** and maximum inlining, keeping the library zero-cost while maintaining complete renderer agnosticism.

---

## Draw Pipeline

```
App draw function
  └── widget calls → DrawCommands accumulated in Builder
        └── Builder::finish() → Vec<DrawCmd>
              └── Renderer consumes draw list (batching, GPU submission)
```

The semantic work (layout, interaction, hit-testing) happens entirely in the first stage.
The render stage is mechanical: no layout, no binding resolution, no hidden updates.

---

## In One Phrase

> **The app owns state, layout, and performance. Framewise provides immediate, composable
> drawing and interaction helpers — with persistent widget state carried honestly in app
> data.**

---

## Comparison with Existing Libraries

| Principle | Nuklear | Dear ImGui | egui | **Framewise** |
|---|---|---|---|---|
| Library, not framework | ✅ | ✅ | Mostly | ✅ |
| App owns state / layout / performance | Mostly | Mostly | Partial | ✅ |
| Widget call fully specifies this frame's widget | Mostly | Mostly | Partial | ✅ |
| No deferred layout / hidden tree | Mostly | Partial | Partial | ✅ |
| Helpers are optional and bounded | ✅ | Mostly | Mostly | ✅ |
| Cost from current-frame visible work | Partial | Partial | Partial | ✅ (goal) |
| Reusable UI via plain functions | ✅ | ✅ | ✅ | ✅ |
| Persistent widget state app-owned | ✗ | ✗ | ✗ | ✅ |

The biggest gap in all existing libraries is that performance cannot be reliably reasoned
about from current-frame visible work alone. Framewise makes that a hard design law, not
just an aspiration.

---

## Things Still to Figure Out

- **Input focus** — how should Tab / Shift+Tab work given the one-pass traversal model?
  Should some widget state be persistent (hybrid approach)? Can the currently-focused
  widget consume a key event and prevent focus from moving? This needs more design work
  before it can be specified.

- **Hit-testing with overlapping widgets** — if a widget drawn later (higher in the visual
  stack) overlaps one drawn earlier, the earlier widget's hit region may still be tested
  first, since it was registered first. We need a clear rule for how draw order, z-order,
  and hit-test priority interact.

---

## Feature Checklist

Features to design and implement, roughly in dependency order:

- [ ] Geometry types, `Rect`, `Color`, `Align`
- [ ] `DrawCmd` and `DrawCommands`
- [ ] `LayoutInfo`, `InputInfo`, `ValueInfo<T>`
- [ ] `WidgetResult` trait and `Builder::emit`
- [ ] Hit-testing and pointer input
- [ ] Buttons and toggles
- [ ] Labels and text measurement
- [ ] Input focus model (TBD — see *Things Still to Figure Out*)
- [ ] Scrolling and scroll regions
- [ ] Splitters and drag handles
- [ ] Text editing (`TextEditState`)
- [ ] Grid and table layouts
- [ ] Clipping and layering
- [ ] Popups, menus, tooltips
- [ ] Drag and drop
- [ ] Accessibility and tab order

* Window min/max sizing based on layout