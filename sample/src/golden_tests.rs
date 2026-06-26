#[cfg(feature = "page_spec")]
mod spec_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, DrawCommands, LayoutSpace, RowLayout, Theme,
        WidgetContext,
    };

    #[test]
    fn spec_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::spec_page::SpecWidgetsState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::framewise(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                RowLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1200.0),
                &mut cmds,
            );

            crate::spec_page::draw_spec_page_inner(&mut state, &mut ctx, false, 1200.0);

            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(rect.w as u32, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_spec_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

mod analytical_aa_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{BorderPlacement, Color, DrawCmd, DrawCommands, Rect, Vec2};

    #[test]
    fn test_rendering() {
        pollster::block_on(async {
            let scale = 2.0;
            let logical_width = 330.0;
            let logical_height = 360.0;
            let width = (logical_width * scale) as u32;
            let height = (logical_height * scale) as u32;
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut cmds = DrawCommands::with_physical_pixels_per_logical_pixel(scale);

            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, logical_width, logical_height),
                color: Color::from_srgb_u8(240, 240, 240, 255),
                z: 0,
            });

            // Columns exercise renderer alignment classes at 2x:
            // 0: integer logical coordinates, therefore physical-pixel aligned.
            // 1: half-logical coordinates, not integer-logical but still physical-pixel aligned.
            // 2: quarter-logical coordinates, not physical-pixel aligned and should use AA where applicable.
            let cols = [20.0, 125.5, 231.25];

            // Rows exercise each primitive lowered by the renderer:
            // FillRect, BorderRect, StrokeLine, FillCircle, StrokeCircle.
            let rows = [20.0, 85.0, 150.0, 215.0, 280.0];
            let colors = [
                Color::from_srgb_u8(44, 116, 179, 255),
                Color::from_srgb_u8(193, 86, 42, 255),
                Color::from_srgb_u8(69, 147, 81, 255),
                Color::from_srgb_u8(141, 78, 166, 255),
                Color::from_srgb_u8(31, 31, 35, 255),
            ];

            for &x in &cols {
                cmds.push(DrawCmd::FillRect {
                    rect: Rect::new(x, rows[0], 54.0, 34.0),
                    color: colors[0],
                    z: 1,
                });

                cmds.push(DrawCmd::BorderRect {
                    rect: Rect::new(x, rows[1], 54.0, 34.0),
                    color: colors[1],
                    width: 3.0,
                    placement: BorderPlacement::Inside,
                    z: 1,
                });

                cmds.push(DrawCmd::StrokeLine {
                    p0: Vec2::new(x, rows[2] + 4.0),
                    p1: Vec2::new(x + 54.0, rows[2] + 30.0),
                    color: colors[2],
                    width: 3.0,
                    z: 1,
                });

                cmds.push(DrawCmd::FillCircle {
                    center: Vec2::new(x + 27.0, rows[3] + 17.0),
                    radius: 17.0,
                    color: colors[3],
                    z: 1,
                });

                cmds.push(DrawCmd::StrokeCircle {
                    center: Vec2::new(x + 27.0, rows[4] + 17.0),
                    radius: 15.0,
                    color: colors[4],
                    width: 3.0,
                    z: 1,
                });
            }

            let Some(actual) = render_commands_to_rgba(width, height, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_rendering.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

#[cfg(feature = "page_button_demo")]
mod button_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn button_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::button_page::ButtonPageState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                ColumnLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "Button Demo",
                    false,
                    true,
                    ColumnLayout,
                );
                crate::button_page::draw_button_page_content(&mut outer.ctx, &mut state);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_button_page.png");
            assert_matches_png_golden(&actual, &golden_path, 1);
        });
    }
}

#[cfg(feature = "page_label_demo")]
mod label_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn label_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                ColumnLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "Label Demo",
                    false,
                    true,
                    ColumnLayout,
                );
                crate::label_page::draw_label_page_content(&mut outer.ctx);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_label_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

#[cfg(feature = "page_frame_demo")]
mod frame_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn frame_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::frame_demo::FrameDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                ColumnLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "Frame Demo",
                    false,
                    true,
                    ColumnLayout,
                );
                crate::frame_demo::draw_frame_page_content(&mut outer.ctx, &mut state, 1600.0);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_frame_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

#[cfg(feature = "page_layout_demo")]
mod layout_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn layout_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::layout_demo::LayoutDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                ColumnLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "Layout Demo",
                    false,
                    true,
                    ColumnLayout,
                );
                crate::layout_demo::draw_layout_page_content(&mut outer.ctx, &mut state, 1600.0);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_layout_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

#[cfg(feature = "page_scroll_demo")]
mod scroll_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, DrawCommands, LayoutSpace, RowLayout, Theme,
        WidgetContext,
    };

    #[test]
    fn scroll_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::scroll_demo::ScrollDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                framewise::layouts::ColumnLayout,
                LayoutSpace::unbounded_height(20.0, 20.0, 1560.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "Scroll Demo",
                    false,
                    true,
                    RowLayout,
                );
                crate::scroll_demo::draw_scroll_demo_content(&mut outer.ctx, &mut state, true);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, (rect.h + 40.0) as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_scroll_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}

#[cfg(feature = "page_text_edit")]
mod text_edit_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextBackend,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn text_edit_page_matches_golden() {
        pollster::block_on(async {
            let mut text_backend = SampleTextBackend::new();
            text_backend.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::text_edit_demo::TextEditDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut output = framewise::Output::default();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_backend,
                &mut focus_system,
                &input,
                &mut output,
                ColumnLayout,
                LayoutSpace::unbounded_height(0.0, 0.0, 1600.0),
                &mut cmds,
            );

            {
                let mut outer = crate::demo_page::begin_demo_page_no_scroll(
                    &mut ctx,
                    "TextEdit Demo",
                    false,
                    true,
                    ColumnLayout,
                );
                crate::text_edit_demo::draw_text_edit_demo_content(&mut outer.ctx, &mut state);
                outer.ctx.finish();
            }
            let rect = ctx.finish();

            focus_system.end_frame();

            let Some(actual) =
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_backend).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/golden_text_edit_page.png");
            assert_matches_png_golden(&actual, &golden_path, 0);
        });
    }
}
