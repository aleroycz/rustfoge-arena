struct ScreenUniform {
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) radius: f32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) half_size: vec2<f32>,
    @location(2) radius: f32,
    @location(3) color: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vidx: u32, instance: InstanceInput) -> VertexOutput {
    var corners = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0),
    );
    let corner = corners[vidx];
    let world = instance.pos + corner * instance.size;

    var out: VertexOutput;
    out.clip_position = vec4<f32>(
        (world.x / screen.size.x) * 2.0 - 1.0,
        1.0 - (world.y / screen.size.y) * 2.0,
        0.0,
        1.0,
    );
    out.local = (corner - vec2<f32>(0.5)) * instance.size;
    out.half_size = instance.size * 0.5;
    out.radius = min(instance.radius, min(instance.size.x, instance.size.y) * 0.5);
    out.color = instance.color;
    return out;
}

fn chamfer_rect_sdf(p: vec2<f32>, half_size: vec2<f32>, cut: f32) -> f32 {
    let q = abs(p) - half_size;
    let box_dist = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);
    let diag_dist = (q.x + q.y + cut) * 0.70710678;
    return max(box_dist, diag_dist);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let dist = chamfer_rect_sdf(in.local, in.half_size, in.radius);
    let alpha = 1.0 - smoothstep(-1.0, 1.0, dist);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
