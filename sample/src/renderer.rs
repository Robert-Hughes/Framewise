use crate::text::PreparedGlyphResources;
use bytemuck::{Pod, Zeroable};
use framewise::{Color, DrawCmd, DrawCommands, DrawGlyph, Rect};
use wgpu::util::DeviceExt;

// ── Vertex layout ─────────────────────────────────────────────────────────────

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

/// One GPU vertex: 2D clip-space position + RGBA colour + depth.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub color: [f32; 4],
    pub z: f32,
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 3] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
        2 => Float32,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// One GPU vertex for text: 2D clip-space position + atlas UV + RGBA colour + depth.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct TextVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub z: f32,
}

impl TextVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 4] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x4,
        3 => Float32,
    ];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

/// One GPU primitive for analytical AA shapes: p0, p1, RGBA colour, width, radius, type, z.
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
pub struct ShapeData {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub color: [f32; 4],
    pub width: f32,
    pub radius: f32,
    pub shape_type: u32,
    pub z: f32,
}

pub const SHAPE_TYPE_LINE: u32 = 1;
pub const SHAPE_TYPE_FILL_CIRCLE: u32 = 2;
pub const SHAPE_TYPE_STROKE_CIRCLE: u32 = 3;
pub const SHAPE_TYPE_FILL_RECT: u32 = 4;
pub const SHAPE_TYPE_STROKE_RECT: u32 = 5;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Globals {
    window_size: [f32; 2],
    _pad: [f32; 2],
}

#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommand {
    DrawQuads(std::ops::Range<u32>),
    DrawText(std::ops::Range<u32>),
    DrawAA(std::ops::Range<u32>),
    SetScissor(Rect),
}

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    quad_pipeline: wgpu::RenderPipeline,
    text_pipeline: wgpu::RenderPipeline,
    aa_pipeline: wgpu::RenderPipeline,

    atlas_texture: wgpu::Texture,
    atlas_bind_group: wgpu::BindGroup,
    depth_target: Option<DepthTarget>,

    aa_bind_group_layout: wgpu::BindGroupLayout,
    globals_buf: wgpu::Buffer,
}

struct DepthTarget {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
    size: (u32, u32),
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
            depth_stencil: Some(depth_stencil_state()),
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
            depth_stencil: Some(depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        eprintln!(
            "[STARTUP]     create_render_pipeline (text) took {:?}",
            t5.elapsed()
        );

        let aa_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("aa_shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("aa.wgsl").into()),
        });

