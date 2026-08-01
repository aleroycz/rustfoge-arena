// Analytic procedural sky: a full-screen triangle whose fragment shader
// reconstructs a world-space view ray and shades it with a day/night
// gradient, a sun disc + glow, a starfield at night, a dim moon glow, and
// weather-driven cloud haze / storm darkening. Not a physically-based
// Hosek-Wilkie sky model, but a deliberately cheap analytic approximation
// that still reads as "sky" under a full day/night/storm cycle.

struct SkyUniform {
    inverse_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    sun_direction: vec4<f32>, // points FROM the ground TOWARD the sun
    sun_color: vec4<f32>,
    // x = cloud coverage [0,1], y = star brightness [0,1], z = storm
    // darkness [0,1], w = elapsed seconds (drives star twinkle).
    params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> sky: SkyUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VertexOutput {
    // Classic "one big triangle covering the screen" trick -- no vertex
    // buffer needed.
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    var out: VertexOutput;
    out.ndc = positions[index];
    out.clip_position = vec4<f32>(out.ndc, 1.0, 1.0);
    return out;
}

fn hash1(p: vec3<f32>) -> f32 {
    let h = dot(p, vec3<f32>(127.1, 311.7, 74.7));
    return fract(sin(h) * 43758.5453123);
}

// Three independent pseudo-random values per input cell: used per-star for
// presence, brightness/twinkle-phase, and a cool/warm color tint.
fn hash3(p: vec3<f32>) -> vec3<f32> {
    let x = hash1(p);
    let y = hash1(p + vec3<f32>(x + 11.13, x + 71.9, x + 3.7));
    let z = hash1(p + vec3<f32>(y + 91.1, y + 5.2, y + 47.3));
    return vec3<f32>(x, y, z);
}

// One layer of a multi-layer procedural starfield. `cell_scale` sets
// density (higher = more, smaller-looking stars); `density` is the
// fraction of cells that actually contain a star. Each star twinkles
// asynchronously (own phase from the hash) and gets a slight cool/warm
// color tint rather than being flat white, which reads much closer to a
// real long-exposure night sky than a uniform static dot pattern.
fn star_layer(dir: vec3<f32>, cell_scale: f32, density: f32, twinkle_speed: f32, time: f32) -> vec3<f32> {
    let cell = floor(dir * cell_scale);
    let rnd = hash3(cell);
    let present = step(1.0 - density, rnd.x);
    let twinkle = 0.55 + 0.45 * sin(time * twinkle_speed + rnd.y * 6.2831853);
    let tint = mix(vec3<f32>(0.75, 0.82, 1.0), vec3<f32>(1.0, 0.92, 0.8), rnd.z);
    let brightness = present * twinkle * mix(0.35, 1.0, rnd.y);
    return tint * brightness;
}

// Leaves values <= 1 untouched and compresses only the part above 1 into
// a smooth highlight rolloff. Used so the sun disc reads as a distinct
// bright feature against the sky instead of clipping to the exact same
// flat white as its own glow / an overcast sky.
fn soft_clip(c: vec3<f32>) -> vec3<f32> {
    let excess = max(c - vec3<f32>(1.0), vec3<f32>(0.0));
    return c - excess + excess / (excess + vec3<f32>(1.0));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let far = sky.inverse_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let far_world = far.xyz / far.w;
    let dir = normalize(far_world - sky.camera_pos.xyz);

    let sun_dir = normalize(sky.sun_direction.xyz);
    let cloud_coverage = sky.params.x;
    let star_brightness = sky.params.y;
    let storm_darkness = sky.params.z;
    let time = sky.params.w;

    // Day/night blend factor from how high the sun is.
    let day = smoothstep(-0.2, 0.15, sun_dir.y);

    let day_zenith = vec3<f32>(0.15, 0.35, 0.75);
    let day_horizon = vec3<f32>(0.75, 0.82, 0.9);
    let night_zenith = vec3<f32>(0.01, 0.015, 0.04);
    let night_horizon = vec3<f32>(0.04, 0.05, 0.09);

    let zenith = mix(night_zenith, day_zenith, day);
    let horizon = mix(night_horizon, day_horizon, day);

    let height = clamp(dir.y, 0.0, 1.0);
    var color = mix(horizon, zenith, pow(height, 0.45));

    // Warm horizon glow at sunrise/sunset.
    let horizon_warmth = (1.0 - abs(sun_dir.y)) * smoothstep(0.0, 0.3, day) * smoothstep(0.6, 0.0, day);
    color += vec3<f32>(0.9, 0.45, 0.15) * horizon_warmth * smoothstep(0.25, -0.05, dir.y) * 0.6;

    // Sun disc + glow.
    let sun_dot = dot(dir, sun_dir);
    let sun_disc = smoothstep(0.9995, 0.9998, sun_dot);
    let sun_glow = pow(max(sun_dot, 0.0), 64.0) * 0.5 + pow(max(sun_dot, 0.0), 8.0) * 0.15;
    color += sky.sun_color.rgb * (sun_disc * 4.0 + sun_glow) * clamp(day + 0.15, 0.0, 1.0);

    // Dim moon glow directly opposite the sun, only visible at night.
    let moon_dot = dot(dir, -sun_dir);
    let moon_glow = pow(max(moon_dot, 0.0), 128.0) * (1.0 - day);
    color += vec3<f32>(0.6, 0.65, 0.75) * moon_glow * 2.0;

    // Starfield: three density layers (sparse/bright, mid, dense/dim) so
    // stars read as varied magnitude rather than uniform dots, each
    // twinkling on its own phase, faded in near the horizon instead of
    // popping at a hard cutoff.
    let horizon_fade = smoothstep(-0.05, 0.08, dir.y);
    let night = (1.0 - day) * star_brightness * horizon_fade;
    if (night > 0.001) {
        var stars = star_layer(dir, 90.0, 0.0009, 0.5, time) * 1.6;
        stars += star_layer(dir, 260.0, 0.006, 0.8, time) * 0.5;
        stars += star_layer(dir, 700.0, 0.0025, 1.3, time) * 0.85;
        color += stars * night;

        // Faint Milky Way-like band: a soft brightening/desaturating
        // stripe along a fixed "galactic plane" axis rather than leaving
        // star density perfectly uniform across the whole sky.
        let galactic_axis = normalize(vec3<f32>(0.4, 0.35, -0.85));
        let band = 1.0 - abs(dot(dir, galactic_axis));
        let milky_way = smoothstep(0.75, 1.0, band) * night;
        color += vec3<f32>(0.55, 0.6, 0.75) * milky_way * 0.12;
    }

    // Faint atmospheric glow low on the night horizon (deep blue, well
    // below star brightness) -- a cheap touch of depth so the night sky
    // doesn't look like a flat black dome behind the stars.
    let night_horizon_glow = smoothstep(0.15, -0.05, dir.y) * (1.0 - day);
    color += vec3<f32>(0.03, 0.05, 0.1) * night_horizon_glow * 0.6;

    // Cloud haze: a soft horizontal band whose opacity scales with
    // coverage, standing in for volumetric clouds without the cost.
    let cloud_band = smoothstep(0.6, -0.1, dir.y) * cloud_coverage;
    color = mix(color, vec3<f32>(0.55, 0.57, 0.6) * mix(1.0, 0.4, storm_darkness), cloud_band * 0.7);

    // Storm darkening desaturates and dims the whole sky.
    let luminance = dot(color, vec3<f32>(0.299, 0.587, 0.114));
    color = mix(color, vec3<f32>(luminance), storm_darkness * 0.6);
    color *= mix(1.0, 0.35, storm_darkness);

    return vec4<f32>(soft_clip(color), 1.0);
}
