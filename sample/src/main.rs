mod renderer;
mod spec_page;
mod text;

use framewise::{
    input::Input, layout::Layout, theme::Theme, types::{Color, Rect, Vec2}, widget::WidgetContext, widgets::{ButtonSpecBuilder, FrameSpecBuilder, LabelSpecBuilder, button::button, frame::frame, label::label, scroll_area::{ScrollbarVisibility, begin_scroll_area}, slider::{Orientation as SliderOrientation, SliderSpecBuilder, SliderState, SliderStyle, slider}, text_edit::{TextEditSpecBuilder, text_edit}}
};

use renderer::Renderer;
use text::SampleTextSystem;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

// ── App page ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppPage {
    ScrollDemo,
    WidgetSpec,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct GpuState {
    surface:   wgpu::Surface<'static>,
    device:    wgpu::Device,
    queue:     wgpu::Queue,
    config:    wgpu::SurfaceConfiguration,
    renderer:  Renderer,
    size:      PhysicalSize<u32>,
}

impl GpuState {
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size        = new_size;
        self.config.width  = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }
}

struct SampleButton {
    state:  framewise::widgets::button::ButtonState,
    clicks: u32,
}

impl Default for SampleButton {
    fn default() -> Self {
        Self {
            state:  framewise::widgets::button::ButtonState::default(),
            clicks: 0,
        }
    }
}

struct App {
    window:          Option<Arc<Window>>,
    gpu:             Option<GpuState>,
    text_system:     Option<SampleTextSystem>,
    focus_sys:       framewise::focus::FocusSystem,
    start_time:      std::time::Instant,
    click_tracker:   framewise::input::ClickTracker,
    text_edit_state: framewise::widgets::text_edit::TextEditState,
    modifiers:       winit::keyboard::ModifiersState,
    input:           Input,
    clipboard:       Option<arboard::Clipboard>,

    // Layout demo state
    sidebar_scroll:  framewise::widgets::scroll_area::ScrollState,
    main_scroll:     framewise::widgets::scroll_area::ScrollState,
    nested_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    nested_rows:     [NestedRowState; 3],
    sidebar_btns:    [SampleButton; 20],
    main_btns:       [SampleButton; 30],
    grid_btns:       [SampleButton; 16],
    top_btn1:        SampleButton,
    top_btn2:        SampleButton,
    standalone_slider_state: framewise::widgets::slider::SliderState,
    standalone_slider_val: f32,
    double_horiz_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    double_horiz_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    double_horiz_btns: [SampleButton; 20],
    right_panel_scroll: framewise::widgets::scroll_area::ScrollState,

    // Page selection.
    active_page: AppPage,
    spec_page_state: spec_page::SpecPageState,

    // Nested 2D: outer[2D] > inner[2D]
    nested_2d_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    nested_2d_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    nested_2d_inner_btns: [SampleButton; 20],
    nested_2d_outer_btns: [SampleButton; 6],

    // Quad-nested: outer_vert -> middle_horiz -> inner_vert -> innermost_horiz
    triple_outer_scroll: framewise::widgets::scroll_area::ScrollState,
    triple_middle_scroll: framewise::widgets::scroll_area::ScrollState,
    triple_inner_scroll: framewise::widgets::scroll_area::ScrollState,
    triple_inner_btns: [SampleButton; 12],
    triple_inner_slider_state: framewise::widgets::slider::SliderState,
    triple_inner_slider_val: f32,
    triple_innermost_scroll: framewise::widgets::scroll_area::ScrollState,
    triple_innermost_btns: [SampleButton; 5],
}

struct NestedRowState {
    inner_scroll: framewise::widgets::scroll_area::ScrollState,
    horiz_scroll: framewise::widgets::scroll_area::ScrollState,
    both_scroll: framewise::widgets::scroll_area::ScrollState,
    btn1: SampleButton,
    inner_btns: [SampleButton; 6],
    horiz_btns: [SampleButton; 10],
    both_btns: [SampleButton; 48],
    slider_state: framewise::widgets::slider::SliderState,
    slider_val: f32,
    horiz_slider_state: framewise::widgets::slider::SliderState,
    horiz_slider_val: f32,
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
            slider_state: Default::default(),
            slider_val: 50.0,
            horiz_slider_state: Default::default(),
            horiz_slider_val: 50.0,
        }
    }
}

impl App {
    fn new() -> Self {
        Self {
            window:          None,
            gpu:             None,
            text_system:     Some(SampleTextSystem::new()),
            focus_sys:       framewise::focus::FocusSystem::new(),
            start_time:      std::time::Instant::now(),
            click_tracker:   framewise::input::ClickTracker::new(),
            text_edit_state: framewise::widgets::text_edit::TextEditState::new("Search..."),
            modifiers:       winit::keyboard::ModifiersState::default(),
            input:           Input::new(),
            clipboard:       arboard::Clipboard::new().ok(),
            active_page:     AppPage::ScrollDemo,
            spec_page_state: spec_page::SpecPageState::default(),
            sidebar_scroll:  framewise::widgets::scroll_area::ScrollState::default(),
            main_scroll:     framewise::widgets::scroll_area::ScrollState::default(),
            nested_outer_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            nested_rows:     std::array::from_fn(|_| NestedRowState::default()),
            sidebar_btns:    std::array::from_fn(|_| SampleButton::default()),
            main_btns:       std::array::from_fn(|_| SampleButton::default()),
            grid_btns:       std::array::from_fn(|_| SampleButton::default()),
            top_btn1:        SampleButton::default(),
            top_btn2:        SampleButton::default(),
            standalone_slider_state: framewise::widgets::slider::SliderState::default(),
            standalone_slider_val: 50.0,
            double_horiz_outer_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            double_horiz_inner_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            double_horiz_btns: std::array::from_fn(|_| SampleButton::default()),
            right_panel_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            nested_2d_outer_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            nested_2d_inner_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            nested_2d_inner_btns: std::array::from_fn(|_| SampleButton::default()),
            nested_2d_outer_btns: std::array::from_fn(|_| SampleButton::default()),
            triple_outer_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            triple_middle_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            triple_inner_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            triple_inner_btns: std::array::from_fn(|_| SampleButton::default()),
            triple_inner_slider_state: framewise::widgets::slider::SliderState::default(),
            triple_inner_slider_val: 50.0,
            triple_innermost_scroll: framewise::widgets::scroll_area::ScrollState::default(),
            triple_innermost_btns: std::array::from_fn(|_| SampleButton::default()),
        }
    }

