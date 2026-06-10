use bytemuck::{Pod, Zeroable};
use framewise::{Color, DrawCmd, Rect};
use wgpu::util::DeviceExt;

// ── Vertex layout ─────────────────────────────────────────────────────────────

/// One GPU vertex: 2D clip-space position + RGBA colour.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// One GPU vertex for text: 2D clip-space position + atlas UV + RGBA colour.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TextVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

impl TextVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    quad_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,

    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, surface_fmt: wgpu::TextureFormat) -> Self {
        let t_total = std::time::Instant::now();

        let t0 = std::time::Instant::now();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        eprintln!(
            "[STARTUP]     create_shader_module (quad) took {:?}",
            t0.elapsed()
        );

        let t1 = std::time::Instant::now();
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pipeline_layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        eprintln!(
            "[STARTUP]     create_pipeline_layout (quad) took {:?}",
            t1.elapsed()
        );

        let t2 = std::time::Instant::now();
        let quad_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("quad_pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_fmt,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        eprintln!(
            "[STARTUP]     create_render_pipeline (quad) took {:?}",
            t2.elapsed()
        );

        let t3 = std::time::Instant::now();
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        eprintln!(
            "[STARTUP]     create_shader_module (text) took {:?}",
            t3.elapsed()
        );

        let atlas_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("atlas_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let t4 = std::time::Instant::now();
        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[&atlas_bind_group_layout],
            push_constant_ranges: &[],
        });
        eprintln!(
            "[STARTUP]     create_pipeline_layout (text) took {:?}",
            t4.elapsed()
        );

        let t5 = std::time::Instant::now();
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text_pipeline"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: Some("vs_main"),
                buffers: &[TextVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_fmt,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        eprintln!(
            "[STARTUP]     create_render_pipeline (text) took {:?}",
            t5.elapsed()
        );

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d {
                width: 1024,
                height: 1024,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        // Use Nearest-neighbor filtering to map atlas pixels 1-to-1 with screen pixels.
        // Linear filtering would blend adjacent texels under fractional positioning, causing
        // glyphs to look blurry. Nearest sampling, coupled with pre-shifted horizontal
        // subpixel glyphs in the atlas, guarantees maximum visual crispness.
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bind_group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        eprintln!(
            "[STARTUP]     Renderer::new total took {:?}",
            t_total.elapsed()
        );

        Self {
            quad_pipeline,
            text_pipeline,
            atlas_texture,
            atlas_bind_group,
        }
    }

    /// Convert a list of `DrawCmd`s into vertices and render them.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        cmds: &[DrawCmd],
        window_size: (u32, u32),
        text_system: &mut crate::text::SampleTextSystem,
    ) {
        if text_system.atlas_dirty {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &text_system.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(text_system.atlas_size),
                    rows_per_image: Some(text_system.atlas_size),
                },
                wgpu::Extent3d {
                    width: text_system.atlas_size,
                    height: text_system.atlas_size,
                    depth_or_array_layers: 1,
                },
            );
            text_system.atlas_dirty = false;
        }

        let mut quad_verts: Vec<Vertex> = Vec::new();
        let mut text_verts: Vec<TextVertex> = Vec::new();

        enum RenderCommand {
            DrawQuads(std::ops::Range<u32>),
            DrawText(std::ops::Range<u32>),
            SetScissor(Rect),
        }

        let mut render_cmds = Vec::new();
        let mut current_quad_start = 0;
        let mut current_text_start = 0;
        let mut clip_stack: Vec<Rect> = Vec::new();

        for cmd in cmds {
            match cmd {
                // TODO: Phase 2b will map command z to depth, and may initially store converted z per vertex
                // for batching simplicity, with future optimization possible via batching by (pipeline, clip, z)
                // plus uniform/push-constant z. For now, z is ignored.
                DrawCmd::FillRect { rect, color, z: _ } => {
                    push_filled_rect(&mut quad_verts, *rect, *color, window_size);
                }
                DrawCmd::StrokeRect {
                    rect,
                    color,
                    width,
                    z: _,
                } => {
                    push_stroked_rect(&mut quad_verts, *rect, *color, *width, window_size);
                }
                DrawCmd::StrokeLine {
                    p0,
                    p1,
                    color,
                    width,
                    z: _,
                } => {
                    push_stroke_line(&mut quad_verts, *p0, *p1, *color, *width, window_size);
                }
                DrawCmd::FillCircle {
                    center,
                    radius,
                    color,
                    z: _,
                } => {
                    push_filled_circle(&mut quad_verts, *center, *radius, *color, window_size);
                }
                DrawCmd::StrokeCircle {
                    center,
                    radius,
                    color,
                    width,
                    z: _,
                } => {
                    push_stroked_circle(
                        &mut quad_verts,
                        *center,
                        *radius,
                        *color,
                        *width,
                        window_size,
                    );
                }
                DrawCmd::Text {
                    rect,
                    color,
                    handle,
                    z: _,
                } => {
                    if let Some(run) = text_system.runs.get(handle.0) {
                        push_text_run(
                            &mut text_verts,
                            *rect,
                            *color,
                            run,
                            text_system,
                            window_size,
                        );
                    }
                }
                DrawCmd::PushClip { rect } => {
                    if quad_verts.len() as u32 > current_quad_start {
                        render_cmds.push(RenderCommand::DrawQuads(
                            current_quad_start..quad_verts.len() as u32,
                        ));
                        current_quad_start = quad_verts.len() as u32;
                    }
                    if text_verts.len() as u32 > current_text_start {
                        render_cmds.push(RenderCommand::DrawText(
                            current_text_start..text_verts.len() as u32,
                        ));
                        current_text_start = text_verts.len() as u32;
                    }
                    let new_clip = if let Some(current) = clip_stack.last() {
                        current.intersect(rect)
                    } else {
                        *rect
                    };
                    clip_stack.push(new_clip);
                    render_cmds.push(RenderCommand::SetScissor(new_clip));
                }
                DrawCmd::PopClip => {
                    if quad_verts.len() as u32 > current_quad_start {
                        render_cmds.push(RenderCommand::DrawQuads(
                            current_quad_start..quad_verts.len() as u32,
                        ));
                        current_quad_start = quad_verts.len() as u32;
                    }
                    if text_verts.len() as u32 > current_text_start {
                        render_cmds.push(RenderCommand::DrawText(
                            current_text_start..text_verts.len() as u32,
                        ));
                        current_text_start = text_verts.len() as u32;
                    }
                    clip_stack.pop();
                    let new_clip = clip_stack.last().copied().unwrap_or_else(|| {
                        Rect::new(0.0, 0.0, window_size.0 as f32, window_size.1 as f32)
                    });
                    render_cmds.push(RenderCommand::SetScissor(new_clip));
                }
            }
        }

        if quad_verts.len() as u32 > current_quad_start {
            render_cmds.push(RenderCommand::DrawQuads(
                current_quad_start..quad_verts.len() as u32,
            ));
        }
        if text_verts.len() as u32 > current_text_start {
            render_cmds.push(RenderCommand::DrawText(
                current_text_start..text_verts.len() as u32,
            ));
        }

        if quad_verts.is_empty() && text_verts.is_empty() {
            return;
        }

        let quad_vbuf = if !quad_verts.is_empty() {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("quad_vbuf"),
                    contents: bytemuck::cast_slice(&quad_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        } else {
            None
        };

        let text_vbuf = if !text_verts.is_empty() {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("text_vbuf"),
                    contents: bytemuck::cast_slice(&text_verts),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            )
        } else {
            None
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear({
                        // paper: #f4f1ea — converted to linear for sRGB framebuffer
                        let p = Color::from_srgb_u8(244, 241, 234, 255);
                        wgpu::Color {
                            r: p.r as f64,
                            g: p.g as f64,
                            b: p.b as f64,
                            a: 1.0,
                        }
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mut last_pipeline = 0; // 1 = quads, 2 = text

        for rc in render_cmds {
            match rc {
                RenderCommand::DrawQuads(range) => {
                    if last_pipeline != 1 {
                        pass.set_pipeline(&self.quad_pipeline);
                        pass.set_vertex_buffer(0, quad_vbuf.as_ref().unwrap().slice(..));
                        last_pipeline = 1;
                    }
                    pass.draw(range, 0..1);
                }
                RenderCommand::DrawText(range) => {
                    if last_pipeline != 2 {
                        pass.set_pipeline(&self.text_pipeline);
                        pass.set_bind_group(0, &self.atlas_bind_group, &[]);
                        pass.set_vertex_buffer(0, text_vbuf.as_ref().unwrap().slice(..));
                        last_pipeline = 2;
                    }
                    pass.draw(range, 0..1);
                }
                RenderCommand::SetScissor(r) => {
                    let x = r.x.max(0.0) as u32;
                    let y = r.y.max(0.0) as u32;
                    let right = (r.x + r.w).min(window_size.0 as f32);
                    let bottom = (r.y + r.h).min(window_size.1 as f32);
                    let w = if right > x as f32 {
                        (right - x as f32) as u32
                    } else {
                        0
                    };
                    let h = if bottom > y as f32 {
                        (bottom - y as f32) as u32
                    } else {
                        0
                    };

                    if w > 0 && h > 0 {
                        pass.set_scissor_rect(x, y, w, h);
                    } else {
                        // Degenerate scissor: set to 1x1 outside window to effectively hide it
                        // Or just set to 1x1 at 0,0 and rely on no overlapping geometry
                        pass.set_scissor_rect(0, 0, 1, 1);
                    }
                }
            }
        }
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

/// Convert a logical-pixel rect to clip-space [-1, 1].
fn to_clip(x: f32, y: f32, w: u32, h: u32) -> [f32; 2] {
    [
        (x / w as f32) * 2.0 - 1.0,
        1.0 - (y / h as f32) * 2.0, // y-flip: window top → clip top
    ]
}

fn color_arr(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Push two triangles (six vertices) for a filled rectangle.
fn push_filled_rect(verts: &mut Vec<Vertex>, rect: Rect, color: Color, (sw, sh): (u32, u32)) {
    let tl = to_clip(rect.x, rect.y, sw, sh);
    let tr = to_clip(rect.x + rect.w, rect.y, sw, sh);
    let bl = to_clip(rect.x, rect.y + rect.h, sw, sh);
    let br = to_clip(rect.x + rect.w, rect.y + rect.h, sw, sh);
    let c = color_arr(color);

    // Two CCW triangles.
    verts.push(Vertex { pos: tl, color: c });
    verts.push(Vertex { pos: bl, color: c });
    verts.push(Vertex { pos: tr, color: c });

    verts.push(Vertex { pos: tr, color: c });
    verts.push(Vertex { pos: bl, color: c });
    verts.push(Vertex { pos: br, color: c });
}

/// Push eight thin filled rects (one per side) to approximate a stroked rect.
fn push_stroked_rect(
    verts: &mut Vec<Vertex>,
    rect: Rect,
    color: Color,
    width: f32,
    win_size: (u32, u32),
) {
    let x = rect.x;
    let y = rect.y;
    let w = rect.w;
    let h = rect.h;
    let lw = width;

    // Top, bottom, left, right strips.
    let strips = [
        Rect::new(x, y, w, lw),          // top
        Rect::new(x, y + h - lw, w, lw), // bottom
        Rect::new(x, y, lw, h),          // left
        Rect::new(x + w - lw, y, lw, h), // right
    ];
    for s in &strips {
        push_filled_rect(verts, *s, color, win_size);
    }
}

/// Push two triangles for a line segment of a given width (screen-aligned cap).
fn push_stroke_line(
    verts: &mut Vec<Vertex>,
    p0: framewise::Vec2,
    p1: framewise::Vec2,
    color: Color,
    width: f32,
    win_size: (u32, u32),
) {
    let dx = p1.x - p0.x;
    let dy = p1.y - p0.y;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 0.001 {
        return;
    }
    let hw = width * 0.5;
    let nx = (-dy / len) * hw;
    let ny = (dx / len) * hw;

    let a = to_clip(p0.x + nx, p0.y + ny, win_size.0, win_size.1);
    let b = to_clip(p0.x - nx, p0.y - ny, win_size.0, win_size.1);
    let c2 = to_clip(p1.x - nx, p1.y - ny, win_size.0, win_size.1);
    let d = to_clip(p1.x + nx, p1.y + ny, win_size.0, win_size.1);
    let c = color_arr(color);

    verts.push(Vertex { pos: a, color: c });
    verts.push(Vertex { pos: b, color: c });
    verts.push(Vertex { pos: c2, color: c });
    verts.push(Vertex { pos: a, color: c });
    verts.push(Vertex { pos: c2, color: c });
    verts.push(Vertex { pos: d, color: c });
}

const CIRCLE_SEGS: usize = 32;

/// Push a triangle fan for a filled circle.
fn push_filled_circle(
    verts: &mut Vec<Vertex>,
    center: framewise::Vec2,
    radius: f32,
    color: Color,
    win_size: (u32, u32),
) {
    let c = color_arr(color);
    let cx = to_clip(center.x, center.y, win_size.0, win_size.1);
    for i in 0..CIRCLE_SEGS {
        let a0 = (i as f32 / CIRCLE_SEGS as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / CIRCLE_SEGS as f32) * std::f32::consts::TAU;
        let p0 = to_clip(
            center.x + a0.cos() * radius,
            center.y + a0.sin() * radius,
            win_size.0,
            win_size.1,
        );
        let p1 = to_clip(
            center.x + a1.cos() * radius,
            center.y + a1.sin() * radius,
            win_size.0,
            win_size.1,
        );
        verts.push(Vertex { pos: cx, color: c });
        verts.push(Vertex { pos: p0, color: c });
        verts.push(Vertex { pos: p1, color: c });
    }
}

/// Push a ring of quads for a stroked circle.
fn push_stroked_circle(
    verts: &mut Vec<Vertex>,
    center: framewise::Vec2,
    radius: f32,
    color: Color,
    width: f32,
    win_size: (u32, u32),
) {
    let c = color_arr(color);
    let r_in = (radius - width * 0.5).max(0.0);
    let r_out = radius + width * 0.5;
    for i in 0..CIRCLE_SEGS {
        let a0 = (i as f32 / CIRCLE_SEGS as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / CIRCLE_SEGS as f32) * std::f32::consts::TAU;
        let i0 = to_clip(
            center.x + a0.cos() * r_in,
            center.y + a0.sin() * r_in,
            win_size.0,
            win_size.1,
        );
        let i1 = to_clip(
            center.x + a1.cos() * r_in,
            center.y + a1.sin() * r_in,
            win_size.0,
            win_size.1,
        );
        let o0 = to_clip(
            center.x + a0.cos() * r_out,
            center.y + a0.sin() * r_out,
            win_size.0,
            win_size.1,
        );
        let o1 = to_clip(
            center.x + a1.cos() * r_out,
            center.y + a1.sin() * r_out,
            win_size.0,
            win_size.1,
        );
        verts.push(Vertex { pos: i0, color: c });
        verts.push(Vertex { pos: o0, color: c });
        verts.push(Vertex { pos: o1, color: c });
        verts.push(Vertex { pos: i0, color: c });
        verts.push(Vertex { pos: o1, color: c });
        verts.push(Vertex { pos: i1, color: c });
    }
}

/// Generate vertices for a prepared text run.
fn push_text_run(
    verts: &mut Vec<TextVertex>,
    rect: Rect,
    color: Color,
    run: &crate::text::CachedLayout,
    text_system: &crate::text::SampleTextSystem,
    (sw, sh): (u32, u32),
) {
    let c = color_arr(color);
    let atlas_size = text_system.atlas_size as f32;

    for g in &run.glyphs {
        let key = crate::text::GlyphKey {
            font_id: run.font_id.0,
            glyph_index: g.key.glyph_index,
            size: (g.key.px * 10.0) as u32,
            subpixel_x: g.subpixel_x,
            weight: g.weight,
            opsz: g.opsz,
        };
        if let Some(info) = text_system.glyph_cache.get(&key) {
            let src = &info.atlas_rect;
            if src.w == 0 || src.h == 0 {
                continue;
            } // Space character

            // Snap screen quad coordinates to integer boundaries using .round().
            // With subpixel-baked bitmaps, snap/quantize the glyph origin first,
            // then add placement. This ensures correct alignment.
            let absolute_x = rect.x + g.x;
            let quantized_x = (absolute_x * 4.0).round() / 4.0; // Snap to subpixel bin
            let gx = quantized_x.floor() + info.left as f32;

            let absolute_y = rect.y + g.y;
            let gy = absolute_y.round() - info.top as f32;

            let gw = g.raster_w as f32;
            let gh = g.raster_h as f32;

            let tl_pos = to_clip(gx, gy, sw, sh);
            let tr_pos = to_clip(gx + gw, gy, sw, sh);
            let bl_pos = to_clip(gx, gy + gh, sw, sh);
            let br_pos = to_clip(gx + gw, gy + gh, sw, sh);

            // Source UV in atlas
            let u0 = src.x as f32 / atlas_size;
            let v0 = src.y as f32 / atlas_size;
            let u1 = (src.x + src.w) as f32 / atlas_size;
            let v1 = (src.y + src.h) as f32 / atlas_size;

            verts.push(TextVertex {
                pos: tl_pos,
                uv: [u0, v0],
                color: c,
            });
            verts.push(TextVertex {
                pos: bl_pos,
                uv: [u0, v1],
                color: c,
            });
            verts.push(TextVertex {
                pos: tr_pos,
                uv: [u1, v0],
                color: c,
            });

            verts.push(TextVertex {
                pos: tr_pos,
                uv: [u1, v0],
                color: c,
            });
            verts.push(TextVertex {
                pos: bl_pos,
                uv: [u0, v1],
                color: c,
            });
            verts.push(TextVertex {
                pos: br_pos,
                uv: [u1, v1],
                color: c,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::push_text_run;
    use crate::text::{GlyphKey, SampleTextSystem};
    use framewise::{Color, FontId, Rect, TextFlow, TextSystem};

    #[test]
    fn text_vertices_snap_origin_before_adding_glyph_placement() {
        let mut text_system = SampleTextSystem::new();
        text_system.begin_frame();

        let rect = Rect::new(10.0, 15.0, 180.0, 30.0);
        let layout = text_system.prepare(
            "Headless Test.",
            framewise::TextStyle::new(FontId(1), 14.0, 400, TextFlow::single_line()),
            rect,
        );
        let run = &text_system.runs[layout.handle.0];

        let glyph_index = run
            .glyphs
            .iter()
            .position(|g| g.parent == 'e')
            .expect("test string should contain an e glyph");
        let glyph = &run.glyphs[glyph_index];
        let key = GlyphKey {
            font_id: run.font_id.0,
            glyph_index: glyph.key.glyph_index,
            size: (glyph.key.px * 10.0) as u32,
            subpixel_x: glyph.subpixel_x,
            weight: glyph.weight,
            opsz: glyph.opsz,
        };
        let info = text_system
            .glyph_cache
            .get(&key)
            .expect("prepared glyph should be in cache");

        let mut verts = Vec::new();
        push_text_run(
            &mut verts,
            rect,
            Color::from_srgb_u8(0, 0, 0, 255),
            run,
            &text_system,
            (200, 50),
        );

        let actual_x = clip_x_to_pixels(verts[glyph_index * 6].pos[0], 200);
        let absolute_x = rect.x + glyph.x;
        let quantized_x = (absolute_x * 4.0).round() / 4.0;
        let expected_x = quantized_x.floor() + info.left as f32;

        assert_eq!(
            actual_x, expected_x,
            "glyph quad x should snap the quantized origin before adding placement.left"
        );
    }

    fn clip_x_to_pixels(x: f32, width: u32) -> f32 {
        ((x + 1.0) * 0.5 * width as f32).round()
    }
}
