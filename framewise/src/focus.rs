use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use crate::input::Input;

/// Specifies which keys a widget makes available for focus traversal.
///
/// A widget passes this to [`FocusSystem::handle_traversal`] to indicate which
/// keys it does NOT consume itself and therefore should trigger focus movement.
/// Keys set to `true` will be used for traversal; keys set to `false` are left
/// for the widget to handle (or ignored).
///
/// # Mapping
/// - `up` / `left` → move focus to previous widget (like Shift+Tab)
/// - `down` / `right` → move focus to next widget (like Tab)
/// - `tab` → Tab moves next, Shift+Tab moves prev (reads `Input::modifier_shift`)
#[derive(Debug, Clone, Copy)]
pub struct FocusTraversalKeys {
    pub up:    bool,
    pub down:  bool,
    pub left:  bool,
    pub right: bool,
    pub tab:   bool,
}

impl FocusTraversalKeys {
    /// All keys trigger traversal. Use for widgets that do not consume any
    /// directional keys themselves (e.g. buttons, labels).
    pub fn all() -> Self {
        Self { up: true, down: true, left: true, right: true, tab: true }
    }

    /// Only Tab (and Shift+Tab) triggers traversal. Use for widgets that
    /// consume all four arrow keys themselves (e.g. sliders, text edits).
    pub fn tab_only() -> Self {
        Self { up: false, down: false, left: false, right: false, tab: true }
    }

    /// No keys trigger traversal. Use for widgets that handle all relevant
    /// keys internally and never want to hand off focus via keyboard.
    pub fn none() -> Self {
        Self { up: false, down: false, left: false, right: false, tab: false }
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
    Next,
    Prev,
}

/// Tracks keyboard focus and Tab-traversal order across frames.
#[derive(Debug, Clone)]
pub struct FocusSystem {
    focused_id: Option<FocusId>,
    current_frame_order: Vec<FocusId>,
    pending_shift: Option<FocusDirection>,
    custom_order: HashMap<FocusId, FocusId>, // map id -> next id
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
            focused_id: None,
            current_frame_order: Vec::new(),
            pending_shift: None,
            custom_order: HashMap::new(),
            active_scroll_up_id: None,
            active_scroll_down_id: None,
            active_scroll_left_id: None,
            active_scroll_right_id: None,
            next_scroll_up_id: None,
            next_scroll_down_id: None,
            next_scroll_left_id: None,
            next_scroll_right_id: None,
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

    pub fn begin_frame(&mut self) {
        self.keyboard_scroll_scopes.clear();
        self.focused_scroll_path.clear();
        
        #[cfg(debug_assertions)]
        self.seen_ids.clear();
    }

    /// Register a widget in the current frame's focus order.
    /// Returns true if the widget currently has focus.
    pub fn register(&mut self, id: FocusId) -> bool {
        #[cfg(debug_assertions)]
        {
            if !self.seen_ids.insert(id) && !cfg!(test) {
                panic!("FocusId {:?} registered twice in the same frame! This usually means widget state is being reused incorrectly.", id);
            }
        }
        
        self.current_frame_order.push(id);
        let has_focus = self.focused_id == Some(id);
        if has_focus {
            self.focused_scroll_path = self.keyboard_scroll_scopes.clone();
        }
        has_focus
    }

    /// Explicitly take focus (e.g. when a widget is clicked).
    pub fn take_focus(&mut self, id: FocusId) {
        self.focused_id = Some(id);
    }

    /// Request focus to shift on the next frame.
    pub fn request_shift(&mut self, direction: FocusDirection) {
        self.pending_shift = Some(direction);
    }
    
    /// Override the next focus target for a specific widget.
    pub fn override_next(&mut self, from: FocusId, to: FocusId) {
        self.custom_order.insert(from, to);
    }

    /// Returns true if there is an active text input or something similar that
    /// should consume keyboard events natively instead of triggering Tab navigation.
    /// (For future use, returning false for now).
    pub fn has_focus(&self) -> bool {
        self.focused_id.is_some()
    }

    /// Returns the [`FocusId`] of the currently focused widget, if any.
    pub fn current_focus(&self) -> Option<FocusId> {
        self.focused_id
    }

