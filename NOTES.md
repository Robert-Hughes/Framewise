# Framewise Notes

Working notes, TODOs, open questions, and half-baked ideas.


---

* Try to add tests for the WidgetContext nesting and scopes, to make sure things are finished in the right order, we don't miss scopes etc.
* DESIGN.md doesn't mention the current implementation's automatic scope finishing in WidgetContext::finish() vs the design document's explicit end_* function pattern. Need to figure out what we want to do here.
* SpecBuilders could have explicit things set on them like colours, but then the high-level widget funcs would override this from the theme. The theme should be a fallback? Make this explicit in DESIGN.md and check the code. Add tests for fallback?
* In sample app, we're setting a lot of stuff on the spec builder that should be coming automatically from the WidgetContext! (e.g. time, style)
* Figure out relationship between (Info, Spec, Style, Result, etc.) structs, Theme, Font etc.
* Should WidgetContext have things like button style, bg_color etc. on it? Should this be in the theme?
* Check Widget types are consistent about theme application in high-level function etc., make sure DESIGN.md is clear on this
* Some widgets embed styling directly into their *Spec struct instead of having a separate *Style struct (e.g., ColorSwatch, Divider, Keycap, Label, Meter). This is a minor variation but still consistent with the overall *Spec and *SpecBuilder pattern. If acceptable, not in DESIGN.md.
* WidgetSpecBuilders should ideally get required fields up-front, so can't panic later on missing values? What about TextSystem etc. though, they're only added later by the high-level widget API from the widget context? Maybe we need a "outside-builder"-only spec struct?
* Add note to DESIGN.md that Spec and SpecBuilders shouldn't include 'systems' like input, focus etc. Just basic parameters. It should ideally be a 'value-type' with no external references.

* FIgure out if clip_rects are being handled properly. SHould these be associated with scopes, WidgetContexts etc? Seems to be too much manual handling atm.

* Add "reset" button to spec page, to reset all state
* Indicate which widgets have deliberately fake state (no input/focus), so user doesn't get confused. Not sure how though, as we don't want to affect the styling (as that's exactly what we're trying to show!)
* Go through the spec_page, check/implement/test each widget/aspect to make better match the mock-up and add interactivity as we go

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
* Should the returned content_bounds be screen space or relative to something? If screen-space, are they useful for much?

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