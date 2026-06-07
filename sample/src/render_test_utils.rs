use crate::renderer::Renderer;
use crate::text::SampleTextSystem;
use framewise::DrawCommands;
use std::path::Path;

pub struct RgbaImage {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub async fn render_commands_to_rgba<F>(
    width: u32,
    height: u32,
    build_commands: F,
) -> Option<RgbaImage>
where
    F: FnOnce(&mut SampleTextSystem) -> DrawCommands,
{
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });

    let Some(adapter) = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await
    else {
        eprintln!("Skipping golden render test: no suitable GPU adapter found");
        return None;
    };

    let (device, queue) = match adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some("golden_test_device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: Default::default(),
            },
            None,
        )
        .await
    {
        Ok(pair) => pair,
        Err(err) => {
            eprintln!("Skipping golden render test: request device failed: {err:?}");
            return None;
        }
    };

    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("golden_test_target_texture"),
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
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

    let mut renderer = Renderer::new(&device, wgpu::TextureFormat::Rgba8UnormSrgb);
    let mut text_system = SampleTextSystem::new();
    text_system.begin_frame();
    let cmds = build_commands(&mut text_system);

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("golden_test_encoder"),
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

    let bytes_per_pixel = 4;
    let unaligned_bytes_per_row = width * bytes_per_pixel;
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let padded_bytes_per_row = unaligned_bytes_per_row.div_ceil(align) * align;

    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("golden_test_output_buffer"),
        size: (padded_bytes_per_row * height) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

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

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let mut pixels = vec![0u8; (width * height * bytes_per_pixel) as usize];
    for row in 0..height {
        let src_start = (row * padded_bytes_per_row) as usize;
        let src_end = src_start + (width * bytes_per_pixel) as usize;
        let dst_start = (row * width * bytes_per_pixel) as usize;
        let dst_end = dst_start + (width * bytes_per_pixel) as usize;
        pixels[dst_start..dst_end].copy_from_slice(&data[src_start..src_end]);
    }
    drop(data);
    output_buffer.unmap();

    Some(RgbaImage {
        width,
        height,
        pixels,
    })
}

pub fn assert_matches_png_golden(actual: &RgbaImage, golden_path: &Path) {
    if !golden_path.exists() {
        write_png(golden_path, actual).unwrap_or_else(|err| {
            panic!(
                "Failed to write missing golden image {}: {err}",
                golden_path.display()
            )
        });
        panic!(
            "Golden image did not exist. Created a new golden image at {}. Please verify it and check it in.",
            golden_path.display()
        );
    }

    let golden = read_png(golden_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read golden image {}: {err}",
            golden_path.display()
        )
    });

    if golden.width != actual.width || golden.height != actual.height {
        let actual_path = actual_path_for(golden_path);
        write_png(&actual_path, actual).unwrap_or_else(|err| {
            panic!(
                "Failed to write mismatched image {}: {err}",
                actual_path.display()
            )
        });
        panic!(
            "Dimension mismatch: golden is {}x{}, actual is {}x{}. Actual image written to {}.",
            golden.width,
            golden.height,
            actual.width,
            actual.height,
            actual_path.display()
        );
    }

    if golden.pixels != actual.pixels {
        let actual_path = actual_path_for(golden_path);
        write_png(&actual_path, actual).unwrap_or_else(|err| {
            panic!(
                "Failed to write mismatched image {}: {err}",
                actual_path.display()
            )
        });
        panic!(
            "Pixel mismatch: rendered image does not match golden. Actual image written to {}.",
            actual_path.display()
        );
    }
}

fn actual_path_for(golden_path: &Path) -> std::path::PathBuf {
    let stem = golden_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("golden");
    golden_path.with_file_name(format!("{stem}_actual.png"))
}

fn write_png(path: &Path, image: &RgbaImage) -> std::io::Result<()> {
    let file = std::fs::File::create(path)?;
    let writer = std::io::BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header()?;
    writer.write_image_data(&image.pixels)?;
    Ok(())
}

fn read_png(path: &Path) -> std::io::Result<RgbaImage> {
    let file = std::fs::File::open(path)?;
    let decoder = png::Decoder::new(std::io::BufReader::new(file));
    let mut reader = decoder.read_info()?;
    let mut buffer = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buffer)?;

    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "expected 8-bit RGBA PNG, got {:?} {:?}",
                info.color_type, info.bit_depth
            ),
        ));
    }

    Ok(RgbaImage {
        width: info.width,
        height: info.height,
        pixels: buffer[..info.buffer_size()].to_vec(),
    })
}
