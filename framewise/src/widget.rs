use crate::Input;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::layout::LayoutState;
use crate::text::{FontId, TextSystem};
use crate::theme::Theme;
use crate::types::{Color, Rect};
use crate::widgets::{button::ButtonStyle, frame::FrameStyle};

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
#[derive(Debug, Clone, Copy, Default)]
pub struct InputInfo {
    /// True while the cursor is over the widget's bounds this frame.
    pub hovered: bool,
    /// True while the primary mouse button is held and the cursor is over the widget.
    pub pressed: bool,
    /// True on the single frame the primary button was released over the widget.
    pub clicked: bool,
}


pub trait WidgetScope {
    /// Finish the scope, returning any draw commands that need to be emitted after the caller's own commands.
    fn finish(self, focus_sys: &mut FocusSystem) -> Vec<DrawCmd>;
}
impl WidgetScope for () {
    fn finish(self, _focus_sys: &mut FocusSystem) -> Vec<DrawCmd> {
        vec![]
    }
}

// ── WidgetContext ───────────────────────────────────────────────────────────

/// Context struct providing theme, input, focus, text system, and draw command
/// accumulation for high-level widget functions. This replaces the old `Builder`
/// pattern with freestanding functions.
/// Can be associated with a 'scope' value, which allows widget cleanup code to be run
/// when this context is finished (e.g. for nested windows)
pub struct WidgetContext<'a, T: TextSystem, LS: LayoutState, Scope: WidgetScope> {
    // Styling & environment fields (formerly BuilderCtx)
    pub theme: Theme,
    pub time: f64,
    pub clip_rect: Option<Rect>,

    // System resources
    pub text_system: &'a mut T,
    pub focus_sys: &'a mut FocusSystem,
    pub input: &'a Input,

    pub layout_state: LS,
    cmds: Vec<DrawCmd>,

    pub scope: Scope, // May be ()
}

impl<'a, T: TextSystem, LS: LayoutState> WidgetContext<'a, T, LS, ()> {
    // root() is only valid on a scope-less context.
    pub fn root(
        theme: Theme,
        text_system: &'a mut T,
        focus_sys: &'a mut FocusSystem,
        input: &'a Input,
        layout_state: LS,
    ) -> Self {
        Self {
            time: 0.0,
            clip_rect: None,
            theme,
            text_system,
            focus_sys,
            input,
            layout_state,
            cmds: Vec::new(),
            scope: (),
        }
    }
}

impl<'a, T: TextSystem, LS: LayoutState, Scope: WidgetScope> WidgetContext<'a, T, LS, Scope> {
    pub fn child_with_layout<'c, LS2: LayoutState, Scope2: WidgetScope>(
            &'c mut self, inner_layout_state: LS2, inner_scope: Scope2
        ) -> WidgetContext<'c, T, LS2, Scope2> {
        WidgetContext {
            theme: self.theme.clone(),
            time: self.time.clone(),
            clip_rect: self.clip_rect.clone(),
            text_system: self.text_system,
            focus_sys: self.focus_sys,
            input: self.input,
            layout_state: inner_layout_state,
            cmds: vec![],
            scope: inner_scope, // The original scope is not copied - correct as the original context will still own it.
        }
   }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, mut cmds: Vec<DrawCmd>) {
        self.cmds.append(&mut cmds);
    }

    /// Consume the context and return all accumulated draw commands.
    pub fn finish(self) -> Vec<DrawCmd> {
        let mut post_cmds = self.scope.finish(self.focus_sys);
        let mut cmds = self.cmds;
        cmds.append(&mut post_cmds);
        cmds
    }




    /// Resolve layout using the context's layout state.
    pub fn layout(&mut self, params: LS::Params) -> Rect {
        self.layout_state.layout(params)
    }

    // pub fn finish_child<'a2, 'b, T2: TextSystem, S2: LayoutState>(&'b mut self, child: WidgetContext<'a2, T2, S2>) {
    //      self.append_cmds(child.finish());
    // }
}
