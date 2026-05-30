#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Robert-Hughes/Framewise/main/logo/framewise-mark.svg"
)]
//! Framewise — a Rust GUI library where the app is always in control.
//!
//! Framewise is a small, procedural library that helps an application describe
//! and draw GUI elements for the current frame. It does not retain a widget
//! tree, does not own an update model, and has zero rendering dependencies.
//!
//! # Quick start
//!
//! ```ignore
//! use framewise::{Builder, BuilderCtx, Input, Rect};
//!
//! fn draw(ui: &mut Builder, input: &Input) {
//!     let btn = ui.button(Rect::new(20.0, 20.0, 120.0, 36.0), "Click me", input);
//!     if btn.input.clicked {
//!         println!("clicked!");
//!     }
//!     let cmds = ui.finish(); // hand to your renderer
//! }
//! ```

pub mod draw;
pub mod focus;
pub mod input;
pub mod layout;
pub mod text;
pub mod theme;
pub mod types;
pub mod widget;
pub mod widgets;

#[cfg(test)]
pub mod test_utils;
// ── Public re-exports ─────────────────────────────────────────────────────────

pub use draw::{DrawCmd, DrawCommands};
pub use input::Input;
pub use layout::{IntrinsicSize, LAYOUT_FALLBACK_SIZE};
pub use text::{FontId, FontRole, TextHandle, TextLayout, TextSystem};
pub use theme::Theme;
pub use types::{ClipRect, Color, Rect, Vec2};
pub use widget::{InputInfo, LayoutInfo, WidgetContext};

// Widget functions (low-level API)
#[cfg(feature = "button")]
pub use widgets::button::{button, ButtonResult, ButtonStyle};
#[cfg(feature = "frame")]
pub use widgets::frame::{frame, FrameResult, FrameStyle};
#[cfg(feature = "label")]
pub use widgets::label::{label, LabelResult};
