use crate::{
    draw::DrawCmd,
    input::Input,
    layout::OffsetLayout,
    types::{Rect, Vec2},
};

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

pub struct ScrollAreaScope {
    pub id: crate::focus::FocusId,
    pub content_bounds: Rect,
    at_top: bool,
    at_bottom: bool,
    at_left: bool,
    at_right: bool,
    fallback: bool,
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
            if self.fallback {
                // Horizontal scroll area:
                // We map PgUp/PgDn to horizontal scrolling.
                // We conditionally claim the horizontal scroll actions only if we have room to scroll.
                // This allows horizontal PgUp/PgDn to bubble up to parent horizontal scroll areas if we are at the limit!
                if !self.at_left { focus_sys.claim_pgup_horiz(self.id); }
                if !self.at_right { focus_sys.claim_pgdn_horiz(self.id); }

                // Crucially, we UNCONDITIONALLY claim the vertical PgUp/PgDn actions!
                // This completely isolates this horizontal scope from the vertical axis, preventing 
                // orthogonal keystrokes from "leaking" out and scrolling a parent vertical area.
                focus_sys.claim_pgup_vert(self.id);
                focus_sys.claim_pgdn_vert(self.id);
            } else {
                // Vertical scroll area:
                // We map PgUp/PgDn to vertical scrolling.
                // We conditionally claim the vertical scroll actions only if we have room to scroll.
                // This allows vertical PgUp/PgDn to bubble up to parent vertical scroll areas if we are at the limit!
                if !self.at_top { focus_sys.claim_pgup_vert(self.id); }
                if !self.at_bottom { focus_sys.claim_pgdn_vert(self.id); }

                // Crucially, we UNCONDITIONALLY claim the horizontal PgUp/PgDn actions!
                // This completely isolates this vertical scope from the horizontal axis, preventing 
                // orthogonal keystrokes from "leaking" out and scrolling a parent horizontal area.
                focus_sys.claim_pgup_horiz(self.id);
                focus_sys.claim_pgdn_horiz(self.id);
            }
        }

        post_cmds
    }
}

