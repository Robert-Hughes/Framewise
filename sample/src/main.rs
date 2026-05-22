mod renderer;
mod text;

use framewise::{
    builder::{Builder, BuilderCtx},
    draw::DrawCmd,
    input::{Input, TextEvent},
    layout::Layout,
    types::{Color, Rect, Vec2},
};
use renderer::Renderer;
use text::SampleTextSystem;
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::{Window, WindowId},
};

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

#[derive(Clone, Copy)]
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
    sidebar_btns:    [SampleButton; 20],
    main_btns:       [SampleButton; 30],
    grid_btns:       [SampleButton; 16],
    top_btn1:        SampleButton,
    top_btn2:        SampleButton,
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
            sidebar_scroll:  framewise::widgets::scroll_area::ScrollState::default(),
            main_scroll:     framewise::widgets::scroll_area::ScrollState::default(),
            sidebar_btns:    [SampleButton::default(); 20],
            main_btns:       [SampleButton::default(); 30],
            grid_btns:       [SampleButton::default(); 16],
            top_btn1:        SampleButton::default(),
            top_btn2:        SampleButton::default(),
        }
    }

    fn draw_ui(&mut self, text_system: &mut SampleTextSystem) -> Vec<framewise::DrawCmd> {
        self.focus_sys.begin_frame();
        let ctx = BuilderCtx {
            text_color: Color::rgb(0.9, 0.9, 0.95),
            time: self.start_time.elapsed().as_secs_f64(),
            ..Default::default()
        };
        let win_size = self
            .gpu
            .as_ref()
            .map(|g| (g.size.width as f32, g.size.height as f32))
            .unwrap_or((800.0, 600.0));

        let mut builder = Builder::new(
            ctx,
            text_system,
            &mut self.focus_sys,
            framewise::layout::ManualLayout.begin(Rect::new(0.0, 0.0, win_size.0, win_size.1)),
        );

        // Background frame covering the whole window.
        let _root = builder.frame(Rect::new(0.0, 0.0, win_size.0, win_size.1));

        // Main container splitting into Sidebar (Left) and Content (Right)
        let root_cmds = {
            let mut main_row = builder.child_with_layout(
                Rect::new(10.0, 10.0, win_size.0 - 20.0, win_size.1 - 20.0),
                framewise::layout::RowLayout { spacing: 10.0 },
            );

            // -- SIDEBAR (Left Column) --
            let sidebar_cmds = {
                let mut sidebar_col = main_row.child_with_layout(
                    Vec2::new(200.0, win_size.1 - 20.0),
                    framewise::layout::ColumnLayout { spacing: 10.0 },
                );

                sidebar_col.label(Vec2::new(200.0, 20.0), "NAVIGATION");

                let scroll_cmds = {
                    let content_height = 20.0 * 32.0 + 20.0 * 8.0; // 20 buttons * 32h + 8 spacing
                    let mut sidebar_scroll = sidebar_col.scroll_area(
                        Vec2::new(200.0, win_size.1 - 60.0),
                        content_height,
                        &mut self.sidebar_scroll,
                        framewise::layout::ColumnLayout { spacing: 8.0 },
                        &self.input,
                    );

                    for i in 0..20 {
                        let btn = sidebar_scroll.button(
                            self.sidebar_btns[i].state,
                            Vec2::new(180.0, 32.0),
                            format!("Menu Item {}", i + 1),
                            &self.input,
                        );
                        self.sidebar_btns[i].state = btn.state;
                        if btn.clicked() { self.sidebar_btns[i].clicks += 1; }
                    }
                    sidebar_scroll.finish()
                };
                sidebar_col.append_cmds(scroll_cmds);
                sidebar_col.finish()
            };
            main_row.append_cmds(sidebar_cmds);

            // -- MAIN CONTENT (Right Column) --
            let content_cmds = {
                let mut content_col = main_row.child_with_layout(
                    Vec2::new(win_size.0 - 240.0, win_size.1 - 20.0),
                    framewise::layout::ColumnLayout { spacing: 15.0 },
                );

                // Top Header Row
                let header_cmds = {
                    let mut header_row = content_col.child_with_layout(
                        Vec2::new(win_size.0 - 240.0, 40.0),
                        framewise::layout::RowLayout { spacing: 10.0 },
                    );

                    let (info, te_state) = header_row.text_edit(
                        self.text_edit_state.clone(),
                        Vec2::new(300.0, 40.0),
                        &self.input,
                    );
                    self.text_edit_state = te_state;

                    if let Some(action) = info.clipboard_action {
                        if let Some(cb) = &mut self.clipboard {
                            match action {
                                framewise::widgets::text_edit::ClipboardAction::Copy(text) => drop(cb.set_text(text)),
                                framewise::widgets::text_edit::ClipboardAction::Cut(text) => drop(cb.set_text(text)),
                            }
                        }
                    }

                    let btn1 = header_row.button(self.top_btn1.state, Vec2::new(100.0, 40.0), "Profile", &self.input);
                    self.top_btn1.state = btn1.state;
                    
                    let btn2 = header_row.button(self.top_btn2.state, Vec2::new(100.0, 40.0), "Settings", &self.input);
                    self.top_btn2.state = btn2.state;

                    header_row.finish()
                };
                content_col.append_cmds(header_cmds);

                // Nested Grid Area (4 Rows of 4 Buttons)
                let grid_cmds = {
                    let mut grid_col = content_col.child_with_layout(
                        Vec2::new(win_size.0 - 240.0, 200.0),
                        framewise::layout::ColumnLayout { spacing: 10.0 },
                    );
                    
                    grid_col.label(Vec2::new(400.0, 20.0), "DASHBOARD GRID");

                    for row in 0..4 {
                        let row_cmds = {
                            let mut grid_row = grid_col.child_with_layout(
                                Vec2::new(win_size.0 - 240.0, 32.0),
                                framewise::layout::RowLayout { spacing: 10.0 },
                            );
                            for col in 0..4 {
                                let idx = row * 4 + col;
                                let btn = grid_row.button(
                                    self.grid_btns[idx].state,
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

                // Main Scroll Area
                content_col.label(Vec2::new(400.0, 20.0), "MAIN FEED");
                let scroll_cmds = {
                    let content_height = 30.0 * 50.0 + 30.0 * 10.0; // 30 items * 50h + 10 spacing
                    let mut main_scroll = content_col.scroll_area(
                        Vec2::new(win_size.0 - 240.0, 250.0),
                        content_height,
                        &mut self.main_scroll,
                        framewise::layout::ColumnLayout { spacing: 10.0 },
                        &self.input,
                    );

                    for i in 0..30 {
                        let btn = main_scroll.button(
                            self.main_btns[i].state,
                            Vec2::new(win_size.0 - 280.0, 50.0),
                            format!("Feed Item #{} - Very Important Notification", i + 1),
                            &self.input,
                        );
                        self.main_btns[i].state = btn.state;
                        if btn.clicked() { self.main_btns[i].clicks += 1; }
                    }
                    main_scroll.finish()
                };
                content_col.append_cmds(scroll_cmds);

                content_col.finish()
            };
            main_row.append_cmds(content_cmds);
            
            main_row.finish()
        };
        builder.append_cmds(root_cmds);

        let cmds = builder.finish();
        self.focus_sys.end_frame();
        cmds
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Create the window.
        let attrs = Window::default_attributes()
            .with_title("Framewise Sample")
            .with_inner_size(PhysicalSize::new(800u32, 600u32));

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
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(x, y) => {
                        self.input.scroll_delta = Vec2::new(x, y);
                    }
                    winit::event::MouseScrollDelta::PixelDelta(pos) => {
                        self.input.scroll_delta = Vec2::new(pos.x as f32 / 20.0, pos.y as f32 / 20.0);
                    }
                }
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
            }

            WindowEvent::KeyboardInput { event, .. } => {
                match event.physical_key {
                    winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::Tab) => {
                        if event.state == ElementState::Pressed {
                            let direction = if self.modifiers.shift_key() {
                                framewise::focus::FocusDirection::Prev
                            } else {
                                framewise::focus::FocusDirection::Next
                            };
                            self.focus_sys.request_shift(direction);
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
