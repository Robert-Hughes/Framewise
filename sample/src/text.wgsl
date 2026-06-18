struct VertexInput {
    @location(0) dst_pos:  vec2<f32>,
    @location(1) dst_size: vec2<f32>,
    @location(2) src_pos:  vec2<f32>,
    @location(3) src_size: vec2<f32>,
    @location(4) color:    vec4<f32>,
    @location(5) z:        f32,
}

struct Globals {
    window_size: vec2<f32>,
    atlas_size:  f32,
    _pad:        f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       uv:            vec2<f32>,
    @location(1)       color:         vec4<f32>,
}

@group(0) @binding(2) var<uniform> globals: Globals;

@vertex
fn vs_main(in: VertexInput, @builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let corner = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    )[vertex_index];

    let pixel_pos = in.dst_pos + corner * in.dst_size;
    let clip_pos = vec2<f32>(
        pixel_pos.x / globals.window_size.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / globals.window_size.y * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4<f32>(clip_pos, in.z, 1.0);
    out.uv = (in.src_pos + corner * in.src_size) / globals.atlas_size;
    out.color = in.color;
    return out;
}

@group(0) @binding(0) var t_atlas: texture_2d<f32>;
@group(0) @binding(1) var s_atlas: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // The atlas stores coverage in the red channel.
    let coverage = textureSample(t_atlas, s_atlas, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
