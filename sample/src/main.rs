#[cfg(feature = "page_button_demo")]
mod button_page;
#[cfg(feature = "page_frame_demo")]
mod frame_demo;
#[cfg(feature = "page_label_demo")]
mod label_page;
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
    #[cfg(feature = "page_label_demo")]
    LabelDemo,
}

// ── App state ─────────────────────────────────────────────────────────────────

struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    renderer: Renderer,
    size: PhysicalSize<u32>,
    vsync_mode: wgpu::PresentMode,
    no_vsync_mode: wgpu::PresentMode,
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

    fn set_vsync(&mut self, vsync: bool) {
        let target_mode = if vsync {
            self.vsync_mode
        } else {
            self.no_vsync_mode
        };
        if self.config.present_mode != target_mode {
            self.config.present_mode = target_mode;
            self.surface.configure(&self.device, &self.config);
        }
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
    vsync: bool,
    last_frame_instant: std::time::Instant,
    fps_sum_frame_time: f64,
    fps_frame_count: u32,
    fps_last_update: std::time::Instant,
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
    #[cfg(feature = "page_label_demo")]
    label_page_state: label_page::LabelPageState,
    is_first_frame: bool,
}

impl App {
    fn new() -> Self {
        let now = std::time::Instant::now();
        eprintln!("[STARTUP] App::new() starting");
        let text_system_start = std::time::Instant::now();
        let text_system = SampleTextSystem::new();
        eprintln!(
            "[STARTUP]   SampleTextSystem::new() took {:?}",
            text_system_start.elapsed()
        );
        Self {
            window: None,
            gpu: None,
            text_system: Some(text_system),
            focus_system: framewise::focus::FocusSystem::new(),
            start_time: now,
            click_tracker: framewise::input::ClickTracker::new(),
            modifiers: winit::keyboard::ModifiersState::default(),
            input: Input::new(),
            clipboard: arboard::Clipboard::new().ok(),
            active_page: AppPage::ButtonDemo,
            debug_layout: false,
            vsync: true,
            last_frame_instant: now,
            fps_sum_frame_time: 0.0,
            fps_frame_count: 0,
            fps_last_update: now,
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
            #[cfg(feature = "page_label_demo")]
            label_page_state: label_page::LabelPageState::default(),
            is_first_frame: true,
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
            framewise::TextStyle::new(FontId(1), 24.0, 400, flow),
            TextBounds::UNBOUNDED,
        );
        let cx = (win_size.0 - m.logical_size.x) * 0.5;
        let cy = (win_size.1 - m.logical_size.y) * 0.5;
        let layout = text_system.prepare(
            "Feature not enabled",
            framewise::TextStyle::new(FontId(1), 24.0, 400, flow),
            Rect::new(cx, cy, m.logical_size.x, m.logical_size.y),
        );
        cmds.push(DrawCmd::Text {
            rect: Rect::new(cx, cy, m.logical_size.x, m.logical_size.y),
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
            AppPage::LabelDemo => {
                #[cfg(feature = "page_label_demo")]
                {
                    self.focus_system.begin_frame();
                    let cmds = label_page::draw_label_page(
                        &mut self.label_page_state,
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
        eprintln!(
            "[STARTUP] [{:?}] resumed() entered",
            self.start_time.elapsed()
        );
        let start = std::time::Instant::now();
        let mut attrs = Window::default_attributes()
            .with_title("Framewise Sample")
            .with_inner_size(PhysicalSize::new(1600u32, 1200u32))
            .with_visible(false);

        let svg_start = std::time::Instant::now();
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
        eprintln!(
            "[STARTUP] [{:?}]   SVG icon processing took {:?}",
            self.start_time.elapsed(),
            svg_start.elapsed()
        );

        let win_start = std::time::Instant::now();
        let window = Arc::new(
            event_loop
                .create_window(attrs)
                .expect("failed to create window"),
        );
        eprintln!(
            "[STARTUP] [{:?}]   window creation took {:?}",
            self.start_time.elapsed(),
            win_start.elapsed()
        );

        let gpu_start = std::time::Instant::now();
        let gpu = pollster::block_on(init_wgpu(Arc::clone(&window)));
        eprintln!(
            "[STARTUP] [{:?}]   init_wgpu took {:?}",
            self.start_time.elapsed(),
            gpu_start.elapsed()
        );

        self.window = Some(window.clone());
        self.gpu = Some(gpu);
        window.set_visible(true);
        window.request_redraw();
        eprintln!(
            "[STARTUP] [{:?}] resumed() completed in {:?}",
            self.start_time.elapsed(),
            start.elapsed()
        );
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
                // F1 = ScrollDemo, F2 = WidgetSpec, F3 = ButtonDemo, F4 = FrameDemo, F5 = LayoutDemo, F11 = toggle VSync, F12 = toggle layout-debug overlay
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
                        #[cfg(feature = "page_label_demo")]
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F6) => {
                            self.active_page = AppPage::LabelDemo;
                        }
                        winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F11) => {
                            self.vsync = !self.vsync;
                            #[cfg(all(
                                feature = "page_spec",
                                feature = "window",
                                feature = "tabs",
                                feature = "segmented",
                                feature = "slider",
                                feature = "switch",
                                feature = "drag_number",
                                feature = "color_swatch",
                                feature = "checkbox",
                                feature = "button",
                                feature = "menu"
                            ))]
                            {
                                self.spec_page_state.w.iu_vsync.checked = self.vsync;
                            }
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
                let redraw_start = std::time::Instant::now();
                let is_first = self.is_first_frame;
                if is_first {
                    eprintln!(
                        "[STARTUP] [{:?}] First RedrawRequested starts",
                        self.start_time.elapsed()
                    );
                    self.is_first_frame = false;
                }
                let mut text_system = self.text_system.take().unwrap();
                text_system.begin_frame();
                let draw_ui_start = std::time::Instant::now();
                let draw_cmds = self.draw_ui(&mut text_system);
                if is_first {
                    eprintln!(
                        "[STARTUP] [{:?}]   First draw_ui took {:?}",
                        self.start_time.elapsed(),
                        draw_ui_start.elapsed()
                    );
                }

                self.input.clear_frame_state();

                #[cfg(all(
                    feature = "page_spec",
                    feature = "window",
                    feature = "tabs",
                    feature = "segmented",
                    feature = "slider",
                    feature = "switch",
                    feature = "drag_number",
                    feature = "color_swatch",
                    feature = "checkbox",
                    feature = "button",
                    feature = "menu"
                ))]
                if self.active_page == AppPage::WidgetSpec {
                    self.vsync = self.spec_page_state.w.iu_vsync.checked;
                }

                if let Some(gpu) = &mut self.gpu {
                    let vsync_start = std::time::Instant::now();
                    gpu.set_vsync(self.vsync);
                    if is_first {
                        eprintln!(
                            "[STARTUP] [{:?}]   First set_vsync took {:?}",
                            self.start_time.elapsed(),
                            vsync_start.elapsed()
                        );
                    }

                    let tex_start = std::time::Instant::now();
                    match gpu.surface.get_current_texture() {
                        Ok(frame) => {
                            if is_first {
                                eprintln!(
                                    "[STARTUP] [{:?}]   First get_current_texture took {:?}",
                                    self.start_time.elapsed(),
                                    tex_start.elapsed()
                                );
                            }
                            let view = frame
                                .texture
                                .create_view(&wgpu::TextureViewDescriptor::default());
                            let mut encoder = gpu.device.create_command_encoder(
                                &wgpu::CommandEncoderDescriptor {
                                    label: Some("frame_encoder"),
                                },
                            );

                            let render_start = std::time::Instant::now();
                            gpu.renderer.render(
                                &gpu.device,
                                &gpu.queue,
                                &view,
                                &mut encoder,
                                &draw_cmds,
                                (gpu.size.width, gpu.size.height),
                                &mut text_system,
                            );
                            if is_first {
                                eprintln!(
                                    "[STARTUP] [{:?}]   First Renderer::render took {:?}",
                                    self.start_time.elapsed(),
                                    render_start.elapsed()
                                );
                            }

                            let submit_start = std::time::Instant::now();
                            gpu.queue.submit(std::iter::once(encoder.finish()));
                            frame.present();
                            if is_first {
                                eprintln!(
                                    "[STARTUP] [{:?}]   First submit & present took {:?}",
                                    self.start_time.elapsed(),
                                    submit_start.elapsed()
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("get_current_texture error: {e}");
                        }
                    }
                }

                self.text_system = Some(text_system);
                if is_first {
                    eprintln!(
                        "[STARTUP] First RedrawRequested completed in {:?}",
                        redraw_start.elapsed()
                    );
                }

                // Update FPS calculation and window title
                let now = std::time::Instant::now();
                let frame_time = now.duration_since(self.last_frame_instant).as_secs_f64();
                self.last_frame_instant = now;

                self.fps_sum_frame_time += frame_time;
                self.fps_frame_count += 1;

                if now.duration_since(self.fps_last_update).as_secs_f64() >= 0.2 {
                    if self.fps_frame_count > 0 {
                        let avg_frame_time = self.fps_sum_frame_time / self.fps_frame_count as f64;
                        let fps = if avg_frame_time > 0.0 {
                            1.0 / avg_frame_time
                        } else {
                            0.0
                        };
                        if let Some(win) = &self.window {
                            let vsync_suffix = if self.vsync { "" } else { " (VSYNC OFF)" };
                            win.set_title(&format!(
                                "Framewise Sample - {:.1} FPS{}",
                                fps, vsync_suffix
                            ));
                        }
                    }
                    self.fps_sum_frame_time = 0.0;
                    self.fps_frame_count = 0;
                    self.fps_last_update = now;
                }

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
    let t0 = std::time::Instant::now();

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    eprintln!("[STARTUP]   wgpu::Instance::new took {:?}", t0.elapsed());
    let t1 = std::time::Instant::now();

    let surface = instance
        .create_surface(Arc::clone(&window))
        .expect("failed to create surface");
    eprintln!("[STARTUP]   create_surface took {:?}", t1.elapsed());
    let t2 = std::time::Instant::now();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::default(),
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .expect("no suitable wgpu adapter found");
    eprintln!("[STARTUP]   request_adapter took {:?}", t2.elapsed());
    let t3 = std::time::Instant::now();

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
    eprintln!("[STARTUP]   request_device took {:?}", t3.elapsed());
    let t4 = std::time::Instant::now();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_fmt = surface_caps
        .formats
        .iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

    let vsync_mode = surface_caps.present_modes[0];
    let no_vsync_mode = if surface_caps
        .present_modes
        .contains(&wgpu::PresentMode::Immediate)
    {
        wgpu::PresentMode::Immediate
    } else if surface_caps
        .present_modes
        .contains(&wgpu::PresentMode::Mailbox)
    {
        wgpu::PresentMode::Mailbox
    } else {
        vsync_mode
    };

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_fmt,
        width: size.width,
        height: size.height,
        present_mode: vsync_mode,
        alpha_mode: surface_caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
    };
    surface.configure(&device, &config);
    eprintln!("[STARTUP]   surface configure took {:?}", t4.elapsed());
    let t5 = std::time::Instant::now();

    let renderer = Renderer::new(&device, surface_fmt);
    eprintln!("[STARTUP]   Renderer::new took {:?}", t5.elapsed());

    GpuState {
        surface,
        device,
        queue,
        config,
        renderer,
        size,
        vsync_mode,
        no_vsync_mode,
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

fn main() {
    let startup_timer = std::time::Instant::now();
    eprintln!("[STARTUP] main() started");
    env_logger::init();

    let event_loop = EventLoop::new().expect("failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);
    eprintln!(
        "[STARTUP] EventLoop::new took {:?}",
        startup_timer.elapsed()
    );

    let mut app = App::new();
    eprintln!("[STARTUP] App::new took {:?}", startup_timer.elapsed());
    event_loop.run_app(&mut app).expect("event loop error");
}
