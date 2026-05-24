use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, text::TextSystem, theme::Theme, types::{Rect, Vec2}
};

pub struct SegmentedSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    /// Top-left origin. Height is fixed at h_md (28).
    pub rect:         Rect,
    pub items:        &'a [&'a str],
    pub active_index: usize,
    pub focused:      Option<usize>,
}

pub struct SegmentedResult {
    pub draw: DrawCommands,
}

impl WidgetResult for SegmentedResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn segmented<'a, T: crate::text::TextSystem>(spec: SegmentedSpec<'a, T>) -> SegmentedResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    if spec.items.is_empty() {
        return SegmentedResult { draw: cmds };
    }

    let h = t.h_md;
    let pad_x = 14.0_f32;

    // Pre-prepare all labels to get their widths.
    let layouts: Vec<_> = spec.items.iter().map(|s| spec.ts.prepare(s, t.text_md)).collect();
    let widths: Vec<f32> = layouts.iter().map(|l| l.size.x + pad_x * 2.0).collect();
    let total_w: f32 = widths.iter().sum();

    let outer = Rect::new(spec.rect.x, spec.rect.y, total_w, h);

    cmds.push(DrawCmd::FillRect { rect: outer, color: t.paper_elev });
    cmds.push(DrawCmd::StrokeRect { rect: outer, color: t.ink, width: 1.0 });

    let mut x = spec.rect.x;
    for (i, (layout, &w)) in layouts.iter().zip(widths.iter()).enumerate() {
        let is_active = i == spec.active_index;
        let seg_rect = Rect::new(x, spec.rect.y, w, h);

        if is_active {
            cmds.push(DrawCmd::FillRect { rect: seg_rect, color: t.ink });
        }

        // Focus ring (inset to stay within bounds).
        if spec.focused == Some(i) {
            cmds.push(DrawCmd::StrokeRect {
                rect:  seg_rect.inset(2.0),
                color: t.rust,
                width: 2.0,
            });
        }

        // Divider between segments (right edge, except last).
        if i + 1 < spec.items.len() {
            let div_x = x + w;
            cmds.push(DrawCmd::StrokeLine {
                p0:    Vec2::new(div_x, spec.rect.y),
                p1:    Vec2::new(div_x, spec.rect.y + h),
                color: t.ink,
                width: 1.0,
            });
        }

        let text_color = if is_active { t.paper } else { t.ink };
        let ty = spec.rect.y + (h - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect:   Rect::new(x + pad_x, ty, layout.size.x, layout.size.y),
            color:  text_color,
            handle: layout.handle,
        });

        x += w;
    }

    SegmentedResult { draw: cmds }
}




pub struct SegmentedSpecBuilder<'a, T: crate::text::TextSystem> {
    pub items: Option<&'a [&'a str]>,
    pub active_index: Option<usize>,
    pub focused: Option<Option<usize>>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> SegmentedSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            items: None,
            active_index: None,
            focused: None,
            rect: None,
            ts: None,
        }
    }

    pub fn items(mut self, items: &'a [&'a str]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn active_index(mut self, active_index: usize) -> Self {
        self.active_index = Some(active_index);
        self
    }
    pub fn focused(mut self, focused: Option<usize>) -> Self {
        self.focused = Some(focused);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for SegmentedSpecBuilder<'a, T> {
    type Spec = SegmentedSpec<'a, T>;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn with_text_system(mut self, ts: &'a mut T) -> Self {
        self.ts = Some(ts);
        self
    }

    fn build(self) -> Self::Spec {
        SegmentedSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            items: self.items.unwrap(),
            active_index: self.active_index.unwrap(),
            focused: self.focused.unwrap(),
        }
    }
}
