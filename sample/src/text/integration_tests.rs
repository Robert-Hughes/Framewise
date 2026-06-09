#[cfg(test)]
mod integration_tests {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{Color, DrawCmd, DrawCommands, FontId, LineHeight, Rect, TextFlow, TextSystem};

    #[test]
    fn test_headless_text_rendering() {
        pollster::block_on(async {
            let width = 600;
            let height = 80;
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let font_id = FontId(1);
            let body_style = framewise::TextStyle::new(font_id, 15.0, 400, TextFlow::wrapped())
                .with_line_height(LineHeight::Relative(1.55));
            let body_rect = Rect::new(0.0, 0.0, width as f32, height as f32);
            let describe_layout = text_system.prepare(
                "Sharp corners, hairline borders, monospaced numerics. One accent — rust — reserved for focus, drag, and primary action. Every widget describes its state explicitly; nothing is hidden behind animation or chrome.",
                body_style,
                body_rect,
            );
            cmds.push(DrawCmd::Text {
                rect: body_rect,
                color: Color::from_srgb_u8(0, 0, 0, 255),
                handle: describe_layout.handle,
            });

            let Some(actual) = render_commands_to_rgba(width, height, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/text/golden_text.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}
