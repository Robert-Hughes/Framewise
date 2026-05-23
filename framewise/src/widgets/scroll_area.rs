use crate::{
    draw::DrawCmd,
    input::Input,
    layout::OffsetLayout,
    types::{Color, Rect, Vec2},
    layout::Layout,
};

#[derive(Debug, Clone)]
pub struct ScrollState {
    pub id: crate::focus::FocusId,
    pub offset_y: f32,
    pub slider_state: crate::widgets::slider::SliderState,
}

impl Default for ScrollState {
    fn default() -> Self {
        Self {
            id: crate::focus::FocusId::new(),
            offset_y: 0.0,
            slider_state: crate::widgets::slider::SliderState::default(),
        }
    }
}

pub struct ScrollAreaScope {
    pub id: crate::focus::FocusId,
    pub content_bounds: Rect,
    at_top: bool,
    at_bottom: bool,
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
            if !self.at_top {
                focus_sys.claim_pgup(self.id);
            }
            if !self.at_bottom {
                focus_sys.claim_pgdn(self.id);
            }
        }

        post_cmds
    }
}

pub fn begin_scroll_area<L: Layout>(
    bounds: Rect,
    content_height: f32,
    state: &mut ScrollState,
    inner_layout: L,
    input: &Input,
    focus_sys: &mut crate::focus::FocusSystem,
    clip_rect: Option<Rect>,
    time: f64,
) -> (Vec<DrawCmd>, ScrollAreaScope, Rect, OffsetLayout<L>) {
    let mut pre_cmds = Vec::new();

    focus_sys.push_keyboard_scroll_scope(state.id);

    let is_visible = clip_rect.map_or(true, |clip| clip.contains(input.mouse_pos));

    // 1. Process mouse wheel (if hovered inside bounds AND visible)
    // We check position first, then claim the scroll directions we can handle.
    // Only claiming directions we have room to move lets an outer scroll area
    // take over when we've hit our limit (nested scrolling).
    let max_scroll = (content_height - bounds.h).max(0.0);
    if bounds.contains(input.mouse_pos) && is_visible {
        let at_top    = state.offset_y <= 0.0;
        let at_bottom = state.offset_y >= max_scroll;

        if !at_top    { focus_sys.claim_scroll_up(state.id); }
        if !at_bottom { focus_sys.claim_scroll_down(state.id); }

        if input.scroll_delta.y > 0.0 && focus_sys.is_active_scroll_up(state.id) {
            state.offset_y -= input.scroll_delta.y * 30.0;
        }
        if input.scroll_delta.y < 0.0 && focus_sys.is_active_scroll_down(state.id) {
            state.offset_y -= input.scroll_delta.y * 30.0;
        }
    }

    let at_top_before = state.offset_y <= 0.0;
    let at_bottom_before = state.offset_y >= max_scroll;

    // Process page up / down (if active from previous frame's claim)
    if input.key_pressed_page_up && focus_sys.is_active_pgup(state.id) {
        state.offset_y -= bounds.h;
    }
    if input.key_pressed_page_down && focus_sys.is_active_pgdn(state.id) {
        state.offset_y += bounds.h;
    }

    // 2. Clamp offset_y
    // (max_scroll already computed above)
    state.offset_y = state.offset_y.clamp(0.0, max_scroll);

    // 3. Draw background (optional, skip for now or draw a subtle frame)
    // We'll leave it transparent.

    // 4. Calculate content bounds and scrollbar track
    let scrollbar_w = 12.0;
    let content_bounds = Rect::new(bounds.x, bounds.y, (bounds.w - scrollbar_w).max(0.0), bounds.h);
    let track_rect = Rect::new(content_bounds.right(), bounds.y, scrollbar_w, bounds.h);

    // 5. Calculate scrollbar thumb and draw slider
    if content_height > bounds.h {
        let view_ratio = (bounds.h / content_height).min(1.0);
        
        let slider_spec = crate::widgets::slider::SliderSpec {
            rect: track_rect,
            min: 0.0,
            max: max_scroll,
            page_step: bounds.h,
            step: 40.0,
            thumb_size_ratio: Some(view_ratio),
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect,
            // This internal scrollbar defers to the directional claim system
            // so scroll propagates to an outer area when we hit the end.
            claim_scroll_at_ends: false,
        };
        
        let slider_cmds = crate::widgets::slider::slider(
            &mut state.slider_state,
            &mut state.offset_y,
            slider_spec,
            input,
            time,
            focus_sys,
        );
        pre_cmds.extend(slider_cmds);
    }

    // 7. Push clip rect for the content
    pre_cmds.push(DrawCmd::PushClip { rect: content_bounds });

    let offset_layout = OffsetLayout {
        offset_y: state.offset_y,
        inner: inner_layout,
    };

    let scope = ScrollAreaScope {
        id: state.id,
        content_bounds,
        at_top: state.offset_y <= 0.0,
        at_bottom: state.offset_y >= max_scroll,
        is_finished: false,
    };

    (pre_cmds, scope, content_bounds, offset_layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ManualLayout;

    // Helper to keep test calls the same
    fn scroll_area<L: crate::layout::Layout>(
        bounds: Rect,
        content_height: f32,
        state: &mut ScrollState,
        inner_layout: L,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        clip_rect: Option<Rect>,
        time: f64,
    ) -> (Vec<DrawCmd>, Rect, crate::layout::OffsetLayout<L>) {
        let (mut pre_cmds, scope, cb, layout) = begin_scroll_area(
            bounds, content_height, state, inner_layout, input, focus_sys, clip_rect, time
        );
        let post_cmds = scope.finish(focus_sys);
        pre_cmds.extend(post_cmds);
        (pre_cmds, cb, layout)
    }

    #[test]
    fn test_scroll_area_math() {
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let mut state = ScrollState { offset_y: 50.0, ..Default::default() };
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, 1.0); // scroll up
        input.mouse_pos = Vec2::new(10.0, 10.0);

        let mut focus_sys = crate::focus::FocusSystem::new();
        // Since we evaluate once, it won't be active yet. Let's register it to be active next frame.
        let (cmds, content_bounds, offset_layout) = scroll_area(bounds, 200.0, &mut state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        focus_sys.end_frame();
        
        // Next frame it will scroll
        let (cmds, content_bounds, offset_layout) = scroll_area(bounds, 200.0, &mut state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        
        // 50.0 - 1.0*30 = 20.0
        assert_eq!(state.offset_y, 20.0);
        assert_eq!(offset_layout.offset_y, 20.0);
        
        // Width should be shrunk by 12.0
        assert_eq!(content_bounds.w, 88.0);

        // Should have FillRect, FillRect, PushClip, PopClip
        assert_eq!(cmds.len(), 4);
        match cmds.last().unwrap() {
            DrawCmd::PopClip => (),
            _ => panic!("Last command should be PopClip"),
        }
        match &cmds[2] {
            DrawCmd::PushClip { rect } => assert_eq!(*rect, content_bounds),
            _ => panic!("Command 2 should be PushClip"),
        }
    }

    #[test]
    fn test_nested_scroll_areas() {
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(50.0, 50.0, 100.0, 100.0); // Inside outer
        
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, -1.0); // Scroll down (delta -1 -> offset +30)
        input.mouse_pos = Vec2::new(75.0, 75.0); // Hovering over INNER scroll area

        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 1: Register hover claims
        scroll_area(outer_bounds, 400.0, &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, 400.0, &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        focus_sys.end_frame(); // Locks inner_state as the active scroll for next frame

        // Frame 2: Process scroll wheel
        scroll_area(outer_bounds, 400.0, &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, 400.0, &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);

        // Expect: Inner scrolled by 30
        assert_eq!(inner_state.offset_y, 30.0);
        
        // Expect: Outer should NOT scroll because inner consumed it
        assert_eq!(outer_state.offset_y, 0.0, "Outer should not scroll when inner is hovered");
    }

    #[test]
    fn test_outer_scroll_area_with_inner_present() {
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(50.0, 50.0, 100.0, 100.0); // Inside outer
        
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState::default();
        
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, -1.0); // Scroll down
        // Hovering over OUTER scroll area, but OUTSIDE the inner scroll area
        input.mouse_pos = Vec2::new(25.0, 25.0); 

        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 1: Register hover claims
        scroll_area(outer_bounds, 400.0, &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, 400.0, &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        focus_sys.end_frame(); // Locks outer_state as active because inner didn't claim it!

        // Frame 2: Process scroll wheel
        scroll_area(outer_bounds, 400.0, &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, 400.0, &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);

        // Expect: Outer scrolled by 30
        assert_eq!(outer_state.offset_y, 30.0);
        
        // Expect: Inner should NOT scroll
        assert_eq!(inner_state.offset_y, 0.0);
    }

    #[test]
    fn test_slider_in_scroll_area() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut scroll_state = ScrollState::default();
        let mut slider_state = crate::widgets::slider::SliderState::default();
        let mut slider_value = 50.0;
        let slider_spec = crate::widgets::slider::SliderSpec {
            rect: Rect::new(10.0, 10.0, 20.0, 100.0), // Inside scroll area
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true, // standalone slider
        };
        
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, 1.0); // positive delta
        input.mouse_pos = Vec2::new(15.0, 15.0); // Inside slider

        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 1: Register hover claims
        focus_sys.begin_frame();
        scroll_area(bounds, 400.0, &mut scroll_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        crate::widgets::slider::slider(&mut slider_state, &mut slider_value, slider_spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame(); // Inner slider wins!

        // Frame 2: Process scroll wheel
        focus_sys.begin_frame();
        scroll_area(bounds, 400.0, &mut scroll_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        crate::widgets::slider::slider(&mut slider_state, &mut slider_value, slider_spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Inner slider should have consumed the scroll
        assert_eq!(slider_value, 45.0);
        
        // Scroll area should NOT have scrolled
        assert_eq!(scroll_state.offset_y, 0.0, "Scroll area should not double-scroll");
    }

    // ── Nested scroll area propagation ─────────────────────────────────────────

    /// Helper: run two frames of outer + inner scroll areas with a given delta.
    fn nested_scroll_two_frames(
        outer_state: &mut ScrollState,
        inner_state: &mut ScrollState,
        outer_content_h: f32,
        inner_content_h: f32,
        outer_bounds: Rect,
        inner_bounds: Rect,
        delta_y: f32,
        mouse_pos: Vec2,
    ) {
        use crate::layout::ManualLayout;
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, delta_y);
        input.mouse_pos = mouse_pos;
        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 1: register claims
        focus_sys.begin_frame();
        scroll_area(outer_bounds, outer_content_h, outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, inner_content_h, inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        focus_sys.end_frame();

        // Frame 2: act on claims
        focus_sys.begin_frame();
        scroll_area(outer_bounds, outer_content_h, outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        scroll_area(inner_bounds, inner_content_h, inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        focus_sys.end_frame();
    }

    #[test]
    fn test_nested_inner_mid_range_scroll_up_no_propagation() {
        // Inner scroll area is in the middle: it claims up, outer gets nothing.
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState { offset_y: 50.0, ..Default::default() }; // mid

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            1.0, // scroll up
            Vec2::new(50.0, 50.0),
        );

        assert!(inner_state.offset_y < 50.0, "inner should have scrolled up");
        assert_eq!(outer_state.offset_y, 0.0, "outer should not scroll when inner has room");
    }

    #[test]
    fn test_nested_inner_mid_range_scroll_down_no_propagation() {
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);
        let mut outer_state = ScrollState::default();
        let mut inner_state = ScrollState { offset_y: 50.0, ..Default::default() };

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            -1.0, // scroll down
            Vec2::new(50.0, 50.0),
        );

        assert!(inner_state.offset_y > 50.0, "inner should have scrolled down");
        assert_eq!(outer_state.offset_y, 0.0, "outer should not scroll when inner has room");
    }

    #[test]
    fn test_nested_inner_at_top_propagates_scroll_up() {
        // Inner is at offset 0 (top). Scroll-up claim stays with outer.
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);
        let mut outer_state = ScrollState { offset_y: 50.0, ..Default::default() };
        let mut inner_state = ScrollState::default(); // offset_y = 0 (at top)

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            1.0, // scroll up
            Vec2::new(50.0, 50.0),
        );

        assert!(outer_state.offset_y < 50.0, "outer should have scrolled up");
        assert_eq!(inner_state.offset_y, 0.0, "inner stays at top");
    }

    #[test]
    fn test_nested_inner_at_bottom_propagates_scroll_down() {
        // Inner is fully scrolled to bottom. Scroll-down claim stays with outer.
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);
        let mut outer_state = ScrollState::default();
        // inner content 400, view 100 -> max_scroll = 300
        let mut inner_state = ScrollState { offset_y: 300.0, ..Default::default() };

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            -1.0, // scroll down
            Vec2::new(50.0, 50.0),
        );

        assert!(outer_state.offset_y > 0.0, "outer should have scrolled down");
        assert_eq!(inner_state.offset_y, 300.0, "inner stays at bottom");
    }

    #[test]
    fn test_nested_inner_at_top_scroll_down_goes_to_inner_not_outer() {
        // Inner is at top but user scrolls DOWN — inner can still handle that.
        let outer_bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let inner_bounds = Rect::new(10.0, 10.0, 150.0, 100.0);
        let mut outer_state = ScrollState { offset_y: 50.0, ..Default::default() };
        let mut inner_state = ScrollState::default(); // at top

        nested_scroll_two_frames(
            &mut outer_state, &mut inner_state,
            600.0, 400.0,
            outer_bounds, inner_bounds,
            -1.0, // scroll down
            Vec2::new(50.0, 50.0),
        );

        assert!(inner_state.offset_y > 0.0, "inner should scroll down");
        assert_eq!(outer_state.offset_y, 50.0, "outer unchanged");
    }

    #[test]
    fn test_standalone_slider_inside_scroll_area_blocks_propagation_at_min() {
        // A standalone slider at its minimum should still block the scroll area
        // from receiving the scroll-up event.
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut scroll_state = ScrollState { offset_y: 50.0, ..Default::default() };
        let mut slider_state = crate::widgets::slider::SliderState::default();
        let mut slider_value = 0.0_f32; // at min
        let slider_spec = crate::widgets::slider::SliderSpec {
            rect: Rect::new(10.0, 10.0, 20.0, 100.0),
            min: 0.0,
            max: 100.0,
            page_step: 20.0,
            step: 5.0,
            thumb_size_ratio: None,
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect: None,
            claim_scroll_at_ends: true, // standalone: always block
        };

        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, 1.0); // scroll up
        input.mouse_pos = Vec2::new(15.0, 50.0); // inside slider

        let mut focus_sys = crate::focus::FocusSystem::new();

        // Frame 1
        focus_sys.begin_frame();
        scroll_area(bounds, 400.0, &mut scroll_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        crate::widgets::slider::slider(&mut slider_state, &mut slider_value, slider_spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Frame 2
        focus_sys.begin_frame();
        scroll_area(bounds, 400.0, &mut scroll_state, ManualLayout, &input, &mut focus_sys, None, 0.0);
        crate::widgets::slider::slider(&mut slider_state, &mut slider_value, slider_spec.clone(), &input, 0.0, &mut focus_sys);
        focus_sys.end_frame();

        // Slider value stays at 0 (clamped), scroll area offset unchanged
        assert_eq!(slider_value, 0.0);
        assert_eq!(scroll_state.offset_y, 50.0, "scroll area must not steal the event");
    }
}
