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
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new(1.0);
    let custom_rect = Rect::new(10.0, 20.0, 100.0, 80.0);
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
    let child = super::begin_window(
        &mut ctx,
        WindowSpecBuilder::new().title("T"),
        custom_rect,
        ManualLayout,
    );
    child.ctx.finish();
    assert!(cmds.iter().any(
        |cmd| matches!(cmd, crate::draw::DrawCmd::FillRect {  rect, .. } if *rect == custom_rect)
    ));
}
