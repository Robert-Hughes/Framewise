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
/// let mut ui = Builder::new(ctx, &mut text_system);
/// let btn = ui.button(rect, "OK", &input);
/// if btn.clicked() { println!("clicked"); }
/// let cmds = ui.finish();
/// ```
pub struct Builder<'a, T: TextSystem> {
    ctx:  BuilderCtx,
    cmds: Vec<DrawCmd>,
    pub text_system: &'a mut T,
    pub focus_sys:   &'a mut crate::focus::FocusSystem,
}

impl<'a, T: TextSystem> Builder<'a, T> {
    /// Create a new top-level builder with the given context.
    pub fn new(ctx: BuilderCtx, text_system: &'a mut T, focus_sys: &'a mut crate::focus::FocusSystem) -> Self {
        Self { ctx, cmds: Vec::new(), text_system, focus_sys }
    }

    /// Create a child builder that inherits a copy of this builder's context.
    /// The child accumulates its own draw commands; call `merge_child` to
    /// incorporate them into the parent.
    pub fn child(&mut self) -> Builder<'_, T> {
        Builder { ctx: self.ctx.clone(), cmds: Vec::new(), text_system: &mut *self.text_system, focus_sys: &mut *self.focus_sys }
    }

    /// Extract a child builder's draw commands into this builder.
    pub fn merge_child(&mut self, child: Builder<'_, T>) {
        self.cmds.extend(child.cmds);
    }

    /// Consume any `WidgetResult`: extract its draw commands into the
    /// accumulated list and return the caller-facing info.
    pub fn emit<R: WidgetResult>(&mut self, result: R) -> R::Info {
        let (draw, info) = result.into_parts();
        self.cmds.extend(draw.0);
        info
    }

    /// Consume the builder and return all accumulated draw commands.
    pub fn finish(self) -> Vec<DrawCmd> {
        self.cmds
    }

    // ── Convenience widget methods ─────────────────────────────────────────

    /// Draw a label (text stub) and return its info.
    pub fn label(&mut self, rect: Rect, text: &str) -> LabelInfo {
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
    pub fn text_edit(&mut self, state: TextEditState, rect: Rect, input: &Input) -> (TextEditInfo, TextEditState) {
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
        let state = res.state.clone();
        (self.emit(res), state)
    }

    /// Draw a button and return its info, including interaction state.
    pub fn button(
        &mut self,
        state: crate::widgets::button::ButtonState,
        rect:  Rect,
        text:  impl Into<String>,
        input: &Input,
    ) -> ButtonInfo {
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

    /// Draw a frame (bordered background) and return its info.
    pub fn frame(&mut self, rect: Rect) -> FrameInfo {
        let result = frame(FrameSpec {
            rect,
            style: self.ctx.frame_style,
        });
        self.emit(result)
    }
}
