use crate::{
    draw::{DrawCmd, DrawCommands}, focus::FocusSystem, text::FontId, types::{Color, Rect, Vec2}, widget::{LayoutInfo, WidgetContext, WidgetScope}
};

pub mod raw {
    use super::*;

    /// Low-level window begin function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn begin_window<'a, T: crate::text::TextSystem>(spec: WindowSpec<'a>,
        text_system: &mut T) -> (Vec<DrawCmd>, WindowScope, Rect)
    {
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
            let status_layout = text_system.prepare(spec.status_text, s.text_size, spec.font);
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

        let scope = WindowScope { is_finished: false };
        (draw, scope, content)
    }

    // Low-level window end function.
    //
    // This is the raw implementation that takes all parameters explicitly.
    // High-level wrappers should use this internally.
    // pub fn end_window(scope: WindowScope) -> Vec<DrawCmd> {
    //     scope.finish()
    // }
}

pub struct WindowButton {
    pub symbol: &'static str,
}

pub struct WindowSpec<'a> {
    pub rect: Rect,
    pub title: &'a str,
    pub buttons: &'a [WindowButton],
    pub font: FontId,
    pub status_bar: bool,
    pub status_text: &'a str,
    pub style: WindowStyle,
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

pub struct WindowResult {
    pub draw: DrawCommands,
    pub layout: LayoutInfo,
}

pub struct WindowInfo {
    pub layout: LayoutInfo,
}

impl WindowInfo {
    pub fn content_rect(&self) -> Rect {
        self.layout.content_bounds
    }
}

impl WindowResult {
    pub fn into_parts(self) -> (DrawCommands, WindowInfo) {
        (
            self.draw,
            WindowInfo {
                layout: self.layout,
            },
        )
    }
}

pub struct WindowScope {
    pub is_finished: bool,
}

impl Drop for WindowScope {
    fn drop(&mut self) {
        if !self.is_finished && !std::thread::panicking() {
            panic!("WindowScope dropped without calling finish()! This leaks clip rects.");
        }
    }
}

impl WidgetScope for WindowScope {
    fn finish(mut self, _focus_sys: &mut FocusSystem) -> Vec<DrawCmd> {
        self.is_finished = true;
        vec![DrawCmd::PopClip]
    }
}

// ── High-level widget functions ───────────────────────────────────────────────────

/// High-level window begin function using WidgetContext.
///
/// This function accepts layout parameters, a WindowSpecBuilder, and an inner layout,
/// and returns a child WidgetContext and the window scope.
pub fn begin_window<'a, 'b, 'c, T: crate::text::TextSystem, LS: crate::layout::LayoutState, L: crate::layout::Layout, Scope: WidgetScope>(
    parent: &'b mut WidgetContext<'a, T, LS, Scope>,
    layout_params: LS::Params,
    builder: WindowSpecBuilder<'c>,
    inner_layout: L,
) -> WidgetContext<'b, T, L::State, WindowScope> {
    let bounds = parent.layout(layout_params);

    let mut resolved_builder = builder
        .with_rect(bounds)
        .with_theme(&parent.theme);

    if resolved_builder.status_bar.is_none() {
        resolved_builder.status_bar = Some(false);
    }
    if resolved_builder.status_text.is_none() {
        resolved_builder.status_text = Some("");
    }
    if resolved_builder.buttons.is_none() {
        resolved_builder.buttons = Some(&[]);
    }

    let spec = resolved_builder.build();
    let (pre_cmds, scope, content) = raw::begin_window(spec, parent.text_system);
    parent.append_cmds(pre_cmds);

    let new_clip = Some(parent.clip_rect.map_or(content, |pc| pc.intersect(&content)));

    let mut child = parent.child_with_layout(inner_layout.begin(content), scope);

    child.clip_rect = new_clip;

    child
}

/// High-level window end function using WidgetContext.
///
/// This function accepts finished child commands and completes the window on the parent context.
// pub fn end_window<T: crate::text::TextSystem, S: crate::layout::LayoutState>(
//     parent: &mut WidgetContext<T, S>,
//     cmds: Vec<crate::draw::DrawCmd>,
//     scope: WindowScope,
// ) {
//     parent.append_cmds(cmds);
//     let post_cmds = raw::end_window(scope);
//     parent.append_cmds(post_cmds);
// }

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
        Self {
            title: None,
            buttons: None,
            font: None,
            style: None,
            status_bar: None,
            status_text: None,
            rect: None,
        }
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
}

impl<'a> WindowSpecBuilder<'a> {
    pub fn with_rect(mut self, rect: Rect) -> Self {
        self.rect = Some(rect);
        self
    }

    pub fn with_theme(mut self, theme: &crate::theme::Theme) -> Self {
        self.style = Some(theme.window_style());
        if self.font.is_none() {
            self.font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> WindowSpec<'a> {
        WindowSpec {
            rect: self.rect.unwrap_or_default(),
            title: self.title.unwrap(),
            buttons: self.buttons.unwrap(),
            font: self.font.expect("font must be specified or resolved from a theme"),
            style: self.style.expect("WindowStyle is required"),
            status_bar: self.status_bar.unwrap(),
            status_text: self.status_text.unwrap(),
        }
    }
}
