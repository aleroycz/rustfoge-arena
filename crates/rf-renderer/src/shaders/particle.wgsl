// Camera-facing billboard sprites, instanced. Each instance is one
// particle; the quad geometry is shared and just gets scaled/offset by
// the camera's right/up axes so it always faces the viewer.

struct CameraUniform {
    view_proj: mat4x4<f32>,
    view_pos: vec4<f32>,
    camera_right: vec4<f32>,
    camera_up: vec4<f32>,
};

struct LightUniform {
    direction: vec4<f32>,
    color: vec4<f32>,
    ambient: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> light: LightUniform;

struct VertexInput {
    @location(0) corner: vec2<f32>,
};

struct InstanceInput {
    @location(1) world_position: vec3<f32>,
    @location(2) size: f32,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(vertex: VertexInput, instance: InstanceInput) -> VertexOutput {
    let offset = (camera.camera_right.xyz * vertex.corner.x + camera.camera_up.xyz * vertex.corner.y) * instance.size;
    let world_position = instance.world_position + offset;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(world_position, 1.0);
    out.local = vertex.corner;
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft circular falloff so sprites read as particles, not squares.
    let dist = length(in.local);
    let falloff = 1.0 - smoothstep(0.35, 0.5, dist);
    let lit = light.ambient.rgb + light.color.rgb * 0.35;
    return vec4<f32>(in.color.rgb * lit, in.color.a * falloff);
}
