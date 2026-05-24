use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::{Color, Rect}, widget::{WidgetSpec, WidgetSpecBuilder}
};

pub struct SwitchSpec {
    /// Top-left of the 30×16 bounding area.
    pub rect:     Rect,
    pub on:       bool,
    pub focused:  bool,
    pub disabled: bool,
}

impl WidgetSpec for SwitchSpec {
    type Builder = SwitchSpecBuilder;
}

pub struct SwitchSpecBuilder {
    spec: SwitchSpec,
}

impl SwitchSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: SwitchSpec {
                rect: Rect::ZERO,
                on: false,
                focused: false,
                disabled: false,
            }
        }
    }

    pub fn on(mut self, on: bool) -> Self {
        self.spec.on = on;
        self
    }

    pub fn focused(mut self, focused: bool) -> Self {
        self.spec.focused = focused;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.spec.disabled = disabled;
        self
    }
}

impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for SwitchSpecBuilder {
    type Spec = SwitchSpec;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn build(self) -> Self::Spec {
        self.spec
    }
}

pub struct SwitchResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SwitchResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn switch(spec: SwitchSpec) -> SwitchResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();
    let alpha = if spec.disabled { 0.35_f32 } else { 1.0 };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

    let r = Rect::new(spec.rect.x, spec.rect.y, 30.0, 16.0);

    // Focus ring.
    if spec.focused {
        cmds.push(DrawCmd::StrokeRect {
            rect:  r.inset(-2.0),
            color: tint(t.rust),
            width: 2.0,
        });
    }

    // Track fill.
    let track_fill = if spec.on { t.ink } else { t.paper_elev };
    cmds.push(DrawCmd::FillRect { rect: r, color: tint(track_fill) });

    // Track border.
    cmds.push(DrawCmd::StrokeRect {
        rect:  r,
        color: tint(t.ink),
        width: 1.5,
    });

    // Thumb dot (10×10, vertically centered, left/right positioned).
    let dot_y = r.y + (r.h - 10.0) * 0.5;
    let dot_x = if spec.on { r.x + r.w - 10.0 - 1.5 } else { r.x + 1.5 };
    let dot_color = if spec.on { t.paper } else { t.ink };
    cmds.push(DrawCmd::FillRect {
        rect:  Rect::new(dot_x, dot_y, 10.0, 10.0),
        color: tint(dot_color),
    });

    SwitchResult { draw: cmds }
}
