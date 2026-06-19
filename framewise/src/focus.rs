use crate::input::Input;
use crate::types::{ClipRect, Rect};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Specifies which keys a widget makes available for focus traversal.
///
/// A widget passes this to [`FocusSystem::handle_keyboard_traversal`] to indicate which
/// keys it does NOT consume itself and therefore should trigger focus movement.
/// Keys set to `true` will be used for traversal; keys set to `false` are left
/// for the widget to handle (or ignored).
///
/// # Mapping
/// - `up` / `left` → spatial Up / Left (falls back to linear Prev if no target)
/// - `down` / `right` → spatial Down / Right (falls back to linear Next)
/// - `tab` → Tab moves next (linear order), Shift+Tab moves prev
#[derive(Debug, Clone, Copy)]
pub struct FocusTraversalKeys {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub tab: bool,
}

impl FocusTraversalKeys {
    /// All keys trigger traversal. Use for widgets that do not consume any
    /// directional keys themselves (e.g. buttons, labels).
    pub fn all() -> Self {
        Self {
            up: true,
            down: true,
            left: true,
            right: true,
            tab: true,
        }
    }

    /// Only Tab (and Shift+Tab) triggers traversal. Use for widgets that
    /// consume all four arrow keys themselves (e.g. sliders, text edits).
    pub fn tab_only() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            tab: true,
        }
    }

    /// No keys trigger traversal. Use for widgets that handle all relevant
    /// keys internally and never want to hand off focus via keyboard.
    pub fn none() -> Self {
        Self {
            up: false,
            down: false,
            left: false,
            right: false,
            tab: false,
        }
    }
}

/// An inter-frame identifier for interactive widgets.
/// Used primarily by the `FocusSystem` to track keyboard focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FocusId(u64);

impl FocusId {
    /// Generates a new globally unique ID.
    pub fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_ID.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for FocusId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusDirection {
    /// Move to the next widget in registration order (Tab).
    Next,
    /// Move to the previous widget in registration order (Shift+Tab).
    Prev,
    /// Move to the nearest widget spatially above, falling back to Prev.
    Up,
    /// Move to the nearest widget spatially below, falling back to Next.
    Down,
    /// Move to the nearest widget spatially to the left, falling back to Prev.
    Left,
    /// Move to the nearest widget spatially to the right, falling back to Next.
    Right,
}

/// Tracks keyboard focus and Tab-traversal order across frames.
#[derive(Debug, Clone)]
pub struct FocusSystem {
    focused_keyboard_id: Option<FocusId>,
    keyboard_frame_order: Vec<FocusId>,
    keyboard_frame_rects: HashMap<FocusId, Rect>,
    pending_keyboard_shift: Option<FocusDirection>,
    custom_keyboard_order: HashMap<FocusId, FocusId>, // map id -> next id
    /// Winner of the upward-scroll hover claim from the previous frame.
    active_scroll_up_id: Option<FocusId>,
    /// Winner of the downward-scroll hover claim from the previous frame.
    active_scroll_down_id: Option<FocusId>,
    active_scroll_left_id: Option<FocusId>,
    active_scroll_right_id: Option<FocusId>,
    /// Upward-scroll claim being accumulated this frame.
    next_scroll_up_id: Option<FocusId>,
    /// Downward-scroll claim being accumulated this frame.
    next_scroll_down_id: Option<FocusId>,
    next_scroll_left_id: Option<FocusId>,
    next_scroll_right_id: Option<FocusId>,
    /// Winner of the mouse hover claim from the previous frame.
    active_hover_id: Option<FocusId>,
    /// Mouse hover claim being accumulated this frame.
    next_hover_id: Option<FocusId>,

    // Keyboard scroll scopes
    keyboard_scroll_scopes: Vec<FocusId>,
    focused_scroll_path: Vec<FocusId>,

    // Keyboard Page Up / Page Down directional claims
    next_pgup_vert_id: Option<FocusId>,
    next_pgdn_vert_id: Option<FocusId>,
    next_pgup_horiz_id: Option<FocusId>,
    next_pgdn_horiz_id: Option<FocusId>,
    active_pgup_vert_id: Option<FocusId>,
    active_pgdn_vert_id: Option<FocusId>,
    active_pgup_horiz_id: Option<FocusId>,
    active_pgdn_horiz_id: Option<FocusId>,

    #[cfg(debug_assertions)]
    seen_ids: std::collections::HashSet<FocusId>,
}

impl Default for FocusSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl FocusSystem {
    pub fn new() -> Self {
        Self {
            focused_keyboard_id: None,
            keyboard_frame_order: Vec::new(),
            keyboard_frame_rects: HashMap::new(),
            pending_keyboard_shift: None,
            custom_keyboard_order: HashMap::new(),
            active_scroll_up_id: None,
            active_scroll_down_id: None,
            active_scroll_left_id: None,
            active_scroll_right_id: None,
            next_scroll_up_id: None,
            next_scroll_down_id: None,
            next_scroll_left_id: None,
            next_scroll_right_id: None,
            active_hover_id: None,
            next_hover_id: None,
            keyboard_scroll_scopes: Vec::new(),
            focused_scroll_path: Vec::new(),
            next_pgup_vert_id: None,
            next_pgdn_vert_id: None,
            next_pgup_horiz_id: None,
            next_pgdn_horiz_id: None,
            active_pgup_vert_id: None,
            active_pgdn_vert_id: None,
            active_pgup_horiz_id: None,
            active_pgdn_horiz_id: None,
            #[cfg(debug_assertions)]
            seen_ids: std::collections::HashSet::new(),
        }
    }