    /// Requests a focus shift based on which keys are pressed, filtered by
    /// `keys`. Only acts when `focused` is true (i.e. the calling widget has
    /// focus). Call this once per widget per frame after the widget has handled
    /// its own key input.
    ///
    /// Keys that the widget consumes itself should be set to `false` in `keys`
    /// so they are skipped here. See [`FocusTraversalKeys`] for presets.
    pub fn handle_traversal(&mut self, focused: bool, input: &Input, keys: FocusTraversalKeys) {
        if !focused {
            return;
        }
        let want_prev = (keys.up    && input.key_pressed_up)
                     || (keys.left  && input.key_pressed_left)
                     || (keys.tab   && input.key_pressed_tab && input.modifier_shift);
        let want_next = (keys.down  && input.key_pressed_down)
                     || (keys.right && input.key_pressed_right)
                     || (keys.tab   && input.key_pressed_tab && !input.modifier_shift);
        if want_prev {
            self.request_shift(FocusDirection::Prev);
        } else if want_next {
            self.request_shift(FocusDirection::Next);
        }
    }

    // ── Hover-scroll claims (last-caller-wins) ────────────────────────────────
    //
    // Scroll claims (up/down/left/right) are checked during widget evaluation
    // (begin_scroll_area's hover block). The traversal is OUTER-FIRST, so each
    // inner widget can overwrite an outer claim — the deepest hovered widget
    // that can scroll wins. A widget should only call these if it can actually
    // scroll on that axis (not at its limit), so a child at its limit lets the
    // parent retain the claim and bubbling works naturally.
    //
    // Contrast with pg* claims below (first-caller-wins).
    pub fn claim_scroll_up(&mut self, id: FocusId) {
        self.next_scroll_up_id = Some(id);
    }

    pub fn claim_scroll_down(&mut self, id: FocusId) {
        self.next_scroll_down_id = Some(id);
    }

    pub fn claim_scroll_left(&mut self, id: FocusId) {
        self.next_scroll_left_id = Some(id);
    }

    pub fn claim_scroll_right(&mut self, id: FocusId) {
        self.next_scroll_right_id = Some(id);
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
        self.active_scroll_down_id = self.next_scroll_down_id.take();
        self.active_scroll_left_id = self.next_scroll_left_id.take();
        self.active_scroll_right_id = self.next_scroll_right_id.take();
        self.active_pgup_vert_id = self.next_pgup_vert_id.take();
        self.active_pgdn_vert_id = self.next_pgdn_vert_id.take();
        self.active_pgup_horiz_id = self.next_pgup_horiz_id.take();
        self.active_pgdn_horiz_id = self.next_pgdn_horiz_id.take();

        if let Some(direction) = self.pending_shift.take() {
            if !self.current_frame_order.is_empty() {
                let new_focus = match self.focused_id {
                    Some(current) => {
                        // Find current in the order
                        if let Some(idx) = self.current_frame_order.iter().position(|&id| id == current) {
                            match direction {
                                FocusDirection::Next => {
                                    if let Some(&next_id) = self.custom_order.get(&current) {
                                        Some(next_id)
                                    } else {
                                        let next_idx = (idx + 1) % self.current_frame_order.len();
                                        Some(self.current_frame_order[next_idx])
                                    }
                                }
                                FocusDirection::Prev => {
                                    let prev_idx = if idx == 0 {
                                        self.current_frame_order.len() - 1
                                    } else {
                                        idx - 1
                                    };
                                    Some(self.current_frame_order[prev_idx])
                                }
                            }
                        } else {
                            // Current focused item wasn't drawn this frame. Pick first.
                            Some(self.current_frame_order[0])
                        }
                    }
                    None => {
                        // Nothing was focused, shift focuses the first item.
                        Some(self.current_frame_order[0])
                    }
                };
                self.focused_id = new_focus;
            } else {
                self.focused_id = None;
            }
        }

        self.current_frame_order.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_focus_system_basic_flow() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();

        // Frame 1
        sys.begin_frame();
        assert!(!sys.register(id1));
        assert!(!sys.register(id2));
        assert!(!sys.register(id3));
        
        // Take focus explicitly
        sys.take_focus(id2);
        sys.end_frame();

        // Frame 2
        sys.begin_frame();
        assert!(!sys.register(id1));
        assert!(sys.register(id2), "id2 should have focus");
        assert!(!sys.register(id3));

        // Request shift Next
        sys.request_shift(FocusDirection::Next);
        sys.end_frame();

        // Frame 3
        sys.begin_frame();
        assert!(!sys.register(id1));
        assert!(!sys.register(id2));
        assert!(sys.register(id3), "id3 should have focus after shifting next from id2");
        
        // Request shift Prev
        sys.request_shift(FocusDirection::Prev);
        sys.end_frame();

        // Frame 4
        sys.begin_frame();
        assert!(!sys.register(id1));
        assert!(sys.register(id2), "id2 should have focus after shifting prev from id3");
    }

    // ── handle_traversal tests ─────────────────────────────────────────────────

