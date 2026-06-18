use crate::types::{Color, Rect, Vec2};
use std::ops::Range;

/// An opaque backend/renderer-owned token for a renderer-ready glyph resource.
///
/// Framewise never inspects this value. It is not a character, text cluster,
/// font glyph id, or layout glyph id.
///
/// It represents something backend-specific, such as an atlas rect, atlas entry
/// index, font face, glyph id, size, weight, optical size, subpixel bin, and
/// raster mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PreparedGlyphToken(pub u64);

/// A single prepared glyph blit emitted into a draw command glyph arena.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawGlyph {
    pub token: PreparedGlyphToken,

    /// Final top-left position of the prepared glyph bitmap in draw-list coordinates.
    ///
    /// This is not the cluster position and not the text baseline origin. It is
    /// the bitmap/blit position after glyph bearings, layout position, line
    /// baseline, alignment, and caller draw origin have all been applied.
    ///
    /// The renderer resolves `token` to atlas/resource data, including bitmap
    /// size and UVs, and performs a no-scale blit at this position.
    pub top_left: Vec2,
}

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

    /// Draw a contiguous range of prepared glyphs from the `DrawCommands` glyph arena.
    GlyphRun {
        glyphs: Range<usize>,
        color: Color,
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
pub struct DrawCommands {
    cmds: Vec<DrawCmd>,
    glyphs: Vec<DrawGlyph>,
}

impl DrawCommands {
    pub fn new() -> Self {
        Self {
            cmds: Vec::new(),
            glyphs: Vec::new(),
        }
    }

    /// Build a command list from an existing vector. Useful for callers that
    /// assemble a batch of commands up front (e.g. demo/sample code).
    ///
    /// `GlyphRun` commands should be created with [`push_glyph_run`](Self::push_glyph_run)
    /// so their ranges point at this command list's glyph arena.
    pub fn from_vec(cmds: Vec<DrawCmd>) -> Self {
        Self {
            cmds,
            glyphs: Vec::new(),
        }
    }

    pub fn push(&mut self, cmd: DrawCmd) -> usize {
        let index = self.cmds.len();
        self.cmds.push(cmd);
        index
    }

    pub fn push_glyph_run<I>(&mut self, glyphs: I, color: Color, z: u32) -> Option<usize>
    where
        I: IntoIterator<Item = DrawGlyph>,
    {
        let start = self.glyphs.len();
        self.glyphs.extend(glyphs);
        let end = self.glyphs.len();
        let index = self.cmds.len();

        if start != end {
            self.cmds.push(DrawCmd::GlyphRun {
                glyphs: start..end,
                color,
                z,
            });
            Some(index)
        } else {
            None
        }
    }

    pub(crate) fn glyph_run_start(&self) -> usize {
        self.glyphs.len()
    }

    pub(crate) fn push_glyph(&mut self, glyph: DrawGlyph) {
        self.glyphs.push(glyph);
    }

    pub(crate) fn finish_glyph_run(&mut self, start: usize, color: Color, z: u32) -> Option<usize> {
        let end = self.glyphs.len();
        if start == end {
            return None;
        }

        let index = self.cmds.len();
        self.cmds.push(DrawCmd::GlyphRun {
            glyphs: start..end,
            color,
            z,
        });
        Some(index)
    }

    pub fn commands(&self) -> &[DrawCmd] {
        &self.cmds
    }

    pub fn glyphs(&self) -> &[DrawGlyph] {
        &self.glyphs
    }

    pub fn append(&mut self, mut other: DrawCommands) {
        let glyph_offset = self.glyphs.len();

        for cmd in &mut other.cmds {
            if let DrawCmd::GlyphRun { glyphs, .. } = cmd {
                glyphs.start += glyph_offset;
                glyphs.end += glyph_offset;
            }
        }

        self.glyphs.extend(other.glyphs);
        self.cmds.extend(other.cmds);
    }

    pub fn is_empty(&self) -> bool {
        self.cmds.is_empty()
    }

    /// Returns the number of draw commands in the list.
    pub fn len(&self) -> usize {
        self.cmds.len()
    }

    /// Mutably borrows a single draw command by its index.
    ///
    /// This is the only way to modify an existing command. Since it only allows
    /// mutating the content of a single command and does not permit removal or
    /// reordering, it preserves index stability.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut DrawCmd> {
        self.cmds.get_mut(index)
    }
}

impl std::ops::Deref for DrawCommands {
    type Target = [DrawCmd];
    fn deref(&self) -> &[DrawCmd] {
        &self.cmds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color() -> Color {
        Color::from_srgb_u8(1, 2, 3, 4)
    }

    fn glyph(handle: u32, x: f32) -> DrawGlyph {
        DrawGlyph {
            token: PreparedGlyphToken(handle as u64),
            top_left: Vec2::new(x, 2.0),
        }
    }

    #[test]
    fn push_glyph_run_stores_glyphs_and_command_range() {
        let mut cmds = DrawCommands::new();

        let index = cmds.push_glyph_run([glyph(10, 1.0), glyph(11, 9.0)], color(), 7);

        assert_eq!(index, Some(0));
        assert_eq!(cmds.glyphs(), &[glyph(10, 1.0), glyph(11, 9.0)]);
        assert_eq!(
            cmds.commands(),
            &[DrawCmd::GlyphRun {
                glyphs: 0..2,
                color: color(),
                z: 7,
            }]
        );
    }

    #[test]
    fn append_rebases_glyph_run_ranges() {
        let mut first = DrawCommands::new();
        first.push_glyph_run([glyph(1, 0.0), glyph(2, 8.0)], color(), 1);

        let mut second = DrawCommands::new();
        second.push(DrawCmd::PopClip);
        second.push_glyph_run([glyph(3, 16.0)], color(), 2);

        first.append(second);

        assert_eq!(
            first.glyphs(),
            &[glyph(1, 0.0), glyph(2, 8.0), glyph(3, 16.0)]
        );
        assert_eq!(
            first.commands(),
            &[
                DrawCmd::GlyphRun {
                    glyphs: 0..2,
                    color: color(),
                    z: 1,
                },
                DrawCmd::PopClip,
                DrawCmd::GlyphRun {
                    glyphs: 2..3,
                    color: color(),
                    z: 2,
                },
            ]
        );
    }

    #[test]
    fn command_indices_remain_stable() {
        let mut cmds = DrawCommands::new();

        let first = cmds.push(DrawCmd::PopClip);
        let second = cmds.push_glyph_run([glyph(1, 0.0)], color(), 1);
        let third = cmds.push(DrawCmd::PushClip {
            rect: Rect::new(1.0, 2.0, 3.0, 4.0),
        });

        assert_eq!((first, second, third), (0, Some(1), 2));
        assert!(matches!(cmds.get_mut(first), Some(DrawCmd::PopClip)));
    }

    #[test]
    fn empty_glyph_run_is_not_emitted() {
        let mut cmds = DrawCommands::new();

        let index = cmds.push_glyph_run([], color(), 1);

        assert_eq!(index, None);
        assert!(cmds.commands().is_empty());
        assert!(cmds.glyphs().is_empty());
    }
}
