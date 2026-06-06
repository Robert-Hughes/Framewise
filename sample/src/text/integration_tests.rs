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
            framewise::TextStyle {
                font: font_id,
                size: 14.0,
                weight: 400,
                flow,
                italic: false,
            },
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

        // Extract unpadded RGBA pixel data
        let mut actual_rgba = vec![0u8; (width * height * bytes_per_pixel) as usize];
        for row in 0..height {
            let src_start = (row * padded_bytes_per_row) as usize;
            let src_end = src_start + (width * bytes_per_pixel) as usize;
            let dst_start = (row * width * bytes_per_pixel) as usize;
            let dst_end = dst_start + (width * bytes_per_pixel) as usize;
            actual_rgba[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
        }

        drop(data);
        output_buffer.unmap();

        let golden_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/text/golden_text.bmp");

        if !golden_path.exists() {
            write_bmp(golden_path.to_str().unwrap(), width, height, &actual_rgba).unwrap();
            panic!(
                "Golden image did not exist. Created a new golden image at {}. Please verify it and check it in to the repository.",
                golden_path.display()
            );
        }

        let (golden_w, golden_h, golden_rgba) = read_bmp(golden_path.to_str().unwrap()).unwrap();

        if golden_w != width || golden_h != height {
            let actual_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/text/golden_text_actual.bmp");
            write_bmp(actual_path.to_str().unwrap(), width, height, &actual_rgba).unwrap();
            panic!(
                "Dimension mismatch: Golden is {}x{}, Actual is {}x{}. Mismatched image written to {}.",
                golden_w, golden_h, width, height, actual_path.display()
            );
        }

        if golden_rgba != actual_rgba {
            let actual_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/text/golden_text_actual.bmp");
            write_bmp(actual_path.to_str().unwrap(), width, height, &actual_rgba).unwrap();
            panic!(
                "Pixel mismatch: Rendered text does not match golden image. Mismatched image written to {} for comparison.",
                actual_path.display()
            );
        }
    }

    fn write_bmp(path: &str, width: u32, height: u32, rgba_pixels: &[u8]) -> std::io::Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(path)?;
        let pixel_data_size = width * height * 4;
        let file_size = 54 + pixel_data_size;

        // File Header (14 bytes)
        file.write_all(b"BM")?;
        file.write_all(&file_size.to_le_bytes())?;
        file.write_all(&0u16.to_le_bytes())?; // reserved 1
        file.write_all(&0u16.to_le_bytes())?; // reserved 2
        file.write_all(&54u32.to_le_bytes())?; // offset to pixel data

        // Info Header (40 bytes)
        file.write_all(&40u32.to_le_bytes())?; // biSize
        file.write_all(&(width as i32).to_le_bytes())?; // biWidth
        file.write_all(&(-(height as i32)).to_le_bytes())?; // biHeight (negative for top-down)
        file.write_all(&1u16.to_le_bytes())?; // biPlanes
        file.write_all(&32u16.to_le_bytes())?; // biBitCount (32 bits for BGRA)
        file.write_all(&0u32.to_le_bytes())?; // biCompression
        file.write_all(&pixel_data_size.to_le_bytes())?; // biSizeImage
        file.write_all(&0i32.to_le_bytes())?; // biXPelsPerMeter
        file.write_all(&0i32.to_le_bytes())?; // biYPelsPerMeter
        file.write_all(&0u32.to_le_bytes())?; // biClrUsed
        file.write_all(&0u32.to_le_bytes())?; // biClrImportant

        // Convert RGBA to BGRA
        let mut bgra = vec![0u8; pixel_data_size as usize];
        for i in 0..(width * height) as usize {
            bgra[i * 4] = rgba_pixels[i * 4 + 2]; // B
            bgra[i * 4 + 1] = rgba_pixels[i * 4 + 1]; // G
            bgra[i * 4 + 2] = rgba_pixels[i * 4]; // R
            bgra[i * 4 + 3] = rgba_pixels[i * 4 + 3]; // A
        }
        file.write_all(&bgra)?;
        Ok(())
    }

    fn read_bmp(path: &str) -> std::io::Result<(u32, u32, Vec<u8>)> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path)?;
        let mut header = [0u8; 54];
        file.read_exact(&mut header)?;

        if &header[0..2] != b"BM" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Not a BMP file",
            ));
        }

        let offset = u32::from_le_bytes(header[10..14].try_into().unwrap()) as usize;
        let width = i32::from_le_bytes(header[18..22].try_into().unwrap());
        let height = i32::from_le_bytes(header[22..26].try_into().unwrap());
        let bit_count = u16::from_le_bytes(header[28..30].try_into().unwrap());

        if bit_count != 32 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Only 32-bit BMPs are supported, got {}", bit_count),
            ));
        }

        if offset > 54 {
            let mut skip = vec![0u8; offset - 54];
            file.read_exact(&mut skip)?;
        }

        let w = width as u32;
        let h = height.unsigned_abs();
        let top_down = height < 0;

        let pixel_count = (w * h) as usize;
        let mut bgra = vec![0u8; pixel_count * 4];
        file.read_exact(&mut bgra)?;

        let mut rgba = vec![0u8; pixel_count * 4];
        for i in 0..pixel_count {
            let b = bgra[i * 4];
            let g = bgra[i * 4 + 1];
            let r = bgra[i * 4 + 2];
            let a = bgra[i * 4 + 3];

            let row = i / w as usize;
            let col = i % w as usize;
            let target_row = if top_down {
                row
            } else {
                (h as usize - 1) - row
            };
            let target_idx = (target_row * w as usize + col) * 4;

            rgba[target_idx] = r;
            rgba[target_idx + 1] = g;
            rgba[target_idx + 2] = b;
            rgba[target_idx + 3] = a;
        }

        Ok((w, h, rgba))
    }
}
