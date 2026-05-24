use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::{Color, Rect, Vec2}, widget::{WidgetSpec, WidgetSpecBuilder}
};

pub struct SpinnerSpec {
    /// Top-left. Size is either 16 or 24 (use `large` flag).
    pub rect:  Rect,
    pub large: bool,
    pub color: Option<Color>,
}

impl WidgetSpec for SpinnerSpec {
    type Builder = SpinnerSpecBuilder;
}

pub struct SpinnerSpecBuilder {
    spec: SpinnerSpec,
}

impl SpinnerSpecBuilder {
    pub fn new() -> Self {
        Self {
            spec: SpinnerSpec {
                rect: Rect::ZERO,
                large: false,
                color: None,
            }
        }
    }

    pub fn large(mut self, large: bool) -> Self {
        self.spec.large = large;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.spec.color = Some(color);
        self
    }
}

impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for SpinnerSpecBuilder {
    type Spec = SpinnerSpec;

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

pub struct SpinnerResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SpinnerResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

/// Square reticle spinner — four corner brackets with a single animated segment.
/// Since we can't animate, we draw it at a fixed phase (segment at top).
pub fn spinner(spec: SpinnerSpec) -> SpinnerResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let size = if spec.large { 24.0_f32 } else { 16.0_f32 };
    let color = spec.color.unwrap_or(t.ink);

    let x = spec.rect.x;
    let y = spec.rect.y;

    // Corner bracket size: 5px at 16, 7px at 24.
    let arm = if spec.large { 7.0_f32 } else { 5.0_f32 };
    let w = 1.5_f32;

    // Top-left bracket.
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x, y + arm), p1: Vec2::new(x, y), color, width: w });
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x, y), p1: Vec2::new(x + arm, y), color, width: w });
    // Top-right bracket.
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x + size - arm, y), p1: Vec2::new(x + size, y), color, width: w });
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x + size, y), p1: Vec2::new(x + size, y + arm), color, width: w });
    // Bottom-right bracket.
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x + size, y + size - arm), p1: Vec2::new(x + size, y + size), color, width: w });
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x + size, y + size), p1: Vec2::new(x + size - arm, y + size), color, width: w });
    // Bottom-left bracket.
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x + arm, y + size), p1: Vec2::new(x, y + size), color, width: w });
    cmds.push(DrawCmd::StrokeLine { p0: Vec2::new(x, y + size), p1: Vec2::new(x, y + size - arm), color, width: w });

    // Animated segment on the top edge — drawn as a rust highlight.
    let seg_w = size * 0.4;
    cmds.push(DrawCmd::StrokeLine {
        p0:    Vec2::new(x + size * 0.1, y),
        p1:    Vec2::new(x + size * 0.1 + seg_w, y),
        color: t.rust,
        width: w,
    });

    SpinnerResult { draw: cmds }
}
