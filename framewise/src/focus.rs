use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;

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
    /// Upward-scroll claim being accumulated this frame.
    next_scroll_up_id: Option<FocusId>,
    /// Downward-scroll claim being accumulated this frame.
    next_scroll_down_id: Option<FocusId>,
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
            next_scroll_up_id: None,
            next_scroll_down_id: None,
        }
    }

    pub fn begin_frame(&mut self) {
        // As requested, clearing is done in end_frame.
        // This is left empty for future use.
    }

    /// Register a widget in the current frame's focus order.
    /// Returns true if the widget currently has focus.
    pub fn register(&mut self, id: FocusId) -> bool {
        self.current_frame_order.push(id);
        self.focused_id == Some(id)
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

    /// Claim the upward-scroll slot for this frame.
    ///
    /// Because widgets are evaluated top-down, inner widgets evaluate last and
    /// naturally overwrite outer claims — so the innermost hovered widget wins.
    /// A widget should only call this if it can actually scroll upward (i.e. is
    /// not already at its minimum). This lets the parent keep its claim when the
    /// child is at its limit, enabling natural scroll propagation.
    pub fn claim_scroll_up(&mut self, id: FocusId) {
        self.next_scroll_up_id = Some(id);
    }

    /// Claim the downward-scroll slot for this frame.
    /// See [`claim_scroll_up`] for the full explanation.
    pub fn claim_scroll_down(&mut self, id: FocusId) {
        self.next_scroll_down_id = Some(id);
    }

    /// Returns true if this widget won the upward-scroll claim in the previous frame.
    pub fn is_active_scroll_up(&self, id: FocusId) -> bool {
        self.active_scroll_up_id == Some(id)
    }

    /// Returns true if this widget won the downward-scroll claim in the previous frame.
    pub fn is_active_scroll_down(&self, id: FocusId) -> bool {
        self.active_scroll_down_id == Some(id)
    }

    /// Resolves any pending focus shifts using the order built this frame.
    pub fn end_frame(&mut self) {
        self.active_scroll_up_id = self.next_scroll_up_id.take();
        self.active_scroll_down_id = self.next_scroll_down_id.take();

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