    fn draw_ui(&mut self, text_system: &mut SampleTextSystem) -> Vec<framewise::DrawCmd> {
        let win_size = self.gpu.as_ref()
            .map(|g| (g.size.width as f32, g.size.height as f32))
            .unwrap_or((1600.0, 1200.0));

         if self.active_page == AppPage::WidgetSpec {
             self.focus_sys.begin_frame();
             let cmds =  spec_page::draw_spec_page(
                 text_system,
                 &mut self.focus_sys,
                 &mut self.spec_page_state,
                 &self.input,
                 self.start_time.elapsed().as_secs_f64(),
                 win_size.0,
                 win_size.1,
             );
            self.focus_sys.end_frame();
            return cmds;
        }

        self.focus_sys.begin_frame();
        let mut ctx = WidgetContext::root(
            Theme::default(),
            text_system,
            &mut self.focus_sys,
            &self.input,
            framewise::layout::ManualLayout.begin(Rect::new(0.0, 0.0, win_size.0, win_size.1)),
        );
        ctx.text_color = Color::WHITE;
        ctx.bg_color = Color::from_srgb_f32(0.05, 0.15, 0.30, 1.0);
        ctx.time = self.start_time.elapsed().as_secs_f64();

        // Background frame covering the whole window.
        frame(&mut ctx, Rect::new(0.0, 0.0, win_size.0, win_size.1),
            FrameSpecBuilder::new());

        // Main container splitting into Sidebar (Left) and Content (Right)
        let root_cmds = {
            let mut main_row = {
                let layout_params = Rect::new(10.0, 10.0, win_size.0 - 20.0, win_size.1 - 20.0);
                let layout = framewise::layout::RowLayout { spacing: 10.0 };
                let bounds = ctx.layout(layout_params);
                ctx.child_with_layout(layout.begin(bounds), ())
            };

            // -- SIDEBAR (Left Column) --
            let sidebar_cmds = {
                let mut sidebar_col = {
                    let layout_params = Vec2::new(200.0, win_size.1 - 20.0);
                    let layout = framewise::layout::ColumnLayout { spacing: 10.0 };
                    let bounds = main_row.layout(layout_params);
                    main_row.child_with_layout(layout.begin(bounds), ())
                };
                sidebar_col.button_style.background = Color::from_srgb_f32(0.60, 0.10, 0.80, 1.0);
                sidebar_col.button_style.hovered = Color::from_srgb_f32(0.70, 0.20, 0.90, 1.0);
                sidebar_col.button_style.pressed = Color::from_srgb_f32(0.50, 0.05, 0.70, 1.0);

                {
                    let layout_params = Vec2::new(200.0, 20.0);
                    let spec_builder = LabelSpecBuilder::new("NAVIGATION".to_string())
                        .size(sidebar_col.text_size)
                        .font(sidebar_col.text_font)
                        .text_color(sidebar_col.text_color)
                        .rule(false)
                    ;
                    label(&mut sidebar_col, layout_params, spec_builder)
                };

                let content_height = 20.0 * 32.0 + 20.0 * 8.0; // 20 buttons * 32h + 8 spacing
                let mut sidebar_scroll = begin_scroll_area(
                    &mut sidebar_col,
                    Vec2::new(200.0, win_size.1 - 60.0),
                    Vec2::new(200.0, content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.sidebar_scroll,
                    framewise::layout::ColumnLayout { spacing: 8.0 },
                );

                for i in 0..20 {
                    let shade = (i % 2) as f32 * 0.15;
                    sidebar_scroll.button_style.background = Color::from_srgb_f32(0.60 + shade, 0.10 + shade, 0.80 + shade, 1.0);
                    let btn = {
                        let state = std::mem::take(&mut self.sidebar_btns[i].state);
                        let layout_params = Vec2::new(180.0, 32.0);
                        let text = format!("Menu Item {}", i + 1);
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(sidebar_scroll.button_style)
                            .disabled(false)
                        ;
                        button(&mut sidebar_scroll, state, layout_params, spec_builder)
                    };
                    let clicked = btn.clicked();
                    self.sidebar_btns[i].state = btn.state;
                    if clicked { self.sidebar_btns[i].clicks += 1; }
                }
                let sidebar_cmds = sidebar_scroll.finish();
                sidebar_col.append_cmds(sidebar_cmds);

                sidebar_col.finish()
            };
            main_row.append_cmds(sidebar_cmds);

            // -- MAIN CONTENT (Right Column) --
            {
                let mut content_col = begin_scroll_area(
                    &mut main_row,
                    Vec2::new(win_size.0 - 240.0, win_size.1 - 20.0),
                    Vec2::new(win_size.0 - 240.0, 2000.0),
                    ScrollbarVisibility::None,
                    ScrollbarVisibility::Always,
                    &mut self.right_panel_scroll,
                    framewise::layout::ColumnLayout { spacing: 15.0 },
                );
                let inner_w = win_size.0 - 240.0 - 15.0;

                // Top Header Row
                let header_cmds = {
                    let mut header_row = {
                        let layout_params = Vec2::new(inner_w, 40.0);
                        let layout = framewise::layout::RowLayout { spacing: 10.0 };
                        let bounds = content_col.layout(layout_params);
                        content_col.child_with_layout(layout.begin(bounds), ())
                    };
                    header_row.button_style.background = Color::from_srgb_f32(0.90, 0.40, 0.10, 1.0);
                    header_row.button_style.hovered = Color::from_srgb_f32(1.00, 0.50, 0.20, 1.0);
                    header_row.button_style.pressed = Color::from_srgb_f32(0.80, 0.30, 0.00, 1.0);

                    let info = {
                        let state = std::mem::take(&mut self.text_edit_state);
                        let layout_params = Vec2::new(300.0, 40.0);
                        let spec_builder = TextEditSpecBuilder::new()
                            .style(header_row.theme.text_edit_style())
                            .clip_rect(header_row.clip_rect)
                            .error(false)
                            .disabled(false);
                        text_edit(&mut header_row, state, layout_params, spec_builder)
                    };
                    self.text_edit_state = info.state;

                    if let Some(action) = info.clipboard_action {
                        if let Some(cb) = &mut self.clipboard {
                            match action {
                                framewise::widgets::text_edit::ClipboardAction::Copy(text) => drop(cb.set_text(text)),
                                framewise::widgets::text_edit::ClipboardAction::Cut(text) => drop(cb.set_text(text)),
                            }
                        }
                    }

                    let btn1 = {
                        let state = std::mem::take(&mut self.top_btn1.state);
                        let layout_params = Vec2::new(100.0, 40.0);
                        let text = "Profile".to_string();
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(header_row.button_style)
                            .disabled(false)
                        ;
                        button(&mut header_row, state, layout_params, spec_builder)
                    };
                    self.top_btn1.state = btn1.state;

                    let btn2 = {
                        let state = std::mem::take(&mut self.top_btn2.state);
                        let layout_params = Vec2::new(100.0, 40.0);
                        let text = "Settings".to_string();
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(header_row.button_style)
                            .disabled(false)
                        ;
                        button(&mut header_row, state, layout_params, spec_builder)
                    };
                    self.top_btn2.state = btn2.state;

                    header_row.finish()
                };
                content_col.append_cmds(header_cmds);

                // Nested Grid Area (4 Rows of 4 Buttons)
                let grid_cmds = {
                    let mut grid_col = {
                        let layout_params = Vec2::new(inner_w, 200.0);
                        let layout = framewise::layout::ColumnLayout { spacing: 10.0 };
                        let bounds = content_col.layout(layout_params);
                        content_col.child_with_layout(layout.begin(bounds), ())
                    };
                    grid_col.button_style.background = Color::from_srgb_f32(0.00, 0.60, 0.70, 1.0);
                    grid_col.button_style.hovered = Color::from_srgb_f32(0.10, 0.70, 0.80, 1.0);
                    grid_col.button_style.pressed = Color::from_srgb_f32(0.00, 0.50, 0.60, 1.0);

                    {
                        let layout_params = Vec2::new(400.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("DASHBOARD GRID".to_string())
                            .size(grid_col.text_size)
                            .font(grid_col.text_font)
                            .text_color(grid_col.text_color)
                            .rule(false)
                        ;
                        label(&mut grid_col, layout_params, spec_builder)
                    };

                    for row in 0..4 {
                        let row_cmds = {
                            let mut grid_row = {
                                let layout_params = Vec2::new(inner_w, 32.0);
                                let layout = framewise::layout::RowLayout { spacing: 10.0 };
                                let bounds = grid_col.layout(layout_params);
                                grid_col.child_with_layout(layout.begin(bounds), ())
                            };
                            for col in 0..4 {
                                let idx = row * 4 + col;
                                let shade = ((row + col) % 2) as f32 * 0.15;
                                grid_row.button_style.background = Color::from_srgb_f32(0.00 + shade, 0.60 + shade, 0.70 + shade, 1.0);
                                let btn = {
                                    let state = std::mem::take(&mut self.grid_btns[idx].state);
                                    let layout_params = Vec2::new(120.0, 32.0);
                                    let text = format!("Grid [{},{}]", row, col);
                                    let spec_builder = ButtonSpecBuilder::new(text)
                                        .style(grid_row.button_style)
                                        .disabled(false)
                                    ;
                                    button(&mut grid_row, state, layout_params, spec_builder)
                                };
                                self.grid_btns[idx].state = btn.state;
                            }
                            grid_row.finish()
                        };
                        grid_col.append_cmds(row_cmds);
                    }
                    grid_col.finish()
                };
                content_col.append_cmds(grid_cmds);

                // Standalone Slider Demo
                let slider_cmds = {
                    let mut slider_row = {
                        let layout_params = Vec2::new(inner_w, 100.0);
                        let layout = framewise::layout::RowLayout { spacing: 20.0 };
                        let bounds = content_col.layout(layout_params);
                        content_col.child_with_layout(layout.begin(bounds), ())
                    };

                    {
                        let layout_params = Vec2::new(150.0, 20.0);
                        let text: &str = &format!("Slider Value: {:.1}", self.standalone_slider_val);
                        let spec_builder = LabelSpecBuilder::new(text.to_string())
                            .size(slider_row.text_size)
                            .font(slider_row.text_font)
                            .text_color(slider_row.text_color)
                            .rule(false)
                        ;
                        label(&mut slider_row, layout_params, spec_builder)
                    };

                    {
                        let state: &mut SliderState = &mut self.standalone_slider_state;
                        let value: &mut f32 = &mut self.standalone_slider_val;
                        let step = 20.0;
                        let orientation = SliderOrientation::Vertical;
                        let layout_params = Vec2::new(30.0, 100.0);
                        let spec_builder = SliderSpecBuilder::new()
                            .orientation(orientation)
                            .min(0.0)
                            .max(100.0)
                            .page_step(step)
                            .step(step)
                            .thumb_size_ratio(None)
                            .style(SliderStyle::default())
                            .clip_rect(slider_row.clip_rect)
                            .claim_scroll_at_ends(true);
                        slider(&mut slider_row, state, value, layout_params, spec_builder);
                    };

                    slider_row.finish()
                };
                content_col.append_cmds(slider_cmds);

                // Main Scroll Area
                {
                    let layout_params = Vec2::new(400.0, 20.0);
                    let spec_builder = LabelSpecBuilder::new("MAIN FEED".to_string())
                        .size(content_col.text_size)
                        .font(content_col.text_font)
                        .text_color(content_col.text_color)
                        .rule(false)
                    ;
                    label(&mut content_col, layout_params, spec_builder)
                };
                let content_height = 30.0 * 50.0 + 30.0 * 10.0;
                let mut main_scroll = begin_scroll_area(
                    &mut content_col,
                    Vec2::new(inner_w, 250.0),
                    Vec2::new(inner_w, content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.main_scroll,
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                );
                main_scroll.button_style.background = Color::from_srgb_f32(0.80, 0.20, 0.20, 1.0);
                main_scroll.button_style.hovered = Color::from_srgb_f32(0.90, 0.30, 0.30, 1.0);
                main_scroll.button_style.pressed = Color::from_srgb_f32(0.70, 0.10, 0.10, 1.0);

                for i in 0..30 {
                    let shade = (i % 2) as f32 * 0.15;
                    main_scroll.button_style.background = Color::from_srgb_f32(0.80 + shade, 0.20 + shade, 0.20 + shade, 1.0);
                    let btn = {
                        let state = std::mem::take(&mut self.main_btns[i].state);
                        let layout_params = Vec2::new(win_size.0 - 280.0, 50.0);
                        let text = format!("Feed Item #{} - Very Important Notification", i + 1);
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(main_scroll.button_style)
                            .disabled(false)
                        ;
                        button(&mut main_scroll, state, layout_params, spec_builder)
                    };
                    let clicked = btn.clicked();
                    self.main_btns[i].state = btn.state;
                    if clicked { self.main_btns[i].clicks += 1; }
                }
                let main_cmds = main_scroll.finish();
                content_col.append_cmds(main_cmds);

                // Nested Scroll Area Demo
                {
                    let layout_params = Vec2::new(400.0, 20.0);
                    let spec_builder = LabelSpecBuilder::new("NESTED SCROLL DEMO  |  Inner area: wheel propagates to outer at ends  |  Slider: always blocks".to_string())
                        .size(content_col.text_size)
                        .font(content_col.text_font)
                        .text_color(content_col.text_color)
                        .rule(false)
                    ;
                    label(&mut content_col, layout_params, spec_builder)
                };

                let row_h = 160.0;
                let outer_content_height = 3.0 * row_h + 2.0 * 10.0;
                let mut outer_scroll = begin_scroll_area(
                    &mut content_col,
                    Vec2::new(inner_w, 300.0),
                    Vec2::new(800.0, outer_content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.nested_outer_scroll,
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                );

                for i in 0..3 {
                    let row_state = &mut self.nested_rows[i];

                    let mut row_builder = {
                        let layout_params = Vec2::new(800.0, row_h);
                        let layout = framewise::layout::RowLayout { spacing: 10.0 };
                        let bounds = outer_scroll.layout(layout_params);
                        outer_scroll.child_with_layout(layout.begin(bounds), ())
                    };
                    let (base_r, base_g, base_b) = match i {
                        0 => (0.40, 0.80, 0.10), // Lime green
                        1 => (0.90, 0.20, 0.60), // Hot pink
                        _ => (0.10, 0.50, 0.90), // Vivid blue
                    };
                    row_builder.button_style.background = Color::from_srgb_f32(base_r, base_g, base_b, 1.0);
                    row_builder.button_style.hovered = Color::from_srgb_f32(base_r + 0.1, base_g + 0.1, base_b + 0.1, 1.0);
                    row_builder.button_style.pressed = Color::from_srgb_f32(base_r - 0.1, base_g - 0.1, base_b - 0.1, 1.0);

                    // Left button
                    let btn1 = {
                        let state = std::mem::take(&mut row_state.btn1.state);
                        let layout_params = Vec2::new(80.0, row_h);
                        let text = format!("R{} A", i + 1);
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(row_builder.button_style)
                            .disabled(false)
                        ;
                        button(&mut row_builder, state, layout_params, spec_builder)
                    };
                    let clicked1 = btn1.clicked();
                    row_state.btn1.state = btn1.state;
                    if clicked1 { row_state.btn1.clicks += 1; }

                    // 1. Vertical Inner scroll area
                    let inner_content_height = 6.0 * 45.0 + 5.0 * 8.0;
                    let mut inner_scroll = begin_scroll_area(
                        &mut row_builder,
                        Vec2::new(120.0, row_h),
                        Vec2::new(120.0, inner_content_height),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Auto,
                        &mut row_state.inner_scroll,
                        framewise::layout::ColumnLayout { spacing: 8.0 },
                        );

                    for j in 0..6 {
                        let shade = (j % 2) as f32 * 0.15;
                        inner_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                        let btn = {
                            let state = std::mem::take(&mut row_state.inner_btns[j].state);
                            let layout_params = Vec2::new(100.0, 45.0);
                            let text = format!("V {}", j + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(inner_scroll.button_style)
                                .disabled(false)
                            ;
                            button(&mut inner_scroll, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        row_state.inner_btns[j].state = btn.state;
                        if clicked { row_state.inner_btns[j].clicks += 1; }
                    }
                    let cmds = inner_scroll.finish();
                    row_builder.append_cmds(cmds);
                    // row_builder.finish_child(inner_scroll);//TODO:

                    // 2. Horizontal Inner scroll area (using None for vertical scrollbar)
                    let horiz_content_width = 10.0 * 80.0 + 9.0 * 8.0;
                    let mut horiz_scroll = begin_scroll_area(
                        &mut row_builder,
                        Vec2::new(180.0, row_h),
                        Vec2::new(horiz_content_width, row_h),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut row_state.horiz_scroll,
                        framewise::layout::RowLayout { spacing: 8.0 },
                        );

                    for j in 0..10 {
                        let shade = (j % 2) as f32 * 0.15;
                        horiz_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                        let btn = {
                            let state = std::mem::take(&mut row_state.horiz_btns[j].state);
                            let layout_params = Vec2::new(80.0, row_h - 25.0);
                            let text = format!("H {}", j + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(horiz_scroll.button_style)
                                .disabled(false)
                            ;
                            button(&mut horiz_scroll, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        row_state.horiz_btns[j].state = btn.state;
                        if clicked { row_state.horiz_btns[j].clicks += 1; }
                    }
                    let horiz_cmds = horiz_scroll.finish();
                    row_builder.append_cmds(horiz_cmds);

                    // 3. Both directions Inner scroll area
                    let both_width = 8.0 * 80.0 + 7.0 * 8.0;
                    let both_height = 6.0 * 45.0 + 5.0 * 8.0;
                    let mut both_scroll = begin_scroll_area(
                        &mut row_builder,
                        Vec2::new(200.0, row_h),
                        Vec2::new(both_width, both_height),
                        ScrollbarVisibility::Auto,
                        ScrollbarVisibility::Auto,
                        &mut row_state.both_scroll,
                        framewise::layout::ManualLayout,
                        );

                    for j in 0..48 {
                        let x = (j % 8) as f32 * 88.0;
                        let y = (j / 8) as f32 * 53.0;
                        let shade = ((j % 8 + j / 8) % 2) as f32 * 0.15;
                        both_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);

                        let btn = {
                            let state = std::mem::take(&mut row_state.both_btns[j].state);
                            let layout_params = Rect::new(x, y, 80.0, 45.0);
                            let text = format!("2D {}", j + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(both_scroll.button_style)
                                .disabled(false)
                            ;
                            button(&mut both_scroll, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        row_state.both_btns[j].state = btn.state;
                        if clicked { row_state.both_btns[j].clicks += 1; }
                    }
                    let both_cmds = both_scroll.finish();
                    row_builder.append_cmds(both_cmds);

                    // Standalone vertical slider
                    {
                        let state: &mut SliderState = &mut row_state.slider_state;
                        let value: &mut f32 = &mut row_state.slider_val;
                        let step = 20.0;
                        let orientation = SliderOrientation::Vertical;
                        let layout_params = Vec2::new(30.0, row_h);
                        let spec_builder = SliderSpecBuilder::new()
                            .orientation(orientation)
                            .min(0.0)
                            .max(100.0)
                            .page_step(step)
                            .step(step)
                            .thumb_size_ratio(None)
                            .style(SliderStyle::default())
                            .clip_rect(row_builder.clip_rect)
                            .claim_scroll_at_ends(true);
                        slider(&mut row_builder, state, value, layout_params, spec_builder);
                    };

                    // Standalone horizontal slider
                    {
                        let state: &mut SliderState = &mut row_state.horiz_slider_state;
                        let value: &mut f32 = &mut row_state.horiz_slider_val;
                        let step = 20.0;
                        let orientation = SliderOrientation::Horizontal;
                        let layout_params = Vec2::new(100.0, 30.0);
                        let spec_builder = SliderSpecBuilder::new()
                            .orientation(orientation)
                            .min(0.0)
                            .max(100.0)
                            .page_step(step)
                            .step(step)
                            .thumb_size_ratio(None)
                            .style(SliderStyle::default())
                            .clip_rect(row_builder.clip_rect)
                            .claim_scroll_at_ends(true);
                        slider(&mut row_builder, state, value, layout_params, spec_builder);
                    };

                    let row_cmds = row_builder.finish();
                    outer_scroll.append_cmds(row_cmds);
                }
                let outer_cmds = outer_scroll.finish();
                content_col.append_cmds(outer_cmds);

                // Double Horizontal Scroll Demo
                {
                    let layout_params = Vec2::new(400.0, 20.0);
                    let spec_builder = LabelSpecBuilder::new("DOUBLE HORIZONTAL SCROLL DEMO".to_string())
                        .size(content_col.text_size)
                        .font(content_col.text_font)
                        .text_color(content_col.text_color)
                        .rule(false)
                    ;
                    label(&mut content_col, layout_params, spec_builder)
                };
                let mut d_outer_scroll = begin_scroll_area(
                    &mut content_col,
                    Vec2::new(inner_w, 150.0),
                    Vec2::new(2000.0, 150.0),
                    ScrollbarVisibility::Always,
                    ScrollbarVisibility::None,
                    &mut self.double_horiz_outer_scroll,
                    framewise::layout::RowLayout { spacing: 20.0 },
                );

                // Left spacer/button
                {
                    let state = Default::default();
                    let layout_params = Vec2::new(100.0, 100.0);
                    let text = "Outer L".to_string();
                    let spec_builder = ButtonSpecBuilder::new(text)
                        .style(d_outer_scroll.button_style)
                        .disabled(false)
                    ;
                    button(&mut d_outer_scroll, state, layout_params, spec_builder)
                };

                // Inner horizontal scroll area
                let mut d_inner_scroll = begin_scroll_area(
                    &mut d_outer_scroll,
                    Vec2::new(600.0, 120.0),
                    Vec2::new(20.0 * 60.0 + 19.0 * 8.0, 120.0),
                    ScrollbarVisibility::Always,
                    ScrollbarVisibility::None,
                    &mut self.double_horiz_inner_scroll,
                    framewise::layout::RowLayout { spacing: 8.0 },
                );

                for j in 0..20 {
                    let btn = {
                        let state = std::mem::take(&mut self.double_horiz_btns[j].state);
                        let layout_params = Vec2::new(60.0, 80.0);
                        let text = format!("H {}", j + 1);
                        let spec_builder = ButtonSpecBuilder::new(text)
                            .style(d_inner_scroll.button_style)
                            .disabled(false)
                        ;
                        button(&mut d_inner_scroll, state, layout_params, spec_builder)
                    };
                    self.double_horiz_btns[j].state = btn.state;
                }
                let d_inner_cmds = d_inner_scroll.finish();
                d_outer_scroll.append_cmds(d_inner_cmds);

                // Right spacer/button
                {
                    let state = Default::default();
                    let layout_params = Vec2::new(300.0, 100.0);
                    let text = "Outer R".to_string();
                    let spec_builder = ButtonSpecBuilder::new(text)
                        .style(d_outer_scroll.button_style)
                        .disabled(false)
                    ;
                    button(&mut d_outer_scroll, state, layout_params, spec_builder)
                };

                let d_outer_cmds = d_outer_scroll.finish();
                content_col.append_cmds(d_outer_cmds);

                // Nested 2D Scroll Demo: outer[2D] > inner[2D]
                {
                    let outer_ox = self.nested_2d_outer_scroll.offset.x;
                    let outer_oy = self.nested_2d_outer_scroll.offset.y;
                    let inner_ox = self.nested_2d_inner_scroll.offset.x;
                    let inner_oy = self.nested_2d_inner_scroll.offset.y;

                    {
                        let layout_params = Vec2::new(inner_w, 20.0);
                        let spec_builder = LabelSpecBuilder::new("NESTED 2D SCROLL  |  outer[H+V] > inner[H+V]  |  Each axis bubbles independently".to_string())
                            .size(content_col.text_size)
                            .font(content_col.text_font)
                            .text_color(content_col.text_color)
                            .rule(false)
                        ;
                        label(&mut content_col, layout_params, spec_builder)
                    };

                    // Outer 2D: viewport 420x200, content 840x400
                    let mut outer = begin_scroll_area(
                        &mut content_col,
                        Vec2::new(inner_w.min(440.0), 200.0),
                        Vec2::new(840.0, 400.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::Always,
                        &mut self.nested_2d_outer_scroll,
                        framewise::layout::ManualLayout,
                        );

                    // Status label at top-left of outer content
                    {
                        let layout_params = Rect::new(0.0, 0.0, 400.0, 18.0);
                        let text: &str = &format!("OUTER x:{:.0} y:{:.0}  |  INNER x:{:.0} y:{:.0}", outer_ox, outer_oy, inner_ox, inner_oy);
                        let spec_builder = LabelSpecBuilder::new(text.to_string())
                            .size(outer.text_size)
                            .font(outer.text_font)
                            .text_color(outer.text_color)
                            .rule(false)
                        ;
                        label(&mut outer, layout_params, spec_builder)
                    };

                    // Some outer-only buttons scattered in the far corners to make outer scrollable
                    for (k, (bx, by, label)) in [
                        (10.0,  30.0, "OA"),
                        (700.0, 30.0, "OB"),
                        (10.0,  340.0, "OC"),
                        (700.0, 340.0, "OD"),
                        (400.0, 180.0, "OE"),
                        (550.0, 100.0, "OF"),
                    ].iter().enumerate() {
                        let btn = {
                            let state = std::mem::take(&mut self.nested_2d_outer_btns[k].state);
                            let layout_params = Rect::new(*bx, *by, 60.0, 28.0);
                            let text = label.to_string();
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(outer.button_style)
                                .disabled(false)
                            ;
                            button(&mut outer, state, layout_params, spec_builder)
                        };
                        self.nested_2d_outer_btns[k].state = btn.state;
                    }

                    // Inner 2D: viewport 250x150, content 500x300 — 4x5 button grid
                    let mut inner = begin_scroll_area(
                        &mut outer,
                        Rect::new(80.0, 50.0, 250.0, 150.0),
                        Vec2::new(500.0, 300.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::Always,
                        &mut self.nested_2d_inner_scroll,
                        framewise::layout::ManualLayout,
                        );

                    for j in 0..20 {
                        let col = j % 4;
                        let row = j / 4;
                        let shade = ((col + row) % 2) as f32 * 0.12;
                        inner.button_style.background = Color::from_srgb_f32(0.10 + shade, 0.35 + shade, 0.70 + shade, 1.0);
                        inner.button_style.hovered    = Color::from_srgb_f32(0.20 + shade, 0.45 + shade, 0.80 + shade, 1.0);
                        let btn = {
                            let state = std::mem::take(&mut self.nested_2d_inner_btns[j].state);
                            let layout_params = Rect::new(col as f32 * 120.0 + 5.0, row as f32 * 58.0 + 5.0, 110.0, 48.0);
                            let text = format!("2D {:02}", j + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(inner.button_style)
                                .disabled(false)
                            ;
                            button(&mut inner, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        self.nested_2d_inner_btns[j].state = btn.state;
                        if clicked { self.nested_2d_inner_btns[j].clicks += 1; }
                    }
                    let inner_cmds = inner.finish();
                    outer.append_cmds(inner_cmds);

                    let outer_cmds = outer.finish();
                    content_col.append_cmds(outer_cmds);
                }

                // Triple-Nested Scroll Demo: outer_vert -> middle_horiz -> inner_vert
                {
                    let outer_y = self.triple_outer_scroll.offset.y;
                    let middle_x = self.triple_middle_scroll.offset.x;
                    let inner_y = self.triple_inner_scroll.offset.y;
                    let innermost_x = self.triple_innermost_scroll.offset.x;

                    {
                        let layout_params = Vec2::new(inner_w, 20.0);
                        let spec_builder = LabelSpecBuilder::new("QUAD NESTED: outer[vert] > middle[horiz] > inner[vert] > innermost[horiz]  |  Explore cross-axis isolation".to_string())
                            .size(content_col.text_size)
                            .font(content_col.text_font)
                            .text_color(content_col.text_color)
                            .rule(false)
                        ;
                        label(&mut content_col, layout_params, spec_builder)
                    };

                    // Outer vertical scroll area (taller content)
                    let mut outer_scroll = begin_scroll_area(
                        &mut content_col,
                        Vec2::new(inner_w, 220.0),
                        Vec2::new(inner_w, 500.0),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut self.triple_outer_scroll,
                        framewise::layout::ColumnLayout { spacing: 10.0 },
                        );

                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let text: &str = &format!(
                                            "OUTER[V]: {:.0}  |  MIDDLE[H]: {:.0}  |  INNER[V]: {:.0}  |  INNERMOST[H]: {:.0}",
                                            outer_y, middle_x, inner_y, innermost_x,
                                        );
                        let spec_builder = LabelSpecBuilder::new(text.to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };

                    // Middle horizontal scroll area inside outer vertical
                    let mut middle_scroll = begin_scroll_area(
                        &mut outer_scroll,
                        Vec2::new(inner_w - 15.0, 160.0),
                        Vec2::new(1400.0, 160.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut self.triple_middle_scroll,
                        framewise::layout::RowLayout { spacing: 10.0 },
                        );

                    {
                        let layout_params = Vec2::new(200.0, 130.0);
                        let spec_builder = LabelSpecBuilder::new("[ horiz padding ]".to_string())
                            .size(middle_scroll.text_size)
                            .font(middle_scroll.text_font)
                            .text_color(middle_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut middle_scroll, layout_params, spec_builder)
                    };

                    // Inner vertical scroll area inside middle horizontal
                    let inner_content_h = 12.0 * 35.0 + 50.0 + 12.0 * 6.0;
                    let mut inner_scroll = begin_scroll_area(
                        &mut middle_scroll,
                        Vec2::new(200.0, 130.0),
                        Vec2::new(200.0, inner_content_h),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut self.triple_inner_scroll,
                        framewise::layout::ColumnLayout { spacing: 6.0 },
                        );

                    for j in 0..12 {
                        let shade = (j % 2) as f32 * 0.12;
                        inner_scroll.button_style.background = Color::from_srgb_f32(0.10 + shade, 0.50 + shade, 0.30 + shade, 1.0);
                        inner_scroll.button_style.hovered = Color::from_srgb_f32(0.20 + shade, 0.60 + shade, 0.40 + shade, 1.0);
                        let btn = {
                            let state = std::mem::take(&mut self.triple_inner_btns[j].state);
                            let layout_params = Vec2::new(165.0, 35.0);
                            let text = format!("Inner V {}", j + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(inner_scroll.button_style)
                                .disabled(false)
                            ;
                            button(&mut inner_scroll, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        self.triple_inner_btns[j].state = btn.state;
                        if clicked { self.triple_inner_btns[j].clicks += 1; }
                    }

                    // Innermost horizontal scroll area — 4th nesting level
                    let innermost_content_w = 5.0 * 80.0 + 4.0 * 6.0;
                    let mut innermost_scroll = begin_scroll_area(
                        &mut inner_scroll,
                        Vec2::new(165.0, 50.0),
                        Vec2::new(innermost_content_w, 50.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut self.triple_innermost_scroll,
                        framewise::layout::RowLayout { spacing: 6.0 },
                        );
                    for k in 0..5 {
                        innermost_scroll.button_style.background = Color::from_srgb_f32(0.60, 0.25 + k as f32 * 0.06, 0.10, 1.0);
                        innermost_scroll.button_style.hovered    = Color::from_srgb_f32(0.70, 0.35 + k as f32 * 0.06, 0.20, 1.0);
                        let btn = {
                            let state = std::mem::take(&mut self.triple_innermost_btns[k].state);
                            let layout_params = Vec2::new(80.0, 26.0);
                            let text = format!("IH {}", k + 1);
                                let spec_builder = ButtonSpecBuilder::new(text)
                                .style(innermost_scroll.button_style)
                                .disabled(false)
                            ;
                            button(&mut innermost_scroll, state, layout_params, spec_builder)
                        };
                        let clicked = btn.clicked();
                        self.triple_innermost_btns[k].state = btn.state;
                        if clicked { self.triple_innermost_btns[k].clicks += 1; }
                    }
                    let innermost_cmds = innermost_scroll.finish();
                    inner_scroll.append_cmds(innermost_cmds);

                    let inner_cmds = inner_scroll.finish();
                    middle_scroll.append_cmds(inner_cmds);

                    // Inner vertical slider
                    {
                        let state: &mut SliderState = &mut self.triple_inner_slider_state;
                        let value: &mut f32 = &mut self.triple_inner_slider_val;
                        let step = 20.0;
                        let orientation = SliderOrientation::Vertical;
                        let layout_params = Vec2::new(30.0, 130.0);
                        let spec_builder = SliderSpecBuilder::new()
                            .orientation(orientation)
                            .min(0.0)
                            .max(100.0)
                            .page_step(step)
                            .step(step)
                            .thumb_size_ratio(None)
                            .style(SliderStyle::default())
                            .clip_rect(middle_scroll.clip_rect)
                            .claim_scroll_at_ends(true);
                        slider(&mut middle_scroll, state, value, layout_params, spec_builder);
                    };

                    {
                        let layout_params = Vec2::new(200.0, 130.0);
                        let spec_builder = LabelSpecBuilder::new("[ horiz padding ]".to_string())
                            .size(middle_scroll.text_size)
                            .font(middle_scroll.text_font)
                            .text_color(middle_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut middle_scroll, layout_params, spec_builder)
                    };

                    let middle_cmds = middle_scroll.finish();
                    outer_scroll.append_cmds(middle_cmds);

                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("[ outer vert padding row ]".to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };
                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("[ outer vert padding row ]".to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };
                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("[ outer vert padding row ]".to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };
                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("[ outer vert padding row ]".to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };
                    {
                        let layout_params = Vec2::new(inner_w - 15.0, 20.0);
                        let spec_builder = LabelSpecBuilder::new("[ outer vert padding row ]".to_string())
                            .size(outer_scroll.text_size)
                            .font(outer_scroll.text_font)
                            .text_color(outer_scroll.text_color)
                            .rule(false)
                        ;
                        label(&mut outer_scroll, layout_params, spec_builder)
                    };

                    let outer_cmds = outer_scroll.finish();
                    content_col.append_cmds(outer_cmds);
                }

                let content_cmds = content_col.finish();
                main_row.append_cmds(content_cmds);
            }

            main_row.finish()
        };
        ctx.append_cmds(root_cmds);

        let cmds = ctx.finish();
        self.focus_sys.end_frame();
        cmds
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window.
        let mut attrs = Window::default_attributes()
            .with_title("Framewise Sample")
            .with_inner_size(PhysicalSize::new(1600u32, 1200u32));

        let svg_data = include_bytes!("../../logo/framewise-mark.svg");
        let opt = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        if let Ok(tree) = usvg::Tree::from_data(svg_data, &opt, &fontdb) {
            let size = tree.size().to_int_size();
            if let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(size.width(), size.height()) {
                resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
                if let Ok(icon) = winit::window::Icon::from_rgba(pixmap.take(), size.width(), size.height()) {
                    attrs = attrs.with_window_icon(Some(icon));
                }
            }
        }

        let window = Arc::new(
            event_loop.create_window(attrs).expect("failed to create window"),
        );

        // Initialise wgpu synchronously using pollster.
        let gpu = pollster::block_on(init_wgpu(Arc::clone(&window)));

        self.window = Some(window);
        self.gpu    = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event:      WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(new_size);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.input.mouse_pos = Vec2::new(
                    position.x as f32,
                    position.y as f32,
                );
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let _delta_y = match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.input.scroll_delta = Vec2::new(x, y);
                        y
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        let dy = pos.y as f32 / 20.0;
                        self.input.scroll_delta = Vec2::new(pos.x as f32 / 20.0, dy);
                        dy
                    }
                };
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if button != winit::event::MouseButton::Left {
                    return;
                }
                match state {
                    ElementState::Pressed => {
                        self.input.mouse_down    = true;
                        self.input.mouse_pressed = true;
                        self.input.mouse_clicked = false;

                        let now = std::time::Instant::now();
                        let count = self.click_tracker.register_click(self.input.mouse_pos, now);
                        self.input.mouse_click_count = count;
                    }
                    ElementState::Released => {
                        self.input.mouse_down    = false;
                        self.input.mouse_clicked = true;
                    }
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.input.modifier_shift = modifiers.state().shift_key();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // F2 toggles between scroll-demo and widget-spec pages.
                if event.state == ElementState::Pressed {
                    if let winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F2) = event.physical_key {
                        self.active_page = match self.active_page {
                            AppPage::ScrollDemo  => AppPage::WidgetSpec,
                            AppPage::WidgetSpec  => AppPage::ScrollDemo,
                        };
                    }
                }

                match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Tab) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_tab = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Enter) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_enter = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Space) => {
                        match event.state {
                            ElementState::Pressed => {
                                if !self.input.key_down_space {
                                    self.input.key_pressed_space = true;
                                }
                                self.input.key_down_space = true;
                            }
                            ElementState::Released => {
                                self.input.key_down_space = false;
                                self.input.key_released_space = true;
                            }
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::PageUp) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_page_up = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::PageDown) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_page_down = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Home) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_home = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::End) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_end = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowUp) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_up = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowDown) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_down = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowLeft) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_left = true;
                        }
                    }
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::ArrowRight) => {
                        if event.state == ElementState::Pressed {
                            self.input.key_pressed_right = true;
                        }
                    }
                    _ => {}
                }

                if event.state == ElementState::Pressed {
                    use winit::keyboard::{Key, NamedKey};
                    use framewise::input::TextEvent;

                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => self.input.text_events.push(TextEvent::Backspace { ctrl: self.modifiers.control_key() }),
                        Key::Named(NamedKey::Delete) => self.input.text_events.push(TextEvent::Delete { ctrl: self.modifiers.control_key() }),
                        Key::Named(NamedKey::ArrowLeft) => self.input.text_events.push(TextEvent::CaretLeft { shift: self.modifiers.shift_key(), ctrl: self.modifiers.control_key() }),
                        Key::Named(NamedKey::ArrowRight)=> self.input.text_events.push(TextEvent::CaretRight { shift: self.modifiers.shift_key(), ctrl: self.modifiers.control_key() }),
                        Key::Named(NamedKey::Home)      => self.input.text_events.push(TextEvent::CaretHome { shift: self.modifiers.shift_key() }),
                        Key::Named(NamedKey::End)       => self.input.text_events.push(TextEvent::CaretEnd { shift: self.modifiers.shift_key() }),
                        Key::Character(s) => {
                            if self.modifiers.control_key() {
                                match s.as_str() {
                                    "a" | "A" => self.input.text_events.push(TextEvent::SelectAll),
                                    "c" | "C" => self.input.text_events.push(TextEvent::Copy),
                                    "x" | "X" => self.input.text_events.push(TextEvent::Cut),
                                    "v" | "V" => {
                                        if let Some(cb) = &mut self.clipboard {
                                            if let Ok(text) = cb.get_text() {
                                                self.input.text_events.push(TextEvent::Paste(text));
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }

                    if let Some(text) = &event.text {
                        if !self.modifiers.control_key() && !self.modifiers.alt_key() {
                            for c in text.chars() {
                                if !c.is_control() {
                                    self.input.text_events.push(TextEvent::Char(c));
                                }
                            }
                        }
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // Build UI and render.
                let mut text_system = self.text_system.take().unwrap();
                text_system.begin_frame();
                let draw_cmds = self.draw_ui(&mut text_system);

                // Clear the one-frame flags after UI has consumed them.
                self.input.clear_frame_state();

                if let Some(gpu) = &mut self.gpu {
                    match gpu.surface.get_current_texture() {
                        Ok(frame) => {
                            let view = frame.texture.create_view(
                                &wgpu::TextureViewDescriptor::default(),
                            );
                            let mut encoder =
                                gpu.device.create_command_encoder(
                                    &wgpu::CommandEncoderDescriptor {
                                        label: Some("frame_encoder"),
                                    },
                                );

                            gpu.renderer.render(
                                &gpu.device,
                                &gpu.queue,
                                &view,
                                &mut encoder,
                                &draw_cmds,
                                (gpu.size.width, gpu.size.height),
                                &mut text_system,
                            );

                            gpu.queue.submit(std::iter::once(encoder.finish()));
                            frame.present();
                        }
                        Err(e) => {
                            log::warn!("get_current_texture error: {e}");
                        }
                    }
                }

                self.text_system = Some(text_system);

                // Request a continuous repaint so hover states update.
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }

            _ => {}
        }
    }
}

// ── wgpu init ─────────────────────────────────────────────────────────────────

async fn init_wgpu(window: Arc<Window>) -> GpuState {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    // Safety: the surface must not outlive the window. We tie the lifetimes
    // via Arc so the window stays alive at least as long as the surface.
    let surface = instance
        .create_surface(Arc::clone(&window))
        .expect("failed to create surface");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference:       wgpu::PowerPreference::default(),
            compatible_surface:     Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("no suitable wgpu adapter found");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label:             Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits:   wgpu::Limits::default(),
                memory_hints:      Default::default(),
            },
            None, // pipeline cache path
        )
        .await
        .expect("failed to create device");

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_fmt  = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage:        wgpu::TextureUsages::RENDER_ATTACHMENT,
        format:       surface_fmt,
        width:        size.width,
        height:       size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode:   surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let renderer = Renderer::new(&device, surface_fmt);

    GpuState { surface, device, queue, config, renderer, size }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("event loop error");
}
