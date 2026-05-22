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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_layout_and_draw() {
        let spec = FrameSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 50.0),
            style: FrameStyle {
                background: Color::rgb(1.0, 1.0, 1.0),
                border: Color::rgb(0.5, 0.5, 0.5),
                border_width: 2.0,
                padding: 3.0,
            },
        };
        
        let res = frame(spec);
        let (draw, info) = res.into_parts();
        
        // Bounds should be exactly the requested rect
        assert_eq!(info.layout.bounds.x, 10.0);
        assert_eq!(info.layout.bounds.y, 10.0);
        assert_eq!(info.layout.bounds.w, 100.0);
        assert_eq!(info.layout.bounds.h, 50.0);
        
        // Content rect should be inset by border_width + padding = 5.0
        let content = info.content_rect();
        assert_eq!(content.x, 15.0);
        assert_eq!(content.y, 15.0);
        assert_eq!(content.w, 90.0);
        assert_eq!(content.h, 40.0);
        
        // Should draw background and border
        assert_eq!(draw.0.len(), 2);
        match &draw.0[0] {
            DrawCmd::FillRect { rect, .. } => assert_eq!(rect.x, 10.0),
            _ => panic!("Expected FillRect"),
        }
        match &draw.0[1] {
            DrawCmd::StrokeRect { rect, width, .. } => {
                assert_eq!(rect.x, 10.0);
                assert_eq!(width, &2.0);
            }
            _ => panic!("Expected StrokeRect"),
        }
    }
}
