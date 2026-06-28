use crate::{
    draw::{BorderPlacement, DrawCommands},
    focus::{FocusId, FocusSystem, FocusTraversalKeys},
    input::Input,
    layout::{AxisBound, SizeOffer, SizeRequest},
    text::{layout_text, TextBackend, TextBounds, TextFlow, TextStyle},
    types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2},
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PrefixedControlLayout {
    pub outer_rect: Rect,
    pub prefix_rect: Rect,
    pub child_rect: Rect,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PrefixedControlStyle {
    pub border: Option<Stroke>,
    pub focus: Option<Outline>,
    pub prefix_background: Color,
    pub prefix_active_background: Color,
    pub prefix_text: Color,
    pub prefix_text_style: TextStyle,
    pub prefix_pad_x: f32,
    pub disabled_alpha: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PrefixedControlDrawSpec<'a> {
    pub layout: PrefixedControlLayout,
    pub prefix: &'a str,
    pub style: PrefixedControlStyle,
    pub active: bool,
    pub disabled: bool,
    pub layer: Layer,
}

impl PrefixedControlStyle {
    pub(crate) fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            border: Some(Stroke::new(theme.ink, theme.border)),
            focus: Some(Outline::new(
                theme.rust,
                theme.focus_width,
                -theme.focus_offset_tight,
            )),
            prefix_background: theme.ink,
            prefix_active_background: theme.rust,
            prefix_text: theme.paper,
            prefix_text_style: TextStyle::new(
                theme.mono_font,
                theme.text_mono,
                theme.sans_weight_regular,
                TextFlow::single_line(),
            )
            .with_letter_spacing(0.04),
            prefix_pad_x: 10.0,
            disabled_alpha: 0.35,
        }
    }
}

pub(crate) fn prefixed_control_prefix_width<T: TextBackend>(
    prefix: &str,
    style: PrefixedControlStyle,
    text_backend: &mut T,
) -> f32 {
    let layout = layout_text(
        text_backend,
        prefix,
        style.prefix_text_style,
        TextBounds::UNBOUNDED,
    );
    layout.metrics().logical_size.x + style.prefix_pad_x * 2.0
}

pub(crate) fn prefixed_control_size_request(child: SizeRequest, prefix_width: f32) -> SizeRequest {
    let add_prefix = |size: Vec2| Vec2::new(size.x + prefix_width, size.y);
    SizeRequest {
        min: child.min.map(add_prefix),
        preferred: child.preferred.map(add_prefix),
        max: child.max.map(add_prefix),
    }
}

pub(crate) fn prefixed_control_child_offer(offer: SizeOffer, prefix_width: f32) -> SizeOffer {
    let subtract = |bound| match bound {
        AxisBound::Exact(w) => AxisBound::Exact((w - prefix_width).max(0.0)),
        AxisBound::AtMost(w) => AxisBound::AtMost((w - prefix_width).max(0.0)),
        AxisBound::Unbounded => AxisBound::Unbounded,
    };
    SizeOffer::new(subtract(offer.width), offer.height)
}

pub(crate) fn layout_prefixed_control(
    outer_rect: Rect,
    prefix_width: f32,
) -> PrefixedControlLayout {
    let prefix_w = prefix_width.min(outer_rect.w).max(0.0);
    let prefix_rect = Rect::new(outer_rect.x, outer_rect.y, prefix_w, outer_rect.h);
    let child_rect = Rect::new(
        outer_rect.x + prefix_w,
        outer_rect.y,
        (outer_rect.w - prefix_w).max(0.0),
        outer_rect.h,
    );
    PrefixedControlLayout {
        outer_rect,
        prefix_rect,
        child_rect,
    }
}

