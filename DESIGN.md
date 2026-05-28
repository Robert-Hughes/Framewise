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

Framewise completely rejects global ID registries. Instead, we solve capture by pushing state into the application. Even simple widgets like buttons take a `&mut ButtonState`. The widget itself tracks whether it was the original target of a mouse press, elegantly handling dragging and hover logic purely locally. This requires slightly more boilerplate from the app, but results in a vastly more robust architecture that is completely immune to ID collisions.

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

Concretely: `raw::begin_scroll_area` returns `(pre_cmds, token, content_bounds, offset)`. The high-level `begin_scroll_area` captures the token in an `on_finish` closure and wraps these primitives into a child `WidgetContext` parameterized with `OffsetLayout { offset, inner }` to handle offsets and clipping. Low-level widgets receive fully-resolved bounds from this context.

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

Plain, low-level functions residing in `raw` submodules (e.g., `widgets::button::raw::button`). They are completely decoupled from `WidgetContext` and the layout system. They receive a fully resolved explicit specification struct and return a `raw::*Result` containing draw commands and interaction info. Every input is explicit; the cost is strictly local.

```rust
pub fn button(spec: ButtonSpec, input: &Input) -> raw::ButtonResult;
pub fn label(spec: LabelSpec) -> raw::LabelResult;
pub fn text_edit(spec: TextEditSpec, state: TextEditState, input: &Input) -> raw::TextEditResult;
```

Each `raw::*Result` is a concrete struct with no traits, no metadata maps, and no dynamic type slots.

### High-Level Freestanding API: Context Integration

