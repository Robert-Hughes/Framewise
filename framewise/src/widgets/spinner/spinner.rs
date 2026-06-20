use crate::{
    draw::{DrawCmd, DrawCommands},
    layout::{LayoutState, SizeOffer, SizeRequest},
    text::TextBackend,
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerSpec {
        /// Top-left. Size is either 16 or 24 (use `large` flag).
        pub rect: Rect,
        pub large: bool,
        pub style: super::SpinnerStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct SpinnerResult {}

    /// Return the size this spinner would request under `offer`.
    ///
    /// The current implementation ignores `offer` because the spinner's extent
    /// is caller-driven. This returns [`SizeRequest::UNKNOWN`].
    pub fn pre_layout_spinner(
        spec: &SpinnerPreLayoutSpec,
        offer: SizeOffer,
    ) -> SpinnerPreLayoutResult {
        SpinnerPreLayoutResult {
            size_request: spinner_size_request(spec, offer),
        }
    }

    fn spinner_size_request(_spec: &SpinnerPreLayoutSpec, _offer: SizeOffer) -> SizeRequest {
        SizeRequest::UNKNOWN
    }

    /// Low-level spinner widget function.
    ///
    /// Appends draw commands to `cmds`.
    /// Square reticle spinner — four corner brackets with a single animated segment.
    /// Since we can't animate, we draw it at a fixed phase (segment at top).
    pub fn post_layout_spinner(
        spec: SpinnerSpec,
        _pre_layout: SpinnerPreLayoutResult,
        cmds: &mut DrawCommands,
    ) {
        let size = if spec.large {
            spec.style.large_size
        } else {
            spec.style.small_size
        };

        let x = spec.rect.x;
        let y = spec.rect.y;

        // Corner bracket size: 5px at 16, 7px at 24.
        let arm = if spec.large {
            spec.style.large_arm
        } else {
            spec.style.small_arm
        };
        let w = spec.style.width;

        // Top-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x, y + arm),
            p1: Vec2::new(x, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x, y),
            p1: Vec2::new(x + arm, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Top-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size - arm, y),
            p1: Vec2::new(x + size, y),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y),
            p1: Vec2::new(x + size, y + arm),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Bottom-right bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y + size - arm),
            p1: Vec2::new(x + size, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size, y + size),
            p1: Vec2::new(x + size - arm, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        // Bottom-left bracket.
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + arm, y + size),
            p1: Vec2::new(x, y + size),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x, y + size),
            p1: Vec2::new(x, y + size - arm),
            color: spec.style.color,
            width: w,
            z: spec.layer.get_z(),
        });

        // Animated segment on the top edge — drawn as a rust highlight.
        let seg_w = size * spec.style.highlight_fraction;
        cmds.push(DrawCmd::StrokeLine {
            anti_alias: false,
            p0: Vec2::new(x + size * 0.1, y),
            p1: Vec2::new(x + size * 0.1 + seg_w, y),
            color: spec.style.highlight,
            width: w,
            z: spec.layer.get_z(),
        });
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerStyle {
    pub color: Color,
    pub highlight: Color,
    pub small_size: f32,
    pub large_size: f32,
    pub small_arm: f32,
    pub large_arm: f32,
    pub width: f32,
    pub highlight_fraction: f32,
}

impl SpinnerStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            color: theme.ink,
            highlight: theme.rust,
            small_size: 16.0,
            large_size: 24.0,
            small_arm: 5.0,
            large_arm: 7.0,
            width: 1.5,
            highlight_fraction: 0.4,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerResult {
    pub layout: LayoutInfo,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct SpinnerSpec {
    pub large: bool,
    pub style: SpinnerStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SpinnerSpecBuilder {
    pub large: Option<bool>,
    pub style: Option<SpinnerStyle>,
}

impl SpinnerSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn large(mut self, large: bool) -> Self {
        self.large = Some(large);
        self
    }

    pub fn style(mut self, style: SpinnerStyle) -> Self {
        self.style = Some(style);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(SpinnerStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> SpinnerSpec {
        SpinnerSpec {
            large: self.large.unwrap_or(false),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level spinner widget function using `WidgetContext`.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the final rect with layout, then runs the raw post-layout phase.
pub fn spinner<T: TextBackend, S: LayoutState, CF>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: SpinnerSpecBuilder,
    layout_params: S::Params,
) -> SpinnerResult {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::SpinnerPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_spinner(&pre_layout_spec, offer);
    let rect = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::SpinnerSpec {
        rect,
        large: spec.large,
        style: spec.style,
        layer: ctx.layer,
    };
    raw::post_layout_spinner(raw_spec, pre_layout, ctx.cmds);
    SpinnerResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
#[path = "spinner_tests.rs"]
mod tests;
