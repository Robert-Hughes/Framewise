# Framewise Design

Detailed design decisions and implementation architecture for Framewise.

---

## State Storage Options

For features requiring state (like Button hover/press tracking, or future TextEdit contents
and caret position), Framewise offers the app three flexible approaches to integrating
that state, managed via traits:

1. **Opt-Out (Null State):** The app chooses not to use the feature. A null implementation
   is passed, which incurs zero storage and simply disables the behaviour.
2. **Library-Provided Opaque State:** The app wants the feature but doesn't care about the
   details. It stores a library-provided struct (e.g., `ButtonState`) in its data model.
   The app fulfills the "app owns the state" philosophy, but treats the struct opaquely.
3. **App-Provided State:** The app implements a trait on its *existing* data. For example,
   for a text edit value, the app may already have a `String` field it wants to edit
   directly. It passes this string (or a wrapper implementing the required trait) to the
   widget, eliminating the need for redundant state synchronisation.

State structs can be composed based on what that widget type can do — e.g. a `FocusState`
struct might be common across many widget types. We can also share logic inside widget
functions: a `handle_focus` function that manipulates `FocusState` could be re-used by
many widget types.

---

## Mouse Capture

A classic challenge in immediate-mode GUIs is "mouse capture". If a user clicks on a
button and drags the mouse off it onto a second button, the second button shouldn't
accidentally trigger a click when the mouse is released.

Framewise completely rejects global ID registries. Instead, we solve capture by pushing
state into the application. Even simple widgets like buttons consume and return a
`ButtonState`. The widget itself tracks whether it was the original target of a mouse
press, elegantly handling dragging and hover logic purely locally. This requires slightly
more boilerplate from the app, but results in a vastly more robust architecture that is
completely immune to ID collisions.

### Alternative Considered: Stateless "Mouse Down Pos"

We considered a stateless alternative: storing the initial `mouse_down_pos` in the global
`Input` struct, and having each widget check if its rectangle contains that position. This
would allow simple buttons to remain entirely stateless and avoid the `ButtonState`
boilerplate.

We explicitly chose the app-owned `ButtonState` approach for two key reasons:

1. **Consistency:** Complex widgets (like scrollable regions or text inputs) absolutely
   require app-owned state anyway. Keeping the architecture consistent — where *every*
   interactive widget owns its state — is cleaner than mixing stateless tricks with
   stateful widgets.
2. **Robustness:** The stateless position trick can break in edge cases, such as when the
   UI layout shifts underneath the mouse while the button is held down (e.g. an element is
   inserted above it, moving the button out from under the original `mouse_down_pos`). By
   binding the active state directly to the specific widget's data struct, the capture is
   strictly guaranteed regardless of how the layout shifts.

---

## Layout System

We decouple the **configuration** of a layout from its **mutable state**. This avoids the
"pyramid of doom" closure nesting found in many immediate-mode libraries, while maintaining
pure, linear predictability.

We define two traits:

1. **`Layout`**: The user-facing configuration (e.g., `ColumnLayout { spacing: 4.0 }`).
   It dictates the `Params` required to position a widget (e.g. `Vec2` for width/height)
   and provides a `begin(bounds)` method to instantiate the layout's state.
2. **`LayoutState`**: The mutable engine that lives inside the `Builder`. It accumulates
   positions as widgets are added.

Widget functions on the builder take `layout_params: S::Params` instead of a hardcoded
`Rect`. There is **no explicit measuring pass** required from the widget side — the layout
dictates the `Rect` from the provided parameters.

### Built-in Layouts

- **`ManualLayout`**: `Params = Rect`. Explicit layout where the app specifies exact
  rectangles. If nested (e.g. inside a scroll view), it treats its bounding box's
  `top_left` as an offset, so explicit rectangles are correctly shifted relative to their
  parent.
- **`ColumnLayout`**: `Params = Vec2`. Stacks widgets vertically, keeping a Y-axis cursor.
- **`RowLayout`**: `Params = Vec2`. Stacks widgets horizontally, keeping an X-axis cursor.
- **`ScrollLayout`**: `Params = Vec2`. Takes an external `&mut ScrollState`. Applies the
  scroll offset to the `Rect`s returned by its internal layout pass, and automatically
  pushes a scissor `clip_rect` into the drawing commands.

Because `ScrollLayout` directly shifts the `Rect`s returned during the layout pass,
**widgets are physically located at their scrolled screen coordinates when created**. This
means standard mouse hit-testing (`rect.contains(mouse_pos)`) works natively without
translating input. We only require widgets to optionally test against a `clip_rect` so
that hidden, scrolled-out elements aren't accidentally clickable.

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

A core challenge of immediate-mode and one-pass GUI architectures is handling keyboard
focus traversal (Tab / Shift+Tab) when the "next" widget might not have been evaluated yet.

Framewise solves this by embracing a **one-frame delay**:

1. Every focusable widget carries a `FocusId` in its app-owned state (like `ButtonState`).
   This ID is globally unique and persists across frames.
2. The app stores a `FocusSystem` and passes it mutably into widgets.
3. On **Frame N**, as widgets are evaluated, they register their `FocusId` with the
   `FocusSystem`. The system builds a sequential `current_frame_order`.
4. If the user presses Tab, a shift is requested. At the **end of Frame N**, the
   `FocusSystem` finds the currently focused widget's index in `current_frame_order` and
   picks the next (or previous) ID to become the new focus target.
