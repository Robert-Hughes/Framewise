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
    pub struct MenuSpec<'a> {
        /// Top-left origin; width is at least 200.
        pub rect: Rect,
        pub items: &'a [super::MenuItem<'a>],
        pub label_font: FontId,
        pub meta_font: FontId,
        pub style: super::MenuStyle,
    }

    /// Low-level menu widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn menu<'a, T: crate::text::TextSystem>(
        spec: MenuSpec<'a>,
        text_system: &mut T,
    ) -> MenuResult {
        let mut cmds = DrawCommands::new();
        let s = spec.style;

        let row_h = s.row_height;
        let sep_h = s.separator_height;
        let group_h = s.group_height;
        let pad_x = s.pad_x;

        let total_h: f32 = spec
            .items
            .iter()
            .map(|item| match item {
                MenuItem::Item { .. } => row_h,
                MenuItem::Separator => sep_h,
                MenuItem::Group(_) => group_h,
            })
            .sum::<f32>()
            + s.pad_y * 2.0;

        let w = spec.rect.w.max(s.min_width);
        let outer = Rect::new(spec.rect.x, spec.rect.y, w, total_h);

        cmds.push(DrawCmd::FillRect {
            rect: outer,
            color: s.background,
        });
        cmds.push(DrawCmd::StrokeRect {
            rect: outer,
            color: s.border,
            width: s.border_width,
        });

        let mut y = spec.rect.y + s.pad_y;

        for item in spec.items {
            match item {
                MenuItem::Separator => {
                    let sep_y = y + s.separator_y;
                    cmds.push(DrawCmd::StrokeLine {
                        p0: Vec2::new(outer.x, sep_y),
                        p1: Vec2::new(outer.x + w, sep_y),
                        color: s.separator,
                        width: s.border_width,
                    });
                    y += sep_h;
                }
                MenuItem::Group(label) => {
                    let layout = text_system.prepare(label, s.meta_size, spec.meta_font);
                    let ty = y + s.group_text_y;
                    cmds.push(DrawCmd::Text {
                        rect: Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                        color: s.muted,
                        handle: layout.handle,
                    });
                    y += group_h;
                }
                MenuItem::Item {
                    label,
                    shortcut,
                    selected,
                    disabled,
                } => {
                    let alpha = if *disabled { s.disabled_alpha } else { 1.0 };
                    let tint = |c: Color| Color::linear_rgba(c.r, c.g, c.b, c.a * alpha);

                    let row_rect = Rect::new(outer.x, y, w, row_h);

                    if *selected {
                        cmds.push(DrawCmd::FillRect {
                            rect: row_rect,
                            color: tint(s.selected_bg),
                        });
                    }

                    let text_color = if *selected {
                        tint(s.selected_text)
                    } else {
                        tint(s.text)
                    };
                    let layout = text_system.prepare(label, s.label_size, spec.label_font);
                    let ty = y + (row_h - layout.size.y) * 0.5;
                    cmds.push(DrawCmd::Text {
                        rect: Rect::new(outer.x + pad_x, ty, layout.size.x, layout.size.y),
                        color: text_color,
                        handle: layout.handle,
                    });

                    if let Some(sc) = shortcut {
                        let sc_color = if *selected {
                            Color::linear_rgba(
                                s.selected_text.r,
                                s.selected_text.g,
                                s.selected_text.b,
                                s.shortcut_selected_alpha * alpha,
                            )
                        } else {
                            tint(s.muted)
                        };
                        let sc_layout = text_system.prepare(sc, s.meta_size, spec.meta_font);
                        let sc_x = outer.x + w - pad_x - sc_layout.size.x;
                        let sc_ty = y + (row_h - sc_layout.size.y) * 0.5;
                        cmds.push(DrawCmd::Text {
                            rect: Rect::new(sc_x, sc_ty, sc_layout.size.x, sc_layout.size.y),
                            color: sc_color,
                            handle: sc_layout.handle,
                        });
                    }

                    y += row_h;
                }
            }
        }

        MenuResult {
            draw: cmds,
            outer,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuResult {
        pub draw: DrawCommands,
        pub outer: Rect,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MenuItem<'a> {
    Item {
        label: &'a str,
        shortcut: Option<&'a str>,
        selected: bool,
        disabled: bool,
    },
    Separator,
    Group(&'a str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuStyle {
    pub row_height: f32,
    pub separator_height: f32,
    pub group_height: f32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub group_text_y: f32,
    pub separator_y: f32,
    pub min_width: f32,
    pub label_size: f32,
    pub meta_size: f32,
    pub background: Color,
    pub border: Color,
    pub separator: Color,
    pub selected_bg: Color,
    pub selected_text: Color,
    pub text: Color,
    pub muted: Color,
    pub shortcut_selected_alpha: f32,
    pub disabled_alpha: f32,
    pub border_width: f32,
}

pub struct MenuResult {
    pub layout: LayoutInfo,
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level menu widget function using WidgetContext.
///
/// This function accepts a MenuSpec and calls the low-level raw::menu function.
pub fn menu<
    'a,
    T: crate::text::TextSystem,
    S: crate::layout::LayoutState,
    CF: FnOnce(&mut FocusSystem) -> Vec<DrawCmd>,
>(
    ctx: &mut WidgetContext<T, S, CF>,
    layout_params: S::Params,
    builder: MenuSpecBuilder<'a>,
) -> MenuResult {
    let layout_rect = ctx.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let builder = builder.rect(rect).defaults_from_theme(&ctx.theme);
    let spec = builder.build();
    let result = raw::menu(spec, ctx.text_system);
    ctx.append_cmds(result.draw.0);
    MenuResult {
        layout: LayoutInfo::tight(result.outer),
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MenuSpecBuilder<'a> {
    pub items: Option<&'a [MenuItem<'a>]>,
    pub label_font: Option<FontId>,
    pub meta_font: Option<FontId>,
    pub style: Option<MenuStyle>,
    pub rect: Option<Rect>,
}

impl<'a> MenuSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [MenuItem<'a>]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn label_font(mut self, font: FontId) -> Self {
        self.label_font = Some(font);
        self
    }
    pub fn meta_font(mut self, font: FontId) -> Self {
        self.meta_font = Some(font);
        self
    }
    pub fn style(mut self, style: MenuStyle) -> Self {
        self.style = Some(style);
        self
    }
}

impl<'a> MenuSpecBuilder<'a> {
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
            self.style = Some(theme.menu_style());
        }
        if self.label_font.is_none() {
            self.label_font = Some(theme.sans_font);
        }
        if self.meta_font.is_none() {
            self.meta_font = Some(theme.mono_font);
        }
        self
    }

    pub fn build(self) -> raw::MenuSpec<'a> {
        raw::MenuSpec {
            rect: self
                .rect
                .expect("rect not set — call .rect() or use the high-level API"),
            items: self.items.expect("items not set — call .items()"),
            label_font: self
                .label_font
                .expect("label_font must be specified or resolved from a theme"),
            meta_font: self
                .meta_font
                .expect("meta_font must be specified or resolved from a theme"),
            style: self.style.expect("MenuStyle is required"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_defaults_from_theme_fills_unset_fields() {
        let theme = crate::theme::Theme::framewise();
        let builder = MenuSpecBuilder::new();
        assert!(builder.style.is_none());
        assert!(builder.label_font.is_none());
        assert!(builder.meta_font.is_none());
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(theme.menu_style()));
        assert_eq!(builder.label_font, Some(theme.sans_font));
        assert_eq!(builder.meta_font, Some(theme.mono_font));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = theme.menu_style();
        custom_style.label_size = 99.0;
        let builder = MenuSpecBuilder::new()
            .style(custom_style)
            .label_font(FontId(99))
            .meta_font(FontId(98));
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().label_size, 99.0);
        assert_eq!(builder.label_font, Some(FontId(99)));
        assert_eq!(builder.meta_font, Some(FontId(98)));
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_sys = DummyTextSys;
        let mut focus = crate::focus::FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = vec![];
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
        let result = super::menu(
            &mut ctx,
            layout_rect,
            MenuSpecBuilder::new().items(&[]).rect(custom_rect),
        );
        // x and y come from the user-provided rect
        assert_eq!(result.layout.bounds.x, custom_rect.x);
        assert_eq!(result.layout.bounds.y, custom_rect.y);
    }
}
