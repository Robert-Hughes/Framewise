use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    text::FontId,
    types::{Color, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct WindowSpec<'a> {
        pub rect: Rect,
        pub title: &'a str,
        pub buttons: &'a [super::WindowButton],
        pub font: FontId,
        pub status_bar: bool,
        pub status_text: Option<&'a str>,
        pub style: super::WindowStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct WindowResult {
        pub draw: DrawCommands,
        pub content_bounds: Rect,
    }

    /// Low-level window begin function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn begin_window<'a, T: crate::text::TextSystem>(
        spec: WindowSpec<'a>,
        text_system: &mut T,
    ) -> WindowResult {
        let mut draw = Vec::new();
        let s = spec.style;

        let title_h = s.title_height;
        let btn_size = s.button_size;
        let status_h = if spec.status_bar {
            s.status_height
        } else {
            0.0
        };

        // Body.
        draw.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: s.background,
        });
        draw.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: s.border,
            width: s.border_width,
        });

        // Title bar.
        let title_rect = Rect::new(spec.rect.x, spec.rect.y, spec.rect.w, title_h);
        draw.push(DrawCmd::FillRect {
            rect: title_rect,
            color: s.title_bg,
        });

        let title_upper = spec.title.to_uppercase();
        let title_layout = text_system.prepare(&title_upper, s.text_size, spec.font);
        let tty = spec.rect.y + (title_h - title_layout.size.y) * 0.5;
        draw.push(DrawCmd::Text {
            rect: Rect::new(
                spec.rect.x + s.text_pad_x,
                tty,
                title_layout.size.x,
                title_layout.size.y,
            ),
            color: s.title_text,
            handle: title_layout.handle,
        });

        // Window buttons (right side).
        let mut btn_x = spec.rect.x + spec.rect.w - s.button_right_pad;
        for btn in spec.buttons.iter().rev() {
            btn_x -= btn_size + s.button_gap;
            let btn_layout = text_system.prepare(btn.symbol, s.text_size, spec.font);
            let bty = spec.rect.y + (title_h - btn_layout.size.y) * 0.5;
            draw.push(DrawCmd::Text {
                rect: Rect::new(btn_x, bty, btn_layout.size.x, btn_layout.size.y),
                color: s.title_text,
                handle: btn_layout.handle,
            });
        }

        // Status bar.
        if spec.status_bar {
            let bar_y = spec.rect.y + spec.rect.h - status_h;
            draw.push(DrawCmd::StrokeLine {
                p0: Vec2::new(spec.rect.x, bar_y),
                p1: Vec2::new(spec.rect.x + spec.rect.w, bar_y),
                color: s.status_border,
                width: s.border_width,
            });
            let status_layout =
                text_system.prepare(spec.status_text.unwrap_or(""), s.text_size, spec.font);
            let sty = bar_y + (status_h - status_layout.size.y) * 0.5;
            draw.push(DrawCmd::Text {
                rect: Rect::new(
                    spec.rect.x + s.text_pad_x,
                    sty,
                    status_layout.size.x,
                    status_layout.size.y,
                ),
                color: s.status_text,
                handle: status_layout.handle,
            });
        }

        let content_top = spec.rect.y + title_h + s.content_pad_y;
        let content_bottom = spec.rect.y + spec.rect.h - status_h - s.content_pad_y;
        let content = Rect::new(
            spec.rect.x + s.content_pad_x,
            content_top,
            spec.rect.w - s.content_pad_x * 2.0,
            (content_bottom - content_top).max(0.0),
        );

        draw.push(DrawCmd::PushClip { rect: content });

        WindowResult {
            draw: DrawCommands(draw),
            content_bounds: content,
        }
    }

    // Low-level window end function.
    //
    // This is the raw implementation that takes all parameters explicitly.
    // High-level wrappers should use this internally.
    pub fn end_window() -> DrawCommands {
        DrawCommands(vec![DrawCmd::PopClip])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WindowButton {
    pub symbol: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowStyle {
    pub title_height: f32,
    pub button_size: f32,
    pub button_gap: f32,
    pub button_right_pad: f32,
    pub status_height: f32,
    pub content_pad_x: f32,
    pub content_pad_y: f32,
    pub text_pad_x: f32,
    pub text_size: f32,
    pub background: Color,
    pub border: Color,
    pub title_bg: Color,
    pub title_text: Color,
    pub status_text: Color,
    pub status_border: Color,
    pub border_width: f32,
}

pub struct WindowResult<
    'b,
    T: crate::text::TextSystem,
    LS: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
> {
    pub layout: LayoutInfo,
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowSpecBuilder<'a> {
    pub title: Option<&'a str>,
    pub buttons: Option<&'a [WindowButton]>,
    pub font: Option<FontId>,
    pub style: Option<WindowStyle>,
    pub status_bar: Option<bool>,
    pub status_text: Option<&'a str>,
    pub rect: Option<Rect>,
}

impl<'a> WindowSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn title(mut self, title: &'a str) -> Self {
        self.title = Some(title);
        self
    }
    pub fn buttons(mut self, buttons: &'a [WindowButton]) -> Self {
        self.buttons = Some(buttons);
        self
    }
    pub fn font(mut self, font: FontId) -> Self {
        self.font = Some(font);
        self
    }
    pub fn style(mut self, style: WindowStyle) -> Self {
        self.style = Some(style);
        self
    }
    pub fn status_bar(mut self, status_bar: bool) -> Self {
        self.status_bar = Some(status_bar);
        self
    }
    pub fn status_text(mut self, status_text: &'a str) -> Self {
        self.status_text = Some(status_text);
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
            self.style = Some(theme.window_style());
        }
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::WindowSpec<'a> {
        raw::WindowSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            title: self.title.expect("title not set — call .title()"),
            buttons: self.buttons.expect("buttons not set — call .buttons()"),
            font: self
                .font
                .expect("font not set — call .font() or defaults_from_theme()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
            status_bar: self.status_bar.unwrap_or(false),
            status_text: self.status_text,
        }
    }
}

