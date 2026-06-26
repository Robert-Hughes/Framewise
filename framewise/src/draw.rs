use crate::types::{Color, Rect, Stroke, Vec2};
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

/// Placement of a border relative to a rectangle's boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderPlacement {
    /// Draw the border inside the rectangle.
    Inside,
    /// Draw the border immediately outside the rectangle.
    Outside,
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
    ///
    /// `FillRect` is box/UI geometry that describes an occupied rectangular area.
    /// The renderer automatically applies antialiasing: non-AA quads are used when
    /// edges are pixel-aligned, and AA rect rendering otherwise.
    FillRect {
        rect: Rect,
        color: Color,
        z: u32,
    },

    /// Draw a box border around a rectangle.
    ///
    /// `BorderRect` is box/UI border geometry, not a vector stroke around a rectangle path.
    /// The renderer lowers it to four rectangular border strips and applies the same
    /// automatic antialiasing policy as `FillRect`.
    ///
    /// - `Inside` draws the border inside the rectangle.
    /// - `Outside` draws the border immediately outside the rectangle.
    /// - Corners may overlap for same-colour borders.
    BorderRect {
        rect: Rect,
        color: Color,
        width: f32,
        placement: BorderPlacement,
        z: u32,
    },

    /// Draw a straight line segment.
    ///
    /// `StrokeLine` is an antialiased vector line segment (always uses analytical AA).
    /// Use `FillRect` for crisp horizontal/vertical UI rules.
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
    },

    /// Fill a circle with a solid colour.
    ///
    /// `FillCircle` is antialiased vector geometry (always uses analytical AA).
    FillCircle {
        center: Vec2,
        radius: f32,
        color: Color,
        z: u32,
    },

    /// Draw the outline of a circle.
    ///
    /// `StrokeCircle` is antialiased vector geometry (always uses analytical AA).
    StrokeCircle {
        center: Vec2,
        radius: f32,
        color: Color,
        width: f32,
        z: u32,
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
#[derive(Debug, Clone, PartialEq)]
pub struct DrawCommands {
    cmds: Vec<DrawCmd>,
    glyphs: Vec<DrawGlyph>,
    physical_pixels_per_logical_pixel: f32,
}

impl DrawCommands {
    pub fn new(physical_pixels_per_logical_pixel: f32) -> Self {
        Self {
            cmds: Vec::new(),
            glyphs: Vec::new(),
            physical_pixels_per_logical_pixel: sanitise_physical_pixels_per_logical_pixel(
                physical_pixels_per_logical_pixel,
            ),
        }
    }

    /// Build a command list from an existing vector. Useful for callers that
    /// assemble a batch of commands up front (e.g. demo/sample code).
    ///
    /// `GlyphRun` commands should be created with [`push_glyph_run`](Self::push_glyph_run)
    /// so their ranges point at this command list's glyph arena.
    pub fn from_vec(cmds: Vec<DrawCmd>) -> Self {
        Self::from_vec_new(cmds, 1.0)
    }

    pub fn from_vec_new(cmds: Vec<DrawCmd>, physical_pixels_per_logical_pixel: f32) -> Self {
        Self {
            cmds,
            glyphs: Vec::new(),
            physical_pixels_per_logical_pixel: sanitise_physical_pixels_per_logical_pixel(
                physical_pixels_per_logical_pixel,
            ),
        }
    }

    pub fn physical_pixels_per_logical_pixel(&self) -> f32 {
        self.physical_pixels_per_logical_pixel
    }

    pub fn set_physical_pixels_per_logical_pixel(&mut self, scale: f32) {
        self.physical_pixels_per_logical_pixel = sanitise_physical_pixels_per_logical_pixel(scale);
    }

    pub fn device_pixel_size(&self) -> f32 {
        1.0 / self.physical_pixels_per_logical_pixel
    }

    pub fn snap_to_physical_pixel(&self, logical: f32) -> f32 {
        let scale = self.physical_pixels_per_logical_pixel;
        (logical * scale).round() / scale
    }

    pub fn floor_to_physical_pixel(&self, logical: f32) -> f32 {
        let scale = self.physical_pixels_per_logical_pixel;
        (logical * scale).floor() / scale
    }

    pub fn ceil_to_physical_pixel(&self, logical: f32) -> f32 {
        let scale = self.physical_pixels_per_logical_pixel;
        (logical * scale).ceil() / scale
    }

    pub fn is_physical_pixel_aligned(&self, logical: f32) -> bool {
        const EPSILON: f32 = 0.001;
        let physical = logical * self.physical_pixels_per_logical_pixel;
        (physical - physical.round()).abs() <= EPSILON
    }

    pub fn snap_rect_edges_to_physical_pixel(&self, rect: Rect) -> Rect {
        let l = self.snap_to_physical_pixel(rect.x);
        let t = self.snap_to_physical_pixel(rect.y);
        let r = self.snap_to_physical_pixel(rect.right());
        let b = self.snap_to_physical_pixel(rect.bottom());

        Rect::from_ltrb(l, t, r.max(l), b.max(t))
    }

    pub fn floor_ceil_rect_to_physical_pixel(&self, rect: Rect) -> Rect {
        let l = self.floor_to_physical_pixel(rect.x);
        let t = self.floor_to_physical_pixel(rect.y);
        let r = self.ceil_to_physical_pixel(rect.right());
        let b = self.ceil_to_physical_pixel(rect.bottom());

        Rect::from_ltrb(l, t, r.max(l), b.max(t))
    }

    pub fn snap_length_to_physical_pixels(&self, logical_len: f32) -> f32 {
        let px = self.device_pixel_size();
        let physical = logical_len * self.physical_pixels_per_logical_pixel;
        physical.round().max(1.0) * px
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

    pub(crate) fn reserve_glyphs(&mut self, additional: usize) {
        self.glyphs.reserve(additional);
    }

    pub(crate) fn reserve_commands(&mut self, additional: usize) {
        self.cmds.reserve(additional);
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
        debug_assert!(
            (self.physical_pixels_per_logical_pixel - other.physical_pixels_per_logical_pixel)
                .abs()
                <= 0.001,
            "DrawCommands scale mismatch: {} vs {}",
            self.physical_pixels_per_logical_pixel,
            other.physical_pixels_per_logical_pixel
        );

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

    pub fn push_border_rect(
        &mut self,
        rect: Rect,
        stroke: Option<Stroke>,
        placement: BorderPlacement,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() {
            return None;
        }

        Some(self.push(DrawCmd::BorderRect {
            rect,
            color: s.color,
            width: s.width,
            placement,
            z,
        }))
    }

    pub fn push_crisp_fill_rect(&mut self, rect: Rect, color: Color, z: u32) -> usize {
        let rect = self.snap_rect_edges_to_physical_pixel(rect);
        self.push(DrawCmd::FillRect { rect, color, z })
    }

    pub fn push_crisp_border_rect(
        &mut self,
        rect: Rect,
        stroke: Option<Stroke>,
        placement: BorderPlacement,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() {
            return None;
        }

        let rect = self.snap_rect_edges_to_physical_pixel(rect);
        let width = self.snap_length_to_physical_pixels(s.width);

        Some(self.push(DrawCmd::BorderRect {
            rect,
            color: s.color,
            width,
            placement,
            z,
        }))
    }

    pub fn push_device_hairline_h(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        color: Color,
        z: u32,
    ) -> Option<usize> {
        let px = self.device_pixel_size();
        let x0 = self.snap_to_physical_pixel(x);
        let x1 = self.snap_to_physical_pixel(x + width);
        let y0 = self.snap_to_physical_pixel(y);

        if x1 <= x0 {
            return None;
        }

        Some(self.push(DrawCmd::FillRect {
            rect: Rect::new(x0, y0, x1 - x0, px),
            color,
            z,
        }))
    }

    pub fn push_device_hairline_v(
        &mut self,
        x: f32,
        y: f32,
        height: f32,
        color: Color,
        z: u32,
    ) -> Option<usize> {
        let px = self.device_pixel_size();
        let x0 = self.snap_to_physical_pixel(x);
        let y0 = self.snap_to_physical_pixel(y);
        let y1 = self.snap_to_physical_pixel(y + height);

        if y1 <= y0 {
            return None;
        }

        Some(self.push(DrawCmd::FillRect {
            rect: Rect::new(x0, y0, px, y1 - y0),
            color,
            z,
        }))
    }

    pub fn push_stroke_line(
        &mut self,
        p0: Vec2,
        p1: Vec2,
        stroke: Option<Stroke>,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() {
            return None;
        }

        Some(self.push(DrawCmd::StrokeLine {
            p0,
            p1,
            color: s.color,
            width: s.width,
            z,
        }))
    }

    /// Push a horizontal UI rule/line segment drawn as a filled box (`FillRect`).
    ///
    /// The rule draws an exact occupied horizontal rectangular UI strip starting at `x`, `y`
    /// with length `width` and height `stroke.width`. It does not center a vector stroke.
    /// Callers are responsible for choosing aligned x/y when they want crisp output. Use
    /// [`push_device_hairline_h`](Self::push_device_hairline_h) for a one-physical-pixel rule.
    /// Returns the command index if a visible command was pushed.
    pub fn push_h_rule(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        stroke: Option<Stroke>,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() || width <= 0.0 {
            return None;
        }

        Some(self.push(DrawCmd::FillRect {
            rect: Rect::new(x, y, width, s.width),
            color: s.color,
            z,
        }))
    }

    /// Push a vertical UI rule/line segment drawn as a filled box (`FillRect`).
    ///
    /// The rule draws an exact occupied vertical rectangular UI strip starting at `x`, `y`
    /// with width `stroke.width` and length `height`. It does not center a vector stroke.
    /// Callers are responsible for choosing aligned x/y when they want crisp output. Use
    /// [`push_device_hairline_v`](Self::push_device_hairline_v) for a one-physical-pixel rule.
    /// Returns the command index if a visible command was pushed.
    pub fn push_v_rule(
        &mut self,
        x: f32,
        y: f32,
        height: f32,
        stroke: Option<Stroke>,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() || height <= 0.0 {
            return None;
        }

        Some(self.push(DrawCmd::FillRect {
            rect: Rect::new(x, y, s.width, height),
            color: s.color,
            z,
        }))
    }

    pub fn push_stroke_circle(
        &mut self,
        center: Vec2,
        radius: f32,
        stroke: Option<Stroke>,
        z: u32,
    ) -> Option<usize> {
        let s = stroke?;
        if !s.is_visible() {
            return None;
        }

        Some(self.push(DrawCmd::StrokeCircle {
            center,
            radius,
            color: s.color,
            width: s.width,
            z,
        }))
    }
}

fn sanitise_physical_pixels_per_logical_pixel(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.max(0.01)
    } else {
        1.0
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
    fn draw_commands_scale_sanitises() {
        let mut cmds = DrawCommands::new(1.0);
        assert_eq!(cmds.physical_pixels_per_logical_pixel(), 1.0);

        cmds.set_physical_pixels_per_logical_pixel(2.0);
        assert_eq!(cmds.physical_pixels_per_logical_pixel(), 2.0);
        assert_eq!(
            DrawCommands::new(0.0).physical_pixels_per_logical_pixel(),
            0.01
        );
        assert_eq!(
            DrawCommands::new(f32::NAN).physical_pixels_per_logical_pixel(),
            1.0
        );
        assert_eq!(
            DrawCommands::from_vec_new(Vec::new(), 1.5).physical_pixels_per_logical_pixel(),
            1.5
        );
    }

    #[test]
    fn physical_pixel_grid_helpers_snap_in_logical_units() {
        let one_x = DrawCommands::new(1.0);
        assert_eq!(one_x.device_pixel_size(), 1.0);
        assert_eq!(one_x.snap_to_physical_pixel(10.5), 11.0);
        assert!(!one_x.is_physical_pixel_aligned(10.5));

        let two_x = DrawCommands::new(2.0);
        assert_eq!(two_x.device_pixel_size(), 0.5);
        assert_eq!(two_x.snap_to_physical_pixel(10.5), 10.5);
        assert!(two_x.is_physical_pixel_aligned(10.5));

        let one_and_quarter = DrawCommands::new(1.25);
        assert!((one_and_quarter.device_pixel_size() - 0.8).abs() < 0.0001);
        assert!((one_and_quarter.snap_to_physical_pixel(1.0) - 0.8).abs() < 0.0001);

        let one_and_half = DrawCommands::new(1.5);
        assert!((one_and_half.device_pixel_size() - (2.0 / 3.0)).abs() < 0.0001);
        assert!((one_and_half.snap_to_physical_pixel(1.0) - (4.0 / 3.0)).abs() < 0.0001);
    }

    #[test]
    fn physical_pixel_rect_helpers_snap_edges() {
        let cmds = DrawCommands::new(2.0);
        let rect = Rect::new(0.24, 0.26, 10.49, 5.49);

        assert_eq!(
            cmds.snap_rect_edges_to_physical_pixel(rect),
            Rect::from_ltrb(0.0, 0.5, 10.5, 6.0)
        );
        assert_eq!(
            cmds.floor_ceil_rect_to_physical_pixel(rect),
            Rect::from_ltrb(0.0, 0.0, 11.0, 6.0)
        );
    }

    #[test]
    fn push_glyph_run_stores_glyphs_and_command_range() {
        let mut cmds = DrawCommands::new(1.0);

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
        let mut first = DrawCommands::new(1.0);
        first.push_glyph_run([glyph(1, 0.0), glyph(2, 8.0)], color(), 1);

        let mut second = DrawCommands::new(1.0);
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
        let mut cmds = DrawCommands::new(1.0);

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
        let mut cmds = DrawCommands::new(1.0);

        let index = cmds.push_glyph_run([], color(), 1);

        assert_eq!(index, None);
        assert!(cmds.commands().is_empty());
        assert!(cmds.glyphs().is_empty());
    }

    #[test]
    fn test_push_stroke_helpers() {
        let mut cmds = DrawCommands::new(1.0);
        let s_valid = Stroke::new(color(), 2.0);

        // Smoke test for push_stroke_line
        let index_line =
            cmds.push_stroke_line(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0), Some(s_valid), 1);
        assert_eq!(index_line, Some(0));
        assert_eq!(cmds.len(), 1);

        // Smoke test for push_stroke_circle
        let index_circle = cmds.push_stroke_circle(Vec2::new(0.0, 0.0), 5.0, Some(s_valid), 1);
        assert_eq!(index_circle, Some(1));
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn test_push_border_rect_helpers() {
        let mut cmds = DrawCommands::new(1.0);
        let r = Rect::new(0.0, 0.0, 10.0, 10.0);
        let s_valid = Stroke::new(color(), 2.0);

        // 1. push_border_rect returns Some(index) when it pushes Inside.
        let index_inside = cmds.push_border_rect(r, Some(s_valid), BorderPlacement::Inside, 1);
        assert_eq!(index_inside, Some(0));
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds.commands()[0],
            DrawCmd::BorderRect {
                rect: r,
                color: color(),
                width: 2.0,
                placement: BorderPlacement::Inside,
                z: 1,
            }
        );

        // 2. push_border_rect returns Some(index) when it pushes Outside.
        let index_outside = cmds.push_border_rect(r, Some(s_valid), BorderPlacement::Outside, 2);
        assert_eq!(index_outside, Some(1));
        assert_eq!(cmds.len(), 2);
        assert_eq!(
            cmds.commands()[1],
            DrawCmd::BorderRect {
                rect: r,
                color: color(),
                width: 2.0,
                placement: BorderPlacement::Outside,
                z: 2,
            }
        );

        // 3. push_border_rect returns None and does not push for None.
        let index_none = cmds.push_border_rect(r, None, BorderPlacement::Inside, 1);
        assert_eq!(index_none, None);
        assert_eq!(cmds.len(), 2);

        // 4. push_border_rect returns None and does not push for Stroke { width: 0.0, ... }.
        let s_zero_width = Stroke::new(color(), 0.0);
        let index_zero_width =
            cmds.push_border_rect(r, Some(s_zero_width), BorderPlacement::Inside, 1);
        assert_eq!(index_zero_width, None);
        assert_eq!(cmds.len(), 2);

        // 5. push_border_rect returns None and does not push for alpha-zero color.
        let transparent_color = Color::from_srgb_u8(1, 2, 3, 0);
        let s_transparent = Stroke::new(transparent_color, 2.0);
        let index_transparent =
            cmds.push_border_rect(r, Some(s_transparent), BorderPlacement::Inside, 1);
        assert_eq!(index_transparent, None);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn test_push_rule_helpers() {
        let mut cmds = DrawCommands::new(1.0);
        let s_valid = Stroke::new(color(), 2.0);

        // 1. push_h_rule visible horizontal rule emits FillRect
        let index_h = cmds.push_h_rule(10.0, 20.0, 50.0, Some(s_valid), 1);
        assert_eq!(index_h, Some(0));
        assert_eq!(cmds.len(), 1);
        assert_eq!(
            cmds.commands()[0],
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 20.0, 50.0, 2.0),
                color: color(),
                z: 1,
            }
        );

        // 2. push_v_rule visible vertical rule emits FillRect
        let index_v = cmds.push_v_rule(10.0, 20.0, 50.0, Some(s_valid), 2);
        assert_eq!(index_v, Some(1));
        assert_eq!(cmds.len(), 2);
        assert_eq!(
            cmds.commands()[1],
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 20.0, 2.0, 50.0),
                color: color(),
                z: 2,
            }
        );

        // 3. None stroke returns None and does not push
        assert_eq!(cmds.push_h_rule(10.0, 20.0, 50.0, None, 1), None);
        assert_eq!(cmds.push_v_rule(10.0, 20.0, 50.0, None, 2), None);
        assert_eq!(cmds.len(), 2);

        // 4. Invisible stroke (zero width or transparent color) returns None and does not push
        let s_zero_width = Stroke::new(color(), 0.0);
        assert_eq!(
            cmds.push_h_rule(10.0, 20.0, 50.0, Some(s_zero_width), 1),
            None
        );
        assert_eq!(
            cmds.push_v_rule(10.0, 20.0, 50.0, Some(s_zero_width), 2),
            None
        );
        assert_eq!(cmds.len(), 2);

        // 5. Non-positive length (<= 0.0) returns None and does not push
        assert_eq!(cmds.push_h_rule(10.0, 20.0, 0.0, Some(s_valid), 1), None);
        assert_eq!(cmds.push_h_rule(10.0, 20.0, -10.0, Some(s_valid), 1), None);
        assert_eq!(cmds.push_v_rule(10.0, 20.0, 0.0, Some(s_valid), 2), None);
        assert_eq!(cmds.push_v_rule(10.0, 20.0, -10.0, Some(s_valid), 2), None);
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn crisp_fill_rect_snaps_at_one_x() {
        let mut cmds = DrawCommands::new(1.0);

        cmds.push_crisp_fill_rect(Rect::from_ltrb(0.25, 0.5, 10.49, 5.51), color(), 1);

        assert_eq!(
            cmds.commands(),
            &[DrawCmd::FillRect {
                rect: Rect::from_ltrb(0.0, 1.0, 10.0, 6.0),
                color: color(),
                z: 1,
            }]
        );
    }

    #[test]
    fn crisp_border_rect_snaps_at_one_x() {
        let mut cmds = DrawCommands::new(1.0);

        cmds.push_crisp_border_rect(
            Rect::from_ltrb(0.25, 0.5, 10.49, 5.51),
            Some(Stroke::new(color(), 1.4)),
            BorderPlacement::Inside,
            2,
        );

        assert_eq!(
            cmds.commands(),
            &[DrawCmd::BorderRect {
                rect: Rect::from_ltrb(0.0, 1.0, 10.0, 6.0),
                color: color(),
                width: 1.0,
                placement: BorderPlacement::Inside,
                z: 2,
            }]
        );
    }

    #[test]
    fn crisp_fill_rect_snaps_at_two_x() {
        let mut cmds = DrawCommands::new(2.0);

        // 0.25 * 2 = 0.5 → rounds to 0 → 0.0 (banker's round, but f32 round() rounds half away from zero)
        // Actually 0.5.round() = 1.0 in Rust, so 0.25 * 2 = 0.5 → 1 → 0.5 logical
        // Let's use unambiguous values: 0.24 → 0.48 → rounds to 0 → 0.0
        // 0.26 → 0.52 → rounds to 1 → 0.5
        // 10.49 → 20.98 → rounds to 21 → 10.5
        // 5.51 → 11.02 → rounds to 11 → 5.5
        cmds.push_crisp_fill_rect(Rect::from_ltrb(0.24, 0.26, 10.49, 5.51), color(), 1);

        assert_eq!(
            cmds.commands(),
            &[DrawCmd::FillRect {
                rect: Rect::from_ltrb(0.0, 0.5, 10.5, 5.5),
                color: color(),
                z: 1,
            }]
        );
    }

    #[test]
    fn crisp_border_rect_snaps_at_two_x() {
        let mut cmds = DrawCommands::new(2.0);

        cmds.push_crisp_border_rect(
            Rect::from_ltrb(0.24, 0.26, 10.49, 5.51),
            Some(Stroke::new(color(), 1.4)),
            BorderPlacement::Outside,
            3,
        );

        // 1.4 * 2 = 2.8 → round → 3 physical px → 1.5 logical
        assert_eq!(
            cmds.commands(),
            &[DrawCmd::BorderRect {
                rect: Rect::from_ltrb(0.0, 0.5, 10.5, 5.5),
                color: color(),
                width: 1.5,
                placement: BorderPlacement::Outside,
                z: 3,
            }]
        );
    }
}
