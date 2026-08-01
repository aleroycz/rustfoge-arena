// Instanced textured quads sampling the font atlas -- one instance per
// on-screen glyph.

struct ScreenUniform {
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: ScreenUniform;
@group(1) @binding(0) var atlas_texture: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct InstanceInput {
    @location(0) pos: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) uv_min: vec2<f32>,
    @location(3) uv_max: vec2<f32>,
    @location(4) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
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
    out.uv = mix(instance.uv_min, instance.uv_max, corner);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas_texture, atlas_sampler, in.uv).a;
    return vec4<f32>(in.color.rgb, in.color.a * coverage);
}