        let aa_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("aa_bind_group_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let aa_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("aa_pipeline_layout"),
            bind_group_layouts: &[&aa_bind_group_layout],
            push_constant_ranges: &[],
        });

        let aa_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("aa_pipeline"),
            layout: Some(&aa_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &aa_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &aa_shader,
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
            depth_stencil: Some(depth_stencil_state()),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let globals_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("globals_buf"),
            size: std::mem::size_of::<Globals>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            aa_pipeline,
            atlas_texture,
            atlas_bind_group,
            depth_target: None,
            aa_bind_group_layout,
            globals_buf,
        }
    }

    fn process_commands(
        cmds: &[DrawCmd],
        glyphs: &[DrawGlyph],
        window_size: (u32, u32),
        text_backend: &mut crate::text::SampleTextBackend,
    ) -> (
        Vec<Vertex>,
        Vec<TextVertex>,
        Vec<ShapeData>,
        Vec<RenderCommand>,
    ) {
        let mut quad_verts: Vec<Vertex> = Vec::new();
        let mut text_verts: Vec<TextVertex> = Vec::new();
        let mut aa_shapes: Vec<ShapeData> = Vec::new();
        let mut render_cmds = Vec::new();

        let mut current_quad_start = 0;
        let mut current_text_start = 0;
        let mut current_aa_start = 0;
        let mut clip_stack: Vec<Rect> = Vec::new();

        let flush_quads = |quad_verts_len: u32,
                           current_quad_start: &mut u32,
                           render_cmds: &mut Vec<RenderCommand>| {
            if quad_verts_len > *current_quad_start {
                render_cmds.push(RenderCommand::DrawQuads(
                    *current_quad_start..quad_verts_len,
                ));
                *current_quad_start = quad_verts_len;
            }
        };

        let flush_text = |text_verts_len: u32,
                          current_text_start: &mut u32,
                          render_cmds: &mut Vec<RenderCommand>| {
            if text_verts_len > *current_text_start {
                render_cmds.push(RenderCommand::DrawText(*current_text_start..text_verts_len));
                *current_text_start = text_verts_len;
            }
        };

        let flush_aa = |aa_shapes_len: u32,
                        current_aa_start: &mut u32,
                        render_cmds: &mut Vec<RenderCommand>| {
            if aa_shapes_len > *current_aa_start {
                render_cmds.push(RenderCommand::DrawAA(*current_aa_start..aa_shapes_len));
                *current_aa_start = aa_shapes_len;
            }
        };

        for cmd in cmds {
            match cmd {
                DrawCmd::FillRect {
                    rect,
                    color,
                    z,
                    anti_alias,
                } => {
                    if *anti_alias {
                        flush_quads(
                            quad_verts.len() as u32,
                            &mut current_quad_start,
                            &mut render_cmds,
                        );
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        aa_shapes.push(ShapeData {
                            p0: [rect.x, rect.y],
                            p1: [rect.x + rect.w, rect.y + rect.h],
                            color: color_arr(*color),
                            width: 0.0,
                            radius: 0.0,
                            shape_type: SHAPE_TYPE_FILL_RECT,
                            z: *z as f32,
                        });
                    } else {
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        flush_aa(
                            aa_shapes.len() as u32,
                            &mut current_aa_start,
                            &mut render_cmds,
                        );
                        push_filled_rect(&mut quad_verts, *rect, *color, *z, window_size);
                    }
                }
                DrawCmd::StrokeRect {
                    rect,
                    color,
                    width,
                    z,
                    anti_alias,
                } => {
                    if *anti_alias {
                        flush_quads(
                            quad_verts.len() as u32,
                            &mut current_quad_start,
                            &mut render_cmds,
                        );
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        aa_shapes.push(ShapeData {
                            p0: [rect.x, rect.y],
                            p1: [rect.x + rect.w, rect.y + rect.h],
                            color: color_arr(*color),
                            width: *width,
                            radius: 0.0,
                            shape_type: SHAPE_TYPE_STROKE_RECT,
                            z: *z as f32,
                        });
                    } else {
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        flush_aa(
                            aa_shapes.len() as u32,
                            &mut current_aa_start,
                            &mut render_cmds,
                        );
                        push_stroked_rect(&mut quad_verts, *rect, *color, *width, *z, window_size);
                    }
                }
                DrawCmd::StrokeLine {
                    p0,
                    p1,
                    color,
                    width,
                    z,
                    anti_alias,
                } => {
                    if *anti_alias {
                        flush_quads(
                            quad_verts.len() as u32,
                            &mut current_quad_start,
                            &mut render_cmds,
                        );
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        aa_shapes.push(ShapeData {
                            p0: [p0.x, p0.y],
                            p1: [p1.x, p1.y],
                            color: color_arr(*color),
                            width: *width,
                            radius: 0.0,
                            shape_type: SHAPE_TYPE_LINE,
                            z: *z as f32,
                        });
                    } else {
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        flush_aa(
                            aa_shapes.len() as u32,
                            &mut current_aa_start,
                            &mut render_cmds,
                        );
                        push_stroke_line(
                            &mut quad_verts,
                            *p0,
                            *p1,
                            *color,
                            *width,
                            *z,
                            window_size,
                        );
                    }
                }
                DrawCmd::FillCircle {
                    center,
                    radius,
                    color,
                    z,
                    anti_alias,
                } => {
                    if *anti_alias {
                        flush_quads(
                            quad_verts.len() as u32,
                            &mut current_quad_start,
                            &mut render_cmds,
                        );
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        aa_shapes.push(ShapeData {
                            p0: [center.x, center.y],
                            p1: [0.0, 0.0],
                            color: color_arr(*color),
                            width: 0.0,
                            radius: *radius,
                            shape_type: SHAPE_TYPE_FILL_CIRCLE,
                            z: *z as f32,
                        });
                    } else {
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        flush_aa(
                            aa_shapes.len() as u32,
                            &mut current_aa_start,
                            &mut render_cmds,
                        );
                        push_filled_circle(
                            &mut quad_verts,
                            *center,
                            *radius,
                            *color,
                            *z,
                            window_size,
                        );
                    }
                }
                DrawCmd::StrokeCircle {
                    center,
                    radius,
                    color,
                    width,
                    z,
                    anti_alias,
                } => {
                    if *anti_alias {
                        flush_quads(
                            quad_verts.len() as u32,
                            &mut current_quad_start,
                            &mut render_cmds,
                        );
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        aa_shapes.push(ShapeData {
                            p0: [center.x, center.y],
                            p1: [0.0, 0.0],
                            color: color_arr(*color),
                            width: *width,
                            radius: *radius,
                            shape_type: SHAPE_TYPE_STROKE_CIRCLE,
                            z: *z as f32,
                        });
                    } else {
                        flush_text(
                            text_verts.len() as u32,
                            &mut current_text_start,
                            &mut render_cmds,
                        );
                        flush_aa(
                            aa_shapes.len() as u32,
                            &mut current_aa_start,
                            &mut render_cmds,
                        );
                        push_stroked_circle(
                            &mut quad_verts,
                            *center,
                            *radius,
                            *color,
                            *width,
                            *z,
                            window_size,
                        );
                    }
                }
                DrawCmd::GlyphRun {
                    glyphs: glyph_range,
                    color,
                    z,
                } => {
                    flush_quads(
                        quad_verts.len() as u32,
                        &mut current_quad_start,
                        &mut render_cmds,
                    );
                    flush_aa(
                        aa_shapes.len() as u32,
                        &mut current_aa_start,
                        &mut render_cmds,
                    );
                    if let Some(run_glyphs) = glyphs.get(glyph_range.clone()) {
                        push_glyph_run(
                            &mut text_verts,
                            run_glyphs,
                            *color,
                            *z,
                            text_backend,
                            window_size,
                        );
                    }
                }
                DrawCmd::PushClip { rect } => {
                    flush_quads(
                        quad_verts.len() as u32,
                        &mut current_quad_start,
                        &mut render_cmds,
                    );
                    flush_text(
                        text_verts.len() as u32,
                        &mut current_text_start,
                        &mut render_cmds,
                    );
                    flush_aa(
                        aa_shapes.len() as u32,
                        &mut current_aa_start,
                        &mut render_cmds,
                    );

                    let new_clip = if let Some(current) = clip_stack.last() {
                        current.intersect(rect)
                    } else {
                        *rect
                    };
                    clip_stack.push(new_clip);
                    render_cmds.push(RenderCommand::SetScissor(new_clip));
                }
                DrawCmd::PopClip => {
                    flush_quads(
                        quad_verts.len() as u32,
                        &mut current_quad_start,
                        &mut render_cmds,
                    );
                    flush_text(
                        text_verts.len() as u32,
                        &mut current_text_start,
                        &mut render_cmds,
                    );
                    flush_aa(
                        aa_shapes.len() as u32,
                        &mut current_aa_start,
                        &mut render_cmds,
                    );

                    clip_stack.pop();
                    let new_clip = clip_stack.last().copied().unwrap_or_else(|| {
                        Rect::new(0.0, 0.0, window_size.0 as f32, window_size.1 as f32)
                    });
                    render_cmds.push(RenderCommand::SetScissor(new_clip));
                }
            }
        }

        flush_quads(
            quad_verts.len() as u32,
            &mut current_quad_start,
            &mut render_cmds,
        );
        flush_text(
            text_verts.len() as u32,
            &mut current_text_start,
            &mut render_cmds,
        );
        flush_aa(
            aa_shapes.len() as u32,
            &mut current_aa_start,
            &mut render_cmds,
        );

        (quad_verts, text_verts, aa_shapes, render_cmds)
    }

    /// Convert a list of `DrawCmd`s into vertices and render them.
    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: &wgpu::TextureView,
        encoder: &mut wgpu::CommandEncoder,
        draw_commands: &DrawCommands,
        window_size: (u32, u32),
        text_backend: &mut crate::text::SampleTextBackend,
    ) {
        if text_backend.atlas_dirty {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &text_backend.atlas_data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(text_backend.atlas_size),
                    rows_per_image: Some(text_backend.atlas_size),
                },
                wgpu::Extent3d {
                    width: text_backend.atlas_size,
                    height: text_backend.atlas_size,
                    depth_or_array_layers: 1,
                },
            );
            text_backend.atlas_dirty = false;
        }

        let (quad_verts, text_verts, aa_shapes, render_cmds) = Self::process_commands(
            draw_commands.commands(),
            draw_commands.glyphs(),
            window_size,
            text_backend,
        );

        if quad_verts.is_empty() && text_verts.is_empty() && aa_shapes.is_empty() {
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

        // Write Globals Uniform
        let globals = Globals {
            window_size: [window_size.0 as f32, window_size.1 as f32],
            _pad: [0.0; 2],
        };
        queue.write_buffer(&self.globals_buf, 0, bytemuck::bytes_of(&globals));

        // Create ShapeData storage buffer
        let aa_sbuf = if !aa_shapes.is_empty() {
            Some(
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("aa_sbuf"),
                    contents: bytemuck::cast_slice(&aa_shapes),
                    usage: wgpu::BufferUsages::STORAGE,
                }),
            )
        } else {
            None
        };

        // Create bind group for AA
        let aa_bind_group = aa_sbuf.as_ref().map(|sbuf| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("aa_bind_group"),
                layout: &self.aa_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: sbuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.globals_buf.as_entire_binding(),
                    },
                ],
            })
        });

        let depth_view = &self.ensure_depth_target(device, window_size).view;

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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        let mut last_pipeline = 0; // 1 = quads, 2 = text, 3 = aa

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
                RenderCommand::DrawAA(range) => {
                    if last_pipeline != 3 {
                        pass.set_pipeline(&self.aa_pipeline);
                        pass.set_bind_group(0, aa_bind_group.as_ref().unwrap(), &[]);
                        last_pipeline = 3;
                    }
                    pass.draw(0..6, range);
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

    fn ensure_depth_target(&mut self, device: &wgpu::Device, size: (u32, u32)) -> &DepthTarget {
        let needs_recreate = self
            .depth_target
            .as_ref()
            .is_none_or(|target| target.size != size);

        if needs_recreate {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("depth_texture"),
                size: wgpu::Extent3d {
                    width: size.0.max(1),
                    height: size.1.max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
            self.depth_target = Some(DepthTarget {
                _texture: texture,
                view,
                size,
            });
        }

        self.depth_target
            .as_ref()
            .expect("depth target should exist after creation")
    }
}

// ── Geometry helpers ──────────────────────────────────────────────────────────

fn depth_stencil_state() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: DEPTH_FORMAT,
        depth_write_enabled: true,
        depth_compare: wgpu::CompareFunction::GreaterEqual,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

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

fn z_to_depth(z: u32) -> f32 {
    // This stores the command z redundantly in every vertex so the renderer can
    // keep its existing broad batching. If vertex bandwidth becomes the bottleneck,
    // batch by (pipeline, clip, z) and pass z via a uniform or push constant instead.
    z as f32 / u32::MAX as f32
}

/// Push two triangles (six vertices) for a filled rectangle.
fn push_filled_rect(
    verts: &mut Vec<Vertex>,
    rect: Rect,
    color: Color,
    z: u32,
    (sw, sh): (u32, u32),
) {
    let tl = to_clip(rect.x, rect.y, sw, sh);
    let tr = to_clip(rect.x + rect.w, rect.y, sw, sh);
    let bl = to_clip(rect.x, rect.y + rect.h, sw, sh);
    let br = to_clip(rect.x + rect.w, rect.y + rect.h, sw, sh);
    let c = color_arr(color);
    let z = z_to_depth(z);

    // Two CCW triangles.
    verts.push(Vertex {
        pos: tl,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: bl,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: tr,
        color: c,
        z,
    });

    verts.push(Vertex {
        pos: tr,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: bl,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: br,
        color: c,
        z,
    });
}

/// Push eight thin filled rects (one per side) to approximate a stroked rect.
fn push_stroked_rect(
    verts: &mut Vec<Vertex>,
    rect: Rect,
    color: Color,
    width: f32,
    z: u32,
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
        push_filled_rect(verts, *s, color, z, win_size);
    }
}

