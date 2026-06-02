# Framewise Design

Detailed design decisions and implementation architecture for Framewise.

---

## State Storage Options

For features requiring state (like Button hover/press tracking, or TextEdit contents and caret position), Framewise offers the app two flexible approaches to integrating that state:

1. **Library-Provided Opaque State:** The app stores a library-provided struct (e.g., `ButtonState`, `TextEditState`) in its data model. The app fulfills the "app owns the state" philosophy, treating the struct opaquely and passing it mutably to the widget.
2. **App-Managed State:** The app extracts and passes values directly (such as simple values or sub-fields of its existing data structures) to widget specs, keeping synchronization simple, explicit, and direct without complex trait layers.

State structs are composed based on what that widget type can do — e.g. a `FocusState` struct might be common across many widget types. We also share logic inside widget functions: a `handle_focus` function that manipulates `FocusState` could be re-used by many widget types.

### `*State` vs `*Spec` — What Changes and Who Changes It

The distinction between the two parameter types is about **who mutates what, and when**:

- **`*State`** holds data that the widget function itself may modify as a direct result of user interaction — hover tracking, pressed flags, scroll position, text content, caret position, focus IDs. The caller passes `&mut *State`; the widget mutates it in place.
- **`*Spec`** holds everything the caller provides as input to the widget for that frame. Spec fields can vary frame-to-frame (e.g. elapsed time, a label string, an enabled flag driven by app logic), but they are **never mutated by the widget function**. The spec is consumed, not updated.

In short: if a value changes because the user clicked or typed, it belongs in `*State`. If it changes because the app decided something different this frame, it belongs in `*Spec`.

---

## Widget Consistency

All widget files must be consistent with each other. A reader browsing from one widget to another should never have to ask "why does widget X do it like this but widget Y does it like that?"

Consistency applies across every dimension of the code:

- **Naming** — struct names, field names, parameter names, local variable names, result field names
- **File structure** — ordering of structs, functions, and sections within the file
- **Derived traits** — the same set of `#[derive(...)]` on equivalent structs (e.g. all `*Spec` structs derive the same traits)
- **Visibility** — `pub`, `pub(crate)`, or private applied consistently to equivalent items
- **Parameters** — order of parameters to raw functions and high-level context functions, including where `&Input`, `&mut *State`, and `*Spec` appear
- **Return types** — if one widget's high-level function returns `layout: LayoutInfo`, all do; if one raw result includes `content_bounds`, equivalent raw results do too
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

The split is captured in one line: a **`LayoutSpace`** says *"what space do I have to work with"*; a **`Layout`** says *"and how I want to fill it"*. The two are handed in separately and combined by `Layout::begin(space)` — which is why `WidgetContext::root` and `begin_scroll_area` take a `Layout` plus a `LayoutSpace` rather than a pre-begun state: the caller states intent, the framework wires up the geometry.

We define two traits:

1. **`Layout`**: The user-facing configuration (e.g., `ColumnLayout { spacing: 4.0 }`). It dictates the `Params` required to position a widget and provides a `begin(space: impl Into<LayoutSpace>)` method to instantiate the layout's state. A plain `Rect` is a fully-bounded space (`From<Rect>`), so the common `begin(some_rect)` call is unchanged; an axis only goes unbounded when a caller hands down a `LayoutSpace` that says so (see [Unbounded Axes](#unbounded-axes)).
2. **`LayoutState`**: The mutable engine that lives inside the `WidgetContext`. It accumulates positions as widgets are added.

