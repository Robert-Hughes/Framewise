use crate::draw::DrawCommands;
use crate::types::Rect;

// ── Common result fragments ───────────────────────────────────────────────────

/// Resolved geometry returned by every widget.
#[derive(Debug, Clone, Copy)]
pub struct LayoutInfo {
    /// The outer bounding box of the widget including any border / padding.
    pub bounds: Rect,
    /// The inner content area (inside any padding).
    pub content_bounds: Rect,
}

impl LayoutInfo {
    pub fn new(bounds: Rect, content_bounds: Rect) -> Self {
        Self { bounds, content_bounds }
    }

    /// Convenience: layout with identical outer and content bounds.
    pub fn tight(bounds: Rect) -> Self {
        Self { bounds, content_bounds: bounds }
    }
}

/// Pointer interaction state returned by interactive widgets.
#[derive(Debug, Clone, Copy, Default)]
pub struct InputInfo {
    /// True while the cursor is over the widget's bounds this frame.
    pub hovered: bool,
    /// True while the primary mouse button is held and the cursor is over the widget.
    pub pressed: bool,
    /// True on the single frame the primary button was released over the widget.
    pub clicked: bool,
}

// ── WidgetResult trait ────────────────────────────────────────────────────────

/// Every value returned by a low-level widget function implements this trait
/// so that `Builder::emit` can extract draw commands while returning the
/// app-facing info.
pub trait WidgetResult {
    /// The information returned to the caller after draw commands are extracted.
    type Info;

    /// Consume `self`, yielding the draw commands and the caller-facing info.
    fn into_parts(self) -> (DrawCommands, Self::Info);
}
