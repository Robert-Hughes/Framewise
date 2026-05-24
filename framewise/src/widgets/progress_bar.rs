use crate::{
    draw::{DrawCmd, DrawCommands},
    types::{Color, Rect},
    widget::{WidgetSpec, WidgetSpecBuilder},
    WidgetResult,
};

pub struct ProgressBarSpec {
    pub rect: Rect,
    /// 0.0–1.0. Pass `f32::NAN` for indeterminate (renders partial fill at `phase`).
    pub value: f32,
    /// Indeterminate sweep offset in 0.0–1.0 range (caller animates over time).
    pub phase: f32,
    /// When true, fill uses rust instead of ink (active/in-progress state).
    pub active: bool,
    pub style: ProgressBarStyle,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressBarStyle {
    pub track_color: Color,
    pub fill_color: Color,
    pub active_fill_color: Color,
    pub track_height: f32,
    pub indeterminate_fraction: f32,
}

impl Default for ProgressBarStyle {
    fn default() -> Self {
        Self {
            track_color: Color::from_srgb_f32(21.0 / 255.0, 19.0 / 255.0, 15.0 / 255.0, 0.10),
            fill_color: Color::from_srgb_u8(21, 19, 15, 255),
            active_fill_color: Color::from_srgb_u8(194, 90, 44, 255),
            track_height: 3.0,
            indeterminate_fraction: 0.3,
        }
    }
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
                style: ProgressBarStyle {
                    track_color: Color::WHITE,
                    fill_color: Color::BLACK,
                    active_fill_color: Color::BLACK,
                    track_height: 3.0,
                    indeterminate_fraction: 0.3,
                },
            },
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

    pub fn style(mut self, style: ProgressBarStyle) -> Self {
        self.spec.style = style;
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

    fn with_theme(mut self, theme: &crate::Theme) -> Self {
        self.spec.style = theme.progress_bar_style();
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
    let mut cmds = DrawCommands::new();

    // Track: 3px high, centered vertically in the given rect.
    let track_h = spec.style.track_height;
    let track = Rect::new(
        spec.rect.x,
        spec.rect.y + (spec.rect.h - track_h) * 0.5,
        spec.rect.w,
        track_h,
    );
    cmds.push(DrawCmd::FillRect {
        rect: track,
        color: spec.style.track_color,
    });

    let fill_color = if spec.active {
        spec.style.active_fill_color
    } else {
        spec.style.fill_color
    };

    if spec.value.is_nan() {
        // Indeterminate: 30% width sweeping along, wrapped by phase.
        let seg_w = spec.rect.w * spec.style.indeterminate_fraction;
        let start = spec.phase * spec.rect.w;
        let x = track.x + start;
        let visible_w = (seg_w).min(track.x + track.w - x).max(0.0);
        if visible_w > 0.0 {
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(x, track.y, visible_w, track_h),
                color: fill_color,
            });
        }
    } else {
        let fill_w = (spec.rect.w * spec.value.clamp(0.0, 1.0)).max(0.0);
        if fill_w > 0.0 {
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(track.x, track.y, fill_w, track_h),
                color: fill_color,
            });
        }
    }

    ProgressBarResult { draw: cmds }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_visual_normal() {
        let spec = ProgressBarSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 10.0), // h=10
            value: 0.5,
            phase: 0.0,
            active: false,
            style: Default::default(),
        };
        let res = progress_bar(spec);
        let cmds = res.draw.0;

        // 1. Track background
        // 2. Fill
        assert_eq!(cmds.len(), 2);

        let t = crate::Theme::default();
        let _track_y = 10.0 + (10.0 - 3.0) * 0.5; // 13.5

        assert!(
            matches!(&cmds[0], DrawCmd::FillRect { color, rect } if *color == t.line_soft && rect == &Rect::new(10.0, 13.5, 100.0, 3.0))
        );
        assert!(
            matches!(&cmds[1], DrawCmd::FillRect { color, rect } if *color == t.ink && rect == &Rect::new(10.0, 13.5, 50.0, 3.0))
        );
    }

    #[test]
    fn test_progress_bar_visual_active() {
        let spec = ProgressBarSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 10.0),
            value: 0.5,
            phase: 0.0,
            active: true,
            style: Default::default(),
        };
        let res = progress_bar(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);

        let t = crate::Theme::default();
        assert!(matches!(&cmds[1], DrawCmd::FillRect { color, .. } if *color == t.rust));
    }

    #[test]
    fn test_progress_bar_visual_indeterminate() {
        let spec = ProgressBarSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 10.0),
            value: f32::NAN,
            phase: 0.5,
            active: false,
            style: Default::default(),
        };
        let res = progress_bar(spec);
        let cmds = res.draw.0;

        assert_eq!(cmds.len(), 2);

        // Indeterminate fill width is 30% of 100.0 = 30.0
        // Starts at phase * 100.0 = 50.0. So rect x = 60.0, w = 30.0.
        // Wait, start = 0.5 * 100 = 50.0. x = track.x + start = 10.0 + 50.0 = 60.0.
        // visible_w = 30.min(10 + 100 - 60).max(0) = 30.min(50).max(0) = 30.0.
        let t = crate::Theme::default();
        if let DrawCmd::FillRect { color, rect } = cmds[1] {
            assert_eq!(color, t.ink);
            assert!(
                (rect.x - 60.0).abs() < 0.01,
                "Expected x around 60.0, got {}",
                rect.x
            );
            assert!(
                (rect.w - 30.0).abs() < 0.01,
                "Expected w around 30.0, got {}",
                rect.w
            );
        } else {
            panic!("Expected FillRect");
        }
    }
}
