#[cfg(feature = "page_spec")]
mod spec_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, DrawCommands, LayoutSpace, RowLayout, Theme,
        WidgetContext,
    };

    #[test]
    fn spec_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::spec_page::SpecWidgetsState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::framewise(),
                &mut text_system,
                &mut focus_system,
                &input,
                RowLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            crate::spec_page::draw_spec_page_inner(&mut state, &mut ctx, false, 1600.0);

            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(rect.w as u32, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_spec_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}
