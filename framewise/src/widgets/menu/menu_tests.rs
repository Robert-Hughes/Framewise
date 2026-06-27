use super::*;
use crate::focus::FocusSystem;

#[test]
fn test_menu_spec_theme_overwrites_style() {
    let theme = crate::theme::Theme::framewise();
    let spec = MenuSpec::new(&[]).theme(&theme);
    assert_eq!(spec.style, MenuStyle::from_theme(&theme));
}

#[test]
fn test_menu_spec_theme_preserves_items() {
    let theme = crate::theme::Theme::framewise();
    let items = [MenuItem::Separator];
    let spec = MenuSpec::new(&items).theme(&theme);
    assert_eq!(spec.items, &items);
    assert_eq!(spec.style, MenuStyle::from_theme(&theme));
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
    let result = super::menu(
        MenuSpec::new_from_theme(&[], &ctx.theme),
        custom_rect,
        &mut ctx,
    );
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
    let res = super::menu(
        MenuSpec::new_from_theme(&[], &ctx.theme),
        layout_rect,
        &mut ctx,
    );

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
