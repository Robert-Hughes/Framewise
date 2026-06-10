use crate::{
    draw::{DrawCmd, DrawCommands},
    focus::FocusSystem,
    layout::{Layout, LayoutState},
    types::{Color, Layer, Rect, Vec2},
    widget::{LayoutInfo, WidgetContext},
    TextSystem,
};

pub mod raw {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct WindowSpec<'a> {
        pub rect: Rect,
        pub title: &'a str,
        pub buttons: &'a [super::WindowButton],
        pub status_bar: bool,
        pub status_text: Option<&'a str>,
        pub style: super::WindowStyle,
        pub layer: Layer,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct WindowCalcIntrinsicSizeSpec {
        pub status_bar: bool,
        pub style: super::WindowStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct WindowResult {
        pub content_bounds: Rect,
    }

    pub fn calc_window_intrinsic_size(
        spec: &WindowCalcIntrinsicSizeSpec,
    ) -> crate::layout::IntrinsicSize {
        let s = spec.style;
        let status_h = if spec.status_bar {
            s.status_height
        } else {
            0.0
        };
        crate::layout::IntrinsicSize::preferred(Vec2::new(
            240.0,
            s.title_height + status_h + s.content_pad_y * 2.0 + 80.0,
        ))
    }

    /// Low-level window begin function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn begin_window<'a, T: TextSystem>(
        spec: WindowSpec<'a>,
        text_system: &mut T,
        cmds: &mut DrawCommands,
    ) -> WindowResult {
        let s = spec.style;

        let title_h = s.title_height;
        let btn_size = s.button_size;
        let status_h = if spec.status_bar {
            s.status_height
        } else {
            0.0
        };

        // Body.
        cmds.push(DrawCmd::FillRect {
            rect: spec.rect,
            color: s.background,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: spec.rect,
            color: s.border,
            width: s.border_width,
        });

        // Title bar.
        let title_rect = Rect::new(spec.rect.x, spec.rect.y, spec.rect.w, title_h);
        cmds.push(DrawCmd::FillRect {
            rect: title_rect,
            color: s.title_bg,
        });

        let title_metrics =
            text_system.measure(spec.title, s.text_style, crate::text::TextBounds::UNBOUNDED);
        let tty = spec.rect.y + (title_h - title_metrics.logical_size.y) * 0.5;
        let title_text_rect = Rect::new(
            spec.rect.x + s.text_pad_x,
            tty,
            title_metrics.logical_size.x,
            title_metrics.logical_size.y,
        );
        let title_layout = text_system.prepare(spec.title, s.text_style, title_text_rect);
        cmds.push(DrawCmd::Text {
            rect: title_text_rect,
            color: s.title_text,
            handle: title_layout.handle,
        });

        // Window buttons (right side).
        let mut btn_x = spec.rect.x + spec.rect.w - s.button_right_pad;
        for btn in spec.buttons.iter().rev() {
            btn_x -= btn_size + s.button_gap;
            let btn_metrics =
                text_system.measure(btn.symbol, s.text_style, crate::text::TextBounds::UNBOUNDED);
            let bty = spec.rect.y + (title_h - btn_metrics.logical_size.y) * 0.5;
            let btn_rect = Rect::new(
                btn_x,
                bty,
                btn_metrics.logical_size.x,
                btn_metrics.logical_size.y,
            );
            let btn_layout = text_system.prepare(btn.symbol, s.text_style, btn_rect);
            cmds.push(DrawCmd::Text {
                rect: btn_rect,
                color: s.title_text,
                handle: btn_layout.handle,
            });
        }

        // Status bar.
        if spec.status_bar {
            let bar_y = spec.rect.y + spec.rect.h - status_h;
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(spec.rect.x, bar_y),
                p1: Vec2::new(spec.rect.x + spec.rect.w, bar_y),
                color: s.status_border,
                width: s.border_width,
            });
            let status_text = spec.status_text.unwrap_or("");
            let status_metrics = text_system.measure(
                status_text,
                s.text_style,
                crate::text::TextBounds::UNBOUNDED,
            );
            let sty = bar_y + (status_h - status_metrics.logical_size.y) * 0.5;
            let status_rect = Rect::new(
                spec.rect.x + s.text_pad_x,
                sty,
                status_metrics.logical_size.x,
                status_metrics.logical_size.y,
            );
            let status_layout = text_system.prepare(status_text, s.text_style, status_rect);
            cmds.push(DrawCmd::Text {
                rect: status_rect,
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

        cmds.push(DrawCmd::PushClip { rect: content });

        WindowResult {
            content_bounds: content,
        }
    }

    /// Low-level window end function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn end_window(cmds: &mut DrawCommands) {
        cmds.push(DrawCmd::PopClip);
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

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
    pub text_style: crate::text::TextStyle,
    pub background: Color,
    pub border: Color,
    pub title_bg: Color,
    pub title_text: Color,
    pub status_text: Color,
    pub status_border: Color,
    pub border_width: f32,
}

impl WindowStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            title_height: theme.h_md,
            button_size: 16.0,
            button_gap: 2.0,
            button_right_pad: 4.0,
            status_height: 22.0,
            content_pad_x: 16.0,
            content_pad_y: 16.0,
            text_pad_x: 10.0,
            text_style: crate::text::TextStyle::new(
                theme.mono_font,
                theme.text_sm,
                theme.sans_weight_regular,
                crate::text::TextFlow::single_line(),
            ),
            background: theme.paper_elev,
            border: theme.ink,
            title_bg: theme.ink,
            title_text: theme.paper,
            status_text: theme.muted,
            status_border: theme.line,
            border_width: theme.border,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

pub struct WindowResult<'b, T: TextSystem, LS: LayoutState, CF> {
    pub layout: LayoutInfo,
    pub ctx: WidgetContext<'b, T, LS, CF>,
}

