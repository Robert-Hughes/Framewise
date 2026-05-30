use crate::text::SampleTextSystem;
use framewise::{
    focus::FocusSystem,
    input::Input,
    layout::Layout,
    theme::Theme,
    types::{Color, Rect, Vec2},
    widget::WidgetContext,
    widgets::{
        button::button,
        frame::frame,
        label::label,
        scroll_area::{begin_scroll_area, ScrollAreaSpecBuilder, ScrollbarVisibility},
        slider::{slider, Orientation as SliderOrientation, SliderSpecBuilder, SliderState},
        text_edit::{text_edit, ClipboardAction, TextEditSpecBuilder, TextEditState},
        ButtonSpecBuilder, FrameSpecBuilder, LabelSpecBuilder,
    },
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
            slider_state: SliderState { value: 50.0, ..Default::default() },
            horiz_slider_state: SliderState { value: 50.0, ..Default::default() },
        }
    }
}

// ── Page state ────────────────────────────────────────────────────────────────

pub struct ScrollDemoState {
    pub text_edit_state: TextEditState,
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
}

impl Default for ScrollDemoState {
    fn default() -> Self {
        Self {
            text_edit_state: TextEditState::new("Search..."),
            sidebar_scroll: Default::default(),
            main_scroll: Default::default(),
            nested_outer_scroll: Default::default(),
            nested_rows: std::array::from_fn(|_| NestedRowState::default()),
            sidebar_btns: std::array::from_fn(|_| SampleButton::default()),
            main_btns: std::array::from_fn(|_| SampleButton::default()),
            grid_btns: std::array::from_fn(|_| SampleButton::default()),
            top_btn1: SampleButton::default(),
            top_btn2: SampleButton::default(),
            standalone_slider_state: SliderState { value: 50.0, ..Default::default() },
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
            triple_inner_slider_state: SliderState { value: 50.0, ..Default::default() },
            triple_innermost_scroll: Default::default(),
            triple_innermost_btns: std::array::from_fn(|_| SampleButton::default()),
        }
    }
}

// ── Draw ──────────────────────────────────────────────────────────────────────

