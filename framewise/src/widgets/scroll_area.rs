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

pub fn scroll_area<L: Layout>(
    bounds: Rect,
    content_height: f32,
    state: &mut ScrollState,
    inner_layout: L,
    input: &Input,
    focus_sys: &mut crate::focus::FocusSystem,
    clip_rect: Option<Rect>,
    time: f64,
) -> (Vec<DrawCmd>, Rect, OffsetLayout<L>) {
    let mut cmds = Vec::new();

    let is_visible = clip_rect.map_or(true, |clip| clip.contains(input.mouse_pos));

    // 1. Process mouse wheel (if hovered inside bounds AND visible)
    if bounds.contains(input.mouse_pos) && is_visible {
        focus_sys.register_scroll_hover(state.id);
        
        if focus_sys.is_active_scroll(state.id) && input.scroll_delta.y != 0.0 {
            state.offset_y -= input.scroll_delta.y * 30.0;
        }
    }

    // 2. Clamp offset_y
    let max_scroll = (content_height - bounds.h).max(0.0);
    state.offset_y = state.offset_y.max(0.0).min(max_scroll);

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
            thumb_size_ratio: Some(view_ratio),
            style: crate::widgets::slider::SliderStyle::default(),
            clip_rect,
        };
        
        let slider_cmds = crate::widgets::slider::slider(
            &mut state.slider_state,
            &mut state.offset_y,
            slider_spec,
            input,
            time,
            focus_sys,
        );
        cmds.extend(slider_cmds);
    }

    // 7. Push clip rect for the content
    cmds.push(DrawCmd::PushClip { rect: content_bounds });

    let offset_layout = OffsetLayout {
        offset_y: state.offset_y,
        inner: inner_layout,
    };

    (cmds, content_bounds, offset_layout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::ManualLayout;

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

        // Should have FillRect, FillRect, PushClip
        assert_eq!(cmds.len(), 3);
        match cmds.last().unwrap() {
            DrawCmd::PushClip { rect } => assert_eq!(*rect, content_bounds),
            _ => panic!("Last command should be PushClip"),
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
}
