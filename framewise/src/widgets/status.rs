use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, text::TextSystem, theme::Theme, types::{Color, Rect}
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusVariant {
    Neutral,
    Ok,
    Warn,
    Err,
    Live,
}

pub struct StatusSpec<'a, T: crate::text::TextSystem> {
    pub ts: &'a mut T,
    pub rect:    Rect,
    pub label:   &'a str,
    pub variant: StatusVariant,
}

pub struct StatusResult {
    pub draw: DrawCommands,
}

impl WidgetResult for StatusResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn status<'a, T: crate::text::TextSystem>(spec: StatusSpec<'a, T>) -> StatusResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    let dot_size = 6.0_f32;
    let gap = 8.0_f32;

    let dot_color = match spec.variant {
        StatusVariant::Neutral => t.muted,
        StatusVariant::Ok      => Color::from_srgb_f32(0.302, 0.541, 0.227, 1.0),
        StatusVariant::Warn    => t.rust,
        StatusVariant::Err     => Color::from_srgb_f32(0.702, 0.145, 0.122, 1.0),
        StatusVariant::Live    => t.rust,
    };

    cmds.push(DrawCmd::FillRect {
        rect:  Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
        color: dot_color,
    });

    let label_upper = spec.label.to_uppercase();
    let layout = spec.ts.prepare(&label_upper, t.text_sm);
    let ty = spec.rect.y + (dot_size - layout.size.y) * 0.5;
    cmds.push(DrawCmd::Text {
        rect:   Rect::new(spec.rect.x + dot_size + gap, ty, layout.size.x, layout.size.y),
        color:  t.muted,
        handle: layout.handle,
    });

    StatusResult { draw: cmds }
}




pub struct StatusSpecBuilder<'a, T: crate::text::TextSystem> {
    pub label: Option<&'a str>,
    pub variant: Option<StatusVariant>,
    pub rect: Option<Rect>,
    pub ts: Option<&'a mut T>,
}

impl<'a, T: crate::text::TextSystem> StatusSpecBuilder<'a, T> {
    pub fn new() -> Self {
        Self {
            label: None,
            variant: None,
            rect: None,
            ts: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn variant(mut self, variant: StatusVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl<'a, T: crate::text::TextSystem> crate::widget::WidgetSpecBuilder<'a, T> for StatusSpecBuilder<'a, T> {
    type Spec = StatusSpec<'a, T>;

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
        StatusSpec {
            ts: self.ts.expect("TextSystem is required"),
            rect: self.rect.unwrap_or_default(),
            label: self.label.unwrap(),
            variant: self.variant.unwrap(),
        }
    }
}
