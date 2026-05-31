use crate::draw::DrawCommands;
use crate::focus::FocusSystem;
use crate::layout::{Layout, LayoutState};
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
    WidgetContext<'a, T, LS, fn(&mut FocusSystem, &mut DrawCommands, Vec2)>
{
    pub fn root(
        theme: Theme,
        text_system: &'a mut T,
        focus_system: &'a mut FocusSystem,
        input: &'a Input,
        layout_state: LS,
        cmds: &'a mut DrawCommands,
    ) -> Self {
        Self {
            time: 0.0,
            clip_rect: None,
            theme,
            text_system,
            focus_system,
            input,
            layout_state,
            cmds,
            on_finish: |_, _, _| (), // No cleanup for root context
        }
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF> WidgetContext<'a, T, LS, CF> {
    pub fn child_with_layout_and_on_finish_and_clip_rect<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2),
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
        CF2: FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2),
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
    /// bounds, then `inner_layout` is begun at those bounds. This is the standard path
    /// for nesting layouts (column inside row, etc.) and replaces the old
    /// `layout(params)` + `inner.begin(bounds)` + `child_with_layout(state)` dance.
    pub fn child_with_layout<'c, L2: Layout>(
        &'c mut self,
        placement: LS::Params,
        inner_layout: L2,
    ) -> WidgetContext<'c, T, L2::State, impl FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2)>
    {
        let bounds = self
            .layout_state
            .layout(placement, crate::layout::IntrinsicSize::UNKNOWN);
        self.child_with_layout_and_on_finish_and_clip_rect(
            inner_layout.begin(bounds),
            |_, _, _| (),
            self.clip_rect, // Clip rect is inherited by default
        )
    }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, cmds: DrawCommands) {
        self.cmds.extend(cmds);
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF: FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2)>
    WidgetContext<'a, T, LS, CF>
{
    /// Consume the context, running the on_finish closure and appending its post-commands.
    ///
    /// The layout's accumulated [`content_extent`](LayoutState::content_extent) is
    /// passed to the closure so container widgets (e.g. a deferred scroll area) can
    /// resolve geometry from how large their children turned out.
    pub fn finish(self) {
        let content_extent = self.layout_state.content_extent();
        (self.on_finish)(self.focus_system, self.cmds, content_extent);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ColumnLayout, IntrinsicSize, Layout, ManualLayout, RowLayout};
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
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
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
}