    /// Creates a new `FocusSystem` with pre-initialized hover and focus state.
    /// Useful for unit testing or rendering isolated mock widget states.
    pub fn new_mocked(
        focused_keyboard_id: Option<FocusId>,
        active_hover_id: Option<FocusId>,
    ) -> Self {
        Self {
            focused_keyboard_id,
            active_hover_id,
            ..Self::new()
        }
    }

    pub fn begin_frame(&mut self) {
        self.keyboard_scroll_scopes.clear();
        self.focused_scroll_path.clear();
        self.next_hover_id = None;

        #[cfg(debug_assertions)]
        self.seen_ids.clear();
    }

    /// Register a widget in the current frame's focus order.
    /// `rect` is the widget's bounding box in window space; it is used for
    /// spatial arrow-key navigation. Returns true if this widget currently has focus.
    pub fn register_keyboard(&mut self, id: FocusId, rect: Rect, clip_rect: ClipRect) -> bool {
        #[cfg(debug_assertions)]
        {
            if !self.seen_ids.insert(id) && !cfg!(test) {
                panic!("FocusId {:?} registered twice in the same frame! This usually means widget state is being reused incorrectly.", id);
            }
        }

        self.keyboard_frame_order.push(id);

        // Only insert into the spatial map if at least one pixel of the widget
        // is inside the clip rect. Fully-clipped widgets are excluded from
        // directional navigation but remain in Tab order (linear).
        let spatially_visible = clip_rect.is_none_or(|c| {
            let i = rect.intersect(&c);
            i.w > 0.0 && i.h > 0.0
        });
        if spatially_visible {
            self.keyboard_frame_rects.insert(id, rect);
        }

        let has_keyboard_focus = self.focused_keyboard_id == Some(id);
        if has_keyboard_focus {
            self.focused_scroll_path = self.keyboard_scroll_scopes.clone();
        }
        has_keyboard_focus
    }

    /// Explicitly take focus (e.g. when a widget is clicked).
    pub fn take_keyboard_focus(&mut self, id: FocusId) {
        self.focused_keyboard_id = Some(id);
    }

    /// Request focus to shift on the next frame.
    pub fn request_keyboard_shift(&mut self, direction: FocusDirection) {
        self.pending_keyboard_shift = Some(direction);
    }

    /// Override the next focus target for a specific widget (linear Next only).
    ///
    /// TODO: consider extending this to support directional overrides, e.g.
    /// `override_direction(from, FocusDirection::Right, to)` for cases where
    /// the spatial algorithm produces wrong results in a specific layout.
    pub fn override_keyboard_next(&mut self, from: FocusId, to: FocusId) {
        self.custom_keyboard_order.insert(from, to);
    }

    /// Returns true if there is an active text input or something similar that
    /// should consume keyboard events natively instead of triggering Tab navigation.
    /// (For future use, returning false for now).
    pub fn has_keyboard_focus(&self) -> bool {
        self.focused_keyboard_id.is_some()
    }

    /// Returns the [`FocusId`] of the currently focused widget, if any.
    pub fn current_keyboard_focus(&self) -> Option<FocusId> {
        self.focused_keyboard_id
    }

    /// Requests a focus shift based on which keys are pressed, filtered by
    /// `keys`. Only acts when `focused` is true (i.e. the calling widget has
    /// focus). Call this once per widget per frame after the widget has handled
    /// its own key input.
    ///
    /// Tab uses linear (registration-order) traversal. Arrow keys use spatial
    /// navigation (nearest widget in that direction), falling back to linear if
    /// no spatial target exists in the pressed direction.
    pub fn handle_keyboard_traversal(
        &mut self,
        focused: bool,
        input: &Input,
        keys: FocusTraversalKeys,
    ) {
        if !focused {
            return;
        }
        // Tab is always linear — check it first and return early.
        if keys.tab && input.key_pressed_tab {
            if input.modifier_shift {
                self.request_keyboard_shift(FocusDirection::Prev);
            } else {
                self.request_keyboard_shift(FocusDirection::Next);
            }
            return;
        }
        // Arrow keys use spatial navigation.
        if keys.up && input.key_pressed_up {
            self.request_keyboard_shift(FocusDirection::Up);
        } else if keys.down && input.key_pressed_down {
            self.request_keyboard_shift(FocusDirection::Down);
        } else if keys.left && input.key_pressed_left {
            self.request_keyboard_shift(FocusDirection::Left);
        } else if keys.right && input.key_pressed_right {
            self.request_keyboard_shift(FocusDirection::Right);
        }
    }

