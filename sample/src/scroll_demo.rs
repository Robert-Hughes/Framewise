use crate::text::SampleTextBackend;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::Align,
    theme::Theme,
    types::{Color, Rect},
    widget::WidgetContext,
    widgets::{
        button::button,
        scroll_area::{begin_scroll_area, ScrollAreaSpecBuilder},
        slider::{slider, Orientation as SliderOrientation, SliderSpec, SliderState, SliderValue},
        ButtonSpec,
    },
    ColumnLayoutParams, RowLayoutParams,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct SampleButton {
    pub state: framewise::widgets::button::ButtonState,
    pub clicks: u32,
}

pub struct NestedRowState {
    pub inner_scroll: framewise::widgets::scroll_area::ScrollState,
    pub horiz_scroll: framewise::widgets::scroll_area::ScrollState,
    pub both_scroll: framewise::widgets::scroll_area::ScrollState,
    pub btn1: SampleButton,
    pub inner_btns: [SampleButton; 6],
    pub horiz_btns: [SampleButton; 10],
    pub both_btns: [SampleButton; 48],
    pub slider_state: SliderState,
    pub horiz_slider_state: SliderState,
}

impl Default for NestedRowState {
    fn default() -> Self {
        Self {
            inner_scroll: Default::default(),
            horiz_scroll: Default::default(),
            both_scroll: Default::default(),
            btn1: Default::default(),
            inner_btns: std::array::from_fn(|_| SampleButton::default()),
            horiz_btns: std::array::from_fn(|_| SampleButton::default()),
            both_btns: std::array::from_fn(|_| SampleButton::default()),
            slider_state: SliderState {
                value: SliderValue::Single(50.0),
                ..Default::default()
            },
            horiz_slider_state: SliderState {
                value: SliderValue::Single(50.0),
                ..Default::default()
            },
        }
    }
}

// ── Page state ────────────────────────────────────────────────────────────────

pub struct ScrollDemoState {
    #[allow(dead_code)]
    pub page: crate::demo_page::DemoPageState,
    pub sidebar_scroll: framewise::widgets::scroll_area::ScrollState,
    pub main_scroll: framewise::widgets::scroll_area::ScrollState,
    pub nested_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    pub nested_rows: [NestedRowState; 3],
    pub sidebar_btns: [SampleButton; 20],
    pub main_btns: [SampleButton; 30],
    pub grid_btns: [SampleButton; 16],
    pub top_btn1: SampleButton,
    pub top_btn2: SampleButton,
    pub standalone_slider_state: SliderState,
    pub double_horiz_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    pub double_horiz_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    pub double_horiz_btns: [SampleButton; 20],
    pub right_panel_scroll: framewise::widgets::scroll_area::ScrollState,
    pub nested_2d_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    pub nested_2d_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    pub nested_2d_inner_btns: [SampleButton; 20],
    pub nested_2d_outer_btns: [SampleButton; 6],
    pub triple_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    pub triple_middle_scroll: framewise::widgets::scroll_area::ScrollState,
    pub triple_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    pub triple_inner_btns: [SampleButton; 12],
    pub triple_inner_slider_state: SliderState,
    pub triple_innermost_scroll: framewise::widgets::scroll_area::ScrollState,
    pub triple_innermost_btns: [SampleButton; 5],
    pub atmost_scroll: framewise::widgets::scroll_area::ScrollState,
    pub atmost_btns: [SampleButton; 3],
}

