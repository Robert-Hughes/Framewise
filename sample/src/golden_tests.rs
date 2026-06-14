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
                LayoutSpace::unbounded_height(0.0, 0.0, 1200.0),
                &mut cmds,
            );

            crate::spec_page::draw_spec_page_inner(&mut state, &mut ctx, false, 1200.0);

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

mod analytical_aa_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{Color, DrawCmd, DrawCommands, Rect, Vec2};

    #[test]
    fn test_analytical_aa_rendering() {
        pollster::block_on(async {
            let width = 400;
            let height = 400;
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut cmds = DrawCommands::new();

            // Background fill
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(0.0, 0.0, width as f32, height as f32),
                color: Color::from_srgb_u8(240, 240, 240, 255),
                z: 0,
                anti_alias: false,
            });

            // 1. Lines with and without AA
            // Non-AA lines
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(20.0, 20.0),
                p1: Vec2::new(180.0, 50.0),
                color: Color::from_srgb_u8(0, 0, 0, 255),
                width: 1.0,
                z: 1,
                anti_alias: false,
            });
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(20.0, 40.0),
                p1: Vec2::new(180.0, 100.0),
                color: Color::from_srgb_u8(0, 0, 0, 255),
                width: 3.0,
                z: 1,
                anti_alias: false,
            });

            // AA lines
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(220.0, 20.0),
                p1: Vec2::new(380.0, 50.0),
                color: Color::from_srgb_u8(0, 0, 0, 255),
                width: 1.0,
                z: 1,
                anti_alias: true,
            });
            cmds.push(DrawCmd::StrokeLine {
                p0: Vec2::new(220.0, 40.0),
                p1: Vec2::new(380.0, 100.0),
                color: Color::from_srgb_u8(0, 0, 0, 255),
                width: 3.0,
                z: 1,
                anti_alias: true,
            });

            // 2. Circles with and without AA
            // Non-AA circles (filled and stroked)
            cmds.push(DrawCmd::FillCircle {
                center: Vec2::new(60.0, 200.0),
                radius: 30.0,
                color: Color::from_srgb_u8(200, 50, 50, 255),
                z: 1,
                anti_alias: false,
            });
            cmds.push(DrawCmd::StrokeCircle {
                center: Vec2::new(140.0, 200.0),
                radius: 25.0,
                color: Color::from_srgb_u8(50, 50, 200, 255),
                width: 4.0,
                z: 1,
                anti_alias: false,
            });

            // AA circles (filled and stroked)
            cmds.push(DrawCmd::FillCircle {
                center: Vec2::new(260.0, 200.0),
                radius: 30.0,
                color: Color::from_srgb_u8(200, 50, 50, 255),
                z: 1,
                anti_alias: true,
            });
            cmds.push(DrawCmd::StrokeCircle {
                center: Vec2::new(340.0, 200.0),
                radius: 25.0,
                color: Color::from_srgb_u8(50, 50, 200, 255),
                width: 4.0,
                z: 1,
                anti_alias: true,
            });

            // 3. Rectangles with and without AA
            // Non-AA Rects
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(20.5, 280.5, 60.0, 40.0),
                color: Color::from_srgb_u8(50, 150, 50, 255),
                z: 1,
                anti_alias: false,
            });
            cmds.push(DrawCmd::StrokeRect {
                rect: Rect::new(100.5, 280.5, 60.0, 40.0),
                color: Color::from_srgb_u8(150, 150, 50, 255),
                width: 3.0,
                z: 1,
                anti_alias: false,
            });

            // AA Rects
            cmds.push(DrawCmd::FillRect {
                rect: Rect::new(220.5, 280.5, 60.0, 40.0),
                color: Color::from_srgb_u8(50, 150, 50, 255),
                z: 1,
                anti_alias: true,
            });
            cmds.push(DrawCmd::StrokeRect {
                rect: Rect::new(300.5, 280.5, 60.0, 40.0),
                color: Color::from_srgb_u8(150, 150, 50, 255),
                width: 3.0,
                z: 1,
                anti_alias: true,
            });

            let Some(actual) = render_commands_to_rgba(width, height, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/golden_analytical_aa.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_button_demo")]
mod button_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn button_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::button_page::ButtonPageState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_button_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_label_demo")]
mod label_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn label_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_label_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_frame_demo")]
mod frame_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn frame_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::frame_demo::FrameDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_frame_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_layout_demo")]
mod layout_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn layout_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::layout_demo::LayoutDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_layout_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_scroll_demo")]
mod scroll_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, DrawCommands, LayoutSpace, RowLayout, Theme,
        WidgetContext,
    };

    #[test]
    fn scroll_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::scroll_demo::ScrollDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, (rect.h + 40.0) as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/golden_scroll_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}

#[cfg(feature = "page_text_edit")]
mod text_edit_page_golden {
    use crate::{
        render_test_utils::{assert_matches_png_golden, render_commands_to_rgba},
        text::SampleTextSystem,
    };
    use framewise::{
        focus::FocusSystem, input::Input, ColumnLayout, DrawCommands, LayoutSpace, Theme,
        WidgetContext,
    };

    #[test]
    fn text_edit_page_matches_golden() {
        pollster::block_on(async {
            let mut text_system = SampleTextSystem::new();
            text_system.begin_frame();

            let mut focus_system = FocusSystem::new();
            let mut state = crate::text_edit_demo::TextEditDemoState::default();
            let input = Input::default();

            focus_system.begin_frame();

            let mut cmds = DrawCommands::new();
            let mut ctx = WidgetContext::root(
                Theme::default(),
                &mut text_system,
                &mut focus_system,
                &input,
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
                render_commands_to_rgba(1600, rect.h as u32, cmds, text_system).await
            else {
                panic!("Failed to render commands to RGBA");
            };

            let golden_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/golden_text_edit_page.png");
            assert_matches_png_golden(&actual, &golden_path);
        });
    }
}