5. On **Frame N+1**, the newly targeted widget registers its ID, sees that it is the focus
   target, and draws its focus state.

This gives the application total control over focus ordering. The default is implicit call
order, but the app can explicitly insert overrides (`override_next`) to jump focus between
disconnected parts of the UI without relying on string hashing or retaining a global UI
tree.

---

## The Three Routing Problems

Because Framewise lacks a retained UI tree, routing user input to the correct widget
requires careful architectural thought. These challenges fall into three categories:

### 1. Persistent Interaction (Mouse Capture)

- **The Problem:** When you click a button and drag the mouse over another button, the
  second button shouldn't receive a click when you release.
- **The Solution:** *Purely Local State.* We use the app-owned state (e.g., `ButtonState`)
  to record `pressed = true`. As long as that specific struct remembers it was pressed, it
  captures the interaction and ignores bounds checks.
- **Why it works:** The interaction starts with a definitive historical event ("Mouse Down")
  that locks the state. It requires 0 frames of lag and no global ID registry.

### 2. Sequential Interaction (Keyboard Focus Tabbing)

- **The Problem:** Pressing 'Tab' should move focus to the "next" widget, but in top-down
  evaluation, the "next" widget hasn't been evaluated yet.
- **The Solution:** *1-Frame Delay + Global ID.* Widgets register their `FocusId` in
  sequence. At the end of Frame N, the `FocusSystem` determines the next ID. In Frame N+1,
  the new widget claims focus.
- **Why it works:** The spatial relationship ("who is next") is only known *after* the
  entire UI is evaluated, forcing us to accept a 1-frame delay managed by a central system.

### 3. Spatial Overlap Interaction (Hover & Scrolling)

- **The Problem:** The mouse is hovering over nested elements that overlap the exact same
  pixel (e.g., a scroll area inside a scroll area). Who gets the mouse wheel event?
- **The Solution:** *1-Frame Delay + Central Tracking.* Widgets register that they are
  hovered. Because inner widgets evaluate after outer widgets, the innermost widget
  overwrites the parent to "win" the hover state for that pixel. In Frame N+1, only the
  winning ID is allowed to consume the scroll event.
- **The Guiding Principle:** Why not solve this locally by having the inner widget consume
  the event bottom-up when its scope closes? Because doing so would mutate the widget's
  local state *after* it has already laid out its children. This violates a core Framewise
  principle: **If local state is modified in Frame N, it must visually reflect in Frame N.**
  If a state change must be delayed to Frame N+1 (due to top-down evaluation constraints),
  that pending intent must be explicitly stored in a central system (like `FocusSystem` or
  `InteractionSystem`), not quietly hidden inside local widget state.

---

## Text Rendering

Text rendering is notoriously complex (shaping, hinting, atlas caching) and is a common
source of hidden costs in immediate-mode GUIs. Framewise handles this by strictly
separating **preparation** from **rendering**.

To draw text, the widget building pass must have access to a `TextSystem` (provided by the
application).

- **Widget pass:** The widget asks the `TextSystem` to prepare a string. The text system
  shapes the string, updates its internal glyph atlas if there are cache misses, and
  returns a size and an opaque `TextHandle`.
- **Render pass:** The library emits `DrawCmd::Text(TextHandle)`. The renderer blindly
  draws the pre-cached quads.

Because the `Builder` takes the text system as a generic parameter (`Builder<'a, T: TextSystem>`),
we guarantee **static dispatch** and maximum inlining, keeping the library zero-cost while
maintaining complete renderer agnosticism.

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

## Scroll Areas, Scopes, and Nested Scroll Claims

Design decisions around how complex widgets like Scroll Areas interact with layout,
clipping, and nested inputs.

- **Decorator Layouts**: Layouts like `OffsetLayout<L>` are pure decorators. They wrap
  another layout and modify the returned rectangles (e.g. subtracting an offset). They do
  NOT track rendering state, apply clipping, or hold application state.
- **Widget-Driven Lifecycle Scopes**: Complex widgets like Scroll Areas return a
  `ScrollAreaScope`. Calling `.finish()` on the scope explicitly manages symmetrical
  commands (like `PopClip`).
- **Strict Lifecycle Enforcement**: The `ScrollAreaScope` has an internal `Drop`
  implementation that panics if `finish()` is not called. This statically and dynamically
  prevents leaking input state and clip rects.
- **Builder Transparency**: The `Builder` seamlessly integrates with these scopes. Outer
  builders immediately append `pre_cmds`, and when `.finish()` is called, the child builder
  automatically executes `scope.finish()` and appends the `post_cmds`.
- **Bottom-Up Scroll Claims**: To handle nested scroll areas gracefully without
  immediate-mode input loops, the `FocusSystem` employs a 1-frame delayed "claim"
  architecture. Inner scroll areas register claims (`claim_scroll_up`, `claim_pgdn`, etc.).
  Because scopes are finished bottom-up, innermost scroll areas always get first pick of
  the claim.
- **Standalone Widget Participation**: Standalone widgets like standalone sliders actively
  participate in this claim system (using `claim_scroll_at_ends`). When hovered or focused,
  they block scroll inputs from propagating up to outer scroll areas, acting as "hard stops"
  instead of allowing the parent to suddenly start scrolling when the slider hits its
  boundary.
