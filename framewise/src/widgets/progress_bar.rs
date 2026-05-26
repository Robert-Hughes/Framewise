use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Low-level progress bar widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
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

        ProgressBarResult {
            draw: cmds,
            layout: LayoutInfo::tight(spec.rect),
        }
    }
}

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

    pub fn rect(mut self, rect: Rect) -> Self {
        self.spec.rect = rect;
        self
    }

    pub fn apply_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.spec.style = theme.progress_bar_style();
        self
    }

    pub fn build(self) -> ProgressBarSpec {
        self.spec
    }
}

pub struct ProgressBarResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct ProgressBarInfo {
    pub layout: LayoutInfo,
}

impl ProgressBarResult {
    pub fn into_parts(self) -> (DrawCommands, ProgressBarInfo) {
        (
            self.draw,
            ProgressBarInfo {
                layout: self.layout,
            },
        )
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level progress bar widget function using WidgetContext.
///
/// This function accepts a ProgressBarSpec and calls the low-level raw::progress_bar function.
pub fn progress_bar<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: ProgressBarSpecBuilder,
) -> ProgressBarInfo {
    let rect = ctx.layout(layout_params);
    let builder = builder.rect(rect).apply_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::progress_bar(spec);
    ctx.append_cmds(result.draw.0);
    ProgressBarInfo {
        layout: result.layout,
    }
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
        let style = spec.style;
        let res = raw::progress_bar(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                    color: style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 13.5, 50.0, 3.0),
                    color: style.fill_color,
                },
            ])
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
        let style = spec.style;
        let res = raw::progress_bar(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                    color: style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 13.5, 50.0, 3.0),
                    color: style.active_fill_color,
                },
            ])
        );
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
        let style = spec.style;
        let res = raw::progress_bar(spec);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 13.5, 100.0, 3.0),
                    color: style.track_color,
                },
                DrawCmd::FillRect {
                    rect: Rect::new(60.0, 13.5, 30.000002, 3.0),
                    color: style.fill_color,
                },
            ])
        );
    }
}
