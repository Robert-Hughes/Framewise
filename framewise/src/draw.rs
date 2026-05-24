use crate::text::TextHandle;
use crate::types::{Color, Rect, Vec2};

/// A single drawing instruction produced by a widget.
///
/// Draw commands are backend-agnostic. The renderer in the application crate
/// is responsible for turning them into GPU calls. Commands must be executed
/// in order; later commands appear visually above earlier ones.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    /// Fill a rectangle with a solid colour.
    FillRect { rect: Rect, color: Color },

    /// Draw the outline of a rectangle.
    StrokeRect { rect: Rect, color: Color, width: f32 },

    /// Draw a straight line segment.
    StrokeLine { p0: Vec2, p1: Vec2, color: Color, width: f32 },

    /// Fill a circle with a solid colour.
    FillCircle { center: Vec2, radius: f32, color: Color },

    /// Draw the outline of a circle.
    StrokeCircle { center: Vec2, radius: f32, color: Color, width: f32 },

    /// Draw a piece of prepared text.
    Text {
        rect: Rect,
        color: Color,
        handle: TextHandle,
    },
    PushClip {
        rect: Rect,
    },
    PopClip,
}

/// An ordered list of draw commands produced by one widget call.
///
/// Widgets build a `DrawCommands` value and return it as part of their result.
/// The `Builder` accumulates all commands into a single flat `Vec<DrawCmd>` for
/// the renderer.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawCommands(pub Vec<DrawCmd>);

impl DrawCommands {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, cmd: DrawCmd) {
        self.0.push(cmd);
    }

    pub fn extend(&mut self, other: DrawCommands) {
        self.0.extend(other.0);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
