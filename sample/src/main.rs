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

#[derive(Default)]
struct SampleButton {
    state: framewise::widgets::button::ButtonState,
    clicks: u32,
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
    btn1:            SampleButton,
    btn2:            SampleButton,
    btn3:            SampleButton,
    clipboard:       Option<arboard::Clipboard>,
    scroll_state:    framewise::widgets::scroll_area::ScrollState,
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
            text_edit_state: framewise::widgets::text_edit::TextEditState::new("Hello, TextEdit!"),
            modifiers:       winit::keyboard::ModifiersState::default(),
            input:           Input::new(),
            btn1:            SampleButton::default(),
            btn2:            SampleButton::default(),
            btn3:            SampleButton::default(),
            clipboard:       arboard::Clipboard::new().ok(),
            scroll_state:    framewise::widgets::scroll_area::ScrollState::default(),
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

        // Nested Layouts Example ───────────────────────────────────────────
        
        let mut col_builder = builder.child_with_layout(
            Rect::new(24.0, 24.0, 400.0, 500.0),
            framewise::layout::ColumnLayout { spacing: 16.0 },
        );

        let btn1 = col_builder.button(
            self.btn1.state,
            Vec2::new(140.0, 40.0),
            format!("Button One ({})", self.btn1.clicks),
            &self.input,
        );
        self.btn1.state = btn1.state;
        if btn1.clicked() {
            self.btn1.clicks += 1;
        }

        // Inner row layout inside the column
        let child_cmds = {
            let mut row_builder = col_builder.child_with_layout(
                Vec2::new(400.0, 40.0),
                framewise::layout::RowLayout { spacing: 10.0 },
            );
            let btn2 = row_builder.button(
                self.btn2.state,
                Vec2::new(140.0, 40.0),
                format!("Button Two ({})", self.btn2.clicks),
                &self.input,
            );
            self.btn2.state = btn2.state;
            if btn2.clicked() {
                self.btn2.clicks += 1;
            }

            let _lbl = row_builder.label(
                Vec2::new(220.0, 40.0),
                "A label in a row layout",
            );
            row_builder.finish()
        };
        col_builder.append_cmds(child_cmds);

        // Text Edit
        let (info, new_te_state) = col_builder.text_edit(
            self.text_edit_state.clone(),
            Vec2::new(300.0, 40.0),
            &self.input,
        );
        self.text_edit_state = new_te_state;

        // Scroll Layout inside the column
        let _scroll_lbl = col_builder.label(Vec2::new(300.0, 20.0), "Scrollable Area:");

        let scroll_cmds = {
            // 10 labels * 30px + 10 spaces * 10px = 400px.
            // 1 button * 32px + 1 space * 10px = 42px.
            // Total content height = 442px.
            let content_height = 442.0;
            
            let mut scroll_builder = col_builder.scroll_area(
                Vec2::new(300.0, 200.0),
                content_height,
                &mut self.scroll_state,
                framewise::layout::ColumnLayout { spacing: 10.0 },
                &self.input,
            );
            
            for i in 0..10 {
                let _ = scroll_builder.label(Vec2::new(280.0, 30.0), &format!("Scrollable item #{}", i));
            }
            
            let btn3 = scroll_builder.button(
                self.btn3.state,
                Vec2::new(120.0, 32.0),
                format!("Scroll Btn ({})", self.btn3.clicks),
                &self.input,
            );
            self.btn3.state = btn3.state;
            if btn3.clicked() {
                self.btn3.clicks += 1;
            }
            scroll_builder.finish()
        };
        col_builder.append_cmds(scroll_cmds);
        let col_cmds = col_builder.finish();
        builder.append_cmds(col_cmds);

        if let Some(action) = info.clipboard_action {
            if let Some(cb) = &mut self.clipboard {
                match action {
                    framewise::widgets::text_edit::ClipboardAction::Copy(text) => drop(cb.set_text(text)),
                    framewise::widgets::text_edit::ClipboardAction::Cut(text) => drop(cb.set_text(text)),
                }
            }
        }

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
