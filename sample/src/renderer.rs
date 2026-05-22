use bytemuck::{Pod, Zeroable};
use framewise::{Color, DrawCmd, Rect};
use wgpu::util::DeviceExt;

// ── Vertex layout ─────────────────────────────────────────────────────────────

/// One GPU vertex: 2D clip-space position + RGBA colour.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos:   [f32; 2],
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
            step_mode:    wgpu::VertexStepMode::Vertex,
            attributes:   &Self::ATTRIBS,
        }
    }
}

/// One GPU vertex for text: 2D clip-space position + atlas UV + RGBA colour.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TextVertex {
    pub pos:   [f32; 2],
    pub uv:    [f32; 2],
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
            step_mode:    wgpu::VertexStepMode::Vertex,
            attributes:   &Self::ATTRIBS,
        }
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    quad_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    
    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    atlas_bind_group_layout: wgpu::BindGroupLayout,
}

impl Renderer {
    pub fn new(
        device:      &wgpu::Device,
        surface_fmt: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("quad_shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("shader.wgsl").into(),
            ),
        });

        let pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label:                Some("pipeline_layout"),
                bind_group_layouts:   &[],
                push_constant_ranges: &[],
            });

        let quad_pipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label:  Some("quad_pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module:      &shader,
                    entry_point: Some("vs_main"),
                    buffers:     &[Vertex::desc()],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module:      &shader,
                    entry_point: Some("fs_main"),
                    targets:     &[Some(wgpu::ColorTargetState {
                        format:     surface_fmt,
                        blend:      Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology:           wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face:         wgpu::FrontFace::Ccw,
                    cull_mode:          None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample:   wgpu::MultisampleState::default(),
                multiview:     None,
                cache:         None,
            });        // --- Text Pipeline Setup ---
        let text_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label:  Some("text_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
        });
        
        let atlas_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("text_pipeline_layout"),
            bind_group_layouts: &[&atlas_bind_group_layout],
            push_constant_ranges: &[],
        });

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

        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("atlas_texture"),
            size: wgpu::Extent3d { width: 1024, height: 1024, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("atlas_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let atlas_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("atlas_bind_group"),
            layout: &atlas_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&atlas_view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&atlas_sampler) },
            ],
        });

        Self { quad_pipeline, text_pipeline, atlas_texture, atlas_bind_group, atlas_bind_group_layout }
    }

    /// Convert a list of `DrawCmd`s into vertices and render them.
    pub fn render(
        &mut self,
        device:       &wgpu::Device,
        queue:        &wgpu::Queue,
        view:         &wgpu::TextureView,
        encoder:      &mut wgpu::CommandEncoder,
        cmds:         &[DrawCmd],
        window_size:  (u32, u32),
        text_system:  &mut crate::text::SampleTextSystem,
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

        for cmd in cmds {
            match cmd {
                DrawCmd::FillRect { rect, color } => {
                    push_filled_rect(&mut quad_verts, *rect, *color, window_size);
                }
                DrawCmd::StrokeRect { rect, color, width } => {
                    push_stroked_rect(&mut quad_verts, *rect, *color, *width, window_size);
                }
                DrawCmd::Text { rect, color, handle } => {
                    if let Some(run) = text_system.runs.get(handle.0) {
                        push_text_run(&mut text_verts, *rect, *color, run, text_system, window_size);
                    }
                }
            }
        }

        if quad_verts.is_empty() && text_verts.is_empty() {
            return;
        }

        let quad_vbuf = if !quad_verts.is_empty() {
            Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label:    Some("quad_vbuf"),
                contents: bytemuck::cast_slice(&quad_verts),
                usage:    wgpu::BufferUsages::VERTEX,
            }))
        } else { None };

        let text_vbuf = if !text_verts.is_empty() {
            Some(device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label:    Some("text_vbuf"),
                contents: bytemuck::cast_slice(&text_verts),
                usage:    wgpu::BufferUsages::VERTEX,
            }))
        } else { None };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load:  wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.08, g: 0.08, b: 0.10, a: 1.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes:         None,
            occlusion_query_set:      None,
        });

        if let Some(vbuf) = quad_vbuf {
            pass.set_pipeline(&self.quad_pipeline);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..quad_verts.len() as u32, 0..1);
        }

        if let Some(vbuf) = text_vbuf {
            pass.set_pipeline(&self.text_pipeline);
            pass.set_bind_group(0, &self.atlas_bind_group, &[]);
            pass.set_vertex_buffer(0, vbuf.slice(..));
            pass.draw(0..text_verts.len() as u32, 0..1);
        }
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

