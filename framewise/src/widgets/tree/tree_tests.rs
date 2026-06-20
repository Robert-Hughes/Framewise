use super::*;
use crate::focus::FocusSystem;

#[test]
fn test_builder_defaults_from_theme_fills_unset_fields() {
    let theme = crate::theme::Theme::framewise();
    let builder = TreeSpecBuilder::new();
    assert!(builder.style.is_none());
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style, Some(TreeStyle::from_theme(&theme)));
}

#[test]
fn test_builder_defaults_from_theme_preserves_explicit_fields() {
    let theme = crate::theme::Theme::framewise();
    let mut custom_style = TreeStyle::from_theme(&theme);
    custom_style.text_style.size = 99.0;
    let builder = TreeSpecBuilder::new().style(custom_style);
    let builder = builder.defaults_from_theme(&theme);
    assert_eq!(builder.style.unwrap().text_style.size, 99.0);
}

#[test]
fn test_explicit_placement_via_manual_layout() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
    let placement = Rect::new(10.0, 20.0, 50.0, 30.0);
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
    let result = super::tree(&mut ctx, TreeSpecBuilder::new().items(&[]), placement);
    // With zero items, resolved bounds x/y should match the placement origin.
    assert_eq!(result.layout.bounds.x, placement.x);
    assert_eq!(result.layout.bounds.y, placement.y);
}

#[test]
fn test_tree_bounds_and_content_bounds() {
    use crate::layouts::ManualLayout;
    use crate::test_utils::TestTextBackend;
    let mut text_backend = TestTextBackend::default();
    let mut focus = FocusSystem::new();
    let input = crate::Input::default();
    let mut cmds = crate::draw::DrawCommands::new();
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
    let res = super::tree(&mut ctx, TreeSpecBuilder::new().items(&[]), layout_rect);

    let style = TreeStyle::from_theme(&ctx.theme);
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

#[test]
fn test_size_tree_empty() {
    let spec = raw::TreePreLayoutSpec {
        items: &[],
        style: TreeStyle::from_theme(&crate::theme::Theme::framewise()),
    };
    let style = spec.style;
    let size_request = raw::pre_layout_tree(&spec, SizeOffer::UNBOUNDED).size_request;
    assert_eq!(
        size_request.preferred,
        Some(Vec2::new(style.min_width, style.pad_y * 2.0))
    );
}

#[test]
fn test_size_tree_with_items() {
    let items = [
        TreeRow {
            indent: 0,
            caret: None,
            label: "a",
            meta: None,
            selected: false,
        },
        TreeRow {
            indent: 0,
            caret: None,
            label: "b",
            meta: None,
            selected: false,
        },
    ];
    let spec = raw::TreePreLayoutSpec {
        items: &items,
        style: TreeStyle::from_theme(&crate::theme::Theme::framewise()),
    };
    let style = spec.style;
    let size_request = raw::pre_layout_tree(&spec, SizeOffer::UNBOUNDED).size_request;
    let expected_h = 2.0 * style.row_height + style.pad_y * 2.0;
    assert_eq!(
        size_request.preferred,
        Some(Vec2::new(style.min_width, expected_h))
    );
}
