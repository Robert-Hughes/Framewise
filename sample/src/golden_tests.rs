#[cfg(feature = "page_spec")]
mod spec_page_golden {
    use crate::render_test_utils::{assert_matches_png_golden, render_commands_to_rgba};
    use framewise::{focus::FocusSystem, input::Input};

    #[test]
    fn spec_page_matches_golden() {
        pollster::block_on(async {
            let width = 1600;
            let height = 1620;
            let Some(actual) = render_commands_to_rgba(width, height, |text_system| {
                let mut focus_system = FocusSystem::new();
                let mut state = crate::spec_page::SpecPageState::default();
                let input = Input::default();

                focus_system.begin_frame();
                let cmds = crate::spec_page::draw_spec_page(
                    text_system,
                    &mut focus_system,
                    &mut state,
                    &input,
                    0.0,
                    width as f32,
                    height as f32,
                    false,
                );
                focus_system.end_frame();
                cmds
            })
            .await
            else {
                return;
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_spec_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}
