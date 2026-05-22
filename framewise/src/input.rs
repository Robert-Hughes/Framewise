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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextEvent {
    Char(char),
    Backspace,
    Delete,
    CursorLeft { shift: bool, ctrl: bool },
    CursorRight { shift: bool, ctrl: bool },
    CursorHome { shift: bool },
    CursorEnd { shift: bool },
    SelectAll,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            mouse_down: false,
            mouse_pressed: false,
            mouse_clicked: false,
            key_pressed_enter: false,
            key_down_space: false,
            key_pressed_space: false,
            key_released_space: false,
            text_events: Vec::new(),
            mouse_click_count: 0,
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }
}
