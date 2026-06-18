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
pub mod layouts;
pub mod text;
pub mod theme;
pub mod types;
pub mod widget;
pub mod widgets;

#[cfg(test)]
pub mod test_utils;
// ── Public re-exports ─────────────────────────────────────────────────────────

pub use draw::{DrawCmd, DrawCommands, DrawGlyph, PreparedGlyphHandle};
pub use input::Input;
pub use layout::{Align, AxisBound, IntrinsicSize, LayoutSpace, Placement, Placement2D, Size};
pub use layouts::{
    ColumnLayout, ColumnLayoutParams, ColumnState, LinearCross, LinearMain, LinearSpacer,
    MainAxisAlign, ManualLayout, ManualState, OffsetLayout, OffsetState, RowLayout,
    RowLayoutParams, RowState, SplitRow, SplitRowState, WrapLayout, WrapState,
};
pub use text::{
    cluster_approx_ink_bounds, CaretGeom, CaretPosition, ContentPlacement, EllipsisFallback,
    FontId, FontRole, LineEndKind, LineHeight, LineMetrics, OverflowX, OverflowY,
    PrepareGlyphRequest, ShapedCluster, ShapedGlyph, ShapedText, SharedShapedText, TextBackend,
    TextBounds, TextContentBasis, TextContentPlacement, TextFlow, TextLayout, TextLineAlign,
    TextLineLayoutMetrics, TextMetrics, TextStyle, WrapClusterFallback, WrapWordFallback,
};
pub use theme::Theme;
pub use types::{ClipRect, Color, Layer, Rect, Vec2};
pub use widget::{InputInfo, LayoutInfo, LayoutViolationPolicy, WidgetContext};

// Widget functions (low-level API)
#[cfg(feature = "button")]
pub use widgets::button::{button, ButtonResult, ButtonStyle};
#[cfg(feature = "frame")]
pub use widgets::frame::{begin_frame, FrameResult, FrameSpecBuilder, FrameStyle};
#[cfg(feature = "label")]
pub use widgets::label::{label, LabelResult};
