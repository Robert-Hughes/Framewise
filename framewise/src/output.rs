/// Cursor icon styles that widgets can request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorIcon {
    Pointer,
    Text,
    Grab,
    Grabbing,
    EwResize,
    NsResize,
    NwseResize,
    Move,
    NotAllowed,
    AllScroll,
}

/// Per-frame side effects requested by widgets.
///
/// `Input` contains user/system events entering Framewise for the current
/// frame. `Output` is the opposite direction: it contains requests produced by
/// widgets which the application shell should handle after the widget tree has
/// been evaluated.
///
/// This is for global application state, such as the system clipboard, where
/// it is awkward for every widget caller to handle the side effect immediately.
///
/// It is not a replacement for regular widget return values like
/// `ButtonResult::clicked` or `TextEditResult::changed`. Those are
/// widget-specific results which are usually useful to the immediate caller
/// while building the UI.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    /// Text that should replace the system clipboard contents after this frame.
    ///
    /// The application should reset this to `None` at the start of each frame.
    /// Widgets may set it when handling copy/cut-style input. If more than one
    /// widget sets it in a frame, the last write wins.
    pub new_clipboard_contents: Option<String>,

    /// A per-frame request to the application shell to change the mouse cursor icon.
    ///
    /// The application shell should fall back to its normal/default cursor when no
    /// widget requests a special cursor this frame (i.e. this field is `None`).
    ///
    /// If multiple widgets request a cursor in the same frame, the last write wins.
    pub cursor_icon: Option<CursorIcon>,
}

impl Output {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset per-frame output state before evaluating a new frame.
    pub fn clear_frame_state(&mut self) {
        self.new_clipboard_contents = None;
        self.cursor_icon = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_frame_state_resets_new_clipboard_contents_and_cursor_icon() {
        let mut output = Output {
            new_clipboard_contents: Some("copied".to_string()),
            cursor_icon: Some(CursorIcon::Text),
        };

        output.clear_frame_state();

        assert_eq!(output.new_clipboard_contents, None);
        assert_eq!(output.cursor_icon, None);
    }
}