    // ── Hover-scroll claims (first-caller-wins) ──────────────────────────────
    //
    // Scroll claims (up/down/left/right) are made during scope teardown
    // (end_scroll_area and the slider's hover block), which runs INNER-FIRST.
    // So "first caller" means the innermost hovered scrollable area — exactly
    // the scope that should win the wheel event. Outer scopes calling later
    // are silently ignored, which is why a parent's claim doesn't override an
    // inner child's: when the inner claimed (has room to scroll), the slot is
    // already taken. When the inner skips (at its limit), no claim was made
    // yet, so the outer's call goes through — natural bubbling.
    //
    // Same convention as pg* claims below.
    // ── Mouse hover claims (last-caller-wins) ──────────────────────────────────
    pub fn claim_hover(&mut self, id: FocusId) {
        self.next_hover_id = Some(id);
    }

    /// Returns true if this widget won the hover claim in the previous frame.
    pub fn is_hover_active(&self, id: FocusId) -> bool {
        self.active_hover_id == Some(id)
    }

    pub fn claim_scroll_up(&mut self, id: FocusId) {
        if self.next_scroll_up_id.is_none() {
            self.next_scroll_up_id = Some(id);
        }
    }

    pub fn claim_scroll_down(&mut self, id: FocusId) {
        if self.next_scroll_down_id.is_none() {
            self.next_scroll_down_id = Some(id);
        }
    }

    pub fn claim_scroll_left(&mut self, id: FocusId) {
        if self.next_scroll_left_id.is_none() {
            self.next_scroll_left_id = Some(id);
        }
    }

    pub fn claim_scroll_right(&mut self, id: FocusId) {
        if self.next_scroll_right_id.is_none() {
            self.next_scroll_right_id = Some(id);
        }
    }

    /// Returns true if this widget won the upward-scroll claim in the previous frame.
    pub fn is_active_scroll_up(&self, id: FocusId) -> bool {
        self.active_scroll_up_id == Some(id)
    }

    /// Returns true if this widget won the downward-scroll claim in the previous frame.
    pub fn is_active_scroll_down(&self, id: FocusId) -> bool {
        self.active_scroll_down_id == Some(id)
    }

    pub fn is_active_scroll_left(&self, id: FocusId) -> bool {
        self.active_scroll_left_id == Some(id)
    }

    pub fn is_active_scroll_right(&self, id: FocusId) -> bool {
        self.active_scroll_right_id == Some(id)
    }

    /// Push a new keyboard scroll scope (e.g. entering a scroll area).
    pub fn push_keyboard_scroll_scope(&mut self, id: FocusId) {
        self.keyboard_scroll_scopes.push(id);
    }

    /// Pop a keyboard scroll scope, returning the ID.
    pub fn pop_keyboard_scroll_scope(&mut self) -> Option<FocusId> {
        self.keyboard_scroll_scopes.pop()
    }

    /// Returns the active scroll scope path for the focused widget.
    pub fn focused_scroll_path(&self) -> &[FocusId] {
        &self.focused_scroll_path
    }

    // ── Page-key claims (first-caller-wins) ────────────────────────────────────
    //
    // PgUp/PgDn claims are made during scope teardown (ScrollAreaScope::finish
    // and slider's keyboard block), which runs INNER-FIRST. So "first caller"
    // means the innermost scope containing focus — exactly the scope that should
    // get the press. Outer scopes calling later are silently ignored, which is
    // why a parent's unconditional claim doesn't override an inner conditional
    // skip: when the inner skipped (at its limit), no claim was made yet, so
    // the outer's call goes through.
    //
    // Contrast with scroll claims above (last-caller-wins).
    pub fn claim_pgup_vert(&mut self, id: FocusId) {
        if self.next_pgup_vert_id.is_none() {
            self.next_pgup_vert_id = Some(id);
        }
    }

    pub fn claim_pgdn_vert(&mut self, id: FocusId) {
        if self.next_pgdn_vert_id.is_none() {
            self.next_pgdn_vert_id = Some(id);
        }
    }

    pub fn claim_pgup_horiz(&mut self, id: FocusId) {
        if self.next_pgup_horiz_id.is_none() {
            self.next_pgup_horiz_id = Some(id);
        }
    }

    pub fn claim_pgdn_horiz(&mut self, id: FocusId) {
        if self.next_pgdn_horiz_id.is_none() {
            self.next_pgdn_horiz_id = Some(id);
        }
    }

    pub fn is_active_pgup_vert(&self, id: FocusId) -> bool {
        self.active_pgup_vert_id == Some(id)
    }

    pub fn is_active_pgdn_vert(&self, id: FocusId) -> bool {
        self.active_pgdn_vert_id == Some(id)
    }

    pub fn is_active_pgup_horiz(&self, id: FocusId) -> bool {
        self.active_pgup_horiz_id == Some(id)
    }

    pub fn is_active_pgdn_horiz(&self, id: FocusId) -> bool {
        self.active_pgdn_horiz_id == Some(id)
    }

