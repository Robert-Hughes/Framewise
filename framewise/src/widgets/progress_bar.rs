use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::TextSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ProgressBarSpec {
        pub rect: Rect,
        /// 0.0–1.0. Pass `f32::NAN` for indeterminate (renders partial fill at `phase`).
        pub value: f32,
        /// Indeterminate sweep offset in 0.0–1.0 range (caller animates over time).
        pub phase: f32,
        /// When true, fill uses rust instead of ink (active/in-progress state).
        pub active: bool,
        pub style: super::ProgressBarStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ProgressBarResult {
        pub draw: DrawCommands,
    }

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

        ProgressBarResult { draw: cmds }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressBarStyle {
    pub track_color: Color,
    pub fill_color: Color,
    pub active_fill_color: Color,
    pub track_height: f32,
    pub indeterminate_fraction: f32,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBarResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressBarSpecBuilder {
    pub value: Option<f32>,
    pub phase: Option<f32>,
    pub active: Option<bool>,
    pub style: Option<ProgressBarStyle>,
    pub rect: Option<Rect>,
}

impl ProgressBarSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn value(mut self, value: f32) -> Self {
        self.value = Some(value);
        self
    }

    pub fn phase(mut self, phase: f32) -> Self {
        self.phase = Some(phase);
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = Some(active);
        self
    }

    pub fn style(mut self, style: ProgressBarStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(theme.progress_bar_style());
        }
        self
    }

    pub fn build(self) -> raw::ProgressBarSpec {
        raw::ProgressBarSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            value: self.value.expect("value not set — call .value()"),
            phase: self.phase.unwrap_or(0.0),
            active: self.active.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level progress bar widget function using WidgetContext.
///
/// This function accepts a ProgressBarSpec and calls the low-level raw::progress_bar function.
pub fn progress_bar<
    T: TextSystem,
    S: LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ProgressBarSpecBuilder,
    layout_params: S::Params,
) -> ProgressBarResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::progress_bar(spec);
    ctx.append_cmds(result.draw);
    ProgressBarResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::ProgressBarSpec;
    use super::*;

    #[test]
    fn test_progress_bar_visual_normal() {
        let spec = ProgressBarSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 10.0), // h=10
            value: 0.5,
            phase: 0.0,
            active: false,
            style: crate::theme::Theme::framewise().progress_bar_style(),
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
            style: crate::theme::Theme::framewise().progress_bar_style(),
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
            style: crate::theme::Theme::framewise().progress_bar_style(),
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

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = ProgressBarSpecBuilder::new().value(0.5);
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.progress_bar_style()));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.progress_bar_style();
        custom_style.track_height = 99.0;
        let builder = ProgressBarSpecBuilder::new().value(0.5).style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().track_height, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::progress_bar(
            &mut ctx,
            ProgressBarSpecBuilder::new().value(0.5).rect(custom_rect),
            layout_rect,
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
