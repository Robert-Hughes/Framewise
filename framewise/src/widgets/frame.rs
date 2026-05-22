use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a frame (bordered background rectangle).
#[derive(Debug, Clone, Copy)]
pub struct FrameStyle {
    pub background:   Color,
    pub border:       Color,
    pub border_width: f32,
    /// Padding between the border and the content area.
    pub padding:      f32,
}

impl Default for FrameStyle {
    fn default() -> Self {
        Self {
            background:   Color::rgb(0.12, 0.12, 0.15),
            border:       Color::rgb(0.30, 0.30, 0.38),
            border_width: 1.0,
            padding:      4.0,
        }
    }
}

// ── Spec ──────────────────────────────────────────────────────────────────────

pub struct FrameSpec {
    pub rect:  Rect,
    pub style: FrameStyle,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct FrameResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct FrameInfo {
    pub layout: LayoutInfo,
}

impl FrameInfo {
    /// The content area inside the frame's border and padding.
    pub fn content_rect(&self) -> Rect { self.layout.content_bounds }
}

impl WidgetResult for FrameResult {
    type Info = FrameInfo;

    fn into_parts(self) -> (DrawCommands, FrameInfo) {
        (self.draw, FrameInfo { layout: self.layout })
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a frame widget — a bordered, filled background rectangle.
///
/// The returned `FrameInfo` exposes the inner `content_rect()` which callers
/// can use to place child widgets.
pub fn frame(spec: FrameSpec) -> FrameResult {
    let mut draw = DrawCommands::new();

    draw.push(DrawCmd::FillRect { rect: spec.rect, color: spec.style.background });

    if spec.style.border_width > 0.0 {
        draw.push(DrawCmd::StrokeRect {
            rect:  spec.rect,
            color: spec.style.border,
            width: spec.style.border_width,
        });
    }

    let inset = spec.style.border_width + spec.style.padding;
    let content = spec.rect.inset(inset);

    FrameResult {
        draw,
        layout: LayoutInfo::new(spec.rect, content),
    }
}
