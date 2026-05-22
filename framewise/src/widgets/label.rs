use crate::{
    draw::{DrawCmd, DrawCommands},
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetResult},
};

// ── Spec ─────────────────────────────────────────────────────────────────────

pub struct LabelSpec {
    pub rect: Rect,
    pub text: String,
    pub size: f32,
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
pub fn label<T: TextSystem>(spec: LabelSpec, text_sys: &mut T) -> LabelResult {
    let mut draw = DrawCommands::new();

    let layout = text_sys.prepare(&spec.text, spec.size);

    draw.push(DrawCmd::Text {
        rect:  spec.rect,
        color: spec.text_color,
        handle: layout.handle,
    });

    LabelResult {
        draw,
        layout: LayoutInfo::tight(spec.rect),
    }
}
