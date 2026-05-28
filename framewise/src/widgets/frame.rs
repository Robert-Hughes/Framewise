use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    types::{Color, Rect},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct FrameSpec {
        pub rect: Rect,
        pub style: super::FrameStyle,
    }

    /// Low-level frame widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn frame(spec: FrameSpec) -> FrameResult {
        let mut draw = DrawCommands::new();

        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: spec.style.background,
        });

        if spec.style.border_width > 0.0 {
            draw.push(DrawCmd::StrokeRect {
                rect: spec.rect,
                color: spec.style.border,
                width: spec.style.border_width,
            });
        }

        let inset = spec.style.border_width + spec.style.padding;
        let content = spec.rect.inset(inset);

        FrameResult {
            draw,
            content_bounds: content,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct FrameResult {
        pub draw: DrawCommands,
        pub content_bounds: Rect,
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

// ── Result ───────────────────────────────────────────────────────────────────

pub struct FrameResult {
    pub layout: LayoutInfo,
}

impl FrameResult {
    /// The content area inside the frame's border and padding.
    pub fn content_rect(&self) -> Rect {
        self.layout.content_bounds
    }
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FrameSpecBuilder {
    pub style: Option<FrameStyle>,
    pub rect: Option<Rect>,
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
            self.style = Some(theme.frame_style());
        }
        self
    }
    pub fn build(self) -> raw::FrameSpec {
        raw::FrameSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level frame widget function using WidgetContext.
///
/// This function accepts a FrameSpecBuilder and layout parameters, resolves layout and styles internally,
/// and calls the low-level raw::frame function.
pub fn frame<
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: FrameSpecBuilder,
) -> FrameResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::frame(spec);

    ctx.append_cmds(result.draw);

    FrameResult {
        layout: LayoutInfo::new(rect, result.content_bounds),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::FrameSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_frame_layout_and_draw() {
        let spec = FrameSpec {
            rect: Rect::new(10.0, 10.0, 100.0, 50.0),
            style: FrameStyle {
                background: Color::WHITE,
                border: Color::linear_rgb(0.5, 0.5, 0.5),
                border_width: 2.0,
                padding: 3.0,
            },
        };

        let res = raw::frame(spec);

        // Content rect should be inset by border_width + padding = 5.0
        let content = res.content_bounds;
        assert_eq!(content.x, 15.0);
        assert_eq!(content.y, 15.0);
        assert_eq!(content.w, 90.0);
        assert_eq!(content.h, 40.0);

        // Should draw background and border
        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 50.0),
                    color: Color::WHITE,
                },
                DrawCmd::StrokeRect {
                    rect: Rect::new(10.0, 10.0, 100.0, 50.0),
                    color: Color::linear_rgb(0.5, 0.5, 0.5),
                    width: 2.0,
                },
            ])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_style() {
        let theme = crate::theme::Theme::framewise();
        let builder = FrameSpecBuilder::new();
        assert!(builder.style.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert!(builder.style.is_some());
        let expected = theme.frame_style();
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
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::frame(
            &mut ctx,
            layout_rect,
            FrameSpecBuilder::new().rect(custom_rect),
        );
        assert_eq!(result.layout.bounds, custom_rect);
    }
}
