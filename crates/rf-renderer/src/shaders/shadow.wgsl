// Depth-only pass from the directional light's point of view. Reuses the
// same vertex layout as the forward pass so every mesh can be drawn into
// the shadow map with no extra vertex buffer.

struct LightMatrixUniform {
    view_proj: mat4x4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> light_matrix: LightMatrixUniform;
@group(1) @binding(0) var<uniform> model: ModelUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    let world_position = model.model * vec4<f32>(in.position, 1.0);
    return light_matrix.view_proj * world_position;
}
