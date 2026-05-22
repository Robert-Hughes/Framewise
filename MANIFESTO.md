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
    * Currently things can be drawn "off screen" or hidden/clipped and might still contribute cost, we should check this
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

#### State Storage Options
For features requiring state (like Button hover/press tracking, or future TextEdit contents and caret position), Framewise aims to offer the app three flexible approaches to integrating that state, likely managed via traits:
1. **Opt-Out (Null State):** The app chooses not to use the feature. A null implementation is passed, which incurs zero storage and simply disables the behaviour.
2. **Library-Provided Opaque State:** The app wants the feature but doesn't care about the details. It stores a library-provided struct (e.g., `ButtonState`) in its data model. The app fulfills the "app owns the state" philosophy, but treats the struct opaquely.
3. **App-Provided State:** The app implements a trait on its *existing* data. For example, for a text edit value, the app may already have a `String` field it wants to edit directly. It passes this string (or a wrapper implementing the required trait) to the widget, eliminating the need for redundant state synchronisation.

#### Rob notes

The state structs can be composed based on what that widget type can do, e.g. a FocusState struct might be common across lots of widget types. We can also share logic inside the widget functions like a "handle_focus" function that manipulates FocusState could be re-used by many widget types.

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

### 7. Explicit, Trait-Based Layout (No Magic)

Layout is performed explicitly by the application, but we decouple the **configuration** of a layout from its **mutable state**. This avoids the "pyramid of doom" closure nesting found in many immediate-mode libraries, while maintaining pure, linear predictability.

We define two traits:
1. **`Layout`**: The user-facing configuration (e.g., `ColumnLayout { spacing: 4.0 }`). It dictates the `Params` required to position a widget (e.g. `Vec2` for width/height) and provides a `begin(bounds)` method to instantiate the layout's state.
2. **`LayoutState`**: The mutable engine that lives inside the `Builder`. It accumulates positions as widgets are added.

Widget functions on the builder take `layout_params: S::Params` instead of a hardcoded `Rect`. There is **no explicit measuring pass** required from the widget side—the layout dictates the `Rect` from the provided parameters.

#### Example Layouts:
- **`ManualLayout`**: `Params = Rect`. This is an explicit layout where the app specifies exact rectangles. If this layout is nested (e.g. inside a scroll view), it simply treats its bounding box's `top_left` as an offset, ensuring explicit rectangles are still correctly shifted relative to their parent!
- **`ColumnLayout`**: `Params = Vec2`. Stacks widgets vertically, keeping a Y-axis cursor.
- **`RowLayout`**: `Params = Vec2`. Stacks widgets horizontally, keeping an X-axis cursor.
- **`ScrollLayout`**: `Params = Vec2`. A layout configuration that takes an external `&mut ScrollState`. It applies the scroll offset to the `Rect`s returned by its internal layout pass, and automatically pushes a scissor `clip_rect` into the drawing commands.

Because `ScrollLayout` directly shifts the `Rect`s returned during the layout pass, **widgets are physically located at their scrolled screen coordinates when created**. This means standard mouse hit-testing (`rect.contains(mouse_pos)`) works natively without translating input! We only require widgets to optionally test against a `clip_rect` so that hidden, scrolled-out elements aren't accidentally clickable.

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
  * How does this work with our scroll areas - the app chooses how many widgets to put inside them, so can choose real vs. virtual still? Is this what we want?
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
    * Is there any point this returning also the overall bounds, as that's always passed in directly when calling a widget function!!
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

A core challenge of immediate-mode and one-pass GUI architectures is handling keyboard focus traversal (Tab / Shift+Tab) when the "next" widget might not have been evaluated yet.

Framewise solves this elegantly by embracing a **one-frame delay**:
1. Every focusable widget carries a `FocusId` in its app-owned state (like `ButtonState`). This ID is globally unique and persists across frames.
2. The app stores a `FocusSystem` and passes it mutably into widgets.
3. On **Frame N**, as widgets are evaluated, they register their `FocusId` with the `FocusSystem`. The system builds a sequential `current_frame_order`.
4. If the user presses Tab, a shift is requested. At the **end of Frame N**, the `FocusSystem` finds the currently focused widget's index in the `current_frame_order` and picks the next (or previous) ID to become the new focus target.
5. On **Frame N+1**, the newly targeted widget registers its ID, sees that it is the focus target, and draws its focus state.

This gives the application total control over focus ordering. The default is implicit call order, but the app can explicitly insert overrides (`override_next`) to jump focus between disconnected parts of the UI without relying on string hashing or retaining a global UI tree.

