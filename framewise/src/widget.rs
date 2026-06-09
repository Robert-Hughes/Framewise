use crate::draw::DrawCommands;
use crate::focus::FocusSystem;
use crate::layout::{
    IntrinsicSize, Layout, LayoutSpace, LayoutState, LayoutToken, LayoutViolation,
    SpacerLayoutState,
};
use crate::theme::Theme;
use crate::types::{ClipRect, Rect, Vec2};
use crate::Input;
use crate::TextSystem;

// ── Common result fragments ───────────────────────────────────────────────────

/// Resolved geometry returned by every widget.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutInfo {
    /// The outer bounding box of the widget including any border / padding.
    pub bounds: Rect,
    /// The inner content area (inside any padding).
    pub content_bounds: Rect,
}

impl LayoutInfo {
    pub fn new(bounds: Rect, content_bounds: Rect) -> Self {
        Self {
            bounds,
            content_bounds,
        }
    }

    /// Convenience: layout with identical outer and content bounds.
    pub fn tight(bounds: Rect) -> Self {
        Self {
            bounds,
            content_bounds: bounds,
        }
    }
}

/// Pointer interaction state returned by interactive widgets.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct InputInfo {
    /// True while the cursor is over the widget's bounds this frame.
    pub hovered: bool,
    /// True while the primary mouse button is held and the cursor is over the widget.
    pub pressed: bool,
    /// True on the single frame the primary button was released over the widget.
    pub clicked: bool,
}

// ── Layout Violation Policy ───────────────────────────────────────────────────

/// Policy for handling layout violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayoutViolationPolicy {
    /// Panic immediately on layout violation (matches standard behavior).
    #[default]
    Panic,
    /// Highlight the fallback geometry by drawing a red stroke outline of width 2
    /// and continue execution without panicking.
    Highlight,
}

/// React to a layout violation according to the specified policy.
///
/// `Panic` rethrows the violation message; `Highlight` draws a red outline over the
/// fallback geometry and labels the violation message in red at its top-left corner.
/// Every reaction path has a `TextSystem` in reach (the immediate `layout()`, and the
/// deferred `begin_layout`/`end_layout` reactions, which receive it through the
/// `on_finish` closure), so the label is always drawn.
pub fn react_layout_violation<T: TextSystem>(
    policy: LayoutViolationPolicy,
    text_system: &mut T,
    cmds: &mut DrawCommands,
    font: crate::text::FontId,
    violation: LayoutViolation,
    fallback_rect: Rect,
) {
    match policy {
        // violation's Display already carries the "Layout panic: …" prefix.
        LayoutViolationPolicy::Panic => panic!("{}", violation),
        LayoutViolationPolicy::Highlight => {
            let color = crate::types::Color::from_srgb_u8(255, 0, 0, 255);
            cmds.push(crate::draw::DrawCmd::StrokeRect {
                rect: fallback_rect,
                color,
                width: 2.0,
            });
            // Label at the top-left corner, in the same red.
            let layout = text_system.prepare(
                &violation.to_string(),
                crate::text::TextStyle::new(font, 12.0, 400, crate::text::TextFlow::single_line()),
                fallback_rect,
            );
            cmds.push(crate::draw::DrawCmd::Text {
                rect: Rect::new(
                    fallback_rect.x,
                    fallback_rect.y,
                    layout.metrics.logical_size.x,
                    layout.metrics.logical_size.y,
                ),
                color,
                handle: layout.handle,
            });
        }
    }
}

// ── WidgetContext ───────────────────────────────────────────────────────────

