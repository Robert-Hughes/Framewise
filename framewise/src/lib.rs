#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Robert-Hughes/Framewise/main/logo/framewise-mark.svg"
)]
//! Framewise — a Rust GUI library where the app is always in control.
//!
//! Framewise is a small, procedural library that helps an application describe
//! and draw GUI elements for the current frame. It does not retain a widget
//! tree, does not own an update model, and has zero rendering dependencies.

pub mod draw;
pub mod focus;
pub mod input;
pub mod layout;
pub mod layouts;
pub mod output;
pub mod text;
pub mod theme;
pub mod types;
pub mod widget;
pub mod widgets;

#[cfg(test)]
pub mod test_utils;
// ── Public re-exports ─────────────────────────────────────────────────────────

pub use draw::{BorderPlacement, DrawCmd, DrawCommands, DrawGlyph, PreparedGlyphToken};
pub use input::{Input, Key, KeySet};
pub use layout::{
    Align, AxisBound, LayoutSpace, Placement, Placement2D, Size, SizeOffer, SizeRequest,
};
pub use layouts::{
    ColumnLayout, ColumnLayoutParams, ColumnState, LinearCross, LinearMain, LinearSpacer,
    MainAxisAlign, ManualLayout, ManualState, OffsetLayout, OffsetState, RowLayout,
    RowLayoutParams, RowState, SplitRow, SplitRowState, WrapLayout, WrapState,
};
pub use output::{CursorIcon, Output};
pub use text::{
    cluster_approx_ink_bounds, CaretGeom, CaretPosition, ContentPlacement, EllipsisFallback,
    FontId, FontRole, LineEndKind, LineHeight, LineMetrics, OverflowX, OverflowY,
    PrepareGlyphRequest, ShapedCluster, ShapedGlyph, ShapedText, SharedShapedText, TextBackend,
    TextBounds, TextContentBasis, TextContentPlacement, TextFlow, TextLayout, TextLineAlign,
    TextLineLayoutMetrics, TextMetrics, TextStyle, WrapClusterFallback, WrapWordFallback,
};
pub use theme::Theme;
pub use types::{ClipRect, Color, Layer, Outline, Rect, Stroke, Vec2};
pub use widget::{InputInfo, LayoutInfo, LayoutViolationPolicy, WidgetContext};
