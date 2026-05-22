use crate::{
    draw::DrawCmd,
    input::Input,
    text::TextSystem,
    types::{Color, Rect},
    widget::WidgetResult,
    widgets::{
        button::{button, ButtonInfo, ButtonSpec, ButtonStyle},
        frame::{frame, FrameInfo, FrameSpec, FrameStyle},
        label::{label, LabelInfo, LabelSpec},
        text_edit::{text_edit, TextEditInfo, TextEditSpec, TextEditState, TextEditStyle},
    },
    layout::LayoutState,
};

// ── BuilderCtx ────────────────────────────────────────────────────────────────

/// Resolved styling context carried by a `Builder` and inherited by child
/// builders. Child builders receive a *copy* of the parent context; changes to
/// the child do not affect the parent.
#[derive(Debug, Clone)]
pub struct BuilderCtx {
    pub bg_color:     Color,
    pub accent_color: Color,
    pub text_color:   Color,
    pub border_color: Color,
    pub button_style: ButtonStyle,
    pub frame_style:  FrameStyle,
    pub text_size:    f32,
    pub time:         f64,
}

impl Default for BuilderCtx {
    fn default() -> Self {
        Self {
            bg_color:     Color::rgb(0.10, 0.10, 0.13),
            accent_color: Color::rgb(0.30, 0.55, 0.95),
            text_color:   Color::rgb(0.90, 0.90, 0.95),
            border_color: Color::rgb(0.30, 0.30, 0.38),
            button_style: ButtonStyle::default(),
            frame_style:  FrameStyle::default(),
            text_size:    14.0,
            time:         0.0,
        }
    }
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Ergonomic layer that carries inherited defaults and accumulates draw
/// commands. Call `finish()` to retrieve the flat `Vec<DrawCmd>` for the
/// renderer.
///
/// # Example
///
/// ```ignore
/// let mut builder = Builder::new(ctx, &mut text_system);
/// let btn = builder.button(rect, "OK", &input);
/// if btn.clicked() { println!("clicked"); }
/// let cmds = builder.finish();
/// ```
pub struct Builder<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState> {
    ctx:  BuilderCtx,
    cmds: Vec<DrawCmd>,
    pub text_system: &'a mut T,
    pub focus_sys:   &'a mut crate::focus::FocusSystem,
    pub layout_state: S,
    pub needs_pop_clip: bool,
}

impl<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState> Builder<'a, T, S> {
    /// Create a new top-level builder with the given context.
    pub fn new(ctx: BuilderCtx, text_system: &'a mut T, focus_sys: &'a mut crate::focus::FocusSystem, layout_state: S) -> Self {
        Self { ctx, cmds: Vec::new(), text_system, focus_sys, layout_state, needs_pop_clip: false }
    }

    /// Extract a child builder's draw commands into this builder.
    pub fn merge_child<ChildS: crate::layout::LayoutState>(&mut self, child: Builder<'_, T, ChildS>) {
        self.cmds.extend(child.cmds);
    }

    /// Append a list of draw commands directly. Useful when a child builder has been finished.
    pub fn append_cmds(&mut self, mut cmds: Vec<DrawCmd>) {
        self.cmds.append(&mut cmds);
    }

    /// Creates a child builder with a new layout configuration. The parent builder allocates
    /// bounds using `parent_params` and passes them to the new layout's `begin` method.
    pub fn child_with_layout<L: crate::layout::Layout>(
        &mut self,
        parent_params: S::Params,
        layout_config: L,
    ) -> Builder<'_, T, L::State> {
        let bounds = self.layout_state.layout(parent_params);
        self.child_with_manual_bounds(bounds, layout_config)
    }

    /// Creates a child builder with a specific bounding box, bypassing the parent layout.
    pub fn child_with_manual_bounds<L: crate::layout::Layout>(
        &mut self,
        bounds: Rect,
        layout_config: L,
    ) -> Builder<'_, T, L::State> {
        let new_state = layout_config.begin(bounds);
        
        Builder {
            ctx: self.ctx.clone(),
            cmds: Vec::new(),
            text_system: &mut *self.text_system,
            focus_sys: &mut *self.focus_sys,
            layout_state: new_state,
            needs_pop_clip: false,
        }
    }

    /// Consume any `WidgetResult`: extract its draw commands into the
    /// accumulated list and return the caller-facing info.
    pub fn emit<R: WidgetResult>(&mut self, result: R) -> R::Info {
        let (draw, info) = result.into_parts();
        self.cmds.extend(draw.0);
        info
    }

    /// Consume the builder and return all accumulated draw commands.
    pub fn finish(mut self) -> Vec<DrawCmd> {
        if self.needs_pop_clip {
            self.cmds.push(DrawCmd::PopClip);
        }
        self.cmds
    }

    // ── Convenience widget methods ─────────────────────────────────────────

    /// Creates a scroll area child builder.
    pub fn scroll_area<L: crate::layout::Layout>(
        &mut self,
        params: S::Params,
        content_height: f32,
        state: &'a mut crate::widgets::scroll_area::ScrollState,
        inner_layout: L,
        input: &Input,
    ) -> Builder<'_, T, crate::layout::OffsetState<L::State>> {
        let bounds = self.layout_state.layout(params);
        let (scroll_cmds, content_bounds, offset_layout) = crate::widgets::scroll_area::scroll_area(
            bounds,
            content_height,
            state,
            inner_layout,
            input,
        );

        self.append_cmds(scroll_cmds);

        let mut child = self.child_with_manual_bounds(content_bounds, offset_layout);
        child.needs_pop_clip = true;
        child
    }

    /// Draw a label (text stub) and return its info.
    pub fn label(&mut self, params: S::Params, text: &str) -> LabelInfo {
        let rect = self.layout_state.layout(params);
        let spec = LabelSpec {
            rect,
            text: text.to_string(),
            size: self.ctx.text_size,
            text_color: self.ctx.text_color,
        };
        let res = label(spec, self.text_system);
        self.emit(res)
    }

    /// Emit a text_edit widget.
    pub fn text_edit(&mut self, state: TextEditState, params: S::Params, input: &Input) -> TextEditInfo {
        let rect = self.layout_state.layout(params);
        let spec = TextEditSpec {
            rect,
            style: TextEditStyle {
                text_size: self.ctx.text_size,
                // you could merge theme colours here
                ..Default::default()
            },
        };
        let res = text_edit(
            state,
            spec,
            input,
            self.ctx.time,
            self.text_system,
            self.focus_sys,
        );
        self.emit(res)
    }

    /// Draw a button and return its info, including interaction state.
    pub fn button(
        &mut self,
        state: crate::widgets::button::ButtonState,
        params: S::Params,
        text:  impl Into<String>,
        input: &Input,
    ) -> ButtonInfo {
        let rect = self.layout_state.layout(params);
        let result = button(
            state,
            ButtonSpec {
                rect,
                text:  text.into(),
                style: self.ctx.button_style,
            },
            input,
            self.text_system,
            self.focus_sys,
        );
        self.emit(result)
    }

    /// Draw a panel frame and return its info.
    pub fn frame(&mut self, params: S::Params) -> FrameInfo {
        let rect = self.layout_state.layout(params);
        let result = frame(FrameSpec {
            rect,
            style: self.ctx.frame_style,
        });
        self.emit(result)
    }
}
