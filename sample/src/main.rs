#[cfg(feature = "page_button_demo")]
mod button_page;
#[cfg(feature = "page_frame_demo")]
mod frame_demo;
#[cfg(feature = "page_layout_demo")]
mod layout_demo;
mod renderer;
#[cfg(feature = "page_scroll_demo")]
mod scroll_demo;
#[cfg(feature = "page_spec")]
mod spec_page;
mod text;

use framewise::input::Input;
use framewise::types::Vec2;

use renderer::Renderer;
use std::sync::Arc;
use text::SampleTextSystem;
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
    ButtonDemo,
    ScrollDemo,
    WidgetSpec,
    #[cfg(feature = "page_frame_demo")]
    FrameDemo,
    #[cfg(feature = "page_layout_demo")]
    LayoutDemo,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    size: PhysicalSize<u32>,
}

impl GpuState {
    fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
    }
}

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<GpuState>,
    text_system: Option<SampleTextSystem>,
    focus_system: framewise::focus::FocusSystem,
    start_time: std::time::Instant,
    click_tracker: framewise::input::ClickTracker,
    modifiers: winit::keyboard::ModifiersState,
    input: Input,
    clipboard: Option<arboard::Clipboard>,
    active_page: AppPage,
    debug_layout: bool,
    #[cfg(feature = "page_button_demo")]
    button_page_state: button_page::ButtonPageState,
    #[cfg(feature = "page_scroll_demo")]
    scroll_demo_state: scroll_demo::ScrollDemoState,
    #[cfg(feature = "page_spec")]
    spec_page_state: spec_page::SpecPageState,
    #[cfg(feature = "page_frame_demo")]
    frame_demo_state: frame_demo::FrameDemoState,
    #[cfg(feature = "page_layout_demo")]
    layout_demo_state: layout_demo::LayoutDemoState,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            gpu: None,
            text_system: Some(SampleTextSystem::new()),
            focus_system: framewise::focus::FocusSystem::new(),
            start_time: std::time::Instant::now(),
            click_tracker: framewise::input::ClickTracker::new(),
            modifiers: winit::keyboard::ModifiersState::default(),
            input: Input::new(),
            clipboard: arboard::Clipboard::new().ok(),
            active_page: AppPage::ButtonDemo,
            debug_layout: false,
            #[cfg(feature = "page_button_demo")]
            button_page_state: button_page::ButtonPageState::default(),
            #[cfg(feature = "page_scroll_demo")]
            scroll_demo_state: scroll_demo::ScrollDemoState::default(),
            #[cfg(feature = "page_spec")]
            spec_page_state: spec_page::SpecPageState::default(),
            #[cfg(feature = "page_frame_demo")]
            frame_demo_state: frame_demo::FrameDemoState::default(),
            #[cfg(feature = "page_layout_demo")]
            layout_demo_state: layout_demo::LayoutDemoState::default(),
        }
    }

    fn draw_missing_feature_page(
        win_size: (f32, f32),
        text_system: &mut SampleTextSystem,
    ) -> framewise::DrawCommands {
        use framewise::{Color, DrawCmd, FontId, Rect, TextBounds, TextFlow, TextSystem};
        let mut cmds = framewise::DrawCommands::new();
        cmds.push(DrawCmd::FillRect {
            rect: Rect::new(0.0, 0.0, win_size.0, win_size.1),
            color: Color::from_srgb_u8(28, 28, 32, 255),
        });
        let flow = TextFlow::single_line();
        let m = text_system.measure(
            "Feature not enabled",
            24.0,
            FontId(1),
            flow,
            TextBounds::UNBOUNDED,
        );
        let cx = (win_size.0 - m.size.x) * 0.5;
        let cy = (win_size.1 - m.size.y) * 0.5;
        let layout = text_system.prepare(
            "Feature not enabled",
            24.0,
            FontId(1),
            flow,
            Rect::new(cx, cy, m.size.x, m.size.y),
        );
        cmds.push(DrawCmd::Text {
            rect: Rect::new(cx, cy, m.size.x, m.size.y),
            color: Color::from_srgb_u8(140, 140, 150, 255),
            handle: layout.handle,
        });
        cmds
    }

    #[allow(unreachable_code)]
    fn draw_ui(&mut self, text_system: &mut SampleTextSystem) -> framewise::DrawCommands {
        let win_size = self
            .gpu
            .as_ref()
            .map(|g| (g.size.width as f32, g.size.height as f32))
            .unwrap_or((1600.0, 1200.0));
        let time = self.start_time.elapsed().as_secs_f64();

        match self.active_page {
            AppPage::ButtonDemo => {
                #[cfg(feature = "page_button_demo")]
                {
                    self.focus_system.begin_frame();
                    let cmds = button_page::draw_button_page(
                        &mut self.button_page_state,
                        &mut self.focus_system,
                        &self.input,
                        time,
                        win_size,
                        text_system,
                        self.debug_layout,
                    );
                    self.focus_system.end_frame();
                    return cmds;
                }
                Self::draw_missing_feature_page(win_size, text_system)
            }
            AppPage::WidgetSpec => {
                #[cfg(feature = "page_spec")]
                {
                    self.focus_system.begin_frame();
                    let cmds = spec_page::draw_spec_page(
                        text_system,
                        &mut self.focus_system,
                        &mut self.spec_page_state,
                        &self.input,
                        time,
                        win_size.0,
                        win_size.1,
                        self.debug_layout,
                    );
                    self.focus_system.end_frame();
                    return cmds;
                }
                Self::draw_missing_feature_page(win_size, text_system)
            }
            AppPage::ScrollDemo => {
                #[cfg(feature = "page_scroll_demo")]
                {
                    self.focus_system.begin_frame();
                    let cmds = scroll_demo::draw_scroll_demo(
                        &mut self.scroll_demo_state,
                        &mut self.clipboard,
                        &mut self.focus_system,
                        &self.input,
                        time,
                        win_size,
                        text_system,
                        self.debug_layout,
                    );
                    self.focus_system.end_frame();
                    return cmds;
                }
                Self::draw_missing_feature_page(win_size, text_system)
            }
            AppPage::FrameDemo => {
                #[cfg(feature = "page_frame_demo")]
                {
                    self.focus_system.begin_frame();
                    let cmds = frame_demo::draw_frame_page(
                        &mut self.frame_demo_state,
                        &mut self.focus_system,
                        &self.input,
                        time,
                        win_size,
                        text_system,
                        self.debug_layout,
                    );
                    self.focus_system.end_frame();
                    return cmds;
                }
                Self::draw_missing_feature_page(win_size, text_system)
            }
            AppPage::LayoutDemo => {
                #[cfg(feature = "page_layout_demo")]
                {
                    self.focus_system.begin_frame();
                    let cmds = layout_demo::draw_layout_page(
                        &mut self.layout_demo_state,
                        &mut self.focus_system,
                        &self.input,
                        time,
                        win_size,
                        text_system,
                        self.debug_layout,
                    );
                    self.focus_system.end_frame();
                    return cmds;
                }
                Self::draw_missing_feature_page(win_size, text_system)
            }
        }
    }
}

