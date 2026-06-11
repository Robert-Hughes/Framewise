struct ShapeData {
    p0: vec2<f32>,
    p1: vec2<f32>,
    color: vec4<f32>,
    width: f32,
    radius: f32,
    shape_type: u32, // 1 = Line, 2 = FillCircle, 3 = StrokeCircle, 4 = FillRect, 5 = StrokeRect
    z: f32,
}

struct Globals {
    window_size: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<storage, read> shapes: array<ShapeData>;
@group(0) @binding(1) var<uniform> globals: Globals;

const SHAPE_TYPE_LINE: u32 = 1u;
const SHAPE_TYPE_FILL_CIRCLE: u32 = 2u;
const SHAPE_TYPE_STROKE_CIRCLE: u32 = 3u;
const SHAPE_TYPE_FILL_RECT: u32 = 4u;
const SHAPE_TYPE_STROKE_RECT: u32 = 5u;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0)       color:         vec4<f32>,
    @location(1)       local_pos:     vec2<f32>,
    @location(2)       p0:            vec2<f32>,
    @location(3)       p1:            vec2<f32>,
    @location(4)       width:         f32,
    @location(5)       radius:        f32,
    @location(6)       @interpolate(flat) shape_type: u32,
}

fn get_local_uv(vertex_idx: u32) -> vec2<f32> {
    var coords = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>( 1.0, -1.0),
        vec2<f32>(-1.0,  1.0),
        vec2<f32>( 1.0,  1.0)
    );
    return coords[vertex_idx];
}

fn to_clip(pos: vec2<f32>, win_size: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(
        (pos.x / win_size.x) * 2.0 - 1.0,
        1.0 - (pos.y / win_size.y) * 2.0
    );
}

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_idx: u32,
) -> VertexOutput {
    let shape = shapes[instance_idx];
    let uv = get_local_uv(vertex_idx);

    var q = vec2<f32>(0.0);

    if (shape.shape_type == SHAPE_TYPE_LINE) {
        // Line
        let v = shape.p1 - shape.p0;
        let L = length(v);
        if (L < 0.001) {
            q = shape.p0;
        } else {
            let u = v / L;
            let n = vec2<f32>(-u.y, u.x);
            let half_w = shape.width * 0.5;
            let expand_n = half_w + 1.0;
            let expand_u = 1.0;
            
            if (uv.x < 0.0) {
                q = shape.p0 - expand_u * u + uv.y * expand_n * n;
            } else {
                q = shape.p1 + expand_u * u + uv.y * expand_n * n;
            }
        }
    } else if (shape.shape_type == SHAPE_TYPE_FILL_CIRCLE || shape.shape_type == SHAPE_TYPE_STROKE_CIRCLE) {
        // Circle (Fill or Stroke)
        var r_outer = shape.radius;
        if (shape.shape_type == SHAPE_TYPE_STROKE_CIRCLE) {
            r_outer = shape.radius + shape.width * 0.5;
        }
        let expand = r_outer + 1.0;
        q = shape.p0 + uv * expand;
    } else if (shape.shape_type == SHAPE_TYPE_FILL_RECT || shape.shape_type == SHAPE_TYPE_STROKE_RECT) {
        // Rect (Fill or Stroke)
        var expand = 1.0;
        if (shape.shape_type == SHAPE_TYPE_STROKE_RECT) {
            expand = shape.width * 0.5 + 1.0;
        }
        let p0_expanded = shape.p0 - vec2<f32>(expand);
        let p1_expanded = shape.p1 + vec2<f32>(expand);
        let t = (uv + vec2<f32>(1.0)) * 0.5;
        q = mix(p0_expanded, p1_expanded, t);
    }

    var out: VertexOutput;
    let depth = shape.z / 4294967295.0; // z_to_depth
    out.clip_position = vec4<f32>(to_clip(q, globals.window_size), depth, 1.0);
    out.color = shape.color;
    out.local_pos = q;
    out.p0 = shape.p0;
    out.p1 = shape.p1;
    out.width = shape.width;
    out.radius = shape.radius;
    out.shape_type = shape.shape_type;
    return out;
}

fn sdf_box_signed(q: vec2<f32>, c: vec2<f32>, h: vec2<f32>) -> f32 {
    let p = abs(q - c);
    let d = p - h;
    return length(max(d, vec2<f32>(0.0))) + min(max(d.x, d.y), 0.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var coverage = 1.0;

    if (in.shape_type == SHAPE_TYPE_LINE) {
        // Line Segment AA
        let v = in.p1 - in.p0;
        let L = length(v);
        if (L > 0.001) {
            let u = v / L;
            let proj = dot(in.local_pos - in.p0, u);
            let t = clamp(proj, 0.0, L);
            let closest = in.p0 + t * u;
            let d = distance(in.local_pos, closest);
            
            let w = in.width;
            let w_eff = max(w, 1.0);
            coverage = clamp((w_eff + 1.0) * 0.5 - d, 0.0, 1.0) * (w / w_eff);
        }
    } else if (in.shape_type == SHAPE_TYPE_FILL_CIRCLE) {
        // Fill Circle AA
        let r = distance(in.local_pos, in.p0);
        let R = in.radius;
        let R_eff = max(R, 1.0);
        coverage = clamp(R_eff + 0.5 - r, 0.0, 1.0) * (R / R_eff);
    } else if (in.shape_type == SHAPE_TYPE_STROKE_CIRCLE) {
        // Stroke Circle AA
        let r = distance(in.local_pos, in.p0);
        let d = abs(r - in.radius);
        let w = in.width;
        let w_eff = max(w, 1.0);
        coverage = clamp((w_eff + 1.0) * 0.5 - d, 0.0, 1.0) * (w / w_eff);
    } else if (in.shape_type == SHAPE_TYPE_FILL_RECT) {
        // Fill Rect AA
        let size = in.p1 - in.p0;
        let center = (in.p0 + in.p1) * 0.5;
        let half_size = size * 0.5;
        let d = sdf_box_signed(in.local_pos, center, half_size);
        
        let w_eff_x = max(size.x, 1.0);
        let w_eff_y = max(size.y, 1.0);
        
        let cov_x = clamp((w_eff_x + 1.0) * 0.5 - abs(in.local_pos.x - center.x), 0.0, 1.0) * (size.x / w_eff_x);
        let cov_y = clamp((w_eff_y + 1.0) * 0.5 - abs(in.local_pos.y - center.y), 0.0, 1.0) * (size.y / w_eff_y);
        
        if (size.x < 1.0 || size.y < 1.0) {
            coverage = cov_x * cov_y;
        } else {
            coverage = clamp(0.5 - d, 0.0, 1.0);
        }
    } else if (in.shape_type == SHAPE_TYPE_STROKE_RECT) {
        // Stroke Rect AA
        let size = in.p1 - in.p0;
        let center = (in.p0 + in.p1) * 0.5;
        // The stroke is centered on the input rectangle bounds
        let half_size = size * 0.5;
        let d_solid = sdf_box_signed(in.local_pos, center, half_size);
        let d = abs(d_solid);
        
        let w = in.width;
        let w_eff = max(w, 1.0);
        coverage = clamp((w_eff + 1.0) * 0.5 - d, 0.0, 1.0) * (w / w_eff);
    }

    if (coverage <= 0.0) {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