    /// Resolves any pending focus shifts using the order built this frame.
    pub fn end_frame(&mut self) {
        // Transfer hover scroll claims
        self.active_scroll_up_id = self.next_scroll_up_id.take();
        self.active_hover_id = self.next_hover_id.take();
        self.active_scroll_down_id = self.next_scroll_down_id.take();
        self.active_scroll_left_id = self.next_scroll_left_id.take();
        self.active_scroll_right_id = self.next_scroll_right_id.take();
        self.active_pgup_vert_id = self.next_pgup_vert_id.take();
        self.active_pgdn_vert_id = self.next_pgdn_vert_id.take();
        self.active_pgup_horiz_id = self.next_pgup_horiz_id.take();
        self.active_pgdn_horiz_id = self.next_pgdn_horiz_id.take();

        if let Some(direction) = self.pending_keyboard_shift.take() {
            if !self.keyboard_frame_order.is_empty() {
                let new_focus = match self.focused_keyboard_id {
                    Some(current) => resolve_shift(
                        current,
                        direction,
                        &self.keyboard_frame_order,
                        &self.keyboard_frame_rects,
                        &self.custom_keyboard_order,
                    ),
                    None => {
                        // Nothing focused yet — start at the first widget.
                        Some(self.keyboard_frame_order[0])
                    }
                };
                self.focused_keyboard_id = new_focus;
            } else {
                self.focused_keyboard_id = None;
            }
        }

        self.keyboard_frame_order.clear();
        self.keyboard_frame_rects.clear();
    }
}

/// A standardized helper to handle basic click-to-focus and traversal logic for simple widgets.
/// Returns a tuple of `(is_focused, was_clicked)`.
pub fn handle_widget_keyboard_focus(
    focus_id: FocusId,
    rect: Rect,
    clip_rect: ClipRect,
    input: &Input,
    focus_system: &mut FocusSystem,
    keys: FocusTraversalKeys,
    disabled: bool,
) -> (bool, bool) {
    if disabled {
        return (false, false);
    }

    // 1. Register with the central FocusSystem
    let focused = focus_system.register_keyboard(focus_id, rect, clip_rect);

    // 2. Perform clip-safe hover/press hit testing
    let is_visible = clip_rect.is_none_or(|clip| clip.contains(input.mouse_pos));
    let hovered = rect.contains(input.mouse_pos) && is_visible;
    let clicked = hovered && input.mouse_pressed;

    // 3. Take focus on mouse press
    if clicked {
        focus_system.take_keyboard_focus(focus_id);
    }

    // 4. Handle keyboard focus shifts
    focus_system.handle_keyboard_traversal(focused, input, keys);

    (focused, clicked)
}

// ── Focus resolution helpers ──────────────────────────────────────────────────

fn resolve_shift(
    current: FocusId,
    direction: FocusDirection,
    order: &[FocusId],
    rects: &HashMap<FocusId, Rect>,
    custom_keyboard_order: &HashMap<FocusId, FocusId>,
) -> Option<FocusId> {
    match direction {
        FocusDirection::Next | FocusDirection::Prev => {
            resolve_linear(current, direction, order, custom_keyboard_order)
        }
        FocusDirection::Up
        | FocusDirection::Down
        | FocusDirection::Left
        | FocusDirection::Right => {
            // Try spatial first, fall back to linear if no target found.
            find_spatial_target(current, direction, rects, order).or_else(|| {
                let fallback = match direction {
                    FocusDirection::Up | FocusDirection::Left => FocusDirection::Prev,
                    _ => FocusDirection::Next,
                };
                resolve_linear(current, fallback, order, custom_keyboard_order)
            })
        }
    }
}

fn resolve_linear(
    current: FocusId,
    direction: FocusDirection,
    order: &[FocusId],
    custom_keyboard_order: &HashMap<FocusId, FocusId>,
) -> Option<FocusId> {
    let idx = match order.iter().position(|&id| id == current) {
        Some(i) => i,
        None => return Some(order[0]), // focused item not drawn this frame
    };
    match direction {
        FocusDirection::Next => {
            if let Some(&next_id) = custom_keyboard_order.get(&current) {
                Some(next_id)
            } else {
                Some(order[(idx + 1) % order.len()])
            }
        }
        FocusDirection::Prev => {
            let prev_idx = if idx == 0 { order.len() - 1 } else { idx - 1 };
            Some(order[prev_idx])
        }
        _ => None,
    }
}

/// Penalty applied per pixel of lateral gap (perpendicular to nav direction).
/// A value of 3 means: 30 px of lateral gap is as bad as 90 px of axial distance.
const LATERAL_PENALTY: f32 = 3.0;

/// Find the best spatial navigation target in `direction` from `from_id`.
/// Returns `None` if no candidate exists in that direction.
fn find_spatial_target(
    from_id: FocusId,
    direction: FocusDirection,
    rects: &HashMap<FocusId, Rect>,
    order: &[FocusId],
) -> Option<FocusId> {
    let from_rect = rects.get(&from_id)?;
    let from_cx = from_rect.center().x;
    let from_cy = from_rect.center().y;

    let mut best: Option<(FocusId, f32)> = None;

    for &candidate_id in order {
        if candidate_id == from_id {
            continue;
        }
        let Some(cand) = rects.get(&candidate_id) else {
            continue;
        };
        let cand_cx = cand.center().x;
        let cand_cy = cand.center().y;

        // Candidate must be strictly past the source in the navigation direction.
        let in_direction = match direction {
            FocusDirection::Up => cand_cy < from_cy,
            FocusDirection::Down => cand_cy > from_cy,
            FocusDirection::Left => cand_cx < from_cx,
            FocusDirection::Right => cand_cx > from_cx,
            _ => false,
        };
        if !in_direction {
            continue;
        }

        let (axial_dist, lateral_gap) = match direction {
            FocusDirection::Up | FocusDirection::Down => {
                let axial = (cand_cy - from_cy).abs();
                // Lateral gap: how far apart the rects are on the X axis (0 if they overlap).
                let gap = (from_rect.x.max(cand.x) - from_rect.right().min(cand.right())).max(0.0);
                (axial, gap)
            }
            FocusDirection::Left | FocusDirection::Right => {
                let axial = (cand_cx - from_cx).abs();
                let gap =
                    (from_rect.y.max(cand.y) - from_rect.bottom().min(cand.bottom())).max(0.0);
                (axial, gap)
            }
            _ => continue,
        };

        let score = axial_dist + LATERAL_PENALTY * lateral_gap;
        if best.is_none_or(|(_, s)| score < s) {
            best = Some((candidate_id, score));
        }
    }

    best.map(|(id, _)| id)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Rect, Vec2};

