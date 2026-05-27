use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::layout::LayoutState;
use crate::text::TextSystem;
use crate::theme::Theme;
use crate::types::Rect;
use crate::Input;

// ── Common result fragments ───────────────────────────────────────────────────

/// Resolved geometry returned by every widget.
#[derive(Debug, Clone, Copy)]
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
pub struct WidgetContext<
    'a,
    T: TextSystem,
    LS: LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
> {
    // Styling & environment fields (formerly BuilderCtx)
    pub theme: Theme,
    pub time: f64,
    pub clip_rect: Option<Rect>,

    // System resources
    pub text_system: &'a mut T,
    pub focus_sys: &'a mut FocusSystem,
    pub input: &'a Input,
    cmds: &'a mut Vec<DrawCmd>,

    pub layout_state: LS,
    on_finish: CF,
}

impl<'a, T: TextSystem, LS: LayoutState>
    WidgetContext<'a, T, LS, fn(&mut FocusSystem) -> Vec<DrawCmd>>
{
    pub fn root(
        theme: Theme,
        text_system: &'a mut T,
        focus_sys: &'a mut FocusSystem,
        input: &'a Input,
        layout_state: LS,
        cmds: &'a mut Vec<DrawCmd>,
    ) -> Self {
        Self {
            time: 0.0,
            clip_rect: None,
            theme,
            text_system,
            focus_sys,
            input,
            layout_state,
            cmds,
            on_finish: |_| vec![], // No cleanup for root context
        }
    }
}

impl<'a, T: TextSystem, LS: LayoutState, CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>>
    WidgetContext<'a, T, LS, CF>
{
    pub fn child_with_layout_and_on_finish<
        'c,
        LS2: LayoutState,
        CF2: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
    >(
        &'c mut self,
        inner_layout_state: LS2,
        inner_on_finish: CF2,
    ) -> WidgetContext<'c, T, LS2, CF2> {
        WidgetContext {
            theme: self.theme,
            time: self.time,
            clip_rect: self.clip_rect,
            text_system: self.text_system,
            focus_sys: self.focus_sys,
            input: self.input,
            layout_state: inner_layout_state,
            cmds: self.cmds,
            on_finish: inner_on_finish, // The original on_finish is not copied - correct as the original context will still own it.
        }
    }

    pub fn child_with_layout<'c, LS2: LayoutState>(
        &'c mut self,
        inner_layout_state: LS2,
    ) -> WidgetContext<'c, T, LS2, impl FnOnce(&mut FocusSystem) -> Vec<DrawCmd>> {
        self.child_with_layout_and_on_finish(inner_layout_state, |_| vec![])
    }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, mut cmds: Vec<DrawCmd>) {
        self.cmds.append(&mut cmds);
    }

    /// Consume the context and return all accumulated draw commands.
    pub fn finish(self) {
        let mut post_cmds = (self.on_finish)(self.focus_sys);
        self.cmds.append(&mut post_cmds);
    }

    /// Resolve layout using the context's layout state.
    pub fn layout(&mut self, params: LS::Params) -> Rect {
        self.layout_state.layout(params)
    }
}
