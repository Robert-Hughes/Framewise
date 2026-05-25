use crate::{
    draw::DrawCmd,
    input::Input,
    types::{Rect, Vec2},
    widget::WidgetContext,
    layout::Layout,
};

pub mod raw {
    use super::*;

    /// Low-level scroll area begin function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn begin_scroll_area(
        bounds: Rect,
        content_size: Vec2,
        h_vis: ScrollbarVisibility,
        v_vis: ScrollbarVisibility,
        state: &mut ScrollState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        clip_rect: Option<Rect>,
        time: f64,
    ) -> (Vec<DrawCmd>, ScrollAreaScope, Rect, Vec2) {
        let mut pre_cmds = Vec::new();

        focus_sys.push_keyboard_scroll_scope(state.id);

        let is_visible = clip_rect.map_or(true, |clip| clip.contains(input.mouse_pos));
        let max_scroll = Vec2::new((content_size.x - bounds.w).max(0.0), (content_size.y - bounds.h).max(0.0));

        let needs_h = match h_vis {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::None => false,
            ScrollbarVisibility::Auto => max_scroll.x > 0.0,
        };
        let needs_v = match v_vis {
            ScrollbarVisibility::Always => true,
            ScrollbarVisibility::None => false,
            ScrollbarVisibility::Auto => max_scroll.y > 0.0,
        };

        let scrollbar_w = 12.0;
        let content_w = if needs_v { (bounds.w - scrollbar_w).max(0.0) } else { bounds.w };
        let content_h = if needs_h { (bounds.h - scrollbar_w).max(0.0) } else { bounds.h };
        let content_bounds = Rect::new(bounds.x, bounds.y, content_w, content_h);

        let mode = ScrollMode::resolve(needs_v, needs_h, max_scroll);

        if content_bounds.contains(input.mouse_pos) && is_visible {
            let at_top    = state.offset.y <= 0.0;
            let at_bottom = state.offset.y >= max_scroll.y;
            let at_left   = state.offset.x <= 0.0;
            let at_right  = state.offset.x >= max_scroll.x;

            // Same-axis: conditional on having room. Cross-axis: unconditional, to
            // block a parent of the other orientation from stealing the wheel.
            match mode {
                ScrollMode::None => {}
                ScrollMode::Vert => {
                    if !at_top    { focus_sys.claim_scroll_up(state.id); }
                    if !at_bottom { focus_sys.claim_scroll_down(state.id); }
                    focus_sys.claim_scroll_left(state.id);
                    focus_sys.claim_scroll_right(state.id);
                }
                ScrollMode::Horiz => {
                    if !at_left  { focus_sys.claim_scroll_left(state.id); }
                    if !at_right { focus_sys.claim_scroll_right(state.id); }
                    focus_sys.claim_scroll_up(state.id);
                    focus_sys.claim_scroll_down(state.id);
                }
                ScrollMode::Both => {
                    if !at_top    { focus_sys.claim_scroll_up(state.id); }
                    if !at_bottom { focus_sys.claim_scroll_down(state.id); }
                    if !at_left   { focus_sys.claim_scroll_left(state.id); }
                    if !at_right  { focus_sys.claim_scroll_right(state.id); }
                }
            }

            apply_wheel(state, mode, focus_sys, input);
        }

        apply_page_keys(state, mode, content_bounds, focus_sys, input);

        state.offset.x = state.offset.x.clamp(0.0, max_scroll.x);
        state.offset.y = state.offset.y.clamp(0.0, max_scroll.y);

        if needs_v {
            let view_ratio = if content_size.y > 0.0 { (content_bounds.h / content_size.y).min(1.0) } else { 1.0 };
            let track_rect = Rect::new(content_bounds.right(), bounds.y, scrollbar_w, content_bounds.h);

            let slider_spec = crate::widgets::slider::SliderSpec {
                orientation: crate::widgets::slider::Orientation::Vertical,
                rect: track_rect,
                min: 0.0,
                max: max_scroll.y,
                page_step: content_bounds.h,
                step: 40.0,
                thumb_size_ratio: Some(view_ratio),
                style: crate::widgets::slider::SliderStyle::scrollbar(),
                clip_rect,
                claim_scroll_at_ends: false,
            };

            let slider_cmds = crate::widgets::slider::slider_raw(
                &mut state.vert_slider_state,
                &mut state.offset.y,
                slider_spec,
                input,
                time,
                focus_sys,
            );
            pre_cmds.extend(slider_cmds);
        }

        if needs_h {
            let view_ratio = if content_size.x > 0.0 { (content_bounds.w / content_size.x).min(1.0) } else { 1.0 };
            let track_rect = Rect::new(bounds.x, content_bounds.bottom(), content_bounds.w, scrollbar_w);

            let slider_spec = crate::widgets::slider::SliderSpec {
                orientation: crate::widgets::slider::Orientation::Horizontal,
                rect: track_rect,
                min: 0.0,
                max: max_scroll.x,
                page_step: content_bounds.w,
                step: 40.0,
                thumb_size_ratio: Some(view_ratio),
                style: crate::widgets::slider::SliderStyle::scrollbar(),
                clip_rect,
                claim_scroll_at_ends: false,
            };

            let slider_cmds = crate::widgets::slider::slider_raw(
                &mut state.horiz_slider_state,
                &mut state.offset.x,
                slider_spec,
                input,
                time,
                focus_sys,
            );
            pre_cmds.extend(slider_cmds);
        }

        pre_cmds.push(DrawCmd::PushClip { rect: content_bounds });

        // at_* is snapshotted AFTER this frame's scroll actions are applied so that
        // reaching the limit on a press releases the corresponding claim next frame,
        // letting the very next PgUp/PgDn press bubble to the parent. Pre-action
        // snapshotting would force a release+repress to bubble.
        let scope = ScrollAreaScope {
            id: state.id,
            content_bounds,
            at_top: state.offset.y <= 0.0,
            at_bottom: state.offset.y >= max_scroll.y,
            at_left: state.offset.x <= 0.0,
            at_right: state.offset.x >= max_scroll.x,
            mode,
            is_finished: false,
        };

        (pre_cmds, scope, content_bounds, state.offset)
    }

    /// Low-level scroll area end function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn end_scroll_area(scope: ScrollAreaScope, focus_sys: &mut crate::focus::FocusSystem) -> Vec<DrawCmd> {
        scope.finish(focus_sys)
    }

    /// Route the wheel delta to the appropriate axis(es) based on mode, but only
    /// fire on claims this scope actually owns (so unconditional cross-axis claims
    /// don't double as a remap trigger).
    fn apply_wheel(
        state: &mut ScrollState,
        mode: ScrollMode,
        focus_sys: &mut crate::focus::FocusSystem,
        input: &Input,
    ) {
        let dy = input.scroll_delta.y;
        let dx_raw = input.scroll_delta.x;

        let scroll_vert = |state: &mut ScrollState, focus_sys: &crate::focus::FocusSystem| {
            if dy > 0.0 && focus_sys.is_active_scroll_up(state.id) {
                state.offset.y -= dy * SCROLL_PIXELS_PER_LINE;
            }
            if dy < 0.0 && focus_sys.is_active_scroll_down(state.id) {
                state.offset.y -= dy * SCROLL_PIXELS_PER_LINE;
            }
        };
        let scroll_horiz = |state: &mut ScrollState, focus_sys: &crate::focus::FocusSystem, dx: f32| {
            if dx > 0.0 && focus_sys.is_active_scroll_left(state.id) {
                state.offset.x -= dx * SCROLL_PIXELS_PER_LINE;
            }
            if dx < 0.0 && focus_sys.is_active_scroll_right(state.id) {
                state.offset.x -= dx * SCROLL_PIXELS_PER_LINE;
            }
        };

        match mode {
            ScrollMode::None => {}
            ScrollMode::Vert => scroll_vert(state, focus_sys),
            ScrollMode::Horiz => {
                // Vertical wheel remaps to horizontal when there's no explicit dx.
                let dx = if dx_raw == 0.0 { dy } else { dx_raw };
                scroll_horiz(state, focus_sys, dx);
            }
            ScrollMode::Both => {
                scroll_vert(state, focus_sys);
                // If we won the horizontal claim but NOT the vertical one (a nested
                // horiz slider blocked vertical), remap dy → dx so the vertical
                // wheel still bubbles to our horizontal axis.
                let mut dx = dx_raw;
                if dx == 0.0 && dy != 0.0 {
                    let won_horiz = focus_sys.is_active_scroll_left(state.id)
                        || focus_sys.is_active_scroll_right(state.id);
                    let own_vert = focus_sys.is_active_scroll_up(state.id)
                        || focus_sys.is_active_scroll_down(state.id);
                    if won_horiz && !own_vert {
                        dx = dy;
                    }
                }
                scroll_horiz(state, focus_sys, dx);
            }
        }
    }

    fn apply_page_keys(
        state: &mut ScrollState,
        mode: ScrollMode,
        content_bounds: Rect,
        focus_sys: &mut crate::focus::FocusSystem,
        input: &Input,
    ) {
        if !input.key_pressed_page_up && !input.key_pressed_page_down {
            return;
        }
        let sign: f32 = if input.key_pressed_page_down { 1.0 } else { -1.0 };
        let (is_pgup_vert, is_pgdn_vert) = (
            focus_sys.is_active_pgup_vert(state.id),
            focus_sys.is_active_pgdn_vert(state.id),
        );
        let (is_pgup_horiz, is_pgdn_horiz) = (
            focus_sys.is_active_pgup_horiz(state.id),
            focus_sys.is_active_pgdn_horiz(state.id),
        );
        let active_vert  = if sign > 0.0 { is_pgdn_vert }  else { is_pgup_vert };
        let active_horiz = if sign > 0.0 { is_pgdn_horiz } else { is_pgup_horiz };

        match mode {
            ScrollMode::None => {}
            ScrollMode::Vert => {
                if active_vert { state.offset.y += sign * content_bounds.h; }
            }
            ScrollMode::Horiz => {
                if active_horiz { state.offset.x += sign * content_bounds.w; }
            }
            ScrollMode::Both => {
                if active_vert  { state.offset.y += sign * content_bounds.h; }
                if active_horiz { state.offset.x += sign * content_bounds.w; }
            }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarVisibility {
    None,
    Auto,
    Always,
}

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub id: crate::focus::FocusId,
    pub offset: Vec2,
    pub vert_slider_state: crate::widgets::slider::SliderState,
    pub horiz_slider_state: crate::widgets::slider::SliderState,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            id: crate::focus::FocusId::new(),
            offset: Vec2::ZERO,
            vert_slider_state: crate::widgets::slider::SliderState::default(),
            horiz_slider_state: crate::widgets::slider::SliderState::default(),
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
            (true, false)  => ScrollMode::Vert,
            (false, true)  => ScrollMode::Horiz,
            (true, true)   => ScrollMode::Both,
        }
    }
}

