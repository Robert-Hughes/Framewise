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
    pub struct MenuSpec<'a> {
        /// Top-left origin; width is at least 200.
        pub rect: Rect,
        pub items: &'a [super::MenuItem<'a>],
        pub style: super::MenuStyle,
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct MenuResult {
        pub draw: DrawCommands,
        pub bounds: Rect,
        pub content_bounds: Rect,
    }

    /// Low-level menu widget function.
    ///
    /// This is the raw implementation that takes all parameters explicitly.
    /// High-level wrappers should use this internally.
    pub fn menu<'a, T: TextSystem>(spec: MenuSpec<'a>, text_system: &mut T) -> MenuResult {
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
                    let layout = text_system.prepare(label, s.meta_size, spec.style.meta_font);
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
                    let layout = text_system.prepare(label, s.label_size, spec.style.label_font);
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
                        let sc_layout = text_system.prepare(sc, s.meta_size, spec.style.meta_font);
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

        let content_bounds = Rect::new(
            outer.x + s.border_width + s.pad_x,
            outer.y + s.border_width + s.pad_y,
            outer.w - (s.border_width + s.pad_x) * 2.0,
            outer.h - (s.border_width + s.pad_y) * 2.0,
        );

        MenuResult {
            draw: cmds,
            bounds: outer,
            content_bounds,
        }
    }
}

// ── Style ─────────────────────────────────────────────────────────────────────

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
    pub label_font: FontId,
    pub meta_font: FontId,
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

impl MenuStyle {
    pub fn from_theme(theme: &crate::theme::Theme) -> Self {
        Self {
            row_height: theme.row_height,
            separator_height: 9.0,
            group_height: 22.0,
            pad_x: 12.0,
            pad_y: 4.0,
            group_text_y: 8.0,
            separator_y: 4.0,
            min_width: 200.0,
            label_size: theme.text_md,
            meta_size: theme.text_sm,
            label_font: theme.sans_font,
            meta_font: theme.mono_font,
            background: theme.paper_elev,
            border: theme.ink,
            separator: theme.line,
            selected_bg: theme.ink,
            selected_text: theme.paper,
            text: theme.ink,
            muted: theme.muted,
            shortcut_selected_alpha: 0.6,
            disabled_alpha: 0.4,
            border_width: theme.border,
        }
    }
}

// ── Result ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub struct MenuResult {
    pub layout: LayoutInfo,
}

// ── Spec Builder ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MenuSpecBuilder<'a> {
    pub rect: Option<Rect>,
    pub items: Option<&'a [MenuItem<'a>]>,
    pub style: Option<MenuStyle>,
}

impl<'a> MenuSpecBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn items(mut self, items: &'a [MenuItem<'a>]) -> Self {
        self.items = Some(items);
        self
    }
    pub fn style(mut self, style: MenuStyle) -> Self {
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
            self.style = Some(MenuStyle::from_theme(theme));
        }
        self
    }

    pub fn build(self) -> raw::MenuSpec<'a> {
        raw::MenuSpec {
            rect: self.rect.expect("rect not set — call .rect()"),
            items: self.items.expect("items not set — call .items()"),
            style: self
                .style
                .expect("style not set — call .style() or defaults_from_theme()"),
        }
    }
}

// ── High-level widget function ───────────────────────────────────────────────────

/// High-level menu widget function using WidgetContext.
///
/// This function accepts a MenuSpecBuilder and calls the low-level raw::menu function.
pub fn menu<'a, T: TextSystem, S: LayoutState, CF: FnOnce(&mut FocusSystem) -> DrawCommands>(
    ctx: &mut WidgetContext<T, S, CF>,
    builder: MenuSpecBuilder<'a>,
    layout_params: S::Params,
) -> MenuResult {
    let layout_rect = ctx.layout_state.layout(layout_params);
    let rect = builder.rect.unwrap_or(layout_rect);
    let spec = builder.rect(rect).defaults_from_theme(&ctx.theme).build();
    let result = raw::menu(spec, ctx.text_system);
    ctx.append_cmds(result.draw);
    MenuResult {
        layout: LayoutInfo::new(result.bounds, result.content_bounds),
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
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style, Some(MenuStyle::from_theme(&theme)));
    }

    #[test]
    fn test_builder_defaults_from_theme_preserves_explicit_fields() {
        let theme = crate::theme::Theme::framewise();
        let mut custom_style = MenuStyle::from_theme(&theme);
        custom_style.label_size = 99.0;
        let builder = MenuSpecBuilder::new().style(custom_style);
        let builder = builder.defaults_from_theme(&theme);
        assert_eq!(builder.style.unwrap().label_size, 99.0);
    }

    #[test]
    fn test_user_rect_not_overridden() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let result = super::menu(
            &mut ctx,
            MenuSpecBuilder::new().items(&[]).rect(custom_rect),
            layout_rect,
        );
        // x and y come from the user-provided rect
        assert_eq!(result.layout.bounds.x, custom_rect.x);
        assert_eq!(result.layout.bounds.y, custom_rect.y);
    }

    #[test]
    fn test_menu_bounds_and_content_bounds() {
        use crate::layout::{Layout, ManualLayout};
        use crate::test_utils::DummyTextSys;
        let mut text_system = DummyTextSys;
        let mut focus = FocusSystem::new();
        let input = crate::Input::default();
        let mut cmds = crate::draw::DrawCommands::new();
        let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
        let mut ctx = crate::widget::WidgetContext::root(
            crate::theme::Theme::framewise(),
            &mut text_system,
            &mut focus,
            &input,
            ManualLayout.begin(Rect::new(0.0, 0.0, 800.0, 600.0)),
            &mut cmds,
        );
        let res = super::menu(&mut ctx, MenuSpecBuilder::new().items(&[]), layout_rect);

        let style = MenuStyle::from_theme(&ctx.theme);
        let expected_h = style.pad_y * 2.0;
        let expected_w = layout_rect.w.max(style.min_width);
        assert_eq!(
            res.layout.bounds,
            Rect::new(layout_rect.x, layout_rect.y, expected_w, expected_h)
        );

        let expected_content = Rect::new(
            layout_rect.x + style.border_width + style.pad_x,
            layout_rect.y + style.border_width + style.pad_y,
            expected_w - (style.border_width + style.pad_x) * 2.0,
            expected_h - (style.border_width + style.pad_y) * 2.0,
        );
        assert_eq!(res.layout.content_bounds, expected_content);
    }
}
