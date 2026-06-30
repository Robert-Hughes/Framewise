use crate::types::Vec2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    Space,
    Enter,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    PageUp,
    PageDown,
    Home,
    End,
    Escape,
    Tab,
}

impl Key {
    const fn bit(self) -> u64 {
        1 << (self as u64)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct KeySet {
    bits: u64,
}

impl KeySet {
    pub const fn from_key(key: Key) -> Self {
        Self { bits: key.bit() }
    }

    pub fn contains(&self, key: Key) -> bool {
        self.bits & key.bit() != 0
    }

    pub fn insert(&mut self, key: Key) {
        self.bits |= key.bit();
    }

    pub fn remove(&mut self, key: Key) {
        self.bits &= !key.bit();
    }

    pub fn clear(&mut self) {
        self.bits = 0;
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }
}

/// A snapshot of all input state for the current frame.
///
/// The application is responsible for constructing this from its windowing
/// system (e.g. winit) and passing it to widget functions. Framewise never
/// reads from the OS or a windowing library directly.
#[derive(Debug, Clone)]
pub struct Input {
    /// Current mouse cursor position in logical pixels, relative to the
    /// top-left of the window.
    pub mouse_pos: Vec2,

    /// True while the primary (left) mouse button is held down.
    pub mouse_down: bool,

    /// True on the single frame the primary mouse button was pressed.
    pub mouse_pressed: bool,

    /// True on the single frame the primary mouse button was released.
    pub mouse_clicked: bool,

    /// Keys currently held down.
    ///
    /// A key remains in this set from its first press event until its release event.
    /// Repeated key-press events do not change this set.
    pub keys_down: KeySet,

    /// Keys that produced a press event this frame.
    ///
    /// This includes both the initial key-down event and any subsequent key-repeat
    /// press events generated while the key is held. Widgets that want repeated
    /// keyboard actions should usually query this set.
    pub keys_pressed: KeySet,

    /// Keys that produced a release event this frame.
    ///
    /// Key-repeat does not produce entries here; a key appears here only when it is
    /// actually released.
    pub keys_released: KeySet,

    /// Sequence of logical text input events this frame.
    pub text_events: Vec<TextEvent>,

    /// The number of consecutive mouse clicks (1 = single, 2 = double, 3 = triple).
    pub mouse_click_count: u32,

    /// Mouse scroll wheel delta for the current frame.
    pub scroll_delta: Vec2,

    /// True while the Shift modifier key is held. Not a per-frame flag; updated
    /// by the embedder on ModifiersChanged and never cleared by clear_frame_state.
    pub modifier_shift: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEvent {
    Char(char),
    Backspace { ctrl: bool },
    Delete { ctrl: bool },
    CaretLeft { shift: bool, ctrl: bool },
    CaretRight { shift: bool, ctrl: bool },
    CaretHome { shift: bool, ctrl: bool },
    CaretEnd { shift: bool, ctrl: bool },
    CaretUp { shift: bool },
    CaretDown { shift: bool },
    SelectAll,
    Copy,
    Cut,
    Paste(String),
}

impl Default for Input {
    fn default() -> Self {
        Self {
            mouse_pos: Vec2::new(0.0, 0.0),
            mouse_down: false,
            mouse_pressed: false,
            mouse_clicked: false,
            keys_down: KeySet::default(),
            keys_pressed: KeySet::default(),
            keys_released: KeySet::default(),
            text_events: Vec::new(),
            mouse_click_count: 0,
            scroll_delta: Vec2::ZERO,
            modifier_shift: false,
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether `key` is currently held down.
    ///
    /// This is a held-state query. For example, `input.key_down(Key::Enter)`
    /// stays true after the initial press until Enter is released.
    pub fn key_down(&self, key: Key) -> bool {
        self.keys_down.contains(key)
    }

    /// Returns whether `key` produced a press event this frame.
    ///
    /// This includes both initial key-down events and key-repeat press events.
    /// For example, `input.key_pressed(Key::Enter)` is true on each Enter press
    /// event delivered during the current frame.
    pub fn key_pressed(&self, key: Key) -> bool {
        self.keys_pressed.contains(key)
    }

    /// Returns whether `key` produced a release event this frame.
    ///
    /// Key-repeat does not affect this query. For example,
    /// `input.key_released(Key::Enter)` is true only on an actual Enter release.
    pub fn key_released(&self, key: Key) -> bool {
        self.keys_released.contains(key)
    }

    /// Reset per-frame state (called at the end of the frame).
    pub fn clear_frame_state(&mut self) {
        self.mouse_pressed = false;
        self.mouse_clicked = false;
        self.keys_pressed.clear();
        self.keys_released.clear();
        self.text_events.clear();
        self.scroll_delta = Vec2::new(0.0, 0.0);
        self.mouse_click_count = 0;
        // modifier_shift intentionally not cleared: it is a held state updated
        // by the embedder on ModifiersChanged, not a per-frame press event.
    }
}

/// A helper to track mouse clicks and determine double/triple clicks
/// based on time and distance thresholds.
#[derive(Debug, Clone)]
pub struct ClickTracker {
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_pos: Vec2,
    pub click_count: u32,
}

impl Default for ClickTracker {
    fn default() -> Self {
        Self {
            last_click_time: None,
            last_click_pos: Vec2::new(0.0, 0.0),
            click_count: 0,
        }
    }
}

impl ClickTracker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_click(&mut self, pos: Vec2, now: std::time::Instant) -> u32 {
        let is_double_click = if let Some(last_time) = self.last_click_time {
            let elapsed = now.duration_since(last_time).as_millis();
            let dx = pos.x - self.last_click_pos.x;
            let dy = pos.y - self.last_click_pos.y;
            let dist_sq = dx * dx + dy * dy;
            // 300ms time threshold, 5px radius distance threshold (25px squared)
            elapsed < 300 && dist_sq < 25.0
        } else {
            false
        };

        if is_double_click {
            self.click_count += 1;
        } else {
            self.click_count = 1;
        }

        self.last_click_time = Some(now);
        self.last_click_pos = pos;
        self.click_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn key_set_default_is_empty() {
        let keys = KeySet::default();

        assert!(keys.is_empty());
        assert!(!keys.contains(Key::Enter));
    }

    #[test]
    fn key_set_insert_makes_contains_true() {
        let mut keys = KeySet::default();

        keys.insert(Key::Enter);

        assert!(keys.contains(Key::Enter));
        assert!(!keys.is_empty());
    }

    #[test]
    fn key_set_remove_makes_contains_false() {
        let mut keys = KeySet::default();
        keys.insert(Key::Enter);

        keys.remove(Key::Enter);

        assert!(!keys.contains(Key::Enter));
        assert!(keys.is_empty());
    }

    #[test]
    fn key_set_multiple_keys_are_independent() {
        let mut keys = KeySet::default();

        keys.insert(Key::Enter);
        keys.insert(Key::Space);
        keys.remove(Key::Enter);

        assert!(!keys.contains(Key::Enter));
        assert!(keys.contains(Key::Space));
    }

    #[test]
    fn key_set_clear_removes_all_keys() {
        let mut keys = KeySet::default();
        keys.insert(Key::Enter);
        keys.insert(Key::Space);

        keys.clear();

        assert!(keys.is_empty());
        assert!(!keys.contains(Key::Enter));
        assert!(!keys.contains(Key::Space));
    }

    #[test]
    fn clear_frame_state_preserves_keys_down_only() {
        let mut input = Input::new();
        input.keys_down.insert(Key::Enter);
        input.keys_pressed.insert(Key::Enter);
        input.keys_released.insert(Key::Space);

        input.clear_frame_state();

        assert!(input.key_down(Key::Enter));
        assert!(input.keys_pressed.is_empty());
        assert!(input.keys_released.is_empty());
    }

    #[test]
    fn test_click_tracker_distance() {
        let mut tracker = ClickTracker::new();
        let start = std::time::Instant::now();

        // First click
        let count = tracker.register_click(Vec2::new(10.0, 10.0), start);
        assert_eq!(count, 1);

        // Second click, far away (should reset to 1)
        let count =
            tracker.register_click(Vec2::new(100.0, 100.0), start + Duration::from_millis(100));
        assert_eq!(count, 1, "Click was far away, should not be a double click");

        // Third click, close by and quick (should be a double click)
        let count =
            tracker.register_click(Vec2::new(101.0, 102.0), start + Duration::from_millis(150));
        assert_eq!(count, 2, "Click was close, should be a double click");

        // Fourth click, close by but too late (should reset to 1)
        let count =
            tracker.register_click(Vec2::new(101.0, 102.0), start + Duration::from_millis(500));
        assert_eq!(count, 1, "Click was too late, should reset");
    }
}
