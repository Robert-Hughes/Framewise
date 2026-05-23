use crate::types::Vec2;

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

    /// True on the single frame the Enter key was pressed.
    pub key_pressed_enter: bool,

    /// True while the Spacebar is held down.
    pub key_down_space: bool,

    /// True on the single frame the Spacebar was pressed.
    pub key_pressed_space: bool,

    /// True on the single frame the Spacebar was released.
    pub key_released_space: bool,

    /// Sequence of logical text input events this frame.
    pub text_events: Vec<TextEvent>,

    /// The number of consecutive mouse clicks (1 = single, 2 = double, 3 = triple).
    pub mouse_click_count: u32,

    /// Mouse scroll wheel delta for the current frame.
    pub scroll_delta: Vec2,

    pub key_pressed_page_up: bool,
    pub key_pressed_page_down: bool,
    pub key_pressed_home: bool,
    pub key_pressed_end: bool,
    pub key_pressed_up: bool,
    pub key_pressed_down: bool,
    pub key_pressed_left: bool,
    pub key_pressed_right: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEvent {
    Char(char),
    Backspace { ctrl: bool },
    Delete { ctrl: bool },
    CaretLeft { shift: bool, ctrl: bool },
    CaretRight { shift: bool, ctrl: bool },
    CaretHome { shift: bool },
    CaretEnd { shift: bool },
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
            key_pressed_enter: false,
            key_down_space: false,
            key_pressed_space: false,
            key_released_space: false,
            text_events: Vec::new(),
            mouse_click_count: 0,
            scroll_delta: Vec2::ZERO,
            key_pressed_page_up: false,
            key_pressed_page_down: false,
            key_pressed_home: false,
            key_pressed_end: false,
            key_pressed_up: false,
            key_pressed_down: false,
            key_pressed_left: false,
            key_pressed_right: false,
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset per-frame state (called at the end of the frame).
    pub fn clear_frame_state(&mut self) {
        self.mouse_pressed      = false;
        self.mouse_clicked      = false;
        self.key_pressed_enter  = false;
        self.key_pressed_up     = false;
        self.key_pressed_down   = false;
        self.key_pressed_left   = false;
        self.key_pressed_right  = false;
        self.key_pressed_space  = false;
        self.key_released_space = false;
        self.text_events.clear();
        self.scroll_delta       = Vec2::new(0.0, 0.0);
        self.mouse_click_count  = 0;
        self.key_pressed_page_up = false;
        self.key_pressed_page_down = false;
        self.key_pressed_home = false;
        self.key_pressed_end = false;
        self.key_pressed_up = false;
        self.key_pressed_down = false;
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
    fn test_click_tracker_distance() {
        let mut tracker = ClickTracker::new();
        let start = std::time::Instant::now();
        
        // First click
        let count = tracker.register_click(Vec2::new(10.0, 10.0), start);
        assert_eq!(count, 1);
        
        // Second click, far away (should reset to 1)
        let count = tracker.register_click(Vec2::new(100.0, 100.0), start + Duration::from_millis(100));
        assert_eq!(count, 1, "Click was far away, should not be a double click");
        
        // Third click, close by and quick (should be a double click)
        let count = tracker.register_click(Vec2::new(101.0, 102.0), start + Duration::from_millis(150));
        assert_eq!(count, 2, "Click was close, should be a double click");
        
        // Fourth click, close by but too late (should reset to 1)
        let count = tracker.register_click(Vec2::new(101.0, 102.0), start + Duration::from_millis(500));
        assert_eq!(count, 1, "Click was too late, should reset");
    }
}