    fn r(x: f32, y: f32) -> Rect {
        Rect::new(x, y, 80.0, 30.0)
    }

    // ── Linear nav tests ───────────────────────────────────────────────────────

    #[test]
    fn test_focus_system_basic_flow() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();

        // Frame 1
        sys.begin_frame();
        assert!(!sys.register_keyboard(id1, r(0.0, 0.0), None));
        assert!(!sys.register_keyboard(id2, r(0.0, 50.0), None));
        assert!(!sys.register_keyboard(id3, r(0.0, 100.0), None));

        sys.take_keyboard_focus(id2);
        sys.end_frame();

        // Frame 2
        sys.begin_frame();
        assert!(!sys.register_keyboard(id1, r(0.0, 0.0), None));
        assert!(
            sys.register_keyboard(id2, r(0.0, 50.0), None),
            "id2 should have focus"
        );
        assert!(!sys.register_keyboard(id3, r(0.0, 100.0), None));

        sys.request_keyboard_shift(FocusDirection::Next);
        sys.end_frame();

        // Frame 3
        sys.begin_frame();
        assert!(!sys.register_keyboard(id1, r(0.0, 0.0), None));
        assert!(!sys.register_keyboard(id2, r(0.0, 50.0), None));
        assert!(
            sys.register_keyboard(id3, r(0.0, 100.0), None),
            "id3 should have focus after Next from id2"
        );

        sys.request_keyboard_shift(FocusDirection::Prev);
        sys.end_frame();

