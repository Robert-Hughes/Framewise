use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::{FocusId, FocusSystem},
    input::Input,
    layout::{AxisBound, Layout, LayoutSpace, LayoutState, SizeOffer},
    text::TextBackend,
    types::{ClipRect, Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
    widgets::SliderStyle,
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScrollAreaSpec {
        pub layer: Layer,
        pub rect: Rect,
        pub horizontal: super::ScrollAxis,
        pub vertical: super::ScrollAxis,
        pub clip_rect: ClipRect,
        pub time: f64,
        pub style: super::ScrollAreaStyle,
        pub keyboard_focusable: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScrollAreaPreLayoutSpec {}

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScrollAreaPreLayoutResult {
        pub size_request: crate::layout::SizeRequest,
    }

    /// Carries the geometry resolved at `begin` that `end` needs to finish the
    /// area once the content extent is known. Scroll geometry (max_scroll, thumb
    /// ratios, at_* flags) is **not** stored here — it is computed in `end` from
    /// the measured content extent.
    #[derive(Debug, Clone, PartialEq)]
    pub struct ScrollAreaToken {
        pub(super) id: FocusId,
        pub(super) rect: Rect,
        pub(super) content_bounds: Rect,
        pub(super) needs_v: bool,
        pub(super) needs_h: bool,
        pub(super) clip_rect: ClipRect,
        pub(super) time: f64,
        pub(super) style: super::ScrollAreaStyle,
        pub(super) layer: Layer,
        pub(super) keyboard_focusable: bool,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct ScrollAreaResult {
        pub token: ScrollAreaToken,
        pub content_bounds: Rect,
        /// The scroll offset to lay this frame's children out at (captured from
        /// last frame's clamped value).
        pub offset: Vec2,
        /// Space to lay children into: the scrollable axis (the one with a
        /// scrollbar) is [`AxisBound::Unbounded`] so content can extend past the
        /// viewport and its extent is measured at `end`.
        pub inner_space: LayoutSpace,
    }

    #[derive(Debug, Clone, Copy, Default, PartialEq)]
    pub struct ScrollAreaEndResult {
        pub scrollbar_pressed: bool,
    }

    /// Return the size this scroll area would request under `offer`.
    ///
    /// The current implementation ignores `offer` because a scroll area's outer
    /// extent is caller-driven (the viewport bounds come
    /// from the layout), so there is nothing to report yet — this returns
    /// [`SizeRequest::UNKNOWN`]. A later revision may report a minimum viewport
    /// size derived from the reserved scrollbar widths.
    ///
    pub fn pre_layout_scroll_area(
        spec: &ScrollAreaPreLayoutSpec,
        offer: SizeOffer,
    ) -> ScrollAreaPreLayoutResult {
        ScrollAreaPreLayoutResult {
            size_request: scroll_area_size_request(spec, offer),
        }
    }

    fn scroll_area_size_request(
        spec: &ScrollAreaPreLayoutSpec,
        _offer: SizeOffer,
    ) -> crate::layout::SizeRequest {
        let _ = spec;
        crate::layout::SizeRequest::UNKNOWN
    }

    /// Whether this axis reserves a scrollbar gutter, decided at `begin` without the
    /// content extent. Concrete (`Px`) overflow is tested against the raw viewport
    /// extent `outer_len` — NOT the post-gutter content extent — so the two axes'
    /// decisions don't mutually depend (a ~12px gutter won't flip the result).
    fn axis_needs_bar(
        extent: super::ScrollExtent,
        vis: super::ScrollbarVisibility,
        outer_len: f32,
    ) -> bool {
        match vis {
            super::ScrollbarVisibility::Always => true,
            super::ScrollbarVisibility::Auto => match extent {
                // Can't prove fit at begin → reserve (deferred; bar drawn iff overflow).
                super::ScrollExtent::Unbounded => true,
                // Fills / capped at viewport → provably fits → no bar.
                super::ScrollExtent::Exact(super::ScrollLen::Viewport)
                | super::ScrollExtent::AtMost(super::ScrollLen::Viewport) => false,
                // Pinned / capped at n → bar iff it can't fit the raw viewport.
                super::ScrollExtent::Exact(super::ScrollLen::Px(n))
                | super::ScrollExtent::AtMost(super::ScrollLen::Px(n)) => n > outer_len,
            },
        }
    }

    /// Lower a per-axis request to the concrete `AxisBound` handed to the inner
    /// layout, now that the post-gutter content extent on this axis is known.
    fn axis_lower(extent: super::ScrollExtent, content_len: f32) -> AxisBound {
        match extent {
            super::ScrollExtent::Exact(super::ScrollLen::Viewport) => AxisBound::Exact(content_len),
            super::ScrollExtent::Exact(super::ScrollLen::Px(n)) => AxisBound::Exact(n),
            super::ScrollExtent::AtMost(super::ScrollLen::Viewport) => {
                AxisBound::AtMost(content_len)
            }
            super::ScrollExtent::AtMost(super::ScrollLen::Px(n)) => AxisBound::AtMost(n),
            super::ScrollExtent::Unbounded => AxisBound::Unbounded,
        }
    }

    /// Low-level scroll area begin function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn begin_scroll_area(
        spec: ScrollAreaSpec,
        _pre_layout: ScrollAreaPreLayoutResult,
        state: &mut ScrollState,
        _input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> ScrollAreaResult {
        focus_system.push_keyboard_scroll_scope(state.id);

        // width  axis ↔ horizontal scrolling ↔ horizontal bar (steals HEIGHT)
        // height axis ↔ vertical   scrolling ↔ vertical   bar (steals WIDTH)
        let needs_h = axis_needs_bar(spec.horizontal.extent, spec.horizontal.vis, spec.rect.w); // horizontal bar
        let needs_v = axis_needs_bar(spec.vertical.extent, spec.vertical.vis, spec.rect.h); // vertical bar

        let content_w = if needs_v {
            (spec.rect.w - spec.style.scrollbar_width).max(0.0)
        } else {
            spec.rect.w
        };
        let content_h = if needs_h {
            (spec.rect.h - spec.style.scrollbar_width).max(0.0)
        } else {
            spec.rect.h
        };
        let content_bounds = Rect::new(spec.rect.x, spec.rect.y, content_w, content_h);

        // Children are clipped to the viewport. Scrollbars are drawn at `end`
        // (after PopClip), so they sit on top of and outside the content clip.
        cmds.push(DrawCmd::PushClip {
            rect: content_bounds,
        });

        let inner_space = LayoutSpace {
            x: content_bounds.x,
            y: content_bounds.y,
            width: axis_lower(spec.horizontal.extent, content_w),
            height: axis_lower(spec.vertical.extent, content_h),
        };

        let token = ScrollAreaToken {
            id: state.id,
            rect: spec.rect,
            content_bounds,
            needs_v,
            needs_h,
            clip_rect: spec.clip_rect,
            time: spec.time,
            style: spec.style,
            layer: spec.layer,
            keyboard_focusable: spec.keyboard_focusable,
        };

        ScrollAreaResult {
            token,
            content_bounds,
            offset: state.offset,
            inner_space,
        }
    }

    /// Low-level scroll area end function.
    ///
    /// Receives the children's measured `content_extent` and resolves every
    /// content-dependent computation: max scroll, offset clamp, wheel/page-key
    /// application, scrollbar thumbs, hover-scroll claims, and the pg* claims.
    /// Scrollbars are drawn here — after `PopClip` — so they render on top of
    /// (and outside the clip of) the content.
    pub fn end_scroll_area(
        token: ScrollAreaToken,
        content_extent: Vec2,
        state: &mut ScrollState,
        input: &Input,
        focus_system: &mut FocusSystem,
        cmds: &mut DrawCommands,
    ) -> ScrollAreaEndResult {
        let max_scroll = Vec2::new(
            (content_extent.x - token.content_bounds.w).max(0.0),
            (content_extent.y - token.content_bounds.h).max(0.0),
        );
        let mode = ScrollMode::resolve(token.needs_v, token.needs_h, max_scroll);

        // Apply this frame's wheel (gated on hover + the claims won last frame) and
        // page keys here, so all offset mutation is co-located at `end` and children
        // always lay out against the offset captured at `begin` (uniform one-frame
        // input lag — see the deferred scroll design).
        let is_visible = token
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
        if token.content_bounds.contains(input.mouse_pos) && is_visible {
            apply_wheel(state, mode, focus_system, input);
        }
        apply_page_keys(state, mode, token.content_bounds, focus_system, input);

        state.offset.x = state.offset.x.clamp(0.0, max_scroll.x);
        state.offset.y = state.offset.y.clamp(0.0, max_scroll.y);

        // End the content clip BEFORE drawing the scrollbars: they live in the
        // reserved gutter (outside content_bounds) and must draw on top of content.
        cmds.push(DrawCmd::PopClip);

        let mut scrollbar_pressed = false;

        if token.needs_v {
            let span = token.content_bounds.h;
            let track_rect = Rect::new(
                token.content_bounds.right(),
                token.rect.y,
                token.style.scrollbar_width,
                token.content_bounds.h,
            );

            let slider_spec = crate::widgets::slider::raw::SliderSpec {
                orientation: crate::widgets::slider::Orientation::Vertical,
                rect: track_rect,
                min: 0.0,
                max: content_extent.y,
                min_gap: Some(span),
                max_gap: Some(span),
                value_snap: None,
                page_step: span,
                step: 40.0,
                style: token.style.scrollbar_style,
                clip_rect: token.clip_rect,
                scroll_claim: crate::widgets::slider::ScrollClaimPolicy::YieldSameAxisAtEnds,
                time: token.time,
                layer: token.layer,
                // Degenerate: bar reserved (Always/Auto) but content fits, so the
                // thumb fills the track and nothing scrolls. Disable it so it
                // leaves focus order and dims, while still holding its gutter.
                disabled: max_scroll.y <= 0.0,
                keyboard_focusable: token.keyboard_focusable,
            };

            state.vert_slider_state.value = crate::widgets::SliderValue::Range {
                lower: state.offset.y,
                upper: state.offset.y + span,
            };
            let slider_result = crate::widgets::slider::raw::post_layout_slider(
                slider_spec,
                crate::widgets::slider::raw::SliderPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                &mut state.vert_slider_state,
                input,
                focus_system,
                cmds,
            );
            scrollbar_pressed |= slider_result.input.pressed;
            state.offset.y = state.vert_slider_state.value.lower();
        }

        if token.needs_h {
            let span = token.content_bounds.w;
            let track_rect = Rect::new(
                token.rect.x,
                token.content_bounds.bottom(),
                token.content_bounds.w,
                token.style.scrollbar_width,
            );

            let slider_spec = crate::widgets::slider::raw::SliderSpec {
                orientation: crate::widgets::slider::Orientation::Horizontal,
                rect: track_rect,
                min: 0.0,
                max: content_extent.x,
                min_gap: Some(span),
                max_gap: Some(span),
                value_snap: None,
                page_step: span,
                step: 40.0,
                style: token.style.scrollbar_style,
                clip_rect: token.clip_rect,
                scroll_claim: crate::widgets::slider::ScrollClaimPolicy::YieldSameAxisAtEnds,
                time: token.time,
                layer: token.layer,
                // Degenerate horizontal bar (see vertical case above).
                disabled: max_scroll.x <= 0.0,
                keyboard_focusable: token.keyboard_focusable,
            };

            state.horiz_slider_state.value = crate::widgets::SliderValue::Range {
                lower: state.offset.x,
                upper: state.offset.x + span,
            };
            let slider_result = crate::widgets::slider::raw::post_layout_slider(
                slider_spec,
                crate::widgets::slider::raw::SliderPreLayoutResult {
                    size_request: crate::layout::SizeRequest::UNKNOWN,
                },
                &mut state.horiz_slider_state,
                input,
                focus_system,
                cmds,
            );
            scrollbar_pressed |= slider_result.input.pressed;
            state.offset.x = state.horiz_slider_state.value.lower();
        }

        if token.needs_v && token.needs_h {
            if let Some(corner_color) = token.style.corner_color {
                let corner_rect = Rect::new(
                    token.content_bounds.right(),
                    token.content_bounds.bottom(),
                    token.style.scrollbar_width,
                    token.style.scrollbar_width,
                );
                cmds.push_crisp_fill_rect(corner_rect, corner_color, token.layer.get_z());
                let border = token.style.scrollbar_style.separator_line;
                if let Some(border) = border {
                    // Left border of the corner
                    cmds.push_device_hairline_v(
                        corner_rect.x,
                        corner_rect.y,
                        corner_rect.h,
                        border.color,
                        token.layer.get_z(),
                    );
                    // Top border of the corner
                    cmds.push_device_hairline_h(
                        corner_rect.x,
                        corner_rect.y,
                        corner_rect.w,
                        border.color,
                        token.layer.get_z(),
                    );
                }
            }
        }

        let popped = focus_system.pop_keyboard_scroll_scope();
        debug_assert_eq!(
            popped,
            Some(token.id),
            "ScrollAreaToken finished out of order!"
        );

        // at_* snapshotted AFTER this frame's scroll actions (wheel, page, drag)
        // so reaching a limit releases the corresponding pg* claim next frame,
        // letting the very next press bubble to the parent.
        let at_top = state.offset.y <= 0.0;
        let at_bottom = state.offset.y >= max_scroll.y;
        let at_left = state.offset.x <= 0.0;
        let at_right = state.offset.x >= max_scroll.x;

        // Hover scroll claims — made at end (inner-first) so the deepest hovered
        // scrollable area wins via first-caller-wins. Claims use this frame's true
        // max_scroll (no one-frame lag).
        let is_visible = token
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
        if token.content_bounds.contains(input.mouse_pos) && is_visible {
            match mode {
                ScrollMode::None => {}
                ScrollMode::Vert => {
                    if !at_top {
                        focus_system.claim_scroll_up(token.id);
                    }
                    if !at_bottom {
                        focus_system.claim_scroll_down(token.id);
                    }
                    focus_system.claim_scroll_left(token.id);
                    focus_system.claim_scroll_right(token.id);
                }
                ScrollMode::Horiz => {
                    if !at_left {
                        focus_system.claim_scroll_left(token.id);
                    }
                    if !at_right {
                        focus_system.claim_scroll_right(token.id);
                    }
                    focus_system.claim_scroll_up(token.id);
                    focus_system.claim_scroll_down(token.id);
                }
                ScrollMode::Both => {
                    if !at_top {
                        focus_system.claim_scroll_up(token.id);
                    }
                    if !at_bottom {
                        focus_system.claim_scroll_down(token.id);
                    }
                    if !at_left {
                        focus_system.claim_scroll_left(token.id);
                    }
                    if !at_right {
                        focus_system.claim_scroll_right(token.id);
                    }
                }
            }
        }

        if focus_system.focused_scroll_path().contains(&token.id) {
            // Same-axis claims are conditional on having room to scroll, so a
            // child at its limit lets the parent's claim take effect (pg* is
            // first-caller-wins — see focus.rs). Cross-axis claims are
            // unconditional: they isolate this scope from the other axis,
            // preventing orthogonal keystrokes from leaking to a parent.
            match mode {
                ScrollMode::None => {}
                ScrollMode::Vert => {
                    if !at_top {
                        focus_system.claim_pgup_vert(token.id);
                    }
                    if !at_bottom {
                        focus_system.claim_pgdn_vert(token.id);
                    }
                    focus_system.claim_pgup_horiz(token.id);
                    focus_system.claim_pgdn_horiz(token.id);
                }
                ScrollMode::Horiz => {
                    if !at_left {
                        focus_system.claim_pgup_horiz(token.id);
                    }
                    if !at_right {
                        focus_system.claim_pgdn_horiz(token.id);
                    }
                    focus_system.claim_pgup_vert(token.id);
                    focus_system.claim_pgdn_vert(token.id);
                }
                ScrollMode::Both => {
                    if !at_top {
                        focus_system.claim_pgup_vert(token.id);
                    }
                    if !at_bottom {
                        focus_system.claim_pgdn_vert(token.id);
                    }
                    if !at_left {
                        focus_system.claim_pgup_horiz(token.id);
                    }
                    if !at_right {
                        focus_system.claim_pgdn_horiz(token.id);
                    }
                }
            }
        }

        ScrollAreaEndResult { scrollbar_pressed }
    }

    /// Route the wheel delta to the appropriate axis(es) based on mode, but only
    /// fire on claims this scope actually owns (so unconditional cross-axis claims
    /// don't double as a remap trigger).
    fn apply_wheel(
        state: &mut ScrollState,
        mode: ScrollMode,
        focus_system: &mut FocusSystem,
        input: &Input,
    ) {
        let dy = input.scroll_delta.y;
        let dx_raw = input.scroll_delta.x;

        let scroll_vert = |state: &mut ScrollState, focus_system: &FocusSystem| {
            if dy > 0.0 && focus_system.is_active_scroll_up(state.id) {
                state.offset.y -= dy * SCROLL_PIXELS_PER_LINE;
            }
            if dy < 0.0 && focus_system.is_active_scroll_down(state.id) {
                state.offset.y -= dy * SCROLL_PIXELS_PER_LINE;
            }
        };
        let scroll_horiz = |state: &mut ScrollState, focus_system: &FocusSystem, dx: f32| {
            if dx > 0.0 && focus_system.is_active_scroll_left(state.id) {
                state.offset.x -= dx * SCROLL_PIXELS_PER_LINE;
            }
            if dx < 0.0 && focus_system.is_active_scroll_right(state.id) {
                state.offset.x -= dx * SCROLL_PIXELS_PER_LINE;
            }
        };

        match mode {
            ScrollMode::None => {}
            ScrollMode::Vert => scroll_vert(state, focus_system),
            ScrollMode::Horiz => {
                // Vertical wheel remaps to horizontal when there's no explicit dx.
                let dx = if dx_raw == 0.0 { dy } else { dx_raw };
                scroll_horiz(state, focus_system, dx);
            }
            ScrollMode::Both => {
                scroll_vert(state, focus_system);
                // If we won the horizontal claim but NOT the vertical one (a nested
                // horiz slider blocked vertical), remap dy → dx so the vertical
                // wheel still bubbles to our horizontal axis.
                let mut dx = dx_raw;
                if dx == 0.0 && dy != 0.0 {
                    let won_horiz = focus_system.is_active_scroll_left(state.id)
                        || focus_system.is_active_scroll_right(state.id);
                    let own_vert = focus_system.is_active_scroll_up(state.id)
                        || focus_system.is_active_scroll_down(state.id);
                    if won_horiz && !own_vert {
                        dx = dy;
                    }
                }
                scroll_horiz(state, focus_system, dx);
            }
        }
    }

    fn apply_page_keys(
        state: &mut ScrollState,
        mode: ScrollMode,
        content_bounds: Rect,
        focus_system: &mut FocusSystem,
        input: &Input,
    ) {
        if !input.key_pressed_page_up && !input.key_pressed_page_down {
            return;
        }
        let sign: f32 = if input.key_pressed_page_down {
            1.0
        } else {
            -1.0
        };
        let (is_pgup_vert, is_pgdn_vert) = (
            focus_system.is_active_pgup_vert(state.id),
            focus_system.is_active_pgdn_vert(state.id),
        );
        let (is_pgup_horiz, is_pgdn_horiz) = (
            focus_system.is_active_pgup_horiz(state.id),
            focus_system.is_active_pgdn_horiz(state.id),
        );
        let active_vert = if sign > 0.0 {
            is_pgdn_vert
        } else {
            is_pgup_vert
        };
        let active_horiz = if sign > 0.0 {
            is_pgdn_horiz
        } else {
            is_pgup_horiz
        };

        match mode {
            ScrollMode::None => {}
            ScrollMode::Vert => {
                if active_vert {
                    state.offset.y += sign * content_bounds.h;
                }
            }
            ScrollMode::Horiz => {
                if active_horiz {
                    state.offset.x += sign * content_bounds.w;
                }
            }
            ScrollMode::Both => {
                if active_vert {
                    state.offset.y += sign * content_bounds.h;
                }
                if active_horiz {
                    state.offset.x += sign * content_bounds.w;
                }
            }
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollLen {
    Px(f32),
    Viewport,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrollExtent {
    Exact(ScrollLen),
    AtMost(ScrollLen),
    Unbounded,
}

impl ScrollExtent {
    pub const FIT: Self = ScrollExtent::Exact(ScrollLen::Viewport);
    pub const SCROLL: Self = ScrollExtent::Unbounded;

    pub fn fixed(n: f32) -> Self {
        ScrollExtent::Exact(ScrollLen::Px(n))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    Auto,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAxis {
    pub extent: ScrollExtent,
    pub vis: ScrollbarVisibility,
}

impl Default for ScrollAxis {
    fn default() -> Self {
        Self {
            extent: ScrollExtent::FIT,
            vis: ScrollbarVisibility::Auto,
        }
    }
}

// ── State ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ScrollState {
    pub id: FocusId,
    pub offset: Vec2,
    pub vert_slider_state: crate::widgets::slider::SliderState,
    pub horiz_slider_state: crate::widgets::slider::SliderState,
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct ScrollAreaResult<'b, T: TextBackend, LS: LayoutState, CF> {
    pub layout: LayoutInfo,
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollAreaStyle {
    pub scrollbar_width: f32,
    pub scrollbar_style: SliderStyle,
    pub corner_color: Option<Color>,
}

impl ScrollAreaStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            scrollbar_width: theme.scrollbar_width,
            scrollbar_style: SliderStyle::scrollbar_from_theme(theme),
            corner_color: Some(theme.paper_elev),
        }
    }
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollAreaSpec {
    pub horizontal: ScrollAxis,
    pub vertical: ScrollAxis,
    pub style: ScrollAreaStyle,
    pub keyboard_focusable: bool,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScrollAreaSpecBuilder {
    pub horizontal: Option<ScrollAxis>,
    pub vertical: Option<ScrollAxis>,
    pub style: Option<ScrollAreaStyle>,
    pub keyboard_focusable: Option<bool>,
}

impl ScrollAreaSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn horizontal(mut self, axis: ScrollAxis) -> Self {
        self.horizontal = Some(axis);
        self
    }

    pub fn vertical(mut self, axis: ScrollAxis) -> Self {
        self.vertical = Some(axis);
        self
    }

    pub fn style(mut self, style: ScrollAreaStyle) -> Self {
        self.style = Some(style);
        self
    }

    pub fn keyboard_focusable(mut self, keyboard_focusable: bool) -> Self {
        self.keyboard_focusable = Some(keyboard_focusable);
        self
    }

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(ScrollAreaStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> ScrollAreaSpec {
        ScrollAreaSpec {
            horizontal: self.horizontal.unwrap_or_default(),
            vertical: self.vertical.unwrap_or(ScrollAxis {
                extent: ScrollExtent::SCROLL,
                vis: ScrollbarVisibility::Auto,
            }),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            keyboard_focusable: self.keyboard_focusable.unwrap_or(true),
        }
    }
}

/// What kind of scrolling this area supports, after resolving visibility +
/// degeneracy. The hover, wheel-routing, and pg* logic all branch on this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScrollMode {
    /// Neither axis is active. No claims, no wheel handling.
    None,
    /// Vertical only. Vertical wheel scrolls; horizontal scroll claims are
    /// taken purely to block a parent `Horiz` area from stealing the wheel.
    Vert,
    /// Horizontal only, possibly with a non-functional vertical scrollbar drawn
    /// (degenerate vertical). Vertical wheel remaps to horizontal scrolling;
    /// vertical scroll claims are taken to block a parent vertical area.
    Horiz,
    /// Both axes meaningful (needs_v && needs_h && max_scroll.y > 0).
    Both,
}

impl ScrollMode {
    fn resolve(needs_v: bool, needs_h: bool, max_scroll: Vec2) -> Self {
        // "Live" = scrollbar present AND content actually overflows. A scrollbar
        // drawn over fitting content (e.g. Always-visible with no overflow)
        // must NOT block a parent's wheel — there's nothing to scroll here.
        let live_v = needs_v && max_scroll.y > 0.0;
        let live_h = needs_h && max_scroll.x > 0.0;
        match (live_v, live_h) {
            (false, false) => ScrollMode::None,
            (true, false) => ScrollMode::Vert,
            (false, true) => ScrollMode::Horiz,
            (true, true) => ScrollMode::Both,
        }
    }
}

/// Pixels of scroll per wheel "line" (winit `LineDelta` unit).
///
/// Windows exposes the user setting via `SPI_GETWHEELSCROLLLINES` (default 3),
/// but the actual pixel size is up to the app. Browsers, GTK, and most editors
/// use 30–40 px/line; we pick 30. macOS and trackpads deliver pixel-precise
/// deltas via `PixelDelta` and the embedder is expected to convert to lines.
///
/// TODO: read `SPI_GETWHEELSCROLLLINES` (Windows) / equivalent (X11/Wayland)
/// when a cross-platform crate exposes it.
const SCROLL_PIXELS_PER_LINE: f32 = 30.0;

// ── High-level widget functions ───────────────────────────────────────────────────

/// High-level scroll area begin function using WidgetContext.
///
/// Resolves defaults, runs the raw pre-layout phase to obtain a `SizeRequest`,
/// resolves the outer rect, runs the raw begin phase to open the child scope,
/// and arranges for the raw end phase to run when the child context finishes.
///
/// Note there is no low-level end_scroll_area - everything is handled by the on_finish callback of the child context, which calls raw::end_scroll_area internally. This is because the scroll area must be ended on the same context it was begun on, and we want to allow users to simply drop the child context when finished without needing to manually call an end function.
#[allow(clippy::type_complexity)]
pub fn begin_scroll_area<'a, 'b, T: TextBackend, S: LayoutState, L: Layout, CF>(
    ctx: &'b mut WidgetContext<'a, T, S, CF>,
    builder: ScrollAreaSpecBuilder,
    layout_params: S::Params,
    state: &'b mut ScrollState,
    inner_layout: L,
) -> ScrollAreaResult<
    'b,
    T,
    crate::layouts::OffsetState<L::State>,
    impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect) + 'b,
> {
    let spec = builder.defaults_from_theme(&ctx.theme).build();
    let pre_layout_spec = raw::ScrollAreaPreLayoutSpec {};
    let offer = ctx.peek_offer(layout_params.clone());
    let pre_layout = raw::pre_layout_scroll_area(&pre_layout_spec, offer);
    let bounds = ctx.layout(layout_params, pre_layout.size_request);
    let raw_spec = raw::ScrollAreaSpec {
        rect: bounds,
        horizontal: spec.horizontal,
        vertical: spec.vertical,
        clip_rect: ctx.clip_rect,
        time: ctx.time,
        style: spec.style,
        layer: ctx.layer,
        keyboard_focusable: spec.keyboard_focusable,
    };
    let input = ctx.input;

    let raw::ScrollAreaResult {
        token,
        content_bounds,
        offset,
        inner_space,
    } = raw::begin_scroll_area(
        raw_spec,
        pre_layout,
        state,
        ctx.input,
        ctx.focus_system,
        ctx.cmds,
    );

    let offset_layout = crate::layouts::OffsetLayout {
        offset,
        inner: inner_layout,
    };

    let ctx_clip = ctx.clip_rect;
    let new_clip = Some(ctx_clip.map_or(content_bounds, |pc| pc.intersect(&content_bounds)));

    // The child context carries `state` and `input` into its cleanup closure;
    // `finish()` supplies the measured `content_extent`, and `end_scroll_area`
    // resolves all deferred scroll geometry (clamp, scrollbars, claims).
    let on_finish = move |focus_system: &mut FocusSystem,
                          _text_backend: &mut T,
                          cmds: &mut DrawCommands,
                          resolved_space: Rect| {
        let content_extent = Vec2::new(resolved_space.w, resolved_space.h);
        raw::end_scroll_area(token, content_extent, state, input, focus_system, cmds);
    };

    let child_ctx = ctx.child_with_layout_and_on_finish_and_clip_rect(
        offset_layout.begin(inner_space),
        on_finish,
        new_clip,
    );
    ScrollAreaResult {
        layout: LayoutInfo::new(bounds, content_bounds),
        ctx: child_ctx,
    }
}

#[cfg(test)]
#[path = "scroll_area_tests.rs"]
mod tests;
