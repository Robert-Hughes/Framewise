use crate::{
    draw::{BorderPlacement, DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::{Layout, LayoutState, SizeOffer},
    text::TextBackend,
    types::{Color, Layer, Rect, Stroke, Vec2},
    widget::WidgetContext,
};

pub mod raw {
    use super::*;

    /// Input specification for a frame.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FrameSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub style: super::FrameStyle,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FramePreLayoutSpec {
        pub style: super::FrameStyle,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FramePreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FrameToken {
        pub fill_index: usize,
        pub clip_index: usize,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FrameResult {
        pub token: FrameToken,
        pub content_bounds: Rect,
    }

    /// Return the size this frame would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a frame's bounds are
    /// resolved bottom-up from its children via the
    /// `begin_frame`/`end_frame` lifecycle, so there is nothing to report yet —
    /// this returns [`SizeRequest::UNKNOWN`]. A later revision may report a
    /// minimum size derived from padding and border width alone, so that a frame
    /// with no children does not collapse to a degenerate zero rect.
    ///
    pub fn pre_layout_frame(spec: &FramePreLayoutSpec, offer: SizeOffer) -> FramePreLayoutResult {
        FramePreLayoutResult {
            size_request: frame_size_request(spec, offer),
        }
    }

    fn frame_size_request(
        spec: &FramePreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// Low-level frame begin function.
    ///
    /// Pushes placeholder `FillRect` and `PushClip` commands (so the background draws behind
    /// children and children are clipped to the content area). Both are patched with the final
    /// resolved bounds in `end_frame`. The border `BorderRect`, if any, is pushed in `end_frame`
    /// after `PopClip` so it draws on top of and outside the clip.
    ///
    /// `spec.rect` may be provisional at call time (e.g. zeroed or a placeholder) when the
    /// frame is auto-sizing — the placeholder commands use it as an initial value only.
    /// `end_frame` receives the final resolved rect and patches them in-place.
    pub fn begin_frame(
        spec: FrameSpec,
        _pre_layout: FramePreLayoutResult,
        cmds: &mut DrawCommands,
    ) -> FrameResult {
        let rect = spec.rect;
        let style = spec.style;
        let border_width = style.border.map_or(0.0, |s| s.width);
        let inset = border_width + style.padding;
        let content = rect.inset(inset);

        let fill_index = cmds.len();
        cmds.push(DrawCmd::FillRect {
            rect,
            color: style.background,
            z: spec.layer.get_z(),
        });

        let clip_index = cmds.len();
        cmds.push(DrawCmd::PushClip { rect: content });

        FrameResult {
            token: FrameToken {
                fill_index,
                clip_index,
            },
            content_bounds: content,
        }
    }

    /// Low-level frame end function.
    ///
    /// Takes the same `FrameSpec` as `begin_frame` with `.rect` updated to the final resolved
    /// bounds. Patches the `FillRect` and `PushClip` placeholders, then appends `PopClip` and
    /// (if the frame has a border) `BorderRect` — both after the clip, so they draw on top of
    /// and outside the content clip.
    ///
    /// # Panics
    /// Panics if either placeholder at the recorded index is missing or modified,
    /// indicating corruption of the command list.
    pub fn end_frame(token: FrameToken, spec: FrameSpec, cmds: &mut DrawCommands) {
        let rect = spec.rect;
        let style = spec.style;
        let border_width = style.border.map_or(0.0, |s| s.width);
        let inset = border_width + style.padding;
        let content = rect.inset(inset);
        let draw_rect = cmds.snap_rect_edges_to_physical_pixel(rect);

        match cmds.get_mut(token.fill_index) {
            Some(DrawCmd::FillRect { rect: r, .. }) => *r = draw_rect,
            _ => panic!(
                "DrawCommands corruption detected: placeholder FillRect at index {} was missing or modified!",
                token.fill_index
            ),
        }
        match cmds.get_mut(token.clip_index) {
            Some(DrawCmd::PushClip { rect: r }) => *r = content,
            _ => panic!(
                "DrawCommands corruption detected: placeholder PushClip at index {} was missing or modified!",
                token.clip_index
            ),
        }

        cmds.push(DrawCmd::PopClip);

        cmds.push_crisp_border_rect(
            rect,
            style.border,
            BorderPlacement::Inside,
            spec.layer.get_z(),
        );
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a frame (bordered background rectangle).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStyle {
    pub background: Color,
    pub border: Option<Stroke>,
    /// Padding between the border and the content area.
    pub padding: f32,
}

impl FrameStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            border: Some(Stroke::new(theme.ink, theme.border)),
            padding: 4.0,
        }
    }
}

impl Default for FrameStyle {
    fn default() -> Self {
        Self::from_theme(&crate::theme::Theme::minimal())
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct FrameResult<'b, T: TextBackend, LS: LayoutState, CF> {
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameSpec {
    pub style: FrameStyle,
}

impl FrameSpec {
    pub fn default_from_theme(theme: &crate::theme::Theme) -> Self {
        Self::default().theme(theme)
    }

    pub fn theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = FrameStyle::from_theme(theme);
        self
    }

    pub fn style(mut self, style: FrameStyle) -> Self {
        self.style = style;
        self
    }
}

// ── High-level container widget function ───────────────────────────────────────────

/// High-level frame container widget function using `WidgetContext`.
///
/// Consumes a complete `FrameSpec` and runs the raw pre-layout phase for
/// lifecycle consistency, then uses deferred layout because the final frame size
/// depends on child extent. The raw begin phase emits provisional placeholder
/// commands and raw end patches them once final bounds are known.
///
/// ### Sizing and fitting
/// Whether the frame fits to its children or respects a fixed/filled footprint is determined
/// dynamically via the generic bounds of `LayoutSpace` (translated from `layout_params` by the parent):
/// - `AxisBound::Exact(w)` -> The frame uses exactly `w` for its extent on that axis.
/// - `AxisBound::Unbounded` / `AxisBound::AtMost` -> The frame sizes itself to the children's extent plus padding.
///
/// ### Lifetime, borrowing, and cursor unlocking
/// The `begin_deferred_layout` call mutably borrows the parent `LayoutState` and returns a `LayoutToken`.
/// The `on_finish` closure captures this token by value.
///
/// To satisfy the Rust borrow checker (avoid E0499), we construct the child context by explicitly
/// destructuring the parent `ctx` fields. This disjointly borrows `ctx.layout_state` (held by the `LayoutToken`
/// inside `on_finish`) separately from `ctx.text_backend`, `ctx.focus_system`, etc., resulting in a perfectly
/// compile-safe cursor-advance deferral.
#[allow(clippy::type_complexity)]
pub fn begin_frame<'a, 'b, T: TextBackend, S: LayoutState, L: Layout, CF>(
    spec: FrameSpec,
    layout_params: S::Params,
    inner_layout: L,
    ctx: &'b mut WidgetContext<'a, T, S, CF>,
) -> FrameResult<
    'b,
    T,
    L::State,
    impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, &mut crate::output::Output, Rect) + 'b,
> {
    let border_width = spec.style.border.map_or(0.0, |b| b.width);
    let inset = border_width + spec.style.padding;

    let pre_layout_spec = raw::FramePreLayoutSpec { style: spec.style };
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_frame(&pre_layout_spec, offer);

    let policy = ctx.layout_policy;
    let violation_font = ctx.theme.sans_font;

    // The deferred-layout borrow plumbing lives in `child_with_deferred_layout`; here we
    // only supply the frame-specific chrome via the two closures.
    let layer = ctx.layer;
    let (child_ctx, _outer_space) = ctx.child_with_deferred_layout(
        layout_params,
        inner_layout,
        // Between begin_deferred_layout and child construction: stamp the provisional rect and push
        // the placeholder background/clip (so they draw beneath the children). The border
        // BorderRect is pushed later in end_frame so it draws on top. The inner layout is
        // begun in the space inset by padding + border_width. Carry (token, spec) to finish.
        move |cmds, outer| {
            let spec = raw::FrameSpec {
                layer,
                rect: Rect::pending_extent(outer.x, outer.y),
                style: spec.style,
            };
            let raw::FrameResult {
                token: frame_token, ..
            } = raw::begin_frame(spec, pre_layout, cmds);
            ((frame_token, spec), outer.inset(inset))
        },
        // At finish: the frame's outer size is its children's extent plus the chrome on
        // both sides. Advance the parent's cursor with that, then retroactively patch the
        // placeholder draw commands with the resolved bounds.
        move |(frame_token, spec), token, content, _focus, text_backend, cmds| {
            let outer_extent = Vec2::new(content.w + inset * 2.0, content.h + inset * 2.0);
            let (bounds, violation) = token.end_deferred_layout(outer_extent).into_parts();
            if let Some(v) = violation {
                crate::widget::react_layout_violation(
                    policy,
                    text_backend,
                    cmds,
                    violation_font,
                    v,
                    bounds,
                    layer.get_z(),
                );
            }
            raw::end_frame(
                frame_token,
                raw::FrameSpec {
                    layer,
                    rect: bounds,
                    ..spec
                },
                cmds,
            );
        },
    );

    FrameResult { ctx: child_ctx }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
