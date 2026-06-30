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
pub struct RepeatTiming {
    pub initial_delay: f64,
    pub interval: f64,
}

impl RepeatTiming {
    pub const PRESS: Self = Self {
        initial_delay: 0.5,
        interval: 0.05,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RepeatTimer {
    next_time: f64,
}

impl Default for RepeatTimer {
    fn default() -> Self {
        Self { next_time: 0.0 }
    }
}

impl RepeatTimer {
    pub fn start(&mut self, now: f64, timing: RepeatTiming) {
        self.next_time = now + timing.initial_delay;
    }

    pub fn due(&self, now: f64) -> bool {
        now >= self.next_time
    }

    pub fn advance(&mut self, now: f64, timing: RepeatTiming) {
        self.next_time = now + timing.interval;
    }

    pub fn consume_due(&mut self, now: f64, timing: RepeatTiming) -> bool {
        if !self.due(now) {
            return false;
        }

        self.advance(now, timing);
        true
    }
}

pub const DEFAULT_DRAG_THRESHOLD: f32 = 4.0;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PressDragState {
    pub held: bool,
    pub dragging: bool,
    pub press_start_pos: Vec2,
    pub drag_start_pos: Vec2,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PressDragInteraction {
    pub held: bool,
    pub dragging: bool,
    pub drag_started: bool,
    pub released: bool,
    pub press_delta: Vec2,
    pub drag_delta: Vec2,
    pub cursor_icon: Option<crate::output::CursorIcon>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HeldCursorPolicy {
    /// Do not show a special cursor while the pointer is merely held.
    ///
    /// Use this when the held phase does not represent a continuing operation.
    /// A drag cursor may still be shown once the interaction is promoted to a drag.
    None,

    /// Show this cursor for as long as this interaction owns the held press.
    ///
    /// Use this only when the held operation continues even if the pointer leaves
    /// the widget or active part.
    #[allow(dead_code)]
    PersistWhileHeld(crate::output::CursorIcon),

    /// Show this cursor only while the pointer is still inside the active part.
    ///
    /// Use this for paused or cancellable held interactions, such as a stepper
    /// button repeating while the pointer remains over the pressed step button.
    WhileActiveContains(crate::output::CursorIcon),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PressDragInteractionSpec {
    pub enabled: bool,
    pub threshold: f32,
    pub held_cursor_policy: HeldCursorPolicy,
    pub active_contains: bool,
    pub drag_cursor_icon: Option<crate::output::CursorIcon>,
}

pub(crate) fn begin_held_press_drag(state: &mut PressDragState, mouse_pos: Vec2) {
    state.held = true;
    state.dragging = false;
    state.press_start_pos = mouse_pos;
    state.drag_start_pos = mouse_pos;
}

pub(crate) fn begin_immediate_drag(
    state: &mut PressDragState,
    mouse_pos: Vec2,
    drag_cursor_icon: Option<crate::output::CursorIcon>,
) -> PressDragInteraction {
    state.held = false;
    state.dragging = true;
    state.press_start_pos = mouse_pos;
    state.drag_start_pos = mouse_pos;

    PressDragInteraction {
        held: false,
        dragging: true,
        drag_started: true,
        released: false,
        press_delta: Vec2::ZERO,
        drag_delta: Vec2::ZERO,
        cursor_icon: drag_cursor_icon,
    }
}

pub(crate) fn handle_press_drag_interaction(
    state: &mut PressDragState,
    input: &Input,
    spec: PressDragInteractionSpec,
) -> PressDragInteraction {
    let was_active = state.held || state.dragging;
    if !spec.enabled {
        *state = PressDragState::default();
        return PressDragInteraction {
            released: was_active,
            ..Default::default()
        };
    }

    if was_active && !input.mouse_down {
        *state = PressDragState::default();
        return PressDragInteraction {
            released: true,
            ..Default::default()
        };
    }

    let mut interaction = PressDragInteraction {
        held: state.held,
        dragging: state.dragging,
        ..Default::default()
    };

    if was_active {
        interaction.press_delta = input.mouse_pos - state.press_start_pos;
    }

    if state.held {
        let press_delta = input.mouse_pos - state.press_start_pos;
        if press_delta.x.hypot(press_delta.y) > spec.threshold {
            state.held = false;
            state.dragging = true;
            state.drag_start_pos = input.mouse_pos;
            interaction.held = false;
            interaction.dragging = true;
            interaction.drag_started = true;
        }
    }

    if state.dragging {
        interaction.dragging = true;
        interaction.drag_delta = input.mouse_pos - state.drag_start_pos;
    }

    interaction.cursor_icon = if state.dragging {
        spec.drag_cursor_icon
    } else if state.held {
        match spec.held_cursor_policy {
            HeldCursorPolicy::None => None,
            HeldCursorPolicy::PersistWhileHeld(icon) => Some(icon),
            HeldCursorPolicy::WhileActiveContains(icon) if spec.active_contains => Some(icon),
            HeldCursorPolicy::WhileActiveContains(_) => None,
        }
    } else {
        None
    };

    interaction
}

/// Result of hit-testing one pointer-interactive rect or sub-part.
///
/// This is deliberately pointer-only. It does not take keyboard focus, mutate
/// widget state, decide release-clicks, or claim widget hover priority.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HoverInteraction {
    /// The pointer is geometrically inside the rect and clip this frame.
    pub contains: bool,
    /// A mouse press began on this rect/sub-part this frame while its widget owns hover priority.
    pub can_start: bool,
    /// This rect/sub-part was already active before this frame, or became active this frame.
    pub active_now: bool,
    /// Passive hover effects may be shown for this rect/sub-part.
    pub passive_hovered: bool,
    /// Passive cursor hint for this rect/sub-part.
    pub cursor_icon: Option<crate::output::CursorIcon>,
}

/// Computes pointer hover state for one rect or sub-part.
///
/// `was_active` must describe this same logical rect/sub-part, not merely the
/// parent widget. The returned `active_now` includes a press that starts on
/// this frame, so a newly active part can draw its pressed affordance
/// immediately.
pub fn handle_hover_interaction(
    rect: Rect,
    clip_rect: ClipRect,
    disabled: bool,
    hover_active: bool,
    was_active: bool,
    hover_cursor_icon: Option<crate::output::CursorIcon>,
    input: &Input,
) -> HoverInteraction {
    let contains = !disabled
        && rect.contains(input.mouse_pos)
        && clip_rect.is_none_or(|clip| clip.contains(input.mouse_pos));
    let can_start = contains && hover_active && input.mouse_pressed;
    // A press that begins this frame is active immediately while mouse_down is already true.
    let active_now = was_active || can_start;
    let passive_hovered = contains && hover_active && (!input.mouse_down || active_now);
    let cursor_icon = if passive_hovered {
        hover_cursor_icon
    } else {
        None
    };

    HoverInteraction {
        contains,
        can_start,
        active_now,
        passive_hovered,
        cursor_icon,
    }
}

/// Result of registering a widget as a keyboard-focus target.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct KeyboardFocusInteraction {
    /// This widget currently has keyboard focus.
    pub focused: bool,
}

/// Registers this widget for keyboard focus and traversal.
///
/// This helper is keyboard-only. It does not perform pointer hit-testing,
/// claim pointer hover, decide pointer clicks, start pointer interactions, or
/// request cursors.
pub fn handle_keyboard_focus(
    focus_id: FocusId,
    rect: Rect,
    clip_rect: ClipRect,
    disabled: bool,
    traversal_keys: FocusTraversalKeys,
    input: &Input,
    focus_system: &mut FocusSystem,
) -> KeyboardFocusInteraction {
    if disabled {
        return KeyboardFocusInteraction::default();
    }

    let focused = focus_system.register_keyboard(focus_id, rect, clip_rect);
    focus_system.handle_keyboard_traversal(focused, input, traversal_keys);
    KeyboardFocusInteraction { focused }
}

/// Result of a simple single-part pressable interaction.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PressInteraction {
    pub input: InputInfo,
    pub focused: bool,
    pub cursor_icon: Option<crate::output::CursorIcon>,
    /// The underlying hover result used to produce `input`.
    pub hover: HoverInteraction,
}

/// Specification for a simple single-part pressable widget.
#[derive(Debug, Clone, Copy)]
pub struct PressInteractionSpec {
    pub focus_id: FocusId,
    pub rect: Rect,
    pub clip_rect: ClipRect,
    pub disabled: bool,
    pub traversal_keys: FocusTraversalKeys,
    pub hover_cursor_icon: Option<crate::output::CursorIcon>,
}

/// Handles keyboard focus, pointer hover, mouse press/release, and keyboard
/// activation for a simple one-part pressable widget.
///
/// For multi-part widgets, compose [`handle_keyboard_focus`] and
/// [`handle_hover_interaction`] directly for each logical part.
pub fn handle_press_interaction(
    spec: PressInteractionSpec,
    input: &Input,
    focus_system: &mut FocusSystem,
    is_active: &mut bool,
    space_is_active: &mut bool,
) -> PressInteraction {
    let keyboard = handle_keyboard_focus(
        spec.focus_id,
        spec.rect,
        spec.clip_rect,
        spec.disabled,
        spec.traversal_keys,
        input,
        focus_system,
    );

    if spec.disabled {
        *is_active = false;
        *space_is_active = false;
        return PressInteraction {
            input: InputInfo::default(),
            focused: false,
            cursor_icon: None,
            hover: HoverInteraction::default(),
        };
    }

    let contains = spec.rect.contains(input.mouse_pos)
        && spec
            .clip_rect
            .is_none_or(|clip| clip.contains(input.mouse_pos));
    if contains {
        focus_system.claim_hover(spec.focus_id);
    }
    let hover_active = focus_system.is_hover_active(spec.focus_id);
    let hover = handle_hover_interaction(
        spec.rect,
        spec.clip_rect,
        spec.disabled,
        hover_active,
        *is_active,
        spec.hover_cursor_icon,
        input,
    );

    if hover.can_start {
        *is_active = true;
        focus_system.take_keyboard_focus(spec.focus_id);
    }

    let mut clicked = *is_active && hover.passive_hovered && input.mouse_clicked;

    if keyboard.focused && input.key_pressed(crate::input::Key::Enter) {
        clicked = true;
    }
    if *space_is_active && input.key_released(crate::input::Key::Space) {
        clicked = true;
    }

    if !keyboard.focused || !input.key_down(crate::input::Key::Space) {
        *space_is_active = false;
    }
    if keyboard.focused && input.key_pressed(crate::input::Key::Space) {
        *space_is_active = true;
    }

    if !input.mouse_down {
        *is_active = false;
    }

    let enter_is_active = keyboard.focused && input.key_down(crate::input::Key::Enter);
    let pressed = (*is_active && hover.passive_hovered && input.mouse_down)
        || *space_is_active
        || enter_is_active;

    PressInteraction {
        input: InputInfo {
            hovered: hover.passive_hovered,
            pressed,
            clicked,
        },
        focused: keyboard.focused,
        cursor_icon: hover.cursor_icon,
        hover,
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
    use crate::input::Input;

    fn rect() -> Rect {
        Rect::new(0.0, 0.0, 20.0, 20.0)
    }

    fn inside_input() -> Input {
        Input {
            mouse_pos: Vec2::new(10.0, 10.0),
            ..Default::default()
        }
    }

    fn press_spec(id: FocusId) -> PressInteractionSpec {
        PressInteractionSpec {
            focus_id: id,
            rect: rect(),
            clip_rect: None,
            disabled: false,
            traversal_keys: FocusTraversalKeys::all(),
            hover_cursor_icon: None,
        }
    }

    fn drag_spec(drag_cursor_icon: Option<crate::output::CursorIcon>) -> PressDragInteractionSpec {
        PressDragInteractionSpec {
            enabled: true,
            threshold: DEFAULT_DRAG_THRESHOLD,
            held_cursor_policy: HeldCursorPolicy::None,
            active_contains: false,
            drag_cursor_icon,
        }
    }

    #[test]
    fn hover_interaction_reports_contains_inside_rect_and_clip() {
        let input = inside_input();
        let hover = handle_hover_interaction(
            rect(),
            Some(Rect::new(0.0, 0.0, 15.0, 15.0)),
            false,
            true,
            false,
            None,
            &input,
        );

        assert!(hover.contains);
    }

    #[test]
    fn hover_interaction_suppresses_contains_when_disabled_outside_or_clipped() {
        let input = inside_input();

        assert!(!handle_hover_interaction(rect(), None, true, true, false, None, &input).contains);
        assert!(
            !handle_hover_interaction(
                Rect::new(30.0, 30.0, 20.0, 20.0),
                None,
                false,
                true,
                false,
                None,
                &input,
            )
            .contains
        );
        assert!(
            !handle_hover_interaction(
                rect(),
                Some(Rect::new(30.0, 30.0, 20.0, 20.0)),
                false,
                true,
                false,
                None,
                &input,
            )
            .contains
        );
    }

    #[test]
    fn hover_interaction_can_start_on_pressed_frame_inside_active_hover() {
        let input = Input {
            mouse_pressed: true,
            mouse_down: true,
            ..inside_input()
        };

        let hover = handle_hover_interaction(rect(), None, false, true, false, None, &input);

        assert!(hover.can_start);
        assert!(hover.active_now);
        assert!(hover.passive_hovered);
    }

    #[test]
    fn hover_interaction_active_now_tracks_existing_active_part() {
        let input = Input {
            mouse_down: true,
            ..inside_input()
        };

        let hover = handle_hover_interaction(rect(), None, false, true, true, None, &input);

        assert!(hover.active_now);
        assert!(hover.passive_hovered);
    }

    #[test]
    fn hover_interaction_suppresses_passive_hover_for_unowned_mouse_down() {
        let input = Input {
            mouse_down: true,
            ..inside_input()
        };

        let hover = handle_hover_interaction(rect(), None, false, true, false, None, &input);

        assert!(hover.contains);
        assert!(!hover.passive_hovered);
    }

    #[test]
    fn hover_interaction_requires_hover_priority_for_start_and_passive_hover() {
        let input = Input {
            mouse_pressed: true,
            mouse_down: true,
            ..inside_input()
        };

        let hover = handle_hover_interaction(rect(), None, false, false, false, None, &input);

        assert!(hover.contains);
        assert!(!hover.can_start);
        assert!(!hover.passive_hovered);
    }

    #[test]
    fn hover_interaction_reports_passive_hover_cursor_only_while_passive_hovered() {
        let hover = handle_hover_interaction(
            rect(),
            None,
            false,
            true,
            false,
            Some(crate::output::CursorIcon::Pointer),
            &inside_input(),
        );
        assert_eq!(hover.cursor_icon, Some(crate::output::CursorIcon::Pointer));

        let suppressed = handle_hover_interaction(
            rect(),
            None,
            false,
            true,
            false,
            Some(crate::output::CursorIcon::Pointer),
            &Input {
                mouse_down: true,
                ..inside_input()
            },
        );
        assert_eq!(suppressed.cursor_icon, None);
    }

    #[test]
    fn press_interaction_hovers_with_mouse_up() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let mut is_active = false;
        let mut space_is_active = false;

        let result = handle_press_interaction(
            press_spec(id),
            &inside_input(),
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );

        assert!(result.input.hovered);
        assert!(!result.input.pressed);
        assert!(!result.input.clicked);
    }

    #[test]
    fn press_interaction_cursor_tracks_simple_press_active_target() {
        let id = FocusId::new();
        let mut is_active = false;
        let mut space_is_active = false;
        let spec = PressInteractionSpec {
            hover_cursor_icon: Some(crate::output::CursorIcon::Pointer),
            ..press_spec(id)
        };

        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let hover = handle_press_interaction(
            spec,
            &inside_input(),
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert_eq!(hover.cursor_icon, Some(crate::output::CursorIcon::Pointer));

        is_active = true;
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let outside = handle_press_interaction(
            spec,
            &Input {
                mouse_pos: Vec2::new(30.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert_eq!(outside.cursor_icon, None);

        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let returned = handle_press_interaction(
            spec,
            &Input {
                mouse_down: true,
                ..inside_input()
            },
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert_eq!(
            returned.cursor_icon,
            Some(crate::output::CursorIcon::Pointer)
        );
    }

    #[test]
    fn press_interaction_suppresses_external_mouse_down_hover() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let mut is_active = false;
        let mut space_is_active = false;
        let input = Input {
            mouse_down: true,
            ..inside_input()
        };

        let result = handle_press_interaction(
            press_spec(id),
            &input,
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );

        assert!(!result.input.hovered);
        assert!(!result.input.pressed);
    }

    #[test]
    fn press_interaction_press_start_is_active_and_hovered_this_frame() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let mut is_active = false;
        let mut space_is_active = false;
        let input = Input {
            mouse_pressed: true,
            mouse_down: true,
            ..inside_input()
        };

        let result = handle_press_interaction(
            press_spec(id),
            &input,
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );

        assert!(is_active);
        assert!(result.hover.can_start);
        assert!(result.input.hovered);
        assert!(result.input.pressed);
    }

    #[test]
    fn press_interaction_release_click_remains_correct() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let mut is_active = true;
        let mut space_is_active = false;
        let input = Input {
            mouse_clicked: true,
            ..inside_input()
        };

        let result = handle_press_interaction(
            press_spec(id),
            &input,
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );

        assert!(result.input.clicked);
        assert!(!is_active);
    }

    #[test]
    fn press_interaction_disabled_does_not_hover_press_or_focus() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(None, Some(id));
        let mut is_active = true;
        let mut space_is_active = true;
        let input = Input {
            mouse_pressed: true,
            mouse_down: true,
            keys_pressed: crate::input::KeySet::from_key(crate::input::Key::Enter),
            ..inside_input()
        };

        let result = handle_press_interaction(
            PressInteractionSpec {
                disabled: true,
                ..press_spec(id)
            },
            &input,
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );

        assert_eq!(result.input, InputInfo::default());
        assert!(!result.focused);
        assert!(!is_active);
        assert!(!space_is_active);
    }

    #[test]
    fn press_interaction_keyboard_enter_and_space_activate_when_focused() {
        let id = FocusId::new();
        let mut focus = FocusSystem::new_mocked(Some(id), None);
        let mut is_active = false;
        let mut space_is_active = false;

        let enter = handle_press_interaction(
            press_spec(id),
            &Input {
                keys_pressed: crate::input::KeySet::from_key(crate::input::Key::Enter),
                ..Default::default()
            },
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert!(enter.input.clicked);

        let mut focus = FocusSystem::new_mocked(Some(id), None);
        let space_down = handle_press_interaction(
            press_spec(id),
            &Input {
                keys_pressed: crate::input::KeySet::from_key(crate::input::Key::Space),
                keys_down: crate::input::KeySet::from_key(crate::input::Key::Space),
                ..Default::default()
            },
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert!(space_down.input.pressed);

        let mut focus = FocusSystem::new_mocked(Some(id), None);
        let space_up = handle_press_interaction(
            press_spec(id),
            &Input {
                keys_released: crate::input::KeySet::from_key(crate::input::Key::Space),
                ..Default::default()
            },
            &mut focus,
            &mut is_active,
            &mut space_is_active,
        );
        assert!(space_up.input.clicked);
    }

    #[test]
    fn press_drag_held_press_waits_until_strictly_above_threshold() {
        let mut state = PressDragState::default();
        begin_held_press_drag(&mut state, Vec2::new(10.0, 10.0));

        let below = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(13.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(None),
        );
        assert!(below.held);
        assert!(!below.dragging);
        assert!(!below.drag_started);

        let equal = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(14.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(None),
        );
        assert!(equal.held);
        assert!(!equal.dragging);
        assert!(!equal.drag_started);
    }

    #[test]
    fn press_drag_held_press_promotes_above_threshold() {
        let mut state = PressDragState::default();
        begin_held_press_drag(&mut state, Vec2::new(10.0, 10.0));

        let interaction = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(15.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(None),
        );

        assert!(interaction.drag_started);
        assert!(interaction.dragging);
        assert!(!interaction.held);
        assert!(state.dragging);
        assert!(!state.held);
    }

    #[test]
    fn press_drag_delta_starts_at_transition_point() {
        let mut state = PressDragState::default();
        begin_held_press_drag(&mut state, Vec2::new(10.0, 10.0));

        let promoted = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(15.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(None),
        );
        assert_eq!(promoted.press_delta, Vec2::new(5.0, 0.0));
        assert_eq!(promoted.drag_delta, Vec2::ZERO);

        let dragged = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(18.0, 12.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(None),
        );
        assert_eq!(dragged.press_delta, Vec2::new(8.0, 2.0));
        assert_eq!(dragged.drag_delta, Vec2::new(3.0, 2.0));
    }

    #[test]
    fn press_drag_immediate_drag_returns_current_frame_interaction() {
        let mut state = PressDragState::default();

        let interaction = begin_immediate_drag(
            &mut state,
            Vec2::new(10.0, 10.0),
            Some(crate::output::CursorIcon::EwResize),
        );

        assert!(interaction.dragging);
        assert!(interaction.drag_started);
        assert!(!interaction.held);
        assert!(!interaction.released);
        assert_eq!(interaction.press_delta, Vec2::ZERO);
        assert_eq!(interaction.drag_delta, Vec2::ZERO);
        assert_eq!(
            interaction.cursor_icon,
            Some(crate::output::CursorIcon::EwResize)
        );
    }

    #[test]
    fn press_drag_returns_cursor_while_dragging() {
        let mut state = PressDragState::default();
        begin_immediate_drag(
            &mut state,
            Vec2::new(10.0, 10.0),
            Some(crate::output::CursorIcon::EwResize),
        );

        let interaction = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(12.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            drag_spec(Some(crate::output::CursorIcon::EwResize)),
        );

        assert_eq!(
            interaction.cursor_icon,
            Some(crate::output::CursorIcon::EwResize)
        );
    }

    #[test]
    fn press_drag_held_cursor_policy_controls_held_cursor() {
        let mut state = PressDragState::default();
        begin_held_press_drag(&mut state, Vec2::new(10.0, 10.0));

        let inactive = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(11.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            PressDragInteractionSpec {
                held_cursor_policy: HeldCursorPolicy::WhileActiveContains(
                    crate::output::CursorIcon::Pointer,
                ),
                active_contains: false,
                ..drag_spec(Some(crate::output::CursorIcon::EwResize))
            },
        );
        assert_eq!(inactive.cursor_icon, None);

        let active = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(12.0, 10.0),
                mouse_down: true,
                ..Default::default()
            },
            PressDragInteractionSpec {
                held_cursor_policy: HeldCursorPolicy::WhileActiveContains(
                    crate::output::CursorIcon::Pointer,
                ),
                active_contains: true,
                ..drag_spec(Some(crate::output::CursorIcon::EwResize))
            },
        );
        assert_eq!(active.cursor_icon, Some(crate::output::CursorIcon::Pointer));
    }

    #[test]
    fn press_drag_release_clears_state() {
        let mut state = PressDragState::default();
        begin_immediate_drag(&mut state, Vec2::new(10.0, 10.0), None);

        let interaction = handle_press_drag_interaction(
            &mut state,
            &Input {
                mouse_pos: Vec2::new(12.0, 10.0),
                mouse_down: false,
                ..Default::default()
            },
            drag_spec(None),
        );

        assert!(interaction.released);
        assert_eq!(state, PressDragState::default());
    }

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
