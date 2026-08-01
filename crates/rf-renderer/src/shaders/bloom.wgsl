// Post-process bloom: bright-pass + separable Gaussian blur + additive
// composite back onto the tonemapped scene. All three passes share one
// full-screen-triangle vertex shader (see sky.wgsl for the same trick).
//
// Binding layout is shared across all three fragment entry points to
// avoid group/binding collisions within this one shader module:
//   group(0) = the pass's primary input texture ("src") -- scene color
//              for extract/composite, the horizontally-blurred bloom
//              texture for the vertical blur pass.
//   group(1) = small params uniform (meaning varies per pass, see below).
//   group(2) = composite-only: the fully blurred bloom texture to add on
//              top of `src`.

struct ParamsUniform {
    // xy = texel size of the texture being sampled this pass.
    // z = bright-pass threshold (extract/h-blur pass only).
    // w = bloom intensity (composite pass only).
    params: vec4<f32>,
};

@group(0) @binding(0) var src_texture: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;
@group(1) @binding(0) var<uniform> params: ParamsUniform;
@group(2) @binding(0) var bloom_texture: texture_2d<f32>;
@group(2) @binding(1) var bloom_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOutput;
    let pos = positions[index];
    out.clip_position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = vec2<f32>(pos.x * 0.5 + 0.5, 1.0 - (pos.y * 0.5 + 0.5));
    return out;
}

// A module-scope `const` array can only be indexed by a compile-time
// constant in WGSL -- the 9-tap blur below needs a runtime index, so the
// weights live in a function-local `var` instead (allowed to be indexed
// dynamically).
fn blur_weight(i: i32) -> f32 {
    var weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    return weights[abs(i)];
}

// Bright-pass threshold + horizontal blur in one pass: only the part of a
// pixel's color above `threshold` contributes, so ordinary lit surfaces
// never bloom -- only emissive materials and blown-out highlights do.
@fragment
fn fs_extract_blur_h(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<f32>(params.params.x, 0.0);
    let threshold = params.params.z;
    var sum = vec3<f32>(0.0);
    for (var i = -4; i <= 4; i = i + 1) {
        let uv = in.uv + texel * f32(i);
        let c = textureSample(src_texture, src_sampler, uv).rgb;
        let bright = max(c - vec3<f32>(threshold), vec3<f32>(0.0));
        sum += bright * blur_weight(i);
    }
    return vec4<f32>(sum, 1.0);
}

@fragment
fn fs_blur_v(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = vec2<f32>(0.0, params.params.y);
    var sum = vec3<f32>(0.0);
    for (var i = -4; i <= 4; i = i + 1) {
        let uv = in.uv + texel * f32(i);
        sum += textureSample(src_texture, src_sampler, uv).rgb * blur_weight(i);
    }
    return vec4<f32>(sum, 1.0);
}

@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(src_texture, src_sampler, in.uv).rgb;
    let bloom = textureSample(bloom_texture, bloom_sampler, in.uv).rgb;
    return vec4<f32>(scene + bloom * params.params.w, 1.0);
}
