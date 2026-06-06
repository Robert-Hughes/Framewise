#[cfg(test)]
mod integration_tests {
    use crate::renderer::Renderer;
    use crate::text::SampleTextSystem;
    use framewise::{Color, DrawCmd, FontId, Rect, TextFlow, TextSystem};

    #[test]
    fn test_headless_text_rendering() {
        pollster::block_on(run_headless_test());
    }

    async fn run_headless_test() {
        // 1. Initialize WGPU Instance and Adapter
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await;

        let adapter = match adapter {
            Some(a) => a,
            None => {
                println!("Skipping headless integration test: No suitable GPU adapter found");
                return;
            }
        };

        // 2. Request Device and Queue
        let (device, queue) = match adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("headless_device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
        {
            Ok((d, q)) => (d, q),
            Err(e) => {
                println!(
                    "Skipping headless integration test: Request device failed: {:?}",
                    e
                );
                return;
            }
        };

        // 3. Create Offscreen Texture and View
        let width = 200;
        let height = 50;
        let texture_desc = wgpu::TextureDescriptor {
            label: Some("headless_target_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        };
        let texture = device.create_texture(&texture_desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // 4. Initialize Renderer and Text System
        let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
        let mut text_system = SampleTextSystem::new();
        text_system.begin_frame();

        // 5. Layout and Draw Command
        let mut cmds = Vec::new();
        let font_id = FontId(1); // Sans Regular
        let flow = TextFlow::single_line();
        let test_str = "Headless Test.";
        let layout = text_system.prepare(
            test_str,
            14.0,
            font_id,
            flow,
            Rect::new(10.0, 15.0, 180.0, 30.0),
        );
        cmds.push(DrawCmd::Text {
            rect: Rect::new(10.0, 15.0, 180.0, 30.0),
            color: Color::from_srgb_u8(0, 0, 0, 255), // black ink
            handle: layout.handle,
        });

        // 6. Draw to Texture
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("headless_encoder"),
        });
        renderer.render(
            &device,
            &queue,
            &view,
            &mut encoder,
            &cmds,
            (width, height),
            &mut text_system,
        );

        // 7. Copy Texture to Buffer for reading
        let bytes_per_pixel = 4;
        // wgpu requires bytes_per_row to be aligned to 256
        let align = 256;
        let unaligned_bytes_per_row = width * bytes_per_pixel;
        let padding = (align - unaligned_bytes_per_row % align) % align;
        let padded_bytes_per_row = unaligned_bytes_per_row + padding;

        let output_buffer_desc = wgpu::BufferDescriptor {
            label: Some("headless_output_buffer"),
            size: (padded_bytes_per_row * height) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &output_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(std::iter::once(encoder.finish()));

        // 8. Map and Read Buffer
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        device.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();

        let data = buffer_slice.get_mapped_range();

        // Let's assert that we rendered some non-white, non-paper pixels (meaning text was rasterized).
        // Background clear color: paper (#f4f1ea)
        // Check if any pixels in the buffer deviate from the background color.
        let mut found_non_background = false;
        let paper_r = 244u8;
        let paper_g = 241u8;
        let paper_b = 234u8;

        for row in 0..height {
            let row_start = (row * padded_bytes_per_row) as usize;
            for col in 0..width {
                let pixel_start = row_start + (col * bytes_per_pixel) as usize;
                let r = data[pixel_start];
                let g = data[pixel_start + 1];
                let b = data[pixel_start + 2];
                let a = data[pixel_start + 3];

                if a > 0 && (r != paper_r || g != paper_g || b != paper_b) {
                    found_non_background = true;
                    break;
                }
            }
            if found_non_background {
                break;
            }
        }

        assert!(
            found_non_background,
            "Headless render failed: target texture contains only background color. No text rendered."
        );

        drop(data);
        output_buffer.unmap();
    }
}