The layout call is `layout(params: S::Params, intrinsic: IntrinsicSize) -> Rect`. It merges three inputs: the caller's `params` (intent — fixed/auto/fill), the widget's `intrinsic` measurement (reported by a `calc_*` companion, see [Intrinsic Sizing](#intrinsic-sizing)), and the layout's own state (available space + cursor). Layouts that don't size from content (`ManualLayout`) ignore `intrinsic`; intrinsic-aware layouts (column/row/wrap) read it. There is still **no separate measuring pass over a retained tree** — the only extra work is the cheap, explicit `calc_*` spec measurement.

### Built-in Layouts

- **`ManualLayout`**: `Params = Rect`. Explicit layout where the app specifies exact rectangles; ignores `intrinsic`. If nested (e.g. inside a scroll view), it treats its bounding box's `top_left` as an offset, so explicit rectangles are correctly shifted relative to their parent. This is also the sanctioned way to place a *high-level* widget at an explicit rect (the rect is the `Params`).
- **`ColumnLayout`**: `Params = SizeReq`. Stacks widgets vertically, keeping a Y-axis cursor. Cross axis (width) may `Fill` the bounds; main axis (height) is typically `Auto` (from intrinsic) or `Fixed`.
- **`RowLayout`**: `Params = SizeReq`. Stacks widgets horizontally, keeping an X-axis cursor.
- **`WrapLayout`**: `Params = SizeReq`. Flows widgets left-to-right and wraps to the next line when the next child would overflow the available width. Never wraps a child already at the start of a line; an unbounded width has no edge to overflow, so the flow stays on one line.
- **`SplitRow`**: `Params = Extent` (cross-axis height only). A *declared-structure* layout (Phase 4): it takes a `count` up front and divides its width into that many **equal** cells, `(width − spacing·(count−1)) / count` each. Each child's width is imposed (the cell), so children declare only their height. Because dividing space needs a committed far edge, `SplitRow` requires `AxisBound::Exact` width and panics on `AtMost`/`Unbounded` — the same rule that governs `Fill` and alignment. Knowing `count` is what makes the equal split one-pass (no measure-all / emit-reorder): an equal split is otherwise a future-sibling dependency, and the declaration turns it into a constant resolved from available space alone.
- **`OffsetLayout<L>`**: A decorator that shifts the inner layout's `Rect`s by a `Vec2` offset (used by scroll areas). It forwards `Params` and `intrinsic` to the inner layout. Scroll areas wrap their content layout in `OffsetLayout { offset, inner }` and push a scissor `clip_rect`.

Because `OffsetLayout` directly shifts the `Rect`s returned during the layout pass, **widgets are physically located at their scrolled screen coordinates when created**. This means standard mouse hit-testing (`rect.contains(mouse_pos)`) works natively without translating input. We only require widgets to optionally test against a `clip_rect` so that hidden, scrolled-out elements aren't accidentally clickable.

### Intrinsic Sizing

Intrinsic-aware layouts let a widget be sized from its own content without abandoning the top-down, one-pass model.

- **`IntrinsicSize`** — a measurement-only value (`min` / `preferred` / `max`, each an `Option<Vec2>`) reported *up* by a widget. It is content + style derived, **never policy**: "fill", "grow", and weights are caller intent and live in the layout's `Params`, not here.
- **`SizeReq { width: Extent, height: Extent }`** — the caller's per-axis intent handed *down* to a layout. `Extent` is `Fixed(px)`, `Auto` (use the intrinsic preferred size on that axis), or `Fill` (span the layout's available extent on that axis). Axes are absolute (width/height), not main/cross, so the same request reads identically regardless of orientation. `From<Vec2>` treats a plain size as fixed on both axes.
- **`LAYOUT_FALLBACK_SIZE`** — a library-global size an intrinsic-aware layout falls back to when it needs a measurement that was never reported (e.g. `Auto` against a widget that returns no `preferred`). Deliberately large and obvious so missing measurements surface during development.

### Three-State Axis Bounds & Unbounded Axes

The space a parent hands down is a `LayoutSpace { x, y, width: AxisBound, height: AxisBound }`, where `AxisBound` represents the parent's layout knowledge:

* **`AxisBound::Exact(f32)`** — "You live in a box of exactly this size". This acts as both a hard limit and a committed coordinate anchor, permitting positioning, filling, centering, and right-alignment.
* **`AxisBound::AtMost(f32)`** — "Choose your own size, but do not exceed this maximum". This is a ceiling without a committed far edge. Only measurement and shrink-wrap decisions are permitted.
* **`AxisBound::Unbounded`** — "No ceiling on this axis". This is typically used inside scroll views, allowing content to grow naturally to its preferred size.

Position is always concrete — a layout always knows *where* a child starts — so only the *extent* can be constrained or unbounded. A fully-specified `Rect` converts automatically via `From<Rect>` to a fully `Exact` space, so layouts without dynamic constraints never see `AtMost` or `Unbounded` axes.

#### The Unifying Rule of Alignment and Distribution

> **Position and distribution policies — fill, right-align, center, space-between — require `AxisBound::Exact`: a committed frame with a far edge. `AtMost` and `Unbounded` bounds permit only measurement / shrink-wrap decisions.**

If a layout (such as `ColumnLayout` or `RowLayout`) is configured with a cross-axis alignment of `Center` or `End`, it will **panic** if:
1. The cross-axis boundary is `AtMost` or `Unbounded`. This prevents alignment math from running against a boundary that was only ever intended as a maximum ceiling or scroll container extent.
2. The aligned object is a deferred container (such as a `Frame`) and has a dynamic size (`Extent::Auto`). Because deferred layouts position and draw their children immediately during the layout pass, the aligned container's size must be resolved upfront during `begin_layout`. If the size is dynamic (`Auto`), it is mathematically impossible to calculate the correct alignment offset upfront, and any attempt to do so will trigger an immediate panic in `begin_layout`.

Similarly, `WrapLayout` does not support deferred containers with `Extent::Auto` widths because line wrapping decisions must be resolved upfront in `begin_layout`. Placing a dynamic-width deferred container in `WrapLayout` will trigger a panic in `begin_layout`.

To align or wrap a nested container safely, it must have a concrete size resolved upfront (e.g. `Extent::Fixed(px)`, or `Extent::Fill` against a parent of exact bounds).

#### Sizing Resolution Rules

Three key rules keep these bounds from leaking infinity into leaf widget geometry:

1. **`Fill` on non-`Exact` axes acts as `Auto`.** Filling an infinite (`Unbounded`) or unanchored (`AtMost`) axis is undefined since there is no committed extent to fill. In these cases, the layout falls back to the widget's intrinsic size (or `LAYOUT_FALLBACK_SIZE` if none is reported), matching `Extent::Auto` resolution behavior.
2. **`AtMost` caps preferred size.** Under `AxisBound::AtMost(w)`, a widget's intrinsic size resolves to `preferred.min(w)`, preventing it from overflowing the ceiling.
3. **Unbounded resolves to concrete at accumulation.** A child laid out in an unbounded axis still resolves to a fully concrete `Rect`. The layout's running cursor stays a concrete `f32`, meaning the accumulated extent remains fully bounded (which is precisely what a deferred scroll area reads as its content size). No infinity ever reaches a `Rect`.

**Reading the accumulated extent — `content_extent`.** `LayoutState` exposes `fn content_extent(&self) -> Vec2`: the total extent the layout has consumed so far, measured from its origin (so it is independent of any scroll offset, and `OffsetState` forwards its inner's value unchanged). Every layout state implements it — a column reports its widest child and stacked height, `ManualLayout` the max far-edge of placed rects, etc. `WidgetContext::finish()` reads it and hands it to the cleanup closure, which is how a deferred scroll area learns how large its children turned out (see [Scroll Areas](#scroll-areas-windows-and-symmetrical-container-life-cycles)). It returns the zero vector before any child is placed.

**The `calc_*_intrinsic_size` companion.** Each raw widget that participates gains an independent `raw::calc_*_intrinsic_size(spec, text_system) -> IntrinsicSize`. It takes the widget's `*Spec` so its signature stays stable as size-relevant fields are added (they live in the spec/style), but it **must not read `spec.rect`**: the rect is the *output* of the layout step that consumes the intrinsic size, so it isn't known yet. Structurally, `rect` is the *only* spec field that is unknowable before `calc` runs — everything else (content, style, clip, disabled) is an input. Callers therefore build the spec with `Rect::PLACEHOLDER` (NaN) before measuring; any arithmetic on it yields NaN, making accidental use loud rather than silent.

**High-level flow.** The high-level widget function: (1) resolves defaults and builds the spec with `Rect::PLACEHOLDER`; (2) calls `calc_*_intrinsic_size(&spec, …)`; (3) calls `layout(params, intrinsic)` to get the real rect; (4) assigns `spec.rect` and calls the raw function. Under `ManualLayout` the intrinsic is computed but ignored — an accepted "double-shape" cost for now (the text is shaped in both `calc` and the raw draw); a later `Layout::WANTS_INTRINSIC` const can gate it.

---

## API Shape

Framewise has two layers:

### Low-Level: Raw Widget Functions

Plain, low-level functions residing in `raw` submodules (e.g., `widgets::button::raw::button`). They are completely decoupled from `WidgetContext` and the layout system. They receive a fully resolved explicit specification struct, append draw commands directly to a caller-supplied `&mut DrawCommands` buffer, and return a `raw::*Result` containing interaction info. Every input is explicit; the cost is strictly local.

Appending directly to a caller-supplied buffer avoids intermediate `Vec` allocation and copying, and gives callers stable index-based access to the command list (which frame containers rely on for placeholder patching). The `cmds: &mut DrawCommands` parameter is always last, after all other inputs.

```rust
pub fn button<T: TextSystem>(spec: ButtonSpec, state: &mut ButtonState, input: &Input, focus_system: &mut FocusSystem, text_system: &mut T, cmds: &mut DrawCommands) -> raw::ButtonResult;
pub fn label<T: TextSystem>(spec: LabelSpec, text_system: &mut T, cmds: &mut DrawCommands) -> raw::LabelResult;
pub fn text_edit<T: TextSystem>(spec: TextEditSpec, state: &mut TextEditState, input: &Input, focus_system: &mut FocusSystem, text_system: &mut T, cmds: &mut DrawCommands) -> raw::TextEditResult;
```

Each `raw::*Result` is a concrete struct with no trait requirements on callers, no metadata maps, and no dynamic type slots. It does **not** contain a `DrawCommands` field — commands are written directly to the caller's buffer. (Result structs may derive utility traits such as `Debug` for inspection, but callers need not implement any traits to receive or use them.)

### High-Level Freestanding API: Context Integration

A unified `WidgetContext<'a, T, S, CF>` carries style parameters (theme, current text size, colors, clip rectangles, time) and system resources (mutable references `&'a mut T` to the text system and `&'a mut FocusSystem` to the focus manager). The `CF` parameter is a one-shot cleanup closure (`FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2)`) called when the context is finished; it receives the shared command buffer and the layout's `content_extent` (from `finish()`), so container cleanup can both emit post-commands and resolve geometry from how large the children turned out. Root contexts use a no-op function pointer, container widgets embed their cleanup in a move closure (see [Scroll Areas and Windows](#scroll-areas-windows-and-symmetrical-container-life-cycles)).

High-level widget APIs are freestanding, highly ergonomic functions that accept a mutable reference to `WidgetContext` along with a simplified spec/state:

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

The high-level function maps between them: it builds the spec (with a `Rect::PLACEHOLDER`), measures the intrinsic size, resolves the real rect via `ctx.layout_state.layout(params, intrinsic)`, assigns it onto the spec, calls `raw::widget()`, pushes draw commands into the context, then constructs the `*Result` forwarding the interaction fields and adding `LayoutInfo`.

Nesting a child layout is done with `ctx.child_with_layout(placement, inner_layout)`: it resolves `placement` against the *current* layout to get the child's bounds, begins `inner_layout` at those bounds, and returns a child `WidgetContext`. (Container widgets that compute their own bounds — scroll areas, windows — instead use the `child_with_layout_and_on_finish[_and_clip_rect]` variants, which take an already-begun layout state plus a self-derived clip.)

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
> **Builder Construction Rule:** All `*SpecBuilder` structs use a no-args `new()` constructor. No field is singled out as a required constructor parameter — **every field, including bool flags like `disabled` and `large`, is `Option<T>`** and starts as `None`. `build()` applies defaults for fields that have an obvious, context-independent value (e.g. `disabled` → `unwrap_or(false)`) and panics with a clear message for fields with no sensible default; the message names the missing field and points to the fix (e.g. *"style not set — call .style() or defaults_from_theme()"*). Making every field `Option<T>` is essential: `None` means "the user did not set this", which lets both `defaults_from_theme` and the high-level widget function inject context-aware defaults — something impossible if bools silently default to `false` in `new()`.

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

**The `rect` exception.** Fields set by the user on the builder are honored — the high-level widget functions will not overwrite them. The **only** exception is `rect`, which is always determined by the layout system; any user-provided value on the builder is ignored by the high-level path. (Internally the high-level function overwrites it: it builds the spec with `Rect::PLACEHOLDER`, measures the intrinsic size, then assigns the layout-resolved rect.) If explicit placement is wanted, use `ManualLayout` with the high-level functions — its `Params` *is* the rect — or drop to the low-level `raw::` function and set `rect` on the spec directly.

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

`*SpecBuilder` fields are currently `pub`. This allows ergonomic struct-literal construction and direct field reads. The trade-off: fields like `rect` and `clip_rect` — which are managed automatically by high-level context functions and should not be set by high-level callers — can be set directly with no compile-time guard. (For `rect` this is harmless, since the high-level path ignores any builder-set value and resolves the rect from the layout regardless — see "The `rect` exception" above.)

The alternative is private fields with setter methods only (standard Rust builder pattern). This would make the "framework manages this" contract self-enforcing for `rect` and `clip_rect`; all operations are already covered by the existing setter methods.

For now, fields remain `pub` and the framework-managed setter methods (`rect`, `clip_rect`, `defaults_from_theme`) carry doc comments explaining when to call them. Those struct fields may also warrant the same doc comments directly on their field declarations for the same reason.

### Default Implementations — Spec, Style, and Builder

None of `*Spec`, `*Style`, or `*SpecBuilder` structs implement `Default`. The reasons differ by type but share a common root: multiple sources of default values creates drift and obscures intent.

**`*Spec` structs — no `Default`**

Specs are fully resolved; every field is a concrete value with no `Option<>`. A `Default` impl must invent values for fields like `rect` (which has no `Default` of its own) and `style`, producing instances that compile but render broken — silent failure instead of an explicit signal. Lifetime-parameterised specs (`MenuSpec<'a>`, `TabsSpec<'a>`, etc.) add a further constraint: they cannot implement `Default` without `'static` bounds, which would be unacceptable. The builder is the correct layer for partial state; the spec is not.

**`*Style` structs — no `Default`**

The only authoritative source of style defaults is the `*Style::from_theme()` (or `*Style::*_from_theme()` for multi-variant styles) methods defined directly on each style struct. A `*Style` struct is always either caller-supplied or theme-derived; there is no meaningful style independent of the theme. Hardcoded defaults on style structs duplicate the theme, diverge silently when the theme changes, and mask missing `defaults_from_theme()` calls with plausible-looking but wrong colors.

**`*SpecBuilder` structs — `derive(Default)` + `new()` forwarding**

Because every builder field is `Option<T>`, `derive(Default)` produces exactly an all-`None` struct — identical to a hand-written `new()`. All builder structs therefore `#[derive(Default)]` and keep a `new()` constructor that forwards to `Self::default()`. This gives callers both spellings (`ButtonSpecBuilder::new()` and `ButtonSpecBuilder::default()`) with zero drift risk: there is only one source of truth.

**When a `*Spec` field is itself `Option<T>`, the builder field is `Option<Option<T>>`**

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

**The asymmetry between `*Spec` and `*SpecBuilder` is intentional**

`*Spec` is fully resolved — no partial state, no `Option<>`, no defaults of any kind. `*SpecBuilder` exists precisely to hold partial state: every field is `Option<T>` and `None` means "not yet set". This distinction enables a three-stage default precedence chain:

1. **User-specified** — fields set by the caller via builder setter methods. Always win.
2. **High-level widget function default** — if a field is still `None` when the high-level function runs, it may inject a context-aware default before calling `build()`. Examples: `clip_rect` is always set here from the context's current clip; a container widget might set `disabled = true` on all children while it is loading.
3. **`build()` default or panic** — fields still `None` at `build()` time either get a context-independent default (`disabled` → `false`, `large` → `false`) via `unwrap_or`, or cause a panic with a descriptive message if no sensible default exists (`rect`, `style`).

This means defaults are applied **as late as possible**, giving higher layers the opportunity to provide sensible context-aware values rather than being silently pre-empted by a `false` baked in at construction time.

### Style Structs

Some widget types group their styling fields into a dedicated `*Style` struct embedded inside `*Spec` and `*SpecBuilder`. The decision rule:

- **Use a `*Style` struct** when the widget has interaction states (hover, press, focus, disabled) or several coordinated color/dimension roles. The style struct keeps the spec readable and lets callers pass a single `ButtonStyle` override rather than setting a dozen fields individually.
- **Embed styling fields directly in `*Spec`** when the widget is purely display-only and has only a small number (roughly ≤ 3) of styling fields. A dedicated struct would be ceremony with no benefit for these simple cases.

The practical dividing line is interaction states: as soon as a widget needs distinct visuals for hover, focus, or disabled, the coordinated color roles naturally belong in a `*Style` struct. Pure display widgets without those states may keep their styling inline.

Example:
```rust
// Low-level: fully resolved, no defaults
pub fn button<T: TextSystem>(spec: ButtonSpec, state: &mut ButtonState, input: &Input, focus_system: &mut FocusSystem, text_system: &mut T) -> raw::ButtonResult;

// High-level: uses builder to resolve defaults
pub fn button<T, S, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ButtonSpecBuilder,
    layout_params: S::Params,
    state: &mut ButtonState,
) -> ButtonResult {
    let rect = ctx.layout(layout_params);
    let spec = builder
        .rect(rect)
        .defaults_from_theme(&ctx.theme)
        .build();
    let r = raw::button(spec, state, ctx.input, ctx.focus_system, ctx.text_system, ctx.cmds);
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
- **`LayoutState`** is the mutable engine: `layout(params, intrinsic) -> Rect` for normal widgets, `begin_layout` / `end_layout` for fit-to-children containers, and `resolve_space() -> Rect` so scroll areas and `finish()` can read the accumulated content **resolved against the layout's own `LayoutSpace` bounds** (an `Exact` axis reports the exact extent, `AtMost` caps the measured size, `Unbounded` shrink-wraps to it).

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

- **Fit-to-Children Containers (Opt-in Sizing)**: Container widgets (such as `frame`) can choose to **opt in** to discovering their children's bounds to dynamically size themselves bottom-up. Standard single-pass leaf widgets call `layout(params, intrinsic)` to obtain their concrete bounds in one go. In contrast, container widgets that want to fit to their content size opt into the deferred layout pattern using a compile-safe token-borrow model:
  - **The Opt-In Pattern**:
    1. The container calls `begin_layout(layout_params) -> (LayoutSpace, LayoutToken<'a>)` instead of `layout()`. This mutably borrows the parent `LayoutState` for the lifetime of the returned `LayoutToken`, preventing any sibling layout calls from being made on the parent context while the token lives (statically borrow-enforcing the evaluation sequence).
    2. The container inspects the generic `LayoutSpace` bounds (`AxisBound`) to make its own sizing policy decisions: if an axis is `Unbounded` or `AtMost`, the container will size itself bottom-up to its children; if it is `Exact(w)`, it honors the parent's rigid constraints. It subtracts padding/borders via `space.inset(amount)` to yield the available child space.
    3. The container creates a child `WidgetContext` with a custom `on_finish` closure, capturing the `LayoutToken` by value.
    4. Sibling widgets are laid out sequentially within the child context. When the child context is finished, `finish()` automatically queries the child layout state for its `resolve_space()` (the accumulated content resolved against the layout's bounds) and passes it to the `on_finish` closure.
    5. Inside the closure, the container consumes the token by calling `token.end_layout(children_extent)`. This resolves the container's final size and visual alignment inside the parent, advances the parent layout cursor, and releases the parent borrow, unlocking the parent context for subsequent sibling widgets.
  - This design decouples the container from concrete layout systems (like `ColumnLayout` or `RowLayout`) and concrete layout parameters (like `SizeReq` or `Rect`), as all sizing policies are decided solely via generic `LayoutSpace` bounds and completed via `LayoutToken`.

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

