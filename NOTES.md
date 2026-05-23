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

---

## Feature Checklist

Features to design and implement, roughly in dependency order:

- [ ] Geometry types: `Rect`, `Color`, `Align`
- [ ] `DrawCmd` and `DrawCommands`
- [ ] `LayoutInfo`, `InputInfo`, `ValueInfo<T>`
- [ ] `WidgetResult` trait and `Builder::emit`
- [ ] Hit-testing and pointer input
- [ ] Buttons and toggles
- [ ] Labels and text measurement
- [x] Input focus model
- [ ] Scrolling and scroll regions
- [ ] Splitters and drag handles
- [ ] Text editing (`TextEditState`)
- [ ] Grid and table layouts
- [ ] Clipping and layering
- [ ] Popups, menus, tooltips
- [ ] Drag and drop
- [ ] Accessibility and tab order
- [ ] IME support
- [ ] Dialogs (blocking and non-blocking)
- [ ] Tabs
- [ ] Graphics / images
- [ ] Animations (spinners, progress bars, animated scrolling)
- [ ] Window min/max sizing based on layout

---

## Scrolling — Open Questions & Ideas

We've been working a lot on scrolling behaviour and propagation recently, added a bunch of
tests. Things to still review or think about:

**Review tasks:**
- Check if there's anything we might have missed — incorrect or confusing behaviour, or
  inconsistencies for the user.
- Check the design/implementation of scrolling logic — can it be simplified?
- Check test coverage — are we missing anything? Can the tests be simplified?

**Specific behaviours to address:**
- Click and hold to repeatedly page down on a slider — if it gets clamped at the end it
  can jump back and forth every frame!
- Slider: click on the trackbar then drag should snap to cursor.
- Middle-click hold-and-drag pan; middle-click without holding.
- Click-and-drag pan (touch / mobile).
- "Flinging" (momentum scrolling).
- Arrow key scrolling — should work if slider is focused at the very least (somewhat
  working, not for horizontal). Possibly also when an inner widget is focused? Not sure if
  arrow keys should be used for focus-swapping too.
- Home / End key when a child widget is focused?

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

- Up / down / left / right for switching focus, as well as Tab?
- Tabbing to a widget that's inside a scroll area (possibly nested) should scroll to make
  it visible.
