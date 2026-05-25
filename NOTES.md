# Framewise Notes

Working notes, TODOs, open questions, and half-baked ideas.


---

## Things Still to Figure Out

- **Hit-testing with overlapping widgets** — if a widget drawn later (higher in the visual
  stack) overlaps one drawn earlier, the earlier widget's hit region may still be tested
  first, since it was registered first. We need a clear rule for how draw order, z-order,
  and hit-test priority interact.

- **Clipping** — for the most part, explicit clipping is rarely needed, as UIs generally
  shouldn't have overflowing content. The primary exception is scroll containers, which
  will require some mechanism for scissor rects or clipping regions in the renderer.

- **`LayoutInfo` bounds redundancy** — is there any point in `LayoutInfo` returning the
  overall bounds, given that's always passed in directly when calling a widget function?

- **Off-screen draw cost** — currently things can be drawn "off screen" or hidden/clipped
  and might still contribute cost. We should check this.

- **Scroll areas and virtualisation** — the app chooses how many widgets to put inside
  scroll areas, so can choose real vs. virtual. Is this the right approach? How does it
  align with the "no opt-in virtualised list" anti-pattern?

- **Text cache miss attribution** — if a widget was "unlucky" and was the one that had the
  glyph atlas cache miss, that might be awkward to figure out in profiling. Will see how
  this plays out in practice.

* How do custom widgets (i.e. functions) get used in a builder? See builder::custom() and builder::add()
* Consider renaming ButtonInfo (WidgetInfo) to ButtonResult/WidgetResult? for clarity?
* Figure out relationship between (Info, Spec, Style, Result, etc.) structs, Theme, Font etc
* Tried something with WidgetSpec and WidgetSpecBuilder - is this good? Apply to all widget kinds? Move this trait to the builder-level, as is only relevant for that?
* Is the "optional" text-system a good design feature - it does does create some awkwardness?
* WidgetSpecBuilders should ideally get required fields up-front, so can't panic later on missing values? What about TextSystem etc. though, they're only added later by the builder? Maybe we need a "outside-builder"-only spec struct?

* Should widgets be returning a LayoutInfo with their bounds, when this is one of the thigns that we always(?) pass in? i.e. just copied out.
Is useful when using builder cos the rect is calculated by the layout, so then maybe the bounds should be returned at hte builder level, not hte widget function level?

//TODO: should the spec traits actually be part of the builder API, as that's the only thing that actually requires a consistent shape.
Also having themes here might be inconsistent as they're supposed to be a high level concept!


* Should the returned content_bounds be screen space or relative to something? If screen-space, are they useful for much?

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
    * Remove all uses of ".emit()" and ".append_cmds()" ideally
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