/// Context struct providing theme, input, focus, text system, and draw command
/// accumulation for high-level widget functions. This replaces the old `Builder`
/// pattern with freestanding functions.
/// Can be associated with a 'on_finish' closure, which allows widget cleanup code to be run
/// when this context is finished (e.g. for nested windows)
#[must_use = "finish() must be called to run cleanup"]
pub struct WidgetContext<'a, T: TextSystem, LS: LayoutState, CF> {
    // Styling & environment fields (formerly BuilderCtx)
    pub theme: Theme,
    pub time: f64,
    pub clip_rect: ClipRect,

    /// Layout-debug overlay. When set, [`finish`](WidgetContext::finish) strokes a
    /// magenta outline around this context's resolved layout space. Inherited by
    /// every child context, so enabling it on the root lights up the whole tree.
    pub debug_layout: bool,

    // System resources
    pub text_system: &'a mut T,
    pub focus_system: &'a mut FocusSystem,
    pub input: &'a Input,
    pub cmds: &'a mut DrawCommands,

    layout_state: LS,
    pub layout_policy: LayoutViolationPolicy,
    pending_violation: Option<LayoutViolation>,
    pub on_finish: CF,
}

impl<'a, T: TextSystem, LS: LayoutState>
    WidgetContext<'a, T, LS, fn(&mut FocusSystem, &mut T, &mut DrawCommands, Rect)>
{
    /// Creates a root `WidgetContext`.
    ///
    /// Creating a root context is a top-level entry point function that does not depend on any
    /// existing `WidgetContext` instance. This generic implementation is not tied to any concrete
    /// state, but instead resolves dynamically to any layout state `LS` via the generic layout constraint.
    #[allow(clippy::type_complexity)]
    pub fn root<L: crate::layout::Layout<State = LS>>(
        theme: Theme,
        text_system: &'a mut T,
        focus_system: &'a mut FocusSystem,
        input: &'a Input,
        layout: L,
        space: impl Into<crate::layout::LayoutSpace>,
        cmds: &'a mut DrawCommands,
    ) -> Self {
        WidgetContext {
            time: 0.0,
            clip_rect: None,
            debug_layout: false,
            theme,
            text_system,
            focus_system,
            input,
            layout_state: layout.begin(space.into()),
            layout_policy: LayoutViolationPolicy::default(),
            pending_violation: None,
            cmds,
            on_finish: |_, _, _, _| (), // No cleanup for root context
        }
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF> WidgetContext<'a, T, LS, CF> {
    pub fn child_with_layout_and_on_finish_and_clip_rect<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect),
    >(
        &'c mut self,
        inner_layout_state: LS2,
        inner_on_finish: CF2,
        inner_clip_rect: Option<Rect>,
    ) -> WidgetContext<'c, T, LS2, CF2> {
        WidgetContext {
            theme: self.theme,
            time: self.time,
            clip_rect: inner_clip_rect,
            debug_layout: self.debug_layout,
            text_system: self.text_system,
            focus_system: self.focus_system,
            input: self.input,
            layout_state: inner_layout_state,
            layout_policy: self.layout_policy,
            pending_violation: None,
            cmds: self.cmds,
            on_finish: inner_on_finish, // The original on_finish is not copied - correct as the original context will still own it.
        }
    }

    pub fn child_with_layout_and_on_finish<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect),
    >(
        &'c mut self,
        inner_layout_state: LS2,
        inner_on_finish: CF2,
    ) -> WidgetContext<'c, T, LS2, CF2> {
        self.child_with_layout_and_on_finish_and_clip_rect(
            inner_layout_state,
            inner_on_finish,
            self.clip_rect, // Clip rect is inherited by default
        )
    }

    /// Open a child context whose layout is placed within this context's layout.
    ///
    /// `placement` is resolved against *this* context's layout to obtain the child's
    /// space, then `inner_layout` is begun in that space. This is the standard path
    /// for nesting layouts (column inside row, etc.) and replaces the old
    /// `layout(params)` + `inner.begin(bounds)` + `child_with_layout(state)` dance.
    ///
    /// ### Why this is deferred (begin/end), not eager (`layout`)
    /// A nested layout is itself a container, so its final size may depend on its
    /// children (e.g. a column placed with `Extent::Auto` height should be as tall as
    /// its rows). We therefore go through [`begin_layout`](LayoutState::begin_layout):
    /// the child is begun in the *provisional* [`LayoutSpace`] (which faithfully carries
    /// `AtMost`/`Unbounded` bounds rather than a flattened `Exact` rect), and the
    /// parent's cursor is only advanced in `on_finish`, once the child's measured content
    /// extent is known via [`end_layout`](LayoutState::end_layout).
    ///
    /// When `placement` resolves to exact bounds (`Extent::Fixed`, or a `ManualLayout`
    /// rect) this is equivalent to the old eager `layout()` call — `end_layout` ignores
    /// the measured extent and returns the same rect — so existing fixed-size nesting is
    /// unchanged. Only `Auto`/`Fill`-under-non-exact slots — which would otherwise panic
    /// for lack of an intrinsic measurement — now fit to their children's content.
    pub fn child_with_layout<'c, L2: Layout>(
        &'c mut self,
        placement: LS::Params,
        inner_layout: L2,
    ) -> WidgetContext<
        'c,
        T,
        L2::State,
        impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'c,
    > {
        // A bare nested layout has no chrome: the inner layout fills the provisional
        // space as-is, and its outer extent is exactly its measured content.
        let policy = self.layout_policy;
        let font = self.theme.sans_font;
        let (child, _outer_space) = self.child_with_deferred_layout(
            placement,
            IntrinsicSize::UNKNOWN,
            inner_layout,
            |_cmds, outer| ((), outer),
            move |(), token, content, _focus, text_system, cmds| {
                let (rect, violation) = token
                    .end_layout(Vec2::new(content.w, content.h))
                    .into_parts();
                if let Some(v) = violation {
                    react_layout_violation(policy, text_system, cmds, font, v, rect);
                }
            },
        );
        child
    }

    /// Deferred-layout borrow harness shared by every fit-to-children container
    /// (`child_with_layout`, `frame`, …).
    ///
    /// [`begin_layout`](LayoutState::begin_layout) is called *inside* this method, so the
    /// [`LayoutToken`] it produces never crosses a `&mut self` boundary — that is what lets
    /// a single reusable constructor own the borrow-splitting that each container would
    /// otherwise hand-roll. The harness knows nothing about chrome (padding, borders,
    /// scroll offsets); the caller injects all space/rect math through two closures:
    ///
    /// - `before_children` runs *between* `begin_layout` and constructing the child. It
    ///   receives the draw-command buffer and the provisional outer [`LayoutSpace`], and
    ///   returns `(carried, inner_space)`: `carried` is handed to `after_children`, and
    ///   `inner_space` is the space the inner layout begins in.
    /// - `after_children` runs at [`finish`](WidgetContext::finish), once the child's
    ///   content rect is known. It receives `carried`, the [`LayoutToken`], the measured
    ///   content rect, and the focus/draw systems. The caller decides the outer extent by
    ///   calling [`LayoutToken::end_layout`] and performs any retroactive draw patching.
    ///
    /// Returns the child context plus the provisional outer [`LayoutSpace`] (containers like
    /// `frame` need it for their `LayoutInfo`; bare callers ignore it).
    #[allow(clippy::type_complexity)]
    pub fn child_with_deferred_layout<'c, L2, U, Before, After>(
        &'c mut self,
        placement: LS::Params,
        intrinsic: IntrinsicSize,
        inner_layout: L2,
        before_children: Before,
        after_children: After,
    ) -> (
        WidgetContext<
            'c,
            T,
            L2::State,
            impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'c,
        >,
        LayoutSpace,
    )
    where
        L2: Layout,
        Before: FnOnce(&mut DrawCommands, LayoutSpace) -> (U, LayoutSpace),
        After:
            FnOnce(U, LayoutToken<'c, LS>, Rect, &mut FocusSystem, &mut T, &mut DrawCommands) + 'c,
        U: 'c,
    {
        let clip = self.clip_rect; // Clip rect is inherited by default.
        let debug_layout = self.debug_layout; // Copied before the layout_state borrow below.
        let policy = self.layout_policy;

        // begin_layout runs *here*: the token stays inside this body (captured into
        // `on_finish` below) and never crosses a `&mut self` boundary, so the disjoint
        // field construction is legal and lives in exactly one place.
        let (outer_space_res, token) = self.layout_state.begin_layout(placement, intrinsic);
        // The begin_layout violation belongs to *this child's* placement; it is carried
        // into the child below and reacted at the child's finish(), where its own
        // resolved_space gives the correct fallback rect. Each child carries its own,
        // so sibling violations are never dropped.
        let (outer_space, begin_violation) = outer_space_res.into_parts();

        // Caller's "between" hook: push placeholder draw commands, decide the inner space.
        let (carried, inner_space) = before_children(self.cmds, outer_space);

        let on_finish = move |focus: &mut FocusSystem,
                              text_system: &mut T,
                              cmds: &mut DrawCommands,
                              resolved: Rect| {
            after_children(carried, token, resolved, focus, text_system, cmds);
        };

        let child = WidgetContext {
            theme: self.theme,
            time: self.time,
            clip_rect: clip,
            debug_layout,
            text_system: self.text_system,
            focus_system: self.focus_system,
            input: self.input,
            cmds: self.cmds,
            layout_state: inner_layout.begin(inner_space),
            layout_policy: policy,
            pending_violation: begin_violation,
            on_finish,
        };
        (child, outer_space)
    }

    /// Perform an immediate layout operation, routing any violations to the policy.
    pub fn layout(&mut self, layout_params: LS::Params, intrinsic: IntrinsicSize) -> Rect {
        let (rect, violation) = self
            .layout_state
            .layout(layout_params, intrinsic)
            .into_parts();
        if let Some(v) = violation {
            react_layout_violation(
                self.layout_policy,
                self.text_system,
                self.cmds,
                self.theme.sans_font,
                v,
                rect,
            );
        }

        // Draw the debug outline
        if self.debug_layout {
            self.cmds.push(crate::draw::DrawCmd::StrokeRect {
                rect,
                color: crate::types::Color::from_srgb_u8(200, 0, 255, 255),
                width: 1.0,
            });
        }

        rect
    }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, cmds: DrawCommands) {
        self.cmds.extend(cmds);
    }
}