pub struct ScrollAreaScope {
    pub id: crate::focus::FocusId,
    pub content_bounds: Rect,
    at_top: bool,
    at_bottom: bool,
    at_left: bool,
    at_right: bool,
    mode: ScrollMode,
    is_finished: bool,
}

impl Drop for ScrollAreaScope {
    fn drop(&mut self) {
        if !self.is_finished && !std::thread::panicking() {
            panic!("ScrollAreaScope dropped without calling finish()! This leaks focus state and clip rects.");
        }
    }
}

impl ScrollAreaScope {
    pub fn finish(
        mut self,
        focus_sys: &mut crate::focus::FocusSystem,
    ) -> Vec<DrawCmd> {
        self.is_finished = true;
        let mut post_cmds = Vec::new();
        post_cmds.push(DrawCmd::PopClip);

        let popped = focus_sys.pop_keyboard_scroll_scope();
        debug_assert_eq!(popped, Some(self.id), "ScrollAreaScope finished out of order!");

        if focus_sys.focused_scroll_path().contains(&self.id) {
            // Same-axis claims are conditional on having room to scroll, so a
            // child at its limit lets the parent's claim take effect (pg* is
            // first-caller-wins — see focus.rs). Cross-axis claims are
            // unconditional: they isolate this scope from the other axis,
            // preventing orthogonal keystrokes from leaking to a parent.
            match self.mode {
                ScrollMode::None => {}
                ScrollMode::Vert => {
                    if !self.at_top    { focus_sys.claim_pgup_vert(self.id); }
                    if !self.at_bottom { focus_sys.claim_pgdn_vert(self.id); }
                    focus_sys.claim_pgup_horiz(self.id);
                    focus_sys.claim_pgdn_horiz(self.id);
                }
                ScrollMode::Horiz => {
                    if !self.at_left  { focus_sys.claim_pgup_horiz(self.id); }
                    if !self.at_right { focus_sys.claim_pgdn_horiz(self.id); }
                    focus_sys.claim_pgup_vert(self.id);
                    focus_sys.claim_pgdn_vert(self.id);
                }
                ScrollMode::Both => {
                    if !self.at_top    { focus_sys.claim_pgup_vert(self.id); }
                    if !self.at_bottom { focus_sys.claim_pgdn_vert(self.id); }
                    if !self.at_left   { focus_sys.claim_pgup_horiz(self.id); }
                    if !self.at_right  { focus_sys.claim_pgdn_horiz(self.id); }
                }
            }
        }

        post_cmds
    }
}

// ── High-level widget functions ───────────────────────────────────────────────────

/// High-level scroll area begin function using WidgetContext.
///
/// This function accepts scroll parameters and calls the low-level raw::begin_scroll_area function.
/// High-level scroll area begin function using WidgetContext.
///
/// This function accepts scroll parameters, performs layout on the parent context,
/// and returns a child WidgetContext parameterized with an OffsetLayout, along with the scroll scope.
pub fn begin_scroll_area<'a, 'b, T: crate::text::TextSystem, S: crate::layout::LayoutState, L: crate::layout::Layout>(
    parent: &'b mut WidgetContext<'a, T, S>,
    layout_params: S::Params,
    content_size: Vec2,
    h_vis: ScrollbarVisibility,
    v_vis: ScrollbarVisibility,
    state: &'b mut ScrollState,
    inner_layout: L,
) -> WidgetContext<'b, T, crate::layout::OffsetState<L::State>> {
    let bounds = parent.layout(layout_params);
    let (pre_cmds, scope, content_bounds, offset) = raw::begin_scroll_area(
        bounds,
        content_size,
        h_vis,
        v_vis,
        state,
        parent.input,
        parent.focus_sys,
        parent.clip_rect,
        parent.time,
    );

    parent.append_cmds(pre_cmds);

    let offset_layout = crate::layout::OffsetLayout {
        offset,
        inner: inner_layout,
    };

    let parent_clip = parent.clip_rect;
    let new_clip = Some(parent_clip.map_or(content_bounds, |pc| pc.intersect(&content_bounds)));

    let mut child = parent.child_with_layout(offset_layout.begin(content_bounds));

    child.clip_rect = new_clip;
    child.scroll_scope = Some(scope); //TODO: assert non-empty?

    child
}

/// High-level scroll area end function using WidgetContext.
///
/// This function accepts finished child commands and a ScrollAreaScope and completes the scroll area on the parent context.
// pub fn end_scroll_area<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
//     parent: &mut WidgetContext<T, S>,
//     cmds: Vec<crate::draw::DrawCmd>,
//     scope: ScrollAreaScope,
// ) {
//     parent.append_cmds(cmds);
//     let post_cmds = raw::end_scroll_area(scope, parent.focus_sys);
//     parent.append_cmds(post_cmds);
// }

// ── Re-export raw functions for direct use ───────────────────────────────────────────
pub use raw::{begin_scroll_area as begin_scroll_area_raw, end_scroll_area as end_scroll_area_raw};


#[cfg(test)]
mod test_helpers {
    use crate::focus::FocusSystem;



