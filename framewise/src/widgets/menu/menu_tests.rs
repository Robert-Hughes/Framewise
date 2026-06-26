use super::*;
use crate::focus::FocusSystem;

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
    custom_style.label_style.size = 99.0;
    let builder = MenuSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().label_style.size, 99.0);
}

#[test]
fn test_user_rect_not_overridden() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let custom_rect = Rect::new(10.0, 20.0, 50.0, 30.0);
    let mut output = crate::Output::default();
    let mut ctx = crate::widget::WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
        &mut cmds,
    );
    let result = super::menu(&mut ctx, MenuSpecBuilder::new().items(&[]), custom_rect);
    // x and y come from the user-provided rect
    assert_eq!(result.layout.bounds.x, custom_rect.x);
    assert_eq!(result.layout.bounds.y, custom_rect.y);
}

#[test]
fn test_menu_bounds_and_content_bounds() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let layout_rect = Rect::new(0.0, 0.0, 100.0, 40.0);
    let mut output = crate::Output::default();
    let mut ctx = crate::widget::WidgetContext::root(
        crate::theme::Theme::framewise(),
        &mut text_backend,
        &mut focus,
        &input,
        &mut output,
        ManualLayout,
        Rect::new(0.0, 0.0, 800.0, 600.0),
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
        layout_rect.x + style.border.map_or(0.0, |b| b.width) + style.pad_x,
        layout_rect.y + style.border.map_or(0.0, |b| b.width) + style.pad_y,
        expected_w - (style.border.map_or(0.0, |b| b.width) + style.pad_x) * 2.0,
        expected_h - (style.border.map_or(0.0, |b| b.width) + style.pad_y) * 2.0,
    );
    assert_eq!(res.layout.content_bounds, expected_content);
}