        // Frame 4
        sys.begin_frame();
        assert!(!sys.register_keyboard(id1, r(0.0, 0.0), None));
        assert!(
            sys.register_keyboard(id2, r(0.0, 50.0), None),
            "id2 should have focus after Prev from id3"
        );
    }

    #[test]
    fn test_focus_system_custom_override() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();

        sys.override_keyboard_next(id1, id3);
        sys.take_keyboard_focus(id1);

        sys.begin_frame();
        sys.register_keyboard(id1, r(0.0, 0.0), None);
        sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.register_keyboard(id3, r(0.0, 100.0), None);
        sys.request_keyboard_shift(FocusDirection::Next);
        sys.end_frame();

        sys.begin_frame();
        assert!(!sys.register_keyboard(id1, r(0.0, 0.0), None));
        assert!(!sys.register_keyboard(id2, r(0.0, 50.0), None));
        assert!(
            sys.register_keyboard(id3, r(0.0, 100.0), None),
            "Focus should jump to id3 via custom override"
        );
    }

    // ── handle_keyboard_traversal tests ─────────────────────────────────────────────────

    /// Run one key press through a two-widget system and return which widget
    /// has focus in the next frame. id1 is at (0,0), id2 is at (0,50).
    fn two_widget_focus_after_key(
        input: crate::input::Input,
        keys: crate::focus::FocusTraversalKeys,
    ) -> (bool, bool) {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_keyboard_focus(id1);

        sys.begin_frame();
        let focused1 = sys.register_keyboard(id1, r(0.0, 0.0), None);
        sys.handle_keyboard_traversal(focused1, &input, keys);
        sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register_keyboard(id1, r(0.0, 0.0), None);
        let has2 = sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.end_frame();
        (has1, has2)
    }

    #[test]
    fn test_handle_traversal_tab_moves_next() {
        let input = crate::input::Input {
            key_pressed_tab: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1, "id1 should lose focus after Tab");
        assert!(has2, "id2 should gain focus after Tab");
    }

    #[test]
    fn test_handle_traversal_shift_tab_moves_prev() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_keyboard_focus(id2);

        let input = crate::input::Input {
            key_pressed_tab: true,
            modifier_shift: true,
            ..Default::default()
        };

        sys.begin_frame();
        sys.register_keyboard(id1, r(0.0, 0.0), None);
        let focused2 = sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.handle_keyboard_traversal(focused2, &input, FocusTraversalKeys::all());
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register_keyboard(id1, r(0.0, 0.0), None);
        let has2 = sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.end_frame();
        assert!(has1, "id1 should gain focus after Shift+Tab from id2");
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_down_moves_to_widget_below() {
        // id2 is directly below id1 — spatial finds it.
        let input = crate::input::Input {
            key_pressed_down: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(has2, "Down should move focus to widget below");
    }

    #[test]
    fn test_handle_traversal_right_no_spatial_target_falls_back_linear() {
        // id1 and id2 are stacked vertically — neither is to the right of the other.
        // Right falls back to linear Next → id2.
        let input = crate::input::Input {
            key_pressed_right: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(
            has2,
            "Right with no spatial target falls back to linear Next"
        );
    }

    #[test]
    fn test_handle_traversal_up_no_spatial_target_falls_back_linear() {
        // id1 is at top, nothing above — Up falls back to linear Prev → wraps to id2.
        let input = crate::input::Input {
            key_pressed_up: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(
            has2,
            "Up with no spatial target falls back to linear Prev (wraps)"
        );
    }

    #[test]
    fn test_handle_traversal_left_no_spatial_target_falls_back_linear() {
        let input = crate::input::Input {
            key_pressed_left: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(
            has2,
            "Left with no spatial target falls back to linear Prev (wraps)"
        );
    }

    #[test]
    fn test_handle_traversal_tab_only_arrows_dont_navigate() {
        let input = crate::input::Input {
            key_pressed_right: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::tab_only());
        assert!(has1, "Arrow should not move focus with tab_only");
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_tab_only_tab_still_navigates() {
        let input = crate::input::Input {
            key_pressed_tab: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::tab_only());
        assert!(!has1);
        assert!(has2, "Tab should still navigate with tab_only");
    }

    #[test]
    fn test_handle_traversal_none_nothing_navigates() {
        let input = crate::input::Input {
            key_pressed_tab: true,
            key_pressed_right: true,
            ..Default::default()
        };
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::none());
        assert!(
            has1,
            "No keys should navigate with FocusTraversalKeys::none()"
        );
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_not_focused_does_nothing() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_keyboard_focus(id1);

        let input = crate::input::Input {
            key_pressed_right: true,
            ..Default::default()
        };

        sys.begin_frame();
        sys.register_keyboard(id1, r(0.0, 0.0), None);
        let focused2 = sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.handle_keyboard_traversal(focused2, &input, FocusTraversalKeys::all());
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register_keyboard(id1, r(0.0, 0.0), None);
        sys.register_keyboard(id2, r(0.0, 50.0), None);
        sys.end_frame();
        assert!(has1, "Unfocused widget must not trigger traversal");
    }

    #[test]
    fn test_current_focus_returns_focused_id() {
        let mut sys = FocusSystem::new();
        let id = FocusId::new();
        assert_eq!(sys.current_keyboard_focus(), None);
        sys.take_keyboard_focus(id);
        assert_eq!(sys.current_keyboard_focus(), Some(id));
    }

    // ── Spatial navigation tests ───────────────────────────────────────────────

    /// Run one spatial key frame and return which of the given IDs has focus next frame.
    fn spatial_focus_after_key(
        rects: &[(FocusId, Rect)],
        focused_idx: usize,
        key_fn: impl Fn(&mut crate::input::Input),
    ) -> FocusId {
        let mut sys = FocusSystem::new();
        sys.take_keyboard_focus(rects[focused_idx].0);

        // Frame 1: register_keyboard all, fire key on focused widget
        sys.begin_frame();
        let mut input = crate::input::Input::default();
        key_fn(&mut input);
        for (i, &(id, rect)) in rects.iter().enumerate() {
            let focused = sys.register_keyboard(id, rect, None);
            if i == focused_idx {
                sys.handle_keyboard_traversal(focused, &input, FocusTraversalKeys::all());
            }
        }
        sys.end_frame();

        // Frame 2: find which widget now has focus
        sys.begin_frame();
        let mut focus_result = rects[focused_idx].0; // default: unchanged
        for &(id, rect) in rects {
            if sys.register_keyboard(id, rect, None) {
                focus_result = id;
            }
        }
        sys.end_frame();
        focus_result
    }

    #[test]
    fn test_spatial_down_picks_nearest_below() {
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();
        // Three widgets stacked: id1 top, id2 middle, id3 bottom.
        let rects = [
            (id1, r(0.0, 0.0)),
            (id2, r(0.0, 50.0)),
            (id3, r(0.0, 100.0)),
        ];

        // From id1 Down → id2 (nearest), not id3.
        let got = spatial_focus_after_key(&rects, 0, |i| i.key_pressed_down = true);
        assert_eq!(got, id2, "Down from top should pick middle, not bottom");
    }

    #[test]
    fn test_spatial_up_picks_nearest_above() {
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();
        let rects = [
            (id1, r(0.0, 0.0)),
            (id2, r(0.0, 50.0)),
            (id3, r(0.0, 100.0)),
        ];

        // From id3 Up → id2, not id1.
        let got = spatial_focus_after_key(&rects, 2, |i| i.key_pressed_up = true);
        assert_eq!(got, id2, "Up from bottom should pick middle, not top");
    }

    #[test]
    fn test_spatial_right_picks_nearest_right() {
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();
        // Three widgets in a row.
        let rects = [
            (id1, r(0.0, 0.0)),
            (id2, r(100.0, 0.0)),
            (id3, r(200.0, 0.0)),
        ];

        let got = spatial_focus_after_key(&rects, 0, |i| i.key_pressed_right = true);
        assert_eq!(
            got, id2,
            "Right from left should pick middle, not far right"
        );
    }

    #[test]
    fn test_spatial_left_picks_nearest_left() {
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();
        let rects = [
            (id1, r(0.0, 0.0)),
            (id2, r(100.0, 0.0)),
            (id3, r(200.0, 0.0)),
        ];

        let got = spatial_focus_after_key(&rects, 2, |i| i.key_pressed_left = true);
        assert_eq!(got, id2, "Left from right should pick middle, not far left");
    }

    #[test]
    fn test_spatial_2x2_grid_all_directions() {
        let tl = FocusId::new(); // top-left
        let tr = FocusId::new(); // top-right
        let bl = FocusId::new(); // bottom-left
        let br = FocusId::new(); // bottom-right
        let rects = [
            (tl, r(0.0, 0.0)),
            (tr, r(100.0, 0.0)),
            (bl, r(0.0, 50.0)),
            (br, r(100.0, 50.0)),
        ];

        assert_eq!(
            spatial_focus_after_key(&rects, 0, |i| i.key_pressed_right = true),
            tr,
            "tl→Right→tr"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 0, |i| i.key_pressed_down = true),
            bl,
            "tl→Down→bl"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 3, |i| i.key_pressed_left = true),
            bl,
            "br→Left→bl"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 3, |i| i.key_pressed_up = true),
            tr,
            "br→Up→tr"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 1, |i| i.key_pressed_left = true),
            tl,
            "tr→Left→tl"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 1, |i| i.key_pressed_down = true),
            br,
            "tr→Down→br"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 2, |i| i.key_pressed_right = true),
            br,
            "bl→Right→br"
        );
        assert_eq!(
            spatial_focus_after_key(&rects, 2, |i| i.key_pressed_up = true),
            tl,
            "bl→Up→tl"
        );
    }

    #[test]
    fn test_spatial_prefers_aligned_over_closer_misaligned() {
        // id_origin at (0,0,80,30) — center (40,15).
        // id_close:  30 px above but 200 px to the right — center (240, -15). Misaligned.
        // id_far:    60 px above, directly above — center (40, -45). Aligned.
        //
        // Scores (Up, lateral_penalty=3):
        //   id_close: axial=30, lateral_gap = max(0, 80 - 240) → gap = 160, score = 30 + 480 = 510
        //   id_far:   axial=60, lateral_gap = 0 (directly above, same x range), score = 60
        //
        // id_far should win despite being further axially.
        let origin = FocusId::new();
        let id_close = FocusId::new();
        let id_far = FocusId::new();
        let rects = [
            (origin, Rect::new(0.0, 0.0, 80.0, 30.0)),
            (id_close, Rect::new(200.0, -30.0, 80.0, 30.0)),
            (id_far, Rect::new(0.0, -60.0, 80.0, 30.0)),
        ];

        let got = spatial_focus_after_key(&rects, 0, |i| i.key_pressed_up = true);
        assert_eq!(
            got, id_far,
            "Aligned but further widget should beat misaligned closer one"
        );
    }

    #[test]
    fn test_spatial_no_target_falls_back_linear() {
        // Two widgets side by side — neither is above/below the other.
        // Up from id1 → no spatial target → linear Prev → wraps to id2.
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let rects = [(id1, r(0.0, 0.0)), (id2, r(100.0, 0.0))];

        let got = spatial_focus_after_key(&rects, 0, |i| i.key_pressed_up = true);
        assert_eq!(
            got, id2,
            "No spatial target: Up falls back to linear Prev (wraps to id2)"
        );
    }

    #[test]
    fn test_spatial_single_widget_stays_focused() {
        let id = FocusId::new();
        let mut sys = FocusSystem::new();
        sys.take_keyboard_focus(id);

        sys.begin_frame();
        let input = crate::input::Input {
            key_pressed_down: true,
            ..Default::default()
        };
        let focused = sys.register_keyboard(id, r(0.0, 0.0), None);
        sys.handle_keyboard_traversal(focused, &input, FocusTraversalKeys::all());
        sys.end_frame();

        sys.begin_frame();
        let still_focused = sys.register_keyboard(id, r(0.0, 0.0), None);
        sys.end_frame();
        assert!(
            still_focused,
            "Single widget should remain focused when no nav target exists"
        );
    }

    #[test]
    fn test_fully_clipped_widget_excluded_from_spatial_nav() {
        // id_clipped is registered with a clip_rect that has zero intersection
        // with its rect → excluded from directional nav.
        // id_visible is below id_focus; id_clipped is between them but clipped.
        // Down from id_focus must skip id_clipped and pick id_visible.
        let id_focus = FocusId::new();
        let id_clipped = FocusId::new();
        let id_visible = FocusId::new();
        let clip = Rect::new(0.0, 0.0, 80.0, 40.0); // clip only covers y=0..40

        let mut sys = FocusSystem::new();
        sys.take_keyboard_focus(id_focus);

        sys.begin_frame();
        let input = crate::input::Input {
            key_pressed_down: true,
            ..Default::default()
        };
        let focused = sys.register_keyboard(id_focus, r(0.0, 0.0), None); // y=0, inside clip
        sys.handle_keyboard_traversal(focused, &input, FocusTraversalKeys::all());
        // id_clipped: rect at y=50 is fully outside clip y=0..40 → excluded from spatial map
        sys.register_keyboard(id_clipped, r(0.0, 50.0), Some(clip));
        sys.register_keyboard(id_visible, r(0.0, 100.0), None);
        sys.end_frame();

        sys.begin_frame();
        sys.register_keyboard(id_focus, r(0.0, 0.0), None);
        sys.register_keyboard(id_clipped, r(0.0, 50.0), Some(clip));
        let got_visible = sys.register_keyboard(id_visible, r(0.0, 100.0), None);
        sys.end_frame();

        assert!(
            got_visible,
            "Fully clipped widget excluded: Down must pick id_visible"
        );
    }

    #[test]
    fn test_partially_clipped_widget_included_in_spatial_nav() {
        // id_partial overlaps the clip rect by one pixel row → still navigable.
        let id_focus = FocusId::new();
        let id_partial = FocusId::new();
        let clip = Rect::new(0.0, 0.0, 80.0, 55.0); // clip covers y=0..55

        // id_partial rect is y=50..80 — 5px overlap with clip → included.
        let rect_partial = Rect::new(0.0, 50.0, 80.0, 30.0);

        let mut sys = FocusSystem::new();
        sys.take_keyboard_focus(id_focus);

        sys.begin_frame();
        let input = crate::input::Input {
            key_pressed_down: true,
            ..Default::default()
        };
        let focused = sys.register_keyboard(id_focus, r(0.0, 0.0), None);
        sys.handle_keyboard_traversal(focused, &input, FocusTraversalKeys::all());
        sys.register_keyboard(id_partial, rect_partial, Some(clip));
        sys.end_frame();

        sys.begin_frame();
        sys.register_keyboard(id_focus, r(0.0, 0.0), None);
        let got_partial = sys.register_keyboard(id_partial, rect_partial, Some(clip));
        sys.end_frame();

        assert!(
            got_partial,
            "Partially clipped widget must remain reachable via directional nav"
        );
    }

    #[test]
    fn test_tab_is_linear_regardless_of_spatial_layout() {
        // Widgets arranged horizontally — Tab should still go in registration
        // order (left to right as registered), not by position.
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();
        // Register in order id1, id2, id3 but spatially id3 is leftmost.
        let rects = [
            (id1, r(200.0, 0.0)),
            (id2, r(100.0, 0.0)),
            (id3, r(0.0, 0.0)),
        ];

        // Tab from id1 should go to id2 (next in registration order), not id3 (spatially left).
        let got = spatial_focus_after_key(&rects, 0, |i| i.key_pressed_tab = true);
        assert_eq!(
            got, id2,
            "Tab follows registration order, not spatial position"
        );
    }

    #[test]
    fn test_handle_widget_focus_disabled() {
        let id = FocusId::new();
        let mut sys = FocusSystem::new();
        let input = crate::input::Input {
            mouse_pos: Vec2::new(10.0, 10.0),
            mouse_pressed: true,
            ..Default::default()
        };

        sys.begin_frame();
        let (focused, clicked) = handle_widget_keyboard_focus(
            id,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            None,
            &input,
            &mut sys,
            FocusTraversalKeys::all(),
            true, // disabled
        );
        sys.end_frame();

        assert!(!focused);
        assert!(!clicked);
        assert_eq!(sys.current_keyboard_focus(), None);
    }

    #[test]
    fn test_handle_widget_focus_clicked_takes_focus() {
        let id = FocusId::new();
        let mut sys = FocusSystem::new();
        let mut input = crate::input::Input {
            mouse_pos: Vec2::new(10.0, 10.0),
            ..Default::default()
        };

        // Hovered but not pressed
        sys.begin_frame();
        let (focused1, clicked1) = handle_widget_keyboard_focus(
            id,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            None,
            &input,
            &mut sys,
            FocusTraversalKeys::all(),
            false,
        );
        sys.end_frame();
        assert!(!focused1);
        assert!(!clicked1);
        assert_eq!(sys.current_keyboard_focus(), None);

        // Hovered and pressed
        input.mouse_pressed = true;
        sys.begin_frame();
        let (focused2, clicked2) = handle_widget_keyboard_focus(
            id,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            None,
            &input,
            &mut sys,
            FocusTraversalKeys::all(),
            false,
        );
        sys.end_frame();
        assert!(!focused2); // not registered focused in the frame it *takes* focus
        assert!(clicked2);
        assert_eq!(sys.current_keyboard_focus(), Some(id));
    }

    #[test]
    fn test_handle_widget_focus_handles_traversal() {
        let id = FocusId::new();
        let mut sys = FocusSystem::new();
        sys.take_keyboard_focus(id);

        let input = crate::input::Input {
            key_pressed_tab: true,
            ..Default::default()
        };

        sys.begin_frame();
        let (focused, clicked) = handle_widget_keyboard_focus(
            id,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            None,
            &input,
            &mut sys,
            FocusTraversalKeys::all(),
            false,
        );

        assert!(focused);
        assert!(!clicked);
        assert_eq!(sys.pending_keyboard_shift, Some(FocusDirection::Next));

        sys.end_frame();
    }
}
