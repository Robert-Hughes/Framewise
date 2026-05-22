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

// ── Renderer ──────────────────────────────────────────────────────────────────

pub struct Renderer {
    pipeline: wgpu::RenderPipeline,
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

        let pipeline =
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
            });

        Self { pipeline }
    }

    /// Convert a list of `DrawCmd`s into vertices and render them.
    pub fn render(
        &self,
        device:       &wgpu::Device,
        _queue:       &wgpu::Queue,
        view:         &wgpu::TextureView,
        encoder:      &mut wgpu::CommandEncoder,
        cmds:         &[DrawCmd],
        window_size:  (u32, u32),
    ) {
        let mut vertices: Vec<Vertex> = Vec::new();

        for cmd in cmds {
            match cmd {
                DrawCmd::FillRect { rect, color } => {
                    push_filled_rect(&mut vertices, *rect, *color, window_size);
                }
                DrawCmd::StrokeRect { rect, color, width } => {
                    push_stroked_rect(&mut vertices, *rect, *color, *width, window_size);
                }
                DrawCmd::TextStub { rect, color } => {
                    // Render as a dim tinted bar until real text is available.
                    let stub_color = Color::new(
                        color.r * 0.6,
                        color.g * 0.6,
                        color.b * 0.6,
                        color.a * 0.5,
                    );
                    push_filled_rect(&mut vertices, *rect, stub_color, window_size);
                }
            }
        }

        if vertices.is_empty() {
            return;
        }

        let vbuf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label:    Some("quad_vbuf"),
            contents: bytemuck::cast_slice(&vertices),
            usage:    wgpu::BufferUsages::VERTEX,
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("quad_pass"),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, vbuf.slice(..));
        pass.draw(0..vertices.len() as u32, 0..1);
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