pub(crate) fn draw_prefixed_control_prefix_and_chrome<T: TextBackend>(
    spec: PrefixedControlDrawSpec<'_>,
    text_backend: &mut T,
    cmds: &mut DrawCommands,
) {
    let layout = spec.layout;
    let style = spec.style;
    let alpha = if spec.disabled {
        style.disabled_alpha
    } else {
        1.0
    };
    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);
    let prefix_rect = cmds.snap_rect_edges_to_physical_pixel(layout.prefix_rect);
    let prefix_background = if spec.active {
        style.prefix_active_background
    } else {
        style.prefix_background
    };

    cmds.push_crisp_fill_rect(prefix_rect, tint(prefix_background), spec.layer.get_z());

    let prefix_layout = layout_text(
        text_backend,
        spec.prefix,
        style.prefix_text_style,
        TextBounds::UNBOUNDED,
    );
    let metrics = prefix_layout.metrics();
    let x = layout.prefix_rect.x + (layout.prefix_rect.w - metrics.logical_size.x) * 0.5;
    let y = layout.prefix_rect.y + (layout.prefix_rect.h - metrics.logical_size.y) * 0.5;
    prefix_layout.emit_glyphs(
        cmds,
        text_backend,
        Vec2::new(x, y),
        tint(style.prefix_text),
        spec.layer.get_z(),
    );

    if spec.active && !spec.disabled {
        if let Some(outline) = style.focus {
            cmds.push_crisp_border_rect(
                layout.outer_rect.inset(-outline.offset),
                Some(Stroke::new(
                    tint(outline.stroke.color),
                    outline.stroke.width,
                )),
                BorderPlacement::Outside,
                spec.layer.get_focus_z(),
            );
        }
    }
    cmds.push_crisp_border_rect(
        layout.outer_rect,
        style.border.map(|b| Stroke::new(tint(b.color), b.width)),
        BorderPlacement::Inside,
        spec.layer.get_z(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefixed_control_size_request_adds_prefix_width() {
        let child = SizeRequest {
            min: Some(Vec2::new(10.0, 20.0)),
            preferred: Some(Vec2::new(30.0, 40.0)),
            max: Some(Vec2::new(50.0, 60.0)),
        };

        let result = prefixed_control_size_request(child, 7.0);

        assert_eq!(result.min, Some(Vec2::new(17.0, 20.0)));
        assert_eq!(result.preferred, Some(Vec2::new(37.0, 40.0)));
        assert_eq!(result.max, Some(Vec2::new(57.0, 60.0)));
    }

    #[test]
    fn test_prefixed_control_child_offer_subtracts_prefix_width_and_clamps() {
        let exact = prefixed_control_child_offer(
            SizeOffer::new(AxisBound::Exact(12.0), AxisBound::AtMost(20.0)),
            5.0,
        );
        assert_eq!(exact.width, AxisBound::Exact(7.0));
        assert_eq!(exact.height, AxisBound::AtMost(20.0));

        let clamped = prefixed_control_child_offer(
            SizeOffer::new(AxisBound::AtMost(3.0), AxisBound::Exact(9.0)),
            5.0,
        );
        assert_eq!(clamped.width, AxisBound::AtMost(0.0));
        assert_eq!(clamped.height, AxisBound::Exact(9.0));

        let unbounded = prefixed_control_child_offer(
            SizeOffer::new(AxisBound::Unbounded, AxisBound::Unbounded),
            5.0,
        );
        assert_eq!(unbounded.width, AxisBound::Unbounded);
        assert_eq!(unbounded.height, AxisBound::Unbounded);
    }

    #[test]
    fn test_layout_prefixed_control_splits_outer_prefix_and_child_rects() {
        let outer = Rect::new(10.0, 20.0, 100.0, 30.0);
        let layout = layout_prefixed_control(outer, 25.0);

        assert_eq!(layout.outer_rect, outer);
        assert_eq!(layout.prefix_rect, Rect::new(10.0, 20.0, 25.0, 30.0));
        assert_eq!(layout.child_rect, Rect::new(35.0, 20.0, 75.0, 30.0));

        let clamped = layout_prefixed_control(outer, 125.0);
        assert_eq!(clamped.prefix_rect, Rect::new(10.0, 20.0, 100.0, 30.0));
        assert_eq!(clamped.child_rect, Rect::new(110.0, 20.0, 0.0, 30.0));
    }
}