/// Push two triangles for a line segment of a given width (screen-aligned cap).
fn push_stroke_line(
    verts: &mut Vec<Vertex>,
    p0: framewise::Vec2,
    p1: framewise::Vec2,
    color: Color,
    width: f32,
    z: u32,
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
    let z = z_to_depth(z);

    verts.push(Vertex {
        pos: a,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: b,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: c2,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: a,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: c2,
        color: c,
        z,
    });
    verts.push(Vertex {
        pos: d,
        color: c,
        z,
    });
}

const CIRCLE_SEGS: usize = 32;

/// Push a triangle fan for a filled circle.
fn push_filled_circle(
    verts: &mut Vec<Vertex>,
    center: framewise::Vec2,
    radius: f32,
    color: Color,
    z: u32,
    win_size: (u32, u32),
) {
    let c = color_arr(color);
    let z = z_to_depth(z);
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
        verts.push(Vertex {
            pos: cx,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: p0,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: p1,
            color: c,
            z,
        });
    }
}

/// Push a ring of quads for a stroked circle.
fn push_stroked_circle(
    verts: &mut Vec<Vertex>,
    center: framewise::Vec2,
    radius: f32,
    color: Color,
    width: f32,
    z: u32,
    win_size: (u32, u32),
) {
    let c = color_arr(color);
    let z = z_to_depth(z);
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
        verts.push(Vertex {
            pos: i0,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: o0,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: o1,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: i0,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: o1,
            color: c,
            z,
        });
        verts.push(Vertex {
            pos: i1,
            color: c,
            z,
        });
    }
}

