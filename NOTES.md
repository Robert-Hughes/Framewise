# Framewise Notes

Working notes, TODOs, open questions, and half-baked ideas.


---

* ROB commented out the background of frame - Z order is wrong!

* Add note to DESIGN.md about frame having a high-level begin/end, but at low-level it's just a single frame(). Not sure if this pattern will be common for containers?
Not sure if this pattern is even correct, e.g. do we need to PushClip in begin? (even if bottom/right can't be clipped due to unbounded, top/left should be (e.g. if padding))

* Moving all raw widgets to take &mut DrawCommands and append in-place is a superior architectural model. Like raw::begin_frame already does. This completely avoids intermediate vector allocation and copying. Perfect Index Stability & Encapsulation. Direct Viewport Background Push. Update DESIGN.md

* Figure out what the new frame demo page is trying to show and see if it's actually working like we wanted!

* For 'container' widgets with the new begin/end thing like frame():
  - Will they still have a calc_natural_size? How would this be used? It's still semantically useful (e.g. a min size), but maybe not used in practice yet?
  - How do their begin() fns (high/low) handle rect - they take a LayoutSpace instead?

* Should "child_with_layout" (i.e. layout without a widget) use begin/end_layout, so that they can auto-size? Does that make any sense? Currently, layouts report a intrinsic size of None, so can't be used with Auto (goes to fallback size). Opt-in? How to get old behaviour? begin/end Should be equivalent to just regular layout() if done right?

* Review table of supported layouts from the LAYOUT CHANGES doc and make sure we test/demo them all. We probably want to add some cases that are enabled as a result of phase 5 & 6,
as these weren't in the original table.

* Finish implementing layout changes plan (all 6 phases), and review code against it!
  - Check DESIGN.md has been updated accordingly
  - check if anything left in the LAYOUT CHANGES file worth preserving, then can delete

* Do we want an option for the scroll_areas content size to be provided up-front by the user, rather than automatically having Unbounded?
 Previously the user provided the content_size, so would be nice to retain this option
 Perhaps the user provides the internal LayoutSpace, not hardcoded in scrollbar? This allows them more control, and allows them to use alignment within the scroll area (otherwise Unbounded = panic!).

* Panicking for alignment requests that can't be satisfied is bad. Similar question to how to handle the FALLBACK layout thing.

* TextSystem improvements - single- and multi-line wrapping, newlines in string, width and height provided (always known and finite?), auto-ellipses

* Scrollbars that aren't needed should be drawn in disabled state - blend in, no focus/interaction

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

- ** Layout alignment **
  We have alignment field on some layouts, but this is fixed for the whole layout. What if user wants to place individual widgets with different alignments? Maybe an override?

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

---

## Feature Checklist

Features to design and implement, roughly in dependency order:

- [ ] `LayoutInfo`, `InputInfo`, `ValueInfo<T>`
- [ ] `WidgetResult` trait
- [ ] Hit-testing and pointer input
- [ ] Buttons and toggles
- [ ] Labels and text measurement
  * All the nice text rendering things like kerning, compositing etc. Text should look great, as good as native OS stuff.
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





Misc
====

 * Consider using crate features to include/exclude certain widget types. Or perhaps move 'non-core' widgets into separate 'extra widgets' crate(s)?