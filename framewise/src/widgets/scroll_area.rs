use crate::{
    draw::DrawCmd,
    input::Input,
    layout::OffsetLayout,
    types::{Color, Rect, Vec2},
    layout::Layout,
};

#[derive(Debug, Clone, Default)]
pub struct ScrollState {
    pub offset_y: f32,
}

pub fn scroll_area<L: Layout>(
    bounds: Rect,
    content_height: f32,
    state: &mut ScrollState,
    inner_layout: L,
    input: &Input,
) -> (Vec<DrawCmd>, Rect, OffsetLayout<L>) {
    let mut cmds = Vec::new();

    // 1. Process mouse wheel (if hovered inside bounds)
    if bounds.contains(input.mouse_pos) && input.scroll_delta.y != 0.0 {
        state.offset_y -= input.scroll_delta.y * 30.0;
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

    // 5. Draw scrollbar track
    cmds.push(DrawCmd::FillRect {
        rect: track_rect,
        color: Color::rgb(0.15, 0.15, 0.18),
    });

    // 6. Calculate scrollbar thumb
    if content_height > bounds.h {
        let view_ratio = (bounds.h / content_height).min(1.0);
        let thumb_h = (bounds.h * view_ratio).max(20.0); // min thumb size
        let scroll_ratio = state.offset_y / max_scroll;
        let thumb_y = bounds.y + scroll_ratio * (bounds.h - thumb_h);

        let thumb_rect = Rect::new(track_rect.x + 2.0, thumb_y, scrollbar_w - 4.0, thumb_h);
        
        // Let's do simple dragging for thumb?
        // To keep it simple, we just draw the thumb. Dragging requires persistent UI ID / focus system.
        // We can add simple clicking/dragging later if needed.

        let mut thumb_color = Color::rgb(0.4, 0.4, 0.45);
        if thumb_rect.contains(input.mouse_pos) {
            thumb_color = Color::rgb(0.5, 0.5, 0.55);
        }

        cmds.push(DrawCmd::FillRect {
            rect: thumb_rect,
            color: thumb_color,
        });
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
        let mut state = ScrollState { offset_y: 50.0 };
        let mut input = Input::new();
        input.scroll_delta = Vec2::new(0.0, 1.0); // scroll up
        input.mouse_pos = Vec2::new(10.0, 10.0);

        let (cmds, content_bounds, offset_layout) = scroll_area(bounds, 200.0, &mut state, ManualLayout, &input);
        
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
}
