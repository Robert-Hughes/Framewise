mod renderer;

use framewise::{Builder, BuilderCtx, Input, Rect, Vec2};
use renderer::Renderer;
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

struct App {
    window:    Option<Arc<Window>>,
    gpu:       Option<GpuState>,
    input:     Input,
    /// Track whether the left button was *just* released this frame.
    btn_was_down: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window:       None,
            gpu:          None,
            input:        Input::new(),
            btn_was_down: false,
        }
    }

    fn draw_ui(&self) -> Vec<framewise::DrawCmd> {
        let ctx = BuilderCtx::default();
        let mut ui = Builder::new(ctx);

        let win_size = self
            .gpu
            .as_ref()
            .map(|g| (g.size.width as f32, g.size.height as f32))
            .unwrap_or((800.0, 600.0));

        // Background frame covering the whole window.
        let _root = ui.frame(Rect::new(0.0, 0.0, win_size.0, win_size.1));

        // Button 1 ─────────────────────────────────────────────────────────
        let btn1 = ui.button(
            Rect::new(24.0, 24.0, 140.0, 40.0),
            "Button One",
            &self.input,
        );
        if btn1.clicked() {
            println!("[sample] Button One clicked");
        }

        // Button 2 ─────────────────────────────────────────────────────────
        let btn2 = ui.button(
            Rect::new(24.0, 76.0, 140.0, 40.0),
            "Button Two",
            &self.input,
        );
        if btn2.clicked() {
            println!("[sample] Button Two clicked");
        }

        // Label stub next to button 1 ──────────────────────────────────────
        let _lbl = ui.label(
            Rect::new(
                btn1.layout.bounds.right() + 16.0,
                btn1.layout.bounds.y,
                220.0,
                btn1.layout.bounds.h,
            ),
            "A label placeholder",
        );

        // Inner framed panel ───────────────────────────────────────────────
        let panel = ui.frame(Rect::new(24.0, 136.0, 360.0, 120.0));
        let content = panel.content_rect();

        // A button inside the panel.
        let btn3 = ui.button(
            Rect::new(content.x, content.y, 120.0, 32.0),
            "Panel button",
            &self.input,
        );
        if btn3.clicked() {
            println!("[sample] Panel button clicked");
        }

        ui.finish()
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

            WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                match state {
                    ElementState::Pressed => {
                        self.input.mouse_down    = true;
                        self.input.mouse_clicked = false;
                        self.btn_was_down        = true;
                    }
                    ElementState::Released => {
                        self.input.mouse_down    = false;
                        // Only set clicked=true for one frame (cleared in RedrawRequested).
                        self.input.mouse_clicked = true;
                        self.btn_was_down        = false;
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                // Build UI and render.
                let cmds = self.draw_ui();

                // Clear the one-frame clicked flag after UI has consumed it.
                self.input.mouse_clicked = false;

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
                                &cmds,
                                (gpu.size.width, gpu.size.height),
                            );

                            gpu.queue.submit(std::iter::once(encoder.finish()));
                            frame.present();
                        }
                        Err(e) => {
                            log::warn!("get_current_texture error: {e}");
                        }
                    }
                }

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
