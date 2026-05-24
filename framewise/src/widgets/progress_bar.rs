use crate::{
    draw::{DrawCmd, DrawCommands},
    theme::Theme,
    types::Rect,
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

pub fn progress_bar(spec: ProgressBarSpec) -> DrawCommands {
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

    cmds
}
