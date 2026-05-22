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

    /// True on the single frame the Spacebar was pressed.
    pub key_pressed_space: bool,
}

impl Default for Input {
    fn default() -> Self {
        Self {
            mouse_pos: Vec2::ZERO,
            mouse_down: false,
            mouse_pressed: false,
            mouse_clicked: false,
            key_pressed_enter: false,
            key_pressed_space: false,
        }
    }
}

impl Input {
    pub fn new() -> Self {
        Self::default()
    }
}