impl Default for ScrollDemoState {
    fn default() -> Self {
        Self {
            page: Default::default(),
            sidebar_scroll: Default::default(),
            main_scroll: Default::default(),
            nested_outer_scroll: Default::default(),
            nested_rows: std::array::from_fn(|_| NestedRowState::default()),
            sidebar_btns: std::array::from_fn(|_| SampleButton::default()),
            main_btns: std::array::from_fn(|_| SampleButton::default()),
            grid_btns: std::array::from_fn(|_| SampleButton::default()),
            top_btn1: SampleButton::default(),
            top_btn2: SampleButton::default(),
            standalone_slider_state: SliderState {
                value: SliderValue::Single(50.0),
                ..Default::default()
            },
            double_horiz_outer_scroll: Default::default(),
            double_horiz_inner_scroll: Default::default(),
            double_horiz_btns: std::array::from_fn(|_| SampleButton::default()),
            right_panel_scroll: Default::default(),
            nested_2d_outer_scroll: Default::default(),
            nested_2d_inner_scroll: Default::default(),
            nested_2d_inner_btns: std::array::from_fn(|_| SampleButton::default()),
            nested_2d_outer_btns: std::array::from_fn(|_| SampleButton::default()),
            triple_outer_scroll: Default::default(),
            triple_middle_scroll: Default::default(),
            triple_inner_scroll: Default::default(),
            triple_inner_btns: std::array::from_fn(|_| SampleButton::default()),
            triple_inner_slider_state: SliderState {
                value: SliderValue::Single(50.0),
                ..Default::default()
            },
            triple_innermost_scroll: Default::default(),
            triple_innermost_btns: std::array::from_fn(|_| SampleButton::default()),
            atmost_scroll: Default::default(),
            atmost_btns: std::array::from_fn(|_| SampleButton::default()),
        }
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn draw_scroll_demo(
    state: &mut ScrollDemoState,
    clipboard: &mut Option<arboard::Clipboard>,
    focus_system: &mut FocusSystem,
    input: &Input,
    output: &mut framewise::Output,
    time: f64,
    win_size: (f32, f32),
    physical_pixels_per_logical_pixel: f32,
    text_backend: &mut SampleTextBackend,
    debug_layout: bool,
) -> framewise::DrawCommands {
    // Clipboard is unused now that the text-edit field has been removed from this
    // (intentionally minimal) scroll demo.
    let _ = clipboard;
    let win_w = win_size.0;
    let win_h = win_size.1;
    let is_unbounded = win_h.is_infinite();
    let pad = 20.0;

    let mut cmds = framewise::DrawCommands::new(physical_pixels_per_logical_pixel);
    let space = if is_unbounded {
        framewise::LayoutSpace::unbounded_height(pad, pad, win_w - 2.0 * pad)
    } else {
        Rect::new(pad, pad, win_w - 2.0 * pad, win_h - 2.0 * pad).into()
    };

    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_backend,
        focus_system,
        input,
        output,
        framewise::layouts::ColumnLayout,
        space,
        &mut cmds,
    );
    ctx.time = time;

    {
        let mut outer = crate::demo_page::begin_demo_page_no_scroll(
            &mut ctx,
            "Scroll Demo",
            debug_layout,
            is_unbounded,
            framewise::layouts::RowLayout,
        );

        draw_scroll_demo_content(&mut outer.ctx, state, is_unbounded);
        outer.ctx.finish();
    }

    ctx.finish();
    cmds
}

pub(crate) fn draw_scroll_demo_content<'a, 'b, CF>(
    main_row: &'b mut WidgetContext<
        'a,
        SampleTextBackend,
        framewise::layouts::OffsetState<framewise::RowState>,
        CF,
    >,
    state: &mut ScrollDemoState,
    is_unbounded: bool,
) {
    // -- SIDEBAR (Left Column) --
    {
        let mut sidebar_col = {
            let layout_params = if is_unbounded {
                RowLayoutParams::auto().fixed_x(200.0)
            } else {
                RowLayoutParams::auto().fixed_x(200.0).fill_y()
            };
            let layout = framewise::layouts::ColumnLayout;
            main_row.child_with_layout(layout_params, layout)
        };
        let mut button_style =
            framewise::widgets::button::ButtonStyle::secondary_from_theme(&sidebar_col.theme);
        button_style.background = Color::from_srgb_f32(0.60, 0.10, 0.80, 1.0);
        button_style.hovered = Color::from_srgb_f32(0.70, 0.20, 0.90, 1.0);
        button_style.pressed = Color::from_srgb_f32(0.50, 0.05, 0.70, 1.0);

        let mut sidebar_scroll = begin_scroll_area(
            &mut sidebar_col,
            ScrollAreaSpecBuilder::new()
                .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::fixed(400.0),
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                })
                .vertical(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                }),
            if is_unbounded {
                ColumnLayoutParams::auto().fill_x().fixed_y(1300.0)
            } else {
                ColumnLayoutParams::auto().fill_x().fill_y()
            },
            &mut state.sidebar_scroll,
            framewise::layouts::ColumnLayout,
        )
        .ctx;

        for i in 0..20 {
            let shade = (i % 2) as f32 * 0.15;
            button_style.background =
                Color::from_srgb_f32(0.60 + shade, 0.10 + shade, 0.80 + shade, 1.0);
            let btn = {
                let state = &mut state.sidebar_btns[i].state;
                let layout_params = ColumnLayoutParams::fixed(180.0, 32.0).align_x(Align::Center);
                let text = format!("Menu Item {}", i + 1);
                let spec = ButtonSpec::new(&text).style(button_style);
                button(spec, layout_params, state, &mut sidebar_scroll)
            };
            let clicked = btn.input.clicked;
            if clicked {
                state.sidebar_btns[i].clicks += 1;
            }
            sidebar_scroll.spacer(8.0);
        }
        sidebar_scroll.finish();

        sidebar_col.finish()
    };

    main_row.spacer(10.0);

    // -- MAIN CONTENT (Right Column) --
    {
        let content_scroll_res = begin_scroll_area(
            main_row,
            ScrollAreaSpecBuilder::new().vertical(framewise::widgets::scroll_area::ScrollAxis {
                extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
            }),
            if is_unbounded {
                RowLayoutParams::auto().fill_x().fixed_y(1300.0)
            } else {
                RowLayoutParams::auto().fill_x().fill_y()
            },
            &mut state.right_panel_scroll,
            framewise::layouts::ColumnLayout,
        );
        let mut content_col = content_scroll_res.ctx;
        let inner_w = content_scroll_res.layout.content_bounds.w;

        // Top Header Row - Centered vertically (cross axis)
        {
            let mut header_row = {
                let layout_params = ColumnLayoutParams::fixed(inner_w, 40.0);
                let layout = framewise::layouts::RowLayout;
                content_col.child_with_layout(layout_params, layout)
            };
            let mut button_style =
                framewise::widgets::button::ButtonStyle::secondary_from_theme(&header_row.theme);
            button_style.background = Color::from_srgb_f32(0.90, 0.40, 0.10, 1.0);
            button_style.hovered = Color::from_srgb_f32(1.00, 0.50, 0.20, 1.0);
            button_style.pressed = Color::from_srgb_f32(0.80, 0.30, 0.00, 1.0);

            let _btn1 = {
                let btn_state = &mut state.top_btn1.state;
                let layout_params = RowLayoutParams::fixed(100.0, 30.0).align_y(Align::Center); // 30px height centered vertically in 40px row
                let text = "Profile";
                let spec = ButtonSpec::new(text).style(button_style);
                button(spec, layout_params, btn_state, &mut header_row)
            };

            header_row.spacer(10.0);

            let _btn2 = {
                let btn_state = &mut state.top_btn2.state;
                let layout_params = RowLayoutParams::fixed(100.0, 30.0).align_y(Align::Center); // 30px height centered vertically in 40px row
                let text = "Settings";
                let spec = ButtonSpec::new(text).style(button_style);
                button(spec, layout_params, btn_state, &mut header_row)
            };

            header_row.finish()
        };

        content_col.spacer(15.0);

        // Nested Grid Area (4 Rows of 4 Buttons)
        {
            let mut grid_col = {
                let layout_params = ColumnLayoutParams::fixed(inner_w, 200.0);
                let layout = framewise::layouts::ColumnLayout;
                content_col.child_with_layout(layout_params, layout)
            };
            let mut button_style =
                framewise::widgets::button::ButtonStyle::secondary_from_theme(&grid_col.theme);
            button_style.background = Color::from_srgb_f32(0.00, 0.60, 0.70, 1.0);
            button_style.hovered = Color::from_srgb_f32(0.10, 0.70, 0.80, 1.0);
            button_style.pressed = Color::from_srgb_f32(0.00, 0.50, 0.60, 1.0);

            for row in 0..4 {
                {
                    let mut grid_row = {
                        let layout_params = ColumnLayoutParams::fixed(inner_w, 32.0);
                        let layout = framewise::layouts::RowLayout;
                        grid_col.child_with_layout(layout_params, layout)
                    };
                    for col in 0..4 {
                        let idx = row * 4 + col;
                        let shade = ((row + col) % 2) as f32 * 0.15;
                        button_style.background =
                            Color::from_srgb_f32(0.00 + shade, 0.60 + shade, 0.70 + shade, 1.0);
                        let _btn = {
                            let btn_state = &mut state.grid_btns[idx].state;
                            let layout_params = RowLayoutParams::fixed(120.0, 32.0);
                            let text = format!("Grid [{},{}]", row, col);
                            let spec = ButtonSpec::new(&text).style(button_style);
                            button(spec, layout_params, btn_state, &mut grid_row)
                        };
                        grid_row.spacer(10.0);
                    }
                    grid_row.finish()
                };
                grid_col.spacer(10.0);
            }
            grid_col.finish()
        };

        content_col.spacer(15.0);

        // Standalone Slider Demo
        {
            let mut slider_row = {
                let layout_params = ColumnLayoutParams::fixed(inner_w, 100.0);
                let layout = framewise::layouts::RowLayout;
                content_col.child_with_layout(layout_params, layout)
            };

            {
                let slider_state: &mut SliderState = &mut state.standalone_slider_state;
                let step = 20.0;
                let layout_params = RowLayoutParams::fixed(30.0, 100.0).align_y(Align::Center);
                let spec = SliderSpec::default_from_theme(&slider_row.theme)
                    .orientation(SliderOrientation::Vertical)
                    .page_step(step)
                    .step(step);
                slider(spec, layout_params, slider_state, &mut slider_row);
            };

            slider_row.finish()
        };

        // Main Scroll Area - Centered feed buttons (cross axis)
        let mut main_scroll = begin_scroll_area(
            &mut content_col,
            ScrollAreaSpecBuilder::new().vertical(framewise::widgets::scroll_area::ScrollAxis {
                extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
            }),
            ColumnLayoutParams::fixed(inner_w, 250.0),
            &mut state.main_scroll,
            framewise::layouts::ColumnLayout,
        )
        .ctx;
        let mut button_style =
            framewise::widgets::button::ButtonStyle::secondary_from_theme(&main_scroll.theme);
        button_style.background = Color::from_srgb_f32(0.80, 0.20, 0.20, 1.0);
        button_style.hovered = Color::from_srgb_f32(0.90, 0.30, 0.30, 1.0);
        button_style.pressed = Color::from_srgb_f32(0.70, 0.10, 0.10, 1.0);

        for i in 0..30 {
            let shade = (i % 2) as f32 * 0.15;
            button_style.background =
                Color::from_srgb_f32(0.80 + shade, 0.20 + shade, 0.20 + shade, 1.0);
            let btn = {
                let btn_state = &mut state.main_btns[i].state;
                let layout_params = ColumnLayoutParams::fixed(350.0, 50.0).align_x(Align::Center); // Narrower width centered in scroll area
                let text = format!("Feed Item #{} - Very Important Notification", i + 1);
                let spec = ButtonSpec::new(&text).style(button_style);
                button(spec, layout_params, btn_state, &mut main_scroll)
            };
            let clicked = btn.input.clicked;
            if clicked {
                state.main_btns[i].clicks += 1;
            }
            main_scroll.spacer(10.0);
        }
        main_scroll.finish();

        content_col.spacer(15.0);

        // Nested Scroll Area Demo
        let row_h = 160.0;
        let mut outer_scroll = begin_scroll_area(
            &mut content_col,
            ScrollAreaSpecBuilder::new()
                .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                })
                .vertical(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                }),
            ColumnLayoutParams::fixed(inner_w, 300.0),
            &mut state.nested_outer_scroll,
            framewise::layouts::ColumnLayout,
        )
        .ctx;

        for i in 0..3 {
            let row_state = &mut state.nested_rows[i];

            let mut row_builder = {
                let layout_params = ColumnLayoutParams::fixed(800.0, row_h);
                let layout = framewise::layouts::RowLayout;
                outer_scroll.child_with_layout(layout_params, layout)
            };
            let (base_r, base_g, base_b) = match i {
                0 => (0.40, 0.80, 0.10),
                1 => (0.90, 0.20, 0.60),
                _ => (0.10, 0.50, 0.90),
            };
            let mut button_style =
                framewise::widgets::button::ButtonStyle::secondary_from_theme(&row_builder.theme);
            button_style.background = Color::from_srgb_f32(base_r, base_g, base_b, 1.0);
            button_style.hovered =
                Color::from_srgb_f32(base_r + 0.1, base_g + 0.1, base_b + 0.1, 1.0);
            button_style.pressed =
                Color::from_srgb_f32(base_r - 0.1, base_g - 0.1, base_b - 0.1, 1.0);

            // Left button
            let btn1 = {
                let btn_state = &mut row_state.btn1.state;
                let layout_params = RowLayoutParams::fixed(80.0, row_h);
                let text = format!("R{} A", i + 1);
                let spec = ButtonSpec::new(&text).style(button_style);
                button(spec, layout_params, btn_state, &mut row_builder)
            };
            let clicked1 = btn1.input.clicked;
            if clicked1 {
                row_state.btn1.clicks += 1;
            }

            row_builder.spacer(10.0);

            // 1. Vertical Inner scroll area
            let mut inner_scroll = begin_scroll_area(
                &mut row_builder,
                ScrollAreaSpecBuilder::new().vertical(
                    framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    },
                ),
                RowLayoutParams::fixed(120.0, row_h),
                &mut row_state.inner_scroll,
                framewise::layouts::ColumnLayout,
            )
            .ctx;

            for j in 0..6 {
                let shade = (j % 2) as f32 * 0.15;
                button_style.background =
                    Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                let btn = {
                    let btn_state = &mut row_state.inner_btns[j].state;
                    let layout_params = ColumnLayoutParams::fixed(100.0, 45.0);
                    let text = format!("V {}", j + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut inner_scroll)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    row_state.inner_btns[j].clicks += 1;
                }
                inner_scroll.spacer(8.0);
            }
            inner_scroll.finish();

            row_builder.spacer(10.0);

            // 2. Horizontal Inner scroll area
            let mut horiz_scroll = begin_scroll_area(
                &mut row_builder,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::FIT,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                    }),
                RowLayoutParams::fixed(180.0, row_h),
                &mut row_state.horiz_scroll,
                framewise::layouts::RowLayout,
            )
            .ctx;

            for j in 0..10 {
                let shade = (j % 2) as f32 * 0.15;
                let mut button_style =
                    framewise::widgets::button::ButtonStyle::secondary_from_theme(
                        &horiz_scroll.theme,
                    );
                button_style.background =
                    Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                let btn = {
                    let btn_state = &mut row_state.horiz_btns[j].state;
                    let layout_params = RowLayoutParams::fixed(80.0, row_h - 25.0);
                    let text = format!("H {}", j + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut horiz_scroll)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    row_state.horiz_btns[j].clicks += 1;
                }
                horiz_scroll.spacer(8.0);
            }
            horiz_scroll.finish();

            row_builder.spacer(10.0);

            // 3. Both directions Inner scroll area
            let mut both_scroll = begin_scroll_area(
                &mut row_builder,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    }),
                RowLayoutParams::fixed(200.0, row_h),
                &mut row_state.both_scroll,
                framewise::layouts::ManualLayout,
            )
            .ctx;

            for j in 0..48 {
                let x = (j % 8) as f32 * 88.0;
                let y = (j / 8) as f32 * 53.0;
                let shade = ((j % 8 + j / 8) % 2) as f32 * 0.15;
                let mut button_style =
                    framewise::widgets::button::ButtonStyle::secondary_from_theme(
                        &both_scroll.theme,
                    );
                button_style.background =
                    Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);

                let btn = {
                    let btn_state = &mut row_state.both_btns[j].state;
                    let layout_params = Rect::new(x, y, 80.0, 45.0);
                    let text = format!("2D {}", j + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut both_scroll)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    row_state.both_btns[j].clicks += 1;
                }
            }
            both_scroll.finish();

            row_builder.spacer(10.0);

            // Standalone vertical slider
            {
                let slider_state: &mut SliderState = &mut row_state.slider_state;
                let step = 20.0;
                let layout_params = RowLayoutParams::fixed(30.0, row_h);
                let spec = SliderSpec::default_from_theme(&row_builder.theme)
                    .orientation(SliderOrientation::Vertical)
                    .page_step(step)
                    .step(step);
                slider(spec, layout_params, slider_state, &mut row_builder);
            };

            row_builder.spacer(10.0);

            // Standalone horizontal slider
            {
                let slider_state: &mut SliderState = &mut row_state.horiz_slider_state;
                let step = 20.0;
                let layout_params = RowLayoutParams::fixed(100.0, 30.0);
                let spec = SliderSpec::default_from_theme(&row_builder.theme)
                    .page_step(step)
                    .step(step);
                slider(spec, layout_params, slider_state, &mut row_builder);
            };

            row_builder.finish();
            outer_scroll.spacer(10.0);
        }
        outer_scroll.finish();

        // Double Horizontal Scroll Demo
        let mut d_outer_scroll = begin_scroll_area(
            &mut content_col,
            ScrollAreaSpecBuilder::new()
                .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                })
                .vertical(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::FIT,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                }),
            ColumnLayoutParams::fixed(inner_w, 150.0),
            &mut state.double_horiz_outer_scroll,
            framewise::layouts::RowLayout,
        )
        .ctx;

        button(
            ButtonSpec::new_from_theme("Outer L", &d_outer_scroll.theme),
            RowLayoutParams::fixed(100.0, 100.0),
            &mut framewise::widgets::button::ButtonState::default(),
            &mut d_outer_scroll,
        );

        d_outer_scroll.spacer(20.0);

        let mut d_inner_scroll = begin_scroll_area(
            &mut d_outer_scroll,
            ScrollAreaSpecBuilder::new()
                .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                })
                .vertical(framewise::widgets::scroll_area::ScrollAxis {
                    extent: framewise::widgets::scroll_area::ScrollExtent::FIT,
                    vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                }),
            RowLayoutParams::fixed(600.0, 120.0),
            &mut state.double_horiz_inner_scroll,
            framewise::layouts::RowLayout,
        )
        .ctx;

        for j in 0..20 {
            let _btn = {
                let btn_state = &mut state.double_horiz_btns[j].state;
                let layout_params = RowLayoutParams::fixed(60.0, 80.0);
                let text = format!("H {}", j + 1);
                let spec = ButtonSpec::new_from_theme(&text, &d_inner_scroll.theme);
                button(spec, layout_params, btn_state, &mut d_inner_scroll)
            };
            d_inner_scroll.spacer(8.0);
        }
        d_inner_scroll.finish();

        d_outer_scroll.spacer(20.0);

        button(
            ButtonSpec::new_from_theme("Outer R", &d_outer_scroll.theme),
            RowLayoutParams::fixed(300.0, 100.0),
            &mut framewise::widgets::button::ButtonState::default(),
            &mut d_outer_scroll,
        );

        d_outer_scroll.finish();

        content_col.spacer(15.0);

        // AtMost extent demo (Phase 5): vertical AtMost(Viewport) + Auto vis.
        // The content shrink-wraps and is capped at the viewport; because it
        // provably fits, no scrollbar gutter is reserved and the content is not
        // force-filled — a case the old fixed `content_size` API could not
        // express (it would either fill or always show a bar). Adding enough
        // rows to exceed the 160px viewport would clip with no scrollbar, since
        // AtMost is a ceiling, not a scroll region.
        {
            let mut atmost = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new().vertical(
                    framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::AtMost(
                            framewise::widgets::scroll_area::ScrollLen::Viewport,
                        ),
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                    },
                ),
                ColumnLayoutParams::fixed(inner_w.min(440.0), 160.0),
                &mut state.atmost_scroll,
                framewise::layouts::ColumnLayout,
            )
            .ctx;
            for j in 0..3 {
                let btn_state = &mut state.atmost_btns[j].state;
                let text = format!("AtMost row {} (fits → no scrollbar)", j + 1);
                button(
                    ButtonSpec::new_from_theme(&text, &atmost.theme),
                    ColumnLayoutParams::fixed(260.0, 30.0),
                    btn_state,
                    &mut atmost,
                );
                atmost.spacer(6.0);
            }
            atmost.finish();
        }

        // Nested 2D Scroll Demo: outer[2D] > inner[2D]
        {
            let mut outer = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    }),
                ColumnLayoutParams::fixed(inner_w.min(440.0), 200.0),
                &mut state.nested_2d_outer_scroll,
                framewise::layouts::ManualLayout,
            )
            .ctx;

            for (k, (bx, by, lbl)) in [
                (10.0, 30.0, "OA"),
                (700.0, 30.0, "OB"),
                (10.0, 340.0, "OC"),
                (700.0, 340.0, "OD"),
                (400.0, 180.0, "OE"),
                (550.0, 100.0, "OF"),
            ]
            .iter()
            .enumerate()
            {
                let _btn = {
                    let btn_state = &mut state.nested_2d_outer_btns[k].state;
                    let layout_params = Rect::new(*bx, *by, 60.0, 28.0);
                    let text = lbl;
                    let spec = ButtonSpec::new_from_theme(text, &outer.theme);
                    button(spec, layout_params, btn_state, &mut outer)
                };
            }

            let mut inner = begin_scroll_area(
                &mut outer,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    }),
                Rect::new(80.0, 50.0, 250.0, 150.0),
                &mut state.nested_2d_inner_scroll,
                framewise::layouts::ManualLayout,
            )
            .ctx;

            for j in 0..20 {
                let col = j % 4;
                let row = j / 4;
                let shade = ((col + row) % 2) as f32 * 0.12;
                let mut button_style =
                    framewise::widgets::button::ButtonStyle::secondary_from_theme(&inner.theme);
                button_style.background =
                    Color::from_srgb_f32(0.10 + shade, 0.35 + shade, 0.70 + shade, 1.0);
                button_style.hovered =
                    Color::from_srgb_f32(0.20 + shade, 0.45 + shade, 0.80 + shade, 1.0);
                let btn = {
                    let btn_state = &mut state.nested_2d_inner_btns[j].state;
                    let layout_params = Rect::new(
                        col as f32 * 120.0 + 5.0,
                        row as f32 * 58.0 + 5.0,
                        110.0,
                        48.0,
                    );
                    let text = format!("2D {:02}", j + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut inner)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    state.nested_2d_inner_btns[j].clicks += 1;
                }
            }
            inner.finish();

            outer.finish();
        }

        // Quad-Nested Scroll Demo
        {
            let mut outer_scroll = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new().vertical(
                    framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    },
                ),
                ColumnLayoutParams::fixed(inner_w, 220.0),
                &mut state.triple_outer_scroll,
                framewise::layouts::ColumnLayout,
            )
            .ctx;

            let mut middle_scroll = begin_scroll_area(
                &mut outer_scroll,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::FIT,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                    }),
                ColumnLayoutParams::fixed(inner_w - 15.0, 160.0),
                &mut state.triple_middle_scroll,
                framewise::layouts::RowLayout,
            )
            .ctx;

            let mut inner_scroll = begin_scroll_area(
                &mut middle_scroll,
                ScrollAreaSpecBuilder::new().vertical(
                    framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    },
                ),
                RowLayoutParams::fixed(200.0, 130.0),
                &mut state.triple_inner_scroll,
                framewise::layouts::ColumnLayout,
            )
            .ctx;

            for j in 0..12 {
                let shade = (j % 2) as f32 * 0.12;
                let mut button_style =
                    framewise::widgets::button::ButtonStyle::secondary_from_theme(
                        &inner_scroll.theme,
                    );
                button_style.background =
                    Color::from_srgb_f32(0.10 + shade, 0.50 + shade, 0.30 + shade, 1.0);
                button_style.hovered =
                    Color::from_srgb_f32(0.20 + shade, 0.60 + shade, 0.40 + shade, 1.0);
                let btn = {
                    let btn_state = &mut state.triple_inner_btns[j].state;
                    let layout_params = ColumnLayoutParams::fixed(165.0, 35.0);
                    let text = format!("Inner V {}", j + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut inner_scroll)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    state.triple_inner_btns[j].clicks += 1;
                }
                inner_scroll.spacer(6.0);
            }

            let mut innermost_scroll = begin_scroll_area(
                &mut inner_scroll,
                ScrollAreaSpecBuilder::new()
                    .horizontal(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::SCROLL,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Always,
                    })
                    .vertical(framewise::widgets::scroll_area::ScrollAxis {
                        extent: framewise::widgets::scroll_area::ScrollExtent::FIT,
                        vis: framewise::widgets::scroll_area::ScrollbarVisibility::Auto,
                    }),
                ColumnLayoutParams::fixed(165.0, 50.0),
                &mut state.triple_innermost_scroll,
                framewise::layouts::RowLayout,
            )
            .ctx;
            for k in 0..5 {
                let mut button_style =
                    framewise::widgets::button::ButtonStyle::secondary_from_theme(
                        &innermost_scroll.theme,
                    );
                button_style.background =
                    Color::from_srgb_f32(0.60, 0.25 + k as f32 * 0.06, 0.10, 1.0);
                button_style.hovered =
                    Color::from_srgb_f32(0.70, 0.35 + k as f32 * 0.06, 0.20, 1.0);
                let btn = {
                    let btn_state = &mut state.triple_innermost_btns[k].state;
                    let layout_params = RowLayoutParams::fixed(80.0, 26.0);
                    let text = format!("IH {}", k + 1);
                    let spec = ButtonSpec::new(&text).style(button_style);
                    button(spec, layout_params, btn_state, &mut innermost_scroll)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    state.triple_innermost_btns[k].clicks += 1;
                }
                innermost_scroll.spacer(6.0);
            }
            innermost_scroll.finish();

            inner_scroll.finish();

            middle_scroll.spacer(10.0);

            {
                let slider_state: &mut SliderState = &mut state.triple_inner_slider_state;
                let step = 20.0;
                let layout_params = RowLayoutParams::fixed(30.0, 130.0);
                let spec = SliderSpec::default_from_theme(&middle_scroll.theme)
                    .orientation(SliderOrientation::Vertical)
                    .page_step(step)
                    .step(step);
                slider(spec, layout_params, slider_state, &mut middle_scroll);
            };

            middle_scroll.finish();

            outer_scroll.finish();
        }

        content_col.finish();
    }
}
