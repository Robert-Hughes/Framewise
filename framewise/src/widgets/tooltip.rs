use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::LayoutState,
    text::{FontId, TextSystem},
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipSpec<'a> {
        pub rect: Rect,
        pub text: &'a str,
        pub font: FontId,
        pub variant: super::TooltipVariant,
        pub style: super::TooltipStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct TooltipResult {
        pub draw: DrawCommands,
    }

    /// Low-level tooltip widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn tooltip<'a, T: TextSystem>(spec: TooltipSpec<'a>, text_system: &mut T) -> TooltipResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let pad_x = s.pad_x;
        let pad_y_top = s.pad_y_top;
        let pad_y_bot = s.pad_y_bot;
        let arrow_h = s.arrow_h;
        let arrow_w = s.arrow_w;

        let (bg, text_color): (Color, Color) = match spec.variant {
            TooltipVariant::Dark => (s.dark_bg, s.dark_text),
            TooltipVariant::Rust => (s.rust_bg, s.rust_text),
        };

        let layout = text_system.prepare(spec.text, s.text_size, spec.font);
        let box_w = (layout.size.x + pad_x * 2.0).min(s.max_width);
        let box_h = layout.size.y + pad_y_top + pad_y_bot;

        let r = Rect::new(spec.rect.x, spec.rect.y, box_w, box_h);
        cmds.push(DrawCmd::FillRect { rect: r, color: bg });

        cmds.push(DrawCmd::Text {
            rect: Rect::new(r.x + pad_x, r.y + pad_y_top, layout.size.x, layout.size.y),
            color: text_color,
            handle: layout.handle,
        });

        // Arrow triangle below (two lines converging to a point).
        let arrow_x = r.x + s.arrow_x;
        let arrow_y = r.y + box_h;
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(arrow_x, arrow_y),
            p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
            color: bg,
            width: s.arrow_width,
        });
        cmds.push(DrawCmd::StrokeLine {
            p0: Vec2::new(arrow_x + arrow_w, arrow_y),
            p1: Vec2::new(arrow_x + arrow_w * 0.5, arrow_y + arrow_h),
            color: bg,
            width: s.arrow_width,
        });

        TooltipResult { draw: cmds }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TooltipVariant {
    Dark,
    Rust,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipStyle {
    pub text_size: f32,
    pub pad_x: f32,
    pub pad_y_top: f32,
    pub pad_y_bot: f32,
    pub arrow_h: f32,
    pub arrow_w: f32,
    pub arrow_x: f32,
    pub max_width: f32,
    pub dark_bg: Color,
    pub dark_text: Color,
    pub rust_bg: Color,
    pub rust_text: Color,
    pub arrow_width: f32,
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct TooltipResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TooltipSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub text: Option<&'a str>,
    pub font: Option<FontId>,
    pub variant: Option<TooltipVariant>,
    pub style: Option<TooltipStyle>,
}

impl<'a> TooltipSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn text(mut self, text: &'a str) -> Self {
        self.text = Some(text);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: TooltipStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: TooltipVariant) -> Self {
        self.variant = Some(variant);
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
            self.style = Some(theme.tooltip_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::TooltipSpec<'a> {
        raw::TooltipSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            text: self.text.expect("text not set — call .text()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            variant: self.variant.expect("variant not set — call .variant()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level tooltip widget function using WidgetContext.
///
/// This function accepts a TooltipSpecBuilder and calls the low-level raw::tooltip function.
pub fn tooltip<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: TooltipSpecBuilder<'a>,
    layout_params: S::Params,
) -> TooltipResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::tooltip(spec, ctx.text_system);
    ctx.append_cmds(result.draw);
    TooltipResult {
        layout: LayoutInfo::tight(rect),
    }
}

#[cfg(test)]
mod tests {
    use super::raw::TooltipSpec;
    use super::*;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_tooltip_visual_dark() {
        let mut text_sys = DummyTextSys;
        let spec = TooltipSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            font: FontId(0),
            variant: TooltipVariant::Dark,
            style: crate::theme::Theme::framewise().tooltip_style(),
        };
        let style = spec.style;
        let res = raw::tooltip(spec, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                    color: style.dark_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 5.0, 56.0, 16.0),
                    color: style.dark_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(14.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.dark_bg,
                    width: style.arrow_width,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(22.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.dark_bg,
                    width: style.arrow_width,
                },
            ])
        );
    }

    #[test]
    fn test_tooltip_visual_rust() {
        let mut text_sys = DummyTextSys;
        let spec = TooltipSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 50.0),
            text: "Tooltip",
            font: FontId(0),
            variant: TooltipVariant::Rust,
            style: crate::theme::Theme::framewise().tooltip_style(),
        };
        let style = spec.style;
        let res = raw::tooltip(spec, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 72.0, 27.0),
                    color: style.rust_bg,
                },
                DrawCmd::Text {
                    rect: Rect::new(8.0, 5.0, 56.0, 16.0),
                    color: style.rust_text,
                    handle: crate::text::TextHandle(0),
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(14.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.rust_bg,
                    width: style.arrow_width,
                },
                DrawCmd::StrokeLine {
                    p0: Vec2::new(22.0, 27.0),
                    p1: Vec2::new(18.0, 31.0),
                    color: style.rust_bg,
                    width: style.arrow_width,
                },
            ])
        );
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        let mut text_sys = DummyTextSys;
        let mut focus = FocusSystem::new();
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
        super::tooltip(
            &mut ctx,
            TooltipSpecBuilder::new()
                .text("hi")
                .variant(TooltipVariant::Dark)
                .rect(custom_rect),
            layout_rect,
        );
        // First draw command is FillRect for the box at (custom_rect.x, custom_rect.y)
        match &cmds[0] {
            crate::draw::DrawCmd::FillRect { rect, .. } => {
                assert_eq!(rect.x, custom_rect.x);
                assert_eq!(rect.y, custom_rect.y);
            }
            other => panic!("Expected FillRect, got {:?}", other),
        }
    }
}
