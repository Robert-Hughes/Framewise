use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

// ── Spec ─────────────────────────────────────────────────────────────────────

pub struct LabelSpec {
    pub rect: Rect,
    /// Placeholder — text rendering is not yet implemented.
    pub text: String,
    /// Colour used for the text-stub rectangle.
    pub text_color: Color,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct LabelResult {
    pub draw:   DrawCommands,
    pub layout: LayoutInfo,
}

pub struct LabelInfo {
    pub layout: LayoutInfo,
}

impl WidgetResult for LabelResult {
    type Info = LabelInfo;

    fn into_parts(self) -> (DrawCommands, LabelInfo) {
        (self.draw, LabelInfo { layout: self.layout })
    }
}

// ── Widget function ───────────────────────────────────────────────────────────

/// Produce a label widget.
///
/// Text rendering is not yet implemented; this emits a `TextStub` draw command
/// so the API shape and cost model are established without a font dependency.
pub fn label(spec: LabelSpec) -> LabelResult {
    let mut draw = DrawCommands::new();

    // Stub: render a flat tinted rect where the text would appear.
    draw.push(DrawCmd::TextStub {
        rect:  spec.rect,
        color: spec.text_color,
    });

    LabelResult {
        draw,
        layout: LayoutInfo::tight(spec.rect),
    }
}
