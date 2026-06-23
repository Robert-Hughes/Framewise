// Core geometric and colour types.
//
// These are plain data structs with no dependencies. All coordinates are in
// logical pixels (f32) with the origin at the top-left of the window.

// ── ClipRect ─────────────────────────────────────────────────────────────────

/// The clipping region passed to a widget spec or builder.
///
/// `None` means **no clipping** — the widget is fully visible and all of its
/// area participates in hit-testing. `Some(rect)` means the widget is clipped
/// to `rect`: only the intersection is visible and only mouse positions inside
/// `rect` count as hover/click events.
pub type ClipRect = Option<Rect>;

// ── Vec2 ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

// ── Rect ─────────────────────────────────────────────────────────────────────

/// An axis-aligned rectangle, stored as origin + size.
///
/// Rectangles represent continuous logical pixel coordinates. Origin is top-left,
/// and `right()` and `bottom()` are exclusive boundaries: `[x, x + w) × [y, y + h)`.
/// This half-open convention ensures that adjacent rectangles tiled side-by-side
/// do not overlap and have unambiguous hit-testing coverage.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Rect = Rect {
        x: 0.0,
        y: 0.0,
        w: 0.0,
        h: 0.0,
    };

    /// A deliberately-invalid (NaN) rect used as a placeholder when a `*Spec` is
    /// built only to calculate its size request — before the real geometry is
    /// known. Size-request calculations must not read the rect; any arithmetic
    /// on this value produces NaN, making accidental use loud rather than silent.
    pub const PLACEHOLDER: Rect = Rect {
        x: f32::NAN,
        y: f32::NAN,
        w: f32::NAN,
        h: f32::NAN,
    };

    /// A provisional rect whose origin is known but whose extent is not yet
    /// resolved — the state a deferred-own-size container (e.g. a `Frame` that
    /// sizes to its children) is in at `begin`. `w`/`h` are NaN so accidental
    /// extent use is loud rather than silent; the placeholder draw commands
    /// stamped with this rect are patched with the real bounds at `end`.
    ///
    /// Distinct from [`PLACEHOLDER`](Self::PLACEHOLDER) (all-NaN), which marks a
    /// rect not resolved *at all* — the leaf `size_*` case, where
    /// even the origin is unknown before the layout step runs.
    pub const fn pending_extent(x: f32, y: f32) -> Self {
        Self {
            x,
            y,
            w: f32::NAN,
            h: f32::NAN,
        }
    }

    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    /// Construct from left / top / right / bottom edges.
    pub fn from_ltrb(l: f32, t: f32, r: f32, b: f32) -> Self {
        Self {
            x: l,
            y: t,
            w: r - l,
            h: b - t,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }
    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }

    pub fn top_left(&self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
    pub fn bottom_right(&self) -> Vec2 {
        Vec2::new(self.right(), self.bottom())
    }
    pub fn center(&self) -> Vec2 {
        Vec2::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    /// Returns true if `pos` falls inside this rect.
    ///
    /// Rectangles are treated as half-open regions:
    /// `[x, x + w) × [y, y + h)`.
    /// Points on the left/top edges are inside; points on the right/bottom
    /// edges are outside. Empty or zero-size rects contain no points.
    pub fn contains(&self, pos: Vec2) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && pos.x >= self.x
            && pos.x < self.right()
            && pos.y >= self.y
            && pos.y < self.bottom()
    }

    /// Shrink the rect by `amount` on all sides.
    pub fn inset(&self, amount: f32) -> Self {
        Self {
            x: self.x + amount,
            y: self.y + amount,
            w: (self.w - amount * 2.0).max(0.0),
            h: (self.h - amount * 2.0).max(0.0),
        }
    }

    /// Computes the intersection of this rect with another.
    pub fn intersect(&self, other: &Self) -> Self {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Self {
            x,
            y,
            w: (right - x).max(0.0),
            h: (bottom - y).max(0.0),
        }
    }
}

// ── Color ─────────────────────────────────────────────────────────────────────

/// Linear-light RGBA colour with components in [0.0, 1.0].
///
/// All components are in **linear** (physically-based) light space, not
/// perceptual/sRGB space. This matches the expectation of a GPU pipeline
/// using an sRGB framebuffer: the hardware gamma-encodes the linear values
/// on output, so vertex and uniform colours must already be linear.
///
/// ## Constructing colours
///
/// | Source                     | Constructor              |
/// |----------------------------|--------------------------|
/// | Hex code / `u8` RGB        | [`Color::from_srgb_u8`]  |
/// | `0xRRGGBB` hex literal     | [`Color::from_srgb_hex`] |
/// | f32 values typed as sRGB   | [`Color::from_srgb_f32`] |
/// | Already-linear f32 values  | [`Color::linear_rgba`]   |
///
/// **Alpha is never gamma-encoded** — all constructors treat it as linear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

fn srgb_to_linear(x: f32) -> f32 {
    if x <= 0.04045 {
        x / 12.92
    } else {
        ((x + 0.055) / 1.055).powf(2.4)
    }
}

impl Color {
    /// Construct from **linear** RGBA components.
    pub const fn linear_rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Construct from **linear** RGB components with full opacity.
    pub const fn linear_rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    pub const BLACK: Color = Color::linear_rgb(0.0, 0.0, 0.0);
    pub const WHITE: Color = Color::linear_rgb(1.0, 1.0, 1.0);
    pub const TRANSPARENT: Color = Color::linear_rgba(0.0, 0.0, 0.0, 0.0);

