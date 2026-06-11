use crate::text::TextHandle;
use crate::types::{Color, Rect, Vec2};

/// A single drawing instruction produced by a widget.
///
/// Draw commands are backend-agnostic. The renderer in the application crate
/// is responsible for turning them into GPU calls. Commands with higher `z`
/// values are drawn above lower `z` values; commands with equal `z` preserve
/// append order, so later commands appear visually above earlier ones.
#[derive(Debug, Clone, PartialEq)]
pub enum DrawCmd {
    /// Fill a rectangle with a solid colour.
    FillRect {
        rect: Rect,
        color: Color,
        z: u32,
        anti_alias: bool,
    },

    /// Draw the outline of a rectangle.
    StrokeRect {
        rect: Rect,
        color: Color,
        width: f32,
        z: u32,
        anti_alias: bool,
    },

    /// Draw a straight line segment.
    ///
    /// The line is drawn using "butt end caps", meaning the stroke terminates flat and
    /// stops immediately at `p0` and `p1` without projecting past them. For connected
    /// line segments to meet cleanly at corners, their endpoints should be manually
    /// extended or overlapped in the widget layout.
    StrokeLine {
        p0: Vec2,
        p1: Vec2,
        color: Color,
        width: f32,
        z: u32,
        anti_alias: bool,
    },

    /// Fill a circle with a solid colour.
    FillCircle {
        center: Vec2,
        radius: f32,
        color: Color,
        z: u32,
        anti_alias: bool,
    },

    /// Draw the outline of a circle.
    StrokeCircle {
        center: Vec2,
        radius: f32,
        color: Color,
        width: f32,
        z: u32,
        anti_alias: bool,
    },

    /// Draw a piece of prepared text.
    Text {
        rect: Rect,
        color: Color,
        handle: TextHandle,
        z: u32,
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
///
/// # Index Stability Invariant
/// Draw commands are strictly appended in order of evaluation. There is no reordering
/// or removal of commands during a layout pass. This stability is critical for container
/// widgets (like `frame`) that push placeholder draw commands at the start of execution
/// and retroactively patch their geometry via stable indices at the end of the pass.
///
/// To enforce this index stability invariant, `DrawCommands` does not expose public APIs
/// that could remove, clear, truncate, or reorder elements (such as `clear`, `remove`,
/// `truncate`, `swap`, or a `DerefMut` implementation).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DrawCommands(Vec<DrawCmd>);

impl DrawCommands {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    /// Build a command list from an existing vector. Useful for callers that
    /// assemble a batch of commands up front (e.g. demo/sample code).
    pub fn from_vec(cmds: Vec<DrawCmd>) -> Self {
        Self(cmds)
    }

    pub fn push(&mut self, cmd: DrawCmd) {
        self.0.push(cmd);
    }

    pub fn extend(&mut self, other: impl IntoIterator<Item = DrawCmd>) {
        self.0.extend(other);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of draw commands in the list.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Mutably borrows a single draw command by its index.
    ///
    /// This is the only way to modify an existing command. Since it only allows
    /// mutating the content of a single command and does not permit removal or
    /// reordering, it preserves index stability.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DrawCmd> {
        self.0.get_mut(index)
    }
}

impl std::ops::Deref for DrawCommands {
    type Target = [DrawCmd];
    fn deref(&self) -> &[DrawCmd] {
        &self.0
    }
}

impl IntoIterator for DrawCommands {
    type Item = DrawCmd;
    type IntoIter = std::vec::IntoIter<DrawCmd>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
