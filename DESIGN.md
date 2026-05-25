# Framewise Design

Detailed design decisions and implementation architecture for Framewise.

---

## State Storage Options

For features requiring state (like Button hover/press tracking, or TextEdit contents and caret position), Framewise offers the app two flexible approaches to integrating that state:

1. **Library-Provided Opaque State:** The app stores a library-provided struct (e.g., `ButtonState`, `TextEditState`) in its data model. The app fulfills the "app owns the state" philosophy, treating the struct opaquely and passing it mutably to the widget.
2. **App-Managed State:** The app extracts and passes values directly (such as simple values or sub-fields of its existing data structures) to widget specs, keeping synchronization simple, explicit, and direct without complex trait layers.

State structs are composed based on what that widget type can do — e.g. a `FocusState` struct might be common across many widget types. We also share logic inside widget functions: a `handle_focus` function that manipulates `FocusState` could be re-used by many widget types.

---

## Mouse Capture

A classic challenge in immediate-mode GUIs is "mouse capture". If a user clicks on a button and drags the mouse off it onto a second button, the second button shouldn't accidentally trigger a click when the mouse is released.

Framewise completely rejects global ID registries. Instead, we solve capture by pushing state into the application. Even simple widgets like buttons consume and return a `ButtonState`. The widget itself tracks whether it was the original target of a mouse press, elegantly handling dragging and hover logic purely locally. This requires slightly more boilerplate from the app, but results in a vastly more robust architecture that is completely immune to ID collisions.

### Alternative Considered: Stateless "Mouse Down Pos"

We considered a stateless alternative: storing the initial `mouse_down_pos` in the global `Input` struct, and having each widget check if its rectangle contains that position. This would allow simple buttons to remain entirely stateless and avoid the `ButtonState` boilerplate.

We explicitly chose the app-owned `ButtonState` approach for two key reasons:

1. **Consistency:** Complex widgets (like scrollable regions or text inputs) absolutely require app-owned state anyway. Keeping the architecture consistent — where *every* interactive widget owns its state — is cleaner than mixing stateless tricks with stateful widgets.
2. **Robustness:** The stateless position trick can break in edge cases, such as when the UI layout shifts underneath the mouse while the button is held down (e.g. an element is inserted above it, moving the button out from under the original `mouse_down_pos`). By binding the active state directly to the specific widget's data struct, the capture is strictly guaranteed regardless of how the layout shifts.

---

## Layout System

We decouple the **configuration** of a layout from its **mutable state**. This avoids the "pyramid of doom" closure nesting found in many immediate-mode libraries, while maintaining pure, linear predictability.

### Why Top-Down (Bounds-First) Layout

Top-down layout — where the parent dictates the bounds children must fit into — is philosophically natural for GUI applications for a simple reason: **you almost always know the size of your container but not the size of your content.**

A window's dimensions are set by the user or the OS. A panel's width comes from your app's layout. But the content inside — user-typed text, a dynamically-loaded list, a network-fetched image — is fundamentally unknown until it arrives.

Bottom-up ("auto-size") layout inverts this: children measure themselves and report their natural size upward. This is elegant when content drives the layout, but it requires a separate measurement pass, makes constraint propagation complex, and forces every widget to handle the case where content size is genuinely unknown. Scroll areas handle the "content is larger than the view" case cleanly: the content gets its logical bounds, the view clips it.

### Layout is a Context-Level Concept

`Layout` and `LayoutState` are high-level abstractions that live exclusively in the `WidgetContext` layer. **Low-level widget functions know nothing about layouts.** They receive and return plain geometry: `Rect`, `Vec2` offset, `Option<Rect>` clip. Layout is a building aid — it helps place widgets in the right position — but it does not change what a widget does or how it draws.

Concretely: `begin_scroll_area` returns `(scope, content_bounds, offset)`. The parent `WidgetContext` wraps these primitives into a child `WidgetContext` parameterized with `OffsetLayout { offset, inner }` to handle offsets and clipping. Low-level widgets receive fully-resolved bounds from this context.