    /// Construct from sRGB `u8` components (0–255).
    ///
    /// RGB channels are decoded from sRGB to linear light. Alpha is linear
    /// (255 = fully opaque).
    pub fn from_srgb_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: srgb_to_linear(r as f32 / 255.0),
            g: srgb_to_linear(g as f32 / 255.0),
            b: srgb_to_linear(b as f32 / 255.0),
            a: a as f32 / 255.0,
        }
    }

    /// Construct from sRGB f32 components (0.0–1.0).
    ///
    /// RGB channels are decoded from sRGB to linear light. Alpha is linear
    /// (1.0 = fully opaque).
    pub fn from_srgb_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            r: srgb_to_linear(r),
            g: srgb_to_linear(g),
            b: srgb_to_linear(b),
            a,
        }
    }

    /// Construct from a 6-digit sRGB hex literal (e.g. `Color::from_srgb_hex(0x15130f)`).
    pub fn from_srgb_hex(hex: u32) -> Self {
        Self::from_srgb_u8(
            ((hex >> 16) & 0xFF) as u8,
            ((hex >> 8) & 0xFF) as u8,
            (hex & 0xFF) as u8,
            255,
        )
    }

    /// Blend towards `other` by `t` (0.0 = self, 1.0 = other). Linear space.
    pub fn lerp(&self, other: Color, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Composite a foreground color over an opaque background and return an opaque result.
    /// Since Color stores linear RGB values, composite in linear space.
    pub fn with_alpha_over(self, bg: Color, alpha: f32) -> Color {
        let a = alpha.clamp(0.0, 1.0);
        Color::linear_rgba(
            self.r * a + bg.r * (1.0 - a),
            self.g * a + bg.g * (1.0 - a),
            self.b * a + bg.b * (1.0 - a),
            1.0,
        )
    }

    /// Multiply RGB by `factor` (brightness adjustment, clamped to [0, 1]). Linear space.
    pub fn darken(&self, factor: f32) -> Self {
        Self {
            r: (self.r * factor).clamp(0.0, 1.0),
            g: (self.g * factor).clamp(0.0, 1.0),
            b: (self.b * factor).clamp(0.0, 1.0),
            a: self.a,
        }
    }
}

// ── Layer ────────────────────────────────────────────────────────────────────

/// A layer concept to control depth sorting of drawn commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Layer {}

impl Layer {
    pub fn get_z(&self) -> u32 {
        0
    }

    pub fn get_focus_z(&self) -> u32 {
        1
    }
}

// ── Stroke & Outline ──────────────────────────────────────────────────────────

/// A solid stroke specification representing a visible line drawn on geometry.
/// Used for borders, separators, dividers, checkmarks, slider thumb borders, rules, etc.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stroke {
    pub color: Color,
    pub width: f32,
}

/// An outline specification representing a stroke drawn offset from a target shape.
///
/// An `Outline` is not just "a border"; it represents a focus ring or highlight
/// style that has a visual gap/offset from the shape itself.
///
/// Positive offsets move the outline outward / away from the target shape.
/// Negative offsets move it inward / into the target shape.
///
/// - Outline is a focus/highlight style with a gap/offset from a target shape.
/// - Positive offset moves outward, negative offset moves inward.
/// - For rectangular focus rings, call sites should usually draw:
///
/// ```text
/// cmds.push_border_rect(
///     rect.inset(-outline.offset),
///     Some(outline.stroke),
///     BorderPlacement::Outside,
///     z,
/// );
/// ```
///
/// or equivalent geometry-specific logic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    pub stroke: Stroke,
    pub offset: f32,
}

impl Stroke {
    pub const fn new(color: Color, width: f32) -> Self {
        Self { color, width }
    }
    pub const fn solid(color: Color, width: f32) -> Self {
        Self { color, width }
    }
    pub fn is_visible(self) -> bool {
        self.width > 0.0 && self.color.a > 0.0
    }
}

impl Outline {
    pub const fn new(color: Color, width: f32, offset: f32) -> Self {
        Self {
            stroke: Stroke::new(color, width),
            offset,
        }
    }
    pub const fn from_stroke(stroke: Stroke, offset: f32) -> Self {
        Self { stroke, offset }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rect_contains() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        // A point inside the rect returns true.
        assert!(rect.contains(Vec2::new(50.0, 45.0)));
        assert!(rect.contains(Vec2::new(15.0, 25.0)));

        // Left/top edges are inside.
        assert!(rect.contains(Vec2::new(10.0, 20.0))); // top-left corner
        assert!(rect.contains(Vec2::new(10.0, 45.0))); // left edge
        assert!(rect.contains(Vec2::new(50.0, 20.0))); // top edge

        // Right/bottom edges are outside.
        assert!(!rect.contains(Vec2::new(110.0, 45.0))); // right edge
        assert!(!rect.contains(Vec2::new(50.0, 70.0))); // bottom edge
        assert!(!rect.contains(Vec2::new(110.0, 70.0))); // bottom-right corner

        // Outside points.
        assert!(!rect.contains(Vec2::new(9.9, 19.9)));
        assert!(!rect.contains(Vec2::new(110.1, 70.1)));

        // Shared edge between adjacent rects
        let r1 = Rect::new(0.0, 0.0, 10.0, 10.0);
        let r2 = Rect::new(10.0, 0.0, 10.0, 10.0);
        let p = Vec2::new(10.0, 5.0);
        assert!(!r1.contains(p));
        assert!(r2.contains(p));

        // Zero-width and zero-height rects contain no points.
        let r_zero_w = Rect::new(10.0, 20.0, 0.0, 50.0);
        assert!(!r_zero_w.contains(Vec2::new(10.0, 45.0)));
        let r_zero_h = Rect::new(10.0, 20.0, 100.0, 0.0);
        assert!(!r_zero_h.contains(Vec2::new(50.0, 20.0)));
        let r_zero_both = Rect::new(10.0, 20.0, 0.0, 0.0);
        assert!(!r_zero_both.contains(Vec2::new(10.0, 20.0)));
    }
}
