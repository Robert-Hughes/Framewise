mod renderer;
mod spec_page;
mod text;

use framewise::{
    input::Input,
    layout::{Layout, LayoutState, OffsetLayout, OffsetState, RowLayout, ColumnLayout, ManualLayout},
    theme::Theme,
    types::{Color, Rect, Vec2},
    widget::WidgetContext,
    DrawCmd,
    text::TextSystem,
    widgets::button::{button, ButtonState, ButtonSpec, ButtonStyle, ButtonInfo},
    widgets::label::{label, LabelSpec, LabelInfo},
    widgets::frame::{frame, FrameSpec, FrameStyle, FrameInfo},
    widgets::slider::{slider, SliderState, SliderSpec, Orientation as SliderOrientation, SliderStyle},
    widgets::text_edit::{text_edit, TextEditState, TextEditSpec, TextEditInfo},
    widgets::scroll_area::{begin_scroll_area, end_scroll_area, ScrollState, ScrollbarVisibility, ScrollAreaScope},
};

fn child_ctx<'a, 'b, T: TextSystem, S: LayoutState, L: Layout>(
    parent: &'b mut WidgetContext<'a, T, S>,
    layout_params: S::Params,
    layout: L,
) -> WidgetContext<'b, T, L::State> {
    let bounds = parent.layout(layout_params);
    let mut child = WidgetContext::new(
        parent.theme.clone(),
        parent.text_system,
        parent.focus_sys,
        layout.begin(bounds),
    );
    child.bg_color = parent.bg_color;
    child.accent_color = parent.accent_color;
    child.text_color = parent.text_color;
    child.border_color = parent.border_color;
    child.button_style = parent.button_style;
    child.frame_style = parent.frame_style;
    child.text_size = parent.text_size;
    child.text_font = parent.text_font;
    child.time = parent.time;
    child.clip_rect = parent.clip_rect;
    child
}

fn scroll_area_ctx<'a, 'b, T: TextSystem, S: LayoutState, L: Layout>(
    parent: &'b mut WidgetContext<'a, T, S>,
    layout_params: S::Params,
    content_size: Vec2,
    h_vis: ScrollbarVisibility,
    v_vis: ScrollbarVisibility,
    state: &mut ScrollState,
    inner_layout: L,
    input: &Input,
) -> (WidgetContext<'b, T, OffsetState<L::State>>, ScrollAreaScope) {
    let bounds = parent.layout(layout_params);
    let (scope, content_bounds, offset) = begin_scroll_area(
        parent,
        bounds,
        content_size,
        h_vis,
        v_vis,
        state,
        input,
        parent.clip_rect,
        parent.time,
    );
    
    let mut child = WidgetContext::new(
        parent.theme.clone(),
        parent.text_system,
        parent.focus_sys,
        OffsetLayout { offset, inner: inner_layout }.begin(content_bounds),
    );
    child.bg_color = parent.bg_color;
    child.accent_color = parent.accent_color;
    child.text_color = parent.text_color;
    child.border_color = parent.border_color;
    child.button_style = parent.button_style;
    child.frame_style = parent.frame_style;
    child.text_size = parent.text_size;
    child.text_font = parent.text_font;
    child.time = parent.time;
    child.clip_rect = Some(parent.clip_rect.map_or(content_bounds, |pc| pc.intersect(&content_bounds)));
    (child, scope)
}

fn end_scroll_area_ctx<'a, T: TextSystem, S: LayoutState>(
    parent: &mut WidgetContext<'a, T, S>,
    cmds: Vec<DrawCmd>,
    scope: ScrollAreaScope,
) {
    parent.append_cmds(cmds);
    end_scroll_area(parent, scope);
}

fn label_widget<T: TextSystem, S: LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    layout_params: S::Params,
    text: &str,
) -> LabelInfo {
    let rect = ctx.layout(layout_params);
    let spec = LabelSpec {
        rect,
        text: text.to_string(),
        size: ctx.text_size,
        font: ctx.text_font,
        text_color: ctx.text_color,
        rule: false,
    };
    label(ctx, spec)
}