---

## The Three Routing Problems (Immediate Mode Challenges)

Because Framewise lacks a retained UI tree, routing user input to the correct widget requires careful architectural thought. These challenges generally fall into three categories, each solved differently:

### 1. Persistent Interaction (Mouse Capture)
* **The Problem:** When you click a button and drag the mouse over another button, the second button shouldn't receive a click when you release.
* **The Solution:** *Purely Local State.* We use the app-owned state (e.g., `ButtonState`) to record `pressed = true`. As long as that specific struct remembers it was pressed, it captures the interaction and ignores bounds checks.
* **Why it works:** The interaction starts with a definitive historical event ("Mouse Down") that locks the state. It requires 0 frames of lag and no global ID registry.

### 2. Sequential Interaction (Keyboard Focus Tabbing)
* **The Problem:** Pressing 'Tab' should move focus to the "next" widget, but in top-down evaluation, the "next" widget hasn't been evaluated yet.
* **The Solution:** *1-Frame Delay + Global ID.* Widgets register their `FocusId` in sequence. At the end of Frame N, the `FocusSystem` determines the next ID. In Frame N+1, the new widget claims focus.
* **Why it works:** The spatial relationship ("who is next") is only known *after* the entire UI is evaluated, forcing us to accept a 1-frame delay managed by a central system.

### 3. Spatial Overlap Interaction (Hover & Scrolling)
* **The Problem:** The mouse is hovering over nested elements that overlap the exact same pixel (e.g., a scroll area inside a scroll area). Who gets the mouse wheel event?
* **The Solution:** *1-Frame Delay + Central Tracking.* Similar to Keyboard Focus, widgets register that they are hovered. Because inner widgets evaluate after outer widgets, the innermost widget overwrites the parent to "win" the hover state for that pixel. In Frame N+1, only the winning ID is allowed to consume the scroll event.
* **The Guiding Principle:** Why not solve this locally by having the inner widget consume the event bottom-up when its scope closes? Because doing so would mutate the widget's local state *after* it has already laid out its children. This violates a core Framewise principle: **If local state is modified in Frame N, it must visually reflect in Frame N.** If a state change must be delayed to Frame N+1 (due to top-down evaluation constraints), that pending intent must be explicitly stored in a central system (like `FocusSystem` or `InteractionSystem`), not quietly hidden inside local widget state.

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

- **Hit-testing with overlapping widgets** — if a widget drawn later (higher in the visual
  stack) overlaps one drawn earlier, the earlier widget's hit region may still be tested
  first, since it was registered first. We need a clear rule for how draw order, z-order,
  and hit-test priority interact.

- **Clipping** — for the most part, explicit clipping is rarely needed, as UIs generally
  shouldn't have overflowing content. The primary exception is scroll containers, which
  will require some mechanism for scissor rects or clipping regions in the renderer.

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
- [x] Input focus model
- [ ] Scrolling and scroll regions
  * Mouse and keyboard interaction with scrollbar
  * Horizontal scrolling
  * Nested scrolling - if reaches end then it should scroll the next outer
- [ ] Splitters and drag handles
- [ ] Text editing (`TextEditState`)
  * Right click text stuff like copy/paste
- [ ] Grid and table layouts
- [ ] Clipping and layering
- [ ] Popups, menus, tooltips
- [ ] Drag and drop
  * Within app
  * To/from OS
  * Between different windows in the same app
- [ ] Accessibility and tab order
    * Up/down/left/right for switching focus, as well as Tab?
    * Tabbing into a widget that's within a scroll area (maybe nested), should scroll to view it
- IME stuff
- Dialogs, blocking and non-blocking
- tabs
- graphics/images
- animations - spinners, loading/progress bars

* Window min/max sizing based on layout

## Scroll Areas and Clip Rects

Recent design decisions have decoupled layouts from input handling and clipping.

-   **Decorator Layouts**: Layouts like \OffsetLayout<L>\ are pure decorators. They wrap another layout and modify the returned rectangles (e.g. subtracting an offset). They do NOT track rendering state, apply clipping, or hold application state.
-   **Widget-Driven Clipping**: Scroll Areas are implemented as low-level widgets. The widget explicitly calculates scroll bounds, handles mouse wheel interactions, and pushes a \PushClip\ command to the draw list.
-   **Builder Scope Management**: The \Builder\ handles closing scopes. If a widget pushes a clip, the child builder created for that scope is flagged with \
eeds_pop_clip = true\, ensuring a \PopClip\ is safely appended when \inish()\ is called.

