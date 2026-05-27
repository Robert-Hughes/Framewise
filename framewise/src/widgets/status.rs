use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    text::FontId,
    types::{Color, Rect},
    widget::WidgetContext,
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct StatusSpec<'a> {
        pub rect: Rect,
        pub label: &'a str,
        pub font: FontId,
        pub variant: super::StatusVariant,
        pub style: super::StatusStyle,
    }

    /// Low-level status widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn status<'a, T: crate::text::TextSystem>(
        spec: StatusSpec<'a>,
        text_system: &mut T,
    ) -> StatusResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let dot_size = s.dot_size;
        let gap = s.gap;

        let dot_color = match spec.variant {
            StatusVariant::Neutral => s.neutral,
            StatusVariant::Ok => s.ok,
            StatusVariant::Warn => s.warn,
            StatusVariant::Err => s.err,
            StatusVariant::Live => s.live,
        };

        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(spec.rect.x, spec.rect.y, dot_size, dot_size),
            color: dot_color,
        });

        let label_upper = spec.label.to_uppercase();
        let layout = text_system.prepare(&label_upper, s.text_size, spec.font);
        let ty = spec.rect.y + (dot_size - layout.size.y) * 0.5;
        cmds.push(DrawCmd::Text {
            rect: Rect::new(
                spec.rect.x + dot_size + gap,
                ty,
                layout.size.x,
                layout.size.y,
            ),
            color: s.text,
            handle: layout.handle,
        });

        StatusResult { draw: cmds }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StatusVariant {
    Neutral,
    Ok,
    Warn,
    Err,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusStyle {
    pub dot_size: f32,
    pub gap: f32,
    pub text_size: f32,
    pub neutral: Color,
    pub ok: Color,
    pub warn: Color,
    pub err: Color,
    pub live: Color,
    pub text: Color,
}

pub struct StatusResult {
    pub draw: DrawCommands,
}

impl StatusResult {
    pub fn into_parts(self) -> (DrawCommands, ()) {
        (self.draw, ())
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level status widget function using WidgetContext.
///
/// This function accepts a StatusSpec and calls the low-level raw::status function.
pub fn status<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: StatusSpecBuilder<'a>,
) {
    let rect = ctx.layout(layout_params);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::status(spec, ctx.text_system);
    ctx.append_cmds(result.draw.0);
}

#[derive(Debug, Clone, PartialEq)]
pub struct StatusSpecBuilder<'a> {
    pub label: Option<&'a str>,
    pub font: Option<FontId>,
    pub style: Option<StatusStyle>,
    pub variant: Option<StatusVariant>,
    pub rect: Option<Rect>,
}

impl<'a> StatusSpecBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            font: None,
            style: None,
            variant: None,
            rect: None,
        }
    }

    pub fn label(mut self, label: &'a str) -> Self {
        self.label = Some(label);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: StatusStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn variant(mut self, variant: StatusVariant) -> Self {
        self.variant = Some(variant);
        self
    }
}

impl<'a> StatusSpecBuilder<'a> {
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
            self.style = Some(theme.status_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::StatusSpec<'a> {
        raw::StatusSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            label: self.label.expect("label not set — call .label()"),
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

#[cfg(test)]
mod tests {
    use super::*;
    use super::raw::StatusSpec;
    use crate::test_utils::DummyTextSys;

    #[test]
    fn test_status_visual_ok() {
        let mut text_sys = DummyTextSys;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            label: "Online",
            font: FontId(0),
            variant: StatusVariant::Ok,
            style: crate::theme::Theme::framewise().status_style(),
        };
        let style = spec.style;
        let res = raw::status(spec, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.ok,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, -5.0, 48.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_status_visual_warn() {
        let mut text_sys = DummyTextSys;
        let spec = StatusSpec {
            rect: Rect::new(0.0, 0.0, 100.0, 20.0),
            label: "Warning",
            font: FontId(0),
            variant: StatusVariant::Warn,
            style: crate::theme::Theme::framewise().status_style(),
        };
        let style = spec.style;
        let res = raw::status(spec, &mut text_sys);

        assert_eq!(
            res.draw,
            DrawCommands(vec![
                DrawCmd::FillRect {
                    rect: Rect::new(0.0, 0.0, 6.0, 6.0),
                    color: style.warn,
                },
                DrawCmd::Text {
                    rect: Rect::new(14.0, -5.0, 56.0, 16.0),
                    color: style.text,
                    handle: crate::text::TextHandle(0),
                },
            ])
        );
    }

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = StatusSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.status_style()));
        assert_eq!(builder.font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.status_style();
        custom_style.text_size = 99.0;
        let builder = StatusSpecBuilder::new()
            .style(custom_style)
            .font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }
}
