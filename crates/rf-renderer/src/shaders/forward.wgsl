// Forward-shaded, single-directional-light Blinn-Phong pass.

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

struct MaterialUniform {
    base_color: vec4<f32>,
    // x = metallic, y = roughness
    metallic_roughness: vec4<f32>,
    // rgb = emissive glow color, added after lighting so it stays
    // visible even at night / deep in fog.
    emissive: vec4<f32>,
};

struct ModelUniform {
    model: mat4x4<f32>,
    normal_matrix: mat4x4<f32>,
};

struct FogUniform {
    // rgb = fog color, a = density
    color_density: vec4<f32>,
    // x = height falloff, rest unused
    params: vec4<f32>,
};

struct ShadowUniform {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> camera: CameraUniform;
@group(0) @binding(1) var<uniform> light: LightUniform;

@group(1) @binding(0) var<uniform> material: MaterialUniform;
@group(1) @binding(1) var base_color_texture: texture_2d<f32>;
@group(1) @binding(2) var base_color_sampler: sampler;

@group(2) @binding(0) var<uniform> model: ModelUniform;

@group(3) @binding(0) var<uniform> fog: FogUniform;

@group(4) @binding(0) var shadow_map: texture_depth_2d;
@group(4) @binding(1) var shadow_sampler: sampler_comparison;
@group(4) @binding(2) var<uniform> shadow: ShadowUniform;

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

// 3x3 PCF: averages 9 taps around the shadow-space texel so shadow edges
// don't read as hard-aliased stair-steps. `bias` grows as the surface
// turns away from the light (grazing angles need more slack) to fight
// shadow acne without needing a second depth-offset pass.
fn shadow_factor(world_position: vec3<f32>, n_dot_l: f32) -> f32 {
    let clip = shadow.view_proj * vec4<f32>(world_position, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));
    if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0 || ndc.z > 1.0 || ndc.z < 0.0) {
        return 1.0;
    }
    let bias = clamp(0.004 * (1.0 - n_dot_l) + 0.0008, 0.0006, 0.006);
    let depth = ndc.z - bias;
    let texel = 1.0 / 2048.0;
    var lit_sum = 0.0;
    for (var x = -1; x <= 1; x = x + 1) {
        for (var y = -1; y <= 1; y = y + 1) {
            let offset = vec2<f32>(f32(x), f32(y)) * texel;
            lit_sum += textureSampleCompare(shadow_map, shadow_sampler, uv + offset, depth);
        }
    }
    return lit_sum / 9.0;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let albedo = textureSample(base_color_texture, base_color_sampler, in.uv) * material.base_color;

    let normal = normalize(in.world_normal);
    let light_dir = normalize(-light.direction.xyz);
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let half_dir = normalize(light_dir + view_dir);

    let n_dot_l = max(dot(normal, light_dir), 0.0);
    let roughness = clamp(material.metallic_roughness.y, 0.05, 1.0);
    let metallic = clamp(material.metallic_roughness.x, 0.0, 1.0);

    let shininess = mix(96.0, 4.0, roughness);
    let spec_strength = mix(0.15, 0.9, metallic);
    let n_dot_h = max(dot(normal, half_dir), 0.0);
    let specular = pow(n_dot_h, shininess) * spec_strength;

    let shadow = shadow_factor(in.world_position, n_dot_l);

    let diffuse = albedo.rgb * (light.ambient.rgb + light.color.rgb * n_dot_l * shadow);
    let lit = diffuse + light.color.rgb * specular * shadow + material.emissive.rgb;

    // Reinhard tonemap: rolls off bright highlights toward white instead
    // of hard-clipping there, so a bright sun/lightning flash doesn't turn
    // every lit surface into a flat white card.
    var color = lit / (lit + vec3<f32>(1.0));

    // Exponential height fog: denser near the ground, thinning with
    // altitude, so distant terrain/objects fade into weather-driven haze
    // instead of popping at a hard draw distance. Blended post-tonemap
    // since the fog color is itself already a display-referred color.
    let dist = length(camera.view_pos.xyz - in.world_position);
    let height_atten = exp(-max(in.world_position.y, 0.0) * fog.params.x);
    let fog_factor = clamp(1.0 - exp(-dist * fog.color_density.a * height_atten), 0.0, 1.0);
    color = mix(color, fog.color_density.rgb, fog_factor);

    return vec4<f32>(color, albedo.a);
}