fn button_widget<T: TextSystem, S: LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: ButtonState,
    layout_params: S::Params,
    text: String,
    input: &Input,
) -> ButtonInfo {
    let rect = ctx.layout(layout_params);
    let spec = ButtonSpec {
        rect,
        text,
        style: ctx.button_style,
        clip_rect: ctx.clip_rect,
        disabled: false,
    };
    button(ctx, state, spec, input)
}

fn text_edit_widget<T: TextSystem, S: LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: TextEditState,
    layout_params: S::Params,
    input: &Input,
) -> TextEditInfo {
    let rect = ctx.layout(layout_params);
    let spec = TextEditSpec {
        rect,
        style: ctx.theme.text_edit_style(),
        clip_rect: ctx.clip_rect,
        error: false,
        disabled: false,
    };
    text_edit(ctx, state, spec, input, ctx.time)
}

fn slider_widget<T: TextSystem, S: LayoutState>(
    ctx: &mut WidgetContext<T, S>,
    state: &mut SliderState,
    value: &mut f32,
    min: f32,
    max: f32,
    step: f32,
    orientation: SliderOrientation,
    layout_params: S::Params,
    input: &Input,
) {
    let rect = ctx.layout(layout_params);
    let spec = SliderSpec {
        orientation,
        rect,
        min,
        max,
        page_step: step,
        step,
        thumb_size_ratio: None,
        style: SliderStyle::default(),
        clip_rect: ctx.clip_rect,
        claim_scroll_at_ends: true,
    };
    slider(ctx, state, value, spec, input);
}
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
            let cmds = spec_page::draw_spec_page(
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
        let mut ctx = WidgetContext::new(
            Theme::default(),
            text_system,
            &mut self.focus_sys,
            framewise::layout::ManualLayout.begin(Rect::new(0.0, 0.0, win_size.0, win_size.1)),
        );
        ctx.text_color = Color::WHITE;
        ctx.bg_color = Color::from_srgb_f32(0.05, 0.15, 0.30, 1.0);
        ctx.time = self.start_time.elapsed().as_secs_f64();

        // Background frame covering the whole window.
        let root_frame_spec = FrameSpec {
            rect: Rect::new(0.0, 0.0, win_size.0, win_size.1),
            style: ctx.frame_style,
        };
        frame(&mut ctx, root_frame_spec);

        // Main container splitting into Sidebar (Left) and Content (Right)
        let root_cmds = {
            let mut main_row = child_ctx(
                &mut ctx,
                Rect::new(10.0, 10.0, win_size.0 - 20.0, win_size.1 - 20.0),
                framewise::layout::RowLayout { spacing: 10.0 },
            );

            // -- SIDEBAR (Left Column) --
            let sidebar_cmds = {
                let mut sidebar_col = child_ctx(
                    &mut main_row,
                    Vec2::new(200.0, win_size.1 - 20.0),
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                );
                sidebar_col.button_style.background = Color::from_srgb_f32(0.60, 0.10, 0.80, 1.0);
                sidebar_col.button_style.hovered = Color::from_srgb_f32(0.70, 0.20, 0.90, 1.0);
                sidebar_col.button_style.pressed = Color::from_srgb_f32(0.50, 0.05, 0.70, 1.0);

                label_widget(&mut sidebar_col, Vec2::new(200.0, 20.0), "NAVIGATION");

                let content_height = 20.0 * 32.0 + 20.0 * 8.0; // 20 buttons * 32h + 8 spacing
                let (mut sidebar_scroll, scope) = scroll_area_ctx(
                    &mut sidebar_col,
                    Vec2::new(200.0, win_size.1 - 60.0),
                    Vec2::new(200.0, content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.sidebar_scroll,
                    framewise::layout::ColumnLayout { spacing: 8.0 },
                    &self.input,
                );

                for i in 0..20 {
                    let shade = (i % 2) as f32 * 0.15;
                    sidebar_scroll.button_style.background = Color::from_srgb_f32(0.60 + shade, 0.10 + shade, 0.80 + shade, 1.0);
                    let btn = button_widget(
                        &mut sidebar_scroll,
                        std::mem::take(&mut self.sidebar_btns[i].state),
                        Vec2::new(180.0, 32.0),
                        format!("Menu Item {}", i + 1),
                        &self.input,
                    );
                    let clicked = btn.clicked();
                    self.sidebar_btns[i].state = btn.state;
                    if clicked { self.sidebar_btns[i].clicks += 1; }
                }
                let sidebar_cmds = sidebar_scroll.finish();
                end_scroll_area_ctx(&mut sidebar_col, sidebar_cmds, scope);

                sidebar_col.finish()
            };
            main_row.append_cmds(sidebar_cmds);

            // -- MAIN CONTENT (Right Column) --
            {
                let (mut content_col, content_scope) = scroll_area_ctx(
                    &mut main_row,
                    Vec2::new(win_size.0 - 240.0, win_size.1 - 20.0),
                    Vec2::new(win_size.0 - 240.0, 2000.0),
                    ScrollbarVisibility::None,
                    ScrollbarVisibility::Always,
                    &mut self.right_panel_scroll,
                    framewise::layout::ColumnLayout { spacing: 15.0 },
                    &self.input,
                );
                let inner_w = win_size.0 - 240.0 - 15.0;

                // Top Header Row
                let header_cmds = {
                    let mut header_row = child_ctx(
                        &mut content_col,
                        Vec2::new(inner_w, 40.0),
                        framewise::layout::RowLayout { spacing: 10.0 },
                    );
                    header_row.button_style.background = Color::from_srgb_f32(0.90, 0.40, 0.10, 1.0);
                    header_row.button_style.hovered = Color::from_srgb_f32(1.00, 0.50, 0.20, 1.0);
                    header_row.button_style.pressed = Color::from_srgb_f32(0.80, 0.30, 0.00, 1.0);

                    let info = text_edit_widget(
                        &mut header_row,
                        std::mem::take(&mut self.text_edit_state),
                        Vec2::new(300.0, 40.0),
                        &self.input,
                    );
                    self.text_edit_state = info.state;

                    if let Some(action) = info.clipboard_action {
                        if let Some(cb) = &mut self.clipboard {
                            match action {
                                framewise::widgets::text_edit::ClipboardAction::Copy(text) => drop(cb.set_text(text)),
                                framewise::widgets::text_edit::ClipboardAction::Cut(text) => drop(cb.set_text(text)),
                            }
                        }
                    }

                    let btn1 = button_widget(
                        &mut header_row,
                        std::mem::take(&mut self.top_btn1.state),
                        Vec2::new(100.0, 40.0),
                        "Profile".to_string(),
                        &self.input,
                    );
                    self.top_btn1.state = btn1.state;

                    let btn2 = button_widget(
                        &mut header_row,
                        std::mem::take(&mut self.top_btn2.state),
                        Vec2::new(100.0, 40.0),
                        "Settings".to_string(),
                        &self.input,
                    );
                    self.top_btn2.state = btn2.state;

                    header_row.finish()
                };
                content_col.append_cmds(header_cmds);

                // Nested Grid Area (4 Rows of 4 Buttons)
                let grid_cmds = {
                    let mut grid_col = child_ctx(
                        &mut content_col,
                        Vec2::new(inner_w, 200.0),
                        framewise::layout::ColumnLayout { spacing: 10.0 },
                    );
                    grid_col.button_style.background = Color::from_srgb_f32(0.00, 0.60, 0.70, 1.0);
                    grid_col.button_style.hovered = Color::from_srgb_f32(0.10, 0.70, 0.80, 1.0);
                    grid_col.button_style.pressed = Color::from_srgb_f32(0.00, 0.50, 0.60, 1.0);

                    label_widget(&mut grid_col, Vec2::new(400.0, 20.0), "DASHBOARD GRID");

                    for row in 0..4 {
                        let row_cmds = {
                            let mut grid_row = child_ctx(
                                &mut grid_col,
                                Vec2::new(inner_w, 32.0),
                                framewise::layout::RowLayout { spacing: 10.0 },
                            );
                            for col in 0..4 {
                                let idx = row * 4 + col;
                                let shade = ((row + col) % 2) as f32 * 0.15;
                                grid_row.button_style.background = Color::from_srgb_f32(0.00 + shade, 0.60 + shade, 0.70 + shade, 1.0);
                                let btn = button_widget(
                                    &mut grid_row,
                                    std::mem::take(&mut self.grid_btns[idx].state),
                                    Vec2::new(120.0, 32.0),
                                    format!("Grid [{},{}]", row, col),
                                    &self.input,
                                );
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
                    let mut slider_row = child_ctx(
                        &mut content_col,
                        Vec2::new(inner_w, 100.0),
                        framewise::layout::RowLayout { spacing: 20.0 },
                    );

                    label_widget(&mut slider_row, Vec2::new(150.0, 20.0), &format!("Slider Value: {:.1}", self.standalone_slider_val));

                    slider_widget(
                        &mut slider_row,
                        &mut self.standalone_slider_state,
                        &mut self.standalone_slider_val,
                        0.0,
                        100.0,
                        20.0,
                        SliderOrientation::Vertical,
                        Vec2::new(30.0, 100.0),
                        &self.input,
                    );

                    slider_row.finish()
                };
                content_col.append_cmds(slider_cmds);

                // Main Scroll Area
                label_widget(&mut content_col, Vec2::new(400.0, 20.0), "MAIN FEED");
                let content_height = 30.0 * 50.0 + 30.0 * 10.0;
                let (mut main_scroll, scope) = scroll_area_ctx(
                    &mut content_col,
                    Vec2::new(inner_w, 250.0),
                    Vec2::new(inner_w, content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.main_scroll,
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                    &self.input,
                );
                main_scroll.button_style.background = Color::from_srgb_f32(0.80, 0.20, 0.20, 1.0);
                main_scroll.button_style.hovered = Color::from_srgb_f32(0.90, 0.30, 0.30, 1.0);
                main_scroll.button_style.pressed = Color::from_srgb_f32(0.70, 0.10, 0.10, 1.0);

                for i in 0..30 {
                    let shade = (i % 2) as f32 * 0.15;
                    main_scroll.button_style.background = Color::from_srgb_f32(0.80 + shade, 0.20 + shade, 0.20 + shade, 1.0);
                    let btn = button_widget(
                        &mut main_scroll,
                        std::mem::take(&mut self.main_btns[i].state),
                        Vec2::new(win_size.0 - 280.0, 50.0),
                        format!("Feed Item #{} - Very Important Notification", i + 1),
                        &self.input,
                    );
                    let clicked = btn.clicked();
                    self.main_btns[i].state = btn.state;
                    if clicked { self.main_btns[i].clicks += 1; }
                }
                let main_cmds = main_scroll.finish();
                end_scroll_area_ctx(&mut content_col, main_cmds, scope);

                // Nested Scroll Area Demo
                label_widget(
                    &mut content_col,
                    Vec2::new(400.0, 20.0),
                    "NESTED SCROLL DEMO  |  Inner area: wheel propagates to outer at ends  |  Slider: always blocks",
                );

                let row_h = 160.0;
                let outer_content_height = 3.0 * row_h + 2.0 * 10.0;
                let (mut outer_scroll, outer_scope) = scroll_area_ctx(
                    &mut content_col,
                    Vec2::new(inner_w, 300.0),
                    Vec2::new(800.0, outer_content_height),
                    ScrollbarVisibility::Auto,
                    ScrollbarVisibility::Auto,
                    &mut self.nested_outer_scroll,
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                    &self.input,
                );

                for i in 0..3 {
                    let row_state = &mut self.nested_rows[i];

                    let mut row_builder = child_ctx(
                        &mut outer_scroll,
                        Vec2::new(800.0, row_h),
                        framewise::layout::RowLayout { spacing: 10.0 }
                    );
                    let (base_r, base_g, base_b) = match i {
                        0 => (0.40, 0.80, 0.10), // Lime green
                        1 => (0.90, 0.20, 0.60), // Hot pink
                        _ => (0.10, 0.50, 0.90), // Vivid blue
                    };
                    row_builder.button_style.background = Color::from_srgb_f32(base_r, base_g, base_b, 1.0);
                    row_builder.button_style.hovered = Color::from_srgb_f32(base_r + 0.1, base_g + 0.1, base_b + 0.1, 1.0);
                    row_builder.button_style.pressed = Color::from_srgb_f32(base_r - 0.1, base_g - 0.1, base_b - 0.1, 1.0);

                    // Left button
                    let btn1 = button_widget(
                        &mut row_builder,
                        std::mem::take(&mut row_state.btn1.state),
                        Vec2::new(80.0, row_h),
                        format!("R{} A", i + 1),
                        &self.input,
                    );
                    let clicked1 = btn1.clicked();
                    row_state.btn1.state = btn1.state;
                    if clicked1 { row_state.btn1.clicks += 1; }

                    // 1. Vertical Inner scroll area
                    let inner_content_height = 6.0 * 45.0 + 5.0 * 8.0;
                    let (mut inner_scroll, inner_scope) = scroll_area_ctx(
                        &mut row_builder,
                        Vec2::new(120.0, row_h),
                        Vec2::new(120.0, inner_content_height),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Auto,
                        &mut row_state.inner_scroll,
                        framewise::layout::ColumnLayout { spacing: 8.0 },
                        &self.input,
                    );

                    for j in 0..6 {
                        let shade = (j % 2) as f32 * 0.15;
                        inner_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                        let btn = button_widget(
                            &mut inner_scroll,
                            std::mem::take(&mut row_state.inner_btns[j].state),
                            Vec2::new(100.0, 45.0),
                            format!("V {}", j + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        row_state.inner_btns[j].state = btn.state;
                        if clicked { row_state.inner_btns[j].clicks += 1; }
                    }
                    let inner_cmds = inner_scroll.finish();
                    end_scroll_area_ctx(&mut row_builder, inner_cmds, inner_scope);

                    // 2. Horizontal Inner scroll area (using None for vertical scrollbar)
                    let horiz_content_width = 10.0 * 80.0 + 9.0 * 8.0;
                    let (mut horiz_scroll, horiz_scope) = scroll_area_ctx(
                        &mut row_builder,
                        Vec2::new(180.0, row_h),
                        Vec2::new(horiz_content_width, row_h),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut row_state.horiz_scroll,
                        framewise::layout::RowLayout { spacing: 8.0 },
                        &self.input,
                    );

                    for j in 0..10 {
                        let shade = (j % 2) as f32 * 0.15;
                        horiz_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);
                        let btn = button_widget(
                            &mut horiz_scroll,
                            std::mem::take(&mut row_state.horiz_btns[j].state),
                            Vec2::new(80.0, row_h - 25.0), // make room for scrollbar
                            format!("H {}", j + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        row_state.horiz_btns[j].state = btn.state;
                        if clicked { row_state.horiz_btns[j].clicks += 1; }
                    }
                    let horiz_cmds = horiz_scroll.finish();
                    end_scroll_area_ctx(&mut row_builder, horiz_cmds, horiz_scope);

                    // 3. Both directions Inner scroll area
                    let both_width = 8.0 * 80.0 + 7.0 * 8.0;
                    let both_height = 6.0 * 45.0 + 5.0 * 8.0;
                    let (mut both_scroll, both_scope) = scroll_area_ctx(
                        &mut row_builder,
                        Vec2::new(200.0, row_h),
                        Vec2::new(both_width, both_height),
                        ScrollbarVisibility::Auto,
                        ScrollbarVisibility::Auto,
                        &mut row_state.both_scroll,
                        framewise::layout::ManualLayout,
                        &self.input,
                    );

                    for j in 0..48 {
                        let x = (j % 8) as f32 * 88.0;
                        let y = (j / 8) as f32 * 53.0;
                        let shade = ((j % 8 + j / 8) % 2) as f32 * 0.15;
                        both_scroll.button_style.background = Color::from_srgb_f32(base_r + shade, base_g + shade, base_b + shade, 1.0);

                        let btn = button_widget(
                            &mut both_scroll,
                            std::mem::take(&mut row_state.both_btns[j].state),
                            Rect::new(x, y, 80.0, 45.0),
                            format!("2D {}", j + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        row_state.both_btns[j].state = btn.state;
                        if clicked { row_state.both_btns[j].clicks += 1; }
                    }
                    let both_cmds = both_scroll.finish();
                    end_scroll_area_ctx(&mut row_builder, both_cmds, both_scope);

                    // Standalone vertical slider
                    slider_widget(
                        &mut row_builder,
                        &mut row_state.slider_state,
                        &mut row_state.slider_val,
                        0.0, 100.0, 20.0,
                        SliderOrientation::Vertical,
                        Vec2::new(30.0, row_h),
                        &self.input,
                    );

                    // Standalone horizontal slider
                    slider_widget(
                        &mut row_builder,
                        &mut row_state.horiz_slider_state,
                        &mut row_state.horiz_slider_val,
                        0.0, 100.0, 20.0,
                        SliderOrientation::Horizontal,
                        Vec2::new(100.0, 30.0),
                        &self.input,
                    );

                    let row_cmds = row_builder.finish();
                    outer_scroll.append_cmds(row_cmds);
                }
                let outer_cmds = outer_scroll.finish();
                end_scroll_area_ctx(&mut content_col, outer_cmds, outer_scope);

                // Double Horizontal Scroll Demo
                label_widget(&mut content_col, Vec2::new(400.0, 20.0), "DOUBLE HORIZONTAL SCROLL DEMO");
                let (mut d_outer_scroll, d_outer_scope) = scroll_area_ctx(
                    &mut content_col,
                    Vec2::new(inner_w, 150.0),
                    Vec2::new(2000.0, 150.0),
                    ScrollbarVisibility::Always,
                    ScrollbarVisibility::None,
                    &mut self.double_horiz_outer_scroll,
                    framewise::layout::RowLayout { spacing: 20.0 },
                    &self.input,
                );

                // Left spacer/button
                button_widget(&mut d_outer_scroll, Default::default(), Vec2::new(100.0, 100.0), "Outer L".to_string(), &self.input);

                // Inner horizontal scroll area
                let (mut d_inner_scroll, d_inner_scope) = scroll_area_ctx(
                    &mut d_outer_scroll,
                    Vec2::new(600.0, 120.0),
                    Vec2::new(20.0 * 60.0 + 19.0 * 8.0, 120.0),
                    ScrollbarVisibility::Always,
                    ScrollbarVisibility::None,
                    &mut self.double_horiz_inner_scroll,
                    framewise::layout::RowLayout { spacing: 8.0 },
                    &self.input,
                );

                for j in 0..20 {
                    let btn = button_widget(
                        &mut d_inner_scroll,
                        std::mem::take(&mut self.double_horiz_btns[j].state),
                        Vec2::new(60.0, 80.0),
                        format!("H {}", j + 1),
                        &self.input,
                    );
                    self.double_horiz_btns[j].state = btn.state;
                }
                let d_inner_cmds = d_inner_scroll.finish();
                end_scroll_area_ctx(&mut d_outer_scroll, d_inner_cmds, d_inner_scope);

                // Right spacer/button
                button_widget(&mut d_outer_scroll, Default::default(), Vec2::new(300.0, 100.0), "Outer R".to_string(), &self.input);

                let d_outer_cmds = d_outer_scroll.finish();
                end_scroll_area_ctx(&mut content_col, d_outer_cmds, d_outer_scope);

                // Nested 2D Scroll Demo: outer[2D] > inner[2D]
                {
                    let outer_ox = self.nested_2d_outer_scroll.offset.x;
                    let outer_oy = self.nested_2d_outer_scroll.offset.y;
                    let inner_ox = self.nested_2d_inner_scroll.offset.x;
                    let inner_oy = self.nested_2d_inner_scroll.offset.y;

                    label_widget(
                        &mut content_col,
                        Vec2::new(inner_w, 20.0),
                        "NESTED 2D SCROLL  |  outer[H+V] > inner[H+V]  |  Each axis bubbles independently",
                    );

                    // Outer 2D: viewport 420x200, content 840x400
                    let (mut outer, outer_scope) = scroll_area_ctx(
                        &mut content_col,
                        Vec2::new(inner_w.min(440.0), 200.0),
                        Vec2::new(840.0, 400.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::Always,
                        &mut self.nested_2d_outer_scroll,
                        framewise::layout::ManualLayout,
                        &self.input,
                    );

                    // Status label at top-left of outer content
                    label_widget(
                        &mut outer,
                        Rect::new(0.0, 0.0, 400.0, 18.0),
                        &format!("OUTER x:{:.0} y:{:.0}  |  INNER x:{:.0} y:{:.0}", outer_ox, outer_oy, inner_ox, inner_oy),
                    );

                    // Some outer-only buttons scattered in the far corners to make outer scrollable
                    for (k, (bx, by, label)) in [
                        (10.0,  30.0, "OA"),
                        (700.0, 30.0, "OB"),
                        (10.0,  340.0, "OC"),
                        (700.0, 340.0, "OD"),
                        (400.0, 180.0, "OE"),
                        (550.0, 100.0, "OF"),
                    ].iter().enumerate() {
                        let btn = button_widget(
                            &mut outer,
                            std::mem::take(&mut self.nested_2d_outer_btns[k].state),
                            Rect::new(*bx, *by, 60.0, 28.0),
                            label.to_string(),
                            &self.input,
                        );
                        self.nested_2d_outer_btns[k].state = btn.state;
                    }

                    // Inner 2D: viewport 250x150, content 500x300 — 4x5 button grid
                    let (mut inner, inner_scope) = scroll_area_ctx(
                        &mut outer,
                        Rect::new(80.0, 50.0, 250.0, 150.0),
                        Vec2::new(500.0, 300.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::Always,
                        &mut self.nested_2d_inner_scroll,
                        framewise::layout::ManualLayout,
                        &self.input,
                    );

                    for j in 0..20 {
                        let col = j % 4;
                        let row = j / 4;
                        let shade = ((col + row) % 2) as f32 * 0.12;
                        inner.button_style.background = Color::from_srgb_f32(0.10 + shade, 0.35 + shade, 0.70 + shade, 1.0);
                        inner.button_style.hovered    = Color::from_srgb_f32(0.20 + shade, 0.45 + shade, 0.80 + shade, 1.0);
                        let btn = button_widget(
                            &mut inner,
                            std::mem::take(&mut self.nested_2d_inner_btns[j].state),
                            Rect::new(col as f32 * 120.0 + 5.0, row as f32 * 58.0 + 5.0, 110.0, 48.0),
                            format!("2D {:02}", j + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        self.nested_2d_inner_btns[j].state = btn.state;
                        if clicked { self.nested_2d_inner_btns[j].clicks += 1; }
                    }
                    let inner_cmds = inner.finish();
                    end_scroll_area_ctx(&mut outer, inner_cmds, inner_scope);

                    let outer_cmds = outer.finish();
                    end_scroll_area_ctx(&mut content_col, outer_cmds, outer_scope);
                }

                // Triple-Nested Scroll Demo: outer_vert -> middle_horiz -> inner_vert
                {
                    let outer_y = self.triple_outer_scroll.offset.y;
                    let middle_x = self.triple_middle_scroll.offset.x;
                    let inner_y = self.triple_inner_scroll.offset.y;
                    let innermost_x = self.triple_innermost_scroll.offset.x;

                    label_widget(
                        &mut content_col,
                        Vec2::new(inner_w, 20.0),
                        "QUAD NESTED: outer[vert] > middle[horiz] > inner[vert] > innermost[horiz]  |  Explore cross-axis isolation",
                    );

                    // Outer vertical scroll area (taller content)
                    let (mut outer_scroll, outer_scope) = scroll_area_ctx(
                        &mut content_col,
                        Vec2::new(inner_w, 220.0),
                        Vec2::new(inner_w, 500.0),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut self.triple_outer_scroll,
                        framewise::layout::ColumnLayout { spacing: 10.0 },
                        &self.input,
                    );

                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), &format!(
                        "OUTER[V]: {:.0}  |  MIDDLE[H]: {:.0}  |  INNER[V]: {:.0}  |  INNERMOST[H]: {:.0}",
                        outer_y, middle_x, inner_y, innermost_x,
                    ));

                    // Middle horizontal scroll area inside outer vertical
                    let (mut middle_scroll, middle_scope) = scroll_area_ctx(
                        &mut outer_scroll,
                        Vec2::new(inner_w - 15.0, 160.0),
                        Vec2::new(1400.0, 160.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut self.triple_middle_scroll,
                        framewise::layout::RowLayout { spacing: 10.0 },
                        &self.input,
                    );

                    label_widget(&mut middle_scroll, Vec2::new(200.0, 130.0), "[ horiz padding ]");

                    // Inner vertical scroll area inside middle horizontal
                    let inner_content_h = 12.0 * 35.0 + 50.0 + 12.0 * 6.0;
                    let (mut inner_scroll, inner_scope) = scroll_area_ctx(
                        &mut middle_scroll,
                        Vec2::new(200.0, 130.0),
                        Vec2::new(200.0, inner_content_h),
                        ScrollbarVisibility::None,
                        ScrollbarVisibility::Always,
                        &mut self.triple_inner_scroll,
                        framewise::layout::ColumnLayout { spacing: 6.0 },
                        &self.input,
                    );

                    for j in 0..12 {
                        let shade = (j % 2) as f32 * 0.12;
                        inner_scroll.button_style.background = Color::from_srgb_f32(0.10 + shade, 0.50 + shade, 0.30 + shade, 1.0);
                        inner_scroll.button_style.hovered = Color::from_srgb_f32(0.20 + shade, 0.60 + shade, 0.40 + shade, 1.0);
                        let btn = button_widget(
                            &mut inner_scroll,
                            std::mem::take(&mut self.triple_inner_btns[j].state),
                            Vec2::new(165.0, 35.0),
                            format!("Inner V {}", j + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        self.triple_inner_btns[j].state = btn.state;
                        if clicked { self.triple_inner_btns[j].clicks += 1; }
                    }

                    // Innermost horizontal scroll area — 4th nesting level
                    let innermost_content_w = 5.0 * 80.0 + 4.0 * 6.0;
                    let (mut innermost_scroll, innermost_scope) = scroll_area_ctx(
                        &mut inner_scroll,
                        Vec2::new(165.0, 50.0),
                        Vec2::new(innermost_content_w, 50.0),
                        ScrollbarVisibility::Always,
                        ScrollbarVisibility::None,
                        &mut self.triple_innermost_scroll,
                        framewise::layout::RowLayout { spacing: 6.0 },
                        &self.input,
                    );
                    for k in 0..5 {
                        innermost_scroll.button_style.background = Color::from_srgb_f32(0.60, 0.25 + k as f32 * 0.06, 0.10, 1.0);
                        innermost_scroll.button_style.hovered    = Color::from_srgb_f32(0.70, 0.35 + k as f32 * 0.06, 0.20, 1.0);
                        let btn = button_widget(
                            &mut innermost_scroll,
                            std::mem::take(&mut self.triple_innermost_btns[k].state),
                            Vec2::new(80.0, 26.0),
                            format!("IH {}", k + 1),
                            &self.input,
                        );
                        let clicked = btn.clicked();
                        self.triple_innermost_btns[k].state = btn.state;
                        if clicked { self.triple_innermost_btns[k].clicks += 1; }
                    }
                    let innermost_cmds = innermost_scroll.finish();
                    end_scroll_area_ctx(&mut inner_scroll, innermost_cmds, innermost_scope);

                    let inner_cmds = inner_scroll.finish();
                    end_scroll_area_ctx(&mut middle_scroll, inner_cmds, inner_scope);

                    // Inner vertical slider
                    slider_widget(
                        &mut middle_scroll,
                        &mut self.triple_inner_slider_state,
                        &mut self.triple_inner_slider_val,
                        0.0, 100.0, 20.0,
                        SliderOrientation::Vertical,
                        Vec2::new(30.0, 130.0),
                        &self.input,
                    );

                    label_widget(&mut middle_scroll, Vec2::new(200.0, 130.0), "[ horiz padding ]");

                    let middle_cmds = middle_scroll.finish();
                    end_scroll_area_ctx(&mut outer_scroll, middle_cmds, middle_scope);

                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), "[ outer vert padding row ]");
                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), "[ outer vert padding row ]");
                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), "[ outer vert padding row ]");
                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), "[ outer vert padding row ]");
                    label_widget(&mut outer_scroll, Vec2::new(inner_w - 15.0, 20.0), "[ outer vert padding row ]");

                    let outer_cmds = outer_scroll.finish();
                    end_scroll_area_ctx(&mut content_col, outer_cmds, outer_scope);
                }

                let content_cmds = content_col.finish();
                end_scroll_area_ctx(&mut main_row, content_cmds, content_scope);
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