/// Convert a logical-pixel rect to clip-space [-1, 1].
fn to_clip(x: f32, y: f32, w: u32, h: u32) -> [f32; 2] {
    [
        (x / w as f32) * 2.0 - 1.0,
        1.0 - (y / h as f32) * 2.0,   // y-flip: window top → clip top
    ]
}

fn color_arr(c: Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

/// Push two triangles (six vertices) for a filled rectangle.
fn push_filled_rect(
    verts:       &mut Vec<Vertex>,
    rect:        Rect,
    color:       Color,
    (sw, sh):    (u32, u32),
) {
    let tl = to_clip(rect.x,           rect.y,           sw, sh);
    let tr = to_clip(rect.x + rect.w,  rect.y,           sw, sh);
    let bl = to_clip(rect.x,           rect.y + rect.h,  sw, sh);
    let br = to_clip(rect.x + rect.w,  rect.y + rect.h,  sw, sh);
    let c  = color_arr(color);

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
    verts:    &mut Vec<Vertex>,
    rect:     Rect,
    color:    Color,
    width:    f32,
    win_size: (u32, u32),
) {
    let x = rect.x;
    let y = rect.y;
    let w = rect.w;
    let h = rect.h;
    let lw = width;

    // Top, bottom, left, right strips.
    let strips = [
        Rect::new(x,           y,           w,  lw), // top
        Rect::new(x,           y + h - lw,  w,  lw), // bottom
        Rect::new(x,           y,           lw, h),  // left
        Rect::new(x + w - lw,  y,           lw, h),  // right
    ];
    for s in &strips {
        push_filled_rect(verts, *s, color, win_size);
    }
}

/// Generate vertices for a prepared text run.
fn push_text_run(
    verts:       &mut Vec<TextVertex>,
    rect:        Rect,
    color:       Color,
    run:         &crate::text::CachedLayout,
    text_system: &crate::text::SampleTextSystem,
    (sw, sh):    (u32, u32),
) {
    let c = color_arr(color);
    let atlas_size = text_system.atlas_size as f32;

    for g in &run.glyphs {
        let key = crate::text::GlyphKey { glyph_index: g.key.glyph_index, size: (g.key.px * 10.0) as u32 };
        if let Some(info) = text_system.glyph_cache.get(&key) {
            let src = &info.atlas_rect;
            if src.w == 0 || src.h == 0 { continue; } // Space character
            
            // Destination rect on screen
            let gx = rect.x + g.x;
            let gy = rect.y + g.y;
            let gw = g.width as f32;
            let gh = g.height as f32;
            
            let tl_pos = to_clip(gx, gy, sw, sh);
            let tr_pos = to_clip(gx + gw, gy, sw, sh);
            let bl_pos = to_clip(gx, gy + gh, sw, sh);
            let br_pos = to_clip(gx + gw, gy + gh, sw, sh);
            
            // Source UV in atlas
            let u0 = src.x as f32 / atlas_size;
            let v0 = src.y as f32 / atlas_size;
            let u1 = (src.x + src.w) as f32 / atlas_size;
            let v1 = (src.y + src.h) as f32 / atlas_size;
            
            verts.push(TextVertex { pos: tl_pos, uv: [u0, v0], color: c });
            verts.push(TextVertex { pos: bl_pos, uv: [u0, v1], color: c });
            verts.push(TextVertex { pos: tr_pos, uv: [u1, v0], color: c });
            
            verts.push(TextVertex { pos: tr_pos, uv: [u1, v0], color: c });
            verts.push(TextVertex { pos: bl_pos, uv: [u0, v1], color: c });
            verts.push(TextVertex { pos: br_pos, uv: [u1, v1], color: c });
        }
    }
}