    fn two_widget_focus_after_key(input: crate::input::Input, keys: crate::focus::FocusTraversalKeys) -> (bool, bool) {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_focus(id1);

        sys.begin_frame();
        let focused1 = sys.register(id1);
        sys.handle_traversal(focused1, &input, keys);
        sys.register(id2);
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register(id1);
        let has2 = sys.register(id2);
        sys.end_frame();
        (has1, has2)
    }

    #[test]
    fn test_handle_traversal_tab_moves_next() {
        let mut input = crate::input::Input::default();
        input.key_pressed_tab = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1, "id1 should lose focus after Tab");
        assert!(has2, "id2 should gain focus after Tab");
    }

    #[test]
    fn test_handle_traversal_shift_tab_moves_prev() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_focus(id2);

        let mut input = crate::input::Input::default();
        input.key_pressed_tab = true;
        input.modifier_shift = true;

        sys.begin_frame();
        sys.register(id1);
        let focused2 = sys.register(id2);
        sys.handle_traversal(focused2, &input, FocusTraversalKeys::all());
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register(id1);
        let has2 = sys.register(id2);
        sys.end_frame();
        assert!(has1, "id1 should gain focus after Shift+Tab from id2");
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_right_moves_next() {
        let mut input = crate::input::Input::default();
        input.key_pressed_right = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(has2, "id2 should gain focus after Right arrow");
    }

    #[test]
    fn test_handle_traversal_down_moves_next() {
        let mut input = crate::input::Input::default();
        input.key_pressed_down = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(has2, "id2 should gain focus after Down arrow");
    }

    #[test]
    fn test_handle_traversal_left_moves_prev() {
        let mut input = crate::input::Input::default();
        input.key_pressed_left = true;
        // id1 is last; Prev wraps to id2 (last in order from id1's perspective)
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        // From id1, Prev wraps to id2 (the only other widget, which is "before" in
        // circular order: id1 is index 0, prev wraps to index 1 = id2).
        assert!(!has1);
        assert!(has2, "Left arrow from first widget wraps to last");
    }

    #[test]
    fn test_handle_traversal_up_moves_prev() {
        let mut input = crate::input::Input::default();
        input.key_pressed_up = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::all());
        assert!(!has1);
        assert!(has2, "Up arrow from first widget wraps to last");
    }

    #[test]
    fn test_handle_traversal_tab_only_arrows_dont_navigate() {
        let mut input = crate::input::Input::default();
        input.key_pressed_right = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::tab_only());
        assert!(has1, "Arrow should not move focus with tab_only");
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_tab_only_tab_still_navigates() {
        let mut input = crate::input::Input::default();
        input.key_pressed_tab = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::tab_only());
        assert!(!has1);
        assert!(has2, "Tab should still navigate with tab_only");
    }

    #[test]
    fn test_handle_traversal_none_nothing_navigates() {
        let mut input = crate::input::Input::default();
        input.key_pressed_tab = true;
        input.key_pressed_right = true;
        let (has1, has2) = two_widget_focus_after_key(input, FocusTraversalKeys::none());
        assert!(has1, "No keys should navigate with FocusTraversalKeys::none()");
        assert!(!has2);
    }

    #[test]
    fn test_handle_traversal_not_focused_does_nothing() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        sys.take_focus(id1);

        let mut input = crate::input::Input::default();
        input.key_pressed_right = true;

        sys.begin_frame();
        sys.register(id1);
        let focused2 = sys.register(id2); // id2 does NOT have focus
        sys.handle_traversal(focused2, &input, FocusTraversalKeys::all());
        sys.end_frame();

        sys.begin_frame();
        let has1 = sys.register(id1);
        sys.register(id2);
        sys.end_frame();
        assert!(has1, "Unfocused widget must not trigger traversal");
    }

    #[test]
    fn test_current_focus_returns_focused_id() {
        let mut sys = FocusSystem::new();
        let id = FocusId::new();
        assert_eq!(sys.current_focus(), None);
        sys.take_focus(id);
        assert_eq!(sys.current_focus(), Some(id));
    }

    #[test]
    fn test_focus_system_custom_override() {
        let mut sys = FocusSystem::new();
        let id1 = FocusId::new();
        let id2 = FocusId::new();
        let id3 = FocusId::new();

        sys.override_next(id1, id3);

        sys.take_focus(id1);
        
        sys.begin_frame();
        sys.register(id1);
        sys.register(id2);
        sys.register(id3);
        sys.request_shift(FocusDirection::Next);
        sys.end_frame();

        // Should jump from id1 to id3 because of override
        sys.begin_frame();
        assert!(!sys.register(id1));
        assert!(!sys.register(id2));
        assert!(sys.register(id3), "Focus should jump to id3 based on custom override");
    }
}
