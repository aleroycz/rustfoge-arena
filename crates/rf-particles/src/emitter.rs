use rf_math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct EmitterConfig {
    pub rate: f32,
    pub lifetime: (f32, f32),
    pub initial_speed: (f32, f32),
    pub direction: Vec3,
    pub direction_spread: f32,
    pub size: (f32, f32),
    pub color_start: [f32; 4],
    pub color_end: [f32; 4],
    pub gravity_scale: f32,
    pub drag: f32,
    pub wind_influence: f32,
}

impl Default for EmitterConfig {
    fn default() -> Self {
        Self {
            rate: 10.0,
            lifetime: (1.0, 2.0),
            initial_speed: (1.0, 2.0),
            direction: Vec3::Y,
            direction_spread: 0.2,
            size: (0.05, 0.1),
            color_start: [1.0, 1.0, 1.0, 1.0],
            color_end: [1.0, 1.0, 1.0, 0.0],
            gravity_scale: 1.0,
            drag: 0.1,
            wind_influence: 1.0,
        }
    }
}
