use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{Color, Layer, Rect},
    widget::{LayoutInfo, WidgetContext},
};

// ── Raw Implementation ───────────────────────────────────────────────────────

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ProgressBarSpec {
        pub layer: Layer,
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
    pub struct ProgressBarPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct ProgressBarPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ProgressBarResult {}

    /// Return the size this progress bar would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a progress bar's
    /// extent is caller-driven. This returns [`SizeRequest::UNKNOWN`].
    pub fn pre_layout_progress_bar(
        spec: &ProgressBarPreLayoutSpec,
        offer: SizeOffer,
    ) -> ProgressBarPreLayoutResult {
        ProgressBarPreLayoutResult {
            size_request: progress_bar_size_request(spec, offer),
        }
    }

    fn progress_bar_size_request(
        _spec: &ProgressBarPreLayoutSpec,
        _offer: SizeOffer,
    ) -> SizeRequest {
        SizeRequest::UNKNOWN
    }

    /// Low‑level progress bar draw function.
    ///
    /// Appends draw commands to `cmds`.
    pub fn post_layout_progress_bar(
        spec: ProgressBarSpec,
        _pre_layout: ProgressBarPreLayoutResult,
        cmds: &mut DrawCommands,
    ) {
        // Track: 3px high, centered vertically in the given rect.
        let track_h = spec.style.track_height;
        let track = Rect::new(
            spec.rect.x,
            spec.rect.y + (spec.rect.h - track_h) * 0.5,
            spec.rect.w,
            track_h,
        );
        cmds.push(DrawCmd::FillRect {
            anti_alias: false,
            rect: track,
            color: spec.style.track_color,
            z: spec.layer.get_z(),
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
                    anti_alias: false,
                    rect: Rect::new(x, track.y, visible_w, track_h),
                    color: fill_color,
                    z: spec.layer.get_z(),
                });
            }
        } else {
            let fill_w = (spec.rect.w * spec.value.clamp(0.0, 1.0)).max(0.0);
            if fill_w > 0.0 {
                cmds.push(DrawCmd::FillRect {
                    anti_alias: false,
                    rect: Rect::new(track.x, track.y, fill_w, track_h),
                    color: fill_color,
                    z: spec.layer.get_z(),
                });
            }
        }
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

impl ProgressBarStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            track_color: theme.line_soft,
            fill_color: theme.ink,
            active_fill_color: theme.rust,
            track_height: 3.0,
            indeterminate_fraction: 0.3,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBarResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ProgressBarSpec {
    pub value: f32,
    pub phase: f32,
    pub active: bool,
    pub style: ProgressBarStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProgressBarSpecBuilder {
    pub value: Option<f32>,
    pub phase: Option<f32>,
    pub active: Option<bool>,
    pub style: Option<ProgressBarStyle>,
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(ProgressBarStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> ProgressBarSpec {
        ProgressBarSpec {
            value: self.value.expect("value not set — call .value()"),
            phase: self.phase.unwrap_or(0.0),
            active: self.active.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High‑level widget function ───────────────────────────────────────────────────

/// High‑level progress bar widget function using `WidgetContext`.
pub fn progress_bar<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: ProgressBarSpecBuilder,
    layout_params: S::Params,
) -> ProgressBarResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::ProgressBarPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_progress_bar(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ProgressBarSpec {
        layer: ctx.layer,
        rect,
        value: spec.value,
        phase: ctx.time as f32,
        active: spec.active,
        style: spec.style,
    };
    raw::post_layout_progress_bar(raw_spec, pre_layout, ctx.cmds);
    ProgressBarResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "progress_bar_tests.rs"]
mod tests;