    /// Run `n` frames against `focus_sys`, wrapping each in begin/end_frame.
    /// `body` receives the frame index and the FocusSystem.
    pub fn frames(focus_sys: &mut FocusSystem, n: usize, mut body: impl FnMut(usize, &mut FocusSystem)) {
        for frame in 0..n {
            focus_sys.begin_frame();
            body(frame, focus_sys);
            focus_sys.end_frame();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::raw::begin_scroll_area;
    use super::test_helpers::frames;
    use crate::test_utils::DummyTextSys;
    use crate::layout::ManualLayout;

    // Helper to keep test calls the same
    fn scroll_area(
        bounds: Rect,
        content_size: Vec2,
        state: &mut ScrollState,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        clip_rect: Option<Rect>,
        time: f64,
    ) -> (Vec<DrawCmd>, Rect, crate::layout::OffsetLayout<crate::layout::ManualLayout>) {
        let (mut pre_cmds, scope, cb, scroll_offset) = raw::begin_scroll_area(
            bounds, content_size,
            ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
            state, input, focus_sys, clip_rect, time
        );
        let post_cmds = scope.finish(focus_sys);
        pre_cmds.extend(post_cmds);
        let layout = crate::layout::OffsetLayout { offset: scroll_offset, inner: ManualLayout };
        (pre_cmds, cb, layout)
    }

    #[test]
    fn test_scroll_area_math() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let input = Input::new();
        let mut focus_sys = crate::focus::FocusSystem::new();

        let (_, content_bounds, layout) = scroll_area(
            bounds, Vec2::new(200.0, 400.0), &mut state, &input, &mut focus_sys, None, 0.0
        );

        assert_eq!(content_bounds.w, 188.0);
        assert_eq!(layout.offset.y, 0.0);
    }

    fn nested_scroll_two_frames(
        outer_state: &mut ScrollState,
        inner_state: &mut ScrollState,
        outer_content_h: f32,
        inner_content_h: f32,
        outer_bounds: Rect,
        inner_bounds: Rect,
        wheel_delta_y: f32,
        mouse_pos: Vec2,
    ) {
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, wheel_delta_y);
        input.mouse_pos = mouse_pos;
        let mut focus_sys = crate::focus::FocusSystem::new();

        for _ in 0..2 {
            focus_sys.begin_frame();
            let (_, outer_scope, cb, _) = begin_scroll_area(
                outer_bounds, Vec2::new(outer_bounds.w, outer_content_h),
                ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
                outer_state, &input, &mut focus_sys, None, 0.0
            );

            let (_, inner_scope, _, _) = begin_scroll_area(
                inner_bounds, Vec2::new(inner_bounds.w, inner_content_h),
                ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
                inner_state, &input, &mut focus_sys, Some(cb), 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
    }

    #[test]
    fn test_nested_scroll_areas() {
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);

        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            -1.0,
            Vec2::new(50.0, 50.0),
        );

        assert!(inner_state.offset.y > 0.0, "Inner scroll should process input first");
        assert_eq!(outer_state.offset.y, 0.0, "Outer scroll should remain at 0");
    }

        #[test]
    fn test_pgup_pgdn_horiz_uses_vert_wheel() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();

        let mut input = Input::new();
        input.key_pressed_page_down = true;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut btn_state = crate::widgets::button::ButtonState::default();

        let mut text_sys = DummyTextSys;

        focus_sys.take_focus(btn_state.focus_id);

        for _ in 0..2 {
            focus_sys.begin_frame();
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(400.0, 200.0),
                ScrollbarVisibility::Auto, ScrollbarVisibility::None,
                &mut state, &input, &mut focus_sys, None, 0.0
            );

            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    text: "dummy".into(),
                    style: crate::widgets::button::ButtonStyle::default(),
                    clip_rect: None,
                    disabled: false,
                },
                &input,
                &mut text_sys,
                &mut focus_sys
            );
            btn_state = info.state;

            scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }

        assert_eq!(state.offset.y, 0.0);
        assert_eq!(state.offset.x, 200.0);
    }

    /// PgDn should advance one viewport — `content_bounds.h`, not `bounds.h`.
    /// In a 2D area, the horizontal scrollbar steals 12px of height, so
    /// content_bounds.h = bounds.h - scrollbar_w. Using bounds.h overshoots by 12px.
    #[test]
    fn test_pgdn_step_uses_content_bounds() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut input = Input::new();
        input.key_pressed_page_down = true;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut btn_state = crate::widgets::button::ButtonState::default();

        let mut text_sys = DummyTextSys;