/// Generate vertices for a range of prepared glyph arena entries.
fn push_glyph_run(
    verts: &mut Vec<TextVertex>,
    glyphs: &[DrawGlyph],
    color: Color,
    z: u32,
    text_backend: &crate::text::SampleTextBackend,
    (sw, sh): (u32, u32),
) {
    let c = color_arr(color);
    let z = z_to_depth(z);
    let atlas_size = text_backend.atlas_size as f32;

    for glyph in glyphs {
        let Some(image) = text_backend.resolve_glyph(glyph.handle) else {
            continue;
        };
        let src = image.atlas_rect;
        if src.w == 0 || src.h == 0 {
            continue;
        }

        let gx = glyph.top_left.x;
        let gy = glyph.top_left.y;
        let gw = src.w as f32;
        let gh = src.h as f32;

        let tl_pos = to_clip(gx, gy, sw, sh);
        let tr_pos = to_clip(gx + gw, gy, sw, sh);
        let bl_pos = to_clip(gx, gy + gh, sw, sh);
        let br_pos = to_clip(gx + gw, gy + gh, sw, sh);

        let u0 = src.x as f32 / atlas_size;
        let v0 = src.y as f32 / atlas_size;
        let u1 = (src.x + src.w) as f32 / atlas_size;
        let v1 = (src.y + src.h) as f32 / atlas_size;

        verts.push(TextVertex {
            pos: tl_pos,
            uv: [u0, v0],
            color: c,
            z,
        });
        verts.push(TextVertex {
            pos: bl_pos,
            uv: [u0, v1],
            color: c,
            z,
        });
        verts.push(TextVertex {
            pos: tr_pos,
            uv: [u1, v0],
            color: c,
            z,
        });

        verts.push(TextVertex {
            pos: tr_pos,
            uv: [u1, v0],
            color: c,
            z,
        });
        verts.push(TextVertex {
            pos: bl_pos,
            uv: [u0, v1],
            color: c,
            z,
        });
        verts.push(TextVertex {
            pos: br_pos,
            uv: [u1, v1],
            color: c,
            z,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::push_glyph_run;
    use crate::text::{GlyphKey, SampleTextBackend};
    use framewise::{Color, DrawCommands, DrawGlyph, Vec2};

    #[test]
    fn glyph_run_vertices_use_draw_glyph_top_left_and_resolved_atlas_size() {
        let mut text_backend = SampleTextBackend::new();
        let key = GlyphKey {
            font_id: 1,
            glyph_index: 43,
            size: 140,
            subpixel_x: 0,
            weight: 400,
            opsz: 14,
        };
        let handle = text_backend.prepare_glyph_handle(key);
        let image = text_backend.glyph_cache.get(&key).unwrap();
        assert!(image.atlas_rect.w > 0);
        assert!(image.atlas_rect.h > 0);

        let mut verts = Vec::new();
        push_glyph_run(
            &mut verts,
            &[DrawGlyph {
                handle,
                top_left: Vec2::new(25.0, 11.0),
            }],
            Color::from_srgb_u8(10, 20, 30, 255),
            7,
            &text_backend,
            (200, 100),
        );

        assert_eq!(verts.len(), 6);
        assert_eq!(clip_x_to_pixels(verts[0].pos[0], 200), 25.0);
        assert_eq!(clip_y_to_pixels(verts[0].pos[1], 100), 11.0);
        assert_eq!(
            clip_x_to_pixels(verts[5].pos[0], 200),
            25.0 + image.atlas_rect.w as f32
        );
        assert_eq!(
            clip_y_to_pixels(verts[5].pos[1], 100),
            11.0 + image.atlas_rect.h as f32
        );
    }

    #[test]
    fn process_commands_draws_glyph_runs_from_arena() {
        let mut text_backend = SampleTextBackend::new();
        let handle = text_backend.prepare_glyph_handle(GlyphKey {
            font_id: 1,
            glyph_index: 43,
            size: 140,
            subpixel_x: 0,
            weight: 400,
            opsz: 14,
        });
        let mut cmds = DrawCommands::new();
        cmds.push_glyph_run(
            [DrawGlyph {
                handle,
                top_left: Vec2::new(4.0, 8.0),
            }],
            Color::from_srgb_u8(0, 0, 0, 255),
            3,
        );

        let (_, text_verts, _, render_cmds) = super::Renderer::process_commands(
            cmds.commands(),
            cmds.glyphs(),
            (100, 100),
            &mut text_backend,
        );

        assert_eq!(text_verts.len(), 6);
        assert!(
            matches!(render_cmds.as_slice(), [super::RenderCommand::DrawText(range)] if range == &(0..6))
        );
    }

    #[test]
    fn test_aa_batching_logic() {
        use super::{RenderCommand, Renderer};
        use framewise::{Color, DrawCmd, Rect, Vec2};

        let mut text_backend = SampleTextBackend::new();
        text_backend.begin_frame();

        // Create a list of mixed commands to test batching/interleaving boundaries
        let cmds = vec![
            // 1. Opaque quads
            DrawCmd::FillRect {
                rect: Rect::new(10.0, 10.0, 100.0, 100.0),
                color: Color::from_srgb_u8(255, 0, 0, 255),
                z: 0,
                anti_alias: false,
            },
            DrawCmd::FillRect {
                rect: Rect::new(20.0, 20.0, 50.0, 50.0),
                color: Color::from_srgb_u8(0, 255, 0, 255),
                z: 1,
                anti_alias: false,
            },
            // 2. AA shape
            DrawCmd::StrokeLine {
                p0: Vec2::new(0.0, 0.0),
                p1: Vec2::new(100.0, 100.0),
                color: Color::from_srgb_u8(0, 0, 0, 255),
                width: 2.0,
                z: 2,
                anti_alias: true,
            },
            // 3. Clip push
            DrawCmd::PushClip {
                rect: Rect::new(0.0, 0.0, 50.0, 50.0),
            },
            // 4. AA shape inside clip
            DrawCmd::FillCircle {
                center: Vec2::new(25.0, 25.0),
                radius: 10.0,
                color: Color::from_srgb_u8(0, 0, 255, 255),
                z: 3,
                anti_alias: true,
            },
            // 5. Opaque quad inside clip
            DrawCmd::FillRect {
                rect: Rect::new(5.0, 5.0, 10.0, 10.0),
                color: Color::from_srgb_u8(255, 255, 0, 255),
                z: 4,
                anti_alias: false,
            },
            // 6. Clip pop
            DrawCmd::PopClip,
        ];

        let (quad_verts, text_verts, aa_shapes, render_cmds) =
            Renderer::process_commands(&cmds, &[], (800, 600), &mut text_backend);

        // Expect:
        // - 3 non-AA rects total -> 3 * 6 = 18 quad_verts
        assert_eq!(quad_verts.len(), 18);
        // - 0 text_verts
        assert_eq!(text_verts.len(), 0);
        // - 2 AA shapes total (Line and Circle)
        assert_eq!(aa_shapes.len(), 2);

        // Let's inspect the batched RenderCommands
        // 1. Quads batch for the first two rects: DrawQuads(0..12) (2 rects * 6 vertices)
        // 2. AA batch for the first StrokeLine: DrawAA(0..1)
        // 3. SetScissor
        // 4. AA batch for the FillCircle inside clip: DrawAA(1..2)
        // 5. Quads batch for the FillRect inside clip: DrawQuads(12..18)
        // 6. SetScissor (restoring clip)

        assert_eq!(render_cmds.len(), 6);
        assert!(matches!(render_cmds[0], RenderCommand::DrawQuads(ref r) if r == &(0..12)));
        assert!(matches!(render_cmds[1], RenderCommand::DrawAA(ref r) if r == &(0..1)));
        assert!(matches!(render_cmds[2], RenderCommand::SetScissor(_)));
        assert!(matches!(render_cmds[3], RenderCommand::DrawAA(ref r) if r == &(1..2)));
        assert!(matches!(render_cmds[4], RenderCommand::DrawQuads(ref r) if r == &(12..18)));
        assert!(matches!(render_cmds[5], RenderCommand::SetScissor(_)));
    }

    fn clip_x_to_pixels(x: f32, width: u32) -> f32 {
        ((x + 1.0) * 0.5 * width as f32).round()
    }

    fn clip_y_to_pixels(y: f32, height: u32) -> f32 {
        ((1.0 - y) * 0.5 * height as f32).round()
    }
}
