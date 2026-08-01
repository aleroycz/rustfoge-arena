use rf_math::Vec3;

#[derive(Debug, Clone, Copy)]
pub struct Particle {
    pub position: Vec3,
    pub velocity: Vec3,
    pub age: f32,
    pub lifetime: f32,
    pub size: f32,

    pub color_start: [f32; 4],
}

impl Particle {
    pub fn life_fraction(&self) -> f32 {
        (self.age / self.lifetime.max(f32::EPSILON)).clamp(0.0, 1.0)
    }
}
