use crate::Input;
use crate::draw::DrawCmd;
use crate::focus::FocusSystem;
use crate::layout::LayoutState;
use crate::text::{FontId, TextSystem};
use crate::theme::Theme;
use crate::types::{Color, Rect};
use crate::widgets::{ScrollAreaScope, WindowScope};
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

// ── WidgetContext ───────────────────────────────────────────────────────────

/// Context struct providing theme, input, focus, text system, and draw command
/// accumulation for high-level widget functions. This replaces the old `Builder`
/// pattern with freestanding functions.
pub struct WidgetContext<'a, T: TextSystem, S: LayoutState> {
    // Styling & environment fields (formerly BuilderCtx)
    pub theme: Theme,
    pub bg_color: Color,
    pub accent_color: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub button_style: ButtonStyle,
    pub frame_style: FrameStyle,
    pub text_size: f32,
    pub text_font: FontId,
    pub time: f64,
    pub clip_rect: Option<Rect>,

    // System resources
    pub text_system: &'a mut T,
    pub focus_sys: &'a mut FocusSystem,
    pub input: &'a Input,

    pub layout_state: S,
    cmds: Vec<DrawCmd>,

    //TODO: this isn't good - not extensible!
    pub scroll_scope: Option<ScrollAreaScope>,
    pub window_scope: Option<WindowScope>,
}

impl<'a, T: TextSystem, S: LayoutState> WidgetContext<'a, T, S> {
    pub fn new(
        theme: Theme,
        text_system: &'a mut T,
        focus_sys: &'a mut FocusSystem,
        input: &'a Input,
        layout_state: S,
    ) -> Self {
        Self {
            bg_color: Color::from_srgb_f32(0.10, 0.10, 0.13, 1.0),
            accent_color: Color::from_srgb_f32(0.30, 0.55, 0.95, 1.0),
            text_color: Color::from_srgb_f32(0.90, 0.90, 0.95, 1.0),
            border_color: Color::from_srgb_f32(0.30, 0.30, 0.38, 1.0),
            button_style: theme.button_secondary_style(),
            frame_style: theme.frame_style(),
            text_size: 14.0,
            text_font: theme.sans_font,
            time: 0.0,
            clip_rect: None,
            theme,
            text_system,
            focus_sys,
            input,
            layout_state,
            cmds: Vec::new(),
            scroll_scope: None,
            window_scope: None,
        }
    }

    /// Append draw commands to the context's accumulated list.
    pub fn append_cmds(&mut self, mut cmds: Vec<DrawCmd>) {
        self.cmds.append(&mut cmds);
    }

    /// Consume the context and return all accumulated draw commands.
    pub fn finish(mut self) -> Vec<DrawCmd> {
        if let Some(scope) = self.scroll_scope.take() {
            let post_cmds = scope.finish(self.focus_sys);
            self.append_cmds(post_cmds);
        }
        if let Some(scope) = self.window_scope.take() {
            let post_cmds = scope.finish();
            self.append_cmds(post_cmds);
        }
        self.cmds
    }


    /// Resolve layout using the context's layout state.
    pub fn layout(&mut self, params: S::Params) -> Rect {
        self.layout_state.layout(params)
    }
}
