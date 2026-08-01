// Terrain shading: blends grass/rock/snow textures by slope (steep faces
// -> rock) and world height (high altitude -> snow), on top of the same
// Blinn-Phong lighting and height-fog terms the forward pass uses.

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

struct TerrainParams {
    // x = snow start height, y = snow blend range, z = slope sharpness, w = tiling
    params: vec4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

struct FogUniform {
    color_density: vec4<f32>,
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> light: LightUniform;

@group(1) @binding(0) var<uniform> terrain: TerrainParams;
@group(1) @binding(1) var grass_texture: texture_2d<f32>;
@group(1) @binding(2) var rock_texture: texture_2d<f32>;
@group(1) @binding(3) var snow_texture: texture_2d<f32>;
@group(1) @binding(4) var terrain_sampler: sampler;

@group(2) @binding(0) var<uniform> model: ModelUniform;

@group(3) @binding(0) var<uniform> fog: FogUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let world_position = model.model * vec4<f32>(in.position, 1.0);
    out.world_position = world_position.xyz;
    out.world_normal = normalize((model.normal_matrix * vec4<f32>(in.normal, 0.0)).xyz);
    out.uv = in.uv;
    out.clip_position = camera.view_proj * world_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(in.world_normal);

    let grass = textureSample(grass_texture, terrain_sampler, in.uv);
    let rock = textureSample(rock_texture, terrain_sampler, in.uv);
    let snow = textureSample(snow_texture, terrain_sampler, in.uv);

    let snow_start = terrain.params.x;
    let snow_range = max(terrain.params.y, 0.01);
    let slope_sharpness = max(terrain.params.z, 0.01);

    // Slope: 1 = flat ground, 0 = vertical cliff face.
    let slope = clamp(normal.y, 0.0, 1.0);
    let rock_factor = 1.0 - pow(slope, slope_sharpness);

    let snow_factor = clamp((in.world_position.y - snow_start) / snow_range, 0.0, 1.0) * slope;

    var albedo = mix(grass.rgb, rock.rgb, rock_factor);
    albedo = mix(albedo, snow.rgb, snow_factor);

    let light_dir = normalize(-light.direction.xyz);
    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let diffuse = albedo * (light.ambient.rgb + light.color.rgb * n_dot_l);

    // Reinhard tonemap -- see forward.wgsl for why.
    var color = diffuse / (diffuse + vec3<f32>(1.0));

    let dist = length(camera.view_pos.xyz - in.world_position);
    let height_atten = exp(-max(in.world_position.y, 0.0) * fog.params.x);
    let fog_factor = clamp(1.0 - exp(-dist * fog.color_density.a * height_atten), 0.0, 1.0);
    color = mix(color, fog.color_density.rgb, fog_factor);

    return vec4<f32>(color, 1.0);
}