pub fn draw_scroll_demo(
    state: &mut ScrollDemoState,
    clipboard: &mut Option<arboard::Clipboard>,
    focus_system: &mut FocusSystem,
    input: &Input,
    time: f64,
    win_size: (f32, f32),
    text_system: &mut SampleTextSystem,
) -> framewise::DrawCommands {
    let win_w = win_size.0;
    let win_h = win_size.1;

    let mut cmds = framewise::DrawCommands::new();
    let mut ctx = WidgetContext::root(
        Theme::default(),
        text_system,
        focus_system,
        input,
        framewise::layout::ManualLayout.begin(Rect::new(0.0, 0.0, win_w, win_h)),
        &mut cmds,
    );
    ctx.time = time;

    // Background frame covering the whole window.
    frame(
        &mut ctx,
        FrameSpecBuilder::new(),
        Rect::new(0.0, 0.0, win_w, win_h),
    );

    // Main container splitting into Sidebar (Left) and Content (Right)
    {
        let mut main_row = {
            let layout_params = Rect::new(10.0, 10.0, win_w - 20.0, win_h - 20.0);
            let layout = framewise::layout::RowLayout { spacing: 10.0 };
            let bounds = ctx.layout(layout_params);
            ctx.child_with_layout(layout.begin(bounds))
        };

        // -- SIDEBAR (Left Column) --
        {
            let mut sidebar_col = {
                let layout_params = Vec2::new(200.0, win_h - 20.0);
                let layout = framewise::layout::ColumnLayout { spacing: 10.0 };
                let bounds = main_row.layout(layout_params);
                main_row.child_with_layout(layout.begin(bounds))
            };
            let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&sidebar_col.theme);
            button_style.background = Color::from_srgb_f32(0.60, 0.10, 0.80, 1.0);
            button_style.hovered = Color::from_srgb_f32(0.70, 0.20, 0.90, 1.0);
            button_style.pressed = Color::from_srgb_f32(0.50, 0.05, 0.70, 1.0);

            {
                let layout_params = Vec2::new(200.0, 20.0);
                let spec_builder = LabelSpecBuilder::new().text("NAVIGATION");
                label(&mut sidebar_col, spec_builder, layout_params)
            };

            let content_height = 20.0 * 32.0 + 20.0 * 8.0;
            let mut sidebar_scroll = begin_scroll_area(
                &mut sidebar_col,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(200.0, content_height))
                    .h_vis(ScrollbarVisibility::Auto)
                    .v_vis(ScrollbarVisibility::Auto),
                Vec2::new(200.0, win_h - 60.0),
                &mut state.sidebar_scroll,
                framewise::layout::ColumnLayout { spacing: 8.0 },
            )
            .ctx;

            for i in 0..20 {
                let shade = (i % 2) as f32 * 0.15;
                button_style.background =
                    Color::from_srgb_f32(0.60 + shade, 0.10 + shade, 0.80 + shade, 1.0);
                let btn = {
                    let state = &mut state.sidebar_btns[i].state;
                    let layout_params = Vec2::new(180.0, 32.0);
                    let text = format!("Menu Item {}", i + 1);
                    let spec_builder = ButtonSpecBuilder::new().text(&text).style(button_style);
                    button(&mut sidebar_scroll, spec_builder, layout_params, state)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    state.sidebar_btns[i].clicks += 1;
                }
            }
            sidebar_scroll.finish();

            sidebar_col.finish()
        };

        // -- MAIN CONTENT (Right Column) --
        {
            let mut content_col = begin_scroll_area(
                &mut main_row,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(win_w - 240.0, 2000.0))
                    .h_vis(ScrollbarVisibility::None)
                    .v_vis(ScrollbarVisibility::Always),
                Vec2::new(win_w - 240.0, win_h - 20.0),
                &mut state.right_panel_scroll,
                framewise::layout::ColumnLayout { spacing: 15.0 },
            )
            .ctx;
            let inner_w = win_w - 240.0 - 15.0;

            // Top Header Row
            {
                let mut header_row = {
                    let layout_params = Vec2::new(inner_w, 40.0);
                    let layout = framewise::layout::RowLayout { spacing: 10.0 };
                    let bounds = content_col.layout(layout_params);
                    content_col.child_with_layout(layout.begin(bounds))
                };
                let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&header_row.theme);
                button_style.background = Color::from_srgb_f32(0.90, 0.40, 0.10, 1.0);
                button_style.hovered = Color::from_srgb_f32(1.00, 0.50, 0.20, 1.0);
                button_style.pressed = Color::from_srgb_f32(0.80, 0.30, 0.00, 1.0);

                let info = {
                    let te_state = &mut state.text_edit_state;
                    let layout_params = Vec2::new(300.0, 40.0);
                    let spec_builder = TextEditSpecBuilder::new();
                    text_edit(&mut header_row, spec_builder, layout_params, te_state)
                };

                if let Some(action) = info.clipboard_action {
                    if let Some(cb) = clipboard {
                        match action {
                            ClipboardAction::Copy(text) => drop(cb.set_text(text)),
                            ClipboardAction::Cut(text) => drop(cb.set_text(text)),
                        }
                    }
                }

                let _btn1 = {
                    let btn_state = &mut state.top_btn1.state;
                    let layout_params = Vec2::new(100.0, 40.0);
                    let text = "Profile";
                    let spec_builder = ButtonSpecBuilder::new().text(text).style(button_style);
                    button(&mut header_row, spec_builder, layout_params, btn_state)
                };

                let _btn2 = {
                    let btn_state = &mut state.top_btn2.state;
                    let layout_params = Vec2::new(100.0, 40.0);
                    let text = "Settings";
                    let spec_builder = ButtonSpecBuilder::new().text(text).style(button_style);
                    button(&mut header_row, spec_builder, layout_params, btn_state)
                };

                header_row.finish()
            };

            // Nested Grid Area (4 Rows of 4 Buttons)
            {
                let mut grid_col = {
                    let layout_params = Vec2::new(inner_w, 200.0);
                    let layout = framewise::layout::ColumnLayout { spacing: 10.0 };
                    let bounds = content_col.layout(layout_params);
                    content_col.child_with_layout(layout.begin(bounds))
                };
                let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&grid_col.theme);
                button_style.background = Color::from_srgb_f32(0.00, 0.60, 0.70, 1.0);
                button_style.hovered = Color::from_srgb_f32(0.10, 0.70, 0.80, 1.0);
                button_style.pressed = Color::from_srgb_f32(0.00, 0.50, 0.60, 1.0);

                {
                    let layout_params = Vec2::new(400.0, 20.0);
                    let spec_builder = LabelSpecBuilder::new().text("DASHBOARD GRID");
                    label(&mut grid_col, spec_builder, layout_params)
                };

                for row in 0..4 {
                    {
                        let mut grid_row = {
                            let layout_params = Vec2::new(inner_w, 32.0);
                            let layout = framewise::layout::RowLayout { spacing: 10.0 };
                            let bounds = grid_col.layout(layout_params);
                            grid_col.child_with_layout(layout.begin(bounds))
                        };
                        for col in 0..4 {
                            let idx = row * 4 + col;
                            let shade = ((row + col) % 2) as f32 * 0.15;
                            button_style.background = Color::from_srgb_f32(
                                0.00 + shade,
                                0.60 + shade,
                                0.70 + shade,
                                1.0,
                            );
                            let _btn = {
                                let btn_state = &mut state.grid_btns[idx].state;
                                let layout_params = Vec2::new(120.0, 32.0);
                                let text = format!("Grid [{},{}]", row, col);
                                let spec_builder =
                                    ButtonSpecBuilder::new().text(&text).style(button_style);
                                button(&mut grid_row, spec_builder, layout_params, btn_state)
                            };
                        }
                        grid_row.finish()
                    };
                }
                grid_col.finish()
            };

            // Standalone Slider Demo
            {
                let mut slider_row = {
                    let layout_params = Vec2::new(inner_w, 100.0);
                    let layout = framewise::layout::RowLayout { spacing: 20.0 };
                    let bounds = content_col.layout(layout_params);
                    content_col.child_with_layout(layout.begin(bounds))
                };

                {
                    let layout_params = Vec2::new(150.0, 20.0);
                    let text: &str =
                        &format!("Slider Value: {:.1}", state.standalone_slider_state.value);
                    let spec_builder = LabelSpecBuilder::new().text(text);
                    label(&mut slider_row, spec_builder, layout_params)
                };

                {
                    let slider_state: &mut SliderState = &mut state.standalone_slider_state;
                    let step = 20.0;
                    let layout_params = Vec2::new(30.0, 100.0);
                    let spec_builder = SliderSpecBuilder::new()
                        .orientation(SliderOrientation::Vertical)
                        .page_step(step)
                        .step(step);
                    slider(&mut slider_row, spec_builder, layout_params, slider_state);
                };

                slider_row.finish()
            };

            // Main Scroll Area
            {
                let layout_params = Vec2::new(400.0, 20.0);
                let spec_builder = LabelSpecBuilder::new().text("MAIN FEED");
                label(&mut content_col, spec_builder, layout_params)
            };
            let content_height = 30.0 * 50.0 + 30.0 * 10.0;
            let mut main_scroll = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(inner_w, content_height))
                    .h_vis(ScrollbarVisibility::Auto)
                    .v_vis(ScrollbarVisibility::Auto),
                Vec2::new(inner_w, 250.0),
                &mut state.main_scroll,
                framewise::layout::ColumnLayout { spacing: 10.0 },
            )
            .ctx;
            let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&main_scroll.theme);
            button_style.background = Color::from_srgb_f32(0.80, 0.20, 0.20, 1.0);
            button_style.hovered = Color::from_srgb_f32(0.90, 0.30, 0.30, 1.0);
            button_style.pressed = Color::from_srgb_f32(0.70, 0.10, 0.10, 1.0);

            for i in 0..30 {
                let shade = (i % 2) as f32 * 0.15;
                button_style.background =
                    Color::from_srgb_f32(0.80 + shade, 0.20 + shade, 0.20 + shade, 1.0);
                let btn = {
                    let btn_state = &mut state.main_btns[i].state;
                    let layout_params = Vec2::new(win_w - 280.0, 50.0);
                    let text = format!("Feed Item #{} - Very Important Notification", i + 1);
                    let spec_builder = ButtonSpecBuilder::new().text(&text).style(button_style);
                    button(&mut main_scroll, spec_builder, layout_params, btn_state)
                };
                let clicked = btn.input.clicked;
                if clicked {
                    state.main_btns[i].clicks += 1;
                }
            }
            main_scroll.finish();

            // Nested Scroll Area Demo
            {
                let layout_params = Vec2::new(400.0, 20.0);
                let spec_builder = LabelSpecBuilder::new().text("NESTED SCROLL DEMO  |  Inner area: wheel propagates to outer at ends  |  Slider: always blocks");
                label(&mut content_col, spec_builder, layout_params)
            };

            let row_h = 160.0;
            let outer_content_height = 3.0 * row_h + 2.0 * 10.0;
            let mut outer_scroll = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(800.0, outer_content_height))
                    .h_vis(ScrollbarVisibility::Auto)
                    .v_vis(ScrollbarVisibility::Auto),
                Vec2::new(inner_w, 300.0),
                &mut state.nested_outer_scroll,
                framewise::layout::ColumnLayout { spacing: 10.0 },
            )
            .ctx;

            for i in 0..3 {
                let row_state = &mut state.nested_rows[i];

                let mut row_builder = {
                    let layout_params = Vec2::new(800.0, row_h);
                    let layout = framewise::layout::RowLayout { spacing: 10.0 };
                    let bounds = outer_scroll.layout(layout_params);
                    outer_scroll.child_with_layout(layout.begin(bounds))
                };
                let (base_r, base_g, base_b) = match i {
                    0 => (0.40, 0.80, 0.10),
                    1 => (0.90, 0.20, 0.60),
                    _ => (0.10, 0.50, 0.90),
                };
                let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&row_builder.theme);
                button_style.background = Color::from_srgb_f32(base_r, base_g, base_b, 1.0);
                button_style.hovered =
                    Color::from_srgb_f32(base_r + 0.1, base_g + 0.1, base_b + 0.1, 1.0);
                button_style.pressed =
                    Color::from_srgb_f32(base_r - 0.1, base_g - 0.1, base_b - 0.1, 1.0);

                // Left button
                let btn1 = {
                    let btn_state = &mut row_state.btn1.state;
                    let layout_params = Vec2::new(80.0, row_h);
                    let text = format!("R{} A", i + 1);
                    let spec_builder = ButtonSpecBuilder::new().text(&text).style(button_style);
                    button(&mut row_builder, spec_builder, layout_params, btn_state)
                };
                let clicked1 = btn1.input.clicked;
                if clicked1 {
                    row_state.btn1.clicks += 1;
                }

                // 1. Vertical Inner scroll area
                let inner_content_height = 6.0 * 45.0 + 5.0 * 8.0;
                let mut inner_scroll = begin_scroll_area(
                    &mut row_builder,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(120.0, inner_content_height))
                        .h_vis(ScrollbarVisibility::None)
                        .v_vis(ScrollbarVisibility::Auto),
                    Vec2::new(120.0, row_h),
                    &mut row_state.inner_scroll,
                    framewise::layout::ColumnLayout { spacing: 8.0 },
                )
                .ctx;

                for j in 0..6 {
                    let shade = (j % 2) as f32 * 0.15;
                    button_style.background = Color::from_srgb_f32(
                        base_r + shade,
                        base_g + shade,
                        base_b + shade,
                        1.0,
                    );
                    let btn = {
                        let btn_state = &mut row_state.inner_btns[j].state;
                        let layout_params = Vec2::new(100.0, 45.0);
                        let text = format!("V {}", j + 1);
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut inner_scroll, spec_builder, layout_params, btn_state)
                    };
                    let clicked = btn.input.clicked;
                    if clicked {
                        row_state.inner_btns[j].clicks += 1;
                    }
                }
                inner_scroll.finish();

                // 2. Horizontal Inner scroll area
                let horiz_content_width = 10.0 * 80.0 + 9.0 * 8.0;
                let mut horiz_scroll = begin_scroll_area(
                    &mut row_builder,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(horiz_content_width, row_h))
                        .h_vis(ScrollbarVisibility::Always)
                        .v_vis(ScrollbarVisibility::None),
                    Vec2::new(180.0, row_h),
                    &mut row_state.horiz_scroll,
                    framewise::layout::RowLayout { spacing: 8.0 },
                )
                .ctx;

                for j in 0..10 {
                    let shade = (j % 2) as f32 * 0.15;
                    let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&horiz_scroll.theme);
                    button_style.background = Color::from_srgb_f32(
                        base_r + shade,
                        base_g + shade,
                        base_b + shade,
                        1.0,
                    );
                    let btn = {
                        let btn_state = &mut row_state.horiz_btns[j].state;
                        let layout_params = Vec2::new(80.0, row_h - 25.0);
                        let text = format!("H {}", j + 1);
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut horiz_scroll, spec_builder, layout_params, btn_state)
                    };
                    let clicked = btn.input.clicked;
                    if clicked {
                        row_state.horiz_btns[j].clicks += 1;
                    }
                }
                horiz_scroll.finish();

                // 3. Both directions Inner scroll area
                let both_width = 8.0 * 80.0 + 7.0 * 8.0;
                let both_height = 6.0 * 45.0 + 5.0 * 8.0;
                let mut both_scroll = begin_scroll_area(
                    &mut row_builder,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(both_width, both_height))
                        .h_vis(ScrollbarVisibility::Auto)
                        .v_vis(ScrollbarVisibility::Auto),
                    Vec2::new(200.0, row_h),
                    &mut row_state.both_scroll,
                    framewise::layout::ManualLayout,
                )
                .ctx;

                for j in 0..48 {
                    let x = (j % 8) as f32 * 88.0;
                    let y = (j / 8) as f32 * 53.0;
                    let shade = ((j % 8 + j / 8) % 2) as f32 * 0.15;
                    let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&both_scroll.theme);
                    button_style.background = Color::from_srgb_f32(
                        base_r + shade,
                        base_g + shade,
                        base_b + shade,
                        1.0,
                    );

                    let btn = {
                        let btn_state = &mut row_state.both_btns[j].state;
                        let layout_params = Rect::new(x, y, 80.0, 45.0);
                        let text = format!("2D {}", j + 1);
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut both_scroll, spec_builder, layout_params, btn_state)
                    };
                    let clicked = btn.input.clicked;
                    if clicked {
                        row_state.both_btns[j].clicks += 1;
                    }
                }
                both_scroll.finish();

                // Standalone vertical slider
                {
                    let slider_state: &mut SliderState = &mut row_state.slider_state;
                    let step = 20.0;
                    let layout_params = Vec2::new(30.0, row_h);
                    let spec_builder = SliderSpecBuilder::new()
                        .orientation(SliderOrientation::Vertical)
                        .page_step(step)
                        .step(step);
                    slider(&mut row_builder, spec_builder, layout_params, slider_state);
                };

                // Standalone horizontal slider
                {
                    let slider_state: &mut SliderState = &mut row_state.horiz_slider_state;
                    let step = 20.0;
                    let layout_params = Vec2::new(100.0, 30.0);
                    let spec_builder = SliderSpecBuilder::new().page_step(step).step(step);
                    slider(&mut row_builder, spec_builder, layout_params, slider_state);
                };

                row_builder.finish();
            }
            outer_scroll.finish();

            // Double Horizontal Scroll Demo
            {
                let layout_params = Vec2::new(400.0, 20.0);
                let spec_builder =
                    LabelSpecBuilder::new().text("DOUBLE HORIZONTAL SCROLL DEMO");
                label(&mut content_col, spec_builder, layout_params)
            };
            let mut d_outer_scroll = begin_scroll_area(
                &mut content_col,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(2000.0, 150.0))
                    .h_vis(ScrollbarVisibility::Always)
                    .v_vis(ScrollbarVisibility::None),
                Vec2::new(inner_w, 150.0),
                &mut state.double_horiz_outer_scroll,
                framewise::layout::RowLayout { spacing: 20.0 },
            )
            .ctx;

            button(
                &mut d_outer_scroll,
                ButtonSpecBuilder::new().text("Outer L"),
                Vec2::new(100.0, 100.0),
                &mut framewise::widgets::button::ButtonState::default(),
            );

            let mut d_inner_scroll = begin_scroll_area(
                &mut d_outer_scroll,
                ScrollAreaSpecBuilder::new()
                    .content_size(Vec2::new(20.0 * 60.0 + 19.0 * 8.0, 120.0))
                    .h_vis(ScrollbarVisibility::Always)
                    .v_vis(ScrollbarVisibility::None),
                Vec2::new(600.0, 120.0),
                &mut state.double_horiz_inner_scroll,
                framewise::layout::RowLayout { spacing: 8.0 },
            )
            .ctx;

            for j in 0..20 {
                let _btn = {
                    let btn_state = &mut state.double_horiz_btns[j].state;
                    let layout_params = Vec2::new(60.0, 80.0);
                    let text = format!("H {}", j + 1);
                    let spec_builder = ButtonSpecBuilder::new().text(&text);
                    button(&mut d_inner_scroll, spec_builder, layout_params, btn_state)
                };
            }
            d_inner_scroll.finish();

            button(
                &mut d_outer_scroll,
                ButtonSpecBuilder::new().text("Outer R"),
                Vec2::new(300.0, 100.0),
                &mut framewise::widgets::button::ButtonState::default(),
            );

            d_outer_scroll.finish();

            // Nested 2D Scroll Demo: outer[2D] > inner[2D]
            {
                let outer_ox = state.nested_2d_outer_scroll.offset.x;
                let outer_oy = state.nested_2d_outer_scroll.offset.y;
                let inner_ox = state.nested_2d_inner_scroll.offset.x;
                let inner_oy = state.nested_2d_inner_scroll.offset.y;

                {
                    let layout_params = Vec2::new(inner_w, 20.0);
                    let spec_builder = LabelSpecBuilder::new().text("NESTED 2D SCROLL  |  outer[H+V] > inner[H+V]  |  Each axis bubbles independently");
                    label(&mut content_col, spec_builder, layout_params)
                };

                let mut outer = begin_scroll_area(
                    &mut content_col,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(840.0, 400.0))
                        .h_vis(ScrollbarVisibility::Always)
                        .v_vis(ScrollbarVisibility::Always),
                    Vec2::new(inner_w.min(440.0), 200.0),
                    &mut state.nested_2d_outer_scroll,
                    framewise::layout::ManualLayout,
                )
                .ctx;

                {
                    let layout_params = Rect::new(0.0, 0.0, 400.0, 18.0);
                    let text: &str = &format!(
                        "OUTER x:{:.0} y:{:.0}  |  INNER x:{:.0} y:{:.0}",
                        outer_ox, outer_oy, inner_ox, inner_oy
                    );
                    let spec_builder = LabelSpecBuilder::new().text(text);
                    label(&mut outer, spec_builder, layout_params)
                };

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
                        let spec_builder = ButtonSpecBuilder::new().text(text);
                        button(&mut outer, spec_builder, layout_params, btn_state)
                    };
                }

                let mut inner = begin_scroll_area(
                    &mut outer,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(500.0, 300.0))
                        .h_vis(ScrollbarVisibility::Always)
                        .v_vis(ScrollbarVisibility::Always),
                    Rect::new(80.0, 50.0, 250.0, 150.0),
                    &mut state.nested_2d_inner_scroll,
                    framewise::layout::ManualLayout,
                )
                .ctx;

                for j in 0..20 {
                    let col = j % 4;
                    let row = j / 4;
                    let shade = ((col + row) % 2) as f32 * 0.12;
                    let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&inner.theme);
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
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut inner, spec_builder, layout_params, btn_state)
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
                let outer_y = state.triple_outer_scroll.offset.y;
                let middle_x = state.triple_middle_scroll.offset.x;
                let inner_y = state.triple_inner_scroll.offset.y;
                let innermost_x = state.triple_innermost_scroll.offset.x;

                {
                    let layout_params = Vec2::new(inner_w, 20.0);
                    let spec_builder = LabelSpecBuilder::new().text("QUAD NESTED: outer[vert] > middle[horiz] > inner[vert] > innermost[horiz]  |  Explore cross-axis isolation");
                    label(&mut content_col, spec_builder, layout_params)
                };

                let mut outer_scroll = begin_scroll_area(
                    &mut content_col,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(inner_w, 500.0))
                        .h_vis(ScrollbarVisibility::None)
                        .v_vis(ScrollbarVisibility::Always),
                    Vec2::new(inner_w, 220.0),
                    &mut state.triple_outer_scroll,
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                )
                .ctx;

                {
                    let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                    let text: &str = &format!(
                        "OUTER[V]: {:.0}  |  MIDDLE[H]: {:.0}  |  INNER[V]: {:.0}  |  INNERMOST[H]: {:.0}",
                        outer_y, middle_x, inner_y, innermost_x,
                    );
                    let spec_builder = LabelSpecBuilder::new().text(text);
                    label(&mut outer_scroll, spec_builder, layout_params)
                };

                let mut middle_scroll = begin_scroll_area(
                    &mut outer_scroll,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(1400.0, 160.0))
                        .h_vis(ScrollbarVisibility::Always)
                        .v_vis(ScrollbarVisibility::None),
                    Vec2::new(inner_w - 15.0, 160.0),
                    &mut state.triple_middle_scroll,
                    framewise::layout::RowLayout { spacing: 10.0 },
                )
                .ctx;

                {
                    let layout_params = Vec2::new(200.0, 130.0);
                    let spec_builder = LabelSpecBuilder::new().text("[ horiz padding ]");
                    label(&mut middle_scroll, spec_builder, layout_params)
                };

                let inner_content_h = 12.0 * 35.0 + 50.0 + 12.0 * 6.0;
                let mut inner_scroll = begin_scroll_area(
                    &mut middle_scroll,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(200.0, inner_content_h))
                        .h_vis(ScrollbarVisibility::None)
                        .v_vis(ScrollbarVisibility::Always),
                    Vec2::new(200.0, 130.0),
                    &mut state.triple_inner_scroll,
                    framewise::layout::ColumnLayout { spacing: 6.0 },
                )
                .ctx;

                for j in 0..12 {
                    let shade = (j % 2) as f32 * 0.12;
                    let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&inner_scroll.theme);
                    button_style.background =
                        Color::from_srgb_f32(0.10 + shade, 0.50 + shade, 0.30 + shade, 1.0);
                    button_style.hovered =
                        Color::from_srgb_f32(0.20 + shade, 0.60 + shade, 0.40 + shade, 1.0);
                    let btn = {
                        let btn_state = &mut state.triple_inner_btns[j].state;
                        let layout_params = Vec2::new(165.0, 35.0);
                        let text = format!("Inner V {}", j + 1);
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut inner_scroll, spec_builder, layout_params, btn_state)
                    };
                    let clicked = btn.input.clicked;
                    if clicked {
                        state.triple_inner_btns[j].clicks += 1;
                    }
                }

                let innermost_content_w = 5.0 * 80.0 + 4.0 * 6.0;
                let mut innermost_scroll = begin_scroll_area(
                    &mut inner_scroll,
                    ScrollAreaSpecBuilder::new()
                        .content_size(Vec2::new(innermost_content_w, 50.0))
                        .h_vis(ScrollbarVisibility::Always)
                        .v_vis(ScrollbarVisibility::None),
                    Vec2::new(165.0, 50.0),
                    &mut state.triple_innermost_scroll,
                    framewise::layout::RowLayout { spacing: 6.0 },
                )
                .ctx;
                for k in 0..5 {
                    let mut button_style = framewise::widgets::button::ButtonStyle::secondary_from_theme(&innermost_scroll.theme);
                    button_style.background =
                        Color::from_srgb_f32(0.60, 0.25 + k as f32 * 0.06, 0.10, 1.0);
                    button_style.hovered =
                        Color::from_srgb_f32(0.70, 0.35 + k as f32 * 0.06, 0.20, 1.0);
                    let btn = {
                        let btn_state = &mut state.triple_innermost_btns[k].state;
                        let layout_params = Vec2::new(80.0, 26.0);
                        let text = format!("IH {}", k + 1);
                        let spec_builder =
                            ButtonSpecBuilder::new().text(&text).style(button_style);
                        button(&mut innermost_scroll, spec_builder, layout_params, btn_state)
                    };
                    let clicked = btn.input.clicked;
                    if clicked {
                        state.triple_innermost_btns[k].clicks += 1;
                    }
                }
                innermost_scroll.finish();

                inner_scroll.finish();

                {
                    let slider_state: &mut SliderState = &mut state.triple_inner_slider_state;
                    let step = 20.0;
                    let layout_params = Vec2::new(30.0, 130.0);
                    let spec_builder = SliderSpecBuilder::new()
                        .orientation(SliderOrientation::Vertical)
                        .page_step(step)
                        .step(step);
                    slider(&mut middle_scroll, spec_builder, layout_params, slider_state);
                };

                {
                    let layout_params = Vec2::new(200.0, 130.0);
                    let spec_builder = LabelSpecBuilder::new().text("[ horiz padding ]");
                    label(&mut middle_scroll, spec_builder, layout_params)
                };

                middle_scroll.finish();

                for _ in 0..5 {
                    let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                    let spec_builder =
                        LabelSpecBuilder::new().text("[ outer vert padding row ]");
                    label(&mut outer_scroll, spec_builder, layout_params);
                }

                outer_scroll.finish();
            }

            content_col.finish();
        }

        main_row.finish()
    };

    ctx.finish();
    cmds
}