// ── High-level widget functions ───────────────────────────────────────────────────

/// High-level window begin function using WidgetContext.
///
/// This function accepts layout parameters, a WindowSpecBuilder, and an inner layout,
/// and returns a WindowResult containing the layout info and child WidgetContext.
///
/// Note there is no low-level end_window - everything is handled by the on_finish callback of the child context, which calls raw::end_window internally.
pub fn begin_window<
    'a,
    'b,
    'c,
    T: crate::text::TextSystem,
    LS: crate::layout::LayoutState,
    L: crate::layout::Layout,
    CF: FnOnce(&mut FocusSystem) -> DrawCommands,
>(
    ctx: &'b mut WidgetContext<'a, T, LS, CF>,
    builder: WindowSpecBuilder<'c>,
    layout_params: LS::Params,
    inner_layout: L,
) -> WindowResult<'b, T, L::State, impl FnOnce(&mut FocusSystem) -> DrawCommands> {
    let layout_bounds = ctx.layout(layout_params);
    let bounds = builder.rect.unwrap_or(layout_bounds);

    let buttons = builder.buttons.unwrap_or(&[]);
    let spec = builder
        .rect(bounds)
        .defaults_from_theme(&ctx.theme)
        .buttons(buttons)
        .build();
    let raw::WindowResult {
        draw,
        content_bounds,
    } = raw::begin_window(spec, ctx.text_system);
    ctx.append_cmds(draw);

    let new_clip = Some(
        ctx.clip_rect
            .map_or(content_bounds, |pc| pc.intersect(&content_bounds)),
    );

    let on_finish = move |_: &mut FocusSystem| raw::end_window();

    let child_ctx = ctx.child_with_layout_and_on_finish_and_clip_rect(
        inner_layout.begin(content_bounds),
        on_finish,
        new_clip,
    );

    WindowResult {
        layout: LayoutInfo::new(bounds, content_bounds),
        ctx: child_ctx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = WindowSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.window_style()));
        assert_eq!(builder.font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.window_style();
        custom_style.text_size = 99.0;
        let builder = WindowSpecBuilder::new()
            .style(custom_style)
            .font(FontId(99));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_size, 99.0);
        assert_eq!(builder.font, Some(FontId(99)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 200.0, 150.0);
        let custom_rect = Rect::new(10.0, 20.0, 100.0, 80.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_sys,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let child = super::begin_window(
            &mut ctx,
            WindowSpecBuilder::new().title("T").rect(custom_rect),
            layout_rect,
            ManualLayout,
        );
        child.ctx.finish();
        assert!(cmds.iter().any(
            |cmd| matches!(cmd, crate::draw::DrawCmd::FillRect { rect, .. } if *rect == custom_rect)
        ));
    }
}
