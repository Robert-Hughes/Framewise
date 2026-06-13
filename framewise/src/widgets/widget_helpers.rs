use crate::{
    focus::{FocusId, FocusSystem, FocusTraversalKeys},
    input::Input,
    types::{ClipRect, Color, Rect},
    widget::InputInfo,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PressInteraction {
    pub input: InputInfo,
    pub focused: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct PressInteractionSpec {
    pub focus_id: FocusId,
    pub rect: Rect,
    pub clip_rect: ClipRect,
    pub disabled: bool,
    pub traversal_keys: FocusTraversalKeys,
}

pub fn handle_press_interaction(
    spec: PressInteractionSpec,
    input: &Input,
    focus_system: &mut FocusSystem,
    is_active: &mut bool,
    space_is_active: &mut bool,
) -> PressInteraction {
    if spec.disabled {
        *is_active = false;
        *space_is_active = false;
        return PressInteraction {
            input: InputInfo::default(),
            focused: false,
        };
    }

    let focused = focus_system.register_keyboard(spec.focus_id, spec.rect, spec.clip_rect);

    let is_visible = spec
        .clip_rect
        .is_none_or(|clip| clip.contains(input.mouse_pos));
    let contains = spec.rect.contains(input.mouse_pos) && is_visible;

    if contains {
        focus_system.claim_hover(spec.focus_id);
    }
    let is_hover_active = focus_system.is_hover_active(spec.focus_id);

    if contains && is_hover_active && input.mouse_pressed {
        *is_active = true;
        focus_system.take_keyboard_focus(spec.focus_id);
    }

    let hovered = contains && is_hover_active && (!input.mouse_down || *is_active);
    let mut clicked = *is_active && hovered && input.mouse_clicked;

    if focused && input.key_pressed_enter {
        clicked = true;
    }
    if *space_is_active && input.key_released_space {
        clicked = true;
    }

    if !focused || !input.key_down_space {
        *space_is_active = false;
    }
    if focused && input.key_pressed_space {
        *space_is_active = true;
    }

    if !input.mouse_down {
        *is_active = false;
    }

    let pressed = (*is_active && hovered && input.mouse_down) || *space_is_active;

    focus_system.handle_keyboard_traversal(focused, input, spec.traversal_keys);

    PressInteraction {
        input: InputInfo {
            hovered,
            pressed,
            clicked,
        },
        focused,
    }
}

pub fn interaction_color(
    normal: Color,
    hovered: Color,
    pressed: Color,
    is_hovered: bool,
    is_pressed: bool,
) -> Color {
    if is_pressed {
        pressed
    } else if is_hovered {
        hovered
    } else {
        normal
    }
}