// ── Spec ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec<'a> {
    pub title: &'a str,
    pub buttons: &'a [WindowButton],
    pub status_bar: bool,
    pub status_text: Option<&'a str>,
    pub style: WindowStyle,
}

// ── Spec Builder ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowSpecBuilder<'a> {
    pub title: Option<&'a str>,
    pub buttons: Option<&'a [WindowButton]>,
    pub status_bar: Option<bool>,
    pub status_text: Option<&'a str>,
    pub style: Option<WindowStyle>,
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

    /// Fills unset fields from `theme`. Called automatically by high-level
    /// context functions.
    pub fn defaults_from_theme(mut self, theme: &crate::theme::Theme) -> Self {
        if self.style.is_none() {
            self.style = Some(WindowStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> WindowSpec<'a> {
        WindowSpec {
            title: self.title.expect("title not set — call .title()"),
            buttons: self.buttons.expect("buttons not set — call .buttons()"),
            status_bar: self.status_bar.unwrap_or(false),
            status_text: self.status_text,
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
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
pub fn begin_window<'a, 'b, 'c, T: TextSystem, S: LayoutState, L: Layout, CF>(
    ctx: &'b mut WidgetContext<'a, T, S, CF>,
    builder: WindowSpecBuilder<'c>,
    layout_params: S::Params,
    inner_layout: L,
) -> WindowResult<'b, T, L::State, impl FnOnce(&mut FocusSystem, &mut T, &mut DrawCommands, Rect)> {
    let buttons = builder.buttons.unwrap_or(&[]);
    let spec = builder
        .defaults_from_theme(&ctx.theme)
        .buttons(buttons)
        .build();
    let calc_spec = raw::WindowCalcIntrinsicSizeSpec {
        status_bar: spec.status_bar,
        style: spec.style,
    };
    let intrinsic = raw::calc_window_intrinsic_size(&calc_spec);
    let bounds = ctx.layout(layout_params, intrinsic);
    let raw_spec = raw::WindowSpec {
        rect: bounds,
        title: spec.title,
        buttons: spec.buttons,
        status_bar: spec.status_bar,
        status_text: spec.status_text,
        style: spec.style,
        layer: ctx.layer,
    };
    let raw::WindowResult { content_bounds } =
        raw::begin_window(raw_spec, ctx.text_system, ctx.cmds);

    let new_clip = Some(
        ctx.clip_rect
            .map_or(content_bounds, |pc| pc.intersect(&content_bounds)),
    );

    // The window's cleanup doesn't depend on its content extent.
    let on_finish = move |_: &mut FocusSystem, _: &mut T, cmds: &mut DrawCommands, _: Rect| {
        raw::end_window(cmds);
    };

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
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(WindowStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = WindowStyle::from_theme(&theme);
        custom_style.text_style.size = 99.0;
        let builder = WindowSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().text_style.size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layouts::ManualLayout;
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let custom_rect = Rect::new(10.0, 20.0, 100.0, 80.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout,
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &mut cmds,
        );
        let child = super::begin_window(
            &mut ctx,
            WindowSpecBuilder::new().title("T"),
            custom_rect,
            ManualLayout,
        );
        child.ctx.finish();
        assert!(cmds.iter().any(
            |cmd| matches!(cmd, crate::draw::DrawCmd::FillRect { rect, .. } if *rect == custom_rect)
        ));
    }
}
