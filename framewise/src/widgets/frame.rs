use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::{Layout, LayoutState},
    text::TextSystem,
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    /// Input specification for a frame.
    #[derive(Debug, Clone, PartialEq)]
    pub struct FrameSpec {
        pub rect: Rect,
        pub style: super::FrameStyle,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FrameToken {
        pub fill_index: usize,
        pub border: Option<(Color, f32)>,
    }

    #[derive(Debug, Clone, Copy, PartialEq)]
    pub struct FrameResult {
        pub token: FrameToken,
        pub content_bounds: Rect,
    }

    /// Low-level frame begin function.
    ///
    /// Pushes a placeholder `FillRect` to the command list (so the background draws behind
    /// children) and returns a `FrameResult` with the stable token and content bounds.
    /// The border `StrokeRect`, if any, is pushed in `end_frame` so it draws on top.
    pub fn begin_frame(spec: FrameSpec, cmds: &mut DrawCommands) -> FrameResult {
        let rect = spec.rect;
        let style = spec.style;
        let fill_index = cmds.len();
        cmds.push(DrawCmd::FillRect {
            rect,
            color: style.background,
        });

        let border = if style.border_width > 0.0 {
            Some((style.border, style.border_width))
        } else {
            None
        };

        let inset = style.border_width + style.padding;
        let content = rect.inset(inset);

        FrameResult {
            token: FrameToken { fill_index, border },
            content_bounds: content,
        }
    }

    /// Low-level frame end function.
    ///
    /// Patches the `FillRect` placeholder with the final resolved bounds, then appends
    /// a `StrokeRect` on top of all children (if the frame has a border).
    ///
    /// # Panics
    /// Panics if the `FillRect` placeholder at the recorded index is missing or modified,
    /// indicating corruption of the command list.
    pub fn end_frame(token: FrameToken, rect: Rect, cmds: &mut DrawCommands) {
        match cmds.get_mut(token.fill_index) {
            Some(DrawCmd::FillRect { rect: r, .. }) => *r = rect,
            _ => panic!(
                "DrawCommands corruption detected: placeholder FillRect at index {} was missing or modified!",
                token.fill_index
            ),
        }

        if let Some((color, width)) = token.border {
            cmds.push(DrawCmd::StrokeRect { rect, color, width });
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

/// Visual configuration for a frame (bordered background rectangle).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameStyle {
    pub background: Color,
    pub border: Color,
    pub border_width: f32,
    /// Padding between the border and the content area.
    pub padding: f32,
}

impl FrameStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            background: theme.paper_elev,
            border: theme.ink,
            border_width: theme.border,
            padding: 4.0,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct FrameResult<'b, T: TextSystem, LS: LayoutState, CF> {
    pub layout: LayoutInfo,
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameSpecBuilder {
    pub rect: Option<Rect>,
    pub style: Option<FrameStyle>,
}

impl FrameSpecBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn style(mut self, style: FrameStyle) -> Self {
        self.style = Some(style);
        self
    }
    /// Sets the bounding rectangle. Called automatically by high-level context
    /// functions from the layout engine — only needed when using the raw API directly.
    pub fn rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }
    /// Fills unset fields from `theme`. Called automatically by high-level context
    /// functions — only needed when using the raw API directly.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(FrameStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> raw::FrameSpec {
        raw::FrameSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level container widget function ───────────────────────────────────────────

/// High-level frame container widget function using WidgetContext.
///
/// This function accepts a FrameSpecBuilder, parent layout parameters, and an inner layout,
/// and returns a FrameResult containing the child WidgetContext.
///
/// ### Sizing and fitting
/// Whether the frame fits to its children or respects a fixed/filled footprint is determined
/// dynamically via the generic bounds of `LayoutSpace` (translated from `layout_params` by the parent):
/// - `AxisBound::Exact(w)` -> The frame uses exactly `w` for its extent on that axis.
/// - `AxisBound::Unbounded` / `AxisBound::AtMost` -> The frame sizes itself to the children's extent plus padding.
///
/// ### Lifetime, borrowing, and cursor unlocking
/// The `begin_layout` call mutably borrows the parent `LayoutState` and returns a `LayoutToken`.
/// The `on_finish` closure captures this token by value.
///
/// To satisfy the Rust borrow checker (avoid E0499), we construct the child context by explicitly
/// destructuring the parent `ctx` fields. This disjointly borrows `ctx.layout_state` (held by the `LayoutToken`
/// inside `on_finish`) separately from `ctx.text_system`, `ctx.focus_system`, etc., resulting in a perfectly
/// compile-safe cursor-advance deferral.
pub fn begin_frame<'a, 'b, T: TextSystem, S: LayoutState, L: Layout, CF>(
    ctx: &'b mut WidgetContext<'a, T, S, CF>,
    builder: FrameSpecBuilder,
    layout_params: S::Params,
    inner_layout: L,
) -> FrameResult<'b, T, L::State, impl FnOnce(&mut FocusSystem, &mut DrawCommands, Vec2) + 'b> {
    let mut builder = builder.defaults_from_theme(&ctx.theme);

    // 1. Begin parent layout deferral to get provisional space and borrow-locking token
    let (outer_space, token) = ctx.layout_state.begin_layout(layout_params);

    // 2. Build the spec using the builder
    let provisional_rect = Rect::new(outer_space.x, outer_space.y, 0.0, 0.0);
    builder = builder.rect(provisional_rect);
    let spec = builder.build();
    let inset = spec.style.border_width + spec.style.padding;

    // 3. Push a placeholder FillRect for the background before children so it draws beneath them.
    // The border StrokeRect is pushed in end_frame so it draws on top.
    let raw::FrameResult {
        token: frame_token, ..
    } = raw::begin_frame(spec, ctx.cmds);

    // 4. Inset the provisional space by padding + border_width to allocate child bounds
    let inner_space = outer_space.inset(inset);

    // 4. Define the finish callback which consumes the borrow token and finalizes the parent layout
    let on_finish = move |_: &mut FocusSystem, cmds: &mut DrawCommands, content_extent: Vec2| {
        // Compute outer size: children extent plus container margins
        let outer_extent = Vec2::new(
            content_extent.x + inset * 2.0,
            content_extent.y + inset * 2.0,
        );

        // Finalize layout constraints on the parent and advance its cursor
        let bounds = token.end_layout(outer_extent);

        // Retroactively patch the placeholder draw commands with the actual resolved bounds!
        raw::end_frame(frame_token, bounds, cmds);
    };

    // 5. Disjointly construct the child context to keep the borrows separate
    let child_ctx = WidgetContext {
        //TODO: should be using the child_with_layout_and_on_finish()?
        theme: ctx.theme,
        time: ctx.time,
        clip_rect: ctx.clip_rect,
        text_system: ctx.text_system,
        focus_system: ctx.focus_system,
        input: ctx.input,
        cmds: ctx.cmds,
        layout_state: inner_layout.begin(inner_space),
        on_finish,
    };

    FrameResult {
        layout: LayoutInfo::new(
            Rect::new(outer_space.x, outer_space.y, 0.0, 0.0),
            Rect::new(inner_space.x, inner_space.y, 0.0, 0.0),
        ),
        ctx: child_ctx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::{ColumnLayout, CrossAlign, Extent, SizeReq};
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_frame_layout_and_draw() {
        let mut cmds = DrawCommands::new();
        let rect = Rect::new(10.0, 10.0, 100.0, 50.0);
        let style = FrameStyle {
            background: Color::WHITE,
            border: Color::linear_rgb(0.5, 0.5, 0.5),
            border_width: 2.0,
            padding: 3.0,
        };

        let spec = raw::FrameSpec { rect, style };
        let raw::FrameResult {
            token,
            content_bounds: content,
        } = raw::begin_frame(spec, &mut cmds);

        // Content rect should be inset by border_width + padding = 5.0
        assert_eq!(content.x, 15.0);
        assert_eq!(content.y, 15.0);
        assert_eq!(content.w, 90.0);
        assert_eq!(content.h, 40.0);

        // Only the FillRect placeholder is pushed before children
        assert_eq!(cmds.len(), 1);
        assert!(matches!(cmds[0], DrawCmd::FillRect { .. }));

        // end_frame patches FillRect and appends StrokeRect on top
        let final_rect = Rect::new(10.0, 10.0, 120.0, 60.0);
        raw::end_frame(token, final_rect, &mut cmds);

        assert_eq!(
            &cmds[..],
            &[
                DrawCmd::FillRect {
                    rect: final_rect,
                    color: Color::WHITE,
                },
                DrawCmd::StrokeRect {
                    rect: final_rect,
                    color: Color::linear_rgb(0.5, 0.5, 0.5),
                    width: 2.0,
                },
            ]
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = FrameSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        let expected = FrameStyle::from_theme(&theme);
        assert_eq!(builder.style.unwrap().border_width, expected.border_width);
        assert_eq!(builder.style.unwrap().padding, expected.padding);
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_style() {
        let theme = crate::theme::Theme::framewise();
        let custom_style = FrameStyle {
            background: Color::TRANSPARENT,
            border: Color::TRANSPARENT,
            border_width: 99.0,
            padding: 0.0,
        };
        let builder = FrameSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().border_width, 99.0);
    }

    #[test]
    fn test_high_level_container_fit_to_children() {
        let mut ts = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = DrawCommands::new();

        let mut ctx = WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut ts,
            &mut focus,
            &input,
            ColumnLayout {
                spacing: 10.0,
                align: CrossAlign::Start,
            }
            .begin(Rect::new(0.0, 0.0, 400.0, 600.0)),
            &mut cmds,
        );

        // 1. Begin an auto-sizing frame inside the column
        let style = FrameStyle {
            background: Color::WHITE,
            border: Color::BLACK,
            border_width: 2.0,
            padding: 8.0,
        };
        let FrameResult {
            layout: _layout,
            ctx: mut f_ctx,
        } = begin_frame(
            &mut ctx,
            FrameSpecBuilder::new().style(style),
            SizeReq {
                width: Extent::Fill,
                height: Extent::Auto,
            },
            ColumnLayout {
                spacing: 5.0,
                align: CrossAlign::Start,
            },
        );

        // 2. Place some children inside the frame context
        // Inner layout starts at (10, 10) due to insets. Fill width spans outer space (400 - 20) = 380.
        let r1 = f_ctx.layout_state.layout(
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(20.0),
            },
            crate::layout::IntrinsicSize::UNKNOWN,
        );
        assert_eq!(r1, Rect::new(10.0, 10.0, 380.0, 20.0));

        let r2 = f_ctx.layout_state.layout(
            SizeReq {
                width: Extent::Fill,
                height: Extent::Fixed(30.0),
            },
            crate::layout::IntrinsicSize::UNKNOWN,
        );
        // stack height: 20 + spacing(5) = 25
        assert_eq!(r2, Rect::new(10.0, 35.0, 380.0, 30.0));

        // 3. Finish the frame!
        f_ctx.finish();

        // 4. Verify outer column layout advanced correctly.
        // Child content extent is: width 380, height (35 + 30 - 10) = 55.
        // Total outer size is: height = 55 + inset * 2 = 75.
        // Next sibling y should be: height(75) + spacing(10) = 85.
        let sibling = ctx.layout_state.layout(
            SizeReq::fixed(50.0, 30.0),
            crate::layout::IntrinsicSize::UNKNOWN,
        );
        assert_eq!(sibling.y, 85.0);
    }
}