This separation means adding a new layout type (e.g. `GridLayout`) requires zero changes to any widget function.

We define two traits:

1. **`Layout`**: The user-facing configuration (e.g., `ColumnLayout { spacing: 4.0 }`). It dictates the `Params` required to position a widget (e.g. `Vec2` for width/height) and provides a `begin(bounds)` method to instantiate the layout's state.
2. **`LayoutState`**: The mutable engine that lives inside the `WidgetContext`. It accumulates positions as widgets are added.

Widget functions on the context take `layout_params: S::Params` instead of a hardcoded `Rect`. There is **no explicit measuring pass** required from the widget side — the layout dictates the `Rect` from the provided parameters.

### Built-in Layouts

- **`ManualLayout`**: `Params = Rect`. Explicit layout where the app specifies exact rectangles. If nested (e.g. inside a scroll view), it treats its bounding box's `top_left` as an offset, so explicit rectangles are correctly shifted relative to their parent.
- **`ColumnLayout`**: `Params = Vec2`. Stacks widgets vertically, keeping a Y-axis cursor.
- **`RowLayout`**: `Params = Vec2`. Stacks widgets horizontally, keeping an X-axis cursor.
- **`ScrollLayout`**: `Params = Vec2`. Takes an external `&mut ScrollState`. Applies the scroll offset to the `Rect`s returned by its internal layout pass, and automatically pushes a scissor `clip_rect` into the drawing commands.

Because `ScrollLayout` directly shifts the `Rect`s returned during the layout pass, **widgets are physically located at their scrolled screen coordinates when created**. This means standard mouse hit-testing (`rect.contains(mouse_pos)`) works natively without translating input. We only require widgets to optionally test against a `clip_rect` so that hidden, scrolled-out elements aren't accidentally clickable.

---

## API Shape

Framewise has two layers:

### Low-Level: Raw Widget Functions

Plain, low-level functions residing in `raw` submodules (e.g., `widgets::button::raw::button` or `button_raw`). They are completely decoupled from `WidgetContext` and the layout system. They receive a fully resolved explicit specification struct and return a concrete typed result containing raw draw commands and interaction/layout info. Every input is explicit; the cost is strictly local.

```rust
pub fn button(spec: ButtonSpec, input: &Input) -> ButtonResult;
pub fn label(spec: LabelSpec) -> LabelResult;
pub fn text_edit(spec: TextEditSpec, state: TextEditState, input: &Input) -> TextEditResult;
```

Each widget function returns a concrete struct composed of the parts it actually provides (e.g., `DrawCommands`, `ButtonInfo`, `TextEditInfo`). There are no traits, no metadata maps, and no dynamic type slots.

### High-Level Freestanding API: Context Integration

A unified `WidgetContext<'a, T, S>` carries style parameters (theme, current text size, colors, clip rectangles, time) and system resources (mutable references `&'a mut T` to the text system and `&'a mut FocusSystem` to the focus manager). 

High-level widget APIs are freestanding, highly ergonomic functions that accept a mutable reference to `WidgetContext` along with a simplified spec/state:

```rust
pub fn button<T, S>(
    ctx: &mut WidgetContext<T, S>,
    state: ButtonState,
    layout_params: S::Params,
    text: String,
    input: &Input,
) -> ButtonInfo;
```

These freestanding functions automatically:
1. Resolve layout geometries using the context's layout engine.
2. Resolve styling parameters from the context's current settings.
3. Call the low-level `raw` widget functions.
4. Accumulate the returned draw commands inside the `WidgetContext`'s internal buffer.
5. Return the high-level semantic info to the caller.

### Theme and Font Boundaries

`Theme` is part of the high-level API. The `WidgetContext` uses it to resolve ergonomic defaults such as colours, spacing, and semantic font choices, but low-level widget functions must not depend on a theme. A low-level `WidgetSpec` is already fully resolved by the time it is passed to the widget function.

> [!IMPORTANT]
> **Static Check Rule:** Widgets must not import `theme::Theme` or call `Theme::framewise` outside tests. All low-level widgets in `framewise/src/widgets/*` must consume fully resolved `WidgetSpec`/`Style` data only.

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

