// Minimal passthrough shader for solid-colour quads.
//
// Each vertex carries its own position (clip-space), colour, and depth.
// No textures, no uniforms — the renderer pre-transforms everything on the CPU.

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color:    vec4<f32>,
    @location(2) z:        f32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       color:         vec4<f32>,
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(in.position, in.z, 1.0);
    out.color         = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