focus_sys.take_focus(btn_state.focus_id);

        // 2D: vertical scrollbar visible (steals width) AND horizontal scrollbar visible (steals height).
        // content_bounds = (0,0,188,188). PgDn step must be 188, not bounds.h=200.
        for _ in 0..2 {
            focus_sys.begin_frame();
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(1000.0, 1000.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    text: "".into(),
                    style: Default::default(),
                    clip_rect: None,
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_state = info.state;
            scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(state.offset.y, 188.0, "PgDn must advance one content-viewport, not full bounds");
    }

    /// Focused widget lives OUTSIDE any scroll scope. PgDn must not scroll the
    /// nearby scroll area (it's not in focus's path).
    #[test]
    fn test_pgdn_with_focus_outside_scope() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut btn_state = crate::widgets::button::ButtonState::default();

        let mut text_sys = DummyTextSys;
        focus_sys.take_focus(btn_state.focus_id);

        for _ in 0..2 {
            focus_sys.begin_frame();
            let mut input = Input::new();
            input.key_pressed_page_down = true;

            // Button rendered OUTSIDE the scroll area's begin/finish.
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(500.0, 500.0, 10.0, 10.0),
                    text: "".into(),
                    style: Default::default(),
                    clip_rect: None,
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_state = info.state;

            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut state, &input, &mut focus_sys, None, 0.0,
            );
            scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(state.offset.y, 0.0, "Focus outside scope must not drive pgdn into the area");
    }

    /// Combined input: an active slider drag plus a wheel tick in the same frame.
    /// The drag is authoritative — drag math sets `offset` last, so wheel changes
    /// applied earlier are overwritten and the result tracks the mouse exactly.
    #[test]
    fn test_slider_drag_with_wheel_drag_wins() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 0: press on vertical thumb (drag start). Thumb at top of track,
        // track at x=188..200 y=0..200. Thumb pos = 0, length ratio = 200/1000.
        focus_sys.begin_frame();
        let mut input = Input::new();
        input.mouse_pos = Vec2::new(194.0, 5.0);
        input.mouse_pressed = true;
        input.mouse_down = true;
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 1000.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        assert!(state.vert_slider_state.is_dragging, "Drag must be active");

        // Frame 1: held, mouse moved down, AND a wheel tick. Drag should win.
        focus_sys.begin_frame();
        let mut input = Input::new();
        input.mouse_pos = Vec2::new(194.0, 50.0);
        input.mouse_down = true;
        input.scroll_delta.y = -5.0; // would drive offset way down if it applied
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 1000.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        // drag: usable_track = 200 - (200 * 200/1000).max(20) = 200-40=160. delta=45 → val_delta=(45/160)*800≈225.
        let expected = (45.0 / 160.0) * 800.0;
        let actual = state.offset.y;
        let diff = (actual - expected).abs();
        assert!(diff < 1.0, "offset {} ≈ drag-projected {} (drag dominates wheel)", actual, expected);
    }

    /// Auto-visibility with content that fits: no scrollbar drawn, no claims,
    /// parent wheel must pass through.
    #[test]
    fn test_auto_degenerate_does_not_block_parent() {
        let mut outer = ScrollState::default();
        let mut inner = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        for frame in 0..3 {
            focus_sys.begin_frame();
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { -1.0 } else { 0.0 };

            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer, &input, &mut focus_sys, None, 0.0,
            );
            // Inner Auto+fits — no scrollbar, no claim.
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 200.0),
                ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
                &mut inner, &input, &mut focus_sys, None, 0.0,
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner.offset.y, 0.0);
        assert!(outer.offset.y > 0.0, "Outer should scroll through fit-content Auto child");
    }

    /// Always-visible scrollbars over content that fits: scrollbars drawn but
    /// they are no-ops — must NOT block a parent's wheel either.
    #[test]
    fn test_always_visible_but_fits_does_not_block_parent() {
        let mut outer = ScrollState::default();
        let mut inner = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        for frame in 0..3 {
            focus_sys.begin_frame();
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { -1.0 } else { 0.0 };

            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer, &input, &mut focus_sys, None, 0.0,
            );
            // Inner Always+fits: scrollbars drawn, no scroll possible.
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(150.0, 150.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner, &input, &mut focus_sys, None, 0.0,
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner.offset.y, 0.0);
        assert!(outer.offset.y > 0.0, "Outer should scroll past inner's no-op scrollbars");
    }

    /// Mouse entirely outside the scroll area's bounds: no claim, no scroll.
    #[test]
    fn test_mouse_outside_bounds_no_scroll() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        frames(&mut focus_sys, 3, |frame, fs| {
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(500.0, 500.0); // far outside
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut state, &input, fs, None, 0.0,
            );
            scope.finish(fs);
        });
        assert_eq!(state.offset.y, 0.0, "Wheel outside bounds must not scroll");
    }

    /// Two sibling scroll areas, mouse only over one. The hovered one consumes
    /// the wheel; the other must not move.
    #[test]
    fn test_sibling_scroll_areas_dont_steal() {
        let mut a = ScrollState::default();
        let mut b = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        frames(&mut focus_sys, 3, |frame, fs| {
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(50.0, 50.0); // inside A only
            input.scroll_delta.y = if frame == 1 { -1.0 } else { 0.0 };

            let (_, scope_a, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut a, &input, fs, None, 0.0,
            );
            scope_a.finish(fs);

            let (_, scope_b, _, _) = begin_scroll_area(
                Rect::new(300.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut b, &input, fs, None, 0.0,
            );
            scope_b.finish(fs);
        });
        assert!(a.offset.y > 0.0, "Hovered sibling A should scroll");
        assert_eq!(b.offset.y, 0.0, "Non-hovered sibling B must not scroll");
    }

    /// 2D area with both scroll_delta.x and scroll_delta.y simultaneously
    /// (trackpad pan or shift-wheel): both axes advance independently.
    #[test]
    fn test_simultaneous_dx_and_dy() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        frames(&mut focus_sys, 3, |frame, fs| {
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta = if frame == 1 { Vec2::new(-1.0, -1.0) } else { Vec2::ZERO };
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(1000.0, 1000.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut state, &input, fs, None, 0.0,
            );
            scope.finish(fs);
        });
        assert!(state.offset.x > 0.0, "dx should advance horizontal");
        assert!(state.offset.y > 0.0, "dy should advance vertical");
        assert_eq!(state.offset.x, state.offset.y, "equal-magnitude deltas → equal offsets");
    }

    /// When content shrinks, an existing offset past the new max must clamp.
    #[test]
    fn test_offset_clamps_on_content_shrink() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        state.offset.y = 500.0; // ahead of any plausible max
        let input = Input::new();
        let mut focus_sys = crate::focus::FocusSystem::new();

        focus_sys.begin_frame();
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 250.0), // content shrunk: max_scroll.y = 50
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        assert_eq!(state.offset.y, 50.0, "Offset must clamp to new max_scroll.y");
    }

    /// Non-zero bounds.x/y must shift content_bounds, mouse hit-test, and the
    /// slider track. Mouse hit at the absolute coordinate inside the offset
    /// content_bounds should still trigger scroll.
    #[test]
    fn test_non_zero_bounds_origin() {
        let bounds = Rect::new(100.0, 200.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        for frame in 0..3 {
            focus_sys.begin_frame();
            let mut input = Input::new();
            // Mouse inside the offset content_bounds (100..288, 200..400).
            input.mouse_pos = Vec2::new(150.0, 250.0);
            input.scroll_delta.y = if frame == 1 { -1.0 } else { 0.0 };
            let (_, scope, cb, _) = begin_scroll_area(
                bounds, Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut state, &input, &mut focus_sys, None, 0.0,
            );
            // content_bounds origin must follow bounds origin.
            assert_eq!(cb.x, 100.0);
            assert_eq!(cb.y, 200.0);
            scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert!(state.offset.y > 0.0, "Wheel inside offset content_bounds should scroll");

        // Mouse just outside the offset bounds must NOT scroll.
        let mut state2 = ScrollState::default();
        let mut focus_sys2 = crate::focus::FocusSystem::new();
        for frame in 0..3 {
            focus_sys2.begin_frame();
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(50.0, 250.0); // x<bounds.x → outside
            input.scroll_delta.y = if frame == 1 { -1.0 } else { 0.0 };
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(200.0, 1000.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut state2, &input, &mut focus_sys2, None, 0.0,
            );
            scope.finish(&mut focus_sys2);
            focus_sys2.end_frame();
        }
        assert_eq!(state2.offset.y, 0.0, "Mouse left of bounds.x must not claim the wheel");
    }

    /// The corner where both scrollbars meet is an intentional dead zone:
    /// mouse there is outside content_bounds so no claim is made and the wheel
    /// does not scroll.
    #[test]
    fn test_scrollbar_corner_is_dead_zone() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        // 2D scroll area: content_bounds=(0,0,188,188). Corner is (188..200, 188..200).
        frames(&mut focus_sys, 3, |frame, fs| {
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(194.0, 194.0); // inside corner, outside content_bounds
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(1000.0, 1000.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut state, &input, fs, None, 0.0,
            );
            scope.finish(fs);
        });
        assert_eq!(state.offset.y, 0.0, "Wheel in scrollbar corner dead zone must not scroll");
        assert_eq!(state.offset.x, 0.0, "Wheel in scrollbar corner dead zone must not scroll horizontally");
    }

    /// clip_rect masks hover-driven scroll: mouse inside content_bounds but outside
    /// the clip → no claim, no scroll.
    #[test]
    fn test_clip_rect_masks_scroll() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();

        // Clip excludes the bottom-right quadrant. Mouse lands at (150,150) — inside
        // content_bounds but outside this clip.
        let clip = Rect::new(0.0, 0.0, 100.0, 100.0);

        frames(&mut focus_sys, 3, |frame, fs| {
            let mut input = Input::new();
            input.mouse_pos = Vec2::new(150.0, 150.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            let (_, scope, _, _) = raw::begin_scroll_area(
                bounds, Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut state, &input, fs, Some(clip), 0.0,
            );
            scope.finish(fs);
        });
        assert_eq!(state.offset.y, 0.0, "Wheel outside clip must not scroll");
    }

    /// Spatial navigation must not land on a widget that is clipped by its scroll area.
    ///
    /// A button inside a scroll area but below the visible viewport (content overflow)
    /// registers at a screen-space rect between the clip boundary and btn_start. Its
    /// axial distance from btn_start is smaller than the visible button's, so the
    /// spatial algorithm currently picks it — even though it is not on screen.
    ///
    /// Layout (screen-space y, all x-aligned so lateral gap = 0):
    ///   0..100   scroll area viewport / clip
    ///     20..50   btn_visible  (inside clip, score 180 from btn_start)
    ///   120..150   btn_clipped  (content overflows below clip, score 80 — WINS WRONGLY)
    ///  200..230   btn_start    (outside scroll area, focused)
    ///
    /// Up from btn_start: currently picks btn_clipped (lower score). Should pick btn_visible.
    #[test]
    fn test_spatial_nav_skips_widget_clipped_by_scroll_area() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut scroll_state = ScrollState::default();
        let mut btn_visible_state = crate::widgets::button::ButtonState::default();
        let mut btn_clipped_state = crate::widgets::button::ButtonState::default();
        let mut btn_start_state = crate::widgets::button::ButtonState::default();
        let mut text_sys = DummyTextSys;

        let btn_visible_id = btn_visible_state.focus_id;

        focus_sys.take_focus(btn_start_state.focus_id);

        // Scroll area: y=0..100, content 300px tall — overflows without scrolling.
        // needs_v=true (Auto), needs_h=false (None) → content_bounds = (0,0,188,100).
        let scroll_bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let content_size = Vec2::new(200.0, 300.0);

        for frame in 0..2 {
            let mut input = Input::new();
            if frame == 1 {
                input.key_pressed_up = true;
            }

            focus_sys.begin_frame();

            let (_, scope, content_bounds, _) = begin_scroll_area(
                scroll_bounds, content_size,
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut scroll_state, &input, &mut focus_sys, None, 0.0,
            );

            // btn_visible: inside the clip rect (y=20..50, clip y=0..100).
            let res = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_visible_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 20.0, 80.0, 30.0),
                    text: "visible".into(),
                    style: Default::default(),
                    clip_rect: Some(content_bounds),
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_visible_state = res.state;

            // btn_clipped: OUTSIDE the clip rect (y=120..150, clip y=0..100).
            // Screen rect is in the gap between the scroll area and btn_start —
            // axial score from btn_start = 80, beating btn_visible's 180.
            let res = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_clipped_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 120.0, 80.0, 30.0),
                    text: "clipped".into(),
                    style: Default::default(),
                    clip_rect: Some(content_bounds),
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_clipped_state = res.state;

            scope.finish(&mut focus_sys);

            // btn_start: below the scroll area, no clip.
            let res = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_start_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 200.0, 80.0, 30.0),
                    text: "start".into(),
                    style: Default::default(),
                    clip_rect: None,
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_start_state = res.state;

            focus_sys.end_frame();
        }

        assert_eq!(
            focus_sys.current_focus(),
            Some(btn_visible_id),
            "Up from btn_start must skip the clipped widget and land on btn_visible"
        );
    }

    /// A widget that partially overlaps the scroll area clip rect is still
    /// reachable via directional navigation — only fully-clipped widgets are excluded.
    ///
    /// Layout (screen-space y):
    ///   0..100   scroll area viewport / clip (content_bounds with v-scrollbar = 0..100)
    ///     70..100  btn_partial (y=70..100, exactly at bottom edge — 30px visible)
    ///  150..180   btn_start  (below scroll area, focused)
    ///
    /// Up from btn_start must land on btn_partial (partially visible, not excluded).
    #[test]
    fn test_spatial_nav_reaches_partially_clipped_widget() {
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut scroll_state = ScrollState::default();
        let mut btn_partial_state = crate::widgets::button::ButtonState::default();
        let mut btn_start_state = crate::widgets::button::ButtonState::default();
        let mut text_sys = DummyTextSys;

        let btn_partial_id = btn_partial_state.focus_id;

        focus_sys.take_focus(btn_start_state.focus_id);

        // Scroll area: y=0..100, content 300px tall → clip = (0,0,188,100).
        let scroll_bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        let content_size = Vec2::new(200.0, 300.0);

        for frame in 0..2 {
            let mut input = Input::new();
            if frame == 1 {
                input.key_pressed_up = true;
            }

            focus_sys.begin_frame();

            let (_, scope, content_bounds, _) = begin_scroll_area(
                scroll_bounds, content_size,
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut scroll_state, &input, &mut focus_sys, None, 0.0,
            );

            // btn_partial: y=70..100 — the bottom edge exactly meets the clip boundary.
            // 30px overlap → must be included in spatial nav.
            let res = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_partial_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 70.0, 80.0, 30.0),
                    text: "partial".into(),
                    style: Default::default(),
                    clip_rect: Some(content_bounds),
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_partial_state = res.state;

            scope.finish(&mut focus_sys);

            // btn_start: below the scroll area.
            let res = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_start_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 150.0, 80.0, 30.0),
                    text: "start".into(),
                    style: Default::default(),
                    clip_rect: None,
                    disabled: false,
                },
                &input, &mut text_sys, &mut focus_sys,
            );
            btn_start_state = res.state;

            focus_sys.end_frame();
        }

        assert_eq!(
            focus_sys.current_focus(),
            Some(btn_partial_id),
            "Up from btn_start must reach the partially-visible widget"
        );
    }

    /// Home/End act when the scrollbar slider is focused (slider's own keyboard handler).
    /// They do not propagate from child widgets via the scope — that's intentional.
    #[test]
    fn test_home_end_on_focused_slider() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let mut focus_sys = crate::focus::FocusSystem::new();
        let input = Input::new();

        // Pre-render to materialise the vertical slider's focus_id.
        focus_sys.begin_frame();
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 1000.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        focus_sys.take_focus(state.vert_slider_state.focus_id);

        // End → offset jumps to max.
        let mut input = Input::new();
        input.key_pressed_end = true;
        focus_sys.begin_frame();
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 1000.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        assert_eq!(state.offset.y, 800.0, "End on focused slider jumps to max_scroll");

        // Home → offset jumps to 0.
        let mut input = Input::new();
        input.key_pressed_home = true;
        focus_sys.begin_frame();
        let (_, scope, _, _) = begin_scroll_area(
            bounds, Vec2::new(200.0, 1000.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut state, &input, &mut focus_sys, None, 0.0,
        );
        scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        assert_eq!(state.offset.y, 0.0, "Home on focused slider jumps to 0");
    }
}




