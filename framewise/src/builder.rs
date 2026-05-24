use crate::{
    draw::DrawCmd,
    input::Input,
    types::{Color, Rect, Vec2},
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
    pub clip_rect:    Option<Rect>,
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
            clip_rect:    None,
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
    pub ctx:  BuilderCtx,
    cmds: Vec<DrawCmd>,
    pub text_system: &'a mut T,
    pub focus_sys:   &'a mut crate::focus::FocusSystem,
    pub layout_state: S,
    pub scroll_scope: Option<crate::widgets::scroll_area::ScrollAreaScope>,
}

impl<'a, T: crate::text::TextSystem, S: crate::layout::LayoutState> Builder<'a, T, S> {
    /// Create a new top-level builder with the given context.
    pub fn new(ctx: BuilderCtx, text_system: &'a mut T, focus_sys: &'a mut crate::focus::FocusSystem, layout_state: S) -> Self {
        Self { ctx, cmds: Vec::new(), text_system, focus_sys, layout_state, scroll_scope: None }
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
            text_system: self.text_system,
            focus_sys: &mut *self.focus_sys,
            layout_state: new_state,
            scroll_scope: None,
        }
    }

    /// Consume any `WidgetResult`: extract its draw commands into the
    /// accumulated list and return the caller-facing info.
    pub fn emit<R: WidgetResult>(&mut self, result: R) -> R::Info {
        let (draw, info) = result.into_parts();
        self.cmds.extend(draw.0);
        info
    }

    pub fn slider(
        &mut self,
        state: &mut crate::widgets::slider::SliderState,
        value: &mut f32,
        min: f32,
        max: f32,
        page_step: f32,
        orientation: crate::widgets::slider::Orientation,
        params: S::Params,
        input: &Input,
    ) {
        let rect = self.layout_state.layout(params);
        let spec = crate::widgets::slider::SliderSpec {
            rect,
            min,
            max,
            page_step,
            step: page_step / 10.0,
            orientation,
            thumb_size_ratio: None, // Generic slider doesn't resize thumb based on content
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect: self.ctx.clip_rect,
            claim_scroll_at_ends: true, // Standalone: always block scroll propagation
        };
        let cmds = crate::widgets::slider::slider(
            state,
            value,
            spec,
            input,
            self.ctx.time,
            self.focus_sys,
        );
        self.append_cmds(cmds);
    }

    /// Consume the builder and return all accumulated draw commands.
    pub fn finish(mut self) -> Vec<DrawCmd> {
        if let Some(scope) = self.scroll_scope.take() {
            let post_cmds = scope.finish(self.focus_sys);
            self.cmds.extend(post_cmds);
        }

        self.cmds
    }

    // ── Convenience widget methods ─────────────────────────────────────────

    /// Creates a scroll area child builder.
    pub fn scroll_area<L: crate::layout::Layout>(
        &mut self,
        params: S::Params,
        content_size: Vec2,
        h_vis: crate::widgets::scroll_area::ScrollbarVisibility,
        v_vis: crate::widgets::scroll_area::ScrollbarVisibility,
        state: &'a mut crate::widgets::scroll_area::ScrollState,
        inner_layout: L,
        input: &Input,
    ) -> Builder<'_, T, crate::layout::OffsetState<L::State>> {
        let bounds = self.layout_state.layout(params);
        let (pre_cmds, scope, content_bounds, offset_layout) = crate::widgets::scroll_area::begin_scroll_area(
            bounds,
            content_size,
            h_vis,
            v_vis,
            state,
            inner_layout,
            input,
            &mut *self.focus_sys,
            self.ctx.clip_rect,
            self.ctx.time,
        );

        self.append_cmds(pre_cmds);

        let parent_clip = self.ctx.clip_rect;
        let mut child = self.child_with_manual_bounds(content_bounds, offset_layout);
        child.scroll_scope = Some(scope);
        
        let new_clip = if let Some(pc) = parent_clip {
            pc.intersect(&content_bounds)
        } else {
            content_bounds
        };
        child.ctx.clip_rect = Some(new_clip);

        child
    }

    /// Draw a label and return its info.
    pub fn label(&mut self, params: S::Params, text: &str) -> LabelInfo {
        let rect = self.layout_state.layout(params);
        let spec = LabelSpec {
            rect,
            text: text.to_string(),
            size: self.ctx.text_size,
            text_color: self.ctx.text_color,
            rule: false,
        };
        let res = label(spec, self.text_system);
        self.emit(res)
    }

    /// Draw a label with explicit size, color and optional rule.
    pub fn label_styled(&mut self, params: S::Params, text: &str, size: f32, color: Color, rule: bool) -> LabelInfo {
        let rect = self.layout_state.layout(params);
        let spec = LabelSpec {
            rect,
            text: text.to_string(),
            size,
            text_color: color,
            rule,
        };
        let res = label(spec, self.text_system);
        self.emit(res)
    }

    /// Emit a text_edit widget.
    pub fn text_edit(&mut self, state: TextEditState, params: S::Params, input: &Input) -> TextEditInfo {
        self.text_edit_ext(state, params, false, false, input)
    }

    /// Emit a text_edit widget with explicit error/disabled flags.
    pub fn text_edit_ext(&mut self, state: TextEditState, params: S::Params, error: bool, disabled: bool, input: &Input) -> TextEditInfo {
        let rect = self.layout_state.layout(params);
        let spec = TextEditSpec {
            rect,
            style: TextEditStyle {
                text_size: self.ctx.text_size,
                ..Default::default()
            },
            clip_rect: self.ctx.clip_rect,
            error,
            disabled,
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
        self.button_styled(state, params, text, self.ctx.button_style, false, input)
    }

    /// Draw a button with explicit style and disabled flag.
    pub fn button_styled(
        &mut self,
        state: crate::widgets::button::ButtonState,
        params: S::Params,
        text:  impl Into<String>,
        style: ButtonStyle,
        disabled: bool,
        input: &Input,
    ) -> ButtonInfo {
        let rect = self.layout_state.layout(params);
        let result = button(
            state,
            ButtonSpec {
                rect,
                text:     text.into(),
                style,
                clip_rect: self.ctx.clip_rect,
                disabled,
            },
            input,
            self.text_system,
            self.focus_sys,
        );
        self.emit(result)
    }

    pub fn frame(&mut self, params: S::Params) -> FrameInfo {
        let rect = self.layout_state.layout(params);
        let result = frame(FrameSpec {
            rect,
            style: self.ctx.frame_style,
        });
        self.emit(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Input;
    use crate::types::{Vec2, Rect};
    use crate::layout::{ManualLayout, Layout};
    use crate::focus::FocusSystem;

    struct DummyTextSystem;
    impl crate::text::TextSystem for DummyTextSystem {
        fn prepare(&mut self, _text: &str, _size: f32) -> crate::text::TextLayout {
            crate::text::TextLayout {
                size: Vec2::new(10.0, 10.0),
                handle: crate::text::TextHandle(0),
            }
        }
        fn measure_byte_x(&self, _handle: crate::text::TextHandle, _byte_index: usize) -> f32 {
            0.0
        }
        fn hit_test_x(&self, _handle: crate::text::TextHandle, _x_offset: f32) -> usize {
            0
        }
    }

    #[test]
    fn test_clipped_hit_testing() {
        let mut text_sys = DummyTextSystem;
        let mut focus_sys = FocusSystem::new();
        focus_sys.begin_frame();

        let mut input = Input::new();
        // Mouse is placed at Y=20.
        input.mouse_pos = Vec2::new(15.0, 20.0);
        input.mouse_pressed = true;
        input.mouse_down = true;

        let ctx = BuilderCtx::default();
        let mut builder = Builder::new(ctx, &mut text_sys, &mut focus_sys, ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)));

        // Create a scroll area positioned at Y=50, Height=100 (so it clips everything above Y=50).
        let mut scroll_state = crate::widgets::scroll_area::ScrollState::default();
        let mut scroll_area = builder.scroll_area(
            Rect::new(10.0, 50.0, 100.0, 100.0), 
            Vec2::new(100.0, 500.0),
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            &mut scroll_state, 
            ManualLayout, 
            &input
        );

        // Place a button INSIDE the scroll area, but position its mathematical Rect at Y=-30 (relative)
        // Since ManualLayout adds the parent's top_left (Y=50), the absolute Y will be 50 - 30 = 20!
        // This simulates a button that has scrolled UP and OUT of the scroll area bounds (Y=50..150).
        let btn_state = crate::widgets::button::ButtonState::default();
        let btn_info = scroll_area.button(btn_state, Rect::new(0.0, -30.0, 50.0, 20.0), "Btn", &input);
        
        scroll_area.finish();
        builder.finish();

        // The button's absolute mathematical bounds (Y=20) contains the mouse (Y=20).
        // However, the button is rendered inside a scroll area that clips at Y=50!
        assert_eq!(btn_info.state.is_active, false, "Button was clicked even though it was clipped out of view!");
    }

    #[test]
    fn test_nested_clip_rect_intersections() {
        // Complex nested layout test: a builder with a clip rect creates a child scroll area, 
        // which creates another child scroll area. We verify the clip rects intersect correctly.
        let mut text_sys = DummyTextSystem;
        let mut focus_sys = FocusSystem::new();
        focus_sys.begin_frame();

        let input = Input::new();
        
        let mut ctx = BuilderCtx::default();
        // Start with an artificial clip rect for the whole app
        ctx.clip_rect = Some(Rect::new(20.0, 20.0, 500.0, 500.0));
        
        let mut builder = Builder::new(ctx, &mut text_sys, &mut focus_sys, ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)));

        // Outer scroll area: at 50,50, size 200x200
        let mut outer_state = crate::widgets::scroll_area::ScrollState::default();
        let mut outer = builder.scroll_area(
            Rect::new(50.0, 50.0, 200.0, 200.0), 
            Vec2::new(200.0, 1000.0),
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            &mut outer_state, 
            ManualLayout, 
            &input
        );

        // Parent clip was 20..520 (x,y). Outer bounds is 50..250. 
        // Inner content bounds should be 50,50 to 238,250 (12px scrollbar).
        // Intersection should be exactly the inner content bounds.
        assert_eq!(outer.ctx.clip_rect, Some(Rect::new(50.0, 50.0, 188.0, 200.0)));

        // Inner scroll area: at 100,100, size 200x200 (extends beyond outer!)
        let mut inner_state = crate::widgets::scroll_area::ScrollState::default();
        let inner = outer.scroll_area(
            Rect::new(100.0, 100.0, 200.0, 200.0), 
            Vec2::new(200.0, 1000.0),
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            crate::widgets::scroll_area::ScrollbarVisibility::Auto,
            &mut inner_state, 
            ManualLayout, 
            &input
        );

        // Outer clip was 50,50, w:188, h:200 => right:238, bottom:250
        // Inner bounds is relative to outer content_bounds (50,50), so inner starts at 150,150.
        // Inner content_bounds is w:188, h:200 => right:338, bottom:350.
        // Intersection of (50..238, 50..250) and (150..338, 150..350):
        // x=150, y=150, right=238, bottom=250 => w=88, h=100.
        assert_eq!(inner.ctx.clip_rect, Some(Rect::new(150.0, 150.0, 88.0, 100.0)));
        
        inner.finish();
        outer.finish();
    }
}