A unified `WidgetContext<'a, T, S, CF>` carries style parameters (theme, current text size, colors, clip rectangles, time) and system resources (mutable references `&'a mut T` to the text system and `&'a mut FocusSystem` to the focus manager). The `CF` parameter is a one-shot cleanup closure (`FnOnce(&mut FocusSystem) -> DrawCommands`) called when the context is finished; root contexts use a no-op function pointer, container widgets embed their cleanup in a move closure (see [Scroll Areas and Windows](#scroll-areas-windows-and-symmetrical-container-life-cycles)).

High-level widget APIs are freestanding, highly ergonomic functions that accept a mutable reference to `WidgetContext` along with a simplified spec/state:

```rust
pub fn button<T, S>(
    ctx: &mut WidgetContext<T, S>,
    state: &mut ButtonState,
    layout_params: S::Params,
    builder: ButtonSpecBuilder,
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
- `draw: DrawCommands` — the caller manages command accumulation directly
- Interaction outputs (`InputInfo`, `focused`, etc.)
- `content_bounds: Rect` when the widget computes an inner area distinct from the input rect (e.g. a widget with a border or padding). The raw function is the authoritative place to compute this, since it has the spec in hand.
- **Not** the input `Rect` itself — the caller supplied it explicitly, echoing it back is redundant
- **Not** `*State` — state is mutated in-place via the `&mut *State` parameter

**`*Result`** is returned by the high-level context function. It contains:
- `layout: LayoutInfo` — includes `bounds` (the rect resolved by the layout engine, which the caller did not know before calling) and `content_bounds`
- The same interaction outputs as `raw::*Result`
- **Not** `DrawCommands` — accumulated into `WidgetContext` automatically
- **Not** `*State` — mutated in-place

The high-level function maps between them: it calls `ctx.layout()` to resolve geometry, calls `raw::widget()`, pushes draw commands into the context, then constructs the `*Result` forwarding the interaction fields and adding `LayoutInfo`.

### Spec and SpecBuilder Pattern

Every widget type follows a consistent two-struct pattern for configuration:

- **`*Spec`**: A fully resolved specification struct used by low-level raw widget functions. All fields are concrete values (colors, fonts, rectangles, etc.) with no optional or unresolved state. The low-level function receives this spec and produces draw commands and interaction info. Each `*Spec` struct is defined inside the widget's `pub mod raw {}` submodule (e.g. `button::raw::ButtonSpec`), co-located with the raw function that consumes it, and avoids clutter the normal module level with stuff the high-level user won't use.

- **`*SpecBuilder`**: A builder struct used by high-level widget functions to construct the `*Spec`. The builder holds optional fields and provides ergonomic setter methods. The high-level function uses the builder to:
  1. Apply defaults from the `WidgetContext` (via `.defaults_from_theme()` and `.rect()`)
  2. Allow the app to override specific parameters (via setter methods like `.text()`, `.style()`, etc.)
  3. Call `.build()` to produce the fully resolved `*Spec` for the low-level function

This pattern cleanly separates concerns:
- **Low-level functions** are pure and testable — they receive explicit values and produce explicit results, with no knowledge of themes, layouts, or context.
- **High-level functions** are ergonomic and integrated — they resolve defaults from the context, handle layout, and bridge to the low-level layer.

> [!IMPORTANT]
> **Spec and SpecBuilder Value-Type Rule:** `*Spec` and `*SpecBuilder` structs must contain only basic parameters (colors, fonts, rectangles, strings, numeric values, etc.). They must NOT include references to "systems" like `Input`, `FocusSystem`, `TextSystem`, or other external state. These structs should be pure value-types with no external references, making them trivially copyable, serializable, and independent of any runtime context.

> [!IMPORTANT]
> **Theme Must Not Appear in `*Spec`:** A `*Spec` struct must never hold a `Theme` field. `Theme` is a high-level convenience that maps semantic intent to concrete values; by the time a spec is constructed, that mapping is complete. The `*SpecBuilder` is the only place `Theme` is touched — its `defaults_from_theme()` method reads the theme and writes resolved colours, sizes, and font handles into the builder's fields. The resulting `*Spec` contains only those resolved primitives. This keeps every `*Spec` self-contained and renderer-agnostic, and prevents the low-level widget layer from having any dependency on the theme system.

> [!IMPORTANT]
> **Builder Construction Rule:** All `*SpecBuilder` structs use a no-args `new()` constructor. No field is singled out as a required constructor parameter — **every field, including bool flags like `disabled` and `large`, is `Option<T>`** and starts as `None`. `build()` applies defaults for fields that have an obvious, context-independent value (e.g. `disabled` → `unwrap_or(false)`) and panics with a clear message for fields with no sensible default; the message names the missing field and points to the fix (e.g. *"style not set — call .style() or defaults_from_theme()"*, *"rect not set — call .rect() or use the high-level API"*). Making every field `Option<T>` is essential: `None` means "the user did not set this", which lets both `defaults_from_theme` and the high-level widget function inject context-aware defaults — something impossible if bools silently default to `false` in `new()`.

### `defaults_from_theme` — Theme as Fallback

Every `*SpecBuilder` exposes a `defaults_from_theme(theme: &Theme)` method. It fills only the fields that are **not already set** — theme values are fallbacks, not overrides. Explicitly set fields always win:

```rust
// custom style is preserved — defaults_from_theme sees style.is_some() and skips it
let spec = ButtonSpecBuilder::new()
    .text("Save".into())
    .style(my_brand_style)
    .rect(rect)
    .defaults_from_theme(&theme)
    .build();
```

This is the only correct behaviour given the call order: the app sets fields on the builder before passing it to the high-level function, which then calls `defaults_from_theme` internally. If `defaults_from_theme` unconditionally overwrote fields, every explicit customisation would be silently discarded.

**High-level API callers never call `defaults_from_theme` directly.** It is called automatically inside every high-level context function. App code just sets the fields it cares about and passes the builder in.

**Raw API callers** must either call `defaults_from_theme` manually or set every field explicitly. Skipping both will cause `build()` to panic on the first unset field:

```rust
// themed defaults for unset fields
let spec = builder.rect(rect).defaults_from_theme(&theme).build();

// fully explicit — no theme involvement
let spec = builder.rect(rect).style(my_style).build();

// panics at build() — style is unset
let spec = builder.rect(rect).build();
```

### SpecBuilder Field Visibility

`*SpecBuilder` fields are currently `pub`. This allows ergonomic struct-literal construction and direct field reads. The trade-off: fields like `rect` and `clip_rect` — which are managed automatically by high-level context functions and should not be set by high-level callers — can be set directly with no compile-time guard.

The alternative is private fields with setter methods only (standard Rust builder pattern). This would make the "framework manages this" contract self-enforcing for `rect` and `clip_rect`; all operations are already covered by the existing setter methods.

For now, fields remain `pub` and the framework-managed setter methods (`rect`, `clip_rect`, `defaults_from_theme`) carry doc comments explaining when to call them. Those struct fields may also warrant the same doc comments directly on their field declarations for the same reason.

### Default Implementations — Spec, Style, and Builder

None of `*Spec`, `*Style`, or `*SpecBuilder` structs implement `Default`. The reasons differ by type but share a common root: multiple sources of default values creates drift and obscures intent.

**`*Spec` structs — no `Default`**

Specs are fully resolved; every field is a concrete value with no `Option<>`. A `Default` impl must invent values for fields like `rect` (which has no `Default` of its own) and `style`, producing instances that compile but render broken — silent failure instead of an explicit signal. Lifetime-parameterised specs (`MenuSpec<'a>`, `TabsSpec<'a>`, etc.) add a further constraint: they cannot implement `Default` without `'static` bounds, which would be unacceptable. The builder is the correct layer for partial state; the spec is not.

**`*Style` structs — no `Default`**

The only authoritative source of style defaults is `Theme::xxx_style()` methods. A `*Style` struct is always either caller-supplied or theme-derived; there is no meaningful style independent of the theme. Hardcoded defaults on style structs duplicate the theme, diverge silently when the theme changes, and mask missing `defaults_from_theme()` calls with plausible-looking but wrong colors.

**`*SpecBuilder` structs — `derive(Default)` + `new()` forwarding**

Because every builder field is `Option<T>`, `derive(Default)` produces exactly an all-`None` struct — identical to a hand-written `new()`. All builder structs therefore `#[derive(Default)]` and keep a `new()` constructor that forwards to `Self::default()`. This gives callers both spellings (`ButtonSpecBuilder::new()` and `ButtonSpecBuilder::default()`) with zero drift risk: there is only one source of truth.

**The asymmetry between `*Spec` and `*SpecBuilder` is intentional**

`*Spec` is fully resolved — no partial state, no `Option<>`, no defaults of any kind. `*SpecBuilder` exists precisely to hold partial state: every field is `Option<T>` and `None` means "not yet set". This distinction enables a three-stage default precedence chain:

1. **User-specified** — fields set by the caller via builder setter methods. Always win.
2. **High-level widget function default** — if a field is still `None` when the high-level function runs, it may inject a context-aware default before calling `build()`. Examples: `clip_rect` is always set here from the context's current clip; a container widget might set `disabled = true` on all children while it is loading.
3. **`build()` default or panic** — fields still `None` at `build()` time either get a context-independent default (`disabled` → `false`, `large` → `false`) via `unwrap_or`, or cause a panic with a descriptive message if no sensible default exists (`rect`, `style`).

This means defaults are applied **as late as possible**, giving higher layers the opportunity to provide sensible context-aware values rather than being silently pre-empted by a `false` baked in at construction time.

**Exception**

Purely numeric/visual specs with no required content (e.g. `MeterSpec`) may implement `Default` when a zero state has a genuinely reasonable rendering and there is no borrowed data. The bar is whether `Default::default()` produces something visually coherent and useful — not just something that compiles.

### Style Structs

Some widget types group their styling fields into a dedicated `*Style` struct embedded inside `*Spec` and `*SpecBuilder`. The decision rule:

- **Use a `*Style` struct** when the widget has interaction states (hover, press, focus, disabled) or several coordinated color/dimension roles. The style struct keeps the spec readable and lets callers pass a single `ButtonStyle` override rather than setting a dozen fields individually.
- **Embed styling fields directly in `*Spec`** when the widget is purely display-only and has only a small number (roughly ≤ 3) of styling fields. A dedicated struct would be ceremony with no benefit for these simple cases.

The practical dividing line is interaction states: as soon as a widget needs distinct visuals for hover, focus, or disabled, the coordinated color roles naturally belong in a `*Style` struct. Pure display widgets without those states may keep their styling inline.

Example:
```rust
// Low-level: fully resolved, no defaults
pub fn button(spec: ButtonSpec, input: &Input) -> raw::ButtonResult;

// High-level: uses builder to resolve defaults
pub fn button<T, S>(
    ctx: &mut WidgetContext<T, S>,
    state: &mut ButtonState,
    layout_params: S::Params,
    builder: ButtonSpecBuilder,
) -> ButtonResult {
    let rect = ctx.layout(layout_params);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .build();
    let r = raw::button(state, spec, ctx.input, ctx.focus_sys);
    ctx.append_cmds(r.draw.0);
    ButtonResult {
        layout: LayoutInfo::new(rect, r.content_bounds),
        input: r.input,
        focused: r.focused,
    }
}
```

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

- **Container Lifecycle — begin/finish**: Container widgets (`begin_scroll_area`, `begin_window`) return a child `WidgetContext` with their cleanup logic embedded as an `on_finish` closure. The caller fills the child context with widgets, then calls `child.finish()`. Commands accumulate directly into the shared buffer and cleanup runs automatically — no explicit high-level `end_*` call or manual command threading needed. The raw layer still exposes `raw::end_scroll_area(token, focus_sys)` and `raw::end_window()` for callers that bypass the context system.

- **Shared Command Buffer**: Each `WidgetContext` holds `cmds: &'a mut DrawCommands`, a mutable reference into a buffer that ultimately belongs to the root caller. Child contexts are constructed by reborrowing the parent's `cmds` reference, so all contexts in a tree write into the same buffer in evaluation order. `finish()` returns `()` — there is no `DrawCommands` to thread back up the call stack.

- **`on_finish` in `WidgetContext`**: Every `WidgetContext` carries `on_finish: CF` where `CF: FnOnce(&mut FocusSystem) -> DrawCommands`. Root contexts use a no-op function pointer. Container widgets construct a child via `child_with_layout_and_on_finish(layout, closure)`, passing a move closure that captures the container's token. `finish()` calls the closure and appends its post-commands (e.g. `PopClip`, focus claims) into the shared buffer after the child's own accumulated commands.

- **Borrow-Enforced Ordering**: Because child contexts are created from `&mut self`, the borrow checker enforces that only one child can be alive at a time. This is the correct constraint for immediate-mode GUI: draw commands are order-sensitive (later commands render on top), so constructing two sibling children simultaneously and finishing them in arbitrary order would be a footgun. The exclusive borrow makes incorrect ordering a compile error, not a runtime bug. An alternative design — separate owned buffers per context with a raw back-pointer for auto-append on `finish()` — is mechanically possible but loses this guarantee.

- **`ScrollAreaToken` — Dumb State Holder**: `begin_scroll_area` internally produces a `ScrollAreaToken`, a plain struct with private fields holding the scroll area's `FocusId` and scroll-limit flags needed to make focus claims at cleanup time. It has no `Drop` impl and no `finish()` method. The high-level `begin_scroll_area` captures this token in a move closure stored on the child `WidgetContext`; the raw API passes the token explicitly to `raw::end_scroll_area`. This design was chosen over a RAII-style `Drop` cleanup because `Drop` cannot receive `&mut FocusSystem` — borrowing it for the token's full lifetime would make the widget API impractical.

- **Why `finish()` vs. explicit `end_scroll_area` — two API layers, two contracts**:

  | | High-level (`WidgetContext::finish()`) | Low-level (`raw::end_scroll_area(token, focus_sys)`) |
  |---|---|---|
  | **Who calls it** | App code using the context API | Widget authors writing raw widget functions |
  | **What it knows** | Nothing about scroll areas specifically — just "run cleanup and close this scope" | Exactly what scroll area cleanup requires: pop clip, pop keyboard scope, make focus claims |
  | **How cleanup is delivered** | Via the `on_finish` closure captured at `begin_scroll_area` time | Via the `ScrollAreaToken` carrying the state needed for claims |
  | **Why explicit, not RAII** | `Drop` can't receive `&mut FocusSystem`; borrowing it for the token's lifetime would pollute every widget call site | Same reason |
  | **Ordering guarantee** | Borrow checker enforces sequential children; `finish()` appends directly into the shared buffer | Caller is responsible for matching `begin`/`end` calls; `debug_assert` checks order at runtime |

  At the high level, `finish()` is a uniform teardown verb — the caller doesn't need to know whether the context wraps a scroll area, a window, or nothing. At the raw level, `end_scroll_area(token, focus_sys)` is explicit because raw callers have all the information and are expected to manage lifecycle manually.

- **Bottom-Up Scroll Claims**: To handle nested scroll areas gracefully without immediate-mode input loops, the `FocusSystem` employs a 1-frame delayed "claim" architecture. Inner scroll areas register claims (`claim_scroll_up`, `claim_pgdn`, etc.). Because contexts are finished bottom-up, innermost scroll areas always get first pick of the claim.

- **Standalone Widget Participation**: Standalone widgets like standalone sliders actively participate in this claim system (using `claim_scroll_at_ends`). When hovered or focused, they block scroll inputs from propagating up to outer scroll areas, acting as "hard stops" instead of allowing the parent to suddenly start scrolling when the slider hits its boundary.