#[cfg(test)]
mod nested_bubbling_tests {
    use crate::widgets::scroll_area::*;
    use crate::widgets::scroll_area::raw::begin_scroll_area;
    use crate::types::*;
    use crate::input::Input;

    use crate::focus::*;

    // 1. Mouse Wheel / Inner Content / Same-axis (Bubble)
    #[test]
    fn test_nested_mouse_content_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0); // Hover content
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 }; // Scroll up
            if frame == 0 {
                inner_state.offset.y = 0.0; // Inner at top
                outer_state.offset.y = 100.0; // Outer has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0);
        assert_eq!(outer_state.offset.y, 70.0, "Should bubble same-axis");
    }

    // 2. Mouse Wheel / Inner Content / Cross-axis (Isolate)
    #[test]
    fn test_nested_mouse_content_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0); // Hover content
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 }; // Scroll up
            if frame == 0 {
                inner_state.offset.x = 0.0; // Inner horizontal at left
                outer_state.offset.y = 100.0; // Outer vertical has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0);
        assert_eq!(outer_state.offset.y, 100.0, "Should not leak cross-axis");
    }

    // 3. Mouse Wheel / Slider Track / Same-axis (Bubble)
    #[test]
    fn test_nested_mouse_scrollbar_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(195.0, 50.0); // Hover inner vertical scrollbar
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;
                outer_state.offset.y = 100.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0);
        assert_eq!(outer_state.offset.y, 70.0, "Should bubble same-axis");
    }

    // 4. Mouse Wheel / Slider Track / Cross-axis (Isolate)
    #[test]
    fn test_nested_mouse_scrollbar_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 195.0); // Hover inner horizontal scrollbar
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.x = 0.0;
                outer_state.offset.y = 100.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0);
        assert_eq!(outer_state.offset.y, 100.0, "Should not leak cross-axis");
    }

    use crate::test_utils::DummyTextSys;

    // 5. Keyboard / Inner Content / Same-axis (Bubble)
    #[test]
    fn test_nested_keyboard_content_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = if frame == 1 { true } else { false };
            if frame == 0 {
                inner_state.offset.y = 100.0; // At bottom
                outer_state.offset.y = 0.0; // Has room to scroll down
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0), // max scroll = 100
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 100.0);
        assert_eq!(outer_state.offset.y, 400.0, "Should bubble same-axis");
    }

    // 6. Keyboard / Inner Content / Cross-axis (Isolate)
    #[test]
    fn test_nested_keyboard_content_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = if frame == 1 { true } else { false };
            if frame == 0 {
                inner_state.offset.x = 100.0; // Inner horiz at bottom
                outer_state.offset.y = 0.0; // Outer vert has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(300.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 100.0);
        assert_eq!(outer_state.offset.y, 0.0, "Should isolate cross-axis");
    }

    // 7. Keyboard / Slider Track / Same-axis (Bubble)
    #[test]
    fn test_nested_keyboard_scrollbar_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // Let's grab the focus ID of the inner scrollbar by rendering it once
        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();

        focus_sys.take_focus(inner_state.vert_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = if frame == 1 { true } else { false };
            if frame == 0 {
                inner_state.offset.y = 100.0; // At bottom
                outer_state.offset.y = 0.0; // Has room to scroll down
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 100.0);
        assert_eq!(outer_state.offset.y, 400.0, "Should bubble same-axis");
    }

    // 8. Keyboard / Slider Track / Cross-axis (Isolate)
    #[test]
    fn test_nested_keyboard_scrollbar_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(300.0, 200.0),
            ScrollbarVisibility::Always, ScrollbarVisibility::None,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();

        focus_sys.take_focus(inner_state.horiz_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = if frame == 1 { true } else { false };
            if frame == 0 {
                inner_state.offset.x = 100.0; // At right
                outer_state.offset.y = 0.0; // Has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(300.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 100.0);
        assert_eq!(outer_state.offset.y, 0.0, "Should isolate cross-axis");
    }

    // ── Reversed axis: outer HORIZONTAL, inner VERTICAL ──────────────────────
    //
    // Bug: outer_horiz always wins scroll_left (inner_vert never claims it).
    // The horiz_uses_vert_wheel path maps delta.y → dx and fires whenever is_active_scroll_left
    // is true, so outer scrolls horizontally at the same time as inner scrolls
    // vertically — both fire on every vertical wheel tick.

    // 9. Mouse Wheel / Outer Horiz → Inner Vert / Content area, inner at top (at_min)
    //    Vertical wheel on inner vert content when inner is already at top.
    //    Bug: inner content block skips claim_scroll_up (at_top), so outer retains
    //    active_scroll_up from its horiz_uses_vert_wheel claim and fires via horiz_uses_vert_wheel dx=delta.y.
    #[test]
    fn test_outer_horiz_inner_vert_mouse_content_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // outer: (0,0,400,200) horiz-only, content 800w; content_bounds=(0,0,400,188)
        // inner: (0,0,200,200) vert-only,  content 400h; content_bounds=(0,0,188,200)
        // mouse (50,50): inside both content areas
        // inner.offset.y=0 (at_top): content block skips claim_scroll_up → outer retains it
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0; // at top — triggers the bug
                outer_state.offset.x = 50.0; // should NOT change
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner vert already at top, cannot scroll further");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll from vertical wheel on inner vert content");
    }

    // 10. Mouse Wheel / Outer Horiz → Inner Vert / Scrollbar track, inner at top (at_min)
    //     Vertical wheel on inner vert scrollbar when inner is already at top.
    //     Bug: inner slider doesn't claim scroll_up when at_min (conditional claim), so outer
    //     retains active_scroll_up from its horiz_uses_vert_wheel claim and fires via horiz_uses_vert_wheel dx=delta.y.
    //
    //     The bug is NOT visible when inner is mid-scroll (inner slider claims scroll_up,
    //     overwriting outer's claim). It only triggers at the limit — matching what the
    //     sample app demonstrates.
    #[test]
    fn test_outer_horiz_inner_vert_mouse_scrollbar_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // inner vert slider track: x=188..200, y=0..200
        // mouse (195,50): in outer content_bounds (0,0,400,188), outside inner content_bounds (0,0,188,200)
        // inner.offset.y=0 (at_min): slider skips claim_scroll_up → outer retains active_scroll_up
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(195.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0; // at top — triggers the bug
                outer_state.offset.x = 50.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner vert already at top, cannot scroll further");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll from vertical wheel on inner vert scrollbar");
    }

    // 11. Keyboard / Outer Horiz → Inner Vert / Content focus, inner at bottom
    //     pgdn with inner at bottom → outer horiz should NOT receive pgdn (cross-axis).
    //     Bug: outer_horiz.finish() claims pgdn via horiz_uses_vert_wheel (!at_right) and scrolls right.
    #[test]
    fn test_outer_horiz_inner_vert_keyboard_content_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 100.0; // at bottom (content=300, view=200 → max=100)
                outer_state.offset.x = 0.0;   // outer has room right
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 100.0, "Inner vert already at bottom");
        assert_eq!(outer_state.offset.x, 0.0, "Outer horiz must NOT receive pgdn (cross-axis isolation)");
    }

    // 12. Keyboard / Outer Horiz → Inner Vert / Slider focus, slider at max
    //     pgdn with inner vert slider at max → outer horiz must NOT scroll right.
    //     Bug: slider doesn't claim, inner scope doesn't claim, outer claims via horiz_uses_vert_wheel.
    #[test]
    fn test_outer_horiz_inner_vert_keyboard_scrollbar_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();

        focus_sys.take_focus(inner_state.vert_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 100.0; // at max
                outer_state.offset.x = 0.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 100.0, "Inner slider at max, should not change");
        assert_eq!(outer_state.offset.x, 0.0, "Outer horiz must NOT receive pgdn (cross-axis isolation)");
    }

    // ── Triple nested: outer VERTICAL → middle HORIZONTAL → inner VERTICAL ───────────────
    //
    // Geometry (all tests):
    //   outer_vert:   bounds=(0,0,400,400)  content=(400,800)  v=Always h=None
    //                 content_bounds=(0,0,388,400)  vert-slider at x=388
    //   middle_horiz: bounds=(0,0,388,300)  content=(800,300)  h=Always v=None  [horiz_uses_vert_wheel]
    //                 content_bounds=(0,0,388,288)  horiz-slider at y=288
    //   inner_vert:   bounds=(0,0,200,288)  content=(200,600)  v=Always h=None
    //                 content_bounds=(0,0,188,288)  vert-slider at x=188
    //                 max_scroll.y = 312
    //
    // Middle_horiz is a "horiz_uses_vert_wheel" area (h-only). Its unconditional scroll_up/down claims block
    // events from reaching outer_vert, and inner_vert's unconditional scroll_left/right claims
    // prevent middle from acting via the scroll_left action path.
    // Result: middle_horiz is a complete bidirectional blocker — outer_vert never fires.
    //
    // WHY THIS MATTERS: if events could tunnel through middle_horiz and reach outer_vert, the
    // user would see vertical scrolling in a container they've mentally left. Worse, it would
    // skip the axis change entirely: a vertical action taken while inside a horizontally-scrolling
    // context would scroll something vertically above it — a disorienting axis jump that breaks
    // the user's spatial model of which scroll area they are controlling.

    // 13. Mouse Wheel / Triple Nested / Inner content, inner at top
    //     Upward wheel on inner_vert content while inner is at top.
    //     Middle absorbs scroll_up (horiz_uses_vert_wheel claim). Inner claims scroll_left (blocks middle action).
    //     Outer never reached. If middle failed to block, outer_vert would scroll vertically —
    //     skipping the horizontal context entirely, confusing axis jump.
    #[test]
    fn test_triple_nested_mouse_content_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0); // inside all three content areas
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at top — skips scroll_up claim
                middle_state.offset.x = 50.0;  // must NOT scroll
                outer_state.offset.y = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner vert already at top");
        assert_eq!(middle_state.offset.x, 50.0, "Middle horiz must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.y, 50.0, "Outer vert must NOT scroll (middle fully blocks)");
    }

    // 14. Mouse Wheel / Triple Nested / Inner slider, inner slider at top (at_min)
    //     Upward wheel on inner_vert slider track while slider is at_min.
    //     Slider skips scroll_up (conditional), claims scroll_left (unconditional).
    //     Middle absorbs scroll_up but cannot act (scroll_left taken, scroll_up path removed).
    //     Outer never reached. If middle failed to block, outer_vert would scroll vertically —
    //     skipping the horizontal context entirely, confusing axis jump.
    #[test]
    fn test_triple_nested_mouse_scrollbar_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // mouse (195,50): inside outer (0,0,388,400) and middle (0,0,388,288) content areas,
        //                 outside inner content (0,0,188,288), on inner slider track (188,0,12,288)
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(195.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at_min — slider skips scroll_up claim
                middle_state.offset.x = 50.0;  // must NOT scroll
                outer_state.offset.y = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner vert already at top");
        assert_eq!(middle_state.offset.x, 50.0, "Middle horiz must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.y, 50.0, "Outer vert must NOT scroll (middle fully blocks)");
    }

    // 15. Keyboard / Triple Nested / Content focus, inner at bottom
    //     pgdn with focus inside inner_vert at bottom.
    //     Inner scope claims pgdn_horiz (blocks middle). Middle scope claims pgdn_vert (blocks outer).
    //     Outer never reached. If middle failed to block, outer_vert would scroll down — the user
    //     pressed pgdn expecting the inner context and outer_vert jumps instead, an axis skip.
    #[test]
    fn test_triple_nested_keyboard_content_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 312.0;  // at bottom (content=600, view=288 → max=312)
                middle_state.offset.x = 50.0;  // must NOT scroll right
                outer_state.offset.y = 50.0;   // must NOT scroll down
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 312.0, "Inner vert already at bottom");
        assert_eq!(middle_state.offset.x, 50.0, "Middle horiz must NOT scroll right (cross-axis)");
        assert_eq!(outer_state.offset.y, 50.0, "Outer vert must NOT scroll down (middle fully blocks)");
    }

    // 16. Keyboard / Triple Nested / Slider focus, inner slider at max
    //     pgdn with inner_vert slider focused and at max.
    //     Slider skips pgdn_vert (at_max), claims pgdn_horiz (blocks middle from acting).
    //     Inner scope claims pgdn_horiz (already taken). Middle scope claims pgdn_vert (blocks outer).
    //     Outer never reached. If middle failed to block, outer_vert would scroll down — the slider
    //     is at its limit so pgdn appears to do nothing locally, then outer_vert jumps unexpectedly.
    #[test]
    fn test_triple_nested_keyboard_scrollbar_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // Pre-render inner once to get slider focus_id
        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
            ScrollbarVisibility::None, ScrollbarVisibility::Always,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();

        focus_sys.take_focus(inner_state.vert_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 312.0;  // at max
                middle_state.offset.x = 50.0;  // must NOT scroll
                outer_state.offset.y = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 312.0, "Inner slider already at max");
        assert_eq!(middle_state.offset.x, 50.0, "Middle horiz must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.y, 50.0, "Outer vert must NOT scroll (middle fully blocks)");
    }

    // ── Reversed triple nested: outer HORIZONTAL → middle VERTICAL → inner HORIZONTAL ────
    //
    // Geometry (all tests):
    //   outer_horiz:  bounds=(0,0,400,400)  content=(800,400)  h=Always v=None  [horiz_uses_vert_wheel]
    //                 content_bounds=(0,0,400,388)  horiz-slider at y=388
    //                 max_scroll.x = 400
    //   middle_vert:  bounds=(0,0,400,388)  content=(400,800)  v=Always h=None
    //                 content_bounds=(0,0,388,388)  vert-slider at x=388
    //                 max_scroll.y = 412
    //   inner_horiz:  bounds=(0,0,388,200)  content=(800,200)  h=Always v=None  [horiz_uses_vert_wheel]
    //                 content_bounds=(0,0,388,188)  horiz-slider at y=188
    //                 max_scroll.x = 412
    //
    // Symmetric to the v→h→v case above. middle_vert now unconditionally claims scroll_left/right
    // (change 1), blocking outer_horiz from winning scroll_left. inner_horiz unconditionally claims
    // scroll_up/down (horiz_uses_vert_wheel), blocking middle_vert from firing.
    // Result: middle_vert is a complete bidirectional blocker — outer_horiz never fires.
    //
    // WHY THIS MATTERS: without blocking, a horizontal event on inner_horiz at its limit would
    // skip middle_vert and scroll outer_horiz — an unexpected jump into a container the user has
    // mentally left, disorienting because outer_horiz is the same axis as inner_horiz.

    // 17. Mouse Wheel / Reversed Triple / Inner content, inner at left (at_min)
    //     Upward wheel (remapped to leftward) on inner_horiz content while inner is at_left.
    //     Middle claims scroll_left (blocks outer). Inner claims scroll_up/down (blocks middle action).
    //     Outer never reached.
    #[test]
    fn test_reversed_triple_nested_mouse_content_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // mouse (50,50): inside all three content areas
        // inner.offset.x=0 (at_left): content block skips scroll_left → middle absorbs it
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.x = 0.0;    // at left — skips scroll_left claim
                middle_state.offset.y = 50.0;  // must NOT scroll
                outer_state.offset.x = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(800.0, 400.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0, "Inner horiz already at left");
        assert_eq!(middle_state.offset.y, 50.0, "Middle vert must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll (middle fully blocks)");
    }

    // 18. Mouse Wheel / Reversed Triple / Inner slider, inner slider at left (at_min)
    //     Upward wheel on inner_horiz slider track while slider is at_min.
    //     Slider skips scroll_left (conditional), claims scroll_up/down (unconditional).
    //     Middle claims scroll_left (blocks outer). Outer never reached.
    #[test]
    fn test_reversed_triple_nested_mouse_scrollbar_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // inner horiz slider track: x=0..388, y=188..200
        // mouse (50,195): inside outer (0,0,400,388) and middle (0,0,388,388) content areas,
        //                 outside inner content (0,0,388,188), on inner horiz slider track
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 195.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.x = 0.0;    // at_min — slider skips scroll_left claim
                middle_state.offset.y = 50.0;  // must NOT scroll
                outer_state.offset.x = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(800.0, 400.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0, "Inner horiz already at left");
        assert_eq!(middle_state.offset.y, 50.0, "Middle vert must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll (middle fully blocks)");
    }

    // 19. Keyboard / Reversed Triple / Content focus, inner at right (at_max)
    //     pgdn with focus inside inner_horiz at right limit.
    //     Inner scope claims pgdn_vert (blocks middle). Middle scope claims pgdn_horiz (blocks outer).
    //     Outer never reached.
    #[test]
    fn test_reversed_triple_nested_keyboard_content_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.x = 412.0;  // at right (content=800, view=388 → max=412)
                middle_state.offset.y = 50.0;  // must NOT scroll down
                outer_state.offset.x = 50.0;   // must NOT scroll right
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(800.0, 400.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 412.0, "Inner horiz already at right");
        assert_eq!(middle_state.offset.y, 50.0, "Middle vert must NOT scroll down (cross-axis)");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll right (middle fully blocks)");
    }

    // 20. Keyboard / Reversed Triple / Slider focus, inner slider at right (at_max)
    //     pgdn with inner_horiz slider focused and at max.
    //     Slider skips pgdn_horiz (at_max), claims pgdn_vert (blocks middle from acting).
    //     Middle scope claims pgdn_horiz (blocks outer). Outer never reached.
    #[test]
    fn test_reversed_triple_nested_keyboard_scrollbar_middle_blocks() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut middle_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // Pre-render inner once to get horiz slider focus_id
        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
            ScrollbarVisibility::Always, ScrollbarVisibility::None,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();

        focus_sys.take_focus(inner_state.horiz_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.x = 412.0;  // at max
                middle_state.offset.y = 50.0;  // must NOT scroll
                outer_state.offset.x = 50.0;   // must NOT scroll
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 400.0), Vec2::new(800.0, 400.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            middle_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 412.0, "Inner slider already at max");
        assert_eq!(middle_state.offset.y, 50.0, "Middle vert must NOT scroll (cross-axis)");
        assert_eq!(outer_state.offset.x, 50.0, "Outer horiz must NOT scroll (middle fully blocks)");
    }

    // ── Nested 2D: outer[H+V] > inner[H+V] ───────────────────────────────────────────────────
    //
    // Geometry (all tests):
    //   outer_2d: bounds=(0,0,400,300) content=(800,600) h=Always v=Always
    //             scrollbar_w=12 → content_bounds=(0,0,388,288)
    //             max_scroll.x = 412, max_scroll.y = 312
    //   inner_2d: bounds=(0,0,200,150) content=(400,300) h=Always v=Always
    //             scrollbar_w=12 → content_bounds=(0,0,188,138)
    //             max_scroll.x = 212, max_scroll.y = 162
    //
    // Mouse pos (50,50): inside outer content_bounds AND inner content_bounds.
    //
    // Critical property: inner[2D] must NOT unconditionally claim scroll_left/scroll_right
    // from its needs_v block (the `if !needs_h` guard in begin_scroll_area ensures this).
    // Without that guard, inner would always win scroll_left/right, preventing horizontal
    // same-axis bubbling to outer even when inner is at its horizontal limit.

    // 21. Mouse Wheel / Nested 2D / Vertical same-axis bubbles when inner at top
    //     Upward wheel while inner is at top. Inner can't scroll up, outer scrolls up.
    #[test]
    fn test_nested_2d_mouse_vert_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at top — inner skips scroll_up claim
                outer_state.offset.y = 100.0;  // outer has room to scroll up
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner already at top");
        assert!(outer_state.offset.y < 100.0, "Outer should scroll up (same-axis bubble)");
    }

    // 22. Mouse Wheel / Nested 2D / Horizontal same-axis bubbles when inner at left
    //     Upward wheel (mapped to left) while inner is at left limit.
    //     Without the `if !needs_h` guard, inner's needs_v block would unconditionally
    //     claim scroll_left, preventing outer from winning it. This test catches that regression.
    #[test]
    fn test_nested_2d_mouse_horiz_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // inner_2d has both axes (needs_h=true, needs_v=true), so it's NOT a horiz_uses_vert_wheel
        // area. Horizontal scroll_delta.x is needed to drive the horizontal axis.
        // Use scroll_delta.x to drive horizontal scrolling directly.
        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.x = if frame == 1 { 1.0 } else { 0.0 }; // scroll left (positive x = left)
            if frame == 0 {
                inner_state.offset.x = 0.0;    // at left — inner skips scroll_left claim
                outer_state.offset.x = 100.0;  // outer has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0, "Inner already at left");
        assert!(outer_state.offset.x < 100.0, "Outer should scroll left (same-axis bubble). \
            Regression: if !needs_h guard missing from needs_v block in begin_scroll_area.");
    }

    // ── Nested 2D propagation suite ───────────────────────────────────────────────────────────
    //
    // All 6 tests use the same geometry:
    //   outer_2d: bounds=(0,0,400,300) content=(800,600) h=Always v=Always
    //             content_bounds=(0,0,388,288)  max_scroll=(400,300)  [uses bounds.w/h, not content_bounds]
    //   inner_2d: bounds=(0,0,200,150) content=(400,300) h=Always v=Always
    //             content_bounds=(0,0,188,138)  max_scroll=(200,150)
    //             inner vert slider track: x=188..200, y=0..138
    //             inner horiz slider track: x=0..188, y=138..150
    //
    // Expected invariant: wheel/keyboard events always propagate to the outer area when the inner
    // area is at its limit on the relevant axis.

    // 24. Mouse Wheel / Nested 2D / Vertical slider at top → outer scrolls up
    //     Mouse (194,50): in outer content_bounds, outside inner content_bounds, on inner vert slider.
    //     Inner vert slider at_min: skips scroll_up → outer wins it.
    #[test]
    fn test_nested_2d_mouse_vert_slider_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(194.0, 50.0); // on inner vert slider track
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at top — vert slider skips scroll_up
                outer_state.offset.y = 100.0;  // outer has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner at top, should not change");
        assert!(outer_state.offset.y < 100.0, "Outer should scroll up");
    }

    // 25. Mouse Wheel / Nested 2D / Horizontal slider at left → outer scrolls left
    //     Mouse (50,144): in outer content_bounds, outside inner content_bounds, on inner horiz slider.
    //     Inner horiz slider at_min: skips scroll_left → outer wins scroll_left.
    //     Outer 2D remaps delta.y → dx because it won scroll_left but not scroll_up/down.
    #[test]
    fn test_nested_2d_mouse_horiz_slider_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 144.0); // on inner horiz slider track
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 }; // vertical wheel; horiz slider remaps
            if frame == 0 {
                inner_state.offset.x = 0.0;    // at left — horiz slider skips scroll_left
                outer_state.offset.x = 100.0;  // outer has room to scroll left
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0, "Inner horiz at left, should not change");
        assert!(outer_state.offset.x < 100.0, "Outer should scroll left (horizontal bubble).");
    }

    // 26. Mouse Wheel / Nested 2D / Inner content at top → outer scrolls up
    //     Mouse (50,50): inside both content_bounds. Inner at_top skips scroll_up → outer wins.
    //     (Same logic as test 21, included for completeness of the suite.)
    #[test]
    fn test_nested_2d_mouse_content_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0); // inside inner content_bounds
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at top
                outer_state.offset.y = 100.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 0.0, "Inner at top, should not change");
        assert!(outer_state.offset.y < 100.0, "Outer should scroll up");
    }

    // 27. Keyboard / Nested 2D / Vertical slider focused, at bottom → outer scrolls down
    //     Slider at_max: skips pgdn_vert, unconditionally claims pgup/pgdn_horiz.
    //     inner_scope.finish(): at_bottom skips pgdn_vert; horiz already taken.
    //     outer_scope.finish(): pgdn_vert not yet taken → outer wins → outer scrolls down.
    #[test]
    fn test_nested_2d_keyboard_vert_slider_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // Pre-render to get vert slider focus_id.
        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
            ScrollbarVisibility::Always, ScrollbarVisibility::Always,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        focus_sys.take_focus(inner_state.vert_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 150.0;  // at bottom (max_scroll.y = 300-150 = 150)
                outer_state.offset.y = 50.0;   // outer has room
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 150.0, "Inner at bottom, slider should not move");
        assert!(outer_state.offset.y > 50.0, "Outer should scroll down");
    }

    // 28. Keyboard / Nested 2D / Horizontal slider focused, at right → outer scrolls right
    //     Slider at_max: skips pgdn_horiz, unconditionally claims pgup/pgdn_vert.
    //     inner_scope.finish() (2D): at_right skips pgdn_horiz; at_bottom skips pgdn_vert (already taken anyway).
    //     outer_scope.finish() (2D): pgdn_horiz not taken → outer wins → outer scrolls right.
    #[test]
    fn test_nested_2d_keyboard_horiz_slider_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        // Pre-render to get horiz slider focus_id.
        focus_sys.begin_frame();
        let (_, inner_scope, _, _) = begin_scroll_area(
            Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
            ScrollbarVisibility::Always, ScrollbarVisibility::Always,
            &mut inner_state, &input, &mut focus_sys, None, 0.0
        );
        inner_scope.finish(&mut focus_sys);
        focus_sys.end_frame();
        focus_sys.take_focus(inner_state.horiz_slider_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.x = 200.0;  // at right (max_scroll.x = 400-200 = 200)
                outer_state.offset.x = 50.0;   // outer has room to scroll right
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 200.0, "Inner horiz at right, slider should not move");
        assert!(outer_state.offset.x > 50.0, "Outer should scroll right.");
    }

    // 29. Keyboard / Nested 2D / Inner widget focused, inner at bottom → outer scrolls down
    //     No slider claims. inner_scope.finish(): at_bottom skips pgdn_vert, unconditionally wins pgdn_horiz.
    //     outer_scope.finish(): pgdn_vert not yet taken → outer wins → outer scrolls down.
    #[test]
    fn test_nested_2d_keyboard_content_at_extent_bubbles_to_outer() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSys;
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        focus_sys.take_focus(btn_state.focus_id);

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.key_pressed_page_down = frame == 1;
            if frame == 0 {
                inner_state.offset.y = 150.0;  // at bottom (max_scroll.y = 300-150 = 150)
                outer_state.offset.y = 50.0;
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button_raw(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None, disabled: false },
                &input, &mut text_sys, &mut focus_sys
            );
            btn_state = info.state;
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.y, 150.0, "Inner at bottom, should not change");
        assert!(outer_state.offset.y > 50.0, "Outer should scroll down");
    }

    // 23. Mouse Wheel / Nested 2D / Cross-axis isolation (vertical wheel → no horizontal scroll)
    //     Vertical wheel on inner 2D at top. Inner can't scroll up. Outer scrolls UP (vertical).
    //     Outer must NOT scroll horizontally — no cross-axis leakage.
    #[test]
    fn test_nested_2d_mouse_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();

        for frame in 0..3 {
            focus_sys.begin_frame();
            input.mouse_pos = Vec2::new(50.0, 50.0);
            input.scroll_delta.y = if frame == 1 { 1.0 } else { 0.0 };
            if frame == 0 {
                inner_state.offset.y = 0.0;    // at top — bubbles to outer vertically
                outer_state.offset.y = 100.0;
                outer_state.offset.x = 50.0;   // must NOT change
            }
            let (_, outer_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 300.0), Vec2::new(800.0, 600.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut outer_state, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 150.0), Vec2::new(400.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::Always,
                &mut inner_state, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert!(outer_state.offset.y < 100.0, "Outer should scroll up (vertical bubble)");
        assert_eq!(outer_state.offset.x, 50.0, "Outer must NOT scroll horizontally (cross-axis isolation)");
    }
}
