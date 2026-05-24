use crate::{
    WidgetResult, draw::{DrawCmd, DrawCommands}, theme::Theme, types::Rect, widget::{WidgetSpec, WidgetSpecBuilder}
};

pub struct ProgressBarSpec {
    pub rect:   Rect,
    /// 0.0–1.0. Pass `f32::NAN` for indeterminate (renders partial fill at `phase`).
    pub value:  f32,
    /// Indeterminate sweep offset in 0.0–1.0 range (caller animates over time).
    pub phase:  f32,
    /// When true, fill uses rust instead of ink (active/in-progress state).
    pub active: bool,
}

impl WidgetSpec for ProgressBarSpec {
    type Builder = ProgressBarSpecBuilder;
}

pub struct ProgressBarSpecBuilder {
    spec: ProgressBarSpec,
}

impl ProgressBarSpecBuilder {
    pub fn new(value: f32) -> Self {
        Self {
            spec: ProgressBarSpec {
                rect: Rect::ZERO,
                value,
                phase: 0.0,
                active: false,
            }
        }
    }

    pub fn phase(mut self, phase: f32) -> Self {
        self.spec.phase = phase;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.spec.active = active;
        self
    }
}

impl<'a, T: crate::text::TextSystem> WidgetSpecBuilder<'a, T> for ProgressBarSpecBuilder {
    type Spec = ProgressBarSpec;

    fn with_rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    fn with_style(self) -> Self {
        self
    }

    fn build(self) -> Self::Spec {
        self.spec
    }
}

pub struct ProgressBarResult {
    pub draw: DrawCommands,
}

impl WidgetResult for ProgressBarResult {
    type Info = ();

    fn into_parts(self) -> (DrawCommands, Self::Info) {
        (self.draw, ())
    }
}

pub fn progress_bar(spec: ProgressBarSpec) -> ProgressBarResult {
    let t = Theme::framewise();
    let mut cmds = DrawCommands::new();

    // Track: 3px high, centered vertically in the given rect.
    let track_h = 3.0_f32;
    let track = Rect::new(
        spec.rect.x,
        spec.rect.y + (spec.rect.h - track_h) * 0.5,
        spec.rect.w,
        track_h,
    );
    cmds.push(DrawCmd::FillRect { rect: track, color: t.line_soft });

    let fill_color = if spec.active { t.rust } else { t.ink };

    if spec.value.is_nan() {
        // Indeterminate: 30% width sweeping along, wrapped by phase.
        let seg_w = spec.rect.w * 0.3;
        let start = spec.phase * spec.rect.w;
        let x = track.x + start;
        let visible_w = (seg_w).min(track.x + track.w - x).max(0.0);
        if visible_w > 0.0 {
            cmds.push(DrawCmd::FillRect {
                rect:  Rect::new(x, track.y, visible_w, track_h),
                color: fill_color,
            });
        }
    } else {
        let fill_w = (spec.rect.w * spec.value.clamp(0.0, 1.0)).max(0.0);
        if fill_w > 0.0 {
            cmds.push(DrawCmd::FillRect {
                rect:  Rect::new(track.x, track.y, fill_w, track_h),
                color: fill_color,
            });
        }
    }

    ProgressBarResult { draw: cmds }
}