impl<'a, T, LS, CF> WidgetContext<'a, T, LS, CF>
where
    T: TextSystem,
    LS: SpacerLayoutState,
{
    pub fn spacer(&mut self, params: impl Into<LS::SpacerParams>) {
        self.layout_state.spacer(params.into());
    }
}

impl<
        'a,
        T: TextSystem,
        LS: LayoutState,
        CF: FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect),
    > WidgetContext<'a, T, LS, CF>
{
    /// Consume the context, running the on_finish closure and appending its post-commands.
    ///
    /// The layout's resolved [`resolve_space`](LayoutState::resolve_space) is
    /// passed to the closure so container widgets (e.g. a deferred scroll area) can
    /// resolve geometry from how large their children turned out.
    pub fn finish(self) -> Rect {
        let resolved_space = self.layout_state.resolve_space();
        let debug_layout = self.debug_layout;
        let font = self.theme.sans_font;
        (self.on_finish)(
            self.focus_system,
            self.text_system,
            self.cmds,
            resolved_space,
        );

        // React to this context's own begin_layout violation (carried here so the
        // fallback rect is concrete).
        if let Some(violation) = self.pending_violation {
            react_layout_violation(
                self.layout_policy,
                self.text_system,
                self.cmds,
                font,
                violation,
                resolved_space,
            );
        }

        // Draw the debug outline *after* on_finish so it sits on top of this
        // layout's content (and any retroactive chrome patching, e.g. a frame).
        if debug_layout {
            self.cmds.push(crate::draw::DrawCmd::StrokeRect {
                rect: resolved_space,
                color: crate::types::Color::from_srgb_u8(255, 0, 200, 255),
                width: 1.0,
            });
        }

        resolved_space
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{Align, AxisBound, IntrinsicSize};
    use crate::layouts::{
        ColumnLayout, ColumnLayoutParams, ManualLayout, RowLayout, RowLayoutParams,
    };
    use crate::test_utils::DummyTextSys;
    use crate::types::Color;

    /// `child_with_layout` resolves `placement` against the parent layout, then begins
    /// the child layout at those bounds — replacing the old layout()/begin() dance.
    #[test]
    fn child_with_layout_fuses_placement_and_begin() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );

        // Place a column at (10,10) sized 200x400 inside the root manual layout,
        // then a row nested at the column's first slot.
        let mut col = ctx.child_with_layout(Rect::new(10.0, 10.0, 200.0, 400.0), ColumnLayout);
        let mut row = col.child_with_layout(ColumnLayoutParams::fixed(200.0, 30.0), RowLayout);

        // The row sits at the column's origin (10,10); its first child lands there.
        let first = row.layout(RowLayoutParams::fixed(50.0, 30.0), IntrinsicSize::UNKNOWN);
        assert_eq!(first, Rect::new(10.0, 10.0, 50.0, 30.0));
        row.spacer(4.0);
        // Second row child advances by width + spacing.
        let second = row.layout(RowLayoutParams::fixed(40.0, 30.0), IntrinsicSize::UNKNOWN);
        assert_eq!(second, Rect::new(64.0, 10.0, 40.0, 30.0));
    }

    /// A bare nested layout placed with `Extent::Auto` should fit to its children:
    /// the parent's cursor must advance by the inner content's measured height. (The old
    /// eager path produced a 96px fallback box here; that case now fits to content.)
    #[test]
    fn nested_auto_layout_fits_children() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ColumnLayout,
            Rect::new(0.0, 0.0, 200.0, 600.0),
            &mut cmds,
        );

        // Place a nested column that fills width but auto-sizes its height.
        {
            let mut inner =
                ctx.child_with_layout(ColumnLayoutParams::auto().fill_x(), ColumnLayout);
            // Two stacked rows of height 30 → inner content height = 60.
            inner.layout(
                ColumnLayoutParams::fixed(50.0, 30.0),
                IntrinsicSize::UNKNOWN,
            );
            inner.layout(
                ColumnLayoutParams::fixed(50.0, 30.0),
                IntrinsicSize::UNKNOWN,
            );
            inner.finish();
        }

        // The next sibling in the parent column should land directly below the inner
        // content (y = 60), not below a 96px fallback box.
        let sibling = ctx.layout(
            ColumnLayoutParams::fixed(50.0, 20.0),
            IntrinsicSize::UNKNOWN,
        );
        assert_eq!(sibling.y, 60.0);
    }

    /// Cross-axis counterpart: a nested layout with `Extent::Auto` width inside a row
    /// should fit to the width its children actually consumed, so the next sibling
    /// advances by that measured width — not by the fallback.
    #[test]
    fn nested_auto_width_fits_children() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            RowLayout,
            Rect::new(0.0, 0.0, 600.0, 200.0),
            &mut cmds,
        );

        // A nested column that auto-sizes its width but takes a fixed height.
        {
            let mut inner =
                ctx.child_with_layout(RowLayoutParams::auto().fixed_y(30.0), ColumnLayout);
            // A single 50-wide row → inner content width = 50.
            inner.layout(
                ColumnLayoutParams::fixed(50.0, 30.0),
                IntrinsicSize::UNKNOWN,
            );
            inner.finish();
        }

        // Next sibling in the row advances by the measured width (50), not 96.
        let sibling = ctx.layout(RowLayoutParams::fixed(20.0, 30.0), IntrinsicSize::UNKNOWN);
        assert_eq!(sibling.x, 50.0);
    }

    /// Equivalence guarantee for the common case: a `Fixed`-sized slot resolves to the
    /// committed size regardless of how large its children turn out, exactly as the old
    /// eager `layout()` path did. Here the inner content (200px tall) far exceeds the
    /// fixed 50px slot, yet the sibling still lands at y = 50.
    #[test]
    fn nested_fixed_slot_ignores_child_extent() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ColumnLayout,
            Rect::new(0.0, 0.0, 200.0, 600.0),
            &mut cmds,
        );

        {
            let mut inner =
                ctx.child_with_layout(ColumnLayoutParams::fixed(80.0, 50.0), ColumnLayout);
            // Overflowing content: two 100px rows = 200px, far taller than the 50px slot.
            inner.layout(
                ColumnLayoutParams::fixed(80.0, 100.0),
                IntrinsicSize::UNKNOWN,
            );
            inner.layout(
                ColumnLayoutParams::fixed(80.0, 100.0),
                IntrinsicSize::UNKNOWN,
            );
            inner.finish();
        }

        // The fixed slot wins: cursor advanced by 50, not by the 200px of content.
        let sibling = ctx.layout(
            ColumnLayoutParams::fixed(50.0, 20.0),
            IntrinsicSize::UNKNOWN,
        );
        assert_eq!(sibling.y, 50.0);
    }

    #[test]
    fn test_highlight_policy_on_violation() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ColumnLayout,
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(100.0)),
            &mut cmds,
        );

        ctx.layout_policy = LayoutViolationPolicy::Highlight;

        // Placement::fill() on AtMost(100.0) width triggers UnsatisfiableFill violation
        let rect = ctx.layout(
            ColumnLayoutParams::auto().fill_x().fill_y(),
            IntrinsicSize::UNKNOWN,
        );

        assert_eq!(rect, Rect::new(0.0, 0.0, 0.0, 100.0)); // fallback width is 0.0

        // The StrokeRect command should have been drawn: red, width 2.0
        let mut found = false;
        for cmd in ctx.cmds.iter() {
            if let crate::draw::DrawCmd::StrokeRect {
                rect: r,
                color,
                width,
            } = cmd
            {
                if *color == Color::from_srgb_u8(255, 0, 0, 255) && *width == 2.0 {
                    assert_eq!(*r, Rect::new(0.0, 0.0, 0.0, 100.0));
                    found = true;
                    break;
                }
            }
        }
        assert!(
            found,
            "Should have found a red StrokeRect highlighting the layout violation"
        );
    }

    /// Deferred path: a begin_layout violation must be carried on the *child* and
    /// reacted at the child's own finish() — once per child. Regression guard for the
    /// bug where the violation was stashed on the parent (dropping all but the first).
    #[test]
    fn test_highlight_policy_deferred_begin_layout_violation_per_child() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ColumnLayout,
            // AtMost width → a Fixed+Center child can't be centered at begin_layout.
            LayoutSpace::new(0.0, 0.0, AxisBound::AtMost(100.0), AxisBound::Exact(200.0)),
            &mut cmds,
        );
        ctx.layout_policy = LayoutViolationPolicy::Highlight;

        // Two deferred children, each violating alignment at begin_layout.
        for _ in 0..2 {
            let placement = ColumnLayoutParams::fixed(40.0, 20.0).align_x(Align::Center);
            let child = ctx.child_with_layout(placement, ColumnLayout);
            child.finish();
        }
        ctx.finish();

        // Center-on-AtMost faults at BOTH begin_layout and end_layout, so each child
        // contributes two highlights → 2 children × 2 = 4. The key point is that no
        // child's violation is dropped. (The old bug stashed the begin_layout violation
        // on the *parent* under an is_none() guard, dropping the 2nd child's → only 3.)
        let count = cmds
            .iter()
            .filter(|cmd| {
                matches!(
                    cmd,
                    crate::draw::DrawCmd::StrokeRect { color, width, .. }
                        if *color == Color::from_srgb_u8(255, 0, 0, 255) && *width == 2.0
                )
            })
            .count();
        assert_eq!(
            count, 4,
            "both children must report begin+end violations (got {count})"
        );
    }
}