- **The Problem:** The mouse is hovering over nested elements that overlap the exact same pixel (e.g., a scroll area inside a scroll area). Who gets the mouse wheel event?
- **The Solution:** *1-Frame Delay + Central Tracking.* Widgets register that they are hovered. Because inner widgets evaluate after outer widgets, the innermost widget overwrites the parent to "win" the hover state for that pixel. In Frame N+1, only the winning ID is allowed to consume the scroll event.
- **The Guiding Principle:** Why not solve this locally by having the inner widget consume the event bottom-up when its scope closes? Because doing so would mutate the widget's local state *after* it has already laid out its children. This violates a core Framewise principle: **If local state is modified in Frame N, it must visually reflect in Frame N.** If a state change must be delayed to Frame N+1 (due to top-down evaluation constraints), that pending intent must be explicitly stored in a central system (like `FocusSystem` or `InteractionSystem`), not quietly hidden inside local widget state.

---

## Text Rendering

Text rendering is notoriously complex (shaping, hinting, atlas caching) and is a common source of hidden costs in immediate-mode GUIs. Framewise handles this by strictly separating **preparation** from **rendering**.

To draw text, the widget building pass must have access to a `TextSystem` (provided by the application).

- **Widget pass:** The widget asks the `TextSystem` to prepare a string. The text system shapes the string, updates its internal glyph atlas if there are cache misses, and returns a size and an opaque `TextHandle`.
- **Render pass:** The library emits `DrawCmd::Text(TextHandle)`. The renderer blindly draws the pre-cached quads.

Because the `WidgetContext` takes the text system as a generic parameter (`WidgetContext<'a, T: TextSystem, S>`), we guarantee **static dispatch** and maximum inlining, keeping the library zero-cost while maintaining complete renderer agnosticism.

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
  └── widget calls → DrawCommands accumulated in WidgetContext
        └── WidgetContext::finish() → Vec<DrawCmd>
              └── Renderer consumes draw list (batching, GPU submission)
```

The semantic work (layout, interaction, hit-testing) happens entirely in the first stage. The render stage is mechanical: no layout, no binding resolution, no hidden updates.

---

## Scroll Areas, Windows, and Symmetrical Container Life-Cycles

Design decisions around how complex container widgets (Scroll Areas and Windows) interact with layout, clipping, and nested inputs.

- **Decorator Layouts**: Layouts like `OffsetLayout<L>` are pure decorators. They wrap another layout and modify the returned rectangles (e.g. subtracting an offset). They do NOT track rendering state, apply clipping, or hold application state.
- **Freestanding Lifecycle Scopes**: Symmetrical container widgets are initiated via `begin_...` freestanding calls and finalized via `end_...` freestanding calls:
  - `begin_scroll_area` / `end_scroll_area`
  - `begin_window` / `end_window`
- **Strict Borrow-Checker Decoupling**: To construct nested scopes without violating Rust's single-mutable-borrow rule, the child `WidgetContext` is created, completely populated with widgets, and then finished using `child.finish()`. This consumes the child context and returns its accumulated draw commands as a `Vec<DrawCmd>`. Once the child is consumed, its borrow on the parent context is released, allowing the parent context to be mutably borrowed again to finalize the container via `end_scroll_area` or `end_window`.
- **Bottom-Up Scroll Claims**: To handle nested scroll areas gracefully without immediate-mode input loops, the `FocusSystem` employs a 1-frame delayed "claim" architecture. Inner scroll areas register claims (`claim_scroll_up`, `claim_pgdn`, etc.). Because scopes are finished bottom-up, innermost scroll areas always get first pick of the claim.
- **Standalone Widget Participation**: Standalone widgets like standalone sliders actively participate in this claim system (using `claim_scroll_at_ends`). When hovered or focused, they block scroll inputs from propagating up to outer scroll areas, acting as "hard stops" instead of allowing the parent to suddenly start scrolling when the slider hits its boundary.

