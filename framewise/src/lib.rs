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
//!     if btn.clicked() {
//!         println!("clicked!");
//!     }
//!     let cmds = ui.finish(); // hand to your renderer
//! }
//! ```

pub mod builder;
pub mod draw;
pub mod input;
pub mod types;
pub mod widget;
pub mod widgets;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use builder::{Builder, BuilderCtx};
pub use draw::{DrawCmd, DrawCommands};
pub use input::Input;
pub use types::{Color, Rect, Vec2};
pub use widget::{InputInfo, LayoutInfo, WidgetResult};

// Widget functions (low-level API)
pub use widgets::button::{button, ButtonInfo, ButtonResult, ButtonSpec, ButtonStyle};
pub use widgets::frame::{frame, FrameInfo, FrameResult, FrameSpec, FrameStyle};
pub use widgets::label::{label, LabelInfo, LabelResult, LabelSpec};