// ── ApplicationHandler ────────────────────────────────────────────────────────

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let mut attrs = Window::default_attributes()
            .with_title("Framewise Sample")
            .with_inner_size(PhysicalSize::new(1600u32, 1200u32));

        let svg_data = include_bytes!("../../logo/framewise-mark.svg");
        let opt = usvg::Options::default();
        let fontdb = usvg::fontdb::Database::new();
        if let Ok(tree) = usvg::Tree::from_data(svg_data, &opt, &fontdb) {
            let size = tree.size().to_int_size();
            if let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(size.width(), size.height()) {
                resvg::render(
                    &tree,
                    resvg::tiny_skia::Transform::default(),
                    &mut pixmap.as_mut(),
                );
                if let Ok(icon) =
                    winit::window::Icon::from_rgba(pixmap.take(), size.width(), size.height())
                {
                    attrs = attrs.with_window_icon(Some(icon));
                }
            }
        }

        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );

        let gpu = pollster::block_on(init_wgpu(Arc::clone(&window)));

        self.window = Some(window);
        self.gpu = Some(gpu);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
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
                self.input.mouse_pos = Vec2::new(position.x as f32, position.y as f32);
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
                        self.input.mouse_down = true;
                        self.input.mouse_pressed = true;
                        self.input.mouse_clicked = false;

                        let now = std::time::Instant::now();
                        let count = self.click_tracker.register_click(self.input.mouse_pos, now);
                        self.input.mouse_click_count = count;
                    }
                    ElementState::Released => {
                        self.input.mouse_down = false;
                        self.input.mouse_clicked = true;
                    }
                }
            }

            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
                self.input.modifier_shift = modifiers.state().shift_key();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                // F1 = ScrollDemo, F2 = WidgetSpec, F3 = ButtonDemo, F4 = FrameDemo, F5 = LayoutDemo, F12 = toggle layout-debug overlay
                if event.state == ElementState::Pressed {
                    match event.physical_key {
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F1) => {
                            self.active_page = AppPage::ScrollDemo;
                        }
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F2) => {
                            self.active_page = AppPage::WidgetSpec;
                        }
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F3) => {
                            self.active_page = AppPage::ButtonDemo;
                        }
                        #[cfg(feature = "page_frame_demo")]
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F4) => {
                            self.active_page = AppPage::FrameDemo;
                        }
                        #[cfg(feature = "page_layout_demo")]
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F5) => {
                            self.active_page = AppPage::LayoutDemo;
                        }
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F12) => {
                            self.debug_layout = !self.debug_layout;
                        }
                        _ => {}
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
                    use framewise::input::TextEvent;
                    use winit::keyboard::{Key, NamedKey};

                    match &event.logical_key {
                        Key::Named(NamedKey::Backspace) => {
                            self.input.text_events.push(TextEvent::Backspace {
                                ctrl: self.modifiers.control_key(),
                            })
                        }
                        Key::Named(NamedKey::Delete) => {
                            self.input.text_events.push(TextEvent::Delete {
                                ctrl: self.modifiers.control_key(),
                            })
                        }
                        Key::Named(NamedKey::ArrowLeft) => {
                            self.input.text_events.push(TextEvent::CaretLeft {
                                shift: self.modifiers.shift_key(),
                                ctrl: self.modifiers.control_key(),
                            })
                        }
                        Key::Named(NamedKey::ArrowRight) => {
                            self.input.text_events.push(TextEvent::CaretRight {
                                shift: self.modifiers.shift_key(),
                                ctrl: self.modifiers.control_key(),
                            })
                        }
                        Key::Named(NamedKey::Home) => {
                            self.input.text_events.push(TextEvent::CaretHome {
                                shift: self.modifiers.shift_key(),
                            })
                        }
                        Key::Named(NamedKey::End) => {
                            self.input.text_events.push(TextEvent::CaretEnd {
                                shift: self.modifiers.shift_key(),
                            })
                        }
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
                let mut text_system = self.text_system.take().unwrap();
                text_system.begin_frame();
                let draw_cmds = self.draw_ui(&mut text_system);

                self.input.clear_frame_state();

                if let Some(gpu) = &mut self.gpu {
                    match gpu.surface.get_current_texture() {
                        Ok(frame) => {
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder = gpu.device.create_command_encoder(
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

    let surface = instance
        .create_surface(Arc::clone(&window))
        .expect("failed to create surface");

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("no suitable wgpu adapter found");

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
        .expect("failed to create device");

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_fmt = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_fmt,
        width: size.width,
        height: size.height,
        present_mode: surface_caps.present_modes[0],
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);

    let renderer = Renderer::new(&device, surface_fmt);

    GpuState {
        surface,
        device,
        queue,
        config,
        renderer,
        size,
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("event loop error");
}
