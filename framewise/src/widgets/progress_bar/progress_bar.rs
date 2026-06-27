use crate::{
    draw::DrawCommands,
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
        cmds.push_crisp_fill_rect(track, spec.style.track_color, spec.layer.get_z());

        let fill_color = if spec.active {
            spec.style.active_fill_color
        } else {
            spec.style.fill_color
        };

        if spec.value.is_nan() {
            // Indeterminate: 30% width sweeping along, wrapped by phase.
            let seg_w = spec.rect.w * spec.style.indeterminate_fraction;
            let phase = spec.phase.rem_euclid(1.0);
            let start = phase * spec.rect.w;
            let x = track.x + start;
            let visible_w = (seg_w).min(track.x + track.w - x).max(0.0);
            if visible_w > 0.0 {
                cmds.push_crisp_fill_rect(
                    Rect::new(x, track.y, visible_w, track_h),
                    fill_color,
                    spec.layer.get_z(),
                );
            }
        } else {
            let fill_w = (spec.rect.w * spec.value.clamp(0.0, 1.0)).max(0.0);
            if fill_w > 0.0 {
                cmds.push_crisp_fill_rect(
                    Rect::new(track.x, track.y, fill_w, track_h),
                    fill_color,
                    spec.layer.get_z(),
                );
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
            track_color: theme.line_soft_on_paper,
            fill_color: theme.ink,
            active_fill_color: theme.rust,
            track_height: 3.0,
            indeterminate_fraction: 0.3,
        }
    }
}

impl Default for ProgressBarStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
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
    pub active: bool,
    pub style: ProgressBarStyle,
}

impl ProgressBarSpec {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            active: false,
            style: ProgressBarStyle::default(),
        }
    }

    pub fn new_from_theme(value: f32, theme: &crate::theme::Theme) -> Self {
        Self::new(value).theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = ProgressBarStyle::from_theme(theme);
        self
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value;
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn style(mut self, style: ProgressBarStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High‑level widget function ───────────────────────────────────────────────────

/// High-level progress bar widget function using `WidgetContext`.
///
/// Consumes a complete `ProgressBarSpec`, runs the raw pre-layout phase to obtain a
/// `SizeRequest`, resolves the final rect with layout, then runs the raw
/// post-layout phase.
///
/// The high-level function derives the raw indeterminate phase from `ctx.time`.
pub fn progress_bar<T: TextBackend, S: LayoutState, CF>(
    spec: ProgressBarSpec,
    layout_params: S::Params,
    ctx: &mut WidgetContext<T, S, CF>,
) -> ProgressBarResult {
    let pre_layout_spec = raw::ProgressBarPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_progress_bar(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let phase = ((ctx.time as f32) * 0.5).rem_euclid(1.0);
    let raw_spec = raw::ProgressBarSpec {
        layer: ctx.layer,
        rect,
        value: spec.value,
        phase,
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
