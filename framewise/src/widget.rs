use crate::draw::DrawCommands;
use crate::focus::FocusSystem;
use crate::layout::{IntrinsicSize, Layout, LayoutState, ManualState};
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

    // System resources
    pub text_system: &'a mut T,
    pub focus_system: &'a mut FocusSystem,
    pub input: &'a Input,
    pub cmds: &'a mut DrawCommands,

    pub layout_state: LS,
    pub on_finish: CF,
}

impl<'a, T: TextSystem, LS: LayoutState>
    WidgetContext<'a, T, LS, fn(&mut FocusSystem, &mut DrawCommands, Rect)>
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
            theme,
            text_system,
            focus_system,
            input,
            layout_state: layout.begin(space.into()),
            cmds,
            on_finish: |_, _, _| (), // No cleanup for root context
        }
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF> WidgetContext<'a, T, LS, CF> {
    pub fn child_with_layout_and_on_finish_and_clip_rect<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem, &mut DrawCommands, Rect),
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
            text_system: self.text_system,
            focus_system: self.focus_system,
            input: self.input,
            layout_state: inner_layout_state,
            cmds: self.cmds,
            on_finish: inner_on_finish, // The original on_finish is not copied - correct as the original context will still own it.
        }
    }

    pub fn child_with_layout_and_on_finish<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem, &mut DrawCommands, Rect),
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
    /// unchanged. Only `Auto`/`Fill`-under-non-exact slots, which previously fell back to
    /// [`LAYOUT_FALLBACK_SIZE`](crate::layout::LAYOUT_FALLBACK_SIZE), now fit to content.
    pub fn child_with_layout<'c, L2: Layout>(
        &'c mut self,
        placement: LS::Params,
        inner_layout: L2,
    ) -> WidgetContext<'c, T, L2::State, impl FnOnce(&mut FocusSystem, &mut DrawCommands, Rect) + 'c>
    {
        let clip = self.clip_rect; // Clip rect is inherited by default.

        // Begin a deferred layout: provisional space for the child + a token that holds
        // the parent's layout state until `on_finish` advances its cursor.
        let (outer_space, token) = self
            .layout_state
            .begin_layout(placement, IntrinsicSize::UNKNOWN);

        // The token is moved into the closure, which borrows `self.layout_state`. The
        // child context below borrows the *other* fields (text_system, focus_system, …),
        // so the borrows are disjoint — hence the explicit field-by-field construction
        // rather than `child_with_layout_and_on_finish` (which would reborrow all of self).
        let on_finish = move |_: &mut FocusSystem, _: &mut DrawCommands, resolved_space: Rect| {
            let content_extent = Vec2::new(resolved_space.w, resolved_space.h);
            // Finalize the parent layout from the child's measured extent and advance
            // its cursor. (A bare layout draws no background, so nothing to patch.)
            let _bounds = token.end_layout(content_extent);
        };

        WidgetContext {
            theme: self.theme,
            time: self.time,
            clip_rect: clip,
            text_system: self.text_system,
            focus_system: self.focus_system,
            input: self.input,
            cmds: self.cmds,
            layout_state: inner_layout.begin(outer_space),
            on_finish,
        }
    }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, cmds: DrawCommands) {
        self.cmds.extend(cmds);
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF: FnOnce(&mut FocusSystem, &mut DrawCommands, Rect)>
    WidgetContext<'a, T, LS, CF>
{
    /// Consume the context, running the on_finish closure and appending its post-commands.
    ///
    /// The layout's resolved [`resolve_space`](LayoutState::resolve_space) is
    /// passed to the closure so container widgets (e.g. a deferred scroll area) can
    /// resolve geometry from how large their children turned out.
    pub fn finish(self) {
        let resolved_space = self.layout_state.resolve_space();
        (self.on_finish)(self.focus_system, self.cmds, resolved_space);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{
        ColumnLayout, CrossAlign, Extent, IntrinsicSize, Layout, ManualLayout, RowLayout, SizeReq,
    };
    use crate::test_utils::DummyTextSys;
    use crate::types::Vec2;

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
        let mut col = ctx.child_with_layout(
            Rect::new(10.0, 10.0, 200.0, 400.0),
            ColumnLayout {
                spacing: 5.0,
                align: crate::layout::CrossAlign::Start,
            },
        );
        let mut row = col.child_with_layout(
            Vec2::new(200.0, 30.0).into(),
            RowLayout {
                spacing: 4.0,
                align: crate::layout::CrossAlign::Start,
            },
        );

        // The row sits at the column's origin (10,10); its first child lands there.
        let first = row
            .layout_state
            .layout(Vec2::new(50.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(first, Rect::new(10.0, 10.0, 50.0, 30.0));
        // Second row child advances by width + spacing.
        let second = row
            .layout_state
            .layout(Vec2::new(40.0, 30.0).into(), IntrinsicSize::UNKNOWN);
        assert_eq!(second, Rect::new(64.0, 10.0, 40.0, 30.0));
    }

    /// A bare nested layout placed with `Extent::Auto` should fit to its children:
    /// the parent's cursor must advance by the inner content's measured height, not
    /// by the `LAYOUT_FALLBACK_SIZE` (96) that the old eager path produced.
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
            ColumnLayout {
                spacing: 0.0,
                align: CrossAlign::Start,
            },
            Rect::new(0.0, 0.0, 200.0, 600.0),
            &mut cmds,
        );

        // Place a nested column that fills width but auto-sizes its height.
        {
            let mut inner = ctx.child_with_layout(
                SizeReq {
                    width: Extent::Fill,
                    height: Extent::Auto,
                },
                ColumnLayout {
                    spacing: 0.0,
                    align: CrossAlign::Start,
                },
            );
            // Two stacked rows of height 30 → inner content height = 60.
            inner
                .layout_state
                .layout(SizeReq::fixed(50.0, 30.0), IntrinsicSize::UNKNOWN);
            inner
                .layout_state
                .layout(SizeReq::fixed(50.0, 30.0), IntrinsicSize::UNKNOWN);
            inner.finish();
        }

        // The next sibling in the parent column should land directly below the inner
        // content (y = 60), not below a 96px fallback box.
        let sibling = ctx
            .layout_state
            .layout(SizeReq::fixed(50.0, 20.0), IntrinsicSize::UNKNOWN);
        assert_eq!(sibling.y, 60.0);
    }
}