pub fn begin_scroll_area<L: crate::layout::Layout>(
    bounds: Rect,
    content_size: Vec2,
    h_vis: ScrollbarVisibility,
    v_vis: ScrollbarVisibility,
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

    if content_bounds.contains(input.mouse_pos) && is_visible {
        let at_top    = state.offset.y <= 0.0;
        let at_bottom = state.offset.y >= max_scroll.y;
        let at_left   = state.offset.x <= 0.0;
        let at_right  = state.offset.x >= max_scroll.x;

        let is_degenerate_v = !needs_v || max_scroll.y == 0.0;
        let fallback = needs_h && is_degenerate_v;

        if needs_v {
            if !at_top    { focus_sys.claim_scroll_up(state.id); }
            if !at_bottom { focus_sys.claim_scroll_down(state.id); }
            // Unconditionally claim horizontal scroll directions to block a parent horizontal
            // fallback area from interpreting vertical wheel events as horizontal scroll.
            // Mirrors what the fallback horizontal area does for scroll_up/down.
            focus_sys.claim_scroll_left(state.id);
            focus_sys.claim_scroll_right(state.id);
        }
        if needs_h {
            if !at_left   { focus_sys.claim_scroll_left(state.id); }
            if !at_right  { focus_sys.claim_scroll_right(state.id); }
            if fallback {
                // Crucially, we UNCONDITIONALLY claim the vertical mouse wheel actions!
                // This completely isolates this horizontal scope from the vertical axis, preventing
                // orthogonal wheel events from "leaking" out and scrolling a parent vertical area.
                focus_sys.claim_scroll_up(state.id);
                focus_sys.claim_scroll_down(state.id);
            }
        }

        if needs_v && focus_sys.is_active_scroll_up(state.id) && input.scroll_delta.y > 0.0 {
            state.offset.y -= input.scroll_delta.y * 30.0;
        }
        if needs_v && focus_sys.is_active_scroll_down(state.id) && input.scroll_delta.y < 0.0 {
            state.offset.y -= input.scroll_delta.y * 30.0;
        }

        if needs_h {
            let mut dx = input.scroll_delta.x;
            if fallback && dx == 0.0 { dx = input.scroll_delta.y; }
            // Only fire on scroll_left/right — scroll_up/down are claimed purely to block
            // parent vertical areas and must not double as a horizontal-remap trigger.
            if dx > 0.0 && focus_sys.is_active_scroll_left(state.id) {
                state.offset.x -= dx * 30.0;
            }
            if dx < 0.0 && focus_sys.is_active_scroll_right(state.id) {
                state.offset.x -= dx * 30.0;
            }
        }
    }

    let is_degenerate_v = !needs_v || max_scroll.y == 0.0;
    let fallback = needs_h && is_degenerate_v;

    if input.key_pressed_page_up {
        if fallback && focus_sys.is_active_pgup_horiz(state.id) {
            state.offset.x -= bounds.w;
        } else if !fallback && focus_sys.is_active_pgup_vert(state.id) {
            state.offset.y -= bounds.h;
        }
    }
    if input.key_pressed_page_down {
        if fallback && focus_sys.is_active_pgdn_horiz(state.id) {
            state.offset.x += bounds.w;
        } else if !fallback && focus_sys.is_active_pgdn_vert(state.id) {
            state.offset.y += bounds.h;
        }
    }

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
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect,
            claim_scroll_at_ends: false,
        };
        
        let slider_cmds = crate::widgets::slider::slider(
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
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect,
            claim_scroll_at_ends: false,
        };
        
        let slider_cmds = crate::widgets::slider::slider(
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

    let offset_layout = OffsetLayout {
        offset: state.offset,
        inner: inner_layout,
    };

    let scope = ScrollAreaScope {
        id: state.id,
        content_bounds,
        at_top: state.offset.y <= 0.0,
        at_bottom: state.offset.y >= max_scroll.y,
        at_left: state.offset.x <= 0.0,
        at_right: state.offset.x >= max_scroll.x,
        fallback,
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
        content_size: Vec2,
        state: &mut ScrollState,
        inner_layout: L,
        input: &Input,
        focus_sys: &mut crate::focus::FocusSystem,
        clip_rect: Option<Rect>,
        time: f64,
    ) -> (Vec<DrawCmd>, Rect, crate::layout::OffsetLayout<L>) {
        let (mut pre_cmds, scope, cb, layout) = begin_scroll_area(
            bounds, content_size, 
            ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
            state, inner_layout, input, focus_sys, clip_rect, time
        );
        let post_cmds = scope.finish(focus_sys);
        pre_cmds.extend(post_cmds);
        (pre_cmds, cb, layout)
    }

    #[test]
    fn test_scroll_area_math() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        let input = Input::new();
        let mut focus_sys = crate::focus::FocusSystem::new();

        let (_, content_bounds, layout) = scroll_area(
            bounds, Vec2::new(200.0, 400.0), &mut state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );

            let (_, inner_scope, _, _) = begin_scroll_area(
                inner_bounds, Vec2::new(inner_bounds.w, inner_content_h),
                ScrollbarVisibility::Auto, ScrollbarVisibility::Auto,
                inner_state, ManualLayout, &input, &mut focus_sys, Some(cb), 0.0
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
    fn test_pgup_pgdn_fallback() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let mut state = ScrollState::default();
        
        let mut input = Input::new();
        input.key_pressed_page_down = true;
        let mut focus_sys = crate::focus::FocusSystem::new();
        let mut btn_state = crate::widgets::button::ButtonState::default();
        
        struct DummyTextSystem;
        impl crate::text::TextSystem for DummyTextSystem {
            fn prepare(&mut self, _text: &str, _size: f32) -> crate::text::TextLayout {
                crate::text::TextLayout { handle: crate::text::TextHandle(0), size: crate::types::Vec2::ZERO }
            }
            fn measure_byte_x(&self, _handle: crate::text::TextHandle, _byte_index: usize) -> f32 { 0.0 }
            fn hit_test_x(&self, _handle: crate::text::TextHandle, _x: f32) -> usize { 0 }
        }
        let mut text_sys = DummyTextSystem;
        
        focus_sys.take_focus(btn_state.focus_id);

        for _ in 0..2 {
            focus_sys.begin_frame();
            let (_, scope, _, _) = begin_scroll_area(
                bounds, Vec2::new(400.0, 200.0),
                ScrollbarVisibility::Auto, ScrollbarVisibility::None,
                &mut state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec {
                    rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                    text: "dummy".into(),
                    style: crate::widgets::button::ButtonStyle::default(),
                    clip_rect: None,
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
}




#[cfg(test)]
mod nested_bubbling_tests {
    use crate::widgets::scroll_area::*;
    use crate::types::*;
    use crate::input::Input;
    use crate::layout::*;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0), 
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0), 
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0), 
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 200.0), Vec2::new(800.0, 200.0), 
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            inner_scope.finish(&mut focus_sys);
            outer_scope.finish(&mut focus_sys);
            focus_sys.end_frame();
        }
        assert_eq!(inner_state.offset.x, 0.0);
        assert_eq!(outer_state.offset.y, 100.0, "Should not leak cross-axis");
    }

    // Dummy Text System for Keyboard tests
    struct DummyTextSystem;
    impl crate::text::TextSystem for DummyTextSystem {
        fn prepare(&mut self, _text: &str, _size: f32) -> crate::text::TextLayout {
            crate::text::TextLayout { handle: crate::text::TextHandle(0), size: Vec2::ZERO }
        }
        fn measure_byte_x(&self, _handle: crate::text::TextHandle, _byte_index: usize) -> f32 { 0.0 }
        fn hit_test_x(&self, _handle: crate::text::TextHandle, _x: f32) -> usize { 0 }
    }

    // 5. Keyboard / Inner Content / Same-axis (Bubble)
    #[test]
    fn test_nested_keyboard_content_same_axis_bubbles() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSystem;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0), // max scroll = 100
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None },
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
        let mut text_sys = DummyTextSystem;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(300.0, 200.0), 
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None },
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
            &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0), 
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
            &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(300.0, 200.0), 
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
    // The fallback path maps delta.y → dx and fires whenever is_active_scroll_left
    // is true, so outer scrolls horizontally at the same time as inner scrolls
    // vertically — both fire on every vertical wheel tick.

    // 9. Mouse Wheel / Outer Horiz → Inner Vert / Content area, inner at top (at_min)
    //    Vertical wheel on inner vert content when inner is already at top.
    //    Bug: inner content block skips claim_scroll_up (at_top), so outer retains
    //    active_scroll_up from its fallback claim and fires via fallback dx=delta.y.
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
    //     retains active_scroll_up from its fallback claim and fires via fallback dx=delta.y.
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 400.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
    //     Bug: outer_horiz.finish() claims pgdn via fallback (!at_right) and scrolls right.
    #[test]
    fn test_outer_horiz_inner_vert_keyboard_content_cross_axis_isolates() {
        let mut focus_sys = FocusSystem::new();
        let mut input = Input::new();
        let mut text_sys = DummyTextSystem;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None },
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
    //     Bug: slider doesn't claim, inner scope doesn't claim, outer claims via fallback.
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
            &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 200.0), Vec2::new(200.0, 300.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
    //   middle_horiz: bounds=(0,0,388,300)  content=(800,300)  h=Always v=None  [fallback]
    //                 content_bounds=(0,0,388,288)  horiz-slider at y=288
    //   inner_vert:   bounds=(0,0,200,288)  content=(200,600)  v=Always h=None
    //                 content_bounds=(0,0,188,288)  vert-slider at x=188
    //                 max_scroll.y = 312
    //
    // Middle_horiz is a "fallback" area (h-only). Its unconditional scroll_up/down claims block
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
    //     Middle absorbs scroll_up (fallback claim). Inner claims scroll_left (blocks middle action).
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
        let mut text_sys = DummyTextSystem;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None },
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
            &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 300.0), Vec2::new(800.0, 300.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 200.0, 288.0), Vec2::new(200.0, 600.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
    //   outer_horiz:  bounds=(0,0,400,400)  content=(800,400)  h=Always v=None  [fallback]
    //                 content_bounds=(0,0,400,388)  horiz-slider at y=388
    //                 max_scroll.x = 400
    //   middle_vert:  bounds=(0,0,400,388)  content=(400,800)  v=Always h=None
    //                 content_bounds=(0,0,388,388)  vert-slider at x=388
    //                 max_scroll.y = 412
    //   inner_horiz:  bounds=(0,0,388,200)  content=(800,200)  h=Always v=None  [fallback]
    //                 content_bounds=(0,0,388,188)  horiz-slider at y=188
    //                 max_scroll.x = 412
    //
    // Symmetric to the v→h→v case above. middle_vert now unconditionally claims scroll_left/right
    // (change 1), blocking outer_horiz from winning scroll_left. inner_horiz unconditionally claims
    // scroll_up/down (fallback), blocking middle_vert from firing.
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
        let mut text_sys = DummyTextSystem;
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let info = crate::widgets::button::button(
                std::mem::take(&mut btn_state),
                crate::widgets::button::ButtonSpec { rect: Rect::new(0.0, 0.0, 10.0, 10.0), text: "".into(), style: Default::default(), clip_rect: None },
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
            &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
                &mut outer_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, middle_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 400.0, 388.0), Vec2::new(400.0, 800.0),
                ScrollbarVisibility::None, ScrollbarVisibility::Always,
                &mut middle_state, ManualLayout, &input, &mut focus_sys, None, 0.0
            );
            let (_, inner_scope, _, _) = begin_scroll_area(
                Rect::new(0.0, 0.0, 388.0, 200.0), Vec2::new(800.0, 200.0),
                ScrollbarVisibility::Always, ScrollbarVisibility::None,
                &mut inner_state, ManualLayout, &input, &mut focus_sys, None, 0.0
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
}
